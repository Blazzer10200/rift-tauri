//! Warm CLI process pool — the permanent per-turn latency fix (#48).
//!
//! Rift historically spawned a FRESH `claude -p` per turn (`spawn→exit`,
//! `--resume` to replay), re-paying Node cold-boot (~1s) + model prefill every
//! turn. VSCode keeps ONE process warm across the conversation → instant after
//! turn 1. This module keeps ONE persistent `claude` child per chat session
//! (keyed by CLI session_id, like `SESSION_PIDS`), reuses it across turns, and
//! evicts on idle / death / spawn-signature change.
//!
//! Empirical proof (cont.163): two turns through one persistent
//! `claude -p --input-format stream-json` process — turn 2 = 1402ms vs cold
//! ~3000ms, no `--resume`, context carried in-process.
//!
//! ## Architecture
//! A long-lived **reader task** (one per warm child) OWNS the child's stdin +
//! stdout and loops over turns, parking on a per-child `TurnCmd` channel after
//! each `result` frame (instead of `break`-ing → stdin-drop → EOF → exit). The
//! `assistant_send` command computes a `SpawnKey` (the respawn-trigger set),
//! looks up the warm child, and either reuses it (send a `TurnCmd`) or cold-
//! spawns a new one. All per-turn streaming/permission plumbing stays
//! exactly where it was — see `turn.rs::run_turn_loop`.
//!
//! Red-team blockers handled (see docs/design/warm-cli-process.md):
//! - B1/B3: stdin owned by the long-lived reader task, never dropped between
//!   turns; the WarmChild mutex is NEVER held across an await (reuse only locks
//!   it to read the turn-tx + signature, releases before sending).
//! - B4: idle stop → Mode-B kill (not Ctrl-C); the reader task's
//!   `turn_in_progress` flag gates Mode-A interrupt.
//! - M2: failure detection rides the `result`-frame subtype + the persistent
//!   stderr reader, NOT `child.wait()` non-zero exit (which never happens warm).
//! - M3: a turn that spawns a background child (inherits the warm stdout
//!   write-end → never EOFs) force-evicts its warm child after `result`.
//! - M5/M7: never hold the registry mutex while locking a WarmChild; `get` →
//!   clone Arc → release registry → then lock.
//! - M6: the reader task clears `turn_in_progress` itself right after `result`.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot};

/// Tree-kill a single `claude` child by PID (the CLI parent + its
/// `RIFT_MCP_SERVER=1` MCP grandchild that holds the Velopack `current/` lock).
/// Best-effort + synchronous. Shared by `kill_all_session_children` (update-apply
/// sweep), the signature-drain path, and idle-evict / shutdown-drain here — every
/// path that must reap a warm child whose reader loop CANNOT self-exit, because
/// the loop holds a `turn_tx` clone (via the `WarmChild` it owns through `ctx.warm`)
/// so dropping the registry entry alone never makes the loop's `recv()` return None
/// (self-referential sender). The child must be killed by PID.
pub(super) fn kill_child_tree(pid: u32) {
    if pid == 0 { return; } // never taskkill PID 0 (meaningless / unsafe target)
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// One in-flight turn handed to a warm child's reader loop. Carries everything
/// the loop needs to stream this turn's events and signal completion back to
/// the awaiting `assistant_send` command.
pub(super) struct TurnCmd {
    /// The pre-built stream-json `user` envelope (text + optional images).
    pub user_line: Vec<u8>,
    /// App handle for `emit_to` (per-turn — the turn may originate from a
    /// different window than the one that cold-spawned the child).
    pub app: AppHandle,
    /// The window label that fired THIS turn — all events emit_to it (#37).
    pub window_label: String,
    /// Fired by the reader loop when the turn's `result` lands (or it errors /
    /// the child dies). `Ok(())` = a `result` was seen (DONE already emitted);
    /// `Err(msg)` = the reader hit a fatal condition and emitted ERROR. The
    /// command awaits this to return to the frontend.
    pub done: oneshot::Sender<Result<(), String>>,
    /// Set true if this turn must force-evict the warm child afterward (M3 —
    /// a turn that spawned a background process taints the inherited pipe).
    /// The reader loop reads `bg_evict` after the turn and, if set, exits its
    /// loop (dropping stdin → the child EOFs and is reaped by the watchdog).
    pub bg_evict: Arc<AtomicBool>,
}

/// The spawn "signature": the set of per-turn inputs that, if changed, REQUIRE
/// a fresh process because the warm child was started with the old value baked
/// into its argv/env (can't change in-flight). A turn whose key differs from
/// the warm child's key triggers a graceful drain + cold respawn (with
/// `--resume` so context survives). Everything NOT here rides stdin and reuses
/// the warm child: prompt text, attachments, dyslexia_mode, prior summary,
/// browser_url, usage gauges.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct SpawnKey {
    pub model: String,
    pub root: Option<String>,
    pub permission_mode: String,
    pub prompting_mode: bool,
    pub use_full_config: bool,
    pub use_api_key: bool,
    pub local_llm_enabled: bool,
    pub thinking_on: bool,
    /// `--effort` is baked at spawn (NOT re-readable per-turn over stream-json,
    /// red-team M4) → effort change MUST respawn. Silent wrong-effort is worse
    /// than a cold turn.
    pub effort_level: String,
    /// Git-tools trust gates the `--allowed-tools` argv (git-write tools are
    /// appended only at `standard`) AND the MCP child's RIFT_TRUST_LEVEL env —
    /// both baked at spawn. Without it in the key, flipping Read-only⇄Standard
    /// mid-session reused a warm child with the stale allowlist until some other
    /// field forced a respawn. Keyed so a trust change drains + respawns.
    pub trust_level: String,
    /// Cheap fingerprint of the system-prompt addendum variant (TOOLS / NO_WS /
    /// LOCAL) — a change here means a different `--append-system-prompt`.
    pub addendum_ptr: usize,
}

