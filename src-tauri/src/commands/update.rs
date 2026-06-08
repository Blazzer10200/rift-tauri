//! Updater command surface — Velopack (v0.4.47+, restored 2026-06-04).
//!
//! Thin Tauri wrappers over `crate::update_service::UpdateService` (a managed
//! singleton). Flow: `check_for_updates` → `download_update` (streams
//! `update-progress` 0..=100, then `update-downloaded`) → `apply_pending_update`
//! (schedules the swap, exits the app; Velopack relaunches the new version).
//!
//! All `UpdateManager` calls are blocking I/O, so each runs on `spawn_blocking`.
//! See `update_service.rs` + `docs/design/velopack-auto-update.md`.

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

    let svc = svc.inner().clone();
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
        Err(e) => Err(e),
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
    let svc = svc.inner().clone();
    tokio::task::spawn_blocking(move || svc.apply(&app))
        .await
        .map_err(|e| format!("apply task: {e}"))?
}
