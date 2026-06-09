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
    WorkspaceState {
        current: cfg.current_root.as_ref().map(|p| p.to_string_lossy().into_owned()),
        recent: cfg.recent_roots.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
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
    let canonical = std::fs::canonicalize(&raw).unwrap_or(raw);
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

#[tauri::command]
pub fn assistant_clear_root() -> Result<WorkspaceState, String> {
    let mut cfg = load_config();
    cfg.current_root = None;
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

#[tauri::command]
pub fn assistant_remove_recent_root(path: String) -> Result<WorkspaceState, String> {
    let target = PathBuf::from(&path);
    let mut cfg = load_config();
    cfg.recent_roots.retain(|p| p != &target);
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

/// The active workspace root, if the user has opened a folder. Exposed for
/// the STT engine's workspace-context prompt injection.
pub(crate) fn current_root() -> Option<PathBuf> {
    load_config().current_root
}

/// Enumerate file paths under the active workspace root, relative to the root,
/// forward-slash normalized. Drives the composer's `@`-file mention picker.
/// Skip set mirrors `mcp_server::SKIP_DIRS`. Capped at `MENTION_LIMIT` files.
#[tauri::command]
pub fn assistant_list_workspace_files() -> Result<Vec<String>, String> {
    const MENTION_LIMIT: usize = 4000;
    const SKIP_DIRS: &[&str] = &[
        "node_modules", ".git", ".svelte-kit", "build", "dist", "target",
        ".rift-trail", ".rift-tmp", "__pycache__", ".venv", ".next",
    ];
    let cfg = load_config();
    let root = match cfg.current_root {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::with_capacity(512);
    for entry in walkdir::WalkDir::new(&root)
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
        let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
        out.push(rel.to_string_lossy().replace('\\', "/"));
        if out.len() >= MENTION_LIMIT {
            break;
        }
    }
    Ok(out)
}

/// Current git branch of the active workspace root, or `None` when the folder
/// isn't a git repo, is in detached-HEAD, or git isn't available. Surfaced in
/// the assistant Welcome's context strip; never fabricated.
#[tauri::command]
pub fn assistant_workspace_branch() -> Option<String> {
    let root = load_config().current_root?;
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(&root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .env_remove("GIT_DIR");
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
