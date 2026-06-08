//! Auto-update service — Velopack (v0.4.47+, restored 2026-06-04).
//!
//! Wraps `velopack::UpdateManager` over the crate's native `GithubSource`
//! (v1.x). The old hand-rolled `GithubSource` (~200 lines against the GitHub
//! REST API) existed only because velopack 0.0.1298 had none — gone now.
//!
//! Flow: `check` → `download` (streams 0..=100 progress) → `apply`. `apply`
//! uses `wait_exit_then_apply_updates(silent, restart)` and then exits the app
//! cleanly via Tauri, so Velopack's Update.exe waits for our PID to die before
//! swapping files + relaunching. This is the GUI-correct path — the old code
//! called `apply_updates_and_restart`, which exits the process from under
//! Tauri and raced WebView2's child handles (the historical flaky-apply bug).
//!
//! Source resolution:
//!   1. RIFT_UPDATE_FEED env var → local FileSource (offline dev / testing),
//!      gated behind `debug_assertions` so a release binary can't be pointed at
//!      an attacker-controlled local feed.
//!   2. GithubSource against the public `rift-releases` repo (production).
//!
//! `UpdateService` is a managed Tauri singleton so the pending `UpdateInfo`
//! survives between the `check`/`download` and `apply` commands.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use velopack::sources::GithubSource;
use velopack::{UpdateInfo, UpdateManager};

/// Public release repo. Source repo is private; releases publish here so
/// unauthenticated GithubSource fetches succeed without exposing source.
const GITHUB_REPO_URL: &str = "https://github.com/Blazzer10200/rift-releases";
/// Alpha/beta tags are eligible for the "newest" pick (mirrors the WPF
/// GithubSource(prerelease:true) call site + the `--pre` upload flag).
const ALLOW_PRERELEASE: bool = true;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub release_name: String,
    pub size_bytes: u64,
    pub notes_markdown: String,
}

struct Inner {
    mgr: Option<UpdateManager>,
    /// Held between `check`/`download` and `apply` so the same plan is reused
    /// without a second roundtrip + re-download.
    pending: Option<UpdateInfo>,
    /// True only after `download_updates` succeeds; guards `apply` from running
    /// without a downloaded package.
    downloaded: bool,
    /// Why `mgr` is None, if it is. A failed `UpdateManager::new` (e.g. Velopack
    /// "not properly installed: could not auto-locate app manifest" — a
    /// corrupted/hand-modified install) leaves no manager. We keep the reason so
    /// `check` can surface "updater unavailable — reinstall" instead of a false
    /// "up to date" that hides a dead updater.
    init_error: Option<String>,
}

pub struct UpdateService {
    inner: Mutex<Inner>,
}

impl UpdateService {
    pub fn new() -> Self {
        let (mgr, init_error) = match resolve_manager() {
            Ok(Some(m)) => (Some(m), None),
            Ok(None) => (None, Some("no update source configured".to_string())),
            Err(e) => {
                log::warn!("UpdateService init: {e}");
                (None, Some(e))
            }
        };
        Self {
            inner: Mutex::new(Inner { mgr, pending: None, downloaded: false, init_error }),
        }
    }

    /// Check for an update. `Ok(None)` when no source is configured or nothing
    /// newer is available. Velopack's `check_for_updates` is blocking I/O —
    /// call from a `spawn_blocking` context.
    pub fn check(&self) -> Result<Option<UpdateInfoDto>, String> {
        // Clone the manager out under a SHORT lock, then release it before the
        // blocking GitHub call. Holding the mutex across the network roundtrip
        // (which has no timeout) wedges every later command behind a single hung
        // check — the "click Check and it spins forever" bug. `download()`/
        // `apply()` already clone out; `check()` must match.
        let mgr = {
            let mut g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
            match g.mgr.as_ref() {
                Some(m) => m.clone(),
                None => {
                    // No manager — the install layout was unreadable at startup
                    // (corrupted/hand-modified Velopack install: manifest missing).
                    // Retry once in case the environment recovered; otherwise
                    // surface a clear, actionable error rather than a false "up to
                    // date" that silently hides a dead updater (the "it says I'm
                    // current but never updates" failure on a broken install).
                    match resolve_manager() {
                        Ok(Some(m)) => {
                            log::info!("update check: manager recovered on retry");
                            g.mgr = Some(m.clone());
                            g.init_error = None;
                            m
                        }
                        recheck => {
                            let reason = match recheck {
                                Err(e) => e,
                                _ => g
                                    .init_error
                                    .clone()
                                    .unwrap_or_else(|| "update source unavailable".to_string()),
                            };
                            g.init_error = Some(reason.clone());
                            log::error!("update check: updater unavailable — {reason}");
                            return Err(format!(
                                "Rift isn't properly installed for auto-update ({reason}). \
                                 Reinstall from the latest Setup.exe to restore updates."
                            ));
                        }
                    }
                }
            }
        };
        match mgr.check_for_updates() {
            // UpdateAvailable wraps a Box<UpdateInfo>; the full target release
            // asset is `TargetFullRelease`.
            Ok(velopack::UpdateCheck::UpdateAvailable(info)) => {
                let asset = &info.TargetFullRelease;
                log::info!("update check: available v{}", asset.Version);
                let dto = UpdateInfoDto {
                    version: asset.Version.clone(),
                    release_name: asset.FileName.clone(),
                    size_bytes: asset.Size,
                    notes_markdown: asset.NotesMarkdown.clone(),
                };
                let mut g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
                g.pending = Some(*info);
                g.downloaded = false;
                Ok(Some(dto))
            }
            Ok(_) => {
                log::info!("update check: up to date");
                let mut g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
                g.pending = None;
                g.downloaded = false;
                Ok(None)
            }
            Err(e) => {
                // Network/parse failures are surfaced so the UI can show an
                // error card rather than a false "up to date".
                log::error!("update check FAILED: {e}");
                Err(format!("check_for_updates: {e}"))
            }
        }
    }

