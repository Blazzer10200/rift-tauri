//! Assistant — JetBrains-style AI partner page (beta).
//!
//! Architecture (locked 2026-05-14, see `docs/design/assistant-page.md`):
//! Rift's Rust side spawns the user's installed `claude` CLI in headless
//! streaming-JSON mode, parses NDJSON line-by-line, re-emits each event to the
//! frontend via Tauri events. The Agent SDK npm package is Node-only and cannot
//! run in the Tauri webview, so we drive the same CLI it would wrap.
//!
//! Auth model: piggyback on the user's existing `claude login` session by
//! default (CLI reads its own keychain). API-key fallback: when configured,
//! spawn with `--bare` + `ANTHROPIC_API_KEY` env so the CLI ignores OAuth.

pub mod ask_user;
pub mod auth_update;
pub mod bridge;
pub mod cli_caps;
pub mod cli_install;
pub mod config;
pub mod convo_store;
pub mod env_checks;
pub mod gh_remote;
pub mod git_local;
pub mod local_llm;
pub mod mcp_bridge;
pub mod mcp_list;
pub mod mcp_server;
pub mod news;
pub mod nothink;
pub mod oneshot;
pub mod permission;
pub mod proc_tree;
pub mod projects;
pub mod providers;
pub mod skills_catalog;
pub mod turn;
pub mod warm_pool;
pub mod workspace;

pub use ask_user::AskUserRegistry;
// R4 split (2026-06-09): auth probe + in-app sign-in + multi-install CLI
// updater in `auth_update.rs`. Glob re-export for the __cmd__ items.
pub use auth_update::*;
// R1 split (2026-06-09): CLI discovery/ranking/cache + spawn-command builder
// in `cli_install.rs`. Re-exports keep `assistant::ClaudeInstall` (DTO) and
// `crate::assistant::claude_command()` (turn/oneshot/stt callers) path-stable.
pub use cli_install::ClaudeInstall;
pub(crate) use cli_install::claude_command;
// R2 split (2026-06-09): AssistantConfig + config get/set
// commands + validation helpers in `config.rs`. Glob re-export for the
// __cmd__ items; helpers stay reachable as `super::X` across the subtree.
pub use config::*;
// R3 split (2026-06-09): conversation persistence (convo JSON store, session
// cwd/model sidecars, session-id validation, retired-JSONL sweep) in
// `convo_store.rs`. Glob re-export for the __cmd__ items + Conversation DTOs
// + lib.rs's `assistant::cleanup_retired_jsonls`.
pub use convo_store::*;
// R5 split (2026-06-09): host-tool probes in
// `env_checks.rs`. Glob re-export (like session_log) so the macro-generated
// `__cmd__*` items travel with the commands — a named re-export strands them.
pub use env_checks::*;
// R6 split (2026-06-09): one-shot headless spawns (enhance / title) in
// `oneshot.rs`. Glob re-export for the __cmd__ items.
pub use oneshot::*;
// Local-LLM commands (test/list/context/optimize) split out of oneshot.rs
// (2026-06-27). Glob re-export for the __cmd__ items.
pub use local_llm::*;
// Multi-model provider registry (2026-07-16, docs/design/multi-model-providers.md).
// Glob re-export for the __cmd__ items.
pub use providers::*;
// "What's new in AI" feed (Workspace page): deterministic changelog+npm fetch +
// opt-in AI digest in `news.rs`. Glob re-export for the __cmd__ items.
pub use news::*;
// `/mcp` harness view: `claude mcp list` shell-out + parser in `mcp_list.rs`.
// Glob re-export for the __cmd__ item.
pub use mcp_list::*;
// R7 split (2026-06-09): workspace root state + @-mention file enumeration +
// branch probe in `workspace.rs`. Glob re-export for the same __cmd__ reason;
// `current_root` stays reachable as `crate::assistant::current_root` (stt).
pub use workspace::*;
pub(crate) use workspace::current_root;
// Projects: named alias over a workspace folder + per-project file-pattern
// config. Glob re-export for the __cmd__ items (same reason as workspace).
pub use projects::*;
// Custom slash-command discovery (`/` menu): user + project skills/commands
// metadata. Glob re-export for the __cmd__ item.
pub use skills_catalog::*;
pub use permission::PermissionRegistry;
// R8 split (2026-06-09): the live-turn nervous system — session registry
// (PIDs/stop + `kill_all_session_children`, load-bearing for the
// Velopack apply), control-response + permission plumbing, and
// assistant_send/stop in `turn.rs`. Glob re-export for the __cmd__
// items + `crate::assistant::kill_all_session_children` path stability.
pub use turn::*;
pub(crate) use turn::kill_all_session_children;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;


