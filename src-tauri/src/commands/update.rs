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
    let svc = svc.inner().clone();
    tokio::task::spawn_blocking(move || svc.check())
        .await
        .map_err(|e| format!("update check task: {e}"))?
}

/// Download the pending update package. Emits `update-progress` (i16 0..=100)
/// as bytes arrive, then `update-downloaded` on success.
#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    svc: tauri::State<'_, Arc<UpdateService>>,
) -> Result<(), String> {
    use tauri::Emitter;
    let svc = svc.inner().clone();
    let (tx, rx) = std::sync::mpsc::channel::<i16>();

    // Forward Velopack's progress ticks to the frontend on a side thread so the
    // blocking download can stream while events flow.
    let pump_app = app.clone();
    let pump = std::thread::spawn(move || {
        while let Ok(pct) = rx.recv() {
            let _ = pump_app.emit("update-progress", pct);
        }
    });

    let result = tokio::task::spawn_blocking(move || svc.download(tx)).await;
    let _ = pump.join();
    let result = result
        .map_err(|e| format!("download task: {e}"))?;

    match result {
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
