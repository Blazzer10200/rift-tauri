//! Updater command surface — Velopack (v0.4.47+, restored 2026-06-04).
//!
//! Thin Tauri wrappers over `crate::update_service::UpdateService` (a managed
//! singleton). Flow: `check_for_updates` → `download_update` (streams
//! `update-progress` 0..=100, then `update-downloaded`) → `apply_pending_update`
//! (schedules the swap, exits the app; Velopack relaunches the new version).
//!
//! All `UpdateManager` calls are blocking I/O, so each runs on `spawn_blocking`.
//! See `update_service.rs` (arc history: `git log -- docs/design/velopack-auto-update.md`).

use crate::update_service::{UpdateInfoDto, UpdateService};
use std::sync::Arc;

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Check GitHub for a newer release.
///
/// * `Ok(Some(info))` — a newer release is available.
/// * `Ok(None)`       — up to date / no source configured.
/// * `Err(msg)`       — the check itself failed (offline, rate-limited, parse).
#[tauri::command]
pub async fn check_for_updates(
    svc: tauri::State<'_, Arc<UpdateService>>,
) -> Result<Option<UpdateInfoDto>, String> {
    use std::time::Duration;
    let svc = svc.inner().clone();
    let task = tokio::task::spawn_blocking(move || svc.check());
    // Hard backstop: velopack's GitHub fetch (reqwest) has no default timeout,
    // so a hung/half-open connection would otherwise spin the UI forever. After
    // 30s surface an error the frontend can render — never an infinite spinner.
    // The orphaned blocking thread is harmless: the mutex is already released
    // before the network call, so a later check spawns cleanly.
    match tokio::time::timeout(Duration::from_secs(30), task).await {
        Ok(joined) => joined.map_err(|e| format!("update check task: {e}"))?,
        Err(_) => Err("update check timed out after 30s — GitHub may be unreachable or blocked".to_string()),
    }
}

/// Arm a "Repair installation" — force the pending plan to the latest full
/// release so the next `download_update` + `apply_pending_update` reinstalls it,
/// overwriting any corrupted binaries (even when already on the latest version).
/// Returns the release that will be reinstalled. Frontend then drives the SAME
/// download → apply chain as a normal update.
#[tauri::command]
pub async fn repair_install(
    svc: tauri::State<'_, Arc<UpdateService>>,
) -> Result<UpdateInfoDto, String> {
    use std::time::Duration;
    log::info!("repair_install: command invoked (frontend → backend OK)");
    let svc = svc.inner().clone();
    let task = tokio::task::spawn_blocking(move || svc.arm_repair());
    match tokio::time::timeout(Duration::from_secs(30), task).await {
        Ok(joined) => joined.map_err(|e| format!("repair task: {e}"))?,
        Err(_) => Err("repair timed out after 30s — the update feed may be unreachable".to_string()),
    }
}

/// Download the pending update package. Emits `update-progress` (i16 0..=100)
/// as bytes arrive, then `update-downloaded` on success.
#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    svc: tauri::State<'_, Arc<UpdateService>>,
) -> Result<(), String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;
    use tauri::Emitter;

    // No progress tick for this long ⇒ the transfer is wedged (half-open
    // socket, dead proxy). Generous enough to cover the initial connect/TLS
    // handshake before the first byte. A flat overall timeout would be wrong —
    // a slow connection legitimately takes minutes for ~15 MB.
    const STALL_SECS: u64 = 90;

    // Bisection marker: proves the frontend `invoke("download_update")` actually
    // reached the backend. If this line is absent from rift.log after a click,
    // the click never invoked (frontend/UI bug); if present without a following
    // "update download: starting", the failure is in the service (no pending/mgr).
    log::info!("download_update: command invoked (frontend → backend OK)");

    let svc = svc.inner().clone();
    let svc_cancel = svc.clone();
    let (tx, rx) = std::sync::mpsc::channel::<i16>();

    // Forward Velopack's progress ticks to the frontend on a side thread. If no
    // tick arrives within STALL_SECS, flag a stall and bail so the UI can't sit
    // on "downloading" forever. Disconnect (tx dropped on task completion) ends
    // the loop cleanly without flagging a stall.
    let pump_app = app.clone();
    let stalled = Arc::new(AtomicBool::new(false));
    let stalled_pump = stalled.clone();
    let pump = std::thread::spawn(move || loop {
        match rx.recv_timeout(Duration::from_secs(STALL_SECS)) {
            Ok(pct) => {
                let _ = pump_app.emit("update-progress", pct);
            }
            Err(RecvTimeoutError::Timeout) => {
                stalled_pump.store(true, Ordering::SeqCst);
                break;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    });

    let task = tokio::task::spawn_blocking(move || svc.download(tx));

    // Race the blocking download against the stall watchdog. On stall we return
    // an error and leave the orphaned blocking task running — it's harmless: the
    // mutex is released before its network I/O, and it only flips `downloaded`
    // under lock if it ever completes.
    let outcome = tokio::select! {
        joined = task => joined.map_err(|e| format!("download task: {e}"))?,
        _ = async {
            while !stalled.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        } => Err(format!("download stalled — no progress for {STALL_SECS}s, the connection may be blocked")),
    };
    let _ = pump.join();

    match outcome {
        Ok(()) => {
            let _ = app.emit("update-downloaded", ());
            Ok(())
        }
        Err(e) => {
            // RR-9: invalidate the abandoned attempt so a zombie blocking task
            // that finishes later can't flip `downloaded` and arm a stale apply.
            svc_cancel.cancel_inflight_download();
            Err(e)
        }
    }
}

/// Apply the previously-downloaded update + relaunch. Velopack schedules the
/// swap, then we exit the app cleanly so its Update.exe can take the file lock;
/// it relaunches the new version. Only returns on error (the app exits on
/// success).
#[tauri::command]
pub async fn apply_pending_update(
    app: tauri::AppHandle,
    svc: tauri::State<'_, Arc<UpdateService>>,
) -> Result<(), String> {
    log::info!("apply_pending_update: command invoked (frontend → backend OK)");
    let svc = svc.inner().clone();
    tokio::task::spawn_blocking(move || svc.apply(&app))
        .await
        .map_err(|e| format!("apply task: {e}"))?
}
