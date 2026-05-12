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

// ─── Tunables (port from WPF) ────────────────────────────────────────────────
//
// Worst-case file-change → flush latency is `DEBOUNCE_MS + LOOP_TICK_MS` (today
// 700 + 150 = 850ms). Mirrors WPF — surface as per-server config in Phase 6 if
// users start asking.
const DEBOUNCE_MS: u64 = 700;
const CEILING_MS: u64 = 3000;
const LOOP_TICK_MS: u64 = 150;
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

fn classify_action(a: &str) -> ActivityKind {
    let lower = a.to_ascii_lowercase();
    if lower.contains("blocked") {
        ActivityKind::Block
    } else if lower.contains("conflict\u{2192}") || lower.contains("conflict skipped") {
        ActivityKind::ConflictResolved
    } else if lower.contains("conflict") {
        ActivityKind::Conflict
    } else if lower.contains("drift") || lower.contains("scan") {
        ActivityKind::Drift
    } else if lower.contains("delete") {
        ActivityKind::Delete
    } else if lower.contains("synced") {
        ActivityKind::Sync
    } else if lower.contains("pull") {
        ActivityKind::Pull
    } else if lower.contains("[bridge]") || lower.contains("restart") || lower.contains("watching") {
        ActivityKind::Bridge
    } else if lower.contains("fail") || lower.contains("error") || lower.contains("rejected") {
        ActivityKind::Error
    } else {
        ActivityKind::System
    }
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

    state: Mutex<(AutoSyncState, String)>,
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
    flush_task: Mutex<Option<JoinHandle<()>>>,
    event_task: Mutex<Option<JoinHandle<()>>>,
    drift_watcher_task: Mutex<Option<JoinHandle<()>>>,
    remote_scan_interval_secs: Arc<AtomicU64>,
    /// Tracker for fire-and-forget background tasks (lock acquire/release,
    /// bridge ping, edit-trail append). Aborted in `stop()` so they can't
    /// outlive the engine and hold stale `Arc<SftpClient>` clones.
    background_tasks: std::sync::Mutex<Vec<JoinHandle<()>>>,
    stop_tx: watch::Sender<bool>,
    disposed: AtomicBool,
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
            state: Mutex::new((AutoSyncState::Idle, String::new())),
            ignored_total: AtomicU64::new(0),
            ignored_by_rule: DashMap::new(),
            recently_written: DashMap::new(),
            watcher: Mutex::new(None),
            flush_task: Mutex::new(None),
            event_task: Mutex::new(None),
            drift_watcher_task: Mutex::new(None),
            remote_scan_interval_secs: Arc::new(AtomicU64::new(
                crate::sync::drift_watcher::DEFAULT_SCAN_INTERVAL_SECS,
            )),
            background_tasks: std::sync::Mutex::new(Vec::new()),
            stop_tx,
            disposed: AtomicBool::new(false),
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

        // Flush loop.
        let flush_engine = engine.clone();
        let mut flush_stop = engine.stop_tx.subscribe();
        let flush_task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(LOOP_TICK_MS));
            loop {
                tokio::select! {
                    _ = flush_stop.changed() => {
                        if *flush_stop.borrow() { break; }
                    }
                    _ = tick.tick() => {
                        flush_engine.flush_cycle().await;
                    }
                }
            }
        });
        *engine.flush_task.lock().await = Some(flush_task);

        engine.set_state(AutoSyncState::Watching, "0 folder(s)".into()).await;

        // DriftWatcher — closes the bidirectional sync loop. Polls the remote
        // every `remote_scan_interval_secs` seconds, pulls files that drifted
        // remotely, surfaces conflicts. Same Arc the public API holds, so it
        // sees the same `disposed` flag and shuts down on `stop()`.
        let dw_engine = engine.clone();
        let dw_interval = engine.remote_scan_interval_secs.clone();
        let dw_handle = crate::sync::drift_watcher::spawn(dw_engine, dw_interval);
        *engine.drift_watcher_task.lock().await = Some(dw_handle);

        Ok(engine)
    }

    /// Update the DriftWatcher tick interval live (Settings > Sync surfaces
    /// this). 0 = pause the watcher entirely; minimum effective tick is 5s
    /// to avoid SFTP roundtrip stampedes on small folders.
    pub fn set_remote_scan_interval(&self, secs: u64) {
        self.remote_scan_interval_secs.store(secs, Ordering::Relaxed);
        diagnostics::emit_with_fields(
            DiagStage::System,
            DiagLevel::Info,
            None,
            None,
            format!("remote scan interval = {secs}s"),
            serde_json::json!({ "interval_secs": secs }),
        );
    }

    pub fn remote_scan_interval(&self) -> u64 {
        self.remote_scan_interval_secs.load(Ordering::Relaxed)
    }

    pub async fn stop(&self) {
        if self.disposed.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = self.stop_tx.send(true);
        if let Some(h) = self.flush_task.lock().await.take() {
            let _ = h.await;
        }
        // event_task is aborted (vs awaited) because on_fs_event futures may
        // park on the dirty DashMap shard locks under heavy churn — we don't
        // want stop() to block on user-driven event flow. The DashMap ops are
        // atomic so an aborted future never leaves the map in a torn state.
        if let Some(h) = self.event_task.lock().await.take() {
            h.abort();
        }
        // Drift watcher pulls and remote scans don't hold critical state — we
        // abort rather than await; an in-flight pull may leave a partial
        // `.rift-tmp` next to the destination but `download_file_atomic`
        // cleans those up on the next attempt.
        if let Some(h) = self.drift_watcher_task.lock().await.take() {
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

    /// Snapshot of currently-watched remote roots — for `LockPresence` scoped poll.
    pub fn watched_remote_roots(&self) -> Vec<String> {
        self.folders.iter().map(|kv| kv.value().remote_root.clone()).collect()
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
    pub fn resource_name_for(&self, remote_root: &str) -> Option<String> {
        self.folders.get(remote_root).map(|fw| fw.resource_name.clone())
    }
    pub fn is_disposed(&self) -> bool { self.disposed.load(Ordering::SeqCst) }
    pub fn track_pull_handle(&self, h: JoinHandle<()>) { self.track_background(h); }

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
        let kind = match ev.kind {
            EventKind::Create(_) => ChangeKind::Created,
            EventKind::Modify(_) => ChangeKind::Modified,
            EventKind::Remove(_) => ChangeKind::Deleted,
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
            self.queue_path(path, kind);
        }
    }

    /// Fire a one-shot drift reconcile across every watched folder. Used by
    /// the rescan-signal path AND callable from the Diagnostics panel via the
    /// `diag_force_drift_scan` Tauri command. Background-spawned so the
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
            let scanner = crate::sync::DriftScanner::new(&sftp, Some(&snapshot));
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
            }
            let mut to_push = 0u32;
            let mut to_pull = 0u32;
            let mut conflicts = 0u32;
            let mut push_paths: Vec<PathBuf> = Vec::new();
            for e in &result.entries {
                match e.bucket {
                    crate::sync::DriftBucket::ToPush => {
                        to_push += 1;
                        push_paths.push(PathBuf::from(&e.local_path));
                    }
                    crate::sync::DriftBucket::ToPull => to_pull += 1,
                    crate::sync::DriftBucket::Conflict => conflicts += 1,
                    crate::sync::DriftBucket::Synced => {}
                }
            }
            // On cancel: do NOT auto-enqueue partial push set — the user just
            // asked to stop. Leave the rows for the modal to surface manually.
            let enqueued = if !result.cancelled && !push_paths.is_empty() {
                engine.enqueue_for_flush_batch(push_paths, false, false).await
            } else {
                0
            };
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
                    "conflicts": conflicts,
                    "enqueued_for_push": enqueued,
                    "missing_remote_folders": result.remote_folders_missing.len(),
                    "listing_error": result.last_batch_listing_error,
                    "cancelled": result.cancelled,
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

    /// Store a cancellation token in the shared scan-cancel slot. Used by
    /// drift_watcher::run_tick so the modal's Cancel button cancels its slow
    /// background SFTP listing. Replaces any prior token.
    pub(crate) fn register_scan_cancel(&self, ct: CancellationToken) {
        if let Ok(mut g) = self.current_scan_cancel.lock() {
            *g = Some(ct);
        }
    }

    /// Clear the scan-cancel slot if it still holds `ct` (no-op if a newer
    /// op replaced it). Match by `is_cancelled()` parity — same pattern as
    /// kick_drift_reconcile uses.
    pub(crate) fn clear_scan_cancel(&self, ct: &CancellationToken) {
        if let Ok(mut g) = self.current_scan_cancel.lock() {
            if let Some(stored) = g.as_ref() {
                if stored.is_cancelled() == ct.is_cancelled() {
                    *g = None;
                }
            }
        }
    }

    /// Force-pull NOW — uses cached ToPull entries from drift_watcher's last
    /// tick. NO scan. Dispatches `pull_one` for each cached ToPull entry and
    /// emits an immediate DriftScanResult so the modal jumps to "complete"
    /// without the 30s scan. Pull progress surfaces via RemotePullStart/Done
    /// into the activity feed.
    pub fn force_pull_now(self: &Arc<Self>) {
        let entries = {
            let g = match self.last_scan_entries.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            g.clone()
        };
        // Register a cancellation token so the modal's Cancel button can stop
        // the dispatch mid-flight. Replaces any prior token — kick_drift_reconcile
        // sharing the same slot means the user's "cancel" always hits the latest
        // long-running op, whichever it is.
        let ct = CancellationToken::new();
        let ct_for_task = ct.clone();
        if let Ok(mut g) = self.current_scan_cancel.lock() {
            *g = Some(ct);
        }
        let engine = self.clone();
        let h = tokio::spawn(async move {
            diagnostics::emit(
                DiagStage::DriftScanStart,
                DiagLevel::Info,
                "pull-now (from cache)",
            );
            let mut to_pull = 0u32;
            let mut to_push = 0u32;
            let mut conflicts = 0u32;
            let mut pull_entries: Vec<crate::sync::DriftEntry> = Vec::new();
            for e in entries {
                match e.bucket {
                    crate::sync::DriftBucket::ToPush => to_push += 1,
                    crate::sync::DriftBucket::ToPull => {
                        to_pull += 1;
                        pull_entries.push(e);
                    }
                    crate::sync::DriftBucket::Conflict => conflicts += 1,
                    crate::sync::DriftBucket::Synced => {}
                }
            }
            // Cap concurrent pulls so a slow uplink doesn't drown the SFTP
            // session with N parallel downloads. 4 was the sweet spot for
            // Trey's Tailscale link; tunable later.
            let sem = Arc::new(tokio::sync::Semaphore::new(4));
            let mut pull_dispatched = 0u32;
            let mut cancelled = false;
            for entry in pull_entries {
                if ct_for_task.is_cancelled() {
                    cancelled = true;
                    break;
                }
                let task_engine = engine.clone();
                let permit_sem = sem.clone();
                let h = tokio::spawn(async move {
                    let _permit = permit_sem.acquire_owned().await.ok();
                    crate::sync::drift_watcher::pull_one(&task_engine, entry).await;
                });
                engine.track_pull_handle(h);
                pull_dispatched += 1;
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
        // path in a cycle. Released on flush success (release inside process_entry)
        // or on Deleted (release here so renamed-old-path locks don't leak).
        if first_dirty || kind == ChangeKind::Deleted {
            if let Some(locks) = self.locks.clone() {
            if let Some(fw) = self.folders.get(&watch_key_for_lock).map(|v| v.value().clone()) {
                if let Some(remote) = map_local_to_remote(&path, &fw) {
                    let kind_at_queue = kind;
                    let h = tokio::spawn(async move {
                        if kind_at_queue == ChangeKind::Deleted {
                            locks.release(&remote).await;
                        } else {
                            locks.acquire(&remote).await;
                        }
                    });
                    self.track_background(h);
                }
            }
            }
        }
    }

    // ─── Flush loop ──────────────────────────────────────────────────────────

    async fn flush_cycle(&self) {
        if self.disposed.load(Ordering::SeqCst) {
            return;
        }
        let now = Utc::now();

        // Drain failed → dirty when backoff elapsed.
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
            self.log(&format!("auto-retry attempt {}/3: {}", e.attempts + 1, e.path.display()));
            self.dirty.insert(k, e);
        }

        // Pick ready entries.
        let mut ready: Vec<DirtyEntry> = Vec::new();
        for kv in self.dirty.iter() {
            if kv.value().next_flush <= now {
                ready.push(kv.value().clone());
            }
        }
        if ready.is_empty() {
            return;
        }
        // Group by watch.
        let mut by_watch: HashMap<String, Vec<DirtyEntry>> = HashMap::new();
        for e in ready {
            by_watch.entry(e.watch_key.clone()).or_default().push(e);
        }
        for (watch_key, entries) in by_watch {
            let Some(fw) = self.folders.get(&watch_key).map(|v| v.value().clone()) else {
                continue;
            };
            self.flush_batch(&fw, entries).await;
        }
    }

    async fn flush_batch(&self, fw: &FolderWatch, entries: Vec<DirtyEntry>) {
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
            return;
        }

        self.set_state(
            AutoSyncState::Syncing,
            format!("{} · {} file(s)", fw.resource_name, entries.len()),
        )
        .await;

        // Pop entries from dirty up front so re-dirty during upload creates a new entry.
        for e in &entries {
            self.dirty.remove(&e.path);
        }

        // Bounded concurrency.
        use futures::stream::{FuturesUnordered, StreamExt};
        let mut futs = FuturesUnordered::new();
        let mut iter = entries.into_iter();
        let mut active = 0usize;
        let mut ok = 0u32;
        let mut fail = 0u32;
        let mut trail_items: Vec<(String, String)> = Vec::new();

        loop {
            while active < UPLOAD_CONCURRENCY {
                let Some(entry) = iter.next() else { break };
                let fw = fw.clone();
                futs.push(async move { self.process_entry(&fw, entry).await });
                active += 1;
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
            let next = if fail > 0 { AutoSyncState::Error } else { AutoSyncState::Idle };
            let detail = if fail > 0 {
                format!("{}: {} ok, {} failed", fw.resource_name, ok, fail)
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
    }

    async fn process_entry(&self, fw: &FolderWatch, entry: DirtyEntry) -> EntryResult {
        let remote = match map_local_to_remote(&entry.path, fw) {
            Some(r) => r,
            None => {
                self.mark_failed(&entry);
                self.log_activity(&fw.resource_name, file_name(&entry.path),
                    "rejected: path outside watch root");
                return EntryResult::Fail;
            }
        };

        if entry.kind == ChangeKind::Deleted {
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
                self.log_activity(&fw.resource_name, file_name(&entry.path), "deleted");
                diagnostics::emit_for(
                    DiagStage::UploadDone,
                    DiagLevel::Info,
                    Some(&fw.resource_name),
                    Some(&remote),
                    "deleted",
                );
                let rel = rel_of(fw, &entry.path);
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
                return EntryResult::Fail;
            }
            if !entry.path.exists() {
                self.mark_failed(&entry);
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
            diagnostics::emit_with_fields(
                DiagStage::UploadStart,
                DiagLevel::Debug,
                Some(&fw.resource_name),
                Some(&remote),
                "upload start",
                serde_json::json!({ "size": local_size }),
            );
            let r = self.sftp.upload_file_atomic(&entry.path, &remote).await;
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
                self.log_activity(&fw.resource_name, file_name(&entry.path), "synced");
                diagnostics::emit_with_fields(
                    DiagStage::UploadDone,
                    DiagLevel::Info,
                    Some(&fw.resource_name),
                    Some(&remote),
                    "synced",
                    serde_json::json!({ "size": local_size, "elapsed_ms": elapsed_ms }),
                );
                let rel = rel_of(fw, &entry.path);
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
            // Permanent drop — flush_cycle's promote loop will skip it next tick.
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

    fn log_activity(&self, resource: &str, file: &str, action: &str) {
        let row = ActivityRow {
            at: Utc::now(),
            resource: resource.to_string(),
            file: file.to_string(),
            action: action.to_string(),
            kind: classify_action(action),
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

fn map_local_to_remote(local: &Path, fw: &FolderWatch) -> Option<String> {
    let rel = local.strip_prefix(&fw.local_root).ok()?;
    let rel_s = rel.to_string_lossy().replace('\\', "/");
    if rel_s == "." || rel_s.is_empty() {
        return Some(fw.remote_root.clone());
    }
    if rel_s.starts_with("../") || rel_s.starts_with('/') {
        return None;
    }
    Some(format!("{}/{}", fw.remote_root.trim_end_matches('/'), rel_s))
}

fn rel_of(fw: &FolderWatch, local: &Path) -> String {
    local
        .strip_prefix(&fw.local_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

fn file_name(p: &Path) -> &str {
    p.file_name().and_then(|s| s.to_str()).unwrap_or("?")
}

/// Read local size + mtime. Returns None on metadata error so callers can
/// skip baseline writes rather than poisoning the snapshot with `(0, now())`.
fn stat_local(p: &Path) -> Option<(i64, DateTime<Utc>)> {
    let m = std::fs::metadata(p).ok()?;
    let size = m.len() as i64;
    let mtime = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
        .unwrap_or_else(Utc::now);
    Some((size, mtime))
}

async fn wait_for_readable(path: &Path) -> bool {
    for _ in 0..4 {
        if tokio::fs::File::open(path).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

/// Bounded recursive count — caps at 5000 enumerations to bound latency.
/// Walked off the tokio runtime via `spawn_blocking` so the flush task isn't
/// stalled by a multi-thousand-file resource folder.
async fn safe_count_files(root: &Path) -> usize {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut count = 0usize;
        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                count += 1;
                if count >= 5000 {
                    break;
                }
            }
        }
        count
    })
    .await
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn merge_kind_delete_wins() {
        assert_eq!(merge_kind(ChangeKind::Created, ChangeKind::Deleted), ChangeKind::Deleted);
        assert_eq!(merge_kind(ChangeKind::Modified, ChangeKind::Deleted), ChangeKind::Deleted);
    }

    #[test]
    fn merge_kind_create_sticks() {
        assert_eq!(merge_kind(ChangeKind::Created, ChangeKind::Modified), ChangeKind::Created);
    }

    #[test]
    fn merge_kind_default_takes_b() {
        assert_eq!(merge_kind(ChangeKind::Modified, ChangeKind::Created), ChangeKind::Created);
    }

    #[test]
    fn map_local_to_remote_basic() {
        let fw = FolderWatch {
            local_root: PathBuf::from("/srv/local/qbx_core"),
            remote_root: "/opt/server/[qbx]/qbx_core".into(),
            resource_name: "qbx_core".into(),
        };
        let out = map_local_to_remote(Path::new("/srv/local/qbx_core/server/main.lua"), &fw);
        assert_eq!(out.as_deref(), Some("/opt/server/[qbx]/qbx_core/server/main.lua"));
    }

    #[test]
    fn map_local_to_remote_outside_returns_none() {
        let fw = FolderWatch {
            local_root: PathBuf::from("/srv/local/qbx_core"),
            remote_root: "/opt/server/[qbx]/qbx_core".into(),
            resource_name: "qbx_core".into(),
        };
        let out = map_local_to_remote(Path::new("/srv/other/main.lua"), &fw);
        assert!(out.is_none());
    }

    #[test]
    fn classify_action_buckets() {
        assert_eq!(classify_action("synced"), ActivityKind::Sync);
        assert_eq!(classify_action("deleted"), ActivityKind::Delete);
        assert_eq!(classify_action("BLOCKED — 30 deletes"), ActivityKind::Block);
        assert_eq!(classify_action("CONFLICT — remote changed"), ActivityKind::Conflict);
        assert_eq!(classify_action("conflict→accept-remote"), ActivityKind::ConflictResolved);
        assert_eq!(classify_action("[bridge] restart triggered"), ActivityKind::Bridge);
        assert_eq!(classify_action("sync failed: timeout"), ActivityKind::Error);
    }
}
