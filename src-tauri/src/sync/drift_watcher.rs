//! Remote-side scan loop — closes the bidirectional sync loop.
//!
//! Rift's local watcher (`auto_sync.rs`) handles `local → remote`. SFTP/FTP
//! protocols expose no push-notify channel for the reverse direction
//! (verified by WinSCP docs + Syncthing scanning model), so we poll: every
//! `interval_secs` we run a `DriftScanner::scan()` against the watched
//! folders. Files in the `ToPull` bucket are downloaded; `Conflict` entries
//! get rendered into the existing ConflictRecord set surfaced in the UI.
//!
//! Safety guarantees (in priority order):
//! 1. **Never overwrite an unflushed local edit.** If a `ToPull` target is
//!    currently in the dirty queue, we sidestep — we download to
//!    `<file>.rift-conflict.<remote-user>-<ts>.<ext>` and emit a
//!    ConflictRecord. The user's bytes stay put. Inspired by Syncthing's
//!    `<file>.sync-conflict-<ts>-<who>.<ext>` model.
//! 2. **Don't fight the local pusher.** When the engine reports `is_pushing`
//!    we skip the tick entirely. We'd just be racing our own uploads.
//! 3. **Respect cross-dev locks.** If `LockPresence` knows another developer
//!    is mid-edit on a file, defer the pull until they release.
//! 4. **Snapshot is the source of truth.** Every successful pull writes the
//!    `(local_size, local_mtime, remote_size, remote_mtime, sha1)` tuple
//!    back so the next tick won't re-flag the same file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::task::JoinHandle;

use crate::diagnostics::{self, DiagLevel, DiagStage};
use crate::state::sync_snapshot::SHA1_MAX_BYTES;
use crate::state::SyncSnapshot;
use crate::sync::auto_sync::AutoSyncEngine;
use crate::sync::drift_scanner::{DriftBucket, DriftScanner, FolderTarget};
use crate::sync::ConflictRecord;

/// Default tick interval (seconds). Configurable per-session via
/// `AutoSyncEngine::set_remote_scan_interval`. Lowered 30 → 10 in v0.2.21 so
/// remote changes pulled by buddies feel live (≤10s lag) instead of "did this
/// even work?" (≤30s lag). 3x more SFTP listings, ~2s each on typical trees.
pub const DEFAULT_SCAN_INTERVAL_SECS: u64 = 10;

/// Sentinel — set the interval to this and the watcher pauses entirely
/// (Settings: "Off"). Loop still runs but every tick is a no-op.
pub const SCAN_INTERVAL_DISABLED: u64 = 0;

/// Spawns the remote-scan loop. Returns the task handle so the engine can
/// abort on stop. The interval is shared so Settings UI can change it live.
pub fn spawn(
    engine: Arc<AutoSyncEngine>,
    interval_secs: Arc<AtomicU64>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Initial delay — let the local watcher attach + first push settle
        // before we start polling. Avoids a thundering-herd scan on connect.
        tokio::time::sleep(Duration::from_secs(5)).await;

        loop {
            if engine.is_disposed() {
                break;
            }
            let secs = interval_secs.load(Ordering::Relaxed);
            if secs == SCAN_INTERVAL_DISABLED {
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            tokio::time::sleep(Duration::from_secs(secs.max(5))).await;
            if engine.is_disposed() {
                break;
            }
            if engine.is_pushing() {
                diagnostics::emit(
                    DiagStage::RemoteScanStart,
                    DiagLevel::Debug,
                    "skipped — local push in flight",
                );
                continue;
            }
            run_tick(&engine).await;
        }
    })
}

