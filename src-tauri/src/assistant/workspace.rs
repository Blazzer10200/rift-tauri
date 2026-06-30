//! R7 (per `docs/design/assistant-mod-split.md`) — workspace root state:
//! open/clear/recent folder management, the `@`-mention file enumeration, and
//! the git-branch probe for the Welcome context strip. Lifted verbatim from
//! `assistant/mod.rs` 2026-06-09. Config load/save + RECENT_ROOTS_MAX stay on
//! the parent (R2), reached via `super::`.

use std::path::PathBuf;

use serde::Serialize;

use super::{load_config, save_config, AssistantConfig, CONFIG_WRITE_LOCK, RECENT_ROOTS_MAX};

/// Workspace state surfaced to the frontend. `current` is the open folder or
/// `None` if no folder is open. `recent` is the MRU list (newest first).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub current: Option<String>,
    pub recent: Vec<String>,
}

fn workspace_state_from(cfg: &AssistantConfig) -> WorkspaceState {
    // Drop roots that don't exist on THIS machine — a config copied across
    // machines (or a roaming profile) carries the old box's absolute paths,
    // which would otherwise surface as dead picker entries.
    WorkspaceState {
        current: cfg.current_root.as_ref().filter(|p| p.is_dir()).map(|p| p.to_string_lossy().into_owned()),
        recent: cfg.recent_roots.iter().filter(|p| p.is_dir()).map(|p| p.to_string_lossy().into_owned()).collect(),
    }
}

#[tauri::command]
pub fn assistant_get_workspace() -> Result<WorkspaceState, String> {
    Ok(workspace_state_from(&load_config()))
}

/// Set the active project folder. Validates the path exists and is a directory,
/// canonicalizes it (so `..`/symlinks don't drift), prepends to recent_roots
/// (dedup, capped at RECENT_ROOTS_MAX), and persists.
#[tauri::command]
pub fn assistant_set_root(path: String) -> Result<WorkspaceState, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let raw = PathBuf::from(&path);
    if !raw.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    // canonicalize can fail on a junction-to-nowhere even when is_dir() passed;
    // only fall back to raw if raw itself still resolves, else fail loud.
    let canonical = match std::fs::canonicalize(&raw) {
        Ok(c) => c,
        Err(_) if raw.is_dir() => raw,
        Err(e) => return Err(format!("could not resolve {path}: {e}")),
    };
    let mut cfg = load_config();
    // Dedup: pull existing entry then re-insert at the front.
    cfg.recent_roots.retain(|p| p != &canonical);
    cfg.recent_roots.insert(0, canonical.clone());
    if cfg.recent_roots.len() > RECENT_ROOTS_MAX {
        cfg.recent_roots.truncate(RECENT_ROOTS_MAX);
    }
    cfg.current_root = Some(canonical);
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

/// Set a single tab/pane's project folder WITHOUT touching the global
/// `current_root`. Validates + canonicalizes the path and records it in the
/// shared recent-roots MRU (so the picker still offers it), then returns the
/// canonical path so the renderer can store it on that tab. This is what keeps
/// per-pane folders from leaking into each other: only the tab's own
/// `workspaceRoot` changes, never the global default the way `assistant_set_root` does.
#[tauri::command]
pub fn assistant_set_tab_root(path: String) -> Result<String, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let raw = PathBuf::from(&path);
    if !raw.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let canonical = match std::fs::canonicalize(&raw) {
        Ok(c) => c,
        Err(_) if raw.is_dir() => raw,
        Err(e) => return Err(format!("could not resolve {path}: {e}")),
    };
    let mut cfg = load_config();
    cfg.recent_roots.retain(|p| p != &canonical);
    cfg.recent_roots.insert(0, canonical.clone());
    if cfg.recent_roots.len() > RECENT_ROOTS_MAX {
        cfg.recent_roots.truncate(RECENT_ROOTS_MAX);
    }
    save_config(&cfg)?;
    Ok(canonical.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn assistant_clear_root() -> Result<WorkspaceState, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    cfg.current_root = None;
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

#[tauri::command]
pub fn assistant_remove_recent_root(path: String) -> Result<WorkspaceState, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let raw = PathBuf::from(&path);
    // Stored roots are canonicalized on insert (assistant_set_root). Canonicalize
    // the target the same way so removal matches regardless of case/trailing-slash/
    // `..` drift; keep the raw form too so a now-deleted dir (canonicalize fails)
    // is still removable.
    let canonical = std::fs::canonicalize(&raw).ok();
    let mut cfg = load_config();
    cfg.recent_roots
        .retain(|p| p != &raw && Some(p) != canonical.as_ref());
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

/// The active workspace root, if the user has opened a folder. Exposed for
/// the STT engine's workspace-context prompt injection.
pub(crate) fn current_root() -> Option<PathBuf> {
    load_config().current_root
}

/// The persistent local scratch workspace used when no project folder is open.
/// Resolves to `%LOCALAPPDATA%\Rift\local` (pattern mirrors `certs.rs` — prefer
/// LOCALAPPDATA, which GPO forbids redirecting, over a redirectable temp dir),
/// always `create_dir_all`'d so it self-heals a deleted dir and is guaranteed to
/// exist for the MCP containment boundary + `current_dir`. Backend-resolved only,
/// never renderer-supplied → no path-injection surface.
pub(crate) fn local_scratch_dir() -> Result<PathBuf, String> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("Rift").join("local");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create local scratch dir: {e}"))?;
    Ok(dir)
}

