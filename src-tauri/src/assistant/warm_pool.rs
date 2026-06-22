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
//! spawns a new one. All per-turn streaming/permission/steer plumbing stays
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot};

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
}

/// `static Mutex<Option<HashMap>>` + poison-recovering accessor — matches the
/// existing SESSION_PIDS / STEER_TX registry idiom (no new dependency). Keyed
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

/// Idle eviction window. A warm child unused for this long is killed +
/// deregistered by the background sweeper — frees the Velopack `current/` lock
/// (the MCP child it parents holds it) and reclaims memory.
const IDLE_EVICT: Duration = Duration::from_secs(300);
/// How often the background sweeper checks for idle children.
const EVICT_TICK: Duration = Duration::from_secs(60);

/// Snapshot the registry, find children idle past `IDLE_EVICT`, and drop their
/// turn_tx (→ reader loop exits → stdin drops → child EOFs → watchdog reaps).
/// Skips any child mid-turn. Returns the count evicted. M5/M7: clone Arcs out
/// under the registry lock, then inspect/remove WITHOUT holding both locks.
fn evict_idle_once() -> usize {
    let now = Instant::now();
    // Collect (session_id, Arc) snapshot under the registry lock, release it.
    let candidates: Vec<(String, Arc<Mutex<WarmChild>>)> =
        with_warm(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let mut evicted = 0;
    for (sid, arc) in candidates {
        // Lock the child (registry NOT held here) to read its state.
        let (idle, in_progress) = {
            let g = match arc.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            (
                now.duration_since(g.last_used) >= IDLE_EVICT,
                g.turn_in_progress.load(Ordering::Acquire),
            )
        };
        if idle && !in_progress {
            // remove_if takes the registry lock alone (child lock released).
            if remove_if(&sid, &arc) {
                evicted += 1;
                log::info!("warm_pool: idle-evicted session {sid} (>{}s)", IDLE_EVICT.as_secs());
                // Dropping the registry's Arc + the caller's local `arc` is what
                // eventually frees the last turn_tx clone. The reader loop holds
                // a clone too via the WarmChild it owns? No — the loop owns the
                // RECEIVER, not a sender. Once all `turn_tx` senders drop, the
                // loop's `recv()` yields None → it exits → stdin drops. The only
                // sender is in the WarmChild we just removed; `arc` drops at
                // end of this iteration.
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
            let n = evict_idle_once();
            if n > 0 {
                log::debug!("warm_pool: idle sweep evicted {n} child(ren)");
            }
        }
    });
}

/// Kill + drain EVERY warm child immediately (synchronous best-effort). Called
/// from the Velopack update-apply path alongside `kill_all_session_children` —
/// drops every turn_tx so reader loops exit and stdin drops, and clears the
/// registry so a late turn can't re-register. The PID tree-kill itself is done
/// by `kill_all_session_children` (warm PIDs are in SESSION_PIDS); this just
/// tears down the registry so nothing re-spawns during the swap.
pub(crate) fn drain_all_for_shutdown() {
    let drained: Vec<Arc<Mutex<WarmChild>>> = with_warm(|m| m.drain().map(|(_, v)| v).collect());
    log::info!("warm_pool: draining {} warm child(ren) for update-apply", drained.len());
    // Dropping the Arcs drops the WarmChild → its turn_tx → reader loop's recv
    // yields None → loop exits → stdin drops. Best-effort; the actual PID kill
    // is the IMAGENAME/tree sweep in update_service + kill_all_session_children.
    drop(drained);
}