async fn run_tick(engine: &Arc<AutoSyncEngine>) {
    // Manual-mode short-circuit (v0.2.37). User opted out of auto-pulls.
    // Force-pull-now still works regardless — that's a separate codepath.
    if !engine.auto_flush_enabled() {
        return;
    }
    let folders = engine.folders_clone();
    if folders.is_empty() {
        return;
    }
    let started = std::time::Instant::now();
    diagnostics::emit_with_fields(
        DiagStage::RemoteScanStart,
        DiagLevel::Debug,
        None,
        None,
        "remote scan",
        serde_json::json!({ "folders": folders.len() }),
    );

    let snapshot = engine.snapshot();
    let sftp = engine.sftp();
    let scanner = DriftScanner::new(&sftp, Some(&snapshot));
    let targets: Vec<FolderTarget> = folders
        .iter()
        .map(|fw| FolderTarget {
            resource_name: fw.resource_name.clone(),
            local_root: fw.local_root.to_string_lossy().to_string(),
            remote_root: fw.remote_root.clone(),
        })
        .collect();
    // Register the tick's cancel token in the shared slot so the modal's
    // Cancel button can stop a slow background scan (30s+ SFTP listing on
    // Trey's Tailscale link was the trigger). Replaces any prior token.
    let ct = tokio_util::sync::CancellationToken::new();
    engine.register_scan_cancel(ct.clone());
    let result = scanner.scan_with_cancel(&targets, Some(&ct)).await;
    engine.clear_scan_cancel(&ct);
    let scan_ms = started.elapsed().as_millis() as u64;
    // Cache entries so SyncModal's Pull Now button can dispatch from this
    // without re-scanning. Drift_watcher ticks every 10s → cache stays fresh.
    engine.cache_scan_entries(result.entries.clone());

    let mut to_pull = 0usize;
    let mut to_delete = 0usize;
    let mut conflicts = 0usize;
    for entry in &result.entries {
        match entry.bucket {
            DriftBucket::ToPull => to_pull += 1,
            DriftBucket::ToDelete => to_delete += 1,
            DriftBucket::Conflict => conflicts += 1,
            _ => {}
        }
    }

    diagnostics::emit_with_fields(
        DiagStage::RemoteScanResult,
        DiagLevel::Info,
        None,
        None,
        format!("scan: {to_pull} to-pull, {to_delete} to-delete, {conflicts} conflict, {} entries total", result.entries.len()),
        serde_json::json!({
            "elapsed_ms": scan_ms,
            "to_pull": to_pull,
            "to_delete": to_delete,
            "conflicts": conflicts,
            "entries": result.entries.len(),
            "listing_error": result.last_batch_listing_error,
        }),
    );

    // ── Mass local-delete guard (v0.2.36) ─────────────────────────────────
    // Mirror of the push-side circuit breaker. When auto-sync sees a teammate
    // (or anyone) deleted a large batch of files remotely, the tombstone path
    // would silently nuke the same files on this machine. Block per-resource
    // batches that cross the scaled threshold and surface ONE prominent row
    // per resource. User has to take explicit action (toggle off, restore
    // remote, or — once manual mode lands — approve via review UI).
    use std::collections::HashMap;
    let mut deletes_by_resource: HashMap<String, Vec<crate::sync::DriftEntry>> = HashMap::new();
    let mut other: Vec<crate::sync::DriftEntry> = Vec::new();
    for entry in result.entries {
        if matches!(entry.bucket, DriftBucket::ToDelete) {
            deletes_by_resource
                .entry(entry.resource_name.clone())
                .or_default()
                .push(entry);
        } else {
            other.push(entry);
        }
    }
    let folders = engine.folders_clone();
    let mut approved_deletes: Vec<crate::sync::DriftEntry> = Vec::new();
    for (resource, entries) in deletes_by_resource {
        let fw = folders.iter().find(|f| f.resource_name == resource);
        let (threshold, total) = match fw {
            Some(f) => engine.scaled_delete_threshold(&f.local_root).await,
            None => (5, 0),
        };
        let count = entries.len();
        if count >= threshold {
            let reason = format!(
                "{count} local-deletes in one batch (≥ scaled threshold {threshold} of {total} files)"
            );
            diagnostics::emit_with_fields(
                DiagStage::RemoteScanResult,
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
            // Drop the entries — next tick will re-evaluate. If remote still
            // missing those files, the guard re-fires. User has to act.
        } else {
            approved_deletes.extend(entries);
        }
    }

    // Dispatch pulls + (approved) deletes + conflicts.
    let final_entries: Vec<_> = other.into_iter().chain(approved_deletes).collect();
    for entry in final_entries {
        if engine.is_disposed() {
            break;
        }
        match entry.bucket {
            DriftBucket::ToPull => {
                let task_engine = engine.clone();
                let h = tokio::spawn(async move {
                    pull_one(&task_engine, entry).await;
                });
                engine.track_pull_handle(h);
            }
            DriftBucket::ToDelete => {
                let task_engine = engine.clone();
                let h = tokio::spawn(async move {
                    delete_local_one(&task_engine, entry).await;
                });
                engine.track_pull_handle(h);
            }
            DriftBucket::Conflict => {
                register_conflict(engine, entry);
            }
            _ => {}
        }
    }
}

/// Pull a single ToPull entry. Three paths:
///   * Path is dirty (unflushed local edit) → conflict-rename.
///   * Path is foreign-locked → skip (next tick retries; lock release will
///     unblock).
///   * Otherwise → atomic download + snapshot baseline write.
pub(crate) async fn pull_one(engine: &Arc<AutoSyncEngine>, entry: crate::sync::DriftEntry) {
    let local_path = PathBuf::from(&entry.local_path);
    let remote_path = entry.remote_path.clone();
    let resource = entry.resource_name.clone();

    // Cross-dev lock → defer.
    if let Some(locks) = engine.locks() {
        if let Some(foreign) = locks.find_lock_by_other(&remote_path) {
            diagnostics::emit_with_fields(
                DiagStage::RemotePullFail,
                DiagLevel::Warn,
                Some(&resource),
                Some(&remote_path),
                format!("deferred — locked by {}@{}", foreign.user, foreign.host),
                serde_json::json!({
                    "reason": "foreign_lock",
                    "user": foreign.user,
                    "host": foreign.host,
                }),
            );
            return;
        }
    }

    // Local edit unflushed → conflict-rename rather than clobber.
    let dirty = engine.is_path_dirty(&local_path);
    let target_local = if dirty {
        let renamed = derive_conflict_path(&local_path);
        diagnostics::emit_with_fields(
            DiagStage::RemotePullStart,
            DiagLevel::Warn,
            Some(&resource),
            Some(&remote_path),
            "conflict-rename — local has unflushed edit",
            serde_json::json!({
                "conflict_path": renamed.to_string_lossy().to_string(),
                "reason": "local_dirty",
            }),
        );
        renamed
    } else {
        diagnostics::emit_for(
            DiagStage::RemotePullStart,
            DiagLevel::Info,
            Some(&resource),
            Some(&remote_path),
            "pull start",
        );
        local_path.clone()
    };

    // Make sure the destination dir exists locally — the remote may have a
    // file under a directory we've never created on this machine.
    if let Some(parent) = target_local.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let started = std::time::Instant::now();
    // Mark BEFORE the write so any fs-event the watcher catches mid-write
    // (Windows fires events as soon as the .rift-tmp rename starts) is
    // suppressed. The window is re-stamped post-success below.
    engine.mark_recently_written(&target_local);
    let r = engine
        .sftp()
        .download_file_atomic(&remote_path, &target_local)
        .await;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    if r.success {
        // Re-stamp so the 5s window starts from completion, not start.
        engine.mark_recently_written(&target_local);
    }
    if !r.success {
        diagnostics::emit_with_fields(
            DiagStage::RemotePullFail,
            DiagLevel::Error,
            Some(&resource),
            Some(&remote_path),
            format!("pull failed: {}", r.error),
            serde_json::json!({
                "elapsed_ms": elapsed_ms,
                "error": r.error,
                "renamed": dirty,
            }),
        );
        return;
    }

    // Update snapshot + remote-state cache so the next tick doesn't re-flag.
    // Skip baseline writes if we conflict-renamed — the original remote_path
    // still doesn't match the local file at remote_path (which we didn't
    // touch); next tick will re-pick it up as ToPull → ConflictRecord path.
    if !dirty {
        let info = engine.sftp().remote_stat(&remote_path).await;
        if info.exists && !info.is_directory {
            engine.cache().set(&remote_path, info.size, info.last_modified);
            if let Ok(meta) = std::fs::metadata(&target_local) {
                let lsize = meta.len() as i64;
                let lmtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
                    .unwrap_or_else(Utc::now);
                let sha = if lsize <= SHA1_MAX_BYTES {
                    SyncSnapshot::compute_sha1(&target_local)
                } else {
                    None
                };
                engine.snapshot().set(
                    &remote_path,
                    lsize,
                    lmtime,
                    info.size,
                    info.last_modified,
                    sha,
                );
            }
        }
    }

    diagnostics::emit_with_fields(
        DiagStage::RemotePullDone,
        DiagLevel::Info,
        Some(&resource),
        Some(&remote_path),
        if dirty { "pulled to conflict copy" } else { "pulled" },
        serde_json::json!({
            "elapsed_ms": elapsed_ms,
            "local_path": target_local.to_string_lossy().to_string(),
            "renamed": dirty,
        }),
    );

    // Activity row for the user-facing feed.
    let pulled_size = std::fs::metadata(&target_local).ok().map(|m| m.len() as i64);
    let row = crate::sync::ActivityRow {
        at: Utc::now(),
        resource: resource.clone(),
        file: file_name_or(&target_local),
        action: if dirty { "pulled (conflict copy)".into() } else { "pulled".into() },
        kind: if dirty { crate::sync::ActivityKind::Conflict } else { crate::sync::ActivityKind::Pull },
        rel_path: Some(entry.rel_path.clone()),
        local_path: Some(target_local.to_string_lossy().to_string()),
        size_bytes: pulled_size,
        latency_ms: Some(elapsed_ms),
        actor: Some(crate::transport::env::current_user()),
        ..Default::default()
    };
    use tauri::Emitter;
    let _ = engine.app().emit("autosync://activity", &row);

    // If we conflict-renamed, register a ConflictRecord so the user sees it
    // in the Conflicts tab and can resolve it via the existing UI.
    if dirty {
        if let Ok(meta) = std::fs::metadata(&local_path) {
            let lsize = meta.len() as i64;
            let lmtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
                .unwrap_or_else(Utc::now);
            let info = engine.sftp().remote_stat(&remote_path).await;
            let snap = engine.snapshot().try_get(&remote_path);
            let record = ConflictRecord {
                local_path: local_path.to_string_lossy().to_string(),
                remote_path: remote_path.clone(),
                resource_name: resource,
                local_size: lsize,
                local_mtime_utc: lmtime,
                remote_size: info.size,
                remote_mtime_utc: info.last_modified,
                last_known_size: snap.as_ref().map(|s| s.remote_size).unwrap_or(0),
                last_known_mtime_utc: snap
                    .as_ref()
                    .map(|s| s.remote_mtime_utc)
                    .unwrap_or_else(Utc::now),
            };
            engine.record_remote_conflict(&local_path, record);
        }
    }
}

/// Propagate a remote-side delete to local. Triggered when the drift scanner
/// classifies an entry as `ToDelete` (local has it, remote doesn't, baseline
/// proves it WAS synced). Three guards:
///   * Foreign lock → defer (next tick retries after release).
///   * Dirty local (unflushed edit) → skip; respect user's in-flight work.
///     If they save+push, the remote delete reverses; if they accept it, the
///     next scan after the buffer flushes will re-fire and clean up.
///   * Otherwise → fs::remove_file + snapshot.forget + cache.forget + best-
///     effort empty-parent-dir cleanup.
pub(crate) async fn delete_local_one(engine: &Arc<AutoSyncEngine>, entry: crate::sync::DriftEntry) {
    let local_path = PathBuf::from(&entry.local_path);
    let remote_path = entry.remote_path.clone();
    let resource = entry.resource_name.clone();

    if let Some(locks) = engine.locks() {
        if let Some(foreign) = locks.find_lock_by_other(&remote_path) {
            diagnostics::emit_with_fields(
                DiagStage::RemotePullFail,
                DiagLevel::Warn,
                Some(&resource),
                Some(&remote_path),
                format!("delete-local deferred — locked by {}@{}", foreign.user, foreign.host),
                serde_json::json!({
                    "reason": "foreign_lock",
                    "user": foreign.user,
                    "host": foreign.host,
                }),
            );
            return;
        }
    }

    if engine.is_path_dirty(&local_path) {
        diagnostics::emit_with_fields(
            DiagStage::RemotePullFail,
            DiagLevel::Warn,
            Some(&resource),
            Some(&remote_path),
            "delete-local skipped — local has unflushed edit",
            serde_json::json!({ "reason": "local_dirty" }),
        );
        return;
    }

    engine.mark_recently_written(&local_path);
    // Stat before delete so the activity row can carry the size that vanished.
    let deleted_size = std::fs::metadata(&local_path).ok().map(|m| m.len() as i64);
    let started = std::time::Instant::now();
    let r = std::fs::remove_file(&local_path);
    let elapsed_ms = started.elapsed().as_millis() as u64;
    if let Err(e) = r {
        if e.kind() != std::io::ErrorKind::NotFound {
            diagnostics::emit_with_fields(
                DiagStage::RemotePullFail,
                DiagLevel::Error,
                Some(&resource),
                Some(&remote_path),
                format!("delete-local failed: {e}"),
                serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "error": e.to_string(),
                }),
            );
            return;
        }
    }

    engine.snapshot().forget(&remote_path);
    engine.cache().forget(&remote_path);

    // Best-effort empty-dir cleanup walks up until a non-empty dir or fs error
    // halts it. remove_dir only succeeds on empties → safe.
    if let Some(parent) = local_path.parent() {
        let mut cur = parent.to_path_buf();
        while std::fs::remove_dir(&cur).is_ok() {
            match cur.parent() {
                Some(p) => cur = p.to_path_buf(),
                None => break,
            }
        }
    }

    diagnostics::emit_with_fields(
        DiagStage::RemotePullDone,
        DiagLevel::Info,
        Some(&resource),
        Some(&remote_path),
        "deleted local (remote removed)",
        serde_json::json!({
            "elapsed_ms": elapsed_ms,
            "local_path": local_path.to_string_lossy().to_string(),
        }),
    );

    let row = crate::sync::ActivityRow {
        at: Utc::now(),
        resource,
        file: file_name_or(&local_path),
        action: "removed locally".into(),
        kind: crate::sync::ActivityKind::Delete,
        rel_path: Some(entry.rel_path.clone()),
        local_path: Some(local_path.to_string_lossy().to_string()),
        size_bytes: deleted_size,
        latency_ms: Some(elapsed_ms),
        actor: Some(crate::transport::env::current_user()),
        ..Default::default()
    };
    use tauri::Emitter;
    let _ = engine.app().emit("autosync://activity", &row);
}