/// Output of `claude auth status` plus our locally-stored API-key flag.
/// All fields camelCase to match the CLI's JSON shape verbatim.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub cli_present: bool,
    pub cli_version: Option<String>,
    pub logged_in: bool,
    pub auth_method: Option<String>,
    pub api_provider: Option<String>,
    pub email: Option<String>,
    pub subscription_type: Option<String>,
    pub api_key_configured: bool,
    /// A system `ANTHROPIC_API_KEY` env var is present in Rift's environment.
    /// Rift deliberately ignores it (env keys are stripped from every spawn so
    /// the keychain/login model stays authoritative) — surfaced only so the UI
    /// can warn that an out-of-band key exists and is NOT being used.
    pub env_api_key_present: bool,
    /// How the resolved `claude` binary was installed — "npm" | "native" |
    /// "unknown". Drives the correct in-app update command. None when no CLI.
    pub install_method: Option<String>,
    /// Pill color: "green" | "yellow" | "red".
    pub pill: String,
    /// One-line user-facing status.
    pub summary: String,
    /// Every Claude CLI install detected on this machine (npm + native can
    /// coexist). The `active` one drives `cli_version`/`install_method`; the
    /// rest are surfaced so the UI can show + update all of them.
    #[serde(default)]
    pub installs: Vec<ClaudeInstall>,
}


pub(super) fn dirs_home() -> Result<PathBuf, String> {
    crate::state::paths::dirs_home().map_err(|e| e.to_string())
}

/// Strip the Windows verbatim/UNC prefix (`\\?\`) so a canonicalized path (which
/// `std::fs::canonicalize` returns as `\\?\C:\…`) compares lexically equal to a
/// plain absolute path. No-op on non-Windows. (cont.228: was duplicated as
/// `strip_unc` in mcp_server.rs + `strip_verbatim` in git_local.rs.)
#[cfg(windows)]
pub(super) fn strip_unc(p: &std::path::Path) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => p.to_path_buf(),
    }
}

#[cfg(not(windows))]
pub(super) fn strip_unc(p: &std::path::Path) -> PathBuf {
    p.to_path_buf()
}

/// Read a reqwest response body with a hard byte `cap`. A hostile/misbehaving
/// endpoint could stream an unbounded body into `.text()` and OOM us; this stops
/// at `cap` bytes (the surplus is dropped at the chunk boundary). Decodes lossily
/// — callers parse JSON / plain-text diagnostics, not exact binary. (cont.228:
/// was duplicated in local_llm.rs (256KB) + news.rs (8MB) — the cap is now the
/// per-call knob, since npm's registry doc must NOT be truncated mid-parse.)
pub(super) async fn read_body_capped(resp: reqwest::Response, cap: usize) -> String {
    let mut resp = resp;
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < cap {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let take = (cap - buf.len()).min(chunk.len());
                buf.extend_from_slice(&chunk[..take]);
                if take < chunk.len() {
                    break;
                }
            }
            Ok(None) | Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}


/// Monotonic per-turn nonce for MCP config filenames. Prevents an outgoing
/// turn's cleanup guard from deleting the file a same-session resume just wrote
/// (the queue-drain race). Process-lifetime counter — uniqueness, not ordering.
static MCP_CFG_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// RAII guard that deletes the per-turn MCP config file when dropped.
/// Ensures cleanup on normal exit, early-return errors, and panics alike.
struct McpConfigGuard(PathBuf);

impl Drop for McpConfigGuard {
    fn drop(&mut self) {
        if self.0.exists() {
            if let Err(e) = std::fs::remove_file(&self.0) {
                log::warn!("assistant: failed to remove per-session mcp-config on drop: {e}");
            } else {
                log::debug!("assistant: removed per-session mcp-config {:?}", self.0);
            }
        }
    }
}