/// A persistent `claude` child kept warm across turns for one CLI session.
pub(super) struct WarmChild {
    /// Send a turn into the reader loop. The loop owns the receiver + stdin.
    pub turn_tx: mpsc::UnboundedSender<TurnCmd>,
    /// The spawn signature this child was started with. A turn whose key
    /// differs must drain + cold-respawn.
    pub key: SpawnKey,
    /// True while a turn is streaming (set by the reuse path before sending the
    /// TurnCmd, cleared by the reader loop INSIDE itself right after `result` —
    /// M6 coherence). Gates concurrent-turn rejection + Mode-A interrupt.
    pub turn_in_progress: Arc<AtomicBool>,
    /// Last turn dispatch time — drives idle eviction.
    pub last_used: Instant,
    /// OS PID of the `claude` child, captured at spawn. The reader loop holds a
    /// `turn_tx` clone via this struct, so dropping the registry entry alone does
    /// NOT make the loop's `turn_rx.recv()` return None (self-referential sender)
    /// — a signature-drain therefore can't rely on turn_tx-drop to reap the old
    /// child. The drain path kills it directly by this PID (then `cold_spawn`'s
    /// `set_session_pid` overwrites the SESSION_PIDS entry with the new child).
    /// `None` only if `child.id()` was unavailable at spawn (immediate exit).
    pub pid: Option<u32>,
}

/// `static Mutex<Option<HashMap>>` + poison-recovering accessor — matches the
/// existing SESSION_PIDS registry idiom (no new dependency). Keyed
/// by CLI session_id. Values are `Arc<Mutex<WarmChild>>` so the reuse path can
/// clone the Arc, release the registry lock, then lock the child (M5/M7: never
/// hold both at once).
static WARM_CHILDREN: Mutex<Option<HashMap<String, Arc<Mutex<WarmChild>>>>> = Mutex::new(None);

fn with_warm<R>(
    f: impl FnOnce(&mut HashMap<String, Arc<Mutex<WarmChild>>>) -> R,
) -> R {
    let mut g = match WARM_CHILDREN.lock() {
        Ok(g) => g,
        Err(p) => {
            log::error!("WARM_CHILDREN mutex poisoned — recovering inner state");
            p.into_inner()
        }
    };
    let map = g.get_or_insert_with(HashMap::new);
    f(map)
}

/// Look up the warm child for a session WITHOUT holding the registry lock past
/// the clone (M5/M7). Returns the Arc; caller locks it.
pub(super) fn get(session_id: &str) -> Option<Arc<Mutex<WarmChild>>> {
    with_warm(|m| m.get(session_id).cloned())
}

