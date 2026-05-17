// Batch flush pipeline + per-entry upload/delete processing for the AutoSync
// engine. Split out from `auto_sync.rs` 2026-05-13. EntryResult lives here
// since it's the return contract of the flush pipeline.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use super::path::{file_name, map_local_to_remote, rel_of, safe_count_files, stat_local, wait_for_readable};
use super::{
    ActivityKind, ActivityRow, AutoSyncEngine, AutoSyncState, ChangeKind, ConflictRecord,
    DirtyEntry, FolderCountCache, FolderWatch, LOCK_HOLD_RETRY_SEC, MASS_DELETE_THRESHOLD,
    RETRY_BACKOFFS_SECS, UPLOAD_CONCURRENCY,
};

/// File-count cache TTL. After this many seconds, the next flush refreshes via
/// `safe_count_files` even if no add/remove deltas have been applied. Covers
/// drift from out-of-band changes (manual `rm -rf`, IDE refactor that bypasses
/// notify, etc.) without paying the walkdir cost on every batch.
const COUNT_CACHE_TTL_SECS: i64 = 300;
use crate::diagnostics::{self, DiagLevel, DiagStage};
use crate::state::sync_snapshot::SHA1_MAX_BYTES;
use crate::state::SyncSnapshot;

pub(super) enum EntryResult {
    Ok(Option<String>, String), // (rel_path, action)
    Fail,
    Requeued,
}

impl AutoSyncEngine {
    pub(super) async fn flush_batch(
        &self,
        fw: &FolderWatch,
        entries: Vec<DirtyEntry>,
        cancel: Option<CancellationToken>,
    ) -> u32 {
        // ── Mass-delete circuit breaker ───────────────────────────────────
        let delete_count = entries.iter().filter(|e| e.kind == ChangeKind::Deleted).count();
        let created_count = entries.iter().filter(|e| e.kind == ChangeKind::Created).count();
        let local_root_gone = !fw.local_root.exists();
        let local_file_count = self.cached_local_file_count(fw).await;
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
        // fallback. Serializing one mkdir per unique parent here avoids
        // worker races on a fresh tree.
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
                    // Don't bail — per-file strict mkdir is the safety net.
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
                if let Some(ct) = &cancel {
                    if ct.is_cancelled() {
                        break;
                    }
                }
                let Some(entry) = iter.next() else { break };
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
            // row with no clean batch between.
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
        // least one upload succeeded.
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

        // Apply best-effort delta to the cached file count. Optimistic — uses
        // input entry kinds (not per-entry success) since the TTL refresh
        // (5 min) corrects accumulated drift. Threshold clamp 5..25 forgives
        // small inaccuracy.
        if created_count > 0 || delete_count > 0 {
            self.apply_count_delta(&fw.remote_root, created_count as i64, delete_count as i64);
        }

        dispatched
    }

    /// Returns the cached local file count, refreshing via `safe_count_files`
    /// on cold cache or TTL expiry. v0.4.2 audit S1 — was inline every batch.
    async fn cached_local_file_count(&self, fw: &FolderWatch) -> usize {
        let now = Utc::now().timestamp();
        if let Some(cache) = self.local_file_counts.get(&fw.remote_root) {
            let age = now - cache.last_refresh_secs.load(Ordering::Relaxed);
            if age >= 0 && age < COUNT_CACHE_TTL_SECS {
                return cache.count.load(Ordering::Relaxed) as usize;
            }
        }
        let count = safe_count_files(&fw.local_root).await;
        let entry = Arc::new(FolderCountCache {
            count: AtomicU64::new(count as u64),
            last_refresh_secs: AtomicI64::new(now),
        });
        self.local_file_counts.insert(fw.remote_root.clone(), entry);
        count
    }

    fn apply_count_delta(&self, remote_root: &str, created: i64, deleted: i64) {
        if let Some(cache) = self.local_file_counts.get(remote_root) {
            let cur = cache.count.load(Ordering::Relaxed) as i64;
            let new = (cur + created - deleted).max(0) as u64;
            cache.count.store(new, Ordering::Relaxed);
        }
    }

    async fn process_entry(
        &self,
        fw: &FolderWatch,
        entry: DirtyEntry,
        cancel: Option<CancellationToken>,
    ) -> EntryResult {
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
        // Release the presence lock on every terminal result. v0.2.50:
        // inline-await w/ 5 s timeout (previous spawn could be aborted by
        // engine `stop()` before delete fired, leaving orphan locks). Release
        // is idempotent so inline + success-path release is harmless.
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
            // Wait briefly for readability (atomic-save mid-rename window).
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

            // Foreign-lock hold — requeue with 30s delay (NOT failure — coordination).
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
                            // Remote moved by stat but if sizes match AND both
                            // SHAs match baseline, only mtimes changed (npm
                            // rebuild touching artifacts, our SETSTAT calls
                            // bumping ctime/mtime). Cost: one SSH-exec SHA
                            // only fires when sizes match — common path
                            // (real change) is unaffected.
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
            self.log_activity(&fw.resource_name, file_name(&entry.path), "uploading…");
            diagnostics::emit_with_fields(
                DiagStage::UploadStart,
                DiagLevel::Debug,
                Some(&fw.resource_name),
                Some(&remote),
                "upload start",
                serde_json::json!({ "size": local_size }),
            );
            // Race upload against cancel. On Stop: russh drops WRITE packets,
            // tmp file is orphaned (rename never fired so target untouched),
            // entry requeues.
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

    pub(super) fn mark_failed(&self, entry: &DirtyEntry) {
        let mut e = entry.clone();
        e.attempts += 1;
        if e.attempts as usize > RETRY_BACKOFFS_SECS.len() {
            // Permanent drop — `flush_all_now`'s promote loop will skip it.
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
}