/// Write the Rift MCP server config the CLI will read via `--mcp-config`.
/// Points at our own `current_exe()` with `RIFT_MCP_SERVER=1`; the binary
/// branches to `mcp_server::run_stdio` instead of launching Tauri. Workspace
/// roots are passed via `RIFT_MCP_ROOTS` (newline-separated) so the spawned
/// child knows the path-safety boundary at request time.
///
/// `session_id` is appended to the filename so concurrent `assistant_send`
/// calls (multi-tab) each get their own file — no cross-tab cred leak.
fn write_mcp_config(
    session_id: &str,
    roots: &[PathBuf],
    trust_level: &str,
    window_label: &str,
    include: &[String],
    exclude: &[String],
) -> Result<PathBuf, String> {
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir ~/.rift/assistant: {e}"))?;
    // Per-TURN filename, not per-session. A queued message drains into a fresh
    // `assistant_send` that --resumes the SAME session_id the instant the prior
    // turn's `result` lands — but that prior turn's McpConfigGuard hasn't dropped
    // yet (it waits on child.wait(), up to REAP_GRACE later). A shared per-session
    // name let the OUTGOING guard delete the file the INCOMING turn just wrote,
    // so claude2 read `--mcp-config` and found nothing → "exited with 1 — config
    // file not found". The monotonic nonce gives each turn its own file; the guard
    // deletes by the returned PathBuf, so it only ever removes its own. The
    // exit-time glob `mcp-config-*.json` still sweeps every variant. sanitize:
    // replace path-unsafe chars so the session UUID is a valid filename component.
    let safe_id = session_id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
    let nonce = MCP_CFG_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!("mcp-config-{safe_id}-{nonce}.json"));

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    // RIFT_MCP_ROOTS is newline-separated, so a root path containing an embedded
    // newline would split into two phantom roots in the MCP child — widening the
    // path-safety boundary. Drop any such path rather than corrupt the list (F59).
    let roots_joined = roots
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| !s.contains('\n') && !s.contains('\r'))
        .collect::<Vec<_>>()
        .join("\n");

    // Per-project file-pattern globs, newline-separated like RIFT_MCP_ROOTS.
    // Same embedded-newline strip so one pattern can't split into two phantom
    // entries in the MCP child. Empty string = no patterns → the child applies
    // only its always-on SKIP_DIRS baseline (unchanged behavior).
    let join_patterns = |pats: &[String]| {
        pats.iter()
            .map(|s| s.to_string())
            .filter(|s| !s.contains('\n') && !s.contains('\r'))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut env_map = serde_json::Map::new();
    env_map.insert("RIFT_MCP_SERVER".into(), Value::from("1"));
    env_map.insert("RIFT_MCP_ROOTS".into(), Value::from(roots_joined));
    env_map.insert("RIFT_MCP_INCLUDE".into(), Value::from(join_patterns(include)));
    env_map.insert("RIFT_MCP_EXCLUDE".into(), Value::from(join_patterns(exclude)));
    // Plumb the convo's session_id so the `ask_user` MCP tool can tag its
    // bridge request — the frontend pairs incoming `assistant://ask-user`
    // events against the correct chat tab by session_id.
    env_map.insert("RIFT_SESSION_ID".into(), Value::from(session_id.to_string()));
    // Trust level gates the local git tools in the MCP child. Always injected —
    // git is a local op, no bridge needed. See `mcp_server::trust_level`.
    env_map.insert("RIFT_TRUST_LEVEL".into(), Value::from(trust_level.to_string()));
    // UI bridge (ask_user / open_browser / notify). Absent when the boot-time
    // bind failed — the MCP child then simply doesn't list those tools.
    if let Some(info) = bridge::bridge_info() {
        env_map.insert("RIFT_BRIDGE_PORT".into(), Value::from(info.port.to_string()));
        env_map.insert("RIFT_BRIDGE_TOKEN".into(), Value::from(info.token.clone()));
        // #37: window label so the MCP child can tag bridge requests, letting the
        // bridge emit_to the originating window instead of broadcasting app-wide.
        env_map.insert("RIFT_BRIDGE_WINDOW".into(), Value::from(window_label.to_string()));
    }

    let payload = serde_json::json!({
        "mcpServers": {
            "rift": {
                "command": exe.to_string_lossy(),
                "args": [],
                "env": Value::Object(env_map),
            }
        }
    });
    let s = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&path, s).map_err(|e| format!("write mcp-config: {e}"))?;

    // #9.2 + #38: tighten permissions so the on-disk bridge token isn't world-
    // readable in the interval between write + delete-on-exit.
    // Unix: explicit 0600 on the file.
    // Windows: explicit DACL via `icacls` — strip inheritance and grant the
    // current user Full Control only. NTFS inheritance from `%USERPROFILE%\.rift\`
    // is user-only on a standalone profile but NOT on domain-joined / shared
    // setups where inheritance can grant SYSTEM/Administrators read.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let user = std::env::var("USERNAME").unwrap_or_default();
        if !user.is_empty() {
            // /inheritance:r — strip inherited ACEs; /grant:r — replace user grant.
            // Output discarded; failure is non-fatal (file is still delete-on-exit
            // and the embedded token rotates each app launch).
            // RR2: `icacls` blocks until the subprocess exits (sub-100ms normally,
            // but seconds under AV/contention). This fn runs on a Tokio worker
            // (called from async `assistant_send`), so spawn the lockdown on a
            // detached OS thread to keep the executor unblocked. Best-effort: the
            // file is delete-on-exit and the token rotates each launch, so a
            // not-yet-applied DACL during the spawn window is acceptable.
            let path_for_acl = path.clone();
            // Domain-qualify the principal — bare USERNAME isn't a resolvable SID
            // for icacls on a domain-joined machine (see config::acl_principal).
            let principal = config::acl_principal(&user);
            std::thread::spawn(move || {
                let icacls_status = std::process::Command::new("icacls")
                    .arg(&path_for_acl)
                    // No manual quotes: Rust's Windows Command quotes any argv element
                    // containing spaces when it builds the command line, so a
                    // `DOMAIN\First Last:(F)` principal is wrapped automatically and
                    // icacls sees a single token. Embedding literal `"` here instead
                    // gets re-escaped into the command line and icacls rejects the
                    // mangled token — the silent every-turn failure cont.202 shipped.
                    .args(["/inheritance:r", "/grant:r", &format!("{principal}:(F)")])
                    .creation_flags(CREATE_NO_WINDOW)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                if !matches!(icacls_status, Ok(s) if s.success()) {
                    log::warn!("icacls failed to lock down {} for {principal}", path_for_acl.display());
                }
            });
        }
    }

    Ok(path)
}