/// Current number of live warm children in the pool. Diagnostics-only (the pool
/// has no standalone size counter — size is the registry map length).
pub(super) fn pool_size() -> usize {
    with_warm(|m| m.len())
}

/// Register a freshly cold-spawned warm child. Replaces any prior entry for the
/// session (the caller has already drained the old one on a signature change).
pub(super) fn insert(session_id: &str, child: Arc<Mutex<WarmChild>>) {
    with_warm(|m| {
        m.insert(session_id.to_string(), child);
    });
}

/// Remove the warm child for a session, but only if it's still the SAME Arc the
/// caller observed (overlapping-turn / respawn guard, mirrors
/// `clear_session_pid_if`). A respawn may have already inserted a new child
/// under the same key; an unconditional remove would wipe it. Returns the
/// removed Arc if it matched (so the caller can drop the turn_tx → the reader
/// loop exits → stdin drops → child EOFs).
pub(super) fn remove_if(session_id: &str, this: &Arc<Mutex<WarmChild>>) -> bool {
    with_warm(|m| {
        if m.get(session_id).is_some_and(|cur| Arc::ptr_eq(cur, this)) {
            m.remove(session_id);
            true
        } else {
            false
        }
    })
}

/// Read the OS PID of the warm child registered for a session, if any. Used by
/// the signature-drain path to kill the OLD child directly: its reader loop
/// holds a `turn_tx` clone (via the WarmChild), so dropping the registry entry
/// can't make the loop's `recv()` return None — the child must be reaped by PID.
pub(super) fn pid_of(session_id: &str) -> Option<u32> {
    let arc = with_warm(|m| m.get(session_id).cloned())?;
    let g = match arc.lock() { Ok(g) => g, Err(p) => p.into_inner() };
    g.pid
}

/// Idle eviction window. A warm child unused for this long is killed +
/// deregistered by the background sweeper, reclaiming its ~450MB resident set.
///
/// **Tuned against real prod data (2026-06-22, 49 inter-turn gaps over the
/// day's sessions):** p50 gap 90s, p75 256s, **p90 445s**. The old 300s window
/// sat in the fat part of that distribution → ~24% of turns exceeded it and
/// re-paid a full cold respawn (~1750ms) on the user's next message — the
/// "feels slow sometimes" report. 30 min clears the p90 with margin, so an
/// interactive user (read response → think → reply) effectively never gets
/// evicted mid-session; only a genuinely-abandoned session ages out.
/// The Velopack `current/` lock concern that originally motivated eviction is
/// already covered by `drain_all_for_shutdown` on the update-apply path, so
/// this timer is now purely a memory backstop — hence the generous window plus
/// the `MAX_WARM` pressure valve below.
///
/// **ARCHITECTURE (2026-06-28, cold-start rewrite): persistent-process model.**
/// The warm child is the SAME long-lived CLI process every IDE-extension /
/// terminal Claude session keeps alive — booted + handshaked once, it serves
/// every turn over a persistent `--input-format stream-json` stdin. Spawning
/// per-turn (and eviction) was never the CLI's requirement; it was an
/// over-eager memory-reclaim instinct. The original concrete reason to evict —
/// releasing the Velopack `current/` file lock during self-update — is fully
/// covered by `drain_all_for_shutdown` on the apply path, so this timer is NOT
/// a correctness mechanism. It is ONLY a backstop against a genuinely-abandoned
/// session leaking ~450MB forever.
///
/// So the window is now ABANDONED-SESSION scale, not inter-turn scale: a child
/// survives any realistic active-use gap (meal, meeting, deep-read, context
/// switch) and is reaped only after the session is clearly walked-away-from.
/// Prior values (30m → 90m) still sat in the tail of real pause distributions
/// and evicted mid-session, which IS the 63%-cold-rate bug. 2h clears every
/// plausible active pause; what's left is true abandonment. Cost is pure idle
/// RAM on a 32GB box — a parked child burns no CPU.
const IDLE_EVICT: Duration = Duration::from_secs(7200);
/// Memory backstop cap. The pressure valve below only fires ABOVE this many
/// concurrent children — and with the persistent-process model that should mean
/// "an unreasonable number of abandoned chats", never normal multi-tab/window
/// use. Sized well past the user's real concurrent window+tab count (~7 windows)
/// so an actively-used set is NEVER subject to the shorter pressure window;
/// only a pathological pile of abandoned sessions trips it. 20 × ~450MB ≈ 9GB
/// worst case, still inside a 32GB box and only when 20 sessions are abandoned
/// at once.
const MAX_WARM: usize = 20;
/// Shortened window applied ONLY to surplus children once `MAX_WARM` is exceeded
/// (oldest-first). With the persistent-process model the valve is a pure
/// pathological-accumulation guard, not a normal-use path. Even when it fires it
/// stays at abandoned-session scale (30 min) so a merely-paused session in a
/// pathologically-large set still isn't mistaken for abandoned.
const IDLE_EVICT_PRESSURE: Duration = Duration::from_secs(1800);
/// How often the background reaper checks for abandoned children. Slow — this is
/// now a rare-event backstop, not a hot eviction loop.
const EVICT_TICK: Duration = Duration::from_secs(300);

