//! Webview-freeze watchdog (v0.131.0 field incident, 2026-07-19).
//!
//! The WebView2 renderer main thread can wedge permanently — frozen frame,
//! all input dead (not even hover), zero JS — while the Rust side stays
//! perfectly healthy. Observed in prod within ~60s of an update-swap boot;
//! unreproducible in dev, mechanism external to app code (renderer-level
//! block). When it happens the user's only recourse was Task Manager.
//!
//! Recovery: the frontend beats `ui_heartbeat` every [`BEAT_EVERY_MS`] ms from
//! its main thread — a blocked renderer stops beating. A monitor task checks
//! every [`CHECK_EVERY`]: if the MAIN window is focused (focused windows never
//! have their timers throttled, so silence there is proof of death, not
//! power-saving) and the beat has been silent for [`SILENT_AFTER`] across two
//! consecutive checks (double-check absorbs wake-from-sleep races), it
//! recovers in two stages: (1) navigate the main webview to its current URL —
//! commits as soon as the renderer can process it (covers transient wedges);
//! (2) if beats STILL haven't resumed by the next fire (a hard-hung renderer
//! queues that navigation forever), kill this instance's WebView2 renderer
//! process (parent-chain matched, PID-exact) and navigate again — the retry
//! boots a fresh renderer. Conversations restore from disk (flush-saved at
//! send + turn-complete). At most one fire per [`MIN_BETWEEN_RELOADS`].

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::Manager;

/// Frontend beat cadence (AppShell setInterval) — documented here for lockstep.
pub const BEAT_EVERY_MS: u64 = 3_000;
const CHECK_EVERY: Duration = Duration::from_secs(15);
const SILENT_AFTER: Duration = Duration::from_secs(45);
/// Short enough that the two-stage recovery (navigate → renderer-kill +
/// navigate) completes in ~4 min; long enough that a slow reload isn't
/// re-fired mid-boot.
const MIN_BETWEEN_RELOADS: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct WatchdogState {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    /// None until the FE beats once — never fire during boot, before the
    /// heartbeat loop exists.
    last_beat: Option<Instant>,
    /// Consecutive monitor checks that found a focused window silent.
    strikes: u8,
    last_reload: Option<Instant>,
    /// True from a reload-fire until the next real beat. A second fire while
    /// still true means navigate alone couldn't commit (same-process
    /// navigation queues behind a truly hung renderer forever) — escalate to
    /// killing the renderer process so the retry boots a fresh one.
    reload_pending: bool,
}

/// Pure decision — split out so the trigger conditions are unit-testable.
fn should_reload(
    silent_for: Duration,
    focused: bool,
    strikes: u8,
    since_last_reload: Option<Duration>,
) -> bool {
    focused
        && silent_for >= SILENT_AFTER
        && strikes >= 2
        && since_last_reload.is_none_or(|d| d >= MIN_BETWEEN_RELOADS)
}

/// FE main-thread heartbeat. Only the main window feeds the watchdog — a
/// secondary window's beats must not mask a dead main webview.
#[tauri::command]
pub fn ui_heartbeat(window: tauri::WebviewWindow, state: tauri::State<'_, WatchdogState>) {
    if window.label() != "main" {
        return;
    }
    let mut g = state.inner.lock().unwrap_or_else(|e| e.into_inner());
    g.last_beat = Some(Instant::now());
    g.strikes = 0;
    g.reload_pending = false; // a real beat = the page is alive again
}

/// Kill the WebView2 *renderer* process(es) belonging to THIS app instance.
/// Parent-chain matched, PID-exact: our pid → `msedgewebview2.exe` browser
/// child → its `--type=renderer` children. Another Rift instance's renderers
/// hang off a different browser parent and are unreachable from here — this
/// can never cross instances, and nothing is ever matched by image name alone.
fn kill_hung_renderers() -> usize {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    let me = std::process::id();
    let mut killed = 0;
    let browsers: Vec<u32> = sys
        .processes()
        .iter()
        .filter(|(_, p)| {
            p.parent().map(|pp| pp.as_u32()) == Some(me)
                && p.name().to_string_lossy().to_ascii_lowercase().starts_with("msedgewebview2")
        })
        .map(|(pid, _)| pid.as_u32())
        .collect();
    for (pid, p) in sys.processes() {
        let Some(pp) = p.parent().map(|pp| pp.as_u32()) else { continue };
        if !browsers.contains(&pp) {
            continue;
        }
        let is_renderer = p.cmd().iter().any(|a| a.to_string_lossy() == "--type=renderer");
        if is_renderer {
            log::warn!("watchdog: killing hung WebView2 renderer pid {}", pid.as_u32());
            if p.kill() {
                killed += 1;
            }
        }
    }
    killed
}

