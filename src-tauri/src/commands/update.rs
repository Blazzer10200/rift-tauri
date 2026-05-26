//! Updater command surface — `tauri-plugin-updater` backed (migrated from
//! Velopack 2026-05-26; see docs/design/updater-migration.md).
//!
//! State machine across three commands:
//!   1. `check_for_updates` → polls latest.json, stashes `Update` in
//!      `PendingUpdate`, returns metadata DTO.
//!   2. `download_update`   → streams bytes, emits `update-size` (one-shot)
//!      and `update-progress` (i16 0..=100), stashes bytes.
//!   3. `apply_pending_update` → stops autosync + tunnel, hands bytes to
//!      `Update::install` — Tauri auto-exits the process on Windows so the
//!      NSIS Setup.exe can swap the binary and relaunch.
//!
//! The `on_before_exit` hook fires `assistant::kill_child_processes_on_exit`
//! so the NSIS swap doesn't trip on a claude CLI child holding a file handle
//! inside the install dir (#R3 in the migration brief).

use crate::{AutoSyncState, TunnelState};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

const UPDATE_PROGRESS: &str = "update-progress";
const UPDATE_SIZE: &str = "update-size";
const UPDATE_DOWNLOADED: &str = "update-downloaded";

/// Tauri-managed state holding the active `Update` + downloaded bytes
/// between `check_for_updates` / `download_update` / `apply_pending_update`.
#[derive(Default)]
pub struct PendingUpdate {
    inner: Mutex<PendingState>,
}

#[derive(Default)]
struct PendingState {
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub release_name: String,
    /// 0 until download starts — the updater feed doesn't expose Content-Length
    /// before the byte stream begins. Frontend listens for `update-size` to
    /// patch this field at download-start.
    pub size_bytes: u64,
    pub notes_markdown: String,
    pub release_url: String,
    pub published_at: String,
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    pending: tauri::State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfoDto>, String> {
    let result = app
        .updater_builder()
        .on_before_exit(|| crate::assistant::kill_child_processes_on_exit())
        .build()
        .map_err(|e| format!("updater builder: {e}"))?
        .check()
        .await;

    let update = match result {
        Ok(Some(u)) => u,
        Ok(None) => {
            if let Ok(mut g) = pending.inner.lock() {
                *g = PendingState::default();
            }
            return Ok(None);
        }
        Err(e) => {
            // Network errors / no-endpoint / parse failures: log + swallow so
            // the UI banner stays hidden. Matches prior Velopack behavior.
            log::warn!("update check: {e}");
            return Ok(None);
        }
    };

    let download_url = update.download_url.to_string();
    let dto = UpdateInfoDto {
        version: update.version.clone(),
        release_name: filename_from_url(&download_url),
        size_bytes: 0,
        notes_markdown: update.body.clone().unwrap_or_default(),
        release_url: tag_url_from_download_url(&download_url),
        published_at: update.date.map(|d| d.to_string()).unwrap_or_default(),
    };

    {
        let mut g = pending.inner.lock().map_err(|_| "pending mutex poisoned".to_string())?;
        *g = PendingState { update: Some(update), bytes: None };
    }
    Ok(Some(dto))
}

/// Download the pending update bundle. Emits `update-size` (u64 total bytes,
/// one-shot at start) then `update-progress` (i16 0..=100) until complete,
/// then `update-downloaded` on success.
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    pending: tauri::State<'_, PendingUpdate>,
) -> Result<(), String> {
    let update = {
        let g = pending.inner.lock().map_err(|_| "pending mutex poisoned".to_string())?;
        g.update.clone().ok_or_else(|| "no pending update — call check_for_updates first".to_string())?
    };

    let app2 = app.clone();
    let mut downloaded: u64 = 0;
    let mut size_emitted = false;
    let bytes = update
        .download(
            move |chunk_length, content_length| {
                if !size_emitted {
                    if let Some(total) = content_length {
                        let _ = app2.emit(UPDATE_SIZE, total);
                        size_emitted = true;
                    }
                }
                downloaded = downloaded.saturating_add(chunk_length as u64);
                if let Some(total) = content_length {
                    if total > 0 {
                        let pct = ((downloaded as f64 / total as f64) * 100.0).round() as i64;
                        let pct = pct.clamp(0, 100) as i16;
                        let _ = app2.emit(UPDATE_PROGRESS, pct);
                    }
                }
            },
            || {},
        )
        .await
        .map_err(|e| format!("update download: {e}"))?;

    {
        let mut g = pending.inner.lock().map_err(|_| "pending mutex poisoned".to_string())?;
        g.bytes = Some(bytes);
    }
    let _ = app.emit(UPDATE_DOWNLOADED, ());
    Ok(())
}

/// Stop autosync + tunnel, then hand the downloaded bytes to the updater
/// plugin. On Windows the plugin runs the NSIS Setup.exe and auto-exits this
/// process so the file swap can proceed — this command typically does not
/// return on success.
#[tauri::command]
pub async fn apply_pending_update(
    state: tauri::State<'_, AutoSyncState>,
    tunnel_state: tauri::State<'_, TunnelState>,
    pending: tauri::State<'_, PendingUpdate>,
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

    let (update, bytes) = {
        let mut g = pending.inner.lock().map_err(|_| "pending mutex poisoned".to_string())?;
        let update = g.update.clone().ok_or_else(|| "no pending update — call check_for_updates first".to_string())?;
        let bytes = g.bytes.take().ok_or_else(|| "no downloaded bytes — call download_update first".to_string())?;
        (update, bytes)
    };

    // `install` is sync — it spawns Setup.exe and Tauri auto-exits the
    // current process so the swap can proceed (Windows). Wrap in
    // spawn_blocking so the runtime isn't held while NSIS starts.
    tokio::task::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|e| format!("install task: {e}"))?
        .map_err(|e| format!("update install: {e}"))?;
    Ok(())
}

fn filename_from_url(url: &str) -> String {
    url.rsplit(['/', '\\']).next().unwrap_or("update").to_string()
}

/// Map a GitHub asset download URL to its release tag page.
/// `…/releases/download/<tag>/<file>` → `…/releases/tag/<tag>`.
fn tag_url_from_download_url(url: &str) -> String {
    let segs: Vec<&str> = url.split('/').collect();
    if let Some(dl_idx) = segs.iter().position(|s| *s == "download") {
        if dl_idx + 1 < segs.len() && dl_idx > 0 {
            let prefix = segs[..dl_idx].join("/");
            let tag = segs[dl_idx + 1];
            return format!("{prefix}/tag/{tag}");
        }
    }
    url.to_string()
}
