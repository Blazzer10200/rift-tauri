// AutoSync engine — Rust port of `Services/Sync/AutoSync.cs` (1167L C#).
//
// Design summary (deviates from WPF where Rust idiom is cleaner):
//   * notify v8 (recommended_watcher) per watch root → events flow through a
//     single tokio::mpsc into the engine's event handler.
//   * 700ms per-file debounce + 3000ms ceiling — replicated verbatim from WPF
//     (DebounceMs/CeilingMs). The flush task ticks every 150ms.
//   * Atomic upload via SftpClient::upload_file_atomic (.rift-tmp + rename).
//   * Concurrent uploads: russh-sftp multiplexes &self ops over a single SFTP
//     channel — we fire up to UPLOAD_CONCURRENCY ops in parallel inside one
//     batch via FuturesUnordered. No worker-pool of separate SSH sessions
//     needed.
//   * Status surface: emits `autosync://status`, `autosync://activity`, and
//     `autosync://conflict` Tauri events instead of the WPF
//     ObservableCollection / Dispatcher dance.
//   * Mass-delete circuit breaker: scaled-by-folder-size threshold preserved.
//   * Background tasks (lock acquire/release, bridge ping, edit-trail append)
//     are tracked in `background_tasks` so `stop()` can abort them — fire-
//     and-forget tasks otherwise outlive the engine via `Arc<SftpClient>`
//     keepalive.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::{mapref::entry::Entry, DashMap};
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::bridge::BridgeClient;
use crate::diagnostics::{self, DiagLevel, DiagStage};
use crate::profile::ServerProfile;
use crate::sftp::SftpClient;
use crate::state::sync_snapshot::SHA1_MAX_BYTES;
use crate::state::{RemoteStateCache, SyncSnapshot};
use crate::sync::ignore;
use crate::sync::lock_presence::LockPresence;

mod path;
#[path = "auto_sync/watch.rs"]
mod watch_mod;
mod flush;

use path::{
    classify_action, file_name, map_local_to_remote, rel_of, safe_count_files, stat_local,
    wait_for_readable,
};

// ─── Tunables (port from WPF) ────────────────────────────────────────────────
const DEBOUNCE_MS: u64 = 700;
const CEILING_MS: u64 = 3000;
const LOCK_HOLD_RETRY_SEC: u64 = 30;
const MASS_DELETE_THRESHOLD: usize = 25;
const UPLOAD_CONCURRENCY: usize = 4;