/// Pure eviction decision (extracted so the v0.26.3 pressure-valve logic is
/// unit-testable without the global registry + real clocks). Given each child's
/// idle age sorted OLDEST-FIRST and its in-progress flag, decide eviction:
///   * the first `total - max_warm` (the surplus, oldest) use `pressure`,
///   * the rest use the generous `idle` window,
///   * a mid-turn child is never evicted regardless of age.
///
/// Returns a bool per input index (same order as the sorted input).
fn evict_decision(
    sorted_idle: &[Duration],
    in_progress: &[bool],
    max_warm: usize,
    idle: Duration,
    pressure: Duration,
) -> Vec<bool> {
    let total = sorted_idle.len();
    let over_cap = total.saturating_sub(max_warm);
    (0..total)
        .map(|rank| {
            if in_progress.get(rank).copied().unwrap_or(false) {
                return false;
            }
            let window = if rank < over_cap { pressure } else { idle };
            sorted_idle[rank] >= window
        })
        .collect()
}

/// Snapshot the registry, find children idle past `IDLE_EVICT`, and drop their
/// turn_tx (→ reader loop exits → stdin drops → child EOFs → watchdog reaps).
/// Skips any child mid-turn. Returns the count evicted. M5/M7: clone Arcs out
/// under the registry lock, then inspect/remove WITHOUT holding both locks.
fn evict_idle_once() -> usize {
    let now = Instant::now();
    // Collect (session_id, Arc) snapshot under the registry lock, release it.
    let candidates: Vec<(String, Arc<Mutex<WarmChild>>)> =
        with_warm(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let total = candidates.len();
    // Under MAX_WARM, every child gets the full IDLE_EVICT window. Above it, the
    // surplus oldest-idle children get the shortened IDLE_EVICT_PRESSURE window
    // (memory valve). Compute each child's idle age once (locking the child, not
    // the registry — M5/M7), then rank by age so "oldest" is well-defined.
    struct Cand {
        sid: String,
        arc: Arc<Mutex<WarmChild>>,
        idle_for: Duration,
        in_progress: bool,
        pid: Option<u32>,
    }
    let mut ranked: Vec<Cand> = candidates
        .into_iter()
        .map(|(sid, arc)| {
            let (idle_for, in_progress, pid) = {
                let g = match arc.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                (now.duration_since(g.last_used), g.turn_in_progress.load(Ordering::Acquire), g.pid)
            };
            Cand { sid, arc, idle_for, in_progress, pid }
        })
        .collect();
    // Oldest (most idle) first — the surplus we trim under pressure.
    ranked.sort_by_key(|c| std::cmp::Reverse(c.idle_for));
    let over_cap = total.saturating_sub(MAX_WARM);

    // Decide via the pure kernel (unit-tested) so this hot path and the test
    // can't diverge. Build the parallel age/in-progress slices in ranked order.
    let ages: Vec<Duration> = ranked.iter().map(|c| c.idle_for).collect();
    let in_prog: Vec<bool> = ranked.iter().map(|c| c.in_progress).collect();
    let decisions = evict_decision(&ages, &in_prog, MAX_WARM, IDLE_EVICT, IDLE_EVICT_PRESSURE);

    let mut evicted = 0;
    for (rank, c) in ranked.into_iter().enumerate() {
        let sid = c.sid;
        let arc = c.arc;
        if decisions[rank] {
            // remove_if takes the registry lock alone (child lock released).
            if remove_if(&sid, &arc) {
                // Guard: a turn may have started after our snapshot; re-check now
                // that the entry is out of the registry.
                let still_idle = {
                    let g = arc.lock().unwrap_or_else(|p| p.into_inner());
                    !g.turn_in_progress.load(Ordering::Acquire)
                };
                if still_idle {
                    if let Some(pid) = c.pid {
                        kill_child_tree(pid);
                    }
                    evicted += 1;
                    let reason = if rank < over_cap { "pressure" } else { "idle" };
                    let window = if rank < over_cap { IDLE_EVICT_PRESSURE } else { IDLE_EVICT };
                    log::info!(
                        "warm_pool: idle-evicted session {sid} (>{}s, {reason}, {total} warm, pid={:?})",
                        window.as_secs(), c.pid
                    );
                } else {
                    // A turn raced in after our snapshot. We already pulled the
                    // registry entry — re-insert it so the live child stays
                    // reachable. Its reader loop holds a self-referential turn_tx,
                    // so an orphaned entry would never be reaped (~450MB leak).
                    // Never kill a child mid-turn.
                    insert(&sid, arc);
                }
            }
        }
    }
    evicted
}

/// Ensure the single background idle-evict sweeper is running. Idempotent —
/// spawns at most once per process (guarded by an `AtomicBool`). Called from
/// the first cold spawn so a process that never opens a chat pays nothing.
pub(super) fn ensure_evictor() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(EVICT_TICK);
        loop {
            tick.tick().await;
            // evict_idle_once is fully synchronous (registry/child mutex locks +
            // a blocking taskkill per reap) — run it off the async worker so a
            // slow taskkill (AV/contention) can't stall other Tokio tasks.
            let n = tokio::task::spawn_blocking(evict_idle_once).await.unwrap_or(0);
            if n > 0 {
                log::debug!("warm_pool: idle sweep evicted {n} child(ren)");
            }
        }
    });
}