/// The local scratch workspace path, surfaced to the renderer for the "Local"
/// badge only (FE never supplies it back — the backend re-resolves per turn).
/// `None` if the dir can't be created (LOCALAPPDATA + temp both unwritable).
#[tauri::command]
pub fn assistant_local_scratch_path() -> Option<String> {
    local_scratch_dir().ok().map(|p| p.to_string_lossy().into_owned())
}

/// Resolve a per-tab root override (validated dir) or fall back to the global
/// `current_root`. Lets the `@`-mention walk + branch probe scope to whichever
/// pane the user is interacting with instead of always the global default.
fn resolve_root(override_path: Option<String>) -> Option<PathBuf> {
    override_path
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| load_config().current_root)
}

/// Enumerate file paths under the active workspace root, relative to the root,
/// forward-slash normalized. Drives the composer's `@`-file mention picker.
/// Capped at `MENTION_LIMIT` files.
/// Synchronous walk shared by the async command and `stt::workspace_context`.
/// Returns root-relative, forward-slash paths, capped at 4000 entries.
pub fn list_workspace_files_sync(root: &std::path::Path) -> Vec<String> {
    const MENTION_LIMIT: usize = 4000;
    use super::mcp_server::SKIP_DIRS;
    // Per-project file-pattern scoping: if a defined project owns this root, the
    // `@`-mention picker honors its include/exclude so it offers the same files
    // the workspace tools can actually read. No project → empty filter → all
    // files (minus SKIP_DIRS), unchanged.
    let (inc, exc) = super::projects::patterns_for_root(&super::load_config(), root);
    let filter = super::mcp_server::PathFilter::from_globs(&inc.join("\n"), &exc.join("\n"));
    let filter_active = filter.is_active();
    let mut out = Vec::with_capacity(512);
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()) {
                return false;
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        let rel = rel.to_string_lossy().replace('\\', "/");
        if filter_active && !filter.allows_rel(&rel) {
            continue;
        }
        out.push(rel);
        if out.len() >= MENTION_LIMIT {
            break;
        }
    }
    out
}

#[tauri::command]
pub async fn assistant_list_workspace_files(root: Option<String>) -> Result<Vec<String>, String> {
    let root = match resolve_root(root) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    // Offload the synchronous walk to a blocking thread — a large monorepo or
    // network FS can take seconds and would otherwise stall a Tokio worker.
    tokio::task::spawn_blocking(move || list_workspace_files_sync(&root))
        .await
        .map_err(|e| format!("walk: {e}"))
}

/// Current git branch of the active workspace root, or `None` when the folder
/// isn't a git repo, is in detached-HEAD, or git isn't available. Surfaced in
/// the assistant Welcome's context strip; never fabricated.
/// Blocking `git rev-parse --abbrev-ref HEAD`. Synchronous helper shared by the
/// async command and the sync `stt::workspace_context` caller (mirrors
/// list_workspace_files_sync). Callers on a Tokio thread MUST wrap in
/// spawn_blocking — this does real subprocess I/O.
pub fn workspace_branch_sync(root: &std::path::Path) -> Option<String> {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        // Mirror run_git's env hardening posture (git_local.rs): strip the env
        // vars that could redirect the git binary (GIT_EXEC_PATH), inject config
        // (GIT_CONFIG_*), or repoint the repo/work-tree out from under us. A
        // read-only rev-parse is low-risk, but the parity keeps the defense
        // consistent across every git invocation.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_EXEC_PATH")
        .env_remove("GIT_CONFIG_GLOBAL")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

#[tauri::command]
pub async fn assistant_workspace_branch(root: Option<String>) -> Option<String> {
    let root = resolve_root(root)?;
    // RR7: `git rev-parse` is a blocking OS subprocess; on a network-mounted
    // workspace it can stall for seconds. Run it on the blocking pool so it
    // can't starve a Tokio worker (mirrors assistant_list_workspace_files).
    tokio::task::spawn_blocking(move || workspace_branch_sync(&root))
        .await
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_scratch_dir_creates_and_is_dir() {
        let dir = local_scratch_dir().expect("scratch dir resolves");
        assert!(dir.is_dir(), "scratch dir must exist after resolve");
        assert!(dir.ends_with("Rift/local") || dir.ends_with("Rift\\local"),
            "scratch dir tail should be Rift/local, got {}", dir.display());
    }

    #[test]
    fn local_scratch_dir_is_idempotent() {
        let a = local_scratch_dir().expect("first resolve");
        let b = local_scratch_dir().expect("second resolve self-heals / no-ops");
        assert_eq!(a, b, "repeated resolution yields the same path");
        assert!(b.is_dir());
    }
}