/// Spawn the monitor loop. Call once from `setup` (state must be managed first).
pub fn spawn(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        log::info!(
            "watchdog: armed (beat {}ms, check {}s, silent-after {}s)",
            BEAT_EVERY_MS,
            CHECK_EVERY.as_secs(),
            SILENT_AFTER.as_secs()
        );
        let mut tick = tokio::time::interval(CHECK_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            // ⚠ `get_webview_window(label)` = None once the window is
            // multi-webview (browser dock) — resolve Window + webview separately.
            let Some(win) = app.get_window("main") else { continue };
            let focused = match win.is_focused() {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("watchdog: is_focused failed: {e}");
                    false
                }
            };
            let state = app.state::<WatchdogState>();
            let reload = {
                let mut g = state.inner.lock().unwrap_or_else(|e| e.into_inner());
                let Some(beat) = g.last_beat else { continue };
                let silent_for = beat.elapsed();
                if silent_for >= Duration::from_secs(20) {
                    log::info!(
                        "watchdog: check silent={}s focused={} strikes={}",
                        silent_for.as_secs(),
                        focused,
                        g.strikes
                    );
                }
                if silent_for < SILENT_AFTER || !focused {
                    g.strikes = 0;
                    continue;
                }
                g.strikes = g.strikes.saturating_add(1);
                log::warn!(
                    "watchdog: heartbeat silent {}s while focused (strike {})",
                    silent_for.as_secs(),
                    g.strikes
                );
                let fire = should_reload(
                    silent_for,
                    focused,
                    g.strikes,
                    g.last_reload.map(|t| t.elapsed()),
                );
                let escalate = fire && g.reload_pending;
                if fire {
                    g.last_reload = Some(Instant::now());
                    // Treat the reload as a fresh beat so the next attempt (if
                    // the reload itself fails to revive the page) waits a full
                    // silence window + cooldown instead of firing every check.
                    g.last_beat = Some(Instant::now());
                    g.strikes = 0;
                    g.reload_pending = true;
                }
                (fire, escalate)
            };
            let (reload, escalate) = reload;
            if escalate {
                // A prior navigate never brought beats back — the renderer is
                // hard-hung and the queued navigation can't commit. Kill it;
                // the navigate below then boots a fresh renderer process.
                let n = kill_hung_renderers();
                log::error!("watchdog: escalation — killed {n} hung renderer process(es)");
                tokio::time::sleep(Duration::from_millis(750)).await;
            }
            if reload {
                // The app webview is labeled "main"; the browser dock (if open)
                // is a sibling child webview — never reload that one.
                let Some(wv) = win.webviews().into_iter().find(|w| w.label() == "main")
                else {
                    log::error!("watchdog: no 'main' webview found on the main window");
                    continue;
                };
                let url = wv.url();
                log::error!(
                    "watchdog: webview heartbeat silent >{}s while focused — renderer presumed hung, reloading {:?}",
                    SILENT_AFTER.as_secs(),
                    url.as_ref().map(|u| u.as_str()).unwrap_or("<unknown url>")
                );
                match url {
                    Ok(u) => {
                        if let Err(e) = wv.navigate(u) {
                            log::error!("watchdog: reload navigate failed: {e}");
                        }
                    }
                    Err(e) => log::error!("watchdog: could not read webview url: {e}"),
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: fn(u64) -> Duration = Duration::from_secs;

    #[test]
    fn fires_only_after_two_focused_silent_strikes() {
        // One strike (first check past the silence threshold) — not yet.
        assert!(!should_reload(S(50), true, 1, None));
        // Second consecutive strike — fire.
        assert!(should_reload(S(65), true, 2, None));
    }

    #[test]
    fn never_fires_unfocused_or_quiet() {
        // Unfocused silence is power-saving/minimized, not proof of death.
        assert!(!should_reload(S(600), false, 10, None));
        // Recently beating — quiet path.
        assert!(!should_reload(S(10), true, 2, None));
    }

    #[test]
    fn honors_reload_cooldown() {
        // A reload 60s ago → hold even if still silent.
        assert!(!should_reload(S(120), true, 3, Some(S(60))));
        // Cooldown elapsed → allowed again.
        assert!(should_reload(S(120), true, 3, Some(S(400))));
    }
}
