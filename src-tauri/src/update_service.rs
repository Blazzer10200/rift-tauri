//! Auto-update service — Velopack (`velopack` crate pinned `=1.2.0`; restored
//! into Rift at app v0.4.47 on 2026-06-04). The vpk CLI version MUST match the
//! crate version — bump both together (see project CLAUDE.md).
//!
//! Wraps `velopack::UpdateManager` over a Velopack `HttpSource` pointed at the
//! Cloudflare R2 feed (`UPDATE_FEED_URL`). The old hand-rolled `GithubSource`
//! (~200 lines against the GitHub REST API) existed only because velopack
//! 0.0.1298 had none — gone now.
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
//!   2. HttpSource against the Cloudflare R2 feed (`UPDATE_FEED_URL`, production).
//!
//! `UpdateService` is a managed Tauri singleton so the pending `UpdateInfo`
//! survives between the `check`/`download` and `apply` commands.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use velopack::sources::HttpSource;
use velopack::{UpdateInfo, UpdateManager};

/// Self-hosted update feed (Cloudflare R2 public bucket). The bridge release
/// shipping this is the first client to read R2 instead of the GitHub repo.
const UPDATE_FEED_URL: &str = "https://pub-4fb26c0fc8df484488e4415f112f2d28.r2.dev";

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
    /// Monotonic download-attempt counter. Bumped by `cancel_inflight_download`
    /// when the command-layer stall watchdog abandons a wedged transfer. A
    /// `download()` whose captured epoch no longer matches at completion is a
    /// zombie (the watchdog already returned an error to the user) and MUST NOT
    /// flip `downloaded` — else a retry could `apply` a package the user
    /// believes failed. (RR-9)
    download_epoch: u64,
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
    /// Lock `inner`, recovering a poisoned guard rather than dead-ending. The
    /// guarded `Inner` is plain data (an Option<UpdateManager> clone-handle,
    /// two flags, an epoch counter) — a panic while some other path held the
    /// lock can't leave that data half-written in a way that matters here. The
    /// old `.map_err(|_| "update mutex poisoned")?` turned a one-time panic into
    /// a permanently bricked updater (every later check/download/apply returned
    /// the same poison error). `into_inner()` keeps the updater alive. (#31)
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

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
            inner: Mutex::new(Inner { mgr, pending: None, downloaded: false, download_epoch: 0, init_error }),
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
            let mut g = self.lock();
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
                let mut g = self.lock();
                // Bump the epoch so any download still in flight against a prior
                // plan can't later flip `downloaded=true` against THIS pending —
                // same race arm_repair guards (RR2). Replacing `pending` while an
                // older transfer is mid-flight would otherwise arm apply() with a
                // package that doesn't match the new plan.
                g.download_epoch = g.download_epoch.wrapping_add(1);
                g.pending = Some(*info);
                g.downloaded = false;
                Ok(Some(dto))
            }
            Ok(_) => {
                log::info!("update check: up to date");
                let mut g = self.lock();
                // RR8: bump the epoch here too (third arm — mirrors UpdateAvailable
                // + arm_repair). If a download is in flight when the feed flips to
                // "up to date" (yanked version), an un-bumped epoch lets the zombie
                // download later set downloaded=true against pending=None — a state
                // that makes apply() fail with a confusing "no pending update".
                g.download_epoch = g.download_epoch.wrapping_add(1);
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
        // Self-heal: if there's no pending plan (cleared/never set), re-check
        // before giving up — the user sees an "available" UI, so a missing
        // pending should resolve transparently, not dead-end the download.
        let needs_recheck = {
            let g = self.lock();
            g.pending.is_none()
        };
        if needs_recheck {
            log::info!("update download: no pending plan — re-checking first");
            // If the re-check resolves to "up to date" (Ok(None)) the plan stays
            // empty; surface a distinct message rather than dead-ending on the
            // generic "no pending update" guard below, which is indistinguishable
            // from the uninitialized-manager case. (RR6)
            if self.check()?.is_none() {
                return Err(
                    "no update available — the feed shows the installed version is current"
                        .to_string(),
                );
            }
        }
        // Clone out under lock so the mutex isn't held across blocking I/O.
        // Capture the epoch now; if it's been bumped by the time we finish,
        // this attempt was abandoned (stall) and must not flip `downloaded`.
        let (mgr, info, epoch) = {
            let g = self.lock();
            let mgr = g.mgr.clone().ok_or_else(|| "no update source configured".to_string())?;
            let info = g
                .pending
                .clone()
                .ok_or_else(|| "no pending update — nothing newer available".to_string())?;
            (mgr, info, g.download_epoch)
        };
        log::info!("update download: starting v{}", info.TargetFullRelease.Version);
        mgr.download_updates(&info, Some(progress))
            .map_err(|e| {
                log::error!("update download FAILED: {e}");
                format!("download_updates: {e}")
            })?;
        log::info!("update download: complete v{}", info.TargetFullRelease.Version);
        // Mark downloaded under lock so apply() can guard against skipped download.
        let mut g = self.lock();
        if g.download_epoch != epoch {
            // Watchdog gave up on this transfer and the user already saw a stall
            // error — discard the zombie result rather than arm apply(). (RR-9)
            log::warn!(
                "update download: superseded (epoch {epoch} != {}) — discarding zombie result",
                g.download_epoch
            );
            return Ok(());
        }
        g.downloaded = true;
        Ok(())
    }

    /// Force the pending plan to the LATEST full release in the feed, regardless
    /// of whether it's newer than the installed version. This is the "Repair
    /// installation" path — it re-fetches and re-applies the current release to
    /// overwrite corrupted/half-written binaries. Velopack's normal `check`
    /// returns `NoUpdateAvailable` when remote == installed, so we bypass it:
    /// pull the release feed, pick the newest `Full` asset, and build an
    /// `UpdateInfo` for it via serde (the public ctor is crate-private). After
    /// this, the regular `download()` → `apply()` chain reinstalls it.
    pub fn arm_repair(&self) -> Result<UpdateInfoDto, String> {
        let mgr = {
            let g = self.lock();
            match g.mgr.as_ref() {
                Some(m) => m.clone(),
                None => {
                    return Err(g
                        .init_error
                        .clone()
                        .map(|e| format!(
                            "Rift isn't properly installed for auto-update ({e}). \
                             Reinstall from the latest Setup.exe to repair."
                        ))
                        .unwrap_or_else(|| "update source unavailable".to_string()))
                }
            }
        };
        let feed = mgr.get_release_feed().map_err(|e| format!("repair: release feed: {e}"))?;
        // Newest "Full" asset by version (matches check_for_updates' own
        // selection). Lightweight numeric compare on the dotted X.Y.Z core —
        // avoids pulling in a semver dep just to rank the feed.
        fn ver_key(v: &str) -> (u64, u64, u64, u8) {
            let core = v.split(['-', '+']).next().unwrap_or(v);
            let mut it = core.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
            // 4th element: stable (no pre-release tag) outranks a pre-release of
            // the same X.Y.Z, so 1.2.0 wins over 1.2.0-beta on a tie.
            let is_stable = u8::from(!v.contains('-'));
            (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0), is_stable)
        }
        let latest = feed
            .Assets
            .iter()
            .filter(|a| a.Type.eq_ignore_ascii_case("Full"))
            .max_by_key(|a| ver_key(&a.Version))
            .ok_or_else(|| "repair: no full release found in feed".to_string())?
            .clone();
        log::info!("repair: arming reinstall of full release v{}", latest.Version);
        let dto = UpdateInfoDto {
            version: latest.Version.clone(),
            release_name: latest.FileName.clone(),
            size_bytes: latest.Size,
            notes_markdown: latest.NotesMarkdown.clone(),
        };
        // The crate-private `UpdateInfo::new_full` ctor is unreachable, but
        // `UpdateInfo` is serde-(de)serializable — round-trip a JSON object with
        // the chosen full release as the target (no base/deltas → forces a full
        // download). `IsDowngrade: true` lets Velopack overwrite a same/lower
        // local version, which is exactly the repair case.
        let target = serde_json::to_value(&latest).map_err(|e| format!("repair: serialize asset: {e}"))?;
        let info_json = serde_json::json!({
            "TargetFullRelease": target,
            "BaseRelease": null,
            "DeltasToTarget": [],
            "IsDowngrade": true,
        });
        let info: UpdateInfo =
            serde_json::from_value(info_json).map_err(|e| format!("repair: build update info: {e}"))?;
        let mut g = self.lock();
        // Bump the epoch so a concurrently in-flight normal download can't later
        // flip `downloaded=true` against the repair plan we're about to set —
        // that would arm an apply with the wrong package on disk. (RR2)
        g.download_epoch = g.download_epoch.wrapping_add(1);
        g.pending = Some(info);
        g.downloaded = false;
        Ok(dto)
    }

    /// Abandon any in-flight download: bump the epoch so a still-running
    /// `download()` task can't later flip `downloaded`, and clear the flag now.
    /// Called by the command layer when its stall watchdog gives up on a wedged
    /// transfer. (RR-9)
    pub fn cancel_inflight_download(&self) {
        let mut g = self.lock();
        g.download_epoch = g.download_epoch.wrapping_add(1);
        g.downloaded = false;
    }

    /// Schedule the downloaded update to apply once this process exits, then
    /// exit Tauri cleanly. Velopack's Update.exe waits (≤60s) for our PID,
    /// swaps files silently, and relaunches. Only returns `Err` — on success
    /// the app exits. Caller passes the `AppHandle` so we trigger Tauri's own
    /// shutdown (so WebView2 children unwind in order, no file lock).
    pub fn apply(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let (mgr, info) = {
            let g = self.lock();
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
        // bug). Reap them before exiting: warm CLI children first, then tracked
        // claude trees (so none can respawn an MCP child in the exit window),
        // then any stray `rift-tauri.exe` MCP child orphaned from an earlier turn.
        //
        // #48 warm pool: each warm child is a parked `claude` process owned by a
        // long-lived reader loop that holds its `tokio::process::Child`. Signal
        // every loop to start_kill + drop its child so the claude tree (and its
        // MCP `rift-tauri.exe` grandchild holding `current/`) goes down before
        // the swap. This is synchronous best-effort; the taskkill sweep below is
        // the backstop for anything that outlives the drain.
        crate::assistant::warm_pool::drain_all_for_shutdown();
        crate::assistant::kill_all_session_children();
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let me = std::process::id();
            // Image name derived from the running exe — a hardcoded name breaks
            // the sweep (and silently blocks the swap) if the binary is renamed.
            let image = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "rift-tauri.exe".to_string());
            match std::process::Command::new("taskkill")
                .args([
                    "/F",
                    // RR9: /T so the sweep also kills grandchildren (e.g. a git
                    // subprocess spawned by an orphaned MCP child). Those aren't
                    // named rift-tauri.exe so the IMAGENAME filter misses them,
                    // but they can hold inherited handles into current/ and
                    // silently block the Velopack rename — the same root cause as
                    // the original child-reap bug. Mirrors kill_all_session_children.
                    "/T",
                    "/FI",
                    &format!("IMAGENAME eq {image}"),
                    "/FI",
                    &format!("PID ne {me}"),
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                Err(e) => log::warn!(
                    "update apply: taskkill sweep failed to run ({e}) — a live MCP child may block the swap"
                ),
                Ok(s) if !s.success() => log::info!(
                    "update apply: taskkill sweep exited {s} (no matching children is benign)"
                ),
                Ok(_) => {}
            }
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
    // Local FileSource (RIFT_UPDATE_FEED) — never compiled into a production
    // release, so a shipped binary can't be pointed at an attacker-controlled
    // local feed. Available in `debug_assertions` (normal `tauri dev`) AND in a
    // release build explicitly opted into the `update-test-feed` feature, which
    // is the only release build that can exercise the full apply chain locally
    // (apply needs a real Velopack `current/` layout — see scripts/test-update.ps1).
    #[cfg(any(debug_assertions, feature = "update-test-feed"))]
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
    let src = HttpSource::new(UPDATE_FEED_URL);
    UpdateManager::new(src, None, None)
        .map(Some)
        .map_err(|e| format!("UpdateManager(http): {e}"))
}
