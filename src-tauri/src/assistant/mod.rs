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
pub mod cli_install;
pub mod config;
pub mod convo_store;
pub mod env_checks;
pub mod git_local;
pub mod mcp_server;
pub mod oneshot;
pub mod permission;
pub mod session_log;
pub mod turn;
pub mod workspace;

pub use ask_user::AskUserRegistry;
// R4 split (2026-06-09): auth probe + in-app sign-in + multi-install CLI
// updater in `auth_update.rs`. Glob re-export for the __cmd__ items.
pub use auth_update::*;
// R1 split (2026-06-09): CLI discovery/ranking/cache + spawn-command builder
// in `cli_install.rs`. Re-exports keep `assistant::ClaudeInstall` (DTO) and
// `crate::assistant::claude_command()` (stt/swarm callers) path-stable.
pub use cli_install::ClaudeInstall;
pub(crate) use cli_install::claude_command;
// R2 split (2026-06-09): AssistantConfig + provider profiles + config get/set
// commands + validation helpers in `config.rs`. Glob re-export for the
// __cmd__ items; helpers stay reachable as `super::X` across the subtree.
pub use config::*;
// R3 split (2026-06-09): conversation persistence (convo JSON store, session
// cwd/model sidecars, session-id validation, retired-JSONL sweep) in
// `convo_store.rs`. Glob re-export for the __cmd__ items + Conversation DTOs
// + lib.rs's `assistant::cleanup_retired_jsonls`.
pub use convo_store::*;
// R5 split (2026-06-09): compression toggle + proxy/host-tool probes in
// `env_checks.rs`. Glob re-export (like session_log) so the macro-generated
// `__cmd__*` items travel with the commands — a named re-export strands them.
pub use env_checks::*;
// R6 split (2026-06-09): one-shot headless spawns (enhance / title / summarize
// / remint) in `oneshot.rs`. Glob re-export for the __cmd__ items +
// SummarizeResult DTO.
pub use oneshot::*;
// R7 split (2026-06-09): workspace root state + @-mention file enumeration +
// branch probe in `workspace.rs`. Glob re-export for the same __cmd__ reason;
// `current_root` stays reachable as `crate::assistant::current_root` (stt).
pub use workspace::*;
pub(crate) use workspace::current_root;
pub use permission::PermissionRegistry;
// Flatten the session-log commands into `crate::assistant` so the wildcard
// `pub use crate::assistant::*` in commands/assistant.rs forwards them to the
// path `invoke_handler!` resolves.
pub use session_log::*;
// R8 split (2026-06-09): the live-turn nervous system — session registry
// (PIDs/stop/steer + `kill_all_session_children`, load-bearing for the
// Velopack apply), control-response + permission plumbing, and
// assistant_send/stop/steer in `turn.rs`. Glob re-export for the __cmd__
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

    let mut env_map = serde_json::Map::new();
    env_map.insert("RIFT_MCP_SERVER".into(), Value::from("1"));
    env_map.insert("RIFT_MCP_ROOTS".into(), Value::from(roots_joined));
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
            let icacls_status = std::process::Command::new("icacls")
                .arg(&path)
                // Quote the principal: domain usernames can contain spaces, which
                // icacls would otherwise parse as separate ACL tokens.
                .args(["/inheritance:r", "/grant:r", &format!("\"{user}\":(F)")])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if !matches!(icacls_status, Ok(s) if s.success()) {
                log::warn!("icacls failed to lock down {} for user {user}", path.display());
            }
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