    /// Download the pending update package, streaming `0..=100` progress ticks
    /// through `progress`. Call `check` first to populate the pending plan.
    /// Blocking I/O — must run on `spawn_blocking`.
    pub fn download(&self, progress: Sender<i16>) -> Result<(), String> {
        // Clone out under lock so the mutex isn't held across blocking I/O.
        let (mgr, info) = {
            let g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
            let mgr = g.mgr.clone().ok_or_else(|| "no update source configured".to_string())?;
            let info = g
                .pending
                .clone()
                .ok_or_else(|| "no pending update — call check first".to_string())?;
            (mgr, info)
        };
        log::info!("update download: starting v{}", info.TargetFullRelease.Version);
        mgr.download_updates(&info, Some(progress))
            .map_err(|e| {
                log::error!("update download FAILED: {e}");
                format!("download_updates: {e}")
            })?;
        log::info!("update download: complete v{}", info.TargetFullRelease.Version);
        // Mark downloaded under lock so apply() can guard against skipped download.
        let mut g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
        g.downloaded = true;
        Ok(())
    }

    /// Schedule the downloaded update to apply once this process exits, then
    /// exit Tauri cleanly. Velopack's Update.exe waits (≤60s) for our PID,
    /// swaps files silently, and relaunches. Only returns `Err` — on success
    /// the app exits. Caller passes the `AppHandle` so we trigger Tauri's own
    /// shutdown (so WebView2 children unwind in order, no file lock).
    pub fn apply(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let (mgr, info) = {
            let g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
            let mgr = g.mgr.clone().ok_or_else(|| "no update source configured".to_string())?;
            let info = g
                .pending
                .clone()
                .ok_or_else(|| "no pending update — call check first".to_string())?;
            if !g.downloaded {
                return Err("update not downloaded — call download_update first".to_string());
            }
            (mgr, info)
        };
        // silent = true (no Velopack UI → unattended), restart = true (relaunch
        // after swap), no extra restart args.
        log::info!("update apply: scheduling swap for v{}", info.TargetFullRelease.Version);
        mgr.wait_exit_then_apply_updates(&info, true, true, Vec::<&str>::new())
            .map_err(|e| {
                log::error!("update apply FAILED: {e}");
                format!("wait_exit_then_apply_updates: {e}")
            })?;
        log::info!("update apply: swap scheduled, reaping children + exiting");

        // Velopack's Update.exe waits only for THIS (main) PID, then renames
        // `current/`. But each per-turn `claude` CLI spawns a `rift-tauri.exe`
        // MCP child (`RIFT_MCP_SERVER=1`), and any live `rift-tauri.exe` holds an
        // exclusive lock on `current/` (a rename fails with a sharing violation).
        // `app.exit(0)` below is `std::process::exit` — it skips `Drop`, so
        // `kill_on_drop` never reaps those children. Left alive they block the
        // swap and the update silently no-ops (the "can't update from the app"
        // bug). Reap them before exiting: tracked claude trees first (so none can
        // respawn an MCP child in the exit window), then any stray
        // `rift-tauri.exe` MCP child orphaned from an earlier turn.
        crate::assistant::kill_all_session_children();
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let me = std::process::id();
            let _ = std::process::Command::new("taskkill")
                .args([
                    "/F",
                    "/FI",
                    "IMAGENAME eq rift-tauri.exe",
                    "/FI",
                    &format!("PID ne {me}"),
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }

        // Hand control to Velopack: exit so Update.exe can take the file lock.
        app.exit(0);
        Ok(())
    }
}

impl Default for UpdateService {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_manager() -> Result<Option<UpdateManager>, String> {
    // Local FileSource (RIFT_UPDATE_FEED) is dev-only — gated behind
    // `debug_assertions` so a release build can't be tricked into pointing at
    // an attacker-controlled local feed via env var.
    #[cfg(debug_assertions)]
    if let Ok(local) = std::env::var("RIFT_UPDATE_FEED") {
        let p = std::path::PathBuf::from(&local);
        if let Ok(canon) = p.canonicalize() {
            if canon.is_absolute() && canon.is_dir() {
                let src = velopack::sources::FileSource::new(&canon);
                return UpdateManager::new(src, None, None)
                    .map(Some)
                    .map_err(|e| format!("UpdateManager(local feed): {e}"));
            }
        }
    }
    // `None` access token → unauthenticated (60 req/hr per IP, fine for the
    // launch + 6h-poll cadence).
    let src = GithubSource::new(GITHUB_REPO_URL, None, ALLOW_PRERELEASE);
    UpdateManager::new(src, None, None)
        .map(Some)
        .map_err(|e| format!("UpdateManager(github): {e}"))
}
