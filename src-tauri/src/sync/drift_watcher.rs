//! Remote-side helpers — `pull_one`, `delete_local_one`, `register_conflict`.
//!
//! v0.2.38 ripped the auto-poll loop (`spawn` / `run_tick`) and the per-tick
//! mass-delete guard out of this module. Sync is now manual-only: the user
//! clicks Push Now / Pull Now in the UI, which routes through
//! `AutoSyncEngine::force_push_now` / `force_pull_now`. Those callers
//! dispatch the helpers below for each `DriftEntry`.
//!
//! Safety guarantees preserved (still enforced in `pull_one` /
//! `delete_local_one`):
//! 1. **Never overwrite an unflushed local edit.** If a `ToPull` target is
//!    currently in the dirty queue, we sidestep — we download to
//!    `<file>.rift-conflict.<remote-user>-<ts>.<ext>` and emit a
//!    ConflictRecord.
//! 2. **Respect cross-dev locks.** If `LockPresence` knows another developer
//!    is mid-edit on a file, defer the pull/delete until they release.
//! 3. **Snapshot is the source of truth.** Every successful pull writes the
//!    `(local_size, local_mtime, remote_size, remote_mtime, sha1)` tuple
//!    back so the next scan won't re-flag the same file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;

use crate::diagnostics::{self, DiagLevel, DiagStage};
use crate::state::sync_snapshot::SHA1_MAX_BYTES;
use crate::state::SyncSnapshot;
use crate::sync::auto_sync::AutoSyncEngine;
use crate::sync::ConflictRecord;

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
    // #76: floor at the resource's local_root so a single-file resource doesn't
    // start removing parents up through the profile local_root and beyond.
    // Without the floor, `remove_dir` only fails on non-empty dirs / IO error
    // — but the resource root could legitimately be empty after the last file
    // is removed, and we don't want to nuke it.
    let floor: Option<PathBuf> = engine
        .folders_clone()
        .into_iter()
        .find(|fw| fw.resource_name == resource)
        .map(|fw| fw.local_root.clone());
    if let Some(parent) = local_path.parent() {
        let mut cur = parent.to_path_buf();
        loop {
            if let Some(ref f) = floor {
                if &cur == f || !cur.starts_with(f) {
                    break;
                }
            }
            if std::fs::remove_dir(&cur).is_err() {
                break;
            }
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

pub(crate) fn register_conflict(engine: &Arc<AutoSyncEngine>, entry: crate::sync::DriftEntry) {
    // ToConflict from the scanner — both sides changed since baseline.
    // Build a ConflictRecord and surface in the existing UI.
    let local_path = PathBuf::from(&entry.local_path);
    let snap = engine.snapshot().try_get(&entry.remote_path);
    // #124: re-stat local right before building the record so the conflict
    // dialog reflects on-disk state at decision time, not scan-time. A
    // user can save/edit the local file between scan and the conflict
    // modal showing up; using the scan-time mtime would mis-frame the
    // comparison ("local is older" when it's actually newer than remote).
    // Mirrors the pull_one re-stat pattern at L132.
    let (local_size, local_mtime) = match std::fs::metadata(&local_path) {
        Ok(meta) => {
            let lmtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
                .unwrap_or_else(Utc::now);
            (meta.len() as i64, lmtime)
        }
        Err(_) => (
            entry.local_size,
            entry.local_mtime.unwrap_or_else(Utc::now),
        ),
    };
    let record = ConflictRecord {
        local_path: entry.local_path.clone(),
        remote_path: entry.remote_path.clone(),
        resource_name: entry.resource_name.clone(),
        local_size,
        local_mtime_utc: local_mtime,
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
