//! Verified shutdown — the "no leftovers in Task Manager" guarantee.
//!
//! Closing the main window is intercepted (lib.rs `on_window_event`) and the
//! frontend shows a confirm modal + live checklist that drives these commands:
//! reap every child this process is responsible for, then VERIFY the tree is
//! actually clean before exiting. Same primitives as the update-apply path
//! (`update_service::apply`) and the `RunEvent::Exit` backstop — this module
//! adds the parent-scoped orphan sweep and the verification count on top.
//!
//! Scope discipline: kills are PID-only and parent-scoped — session-registry
//! PIDs (claude trees) plus same-exe processes whose ParentProcessId is US
//! (orphaned `RIFT_MCP_SERVER=1` children). Another Rift instance, the user's
//! own terminal `claude` sessions, and unrelated processes are never touched
//! (unlike update-apply's path-scoped sweep, which must kill same-install
//! siblings to free the Velopack swap target).

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Close-intercept gate. First ✕ intercepts and asks the frontend to confirm;
/// a second ✕ while a confirm is pending (within the window) passes through, so
/// a dead/wedged webview can never make the app unclosable. The frontend's
/// Cancel resets it via `app_close_dismissed`.
static LAST_INTERCEPT: Mutex<Option<Instant>> = Mutex::new(None);
const INTERCEPT_WINDOW: Duration = Duration::from_secs(8);

pub fn should_intercept_close() -> bool {
    let mut g = LAST_INTERCEPT.lock().unwrap_or_else(|p| p.into_inner());
    match *g {
        Some(at) if at.elapsed() < INTERCEPT_WINDOW => false,
        _ => {
            *g = Some(Instant::now());
            true
        }
    }
}

/// Frontend cancelled the close modal — re-arm the intercept.
#[tauri::command]
pub fn app_close_dismissed() {
    *LAST_INTERCEPT.lock().unwrap_or_else(|p| p.into_inner()) = None;
}

/// Same-exe processes whose ParentProcessId is THIS process — orphaned MCP
/// children (`RIFT_MCP_SERVER=1`) whose `claude` parent died mid-turn. `None`
/// when enumeration is unavailable (CIM broken) — caller treats as unknown,
/// never as clean.
#[cfg(windows)]
fn orphaned_child_pids() -> Option<Vec<u32>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let me = std::process::id();
    let exe_path = std::env::current_exe().ok()?;
    let image = exe_path.file_name()?.to_string_lossy().into_owned();
    let my_exe_lower = exe_path.to_string_lossy().to_ascii_lowercase();
    let script = format!(
        "Get-CimInstance Win32_Process -Filter \"Name='{}'\" | ForEach-Object {{ \"$($_.ProcessId)|$($_.ParentProcessId)|$($_.ExecutablePath)\" }}",
        image.replace('\'', "''")
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let mut pids = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut parts = line.trim().split('|');
        let (Some(pid_s), Some(ppid_s), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (Ok(pid), Ok(ppid)) = (pid_s.trim().parse::<u32>(), ppid_s.trim().parse::<u32>())
        else {
            continue;
        };
        // Parent must be us; exe path must provably be ours (null path → skip).
        if ppid != me || pid == me || path.trim().to_ascii_lowercase() != my_exe_lower {
            continue;
        }
        pids.push(pid);
    }
    Some(pids)
}

#[cfg(not(windows))]
fn orphaned_child_pids() -> Option<Vec<u32>> {
    // Unix: session-registry SIGTERM covers the tree; no orphan enumeration.
    Some(Vec::new())
}

/// Step 1 of the close checklist: reap everything we own. Warm-pool drain +
/// tracked claude trees (same as update-apply / RunEvent::Exit, idempotent),
/// then tree-kill any orphaned direct MCP children the registries no longer
/// know about. Returns what was done so the UI can show honest numbers.
#[tauri::command]
pub async fn app_close_reap() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let warm = crate::assistant::pool_size();
        crate::assistant::warm_pool::drain_all_for_shutdown();
        crate::assistant::kill_all_session_children();
        let orphans = orphaned_child_pids();
        let orphan_count = orphans.as_ref().map(|v| v.len());
        if let Some(pids) = orphans {
            for pid in pids {
                crate::assistant::kill_child_tree(pid);
            }
        }
        Ok(serde_json::json!({
            "warmDrained": warm,
            // null = enumeration unavailable, not "zero orphans"
            "orphansKilled": orphan_count,
        }))
    })
    .await
    .map_err(|e| format!("reap task failed: {e}"))?
}

/// Step 2: verify the tree is clean. Counts same-exe processes still parented
/// to us. `leftover: null` = could not verify (CIM unavailable) — the UI says
/// so instead of claiming clean.
#[tauri::command]
pub async fn app_close_verify() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let leftover = orphaned_child_pids();
        Ok(serde_json::json!({
            "leftover": leftover.as_ref().map(|v| v.len()),
            "pids": leftover,
        }))
    })
    .await
    .map_err(|e| format!("verify task failed: {e}"))?
}

/// Final step: exit for real. `RunEvent::Exit` (lib.rs) runs the registry reap
/// again (no-op after `app_close_reap`) + scrubs the MCP bridge token.
#[tauri::command]
pub fn app_exit_now(app: tauri::AppHandle) {
    app.exit(0);
}

/// CLI-update follow-through: the resolved CLI path is cached at startup
/// (`cli_install::CLAUDE_EXE`), so only a relaunch provably puts every code
/// path on the freshly-updated binary. `assistant_update_cli` already reaped
/// the children pre-overwrite; re-reap defensively (idempotent) and relaunch.
#[tauri::command]
pub async fn app_restart_now(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        crate::assistant::warm_pool::drain_all_for_shutdown();
        crate::assistant::kill_all_session_children();
        if let Some(pids) = orphaned_child_pids() {
            for pid in pids {
                crate::assistant::kill_child_tree(pid);
            }
        }
    })
    .await
    .ok();
    app.restart();
}
