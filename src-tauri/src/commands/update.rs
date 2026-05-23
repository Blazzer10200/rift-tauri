//! Velopack updater + app version command surface (#20).

use crate::{update_service, AutoSyncState, TunnelState};

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn check_for_updates(
    svc: tauri::State<'_, std::sync::Arc<update_service::UpdateService>>,
) -> Result<Option<update_service::UpdateInfoDto>, String> {
    let svc = svc.inner().clone();
    tokio::task::spawn_blocking(move || svc.check())
        .await
        .map_err(|e| format!("update check task: {e}"))?
}

/// Download the pending update package. Emits `update-progress` (i16 0..=100)
/// then `update-downloaded` on success.
#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    svc: tauri::State<'_, std::sync::Arc<update_service::UpdateService>>,
) -> Result<(), String> {
    use tauri::Emitter;
    let svc = svc.inner().clone();
    let (tx, rx) = std::sync::mpsc::channel::<i16>();

    let pump_app = app.clone();
    let pump = std::thread::spawn(move || {
        while let Ok(pct) = rx.recv() {
            let _ = pump_app.emit("update-progress", pct);
        }
    });

    let result = tokio::task::spawn_blocking(move || svc.download(tx))
        .await
        .map_err(|e| format!("download task: {e}"))?;

    let _ = pump.join();

    match result {
        Ok(()) => {
            let _ = app.emit("update-downloaded", ());
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Stop autosync + tunnel, then apply previously-downloaded update + relaunch.
/// Velopack's `apply_updates_and_restart` exits on success.
#[tauri::command]
pub async fn apply_pending_update(
    state: tauri::State<'_, AutoSyncState>,
    tunnel_state: tauri::State<'_, TunnelState>,
    svc: tauri::State<'_, std::sync::Arc<update_service::UpdateService>>,
) -> Result<(), String> {
    {
        let mut g = state.0.lock().await;
        if let Some(engine) = g.take() {
            engine.stop().await;
        }
    }
    {
        let mut tg = tunnel_state.0.lock().await;
        if let Some(t) = tg.take() {
            t.stop().await;
        }
    }
    let svc = svc.inner().clone();
    tokio::task::spawn_blocking(move || svc.apply())
        .await
        .map_err(|e| format!("apply task: {e}"))?
}