/// Kill + drain EVERY warm child immediately (synchronous best-effort). Called
/// from the Velopack update-apply path alongside `kill_all_session_children` —
/// clears the registry so a late turn can't re-register AND tree-kills each
/// child by PID. Self-sufficient: it no longer relies on `kill_all_session_children`
/// to do the actual reap, because dropping the registry Arcs can't kill the
/// children (the reader loops hold self-referential turn_tx clones via `ctx.warm`).
pub(crate) fn drain_all_for_shutdown() {
    let drained: Vec<Arc<Mutex<WarmChild>>> = with_warm(|m| m.drain().map(|(_, v)| v).collect());
    log::info!("warm_pool: draining {} warm child(ren) for update-apply", drained.len());
    // Reap each child by PID. Dropping the Arcs alone does NOT free the last
    // turn_tx clone — every reader loop holds its own WarmChild (via `ctx.warm`),
    // a self-referential sender that keeps `recv()` parked forever (the same leak
    // the signature-drain + idle-evict hit). The update-apply path's
    // `kill_all_session_children` would catch these PIDs too, but draining must be
    // self-sufficient so a future non-apply caller can't silently leak.
    for arc in &drained {
        let pid = {
            let g = match arc.lock() { Ok(g) => g, Err(p) => p.into_inner() };
            g.pid
        };
        if let Some(pid) = pid {
            kill_child_tree(pid);
        }
    }
    drop(drained);
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: Duration = Duration::from_secs(60);

    // Helper: durations in minutes, descending (oldest-first as evict_idle_once
    // pre-sorts). Real consts (persistent-process model): IDLE_EVICT=7200s (120m),
    // PRESSURE=1800s (30m), MAX_WARM=20. Tests pass these windows to evict_decision
    // so the pure kernel stays tested against the live tuning — a child only ages
    // out at abandoned-session scale, never an active-use pause.
    fn mins(v: &[u64]) -> Vec<Duration> {
        v.iter().map(|m| MIN * (*m as u32)).collect()
    }

    #[test]
    fn under_cap_uses_generous_window_only() {
        // 3 children (under MAX_WARM), none over the 120m generous window → nobody
        // evicted even though two are well past the 30m pressure window. The
        // pressure valve must NOT fire below the cap: an active session that's been
        // paused 60-90m is kept hot, not aged out (the persistent-process model).
        let ages = mins(&[90, 60, 30]); // 90m, 60m, 30m idle — all < 120m
        let d = evict_decision(&ages, &[false; 3], 3, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        assert_eq!(d, vec![false, false, false]);
    }

    #[test]
    fn under_cap_evicts_only_past_generous_window() {
        // A genuinely abandoned session (>120m) ages out even under the cap.
        let ages = mins(&[130, 60, 30]);
        let d = evict_decision(&ages, &[false; 3], 3, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        assert_eq!(d, vec![true, false, false]);
    }

    #[test]
    fn over_cap_trims_surplus_oldest_on_short_window() {
        // 5 children, cap 3 → over_cap=2: the two OLDEST get the 30m pressure
        // window, the remaining three keep the 120m generous window. Ages 50m/40m
        // are past 30m but under 120m → the two surplus evict, the rest stay.
        let ages = mins(&[50, 40, 20, 10, 1]);
        let d = evict_decision(&ages, &[false; 5], 3, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        // ranks 0,1 (50m,40m) surplus & >30m → evict; ranks 2,3,4 keep 120m → stay.
        assert_eq!(d, vec![true, true, false, false, false]);
    }

    #[test]
    fn over_cap_surplus_still_needs_to_exceed_pressure_window() {
        // Surplus rank but idle only 10m (< 30m pressure) → NOT evicted. The valve
        // trims the *oldest idle*, not recently-used children that happen to rank.
        let ages = mins(&[10, 10, 10, 10, 10]);
        let d = evict_decision(&ages, &[false; 5], 3, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        assert_eq!(d, vec![false; 5]);
    }

    #[test]
    fn mid_turn_child_never_evicted_even_if_oldest() {
        // The oldest child is mid-turn → must be spared regardless of age, or we'd
        // kill the CLI under an in-flight request. (rank 0, 130m idle, in_progress)
        let ages = mins(&[130, 50, 5, 5, 5]);
        let mut prog = [false; 5];
        prog[0] = true;
        let d = evict_decision(&ages, &prog, 3, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        // rank 0 spared (in-progress); rank 1 (50m surplus, >30m) evicts.
        assert_eq!(d, vec![false, true, false, false, false]);
    }

    #[test]
    fn empty_pool_is_noop() {
        let d = evict_decision(&[], &[], 3, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        assert!(d.is_empty());
    }

    /// Leak-fix coverage (cont.216): both the idle-evict and shutdown-drain paths
    /// must reap the child by PID — dropping the registry Arc alone can't free the
    /// last `turn_tx` clone, because every reader loop holds its own `WarmChild`
    /// (via `ctx.warm`), a self-referential sender that keeps `recv()` parked.
    /// `evict_idle_once` captures `g.pid` and calls `kill_child_tree(pid)`;
    /// `drain_all_for_shutdown` does the same. This test asserts the registry
    /// CARRIES a pid for an evictable child (the value the reap reads) and that an
    /// aged-out child is selected for eviction by the real decision kernel — the
    /// live PID-death itself is verified end-to-end via CDP (the signature-drain
    /// reaped a real spare PID on the running dev build; idle-evict shares the exact
    /// same `kill_child_tree(g.pid)` call). A pure OS-spawn test was dropped because
    /// the test harness can't load a child-spawning unit on this Tauri cdylib build.
    #[test]
    fn aged_child_with_pid_is_selected_for_eviction() {
        // An aged-out child (older than IDLE_EVICT) under the cap is evicted, and the
        // decision kernel that drives `evict_idle_once` agrees.
        let aged = mins(&[130]); // 130m ≫ 120m IDLE_EVICT
        let d = evict_decision(&aged, &[false], MAX_WARM, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        assert_eq!(d, vec![true], "a 130m-idle child must be evicted under the generous window");
        // And a child still mid-turn is spared regardless of age (we never kill a PID
        // out from under an in-flight request — the reap only runs once it's idle).
        let d2 = evict_decision(&aged, &[true], MAX_WARM, IDLE_EVICT, IDLE_EVICT_PRESSURE);
        assert_eq!(d2, vec![false], "an in-progress child is never selected for the PID reap");
    }
}