/// #9.2: best-effort removal of all per-session MCP config files on app exit
/// so bridge tokens don't sit on disk between sessions. Tokens become stale
/// the moment the process exits (new ones generated next run), but leaked
/// stale tokens are still strictly more information than non-existent files.
/// Globs `mcp-config-*.json` to catch any files left behind by crashed or
/// cancelled sessions, plus the legacy fixed `mcp-config.json` name.
/// Errors are logged + swallowed — cleanup failure must not block app shutdown.
pub fn cleanup_mcp_config_on_exit() {
    let Ok(home) = dirs_home() else { return };
    let dir = home.join(".rift").join("assistant");

    // Legacy fixed-path (pre-per-session fix) — remove if present.
    let legacy = dir.join("mcp-config.json");
    if legacy.exists() {
        match std::fs::remove_file(&legacy) {
            Ok(()) => log::info!("assistant: removed legacy mcp-config.json on exit"),
            Err(e) => log::warn!("assistant: failed to remove legacy mcp-config.json on exit: {e}"),
        }
    }

    // Per-session files: glob mcp-config-*.json.
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        if name.starts_with("mcp-config-") && name.ends_with(".json") {
            match std::fs::remove_file(entry.path()) {
                Ok(()) => log::info!("assistant: removed stale {} on exit", name),
                Err(e) => log::warn!("assistant: failed to remove {} on exit: {e}", name),
            }
        }
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_level_allowlist_and_resolution() {
        assert!(is_valid_trust_level("readonly"));
        assert!(is_valid_trust_level("standard"));
        // CR-UX: "full" collapsed — invalid for NEW writes, but persisted
        // ternary-era configs migrate read-side to "standard".
        assert!(!is_valid_trust_level("full"));
        assert!(!is_valid_trust_level("admin"));
        assert!(!is_valid_trust_level(""));
        // Unset or garbage must floor to readonly — never escalate.
        assert_eq!(effective_trust_level(&None), "readonly");
        assert_eq!(effective_trust_level(&Some("garbage".into())), "readonly");
        assert_eq!(effective_trust_level(&Some("standard".into())), "standard");
        assert_eq!(effective_trust_level(&Some("full".into())), "standard");
    }

    #[test]
    fn permission_mode_allowlist_excludes_dontask() {
        for ok in ["default", "acceptEdits", "plan", "auto", "bypassPermissions"] {
            assert!(is_valid_permission_mode(ok), "{ok} should be valid");
        }
        // `dontAsk` is the CLI's auto-DENY mode — must NOT be exposed.
        assert!(!is_valid_permission_mode("dontAsk"));
        assert!(!is_valid_permission_mode(""));
    }
}