// ─── Public types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoSyncState {
    Idle,
    Syncing,
    Error,
    Disabled,
    Watching,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSyncStatus {
    pub state: AutoSyncState,
    pub detail: String,
    pub pending: usize,
    pub failed: usize,
    pub ignored_total: u64,
    pub conflicts: usize,
    pub watches: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Sync,
    Pull,
    Delete,
    Conflict,
    ConflictResolved,
    Drift,
    Bridge,
    Block,
    Error,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRow {
    pub at: DateTime<Utc>,
    pub resource: String,
    pub file: String,
    pub action: String,
    pub kind: ActivityKind,
    // v0.2.35 enrichment — drives the master/detail Activity panel. All five
    // are optional so unrelated emission sites (system events, errors, etc.)
    // can stay near-empty via `..Default::default()`. Skip-serializing keeps
    // the IPC payload compact on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
}

impl Default for ActivityRow {
    fn default() -> Self {
        Self {
            at: DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now),
            resource: String::new(),
            file: String::new(),
            action: String::new(),
            kind: ActivityKind::System,
            rel_path: None,
            local_path: None,
            size_bytes: None,
            latency_ms: None,
            sha: None,
            actor: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub local_path: String,
    pub remote_path: String,
    pub resource_name: String,
    pub local_size: i64,
    pub local_mtime_utc: DateTime<Utc>,
    pub remote_size: i64,
    pub remote_mtime_utc: DateTime<Utc>,
    pub last_known_size: i64,
    pub last_known_mtime_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    Skip,
    SaveLocalCopy,
    ForceLocal,
    AcceptRemote,
}

/// Caller-supplied watch spec. Mirrors the `scan_drift` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSpec {
    pub resource_name: String,
    pub remote_subpath: String,
}

// ─── Internal types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeKind {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
struct DirtyEntry {
    watch_key: String,
    path: PathBuf,
    kind: ChangeKind,
    first_seen: DateTime<Utc>,
    next_flush: DateTime<Utc>,
    attempts: u32,
    next_retry: DateTime<Utc>,
    bypass_preflight: bool,
}

#[derive(Debug, Clone)]
pub struct FolderWatch {
    pub local_root: PathBuf,
    pub remote_root: String,
    pub resource_name: String,
}

const RETRY_BACKOFFS_SECS: &[u64] = &[30, 120, 600];

fn merge_kind(a: ChangeKind, b: ChangeKind) -> ChangeKind {
    if b == ChangeKind::Deleted {
        return ChangeKind::Deleted;
    }
    if a == ChangeKind::Created {
        return ChangeKind::Created;
    }
    b
}

// ─── Engine ──────────────────────────────────────────────────────────────────

pub struct AutoSyncEngine {
    sftp: Arc<SftpClient>,
    profile: ServerProfile,
    snapshot: Arc<SyncSnapshot>,
    cache: Arc<RemoteStateCache>,
    locks: Option<Arc<LockPresence>>,
    bridge: Option<Arc<BridgeClient>>,
    app: AppHandle,

    folders: DashMap<String, FolderWatch>, // remote_root -> FolderWatch
    dirty: DashMap<PathBuf, DirtyEntry>,
    failed: DashMap<PathBuf, DirtyEntry>,
    conflicts: DashMap<PathBuf, ConflictRecord>,
    manual_delete_suppress_until: DashMap<PathBuf, DateTime<Utc>>,

    /// Cancellation token for the currently in-flight drift reconcile.
    /// Replaced on each `kick_drift_reconcile`; `cancel_drift_reconcile`
    /// fires the stored token (no-op when None). std::sync::Mutex because
    /// `kick_drift_reconcile` is sync (notify event handler can call it).
    current_scan_cancel: std::sync::Mutex<Option<CancellationToken>>,

    /// Cached entries from the most recent scan (drift_watcher tick OR
    /// kick_drift_reconcile). `force_pull_now` dispatches from this cache
    /// instead of re-scanning — drift_watcher runs every 10s so cache is
    /// almost always fresh. Empty = "no scan yet, fall back to full scan."
    last_scan_entries: std::sync::Mutex<Vec<crate::sync::DriftEntry>>,

    /// Folders that the last reconcile aborted via the suspicious-shrink
    /// guard. Surfaced to the frontend via `sync_get_aborted_shrunk` so the
    /// user can rebaseline after a deliberate cleanup. v0.2.49.
    last_aborted_shrunk: std::sync::Mutex<Vec<crate::sync::AbortedShrunkFolder>>,

    state: Mutex<(AutoSyncState, String)>,
    /// v0.2.52: track consecutive flush_batch failures so we don't flip the
    /// engine to Error on a one-off single-file fail (an editor-tmp race, an
    /// adjacent vanished path from a folder rename). Only escalate to Error
    /// when (a) `ConnectionWedged` fires (caught by sftp/transfer.rs::with_t)
    /// or (b) 3+ consecutive batches end with `fail > 0`. Single fails go to
    /// Watching state with a "N retry pending" detail.
    consecutive_failed_batches: AtomicU64,
    /// v0.2.53: Mirror mode toggle. When true, the drift scanner buckets
    /// `local-missing + remote-has + baseline-has` as `ToDeleteRemote`
    /// (propagate local delete to remote) instead of `ToPull` (restore from
    /// remote). Session-scoped — does NOT persist across engine restarts.
    /// User toggles via `sync_set_mirror_mode` Tauri cmd; the typed-confirm
    /// + dry-run preview flow lives in the frontend.
    mirror_mode: AtomicBool,
    ignored_total: AtomicU64,
    ignored_by_rule: DashMap<String, u64>,

    /// Paths Rift just wrote to via download_file_atomic. The Windows
    /// atomic-replace pattern fires Delete+Modify fs-events on the real
    /// path that aren't covered by the `.rift-tmp` ignore rule, causing
    /// a pull → upload → pull loop. We suppress events for ~5s after
    /// every successful pull. Lazy-evicted in queue_path.
    recently_written: DashMap<PathBuf, std::time::Instant>,

    // notify Watcher is !Send-friendly via a Mutex; held to keep watching alive.
    watcher: Mutex<Option<RecommendedWatcher>>,
    event_task: Mutex<Option<JoinHandle<()>>>,
    /// Tracker for fire-and-forget background tasks (lock acquire/release,
    /// bridge ping, edit-trail append). Aborted in `stop()` so they can't
    /// outlive the engine and hold stale `Arc<SftpClient>` clones.
    background_tasks: std::sync::Mutex<Vec<JoinHandle<()>>>,
    stop_tx: watch::Sender<bool>,
    disposed: AtomicBool,
    /// Coalesce flag for the Created+Dir → kick_drift_reconcile path. Set
    /// when a 500 ms-delayed reconcile is already queued; new Create(Dir)
    /// events during the window dedupe to no-op. Cleared by the delayed
    /// task right before it dispatches the reconcile. Fixes Bug 5
    /// 2026-05-12: a brand-new subtree like `[endure]/endure_rifttest/`
    /// fires dir-Create BEFORE its files exist on disk, so an immediate
    /// scan returns empty. 500 ms lets Windows finish writing the files,
    /// then one scan picks the whole tree up.
    pending_dir_reconcile: AtomicBool,
}

impl AutoSyncEngine {
    pub async fn start_with(
        sftp: Arc<SftpClient>,
        profile: ServerProfile,
        app: AppHandle,
        locks: Option<Arc<LockPresence>>,
        bridge: Option<Arc<BridgeClient>>,
    ) -> Result<Arc<Self>, String> {
        let snapshot = Arc::new(
            SyncSnapshot::new(&profile.key).map_err(|e| format!("snapshot init: {e}"))?,
        );
        let cache = Arc::new(
            RemoteStateCache::new(&profile.key).map_err(|e| format!("state cache init: {e}"))?,
        );
        let (stop_tx, _) = watch::channel(false);

        let engine = Arc::new(Self {
            sftp,
            profile,
            snapshot,
            cache,
            locks,
            bridge,
            app,
            folders: DashMap::new(),
            dirty: DashMap::new(),
            failed: DashMap::new(),
            conflicts: DashMap::new(),
            manual_delete_suppress_until: DashMap::new(),
            current_scan_cancel: std::sync::Mutex::new(None),
            last_scan_entries: std::sync::Mutex::new(Vec::new()),
            last_aborted_shrunk: std::sync::Mutex::new(Vec::new()),
            state: Mutex::new((AutoSyncState::Idle, String::new())),
            consecutive_failed_batches: AtomicU64::new(0),
            mirror_mode: AtomicBool::new(false),
            ignored_total: AtomicU64::new(0),
            ignored_by_rule: DashMap::new(),
            recently_written: DashMap::new(),
            watcher: Mutex::new(None),
            event_task: Mutex::new(None),
            background_tasks: std::sync::Mutex::new(Vec::new()),
            stop_tx,
            disposed: AtomicBool::new(false),
            pending_dir_reconcile: AtomicBool::new(false),
        });

        // Channel for FS events from the notify thread → tokio runtime.
        // Bounded (audit #4) — webpack/IDE rebuild bursts under a stalled flush
        // could grow an unbounded channel without limit. 2048 absorbs typical
        // bursts (git checkout, webpack hot-rebuild). Try-send + drop-with-warn
        // is the only non-blocking option since the watcher closure is sync.
        let (tx, rx) = mpsc::channel::<Event>(2048);
        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(ev) = res {
                if tx.try_send(ev).is_err() {
                    log::warn!("autosync FS event channel full (cap=2048); dropping event");
                    diagnostics::emit(
                        DiagStage::QueueDropped,
                        DiagLevel::Warn,
                        "FS event channel full (cap=2048); event dropped",
                    );
                }
            }
        })
        .map_err(|e| format!("notify watcher init: {e}"))?;
        *engine.watcher.lock().await = Some(watcher);

        // Event loop — drains FS events into _dirty.
        let ev_engine = engine.clone();
        let mut ev_stop = engine.stop_tx.subscribe();
        let event_task = tokio::spawn(async move {
            let mut rx = rx;
            loop {
                tokio::select! {
                    _ = ev_stop.changed() => {
                        if *ev_stop.borrow() { break; }
                    }
                    maybe_ev = rx.recv() => {
                        match maybe_ev {
                            Some(ev) => ev_engine.on_fs_event(ev).await,
                            None => break,
                        }
                    }
                }
            }
        });
        *engine.event_task.lock().await = Some(event_task);

        // v0.2.52: watched-root-vanished poll. notify-rs issue #403 (open in
        // v8.2.0) — when the user deletes the dir Rift is directly watching,
        // the Windows backend silently unregisters the watch w/ no `Remove`
        // event. Pre-v0.2.52 this left the engine alive but oblivious, every
        // subsequent edit landing nowhere. Poll local_root.exists() every 5 s
        // across all watched folders; on miss, log + emit a high-vis diag so
        // the user sees it in the panel + kick a drift reconcile (which will
        // surface the folder-level delete via baseline diff once Mirror mode
        // lands in v0.2.53; for now it just confirms the gap visibly).
        let root_engine = engine.clone();
        let mut root_stop = engine.stop_tx.subscribe();
        let root_task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
            // Skip the immediate-fire tick so we don't false-alarm on startup
            // before folders register.
            tick.tick().await;
            let mut seen_missing: std::collections::HashSet<std::path::PathBuf> =
                std::collections::HashSet::new();
            loop {
                tokio::select! {
                    _ = root_stop.changed() => { if *root_stop.borrow() { break; } }
                    _ = tick.tick() => {
                        if root_engine.disposed.load(Ordering::SeqCst) { break; }
                        for kv in root_engine.folders.iter() {
                            let local = kv.value().local_root.clone();
                            let resource = kv.value().resource_name.clone();
                            if !local.exists() {
                                // De-dup: only fire once per missing root until
                                // it comes back. Avoids spamming the panel
                                // every 5 s while the user investigates.
                                if seen_missing.insert(local.clone()) {
                                    let msg = format!(
                                        "watched local root vanished: {} ({})",
                                        local.display(),
                                        resource,
                                    );
                                    log::error!("{msg}");
                                    diagnostics::emit_for(
                                        DiagStage::Log,
                                        DiagLevel::Error,
                                        Some(&resource),
                                        Some(&local.to_string_lossy()),
                                        &msg,
                                    );
                                    root_engine.kick_drift_reconcile();
                                }
                            } else if seen_missing.remove(&local) {
                                log::info!(
                                    "watched local root reappeared: {} ({})",
                                    local.display(),
                                    resource,
                                );
                            }
                        }
                    }
                }
            }
        });
        engine.track_background(root_task);

        engine.set_state(AutoSyncState::Watching, "0 folder(s)".into()).await;

        Ok(engine)
    }

    pub async fn stop(&self) {
        if self.disposed.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = self.stop_tx.send(true);
        // event_task is aborted (vs awaited) because on_fs_event futures may
        // park on the dirty DashMap shard locks under heavy churn — we don't
        // want stop() to block on user-driven event flow. The DashMap ops are
        // atomic so an aborted future never leaves the map in a torn state.
        if let Some(h) = self.event_task.lock().await.take() {
            h.abort();
        }
        // Abort any in-flight fire-and-forget tasks (lock release, bridge ping,
        // edit-trail append) so they can't outlive the engine.
        let outstanding: Vec<JoinHandle<()>> = match self.background_tasks.lock() {
            Ok(mut g) => std::mem::take(&mut *g),
            Err(e) => std::mem::take(&mut *e.into_inner()),
        };
        for h in outstanding {
            h.abort();
        }
        // Drop the watcher to release notify resources.
        *self.watcher.lock().await = None;
        if let Some(lp) = &self.locks {
            lp.stop().await;
        }
        self.set_state(AutoSyncState::Disabled, "stopped".into()).await;
    }

    /// Track a fire-and-forget background task so `stop()` can abort it. Drops
    /// already-completed handles opportunistically to keep the vec small.
    fn track_background(&self, h: JoinHandle<()>) {
        if let Ok(mut g) = self.background_tasks.lock() {
            g.retain(|t| !t.is_finished());
            g.push(h);
        }
    }

    /// v0.2.53 Mirror toggle. Session-scoped; reset on engine restart.
    pub fn set_mirror_mode(&self, enabled: bool) {
        self.mirror_mode.store(enabled, Ordering::Relaxed);
    }

    pub fn mirror_mode_enabled(&self) -> bool {
        self.mirror_mode.load(Ordering::Relaxed)
    }

    /// Snapshot of currently-watched remote roots — for `LockPresence` scoped poll.
    pub fn watched_remote_roots(&self) -> Vec<String> {
        self.folders.iter().map(|kv| kv.value().remote_root.clone()).collect()
    }

    /// v0.2.50: walk every watched root and reclaim stale `.rift-lock` files
    /// owned by this user. Returns the total count of locks swept. The
    /// underlying `LockPresence::sweep_stale_mine` already gates on
    /// `body.user == my_user` + `since older than STALE_SEC` so this is safe
    /// to run any time; it just won't sweep locks held by OTHER users (those
    /// stay visible via badges until naturally aging out or being released).
    /// Exposed as `sync_sweep_stale_locks` Tauri command for the user's
    /// manual recovery path when the poll-task got behind (e.g. after a
    /// connection wedge).
    pub async fn sweep_stale_locks(&self) -> Result<usize, String> {
        let Some(locks) = self.locks.as_ref() else {
            return Ok(0);
        };
        let mut total = 0usize;
        for root in self.watched_remote_roots() {
            match locks.sweep_stale_mine(&root, 12).await {
                Ok(n) => total += n,
                Err(e) => log::warn!("sweep_stale_locks {root}: {e}"),
            }
        }
        Ok(total)
    }

    pub async fn try_watch(&self, spec: FolderSpec) -> Result<bool, String> {
        if self.disposed.load(Ordering::SeqCst) {
            return Err("engine disposed".into());
        }
        let remote_root = format!(
            "{}/{}",
            self.profile.remote_root.trim_end_matches('/'),
            spec.remote_subpath.trim_start_matches('/')
        );
        if self.folders.contains_key(&remote_root) {
            return Ok(true);
        }
        let local_root = Path::new(&self.profile.local_root)
            .join(spec.remote_subpath.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !local_root.exists() {
            self.log(&format!(
                "watch failed (local missing): {} -> {}",
                remote_root,
                local_root.display()
            ));
            return Ok(false);
        }
        // Refuse to attach to ignored paths (e.g. `[disabled]/`).
        let probe = format!("{}{}", local_root.display(), std::path::MAIN_SEPARATOR);
        if ignore::should_ignore(&probe) {
            self.log(&format!("watch refused (ignored path): {}", local_root.display()));
            return Ok(false);
        }

        // Add notify watch.
        if let Some(w) = self.watcher.lock().await.as_mut() {
            w.watch(&local_root, RecursiveMode::Recursive)
                .map_err(|e| format!("notify watch {}: {e}", local_root.display()))?;
        }

        let watch = FolderWatch {
            local_root: local_root.clone(),
            remote_root: remote_root.clone(),
            resource_name: spec.resource_name.clone(),
        };
        self.folders.insert(remote_root.clone(), watch);
        self.log(&format!("watching {} ({})", spec.resource_name, local_root.display()));
        if let Some(locks) = self.locks.clone() {
            let root_for_sweep = remote_root.clone();
            let h = tokio::spawn(async move {
                match locks.sweep_stale_mine(&root_for_sweep, 4).await {
                    Ok(n) if n > 0 => log::info!("removed {n} stale own .rift-lock file(s) under {root_for_sweep}"),
                    Err(e) => log::warn!("stale .rift-lock sweep failed under {root_for_sweep}: {e}"),
                    _ => {}
                }
            });
            self.track_background(h);
        }

        // Fire-and-forget perm-heal: chmod 2775 on every dir we own under this
        // root. Backlog cleanup for dirs Rift created pre-v0.2.31 (umask 0022
        // → 0755 → teammates couldn't push into them). v0.2.31's mkdir_p_via
        // handles new dirs; this catches the rest. Async + best-effort — if
        // it fails, nothing breaks; mkdir_p will heal piece-meal on future
        // pushes anyway.
        let sftp = self.sftp.clone();
        let root_for_heal = remote_root.clone();
        tokio::spawn(async move {
            sftp.heal_owned_dirs(&root_for_heal).await;
        });

        let count = self.folders.len();
        let (cur_state, _) = *self.state.lock().await;
        if cur_state == AutoSyncState::Watching {
            self.set_state(AutoSyncState::Watching, format!("{count} folder(s)")).await;
        } else {
            self.fire_status().await;
        }
        Ok(true)
    }

    async fn stop_watch(&self, remote_root: &str) {
        let Some((_, fw)) = self.folders.remove(remote_root) else { return };
        if let Some(w) = self.watcher.lock().await.as_mut() {
            let _ = w.unwatch(&fw.local_root);
        }
        self.log(&format!("stopped watching {}", fw.resource_name));
        let count = self.folders.len();
        let (cur_state, _) = *self.state.lock().await;
        if cur_state == AutoSyncState::Watching {
            self.set_state(AutoSyncState::Watching, format!("{count} folder(s)")).await;
        } else {
            self.fire_status().await;
        }
    }

    /// Snapshot of the per-rule ignore counts. Sync Inspector consumes this to
    /// answer "why isn't my file syncing" — counts bucket by rule label
    /// (e.g. `seg:.git`, `ext:.tmp`, `editor-lock(~$)`).
    pub fn ignored_by_rule_snapshot(&self) -> HashMap<String, u64> {
        self.ignored_by_rule
            .iter()
            .map(|kv| (kv.key().clone(), *kv.value()))
            .collect()
    }

    // ─── DriftWatcher accessors (Phase: bidirectional sync) ──────────────────
    //
    // These expose the minimum surface the remote-scan watcher needs without
    // forcing it to live inside this module. Kept narrow on purpose.

    /// Cloned snapshot of currently-watched folders. Watcher uses this every
    /// tick — a fresh clone is cheaper than holding the DashMap lock across
    /// an SFTP roundtrip.
    pub fn folders_clone(&self) -> Vec<FolderWatch> {
        self.folders.iter().map(|kv| kv.value().clone()).collect()
    }

    /// Walk the last-scan cache and inject every ToPush entry into the dirty
    /// queue (skipping ones already there, ones whose local file vanished
    /// since the scan, and ones whose folder watch no longer exists).
    /// Returns the count of newly-enqueued entries.
    ///
    /// Why this exists: the watcher only catches local edits that happen
    /// AFTER it attaches. A file that existed before the watcher started
    /// (or whose change-event was missed) lives only in the drift scanner's
    /// view — `ToPush` bucket in `last_scan_entries`. Push used to drain
    /// only the watcher dirty queue, so those entries were stranded.
    fn promote_scan_pushes_to_dirty(&self) -> u32 {
        let entries: Vec<crate::sync::DriftEntry> = match self.last_scan_entries.lock() {
            Ok(g) => g
                .iter()
                .filter(|e| matches!(e.bucket, crate::sync::DriftBucket::ToPush))
                .cloned()
                .collect(),
            Err(_) => return 0,
        };
        if entries.is_empty() {
            return 0;
        }
        let folders = self.folders_clone();
        let now = Utc::now();
        let mut added = 0u32;
        for de in entries {
            let watch_key = match folders.iter().find(|fw| fw.resource_name == de.resource_name) {
                Some(fw) => fw.remote_root.clone(),
                None => continue,
            };
            let path = PathBuf::from(&de.local_path);
            if self.dirty.contains_key(&path) {
                continue;
            }
            if !path.exists() {
                continue;
            }
            let entry = DirtyEntry {
                watch_key,
                path: path.clone(),
                kind: ChangeKind::Modified,
                first_seen: now,
                next_flush: now,
                attempts: 0,
                next_retry: now,
                bypass_preflight: false,
            };
            self.dirty.insert(path, entry);
            added += 1;
        }
        added
    }

    /// `true` if a local-side push is in flight or queued. The remote-scan
    /// loop pauses while this is set so we don't pull a file we're about to
    /// upload, then re-flag it as drifted on the next tick.
    pub fn is_pushing(&self) -> bool {
        if !self.dirty.is_empty() {
            return true;
        }
        // best-effort state read — if the lock is held briefly elsewhere,
        // treat as not pushing rather than block.
        match self.state.try_lock() {
            Ok(g) => matches!(g.0, AutoSyncState::Syncing),
            Err(_) => false,
        }
    }

    /// `true` if `path` is currently sitting in the dirty queue (pending
    /// upload). Used by the conflict-rename guard so we never overwrite a
    /// local file that has unflushed bytes.
    pub fn is_path_dirty(&self, path: &Path) -> bool {
        self.dirty.contains_key(path)
    }

    /// Push a remote-pull-detected conflict into the same DashMap that local
    /// pre-flight feeds. Surfaces in the existing Conflicts tab UI with the
    /// existing ConflictResolver. Also fires the autosync conflict event.
    pub fn record_remote_conflict(&self, local_path: &Path, record: ConflictRecord) {
        let _ = self.app.emit("autosync://conflict", &record);
        self.conflicts.insert(local_path.to_path_buf(), record);
    }

    /// Direct refs the watcher needs to download + update baseline.
    pub fn sftp(&self) -> Arc<SftpClient> { self.sftp.clone() }
    pub fn snapshot(&self) -> Arc<SyncSnapshot> { self.snapshot.clone() }
    pub fn cache(&self) -> Arc<RemoteStateCache> { self.cache.clone() }
    pub fn locks(&self) -> Option<Arc<LockPresence>> { self.locks.clone() }
    pub fn app(&self) -> AppHandle { self.app.clone() }
    pub fn profile_key(&self) -> &str { &self.profile.key }
    pub fn owns_local_path(&self, local_path: &Path) -> bool {
        self.folders
            .iter()
            .any(|kv| local_path.starts_with(&kv.value().local_root))
    }
    pub fn is_disposed(&self) -> bool { self.disposed.load(Ordering::SeqCst) }

    /// Drain every dirty entry NOW. Used by `force_push_now` — the user's
    /// manual "push everything" button. Promotes any backoff-elapsed failed
    /// entries into dirty first so transient SFTP failures still retry on
    /// the next click. Mass-delete circuit breaker fires inside `flush_batch`.
    /// `cancel` propagates into flush_batch's dispatch loop — clicking Stop
    /// during a push bails between entries, leaving un-dispatched ones in
    /// the dirty queue for the next click to pick up.
    pub async fn flush_all_now(&self, cancel: Option<CancellationToken>) -> u32 {
        if self.disposed.load(Ordering::SeqCst) {
            return 0;
        }
        // Promote failed → dirty when backoff elapsed (used to live in the
        // killed flush_cycle loop; without this, transient failures never
        // retry on manual Push).
        let now = Utc::now();
        let mut promote: Vec<(PathBuf, DirtyEntry)> = Vec::new();
        let mut drop_failed: Vec<PathBuf> = Vec::new();
        for kv in self.failed.iter() {
            let e = kv.value();
            if e.attempts as usize >= RETRY_BACKOFFS_SECS.len() {
                continue;
            }
            if e.next_retry > now {
                continue;
            }
            if e.kind != ChangeKind::Deleted && !e.path.exists() {
                drop_failed.push(kv.key().clone());
                continue;
            }
            let mut promoted = e.clone();
            promoted.next_flush = now;
            promote.push((kv.key().clone(), promoted));
        }
        for k in drop_failed {
            self.failed.remove(&k);
        }
        for (k, e) in promote {
            self.failed.remove(&k);
            self.log(&format!("manual-retry attempt {}/3: {}", e.attempts + 1, e.path.display()));
            self.dirty.insert(k, e);
        }

        // Lazy-pop: clone entries into ready but DON'T remove from dirty here.
        // flush_batch removes each entry just before it dispatches; on cancel,
        // un-dispatched entries stay in dirty so the next Push picks them up.
        // Up-front pop would've lost queued work on cancel.
        let ready: Vec<DirtyEntry> = self.dirty
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        let mut by_watch: HashMap<String, Vec<DirtyEntry>> = HashMap::new();
        for e in ready.iter() {
            by_watch.entry(e.watch_key.clone()).or_default().push(e.clone());
        }
        let mut count = 0u32;
        for (watch_key, entries) in by_watch {
            if let Some(ct) = &cancel {
                if ct.is_cancelled() {
                    break;
                }
            }
            let Some(fw) = self.folders.get(&watch_key).map(|v| v.value().clone()) else {
                continue;
            };
            let dispatched = self.flush_batch(&fw, entries, cancel.clone()).await;
            count += dispatched;
        }
        count
    }

    /// Force-push NOW — drains every dirty entry regardless of debounce, AND
    /// promotes ToPush drift entries from the last scan into the dirty queue
    /// first. Without the scan-promotion step, Push only saw files edited
    /// AFTER the watcher attached — files that existed locally and never got
    /// a watcher event (pre-existing drift, missed events) showed up in the
    /// scan as ToPush but the Push button did nothing. Pull was symmetric;
    /// Push wasn't. This restores parity.
    pub fn force_push_now(self: &Arc<Self>) {
        eprintln!("[rift] force_push_now: entry");
        let ct = CancellationToken::new();
        let ct_for_task = ct.clone();
        if let Ok(mut g) = self.current_scan_cancel.lock() {
            *g = Some(ct);
        }
        let engine = self.clone();
        let h = tokio::spawn(async move {
            eprintln!("[rift] force_push_now: task spawned");
            diagnostics::emit(
                DiagStage::DriftScanStart,
                DiagLevel::Info,
                "push-now",
            );
            eprintln!("[rift] force_push_now: drift_scan_start emitted");

            // Promote ToPush drift entries (from last scan) into the dirty
            // queue. They're idempotent — flush_all_now's lazy-pop handles
            // duplicates, and we skip entries already in dirty / vanished
            // since scan. Without this step a freshly-connected session can
            // see "78 to push" in the scan and dispatch zero on Push click.
            let promoted = engine.promote_scan_pushes_to_dirty();
            eprintln!("[rift] force_push_now: promoted {promoted} from scan cache, dirty queue size = {}",
                engine.dirty.len());

            // Auto-scan fallback — if after the cache-promote pass both the
            // dirty queue AND the scan cache are empty (cold session), run
            // an inline drift scan so Push can find files that exist locally
            // but never went through the watcher. Parity w/ force_pull_now.
            if engine.dirty.is_empty() && promoted == 0 {
                eprintln!("[rift] force_push_now: dirty empty + cache empty → running inline scan");
                let folders = engine.folders_clone();
                if folders.is_empty() {
                    eprintln!("[rift] force_push_now: no watched folders — nothing to scan");
                } else {
                    let targets: Vec<crate::sync::drift_scanner::FolderTarget> = folders
                        .iter()
                        .map(|fw| crate::sync::drift_scanner::FolderTarget {
                            resource_name: fw.resource_name.clone(),
                            local_root: fw.local_root.to_string_lossy().to_string(),
                            remote_root: fw.remote_root.clone(),
                        })
                        .collect();
                    let snap = engine.snapshot();
                    let sftp = engine.sftp();
                    let scanner = crate::sync::drift_scanner::DriftScanner::new(&sftp, Some(&snap))
                        .with_mirror(engine.mirror_mode.load(Ordering::Relaxed));
                    let result = scanner.scan_with_cancel(&targets, Some(&ct_for_task)).await;
                    eprintln!("[rift] force_push_now: inline scan returned {} entries (cancelled={})",
                        result.entries.len(), result.cancelled);
                    if !result.cancelled {
                        engine.cache_scan_entries(result.entries.clone());
                        let re_promoted = engine.promote_scan_pushes_to_dirty();
                        eprintln!("[rift] force_push_now: after inline scan promoted {re_promoted}");
                    }
                }
            }

            if promoted > 0 {
                engine.log(&format!(
                    "push-now: promoted {promoted} scan ToPush entries into dirty queue"
                ));
            }

            let started = std::time::Instant::now();
            // flush_all_now + flush_batch both honor the cancel token: between
            // resources in flush_all_now, between entries in flush_batch's
            // dispatch loop. In-flight russh streams (1-4 per batch) finish
            // naturally — we can't abort mid-stream w/o leaving partial files.
            // Un-dispatched entries stay in the dirty queue.
            let dispatched = engine.flush_all_now(Some(ct_for_task.clone())).await;
            let cancelled = ct_for_task.is_cancelled();
            let elapsed_ms = started.elapsed().as_millis() as u64;
            eprintln!("[rift] force_push_now: flush_all_now returned dispatched={dispatched} cancelled={cancelled} elapsed_ms={elapsed_ms}");

            // Invalidate the scan cache after a successful (non-cancelled)
            // push. Without this, re-clicking Push re-promotes the same
            // entries from the stale cache and "uploads" them again (SHA-
            // collapse hides it, but the count lies). Next Push triggers an
            // auto-scan via the empty-cache fallback, getting fresh state.
            if !cancelled && dispatched > 0 {
                engine.cache_scan_entries(Vec::new());
                eprintln!("[rift] force_push_now: cleared scan cache (was stale post-push)");
            }
            diagnostics::emit_with_fields(
                DiagStage::DriftScanResult,
                DiagLevel::Info,
                None,
                None,
                if cancelled {
                    format!("push-now cancelled after {dispatched} flushed")
                } else if dispatched > 0 {
                    format!("push-now: flushed {dispatched} entries")
                } else {
                    "push-now: nothing pending".to_string()
                },
                serde_json::json!({
                    "entries": 0,
                    "to_push": dispatched,
                    "to_pull": 0,
                    "to_delete": 0,
                    "conflicts": 0,
                    "enqueued_for_push": dispatched,
                    "pull_dispatched": 0,
                    "missing_remote_folders": 0,
                    "listing_error": null,
                    "cancelled": cancelled,
                    "elapsed_ms": elapsed_ms,
                    "from_cache": true,
                }),
            );
        });
        self.track_background(h);
    }

    /// Scaled mass-delete threshold for a given local root. Returns
    /// `(threshold, total_file_count)`. Mirrors the push-side guard formula
    /// (base 25 ceiling, 30% of file count, floor 5) so local-delete and
    /// remote-delete blocks behave symmetrically. Called by drift_watcher
    /// before dispatching ToDelete entries; if N ≥ threshold, the batch is
    /// blocked with a `[guard]` activity row.
    pub async fn scaled_delete_threshold(&self, local_root: &Path) -> (usize, usize) {
        let n = safe_count_files(local_root).await;
        let threshold = ((n as f64 * 0.30) as usize).clamp(5, MASS_DELETE_THRESHOLD);
        (threshold, n)
    }

    pub fn reject_remote_locked(&self, remote_path: &str) -> Result<(), String> {
        if let Some(locks) = &self.locks {
            if let Some(lock) = locks.find_lock_by_other(remote_path) {
                return Err(format!("locked by {}@{}", lock.user, lock.host));
            }
        }
        Ok(())
    }

    pub fn reject_local_locked(&self, local_path: &Path) -> Result<(), String> {
        if let Some(remote) = self.remote_for_local(local_path) {
            self.reject_remote_locked(&remote)?;
        }
        Ok(())
    }

    pub fn suppress_local_delete_uploads(&self, paths: &[PathBuf]) {
        let until = Utc::now() + chrono::Duration::seconds(2);
        for path in paths {
            self.manual_delete_suppress_until.insert(path.clone(), until);
            self.remove_pending_under(path);
        }
    }

    pub fn suppress_remote_delete_uploads(&self, remote_paths: &[String]) {
        let locals: Vec<PathBuf> = remote_paths
            .iter()
            .filter_map(|remote| self.local_for_remote(remote))
            .collect();
        self.suppress_local_delete_uploads(&locals);
    }

    pub async fn status(&self) -> AutoSyncStatus {
        let (state, detail) = self.state.lock().await.clone();
        AutoSyncStatus {
            state,
            detail,
            pending: self.dirty.len(),
            failed: self.failed.len(),
            ignored_total: self.ignored_total.load(Ordering::Relaxed),
            conflicts: self.conflicts.len(),
            watches: self.folders.len(),
        }
    }

    pub async fn enqueue_for_flush_batch(
        &self,
        local_paths: Vec<PathBuf>,
        deleted: bool,
        bypass_preflight: bool,
    ) -> u32 {
        if self.disposed.load(Ordering::SeqCst) {
            return 0;
        }
        // Snapshot watches once, sorted by local-root len desc so most-specific wins.
        let mut watches: Vec<(String, PathBuf)> = self
            .folders
            .iter()
            .map(|kv| (kv.key().clone(), kv.value().local_root.clone()))
            .collect();
        watches.sort_by_key(|w| std::cmp::Reverse(w.1.components().count()));
        if watches.is_empty() {
            return 0;
        }
        let now = Utc::now();
        let mut enqueued = 0u32;
        let mut orphan = 0u32;
        for path in local_paths {
            let mut owner: Option<String> = None;
            for (key, root) in &watches {
                if path.starts_with(root) {
                    owner = Some(key.clone());
                    break;
                }
            }
            let Some(watch_key) = owner else {
                orphan += 1;
                continue;
            };
            let entry = DirtyEntry {
                watch_key,
                path: path.clone(),
                kind: if deleted { ChangeKind::Deleted } else { ChangeKind::Modified },
                first_seen: now,
                next_flush: now,
                attempts: 0,
                next_retry: now,
                bypass_preflight,
            };
            self.dirty.insert(path, entry);
            enqueued += 1;
        }
        if orphan > 0 {
            self.log(&format!("EnqueueForFlushBatch: {orphan} path(s) had no owning watch — skipped"));
        }
        enqueued
    }

    pub async fn retry_failed(&self) {
        if self.disposed.load(Ordering::SeqCst) {
            return;
        }
        let now = Utc::now();
        let mut moved = Vec::new();
        for kv in self.failed.iter() {
            let mut e = kv.value().clone();
            if e.kind != ChangeKind::Deleted && !e.path.exists() {
                moved.push((kv.key().clone(), None));
                continue;
            }
            e.next_flush = now;
            moved.push((kv.key().clone(), Some(e)));
        }
        for (k, v) in moved {
            self.failed.remove(&k);
            if let Some(e) = v {
                self.dirty.insert(k, e);
            }
        }
        self.fire_status().await;
    }

    pub async fn resolve_conflict(
        &self,
        local_path: &Path,
        resolution: ConflictResolution,
    ) -> Result<(), String> {
        let Some((_, c)) = self.conflicts.remove(local_path) else {
            return Ok(());
        };
        match resolution {
            ConflictResolution::Skip => {
                self.log(&format!("conflict skipped: {}", local_path.display()));
                self.log_activity(&c.resource_name, file_name(local_path), "conflict skipped");
            }
            ConflictResolution::AcceptRemote => {
                self.mark_recently_written(local_path);
                let r = self.sftp.download_file_atomic(&c.remote_path, local_path).await;
                if r.success {
                    self.mark_recently_written(local_path);
                    self.update_snapshot_after_sync(local_path, &c.remote_path).await;
                    self.log_activity(&c.resource_name, file_name(local_path), "conflict\u{2192}accept-remote");
                } else {
                    self.log_activity(&c.resource_name, file_name(local_path),
                        &format!("conflict accept-remote pull FAILED: {}", r.error));
                }
            }
            ConflictResolution::SaveLocalCopy => {
                let dir = local_path.parent().unwrap_or(Path::new("."));
                let stem = local_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
                let ext = local_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let aside_name = if ext.is_empty() {
                    format!("{stem}.local-{ts}")
                } else {
                    format!("{stem}.local-{ts}.{ext}")
                };
                let aside = dir.join(&aside_name);
                if local_path.exists() {
                    if let Err(e) = std::fs::rename(local_path, &aside) {
                        // CRITICAL: bail before download — overwriting the local
                        // file w/o the aside copy is silent data loss.
                        self.conflicts.insert(local_path.to_path_buf(), c.clone());
                        let msg = format!(
                            "conflict→savecopy aborted: aside rename failed ({e}); local file preserved"
                        );
                        self.log(&format!("CONFLICT savecopy abort {}: {e}", local_path.display()));
                        self.log_activity(&c.resource_name, file_name(local_path), &msg);
                        self.fire_status().await;
                        return Err(msg);
                    }
                }
                self.mark_recently_written(local_path);
                let r = self.sftp.download_file_atomic(&c.remote_path, local_path).await;
                if r.success {
                    self.mark_recently_written(local_path);
                    self.update_snapshot_after_sync(local_path, &c.remote_path).await;
                    self.log_activity(&c.resource_name, file_name(local_path),
                        &format!("conflict\u{2192}savecopy ({})", aside_name));
                } else {
                    self.log_activity(&c.resource_name, file_name(local_path),
                        &format!("conflict\u{2192}savecopy pull FAILED: {}", r.error));
                }
            }
            ConflictResolution::ForceLocal => {
                self.cache.set(&c.remote_path, c.local_size, c.local_mtime_utc);
                self.enqueue_for_flush_batch(vec![local_path.to_path_buf()], false, true).await;
                self.log_activity(&c.resource_name, file_name(local_path), "conflict\u{2192}force-local");
            }
        }
        self.fire_status().await;
        Ok(())
    }

    // ─── FS event handler ────────────────────────────────────────────────────

    async fn on_fs_event(self: &Arc<Self>, ev: Event) {
        if self.disposed.load(Ordering::SeqCst) {
            return;
        }
        // Rescan signal: kernel/FSEvents/ReadDirectoryChangesW dropped events
        // and is asking us to do a full reconcile. inotify man-page + Apple
        // FSEvents docs both say robust apps MUST handle this — silently
        // missing events otherwise. Auto-fire a drift scan against every
        // watched folder so the snapshot catches up.
        if ev.need_rescan() {
            diagnostics::emit(
                DiagStage::RescanSignal,
                DiagLevel::Warn,
                "notify reported event drop — triggering drift reconcile",
            );
            self.kick_drift_reconcile();
            return;
        }
        // v0.2.52: explicit rename-event handling. notify v8 on Windows emits
        // folder rename as two separate events: `Modify(Name(From))` for the
        // old path + `Modify(Name(To))` for the new path (NOT a single Both —
        // confirmed never emitted by the Windows backend per notify issue
        // #376). Recycle-Bin delete is technically a MOVE to $RECYCLE.BIN
        // and surfaces as `Modify(Name(From))` w/ no matching `To` since the
        // destination is outside the watched scope. Pre-v0.2.52 these were
        // bucketed as `Modified`, which then failed `wait_for_readable` on
        // the now-vanished path, flipping engine to Error w/o propagating
        // the delete. Explicit From→Deleted + To→Created routes around it.
        let kind = match ev.kind {
            EventKind::Create(_) => ChangeKind::Created,
            EventKind::Remove(_) => ChangeKind::Deleted,
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => ChangeKind::Deleted,
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => ChangeKind::Created,
            EventKind::Modify(ModifyKind::Name(_)) => ChangeKind::Modified, // Any/Both/Other safety
            EventKind::Modify(_) => ChangeKind::Modified,
            _ => return, // Access, Other — skip
        };
        for path in ev.paths {
            // Diagnostic emit BEFORE filter so the panel shows what notify saw,
            // even if the path is later ignored or doesn't map to a watch.
            let path_str = path.to_string_lossy().to_string();
            diagnostics::emit_for(
                DiagStage::FsEvent,
                DiagLevel::Debug,
                None,
                Some(&path_str),
                format!("{kind:?}"),
            );
            // Belt-and-suspenders for Bug 5 (Endure RP 2026-05-12): when a new
            // directory subtree is created under a watched bracket, Windows
            // ReadDirectoryChangesW races the subtree registration — Create
            // events for files nested inside a freshly-created dir BEFORE
            // it fully registers get dropped. The dir-Create event itself
            // reaches us, but queue_path discards dir events.
            //
            // v0.2.47 fired kick_drift_reconcile immediately on Create(Dir),
            // but the files don't exist on disk yet — scan walked an empty
            // tree and surfaced nothing. v0.2.48 adds a 500 ms delay + an
            // AtomicBool coalesce so 50 rapid Create(Dir) events inside one
            // new tree collapse to one delayed reconcile that fires AFTER
            // Windows finishes writing the files. compare_exchange takes
            // ownership of the dispatch slot; if another Create(Dir) lost
            // the race, it skips (delayed task will pick up the new files).
            if kind == ChangeKind::Created && path.is_dir() {
                if self
                    .pending_dir_reconcile
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    let engine = self.clone();
                    let h = tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        engine.pending_dir_reconcile.store(false, Ordering::Release);
                        if engine.disposed.load(Ordering::SeqCst) {
                            return;
                        }
                        engine.kick_drift_reconcile();
                    });
                    self.track_background(h);
                }
            }
            self.queue_path(path, kind);
        }
    }

    /// Fire a one-shot drift reconcile across every watched folder. Used by
    /// the rescan-signal path AND callable from the Quick Actions panel via the
    /// `sync_reconcile` Tauri command. Background-spawned so the
    /// notify handler isn't stalled by SFTP latency.
    pub fn kick_drift_reconcile(self: &Arc<Self>) {
        let folders: Vec<FolderWatch> = self
            .folders
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        if folders.is_empty() {
            return;
        }
        let sftp = self.sftp.clone();
        let snapshot = self.snapshot.clone();
        let engine = self.clone();
        let ct = CancellationToken::new();
        let ct_for_task = ct.clone();
        // Replace any prior in-flight token. Old scan keeps its own clone — if it
        // was mid-folder it will check on the next iteration; if it already
        // finished the cancel is a harmless no-op.
        if let Ok(mut g) = self.current_scan_cancel.lock() {
            *g = Some(ct);
        }
        let h = tokio::spawn(async move {
            diagnostics::emit(
                DiagStage::DriftScanStart,
                DiagLevel::Info,
                format!("reconcile across {} folder(s)", folders.len()),
            );
            let scanner = crate::sync::DriftScanner::new(&sftp, Some(&snapshot))
                .with_mirror(engine.mirror_mode.load(Ordering::Relaxed));
            let targets: Vec<crate::sync::FolderTarget> = folders
                .iter()
                .map(|fw| crate::sync::FolderTarget {
                    resource_name: fw.resource_name.clone(),
                    local_root: fw.local_root.to_string_lossy().to_string(),
                    remote_root: fw.remote_root.clone(),
                })
                .collect();
            let result = scanner.scan_with_cancel(&targets, Some(&ct_for_task)).await;
            // Cache entries for force_pull_now's fast path.
            if !result.cancelled {
                if let Ok(mut g) = engine.last_scan_entries.lock() {
                    *g = result.entries.clone();
                }
                if let Ok(mut g) = engine.last_aborted_shrunk.lock() {
                    *g = result.aborted_shrunk.clone();
                }
                for a in &result.aborted_shrunk {
                    diagnostics::emit_with_fields(
                        DiagStage::BaselineShrinkDetected,
                        DiagLevel::Warn,
                        Some(&a.resource_name),
                        None,
                        format!(
                            "baseline shrink: {} baseline → {} listing — bracket aborted, files local-only invisible until rebaseline",
                            a.baseline_count, a.listing_count,
                        ),
                        serde_json::json!({
                            "resource": a.resource_name,
                            "remote_root": a.remote_root,
                            "baseline_count": a.baseline_count,
                            "listing_count": a.listing_count,
                        }),
                    );
                }
            }
            let mut to_push = 0u32;
            let mut to_pull = 0u32;
            let mut to_delete = 0u32;
            let mut to_delete_remote = 0u32;
            let mut conflicts = 0u32;
            for e in &result.entries {
                match e.bucket {
                    crate::sync::DriftBucket::ToPush => to_push += 1,
                    crate::sync::DriftBucket::ToPull => to_pull += 1,
                    crate::sync::DriftBucket::ToDelete => to_delete += 1,
                    crate::sync::DriftBucket::ToDeleteRemote => to_delete_remote += 1,
                    crate::sync::DriftBucket::Conflict => conflicts += 1,
                    crate::sync::DriftBucket::Synced => {}
                }
            }
            // Per-resource breakdown for diagnosability — without this, a scan
            // result like "430 to_delete" gives no hint which resources are
            // affected. Helps root-cause partial-listing issues (e.g. deep
            // ox_lib paths or bracket-encoded dirs dropping files silently).
            let mut by_resource: std::collections::HashMap<String, (u32, u32, u32, u32, u32)> =
                std::collections::HashMap::new();
            for e in &result.entries {
                let row = by_resource.entry(e.resource_name.clone()).or_insert((0, 0, 0, 0, 0));
                match e.bucket {
                    crate::sync::DriftBucket::ToPush          => row.0 += 1,
                    crate::sync::DriftBucket::ToPull          => row.1 += 1,
                    crate::sync::DriftBucket::ToDelete        => row.2 += 1,
                    crate::sync::DriftBucket::Conflict        => row.3 += 1,
                    crate::sync::DriftBucket::ToDeleteRemote  => row.4 += 1,
                    crate::sync::DriftBucket::Synced          => {}
                }
            }
            let breakdown: serde_json::Value = serde_json::Value::Object(
                by_resource.iter().map(|(k, v)| {
                    (k.clone(), serde_json::json!({
                        "to_push": v.0, "to_pull": v.1,
                        "to_delete": v.2, "conflicts": v.3,
                        "to_delete_remote": v.4,
                    }))
                }).collect()
            );
            eprintln!("[rift] reconcile complete: to_push={to_push} to_pull={to_pull} to_delete={to_delete} to_delete_remote={to_delete_remote} conflicts={conflicts} by_resource={breakdown}");

            // Reconcile is read-only: it refreshes the cached scan result.
            // User must click Push Now / Pull Now to act on the findings.
            diagnostics::emit_with_fields(
                DiagStage::DriftScanResult,
                DiagLevel::Info,
                None,
                None,
                if result.cancelled { "reconcile cancelled" } else { "reconcile complete" },
                serde_json::json!({
                    "entries": result.entries.len(),
                    "to_push": to_push,
                    "to_pull": to_pull,
                    "to_delete": to_delete,
                    "to_delete_remote": to_delete_remote,
                    "conflicts": conflicts,
                    "enqueued_for_push": 0,
                    "missing_remote_folders": result.remote_folders_missing.len(),
                    "listing_error": result.last_batch_listing_error,
                    "cancelled": result.cancelled,
                    "by_resource": breakdown,
                }),
            );
            // Clear token slot if it's still ours (another kick could have replaced it).
            if let Ok(mut g) = engine.current_scan_cancel.lock() {
                if let Some(stored) = g.as_ref() {
                    if stored.is_cancelled() == ct_for_task.is_cancelled() {
                        *g = None;
                    }
                }
            }
        });
        self.track_background(h);
    }

    /// Cache the most recent scan result entries (called from drift_watcher
    /// run_tick AND kick_drift_reconcile). `force_pull_now` dispatches from
    /// this cache — drift_watcher's 10s cadence keeps it fresh.
    pub(crate) fn cache_scan_entries(&self, entries: Vec<crate::sync::DriftEntry>) {
        if let Ok(mut g) = self.last_scan_entries.lock() {
            *g = entries;
        }
    }

    /// Snapshot of folders the last reconcile aborted via the suspicious-shrink
    /// guard. Surfaced to the frontend rebaseline banner. v0.2.49.
    pub fn aborted_shrunk(&self) -> Vec<crate::sync::AbortedShrunkFolder> {
        self.last_aborted_shrunk
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Rebaseline a single folder: re-list remote authoritatively + walk local,
    /// re-hash from disk (no SHA trust), atomically replace snapshot rows under
    /// the remote_root prefix. Files present on both sides → fresh Synced
    /// baseline. Files local-only → snapshot row dropped → next scan buckets
    /// as ToPush. Files remote-only → row dropped → next scan buckets as
    /// ToPull. Clears the matching aborted_shrunk entry + kicks reconcile.
    /// Returns (old_count, new_count, queued_for_push_estimate).
    pub async fn rebaseline_folder(
        self: &Arc<Self>,
        remote_subpath: &str,
    ) -> Result<(usize, usize, usize), String> {
        let remote_root = format!(
            "{}/{}",
            self.profile.remote_root.trim_end_matches('/'),
            remote_subpath.trim_start_matches('/').trim_end_matches('/')
        );
        let local_root = Path::new(&self.profile.local_root)
            .join(remote_subpath.replace('/', std::path::MAIN_SEPARATOR_STR));

        let listings = self
            .sftp
            .list_recursive_batch(&[remote_root.clone()], 12, None, 4)
            .await
            .map_err(|e| format!("rebaseline list: {e}"))?;
        let remote_hits = listings.get(&remote_root).cloned().unwrap_or_default();

        let remote_root_trim = remote_root.trim_end_matches('/').to_string();
        let mut remote_rel: std::collections::HashMap<String, (i64, chrono::DateTime<Utc>)> =
            std::collections::HashMap::new();
        for r in &remote_hits {
            if r.is_dir {
                continue;
            }
            let rel = match r.full_path.strip_prefix(&remote_root_trim) {
                Some(t) => t.trim_start_matches('/').to_string(),
                None => r.name.clone(),
            };
            if crate::sync::ignore::should_ignore(&rel) {
                continue;
            }
            remote_rel.insert(rel, (r.size as i64, r.last_modified));
        }

        let local_root_owned = local_root.clone();
        let local_rel: std::collections::HashMap<String, (i64, chrono::DateTime<Utc>, PathBuf)> =
            tokio::task::spawn_blocking(move || {
                let mut m = std::collections::HashMap::new();
                if local_root_owned.exists() {
                    walk_local_rebaseline(&local_root_owned, &local_root_owned, &mut m);
                }
                m
            })
            .await
            .map_err(|e| format!("local walk: {e}"))?;

        let mut new_entries: std::collections::HashMap<String, crate::state::sync_snapshot::Entry> =
            std::collections::HashMap::new();
        let mut local_only = 0usize;
        for (rel, (rsize, rmtime)) in &remote_rel {
            if let Some((lsize, lmtime, lpath)) = local_rel.get(rel) {
                let key = format!("{remote_root_trim}/{rel}");
                let sha = if *lsize <= crate::state::sync_snapshot::SHA1_MAX_BYTES {
                    crate::state::SyncSnapshot::compute_sha1(lpath)
                } else {
                    None
                };
                new_entries.insert(
                    key,
                    crate::state::sync_snapshot::Entry {
                        local_size: *lsize,
                        local_mtime_utc: *lmtime,
                        remote_size: *rsize,
                        remote_mtime_utc: *rmtime,
                        sha1: sha,
                    },
                );
            }
        }
        for rel in local_rel.keys() {
            if !remote_rel.contains_key(rel) {
                local_only += 1;
            }
        }

        let (old_count, new_count) = self
            .snapshot
            .replace_under(&remote_root_trim, new_entries)
            .map_err(|e| format!("snapshot replace: {e}"))?;

        if let Ok(mut g) = self.last_aborted_shrunk.lock() {
            g.retain(|a| a.remote_root != remote_root_trim);
        }

        diagnostics::emit_with_fields(
            DiagStage::BaselineRebaselined,
            DiagLevel::Info,
            Some(remote_subpath),
            None,
            format!(
                "rebaselined {remote_subpath}: snapshot {old_count} → {new_count}, {local_only} local-only queued for push"
            ),
            serde_json::json!({
                "remote_subpath": remote_subpath,
                "remote_root": remote_root_trim,
                "old_count": old_count,
                "new_count": new_count,
                "local_only": local_only,
            }),
        );

        self.kick_drift_reconcile();
        Ok((old_count, new_count, local_only))
    }

    /// Snapshot the cached drift entries for the Sync page. Read-only —
    /// returns a clone so the frontend can render without holding the lock.
    pub fn drift_snapshot(&self) -> Vec<crate::sync::DriftEntry> {
        match self.last_scan_entries.lock() {
            Ok(g) => g.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Dispatch a user-selected subset of cached drift entries. Each path is
    /// matched against `last_scan_entries`; the entry's bucket determines the
    /// action (ToPull → pull_one, ToDelete → delete_local_one, ToPush →
    /// enqueue dirty). Circuit breaker runs per-resource on the selected
    /// deletes — same formula as force_pull_now. Returns (dispatched, blocked).
    pub fn apply_selected(self: &Arc<Self>, local_paths: Vec<String>) {
        let engine = self.clone();
        let h = tokio::spawn(async move {
            let cache: Vec<crate::sync::DriftEntry> = engine.drift_snapshot();
            let want: std::collections::HashSet<String> = local_paths.into_iter().collect();
            let selected: Vec<crate::sync::DriftEntry> = cache
                .into_iter()
                .filter(|e| want.contains(&e.local_path))
                .collect();

            let mut delete_buckets: std::collections::HashMap<String, Vec<crate::sync::DriftEntry>> =
                std::collections::HashMap::new();
            let mut pulls: Vec<crate::sync::DriftEntry> = Vec::new();
            let mut pushes: Vec<crate::sync::DriftEntry> = Vec::new();
            // v0.2.53 Mirror: ToDeleteRemote entries delete the remote file
            // via sftp.delete (which routes dirs through delete_recursive_via).
            // Reached this dispatch via the UI's typed "MIRROR" confirm gate,
            // so we trust the selection and skip the local mass-delete guard
            // (that guard exists for ToDelete which deletes LOCAL files).
            let mut remote_deletes: Vec<crate::sync::DriftEntry> = Vec::new();
            for e in selected {
                match e.bucket {
                    crate::sync::DriftBucket::ToDelete => {
                        delete_buckets.entry(e.resource_name.clone()).or_default().push(e);
                    }
                    crate::sync::DriftBucket::ToDeleteRemote => remote_deletes.push(e),
                    crate::sync::DriftBucket::ToPull => pulls.push(e),
                    crate::sync::DriftBucket::ToPush => pushes.push(e),
                    _ => {}
                }
            }

            // Policy change (v0.2.45): explicit user selection from the Sync
            // page IS the override for the mass-delete circuit breaker. The
            // breaker exists to catch SCAN-DRIVEN runaways (auto-pull,
            // tombstone propagation in force_pull_now) — those still hard-block.
            // When the user ticks checkboxes and clicks Apply, that's
            // informed consent: they saw each path. We log a WARN to the
            // activity feed when threshold is exceeded so the action is still
            // visible/auditable, but dispatch the deletes.
            let folders_snap = engine.folders_clone();
            let mut to_dispatch_deletes: Vec<crate::sync::DriftEntry> = Vec::new();
            for (resource, deletes) in delete_buckets {
                let fw = folders_snap.iter().find(|f| f.resource_name == resource);
                let (threshold, total) = match fw {
                    Some(f) => engine.scaled_delete_threshold(&f.local_root).await,
                    None => (5usize, 0usize),
                };
                let count = deletes.len();
                if count >= threshold {
                    let reason = format!(
                        "{count} local-deletes (\u{2265} scaled threshold {threshold} of {total} files) — user-selected, dispatching anyway"
                    );
                    let row = crate::sync::ActivityRow {
                        at: Utc::now(),
                        resource: resource.clone(),
                        file: "[guard-override]".into(),
                        action: format!("WARN — {reason}"),
                        kind: crate::sync::ActivityKind::Drift,
                        actor: Some(crate::transport::env::current_user()),
                        ..Default::default()
                    };
                    use tauri::Emitter;
                    let _ = engine.app().emit("autosync://activity", &row);
                }
                to_dispatch_deletes.extend(deletes);
            }

            let sem = Arc::new(tokio::sync::Semaphore::new(4));
            let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
            for entry in pulls {
                let e = engine.clone();
                let s = sem.clone();
                handles.push(tokio::spawn(async move {
                    let _p = s.acquire_owned().await.ok();
                    crate::sync::drift_watcher::pull_one(&e, entry).await;
                }));
            }
            for entry in to_dispatch_deletes {
                let e = engine.clone();
                let s = sem.clone();
                handles.push(tokio::spawn(async move {
                    let _p = s.acquire_owned().await.ok();
                    crate::sync::drift_watcher::delete_local_one(&e, entry).await;
                }));
            }
            // v0.2.53 Mirror: dispatch remote deletes via the engine's SFTP
            // client. SftpClient::delete routes by remote stat — dirs hit
            // delete_recursive_via, files hit remove_file. Each success
            // forgets the snapshot row so the next scan doesn't re-surface
            // the bucket.
            for entry in remote_deletes {
                let e = engine.clone();
                let s = sem.clone();
                handles.push(tokio::spawn(async move {
                    let _p = s.acquire_owned().await.ok();
                    let started = std::time::Instant::now();
                    let r = e.sftp.delete(&entry.remote_path).await;
                    let elapsed_ms = started.elapsed().as_millis() as u64;
                    if r.success {
                        e.snapshot.forget(&entry.remote_path);
                        e.log_activity_rich(
                            &entry.resource_name,
                            file_name(std::path::Path::new(&entry.local_path)),
                            "remote deleted (Mirror)",
                            Some(entry.rel_path.clone()),
                            Some(entry.local_path.clone()),
                            None,
                            Some(elapsed_ms),
                        );
                        diagnostics::emit_for(
                            DiagStage::UploadDone,
                            DiagLevel::Info,
                            Some(&entry.resource_name),
                            Some(&entry.remote_path),
                            "mirror delete ok",
                        );
                    } else {
                        e.log_activity(
                            &entry.resource_name,
                            file_name(std::path::Path::new(&entry.local_path)),
                            &format!("mirror delete failed: {}", r.error),
                        );
                        diagnostics::emit_for(
                            DiagStage::UploadFail,
                            DiagLevel::Warn,
                            Some(&entry.resource_name),
                            Some(&entry.remote_path),
                            &format!("mirror delete failed: {}", r.error),
                        );
                    }
                }));
            }
            let mut enqueued_push = false;
            for entry in pushes {
                let path = PathBuf::from(&entry.local_path);
                if path.exists() {
                    engine.queue_path(path, ChangeKind::Modified);
                    enqueued_push = true;
                }
            }
            for h in handles {
                let _ = h.await;
            }
            if enqueued_push {
                engine.flush_all_now(None).await;
            }
        });
        self.track_background(h);
    }

    /// Force-pull NOW — runs an inline drift scan and dispatches pulls +
    /// approved-deletes against the result. Uses cached ToPull entries
    /// when populated by a prior `kick_drift_reconcile` so back-to-back
    /// clicks don't re-scan; otherwise an inline scan runs first. Mass
    /// local-delete circuit breaker fires before dispatch. Pull progress
    /// surfaces via RemotePullStart/Done into the activity feed.
    pub fn force_pull_now(self: &Arc<Self>) {
        eprintln!("[rift] force_pull_now: entry");
        // Register a cancellation token so the modal's Cancel button can stop
        // the operation mid-flight. Replaces any prior token — kick_drift_reconcile
        // sharing the same slot means the user's "cancel" always hits the latest
        // long-running op, whichever it is.
        let ct = CancellationToken::new();
        let ct_for_task = ct.clone();
        if let Ok(mut g) = self.current_scan_cancel.lock() {
            *g = Some(ct);
        }
        let engine = self.clone();
        let h = tokio::spawn(async move {
            eprintln!("[rift] force_pull_now: task spawned");
            diagnostics::emit(
                DiagStage::DriftScanStart,
                DiagLevel::Info,
                "pull-now",
            );

            // Pull from cache when available; otherwise run an inline scan
            // first so Pull Now isn't a silent no-op on a cold session. The
            // drift_watcher's first periodic tick can be 30-60s out on slow
            // links — making the user wait was the "Pull Now does nothing"
            // bug Blazzer reported. Inline scan respects the cancel token.
            let cached: Vec<crate::sync::DriftEntry> = {
                let g = match engine.last_scan_entries.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                g.clone()
            };
            let entries = if cached.is_empty() {
                diagnostics::emit(
                    DiagStage::DriftScanStart,
                    DiagLevel::Info,
                    "pull-now: cache empty — inline scan first",
                );
                let folders = engine.folders_clone();
                let targets: Vec<crate::sync::drift_scanner::FolderTarget> = folders
                    .iter()
                    .map(|fw| crate::sync::drift_scanner::FolderTarget {
                        resource_name: fw.resource_name.clone(),
                        local_root: fw.local_root.to_string_lossy().to_string(),
                        remote_root: fw.remote_root.clone(),
                    })
                    .collect();
                let snap = engine.snapshot();
                let sftp = engine.sftp();
                let scanner = crate::sync::drift_scanner::DriftScanner::new(&sftp, Some(&snap))
                    .with_mirror(engine.mirror_mode.load(Ordering::Relaxed));
                let result = scanner.scan_with_cancel(&targets, Some(&ct_for_task)).await;
                if !result.cancelled {
                    engine.cache_scan_entries(result.entries.clone());
                }
                if result.cancelled {
                    diagnostics::emit_with_fields(
                        DiagStage::DriftScanResult,
                        DiagLevel::Info,
                        None, None,
                        "pull-now cancelled during scan",
                        serde_json::json!({
                            "entries": 0, "to_push": 0, "to_pull": 0, "to_delete": 0,
                            "conflicts": 0, "enqueued_for_push": 0, "pull_dispatched": 0,
                            "missing_remote_folders": 0, "listing_error": null,
                            "cancelled": true,
                        }),
                    );
                    return;
                }
                result.entries
            } else {
                cached
            };

            let mut to_pull = 0u32;
            let mut to_push = 0u32;
            let mut to_delete = 0u32;
            let mut conflicts = 0u32;
            // Each pending entry tagged with its bucket so the dispatcher can
            // route ToPull → pull_one vs ToDelete → delete_local_one.
            let mut pending: Vec<(crate::sync::DriftBucket, crate::sync::DriftEntry)> = Vec::new();
            let mut delete_buckets: std::collections::HashMap<String, Vec<crate::sync::DriftEntry>> =
                std::collections::HashMap::new();
            let mut to_delete_remote = 0u32;
            for e in entries {
                match e.bucket {
                    crate::sync::DriftBucket::ToPush => to_push += 1,
                    crate::sync::DriftBucket::ToPull => {
                        to_pull += 1;
                        pending.push((crate::sync::DriftBucket::ToPull, e));
                    }
                    crate::sync::DriftBucket::ToDelete => {
                        to_delete += 1;
                        delete_buckets.entry(e.resource_name.clone()).or_default().push(e);
                    }
                    // v0.2.53 Mirror: ToDeleteRemote is dispatched ONLY via
                    // explicit user selection from the Sync page (the typed-
                    // confirm gate). force_pull_now is the Pull-all path and
                    // never fires remote deletes — count for visibility only.
                    crate::sync::DriftBucket::ToDeleteRemote => to_delete_remote += 1,
                    crate::sync::DriftBucket::Conflict => {
                        conflicts += 1;
                        crate::sync::drift_watcher::register_conflict(&engine, e);
                    }
                    crate::sync::DriftBucket::Synced => {}
                }
            }
            // Mass local-delete circuit breaker — mirrors push-side guard.
            // If a resource's pending delete batch crosses the scaled
            // threshold, drop the batch + emit one [guard] activity row.
            // Without this, a teammate's mass-delete or server cleanup
            // would silently nuke local files when the user clicks Pull.
            let folders_snap = engine.folders_clone();
            for (resource, deletes) in delete_buckets {
                let fw = folders_snap.iter().find(|f| f.resource_name == resource);
                let (threshold, total) = match fw {
                    Some(f) => engine.scaled_delete_threshold(&f.local_root).await,
                    None => (5usize, 0usize),
                };
                let count = deletes.len();
                if count >= threshold {
                    let reason = format!(
                        "{count} local-deletes in one batch (\u{2265} scaled threshold {threshold} of {total} files)"
                    );
                    diagnostics::emit_with_fields(
                        DiagStage::DriftScanResult,
                        DiagLevel::Warn,
                        Some(&resource),
                        None,
                        format!("BLOCKED — {reason}"),
                        serde_json::json!({
                            "guard": "local_delete_mass",
                            "resource": resource,
                            "count": count,
                            "threshold": threshold,
                            "total_files": total,
                        }),
                    );
                    let row = crate::sync::ActivityRow {
                        at: Utc::now(),
                        resource: resource.clone(),
                        file: "[guard]".into(),
                        action: format!("BLOCKED — {reason}"),
                        kind: crate::sync::ActivityKind::Block,
                        actor: Some(crate::transport::env::current_user()),
                        ..Default::default()
                    };
                    use tauri::Emitter;
                    let _ = engine.app().emit("autosync://activity", &row);
                } else {
                    for d in deletes {
                        pending.push((crate::sync::DriftBucket::ToDelete, d));
                    }
                }
            }
            // Cap concurrent pulls so a slow uplink doesn't drown the SFTP
            // session with N parallel downloads. 4 was the sweet spot for
            // Trey's Tailscale link; tunable later.
            let sem = Arc::new(tokio::sync::Semaphore::new(4));
            let mut pull_dispatched = 0u32;
            let mut cancelled = false;
            // Track handles locally so we can await them before emitting the
            // final result. Prior code spawned + emitted instantly, so Cancel
            // never reached in-flight pulls and the modal had no way to know
            // when work actually stopped.
            let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
            for (bucket, entry) in pending {
                if ct_for_task.is_cancelled() {
                    cancelled = true;
                    break;
                }
                let task_engine = engine.clone();
                let permit_sem = sem.clone();
                let ct_inner = ct_for_task.clone();
                let h = tokio::spawn(async move {
                    let _permit = permit_sem.acquire_owned().await.ok();
                    // Second cancel check — N tasks queue on the 4-permit
                    // semaphore; without this, cancelling mid-burst still
                    // drains the entire queue once permits free up.
                    if ct_inner.is_cancelled() { return; }
                    match bucket {
                        crate::sync::DriftBucket::ToDelete => {
                            crate::sync::drift_watcher::delete_local_one(&task_engine, entry).await;
                        }
                        _ => {
                            crate::sync::drift_watcher::pull_one(&task_engine, entry).await;
                        }
                    }
                });
                handles.push(h);
                pull_dispatched += 1;
            }

            // On cancel, orphan in-flight downloads into the background-task
            // tracker so the modal closes immediately. Awaiting handles here
            // was the "Stop pull does nothing visible" lie — modal hung at
            // "Pulling…" until the last in-flight download finished (could
            // be minutes on a slow link). In-flight downloads still complete,
            // they just don't block the result emit.
            if ct_for_task.is_cancelled() {
                cancelled = true;
                for h in handles {
                    engine.track_background(h);
                }
            } else {
                for h in handles {
                    let _ = h.await;
                }
                if ct_for_task.is_cancelled() { cancelled = true; }
            }
            // Symmetric w/ force_push_now: clear the scan cache after a
            // non-cancelled pull that actually moved files. Without this,
            // re-clicking Pull would re-pull the same entries from the
            // stale cache (sha-collapse hides it, but the count lies).
            // Next Pull triggers a fresh inline scan.
            if !cancelled && pull_dispatched > 0 {
                engine.cache_scan_entries(Vec::new());
                eprintln!("[rift] force_pull_now: cleared scan cache (was stale post-pull)");
            }
            diagnostics::emit_with_fields(
                DiagStage::DriftScanResult,
                DiagLevel::Info,
                None,
                None,
                if cancelled {
                    format!("pull-now cancelled after {pull_dispatched} dispatched")
                } else if pull_dispatched > 0 {
                    format!("pull-now: dispatched {pull_dispatched} pull(s) from cached scan")
                } else {
                    "pull-now: nothing pending (drift watcher already caught up)".to_string()
                },
                serde_json::json!({
                    "entries": 0,
                    "to_push": to_push,
                    "to_pull": to_pull,
                    "to_delete": to_delete,
                    "to_delete_remote": to_delete_remote,
                    "conflicts": conflicts,
                    "enqueued_for_push": 0,
                    "pull_dispatched": pull_dispatched,
                    "missing_remote_folders": 0,
                    "listing_error": null,
                    "cancelled": cancelled,
                    "from_cache": true,
                }),
            );
            // Clear token slot if it's still ours.
            if let Ok(mut g) = engine.current_scan_cancel.lock() {
                if let Some(stored) = g.as_ref() {
                    if stored.is_cancelled() == ct_for_task.is_cancelled() {
                        *g = None;
                    }
                }
            }
        });
        self.track_background(h);
    }

    /// Cancel the in-flight drift reconcile if one is running. No-op otherwise.
    /// The running scan checks the token between folders and bails with a
    /// `cancelled: true` ScanResult; the modal surfaces that as "cancelled".
    pub fn cancel_drift_reconcile(&self) {
        if let Ok(g) = self.current_scan_cancel.lock() {
            if let Some(ct) = g.as_ref() {
                ct.cancel();
            }
        }
    }

    /// Mark a path as just-written by Rift so the next ~5s of fs-events
    /// for it get suppressed. Closes the pull→push loop caused by the
    /// Windows atomic-replace Delete+Modify burst that the `.rift-tmp`
    /// ignore rule doesn't cover (the real-file events, not the tmp).
    pub fn mark_recently_written(&self, path: &std::path::Path) {
        self.recently_written
            .insert(path.to_path_buf(), std::time::Instant::now());
    }

    /// Returns true if `path` was marked via `mark_recently_written`
    /// within the suppression window. Side-effect: lazily evicts the
    /// entry on a hit so the map doesn't grow unbounded.
    fn is_recently_written(&self, path: &std::path::Path) -> bool {
        const WINDOW: std::time::Duration = std::time::Duration::from_secs(5);
        let Some(entry) = self.recently_written.get(path) else {
            return false;
        };
        let fresh = entry.elapsed() < WINDOW;
        drop(entry);
        if !fresh {
            self.recently_written.remove(path);
        }
        fresh
    }

    fn queue_path(&self, path: PathBuf, kind: ChangeKind) {
        let path_str = path.to_string_lossy().to_string();
        if self.is_recently_written(&path) {
            self.log(&format!(
                "fs ignore [recent-pull]: {kind:?} {}",
                path.display()
            ));
            self.ignored_total.fetch_add(1, Ordering::Relaxed);
            self.ignored_by_rule
                .entry("recent-pull".to_string())
                .and_modify(|n| *n += 1)
                .or_insert(1);
            diagnostics::emit_with_fields(
                DiagStage::Ignored,
                DiagLevel::Debug,
                None,
                Some(&path_str),
                "ignored [recent-pull]",
                serde_json::json!({ "rule": "recent-pull", "kind": format!("{kind:?}") }),
            );
            return;
        }
        if let Some(rule) = ignore::classify(&path_str) {
            self.log(&format!("fs ignore [{rule}]: {kind:?} {}", path.display()));
            self.ignored_total.fetch_add(1, Ordering::Relaxed);
            self.ignored_by_rule
                .entry(rule.to_string())
                .and_modify(|n| *n += 1)
                .or_insert(1);
            diagnostics::emit_with_fields(
                DiagStage::Ignored,
                DiagLevel::Debug,
                None,
                Some(&path_str),
                format!("ignored [{rule}]"),
                serde_json::json!({ "rule": rule, "kind": format!("{kind:?}") }),
            );
            return;
        }
        if self.is_manual_delete_suppressed(&path) {
            diagnostics::emit_for(
                DiagStage::Ignored,
                DiagLevel::Debug,
                None,
                Some(&path_str),
                "ignored [manual-delete-suppression]",
            );
            return;
        }
        // Skip directory events (children fire their own).
        let is_dir = kind != ChangeKind::Deleted && path.is_dir();
        if is_dir {
            return;
        }
        // Resolve owning watch.
        let mut owner: Option<String> = None;
        let mut best_len = 0usize;
        for kv in self.folders.iter() {
            let root = &kv.value().local_root;
            if path.starts_with(root) {
                let l = root.components().count();
                if l > best_len {
                    best_len = l;
                    owner = Some(kv.key().clone());
                }
            }
        }
        let Some(watch_key) = owner else { return };
        let now = Utc::now();
        let debounce = chrono::Duration::milliseconds(DEBOUNCE_MS as i64);
        let ceiling = chrono::Duration::milliseconds(CEILING_MS as i64);
        let watch_key_for_lock = watch_key.clone();
        let first_dirty = match self.dirty.entry(path.clone()) {
            Entry::Occupied(mut occupied) => {
                let existing = occupied.get_mut();
                let cap = existing.first_seen + ceiling;
                let fresh = now + debounce;
                existing.kind = merge_kind(existing.kind, kind);
                existing.next_flush = if fresh > cap { cap } else { fresh };
                false
            }
            Entry::Vacant(vacant) => {
                vacant.insert(DirtyEntry {
                    watch_key,
                    path: path.clone(),
                    kind,
                    first_seen: now,
                    next_flush: now + debounce,
                    attempts: 0,
                    next_retry: now,
                    bypass_preflight: false,
                });
                true
            }
        };
        diagnostics::emit_for(
            DiagStage::Queued,
            DiagLevel::Debug,
            None,
            Some(&path_str),
            format!("queued ({kind:?})"),
        );

        // Drop a presence lock fire-and-forget on the FIRST dirty event for this
        // path in a cycle. Released on flush (process_entry wrapper releases on
        // every terminal result — Ok or Fail) or on Deleted (release here so
        // renamed-old-path locks don't leak).
        //
        // Gate acquire on `path.is_file()` — never acquire a lock for a
        // directory path. notify can emit Modified events with dir paths on
        // Windows when child files change; previously these queued, the
        // upload step failed (read-dir-as-file), and the orphan `<dir>.rift-
        // lock` was never released. Pattern observed 2026-05-12 with
        // `[depend]/oxmysql.rift-lock`, `[community]/illenium-appearance/sql.
        // rift-lock`, etc. Directories never need presence locks anyway —
        // their constituent files each get one.
        if first_dirty || kind == ChangeKind::Deleted {
            if let Some(locks) = self.locks.clone() {
            if let Some(fw) = self.folders.get(&watch_key_for_lock).map(|v| v.value().clone()) {
                if let Some(remote) = map_local_to_remote(&path, &fw) {
                    let kind_at_queue = kind;
                    let path_for_check = path.clone();
                    let h = tokio::spawn(async move {
                        if kind_at_queue == ChangeKind::Deleted {
                            locks.release(&remote).await;
                        } else if path_for_check.is_file() {
                            locks.acquire(&remote).await;
                        }
                    });
                    self.track_background(h);
                }
            }
            }
        }
    }

    async fn flush_batch(
        &self,
        fw: &FolderWatch,
        entries: Vec<DirtyEntry>,
        cancel: Option<CancellationToken>,
    ) -> u32 {
        // ── Mass-delete circuit breaker ───────────────────────────────────
        let delete_count = entries.iter().filter(|e| e.kind == ChangeKind::Deleted).count();
        let local_root_gone = !fw.local_root.exists();
        let local_file_count = safe_count_files(&fw.local_root).await;
        let scaled_threshold =
            ((local_file_count as f64 * 0.30) as usize).clamp(5, MASS_DELETE_THRESHOLD);
        if local_root_gone || delete_count >= scaled_threshold {
            let reason = if local_root_gone {
                format!("local root vanished ({})", fw.local_root.display())
            } else {
                format!(
                    "{delete_count} deletes in one batch (≥ scaled threshold {scaled_threshold} of {local_file_count} files)"
                )
            };
            self.log(&format!(
                "BLOCKED: {} — {reason}. Watch stopped, pending entries dropped to prevent remote data loss.",
                fw.resource_name
            ));
            self.log_activity(&fw.resource_name, "[guard]", &format!("BLOCKED — {reason}"));
            for e in &entries {
                self.dirty.remove(&e.path);
            }
            self.stop_watch(&fw.remote_root).await;
            self.set_state(
                AutoSyncState::Error,
                format!("BLOCKED · {} · {reason}", fw.resource_name),
            )
            .await;
            return 0;
        }

        self.set_state(
            AutoSyncState::Syncing,
            format!("{} · {} file(s)", fw.resource_name, entries.len()),
        )
        .await;

        // ── Pre-create unique parent dirs on the main session ────────────
        // Per-file `mkdir_p_strict` inside `upload_atomic_via` is the inner
        // fallback, but when N workers race the same fresh parent tree
        // (50-file new bracket) any transient mkdir miss can desync the
        // workers. Serializing here = single authoritative mkdir per unique
        // parent, errors surface immediately, and the parallel upload loop
        // runs against a known-good tree. Fixes Bug 2/4 from the 2026-05-12
        // Endure RP report (entire `[depend]/oxmysql/` directory missing
        // after Push). Symmetric with `upload_files_batch` at
        // sftp/transfer.rs:116-128 which already did this for its codepath.
        {
            let mut unique_parents: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for e in &entries {
                if e.kind == ChangeKind::Deleted {
                    continue;
                }
                if let Some(remote) = map_local_to_remote(&e.path, fw) {
                    if let Some(p) = crate::sftp::ops::remote_parent(&remote) {
                        if p != "/" && !p.is_empty() {
                            unique_parents.insert(p.to_string());
                        }
                    }
                }
            }
            for parent in &unique_parents {
                if let Err(err) = self.sftp.mkdir_p_strict(parent).await {
                    self.log_activity(
                        &fw.resource_name,
                        "[mkdir]",
                        &format!("pre-create {parent} failed: {err}"),
                    );
                    diagnostics::emit_with_fields(
                        DiagStage::UploadFail,
                        DiagLevel::Warn,
                        Some(&fw.resource_name),
                        Some(parent),
                        format!("pre-mkdir failed: {err}"),
                        serde_json::json!({ "op": "pre_mkdir", "error": err }),
                    );
                    // Don't bail the batch — children may still succeed
                    // (network blip, race-with-other-worker). The per-file
                    // strict mkdir is the safety net; this is the fast path.
                }
            }
        }

        // Bounded concurrency.
        use futures::stream::{FuturesUnordered, StreamExt};
        let mut futs = FuturesUnordered::new();
        let mut iter = entries.into_iter();
        let mut active = 0usize;
        let mut dispatched = 0u32;
        let mut ok = 0u32;
        let mut fail = 0u32;
        let mut trail_items: Vec<(String, String)> = Vec::new();

        loop {
            while active < UPLOAD_CONCURRENCY {
                // Check cancel BEFORE popping the next entry from dirty. On
                // bail, remaining items stay in the dirty map so the next
                // Push click resumes where this one left off.
                if let Some(ct) = &cancel {
                    if ct.is_cancelled() {
                        break;
                    }
                }
                let Some(entry) = iter.next() else { break };
                // Lazy pop: only remove from dirty once we've committed to
                // dispatching this entry. A re-dirty during upload still
                // creates a new entry (path no longer in map).
                self.dirty.remove(&entry.path);
                let fw = fw.clone();
                let ct_per_entry = cancel.clone();
                futs.push(async move { self.process_entry(&fw, entry, ct_per_entry).await });
                active += 1;
                dispatched += 1;
            }
            let Some(out) = futs.next().await else { break };
            active -= 1;
            match out {
                EntryResult::Ok(rel, action) => {
                    ok += 1;
                    if let Some(rel) = rel {
                        trail_items.push((rel, action));
                    }
                }
                EntryResult::Fail => fail += 1,
                EntryResult::Requeued => {}
            }
        }

        if ok > 0 || fail > 0 {
            // v0.2.52: smarter escalation. Don't flip to Error on a single
            // fail (editor-tmp race, vanished-rename source path, etc.).
            // Track consecutive-fail batches; only escalate after 3+ in a
            // row with no clean batch between. Any clean batch resets the
            // counter. ConnectionWedged is handled independently by the
            // sftp/transfer.rs with_t() helper which emits its own diag.
            const FAIL_STREAK_THRESHOLD: u64 = 3;
            let streak = if fail > 0 {
                self.consecutive_failed_batches.fetch_add(1, Ordering::Relaxed) + 1
            } else {
                self.consecutive_failed_batches.store(0, Ordering::Relaxed);
                0
            };
            let next = if streak >= FAIL_STREAK_THRESHOLD {
                AutoSyncState::Error
            } else if fail > 0 {
                // Single/double fail — surface in detail but stay Watching.
                AutoSyncState::Watching
            } else {
                AutoSyncState::Idle
            };
            let detail = if streak >= FAIL_STREAK_THRESHOLD {
                format!("{}: {} ok, {} failed ({}× consecutive)", fw.resource_name, ok, fail, streak)
            } else if fail > 0 {
                format!("{}: {} ok, {} retry pending", fw.resource_name, ok, fail)
            } else {
                format!("{}: {} synced", fw.resource_name, ok)
            };
            self.set_state(next, detail).await;
        }

        // Bridge `/sync-done` ping for FXServer hot-reload — fire only when at
        // least one upload succeeded. Failures don't escalate; we log activity.
        if ok > 0 {
            if let Some(bridge) = self.bridge.clone() {
                let resource = fw.resource_name.clone();
                let app = self.app.clone();
                let h = tokio::spawn(async move {
                    diagnostics::emit_for(
                        DiagStage::BridgePing,
                        DiagLevel::Debug,
                        Some(&resource),
                        None,
                        "sync-done ping",
                    );
                    let started = std::time::Instant::now();
                    let r = bridge.sync_done(&resource).await;
                    let elapsed_ms = started.elapsed().as_millis() as u64;
                    let row = ActivityRow {
                        at: Utc::now(),
                        resource: resource.clone(),
                        file: "[bridge]".into(),
                        action: if r.success {
                            "restart triggered".into()
                        } else if !r.error.is_empty() {
                            format!("restart failed: {}", r.error)
                        } else {
                            format!("restart failed: HTTP {}", r.status)
                        },
                        kind: ActivityKind::Bridge,
                        latency_ms: Some(elapsed_ms),
                        actor: Some(crate::transport::env::current_user()),
                        ..Default::default()
                    };
                    let _ = app.emit("autosync://activity", &row);
                    diagnostics::emit_with_fields(
                        DiagStage::BridgeAck,
                        if r.success { DiagLevel::Info } else { DiagLevel::Warn },
                        Some(&resource),
                        None,
                        if r.success { "bridge ack".into() } else { format!("bridge fail: {}", r.error) },
                        serde_json::json!({
                            "success": r.success,
                            "status": r.status,
                            "error": r.error,
                            "elapsed_ms": elapsed_ms,
                        }),
                    );
                });
                self.track_background(h);
            }
        }

        // Edit trail — fire-and-forget so it never blocks the next cycle.
        if !trail_items.is_empty() {
            let sftp = self.sftp.clone();
            let remote_root = fw.remote_root.clone();
            let h = tokio::spawn(async move {
                let trail = crate::sync::EditTrail::new(&sftp);
                let _ = trail.append(&remote_root, &trail_items).await;
            });
            self.track_background(h);
        }

        dispatched
    }

    async fn process_entry(
        &self,
        fw: &FolderWatch,
        entry: DirtyEntry,
        cancel: Option<CancellationToken>,
    ) -> EntryResult {
        // Outer cancel guard wraps the WHOLE function body. Inner awaits
        // (conflict pre-flight, remote_stat, mkdir, upload) all become
        // cancellable — whichever await is pending at the moment cancel
        // fires gets dropped, function returns Requeued. Belt-and-suspenders
        // with the inner select! around upload_file_atomic.
        let entry_for_requeue = entry.clone();
        let fw_name = fw.resource_name.clone();
        let file_for_log = file_name(&entry_for_requeue.path).to_string();
        let entry_path_for_release = entry.path.clone();
        let fw_for_release = fw.clone();
        let work = self.process_entry_body(fw, entry, cancel.clone());
        let result = if let Some(ct) = cancel {
            tokio::select! {
                biased;
                _ = ct.cancelled() => {
                    self.dirty.insert(entry_for_requeue.path.clone(), entry_for_requeue);
                    self.log_activity(&fw_name, &file_for_log,
                        "cancelled mid-flight — requeued");
                    EntryResult::Requeued
                }
                r = work => r,
            }
        } else {
            work.await
        };
        // Release the presence lock on every terminal result. Without this,
        // any non-Ok exit (Fail, conflict, vanish, read-error) leaked the
        // `<file>.rift-lock` on the remote because release only fired in the
        // success branch of process_entry_body. Pattern observed 2026-05-12
        // — 44 orphan locks across the prod tree. Requeued is intentionally
        // skipped: the entry is still pending, keep the lock until it
        // terminally succeeds or fails. Release is idempotent so the inner
        // success-path release is harmless.
        //
        // v0.2.50: switched from `tokio::spawn` + `track_background` to
        // inline-await w/ 5 s timeout. The previous spawn could be aborted
        // by engine `stop()` before the SFTP delete fired, leaving orphan
        // `.rift-lock` files on remote (observed in probe2.txt sanity test:
        // engine flipped to Error on the same batch, aborted the spawned
        // release task before it ran). Inline await blocks this worker for
        // up to 5 s on the release; lock release is a single SFTP delete,
        // typically <100 ms, so the worst-case cost is negligible. The
        // 5 s timeout caps cost on a hung session.
        if !matches!(result, EntryResult::Requeued) {
            if let Some(locks) = self.locks.clone() {
                if let Some(remote) = map_local_to_remote(&entry_path_for_release, &fw_for_release) {
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        locks.release(&remote),
                    ).await;
                }
            }
        }
        result
    }

    async fn process_entry_body(
        &self,
        fw: &FolderWatch,
        entry: DirtyEntry,
        cancel: Option<CancellationToken>,
    ) -> EntryResult {
        let remote = match map_local_to_remote(&entry.path, fw) {
            Some(r) => r,
            None => {
                self.mark_failed(&entry);
                self.log_activity(&fw.resource_name, file_name(&entry.path),
                    "rejected: path outside watch root");
                return EntryResult::Fail;
            }
        };

        // Cancel before doing real work — if Stop fired while this entry was
        // queued in the dispatch loop, re-insert into dirty and bail without
        // touching the SFTP socket.
        if let Some(ct) = &cancel {
            if ct.is_cancelled() {
                self.dirty.insert(entry.path.clone(), entry.clone());
                self.log_activity(&fw.resource_name, file_name(&entry.path),
                    "cancelled — requeued");
                return EntryResult::Requeued;
            }
        }

        if entry.kind == ChangeKind::Deleted {
            self.log_activity(&fw.resource_name, file_name(&entry.path), "deleting…");
            diagnostics::emit_for(
                DiagStage::UploadStart,
                DiagLevel::Debug,
                Some(&fw.resource_name),
                Some(&remote),
                "delete start",
            );
            let r = self.sftp.delete(&remote).await;
            if r.success {
                self.failed.remove(&entry.path);
                self.cache.forget(&remote);
                self.snapshot.forget(&remote);
                let rel = rel_of(fw, &entry.path);
                self.log_activity_rich(
                    &fw.resource_name,
                    file_name(&entry.path),
                    "pushed delete",
                    Some(rel.clone()),
                    Some(entry.path.to_string_lossy().to_string()),
                    None,
                    None,
                );
                diagnostics::emit_for(
                    DiagStage::UploadDone,
                    DiagLevel::Info,
                    Some(&fw.resource_name),
                    Some(&remote),
                    "deleted",
                );
                EntryResult::Ok(Some(rel), "deleted".into())
            } else {
                self.mark_failed(&entry);
                self.log_activity(&fw.resource_name, file_name(&entry.path),
                    &format!("delete failed: {}", r.error));
                diagnostics::emit_with_fields(
                    DiagStage::UploadFail,
                    DiagLevel::Warn,
                    Some(&fw.resource_name),
                    Some(&remote),
                    format!("delete failed: {}", r.error),
                    serde_json::json!({ "op": "delete", "error": r.error }),
                );
                EntryResult::Fail
            }
        } else {
            // Wait briefly for the file to be readable (atomic-save mid-rename window).
            if !wait_for_readable(&entry.path).await {
                self.mark_failed(&entry);
                self.log_activity(&fw.resource_name, file_name(&entry.path),
                    "skipped: file locked or unreadable");
                return EntryResult::Fail;
            }
            if !entry.path.exists() {
                self.mark_failed(&entry);
                self.log_activity(&fw.resource_name, file_name(&entry.path),
                    "skipped: file vanished before upload");
                return EntryResult::Fail;
            }

            // Foreign-lock hold — if another dev's Rift is editing this file
            // right now, requeue with 30s delay (NOT a failure — coordination).
            if let Some(locks) = &self.locks {
                if let Some(foreign) = locks.find_lock_by_other(&remote) {
                    let mut requeued = entry.clone();
                    requeued.next_flush =
                        Utc::now() + chrono::Duration::seconds(LOCK_HOLD_RETRY_SEC as i64);
                    self.dirty.insert(entry.path.clone(), requeued);
                    self.log_activity(
                        &fw.resource_name,
                        file_name(&entry.path),
                        &format!(
                            "BLOCKED · locked by {}@{} — waiting",
                            foreign.user, foreign.host
                        ),
                    );
                    diagnostics::emit_with_fields(
                        DiagStage::LockHeldByOther,
                        DiagLevel::Warn,
                        Some(&fw.resource_name),
                        Some(&remote),
                        format!("blocked by {}@{}", foreign.user, foreign.host),
                        serde_json::json!({ "user": foreign.user, "host": foreign.host }),
                    );
                    return EntryResult::Requeued;
                }
            }

            // Conflict pre-flight (3-way against snapshot).
            if !entry.bypass_preflight {
                if let Some(snap) = self.snapshot.try_get(&remote) {
                    let info = self.sftp.remote_stat(&remote).await;
                    if info.exists && !info.is_directory {
                        let remote_changed = !SyncSnapshot::remote_matches(
                            &crate::state::sync_snapshot::Entry {
                                local_size: snap.local_size,
                                local_mtime_utc: snap.local_mtime_utc,
                                remote_size: snap.remote_size,
                                remote_mtime_utc: snap.remote_mtime_utc,
                                sha1: snap.sha1.clone(),
                            },
                            info.size,
                            info.last_modified,
                        );
                        if remote_changed {
                            // SHA-equality collapse — phantom-conflict guard.
                            //
                            // Remote moved by stat (mtime or size differs from
                            // baseline). But if size hasn't actually changed
                            // AND local SHA still matches the baseline AND
                            // remote SHA also matches the baseline, the bytes
                            // never changed on either side — only mtimes did
                            // (npm rebuild touching artifacts, our v0.2.26-31
                            // SETSTAT calls bumping ctime/mtime, etc). Real
                            // case Trey hit on 2026-05-12: 53 phantom conflicts
                            // on `[ox]/ox_lib/web/build/` fonts + bundles.
                            //
                            // Drift scanner already collapses this on scan; we
                            // need the same logic here for queued uploads, b/c
                            // the watcher fires on local mtime-only touches.
                            // Cost: one remote SHA1 SSH-exec when sizes match
                            // — only fires in the false-conflict shape, so the
                            // common (real-change) path is unaffected.
                            let sizes_match = info.size == snap.local_size
                                && info.size == snap.remote_size;
                            if sizes_match && snap.sha1.is_some() {
                                let local_sha =
                                    SyncSnapshot::compute_sha1(&entry.path);
                                if local_sha.is_some() && local_sha == snap.sha1 {
                                    let remote_sha =
                                        self.sftp.get_remote_sha1(&remote).await;
                                    if remote_sha.is_some() && remote_sha == snap.sha1 {
                                        // Phantom — refresh baseline mtime + drop the push.
                                        if let Some((lsize, lmtime)) =
                                            stat_local(&entry.path)
                                        {
                                            self.snapshot.set(
                                                &remote,
                                                lsize,
                                                lmtime,
                                                info.size,
                                                info.last_modified,
                                                snap.sha1.clone(),
                                            );
                                        }
                                        self.cache.set(
                                            &remote,
                                            info.size,
                                            info.last_modified,
                                        );
                                        self.failed.remove(&entry.path);
                                        self.conflicts.remove(&entry.path);
                                        diagnostics::emit_for(
                                            DiagStage::UploadDone,
                                            DiagLevel::Debug,
                                            Some(&fw.resource_name),
                                            Some(&remote),
                                            "phantom-conflict collapsed (SHA-equal)",
                                        );
                                        self.log_activity(&fw.resource_name,
                                            file_name(&entry.path),
                                            "already in sync (mtime jitter)");
                                        let rel = rel_of(fw, &entry.path);
                                        return EntryResult::Ok(
                                            Some(rel),
                                            "synced (mtime jitter)".into(),
                                        );
                                    }
                                }
                            }
                            let (local_size, local_mtime) = stat_local(&entry.path)
                                .unwrap_or((0, Utc::now()));
                            let record = ConflictRecord {
                                local_path: entry.path.to_string_lossy().to_string(),
                                remote_path: remote.clone(),
                                resource_name: fw.resource_name.clone(),
                                local_size,
                                local_mtime_utc: local_mtime,
                                remote_size: info.size,
                                remote_mtime_utc: info.last_modified,
                                last_known_size: snap.remote_size,
                                last_known_mtime_utc: snap.remote_mtime_utc,
                            };
                            self.conflicts.insert(entry.path.clone(), record.clone());
                            self.log(&format!(
                                "CONFLICT: {} — remote moved since last agreement",
                                entry.path.display()
                            ));
                            self.log_activity(&fw.resource_name, file_name(&entry.path),
                                "CONFLICT — remote changed since last sync");
                            let _ = self.app.emit("autosync://conflict", &record);
                            return EntryResult::Requeued;
                        }
                    }
                }
            }

            let upload_started = std::time::Instant::now();
            let local_size = std::fs::metadata(&entry.path).ok().map(|m| m.len()).unwrap_or(0);
            // Surface a start row in the activity feed so the user sees which
            // file is in-flight the moment dispatch happens, not after it
            // completes. Without this, a hung push shows zero activity rows
            // and the modal looks dead.
            self.log_activity(&fw.resource_name, file_name(&entry.path), "uploading…");
            diagnostics::emit_with_fields(
                DiagStage::UploadStart,
                DiagLevel::Debug,
                Some(&fw.resource_name),
                Some(&remote),
                "upload start",
                serde_json::json!({ "size": local_size }),
            );
            // Race the upload against the cancel token — when Stop fires, the
            // upload future drops, russh stops emitting WRITE packets, the
            // atomic tmp file is left on the server (rename never happened
            // so target file is untouched), and we requeue the entry.
            let r = {
                let upload_fut = self.sftp.upload_file_atomic(&entry.path, &remote);
                if let Some(ct) = &cancel {
                    tokio::select! {
                        biased;
                        _ = ct.cancelled() => {
                            self.dirty.insert(entry.path.clone(), entry.clone());
                            self.log_activity(&fw.resource_name, file_name(&entry.path),
                                "upload cancelled — requeued");
                            return EntryResult::Requeued;
                        }
                        result = upload_fut => result,
                    }
                } else {
                    upload_fut.await
                }
            };
            let elapsed_ms = upload_started.elapsed().as_millis() as u64;
            if r.success {
                self.failed.remove(&entry.path);
                // Capture authoritative server mtime.
                let info = self.sftp.remote_stat(&remote).await;
                if info.exists && !info.is_directory {
                    self.cache.set(&remote, info.size, info.last_modified);
                    // Skip baseline write if local stat fails — recording (0, now)
                    // would poison the conflict pre-flight on the next sync.
                    if let Some((lsize, lmtime)) = stat_local(&entry.path) {
                        let sha = if lsize <= SHA1_MAX_BYTES {
                            SyncSnapshot::compute_sha1(&entry.path)
                        } else {
                            None
                        };
                        self.snapshot.set(&remote, lsize, lmtime, info.size, info.last_modified, sha);
                    }
                }
                if let Some(locks) = self.locks.clone() {
                    let r = remote.clone();
                    let h = tokio::spawn(async move { locks.release(&r).await });
                    self.track_background(h);
                }
                let rel = rel_of(fw, &entry.path);
                self.log_activity_rich(
                    &fw.resource_name,
                    file_name(&entry.path),
                    "synced",
                    Some(rel.clone()),
                    Some(entry.path.to_string_lossy().to_string()),
                    Some(local_size as i64),
                    Some(elapsed_ms),
                );
                diagnostics::emit_with_fields(
                    DiagStage::UploadDone,
                    DiagLevel::Info,
                    Some(&fw.resource_name),
                    Some(&remote),
                    "synced",
                    serde_json::json!({ "size": local_size, "elapsed_ms": elapsed_ms }),
                );
                EntryResult::Ok(Some(rel), "synced".into())
            } else {
                self.mark_failed(&entry);
                self.log_activity(&fw.resource_name, file_name(&entry.path),
                    &format!("sync failed: {}", r.error));
                diagnostics::emit_with_fields(
                    DiagStage::UploadFail,
                    DiagLevel::Warn,
                    Some(&fw.resource_name),
                    Some(&remote),
                    format!("sync failed: {}", r.error),
                    serde_json::json!({ "op": "upload", "error": r.error, "elapsed_ms": elapsed_ms }),
                );
                EntryResult::Fail
            }
        }
    }

    async fn update_snapshot_after_sync(&self, local_path: &Path, remote_path: &str) {
        let info = self.sftp.remote_stat(remote_path).await;
        if !info.exists || info.is_directory {
            return;
        }
        self.cache.set(remote_path, info.size, info.last_modified);
        if !local_path.exists() {
            return;
        }
        let Some((lsize, lmtime)) = stat_local(local_path) else { return };
        let sha = if lsize <= SHA1_MAX_BYTES {
            SyncSnapshot::compute_sha1(local_path)
        } else {
            None
        };
        self.snapshot.set(remote_path, lsize, lmtime, info.size, info.last_modified, sha);
    }

    fn mark_failed(&self, entry: &DirtyEntry) {
        let mut e = entry.clone();
        e.attempts += 1;
        if e.attempts as usize > RETRY_BACKOFFS_SECS.len() {
            // Permanent drop — `flush_all_now`'s promote loop will skip it.
            // Surface to logs so the file doesn't disappear silently.
            log::warn!(
                "auto-sync giving up on {} after {} attempts",
                e.path.display(),
                e.attempts
            );
            self.log_activity(
                "[autosync]",
                file_name(&e.path),
                &format!("FAILED — gave up after {} attempts", e.attempts),
            );
        }
        if e.attempts as usize > RETRY_BACKOFFS_SECS.len() {
            self.failed.remove(&entry.path);
            return;
        }
        let idx = std::cmp::min(e.attempts as usize - 1, RETRY_BACKOFFS_SECS.len() - 1);
        e.next_retry = Utc::now() + chrono::Duration::seconds(RETRY_BACKOFFS_SECS[idx] as i64);
        self.failed.insert(entry.path.clone(), e);
    }

    fn remote_for_local(&self, local_path: &Path) -> Option<String> {
        let mut best: Option<FolderWatch> = None;
        let mut best_len = 0usize;
        for kv in self.folders.iter() {
            let fw = kv.value();
            if local_path.starts_with(&fw.local_root) {
                let len = fw.local_root.components().count();
                if len > best_len {
                    best_len = len;
                    best = Some(fw.clone());
                }
            }
        }
        best.and_then(|fw| map_local_to_remote(local_path, &fw))
    }

    fn local_for_remote(&self, remote_path: &str) -> Option<PathBuf> {
        for kv in self.folders.iter() {
            let fw = kv.value();
            let root = fw.remote_root.trim_end_matches('/');
            let rel = if remote_path == root {
                ""
            } else if let Some(tail) = remote_path.strip_prefix(&format!("{root}/")) {
                tail
            } else {
                continue;
            };
            let mut local = fw.local_root.clone();
            if !rel.is_empty() {
                local.push(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            }
            return Some(local);
        }
        None
    }

    fn remove_pending_under(&self, root: &Path) {
        let dirty: Vec<PathBuf> = self
            .dirty
            .iter()
            .filter(|kv| kv.key().starts_with(root))
            .map(|kv| kv.key().clone())
            .collect();
        for key in dirty {
            self.dirty.remove(&key);
        }
        let failed: Vec<PathBuf> = self
            .failed
            .iter()
            .filter(|kv| kv.key().starts_with(root))
            .map(|kv| kv.key().clone())
            .collect();
        for key in failed {
            self.failed.remove(&key);
        }
    }

    fn is_manual_delete_suppressed(&self, path: &Path) -> bool {
        let now = Utc::now();
        let expired: Vec<PathBuf> = self
            .manual_delete_suppress_until
            .iter()
            .filter(|kv| *kv.value() <= now)
            .map(|kv| kv.key().clone())
            .collect();
        for key in expired {
            self.manual_delete_suppress_until.remove(&key);
        }
        self.manual_delete_suppress_until
            .iter()
            .any(|kv| path.starts_with(kv.key()) && *kv.value() > now)
    }


    async fn set_state(&self, s: AutoSyncState, detail: String) {
        {
            let mut g = self.state.lock().await;
            *g = (s, detail);
        }
        self.fire_status().await;
    }

    async fn fire_status(&self) {
        let status = self.status().await;
        let _ = self.app.emit("autosync://status", &status);
    }

    fn log(&self, msg: &str) {
        log::info!("{msg}");
    }

    #[allow(clippy::too_many_arguments)]
    fn log_activity_rich(
        &self,
        resource: &str,
        file: &str,
        action: &str,
        rel_path: Option<String>,
        local_path: Option<String>,
        size_bytes: Option<i64>,
        latency_ms: Option<u64>,
    ) {
        let row = ActivityRow {
            at: Utc::now(),
            resource: resource.to_string(),
            file: file.to_string(),
            action: action.to_string(),
            kind: classify_action(action),
            rel_path,
            local_path,
            size_bytes,
            latency_ms,
            sha: None,
            actor: Some(crate::transport::env::current_user()),
        };
        use tauri::Emitter;
        let _ = self.app.emit("autosync://activity", &row);
    }

    fn log_activity(&self, resource: &str, file: &str, action: &str) {
        let row = ActivityRow {
            at: Utc::now(),
            resource: resource.to_string(),
            file: file.to_string(),
            action: action.to_string(),
            kind: classify_action(action),
            actor: Some(crate::transport::env::current_user()),
            ..Default::default()
        };
        let _ = self.app.emit("autosync://activity", &row);
        log::info!("[{resource}] {file}: {action}");
    }
}

enum EntryResult {
    Ok(Option<String>, String), // (rel_path, action)
    Fail,
    Requeued,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Rebaseline-only local walker. Captures rel_path → (size, mtime_utc, full_path)
/// applying the same ignore rules walk_local in drift_scanner uses. Kept here
/// (vs. reusing drift_scanner::walk_local) because that one returns its own
/// LocalStat struct private to drift_scanner.
fn walk_local_rebaseline(
    root: &Path,
    dir: &Path,
    out: &mut std::collections::HashMap<String, (i64, chrono::DateTime<Utc>, PathBuf)>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if crate::sync::ignore::should_ignore(name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk_local_rebaseline(root, &path, out);
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else { continue };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if crate::sync::ignore::should_ignore(&rel_s) {
            continue;
        }
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
            .unwrap_or_else(Utc::now);
        out.insert(rel_s, (meta.len() as i64, mtime, path));
    }
}

