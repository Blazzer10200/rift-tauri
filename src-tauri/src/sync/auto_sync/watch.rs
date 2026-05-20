// Notify lifecycle, FS-event ingestion, and dirty-queue admission for the
// AutoSync engine. Split out from `auto_sync.rs` 2026-05-13 — parent module
// owns the engine struct + flush pipeline + drift reconcile.
//
// Methods here only access engine state via `&self` / `self: &Arc<Self>` —
// submodule privacy lets us reach the engine's private fields without
// `pub(super)` shims on the struct.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use chrono::Utc;
use dashmap::mapref::entry::Entry;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Watcher};

use super::path::{map_local_to_remote};
use super::{
    merge_kind, AutoSyncEngine, AutoSyncState, ChangeKind, DirtyEntry, FolderSpec, FolderWatch,
    CEILING_MS, DEBOUNCE_MS,
};
use crate::diagnostics::{self, DiagLevel, DiagStage};
use crate::sync::ignore;

impl AutoSyncEngine {
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
        // Rest of the codebase carries forward-slash paths even on Windows
        // (`auto_sync/path.rs` normalizes both ways). Building the watched
        // root via `MAIN_SEPARATOR_STR` previously produced `C:\path\a\b`
        // on Win and `/path/a/b` on POSIX, splitting the equality story
        // for callers that pass already-normalized subpaths.
        let local_root = std::path::Path::new(&self.profile.local_root)
            .join(spec.remote_subpath.trim_start_matches('/'));
        if !local_root.exists() {
            let profile_root = std::path::Path::new(&self.profile.local_root);
            if !profile_root.exists() {
                self.log(&format!(
                    "watch failed (profile local_root missing): {} -> {}",
                    remote_root,
                    local_root.display()
                ));
                return Ok(false);
            }
            if let Err(e) = std::fs::create_dir_all(&local_root) {
                self.log(&format!(
                    "watch failed (mkdir {}): {e}",
                    local_root.display()
                ));
                return Ok(false);
            }
            self.log(&format!(
                "auto-created local folder for first-time bootstrap: {}",
                local_root.display()
            ));
        }
        // Refuse to attach to ignored paths (e.g. `[disabled]/`). Probe with
        // forward-slash so `ignore::should_ignore` (which normalizes the
        // input the same way) gets a stable representation on both OSes.
        let probe = format!("{}/", local_root.display().to_string().replace('\\', "/"));
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
        // handles new dirs; this catches the rest. Async + best-effort.
        // #35: track this spawn via track_background so engine.stop() aborts it
        // alongside other tasks. Previously untracked → kept issuing chmod
        // against a disconnected/reconnected session after stop().
        let sftp = self.sftp.clone();
        let root_for_heal = remote_root.clone();
        let h = tokio::spawn(async move {
            sftp.heal_owned_dirs(&root_for_heal).await;
        });
        self.track_background(h);

        let count = self.folders.len();
        let (cur_state, _) = *self.state.lock().await;
        if cur_state == AutoSyncState::Watching {
            self.set_state(AutoSyncState::Watching, format!("{count} folder(s)")).await;
        } else {
            self.fire_status().await;
        }
        Ok(true)
    }

    pub(super) async fn stop_watch(&self, remote_root: &str) {
        // #44: unwatch FIRST so notify stops emitting events for this root,
        // THEN remove from the folders map. Reverse order leaves a small
        // window where FS events arrive for an already-removed root and
        // queue_path silently drops them.
        let local_root = match self.folders.get(remote_root) {
            Some(e) => e.value().local_root.clone(),
            None => return,
        };
        if let Some(w) = self.watcher.lock().await.as_mut() {
            let _ = w.unwatch(&local_root);
        }
        let Some((_, fw)) = self.folders.remove(remote_root) else { return };
        self.local_file_counts.remove(remote_root);
        self.log(&format!("stopped watching {}", fw.resource_name));
        let count = self.folders.len();
        let (cur_state, _) = *self.state.lock().await;
        if cur_state == AutoSyncState::Watching {
            self.set_state(AutoSyncState::Watching, format!("{count} folder(s)")).await;
        } else {
            self.fire_status().await;
        }
    }

    pub(super) async fn on_fs_event(self: &Arc<Self>, ev: Event) {
        if self.disposed.load(Ordering::SeqCst) {
            return;
        }
        // Rescan signal: kernel/FSEvents/ReadDirectoryChangesW dropped events
        // and is asking us to do a full reconcile.
        if ev.need_rescan() {
            diagnostics::emit(
                DiagStage::RescanSignal,
                DiagLevel::Warn,
                "notify reported event drop — triggering drift reconcile",
            );
            self.kick_drift_reconcile();
            return;
        }
        // v0.2.52: explicit rename-event handling on Windows — `Modify(Name(From))`
        // for the old path + `Modify(Name(To))` for the new path. Recycle-Bin
        // delete surfaces as `Modify(Name(From))` w/ no matching `To`. Pre-v0.2.52
        // these bucketed as Modified → failed wait_for_readable → Error state.
        let kind = match ev.kind {
            EventKind::Create(_) => ChangeKind::Created,
            EventKind::Remove(_) => ChangeKind::Deleted,
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => ChangeKind::Deleted,
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => ChangeKind::Created,
            EventKind::Modify(ModifyKind::Name(_)) => ChangeKind::Modified,
            EventKind::Modify(_) => ChangeKind::Modified,
            _ => return,
        };
        for path in ev.paths {
            let path_str = path.to_string_lossy().to_string();
            diagnostics::emit_for(
                DiagStage::FsEvent,
                DiagLevel::Debug,
                None,
                Some(&path_str),
                format!("{kind:?}"),
            );
            // Belt-and-suspenders for the Created+Dir race: Windows
            // ReadDirectoryChangesW drops Create events for files nested in a
            // freshly-created dir before it fully registers. v0.2.48 added a
            // 500 ms delay + AtomicBool coalesce so rapid Create(Dir) events
            // collapse to one delayed reconcile.
            if kind == ChangeKind::Created && path.is_dir() {
                if self
                    .pending_dir_reconcile
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    let engine = self.clone();
                    let h = tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        if engine.disposed.load(Ordering::SeqCst) {
                            engine.pending_dir_reconcile.store(false, Ordering::Release);
                            return;
                        }
                        // #46: kick BEFORE clearing the flag. A new Create(Dir)
                        // arriving in the gap between clear and kick (old order)
                        // would pass compare_exchange → second 500ms reconcile
                        // + double SFTP scan. Kick-then-clear closes that window:
                        // events during the kick see flag=true and dedupe.
                        engine.kick_drift_reconcile();
                        engine.pending_dir_reconcile.store(false, Ordering::Release);
                    });
                    self.track_background(h);
                }
            }
            self.queue_path(path, kind);
        }
    }

    pub fn mark_recently_written(&self, path: &std::path::Path) {
        self.recently_written
            .insert(path.to_path_buf(), std::time::Instant::now());
    }

    /// Returns true if `path` was marked via `mark_recently_written`
    /// within the suppression window. Side-effect: lazily evicts the
    /// entry on a hit so the map doesn't grow unbounded.
    pub(super) fn is_recently_written(&self, path: &std::path::Path) -> bool {
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

    pub(super) fn queue_path(&self, path: PathBuf, kind: ChangeKind) {
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
        // path. Released on flush terminal result OR on Deleted (release here
        // so renamed-old-path locks don't leak). Acquire gated on
        // `path.is_file()` — dirs never need presence locks.
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
}