fn register_conflict(engine: &Arc<AutoSyncEngine>, entry: crate::sync::DriftEntry) {
    // ToConflict from the scanner — both sides changed since baseline.
    // Build a ConflictRecord and surface in the existing UI.
    let local_path = PathBuf::from(&entry.local_path);
    let snap = engine.snapshot().try_get(&entry.remote_path);
    let record = ConflictRecord {
        local_path: entry.local_path.clone(),
        remote_path: entry.remote_path.clone(),
        resource_name: entry.resource_name.clone(),
        local_size: entry.local_size,
        local_mtime_utc: entry.local_mtime.unwrap_or_else(Utc::now),
        remote_size: entry.remote_size,
        remote_mtime_utc: entry.remote_mtime.unwrap_or_else(Utc::now),
        last_known_size: snap.as_ref().map(|s| s.remote_size).unwrap_or(0),
        last_known_mtime_utc: snap
            .as_ref()
            .map(|s| s.remote_mtime_utc)
            .unwrap_or_else(Utc::now),
    };
    engine.record_remote_conflict(&local_path, record);
}

/// Build a "<base>.rift-conflict.<user>-<YYYYMMDDHHMMSS>.<ext>" path next to
/// the original. Pattern is matched by `ignore.rs` (`rift-conflict-marker`)
/// so we never re-upload the conflict file as its own thing.
fn derive_conflict_path(original: &Path) -> PathBuf {
    let parent = original.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    let ext = original
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    let user = crate::transport::env::current_user();
    let host = crate::transport::env::hostname().unwrap_or_else(|| "unknown".into());
    let ts = Utc::now().format("%Y%m%dT%H%M%S");
    let name = format!("{stem}.rift-conflict.{user}@{host}-{ts}{ext}");
    parent.join(name)
}

fn file_name_or(p: &Path) -> String {
    p.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string()
}
