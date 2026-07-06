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
    /// True once we've logged the current `init_error` at ERROR. A broken-install
    /// / missing-manifest failure won't self-heal mid-session, but the periodic
    /// background check calls `check()` every ~10-30s — without this guard each
    /// poll re-logged the same "updater unavailable" ERROR, flooding rift.log
    /// (645 identical lines over 2 days in the field). Log it once; demote the
    /// repeats to debug. Reset whenever the manager recovers so a genuinely new
    /// failure logs again.
    reason_logged: bool,
    /// Set for the duration of `apply()`. Guards against a second concurrent
    /// call (stale/duplicate frontend invoke, devtools, a second window)
    /// re-entering the child-reap + taskkill + exit sequence while the first
    /// is still in flight.
    applying: bool,
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
            inner: Mutex::new(Inner { mgr, pending: None, downloaded: false, download_epoch: 0, init_error, reason_logged: false, applying: false }),
        }
    }

    /// Check for an update. `Ok(None)` when no source is configured or nothing
    /// newer is available. Velopack's `check_for_updates` is blocking I/O —
    /// call from a `spawn_blocking` context.
    pub fn check(&self) -> Result<Option<UpdateInfoDto>, String> {
        // Don't touch the plan while an apply is scheduled/in-flight. apply()
        // captures its package locally then releases the lock before the blocking
        // swap; a check completing in that window would replace `pending`, and if
        // the apply then FAILS the service is left armed with a plan the user
        // never reviewed. Treat `applying` as a full state lock, not just apply()'s
        // own re-entry guard. (An apply exits the process on success, so this only
        // bites the failed-apply / background-poll edge — harmless to reject.)
        if self.lock().applying {
            return Err("an update is currently being applied".to_string());
        }
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
                            g.reason_logged = false;
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
                            // Log the unavailable-updater reason ONCE at ERROR; a
                            // broken-install failure won't self-heal mid-session, so
                            // the ~10-30s background poll would otherwise flood the
                            // log with identical lines. Repeats go to debug.
                            let first_log = !g.reason_logged || g.init_error.as_deref() != Some(reason.as_str());
                            g.init_error = Some(reason.clone());
                            g.reason_logged = true;
                            if first_log {
                                log::error!("update check: updater unavailable — {reason} (suppressing repeats this session)");
                                crate::diagnostics::emit_with_fields(
                                    crate::diagnostics::DiagStage::System,
                                    crate::diagnostics::DiagLevel::Error,
                                    Some("update"),
                                    Some(file!()),
                                    "updater unavailable",
                                    serde_json::json!({"stage": "init", "reason": reason}),
                                );
                            } else {
                                log::debug!("update check: updater still unavailable — {reason}");
                            }
                            // Distinguish a blocked network from a broken install —
                            // a firewalled R2 feed isn't a reinstall problem.
                            let low = reason.to_ascii_lowercase();
                            let looks_network = ["connect", "dns", "timeout", "timed out", "network", "refused", "unreachable", "resolve"]
                                .iter().any(|k| low.contains(k));
                            return Err(if looks_network {
                                format!("Couldn't reach the update server (the feed host pub-*.r2.dev may be blocked by your network/firewall): {reason}")
                            } else {
                                format!("Rift isn't properly installed for auto-update ({reason}). Reinstall from the latest Setup.exe to restore updates.")
                            });
                        }
                    }
                }
            }
        };
        // Snapshot the epoch as of check-start. The command layer wraps check()
        // in a 30s timeout but leaves an orphaned call running on stall; without
        // this guard that zombie's late completion would unconditionally bump the
        // epoch + reset pending/downloaded, invalidating a newer check's in-flight
        // download (which then discards its own good package as a "zombie"). If the
        // epoch moved while we were blocked, a newer check/download/repair already
        // owns the plan — skip our writes. (download() already self-guards this way.)
        let epoch_at_entry = self.lock().download_epoch;
        match mgr.check_for_updates() {
            // UpdateAvailable wraps a Box<UpdateInfo>; the full target release
            // asset is `TargetFullRelease`.
            Ok(velopack::UpdateCheck::UpdateAvailable(info)) => {
                let asset = &info.TargetFullRelease;
                log::info!("update check: available v{}", asset.Version);
                crate::diagnostics::emit_with_fields(
                    crate::diagnostics::DiagStage::Log,
                    crate::diagnostics::DiagLevel::Info,
                    Some("update"),
                    Some(file!()),
                    "update available",
                    serde_json::json!({"stage": "check", "ok": true, "version": asset.Version}),
                );
                let dto = UpdateInfoDto {
                    version: asset.Version.clone(),
                    release_name: asset.FileName.clone(),
                    size_bytes: asset.Size,
                    notes_markdown: asset.NotesMarkdown.clone(),
                };
                let mut g = self.lock();
                // Stale-check guard: a newer check/download/repair superseded us
                // while we were blocked — return the (discarded) result without
                // clobbering the current plan or invalidating its download.
                if g.download_epoch != epoch_at_entry {
                    log::debug!("update check: result superseded (epoch {} != {epoch_at_entry}), not applying", g.download_epoch);
                    return Ok(Some(dto));
                }
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
                // Stale-check guard (see UpdateAvailable arm) — an orphaned check
                // completing late must not wipe a newer plan/download.
                if g.download_epoch != epoch_at_entry {
                    log::debug!("update check: up-to-date result superseded (epoch {} != {epoch_at_entry}), not applying", g.download_epoch);
                    return Ok(None);
                }
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
                crate::diagnostics::emit_with_fields(
                    crate::diagnostics::DiagStage::Log,
                    crate::diagnostics::DiagLevel::Warn,
                    Some("update"),
                    Some(file!()),
                    "update check failed",
                    serde_json::json!({"stage": "check", "ok": false}),
                );
                // Mirror the init-failure classification (above) + add TLS/cert
                // keywords: a corporate TLS-intercepting proxy (Zscaler etc.)
                // presents a CA the Velopack client's bundled roots don't trust,
                // so the raw error reads as an opaque cert failure. Name the cause.
                let raw = e.to_string();
                let low = raw.to_ascii_lowercase();
                let looks_network = [
                    "connect", "dns", "timeout", "timed out", "network", "refused",
                    "unreachable", "resolve", "certificate", "tls", "ssl", "verify",
                ]
                .iter()
                .any(|k| low.contains(k));
                if looks_network {
                    Err(format!("Couldn't reach the update server — check your network/firewall, or a TLS-intercepting proxy may be blocking the feed host pub-*.r2.dev: {raw}"))
                } else {
                    Err(format!("check_for_updates: {raw}"))
                }
            }
        }
    }

    /// Download the pending update package, streaming `0..=100` progress ticks
    /// through `progress`. Call `check` first to populate the pending plan.
    /// Blocking I/O — must run on `spawn_blocking`.
    pub fn download(&self, progress: Sender<i16>) -> Result<(), String> {
        // Reject while an apply is scheduled/in-flight — see check()'s guard.
        if self.lock().applying {
            return Err("an update is currently being applied".to_string());
        }
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
            crate::diagnostics::emit_with_fields(
                crate::diagnostics::DiagStage::Log,
                crate::diagnostics::DiagLevel::Warn,
                Some("update"),
                Some(file!()),
                "download superseded",
                serde_json::json!({"stage": "download_superseded"}),
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
        // Reject while an apply is scheduled/in-flight — see check()'s guard.
        // Repair replaces `pending` with an IsDowngrade plan; doing that mid-apply
        // is the worst case (a failed apply would leave a silent downgrade armed).
        if self.lock().applying {
            return Err("an update is currently being applied".to_string());
        }
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
            let mut g = self.lock();
            if g.applying {
                return Err("update already applying".to_string());
            }
            let mgr = g.mgr.clone().ok_or_else(|| "no update source configured".to_string())?;
            let info = g
                .pending
                .clone()
                .ok_or_else(|| "no pending update — call check first".to_string())?;
            if !g.downloaded {
                return Err("update not downloaded — call download_update first".to_string());
            }
            g.applying = true;
            (mgr, info)
        };
        // silent = true (no Velopack UI → unattended), restart = true (relaunch
        // after swap), no extra restart args.
        log::info!("update apply: scheduling swap for v{}", info.TargetFullRelease.Version);
        crate::diagnostics::emit_with_fields(
            crate::diagnostics::DiagStage::Log,
            crate::diagnostics::DiagLevel::Info,
            Some("update"),
            Some(file!()),
            "apply started",
            serde_json::json!({"stage": "apply", "version": info.TargetFullRelease.Version}),
        );
        if let Err(e) = mgr.wait_exit_then_apply_updates(&info, true, true, Vec::<&str>::new()) {
            log::error!("update apply FAILED: {e}");
            self.lock().applying = false;
            return Err(format!("wait_exit_then_apply_updates: {e}"));
        }
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
            // /T so the kill also reaps grandchildren (e.g. a git subprocess of
            // an orphaned MCP child) — they aren't named rift-tauri.exe but hold
            // inherited handles into current/ that block the Velopack rename.
            fn kill_pid_tree(pid: u32) {
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .creation_flags(CREATE_NO_WINDOW)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
            }
            let me = std::process::id();
            // Image name derived from the running exe — a hardcoded name breaks
            // the sweep (and silently blocks the swap) if the binary is renamed.
            let exe_path = std::env::current_exe().ok();
            let image = exe_path
                .as_ref()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "rift-tauri.exe".to_string());
            // Scope the sweep to processes running THIS exe file. The dev binary
            // and the installed app share the image NAME (Velopack per-user
            // install), so the old blanket `IMAGENAME eq` + `PID ne me` sweep
            // could kill an unrelated live Rift (a dev-build apply() murdering
            // the real app mid-session). Orphaned MCP children — the current/
            // lock-holders this backstop exists for — always run MY exe, so
            // full-path equality keeps the swap-unblocking guarantee. A second
            // instance of the SAME install still dies, which is required anyway:
            // it holds current/ locked and would block the rename regardless.
            let my_exe_lower = exe_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_ascii_lowercase());
            let swept: Option<usize> = my_exe_lower.as_ref().and_then(|my_exe| {
                let script = format!(
                    "Get-CimInstance Win32_Process -Filter \"Name='{}'\" | ForEach-Object {{ \"$($_.ProcessId)|$($_.ExecutablePath)\" }}",
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
                let mut killed = 0usize;
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let Some((pid_s, path)) = line.trim().split_once('|') else { continue };
                    let Ok(pid) = pid_s.trim().parse::<u32>() else { continue };
                    if pid == me {
                        continue;
                    }
                    // Null/inaccessible ExecutablePath → can't prove it's ours →
                    // don't kill (a foreign instance is worse than a retried swap).
                    if path.trim().to_ascii_lowercase() != *my_exe {
                        continue;
                    }
                    kill_pid_tree(pid);
                    killed += 1;
                }
                Some(killed)
            });
            match swept {
                Some(n) => {
                    log::info!("update apply: path-scoped sweep reaped {n} same-install process(es)");
                }
                None => {
                    // Enumeration unavailable (powershell/CIM broken) — fall back
                    // to the old image-name sweep: on a box that degraded, an
                    // unswept lock-holder silently no-ops the swap (the original
                    // "can't update from the app" bug), which is the worse evil.
                    log::warn!(
                        "update apply: process enumeration failed — falling back to image-name taskkill sweep"
                    );
                    match std::process::Command::new("taskkill")
                        .args([
                            "/F",
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
                        Err(e) => {
                            log::warn!(
                                "update apply: taskkill sweep failed to run ({e}) — a live MCP child may block the swap"
                            );
                            crate::diagnostics::emit_with_fields(
                                crate::diagnostics::DiagStage::System,
                                crate::diagnostics::DiagLevel::Error,
                                Some("update"),
                                Some(file!()),
                                "apply sweep failed",
                                serde_json::json!({"stage": "apply_sweep", "ok": false}),
                            );
                        }
                        Ok(s) if !s.success() => log::info!(
                            "update apply: taskkill sweep exited {s} (no matching children is benign)"
                        ),
                        Ok(_) => {}
                    }
                }
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
