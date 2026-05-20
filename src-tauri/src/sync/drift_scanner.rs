// 3-way drift scan against the SyncSnapshot baseline. Port of
// `Services/Sync/DriftScanner.cs`. Phase 1b first pass + Phase 1f upgrades.
//
// **Phase 1f additions:**
// - Per-folder SHA1 budget (25 hashes / folder, was-deferred from 1b).
// - Stat-only jitter elimination: when local OR remote "changed" by stat but
//   size still matches the snapshot column, hash to confirm. If hash matches
//   the snapshot's recorded SHA1, skip the entry as Synced and refresh the
//   baseline's stat columns so AutoSync's pre-flight doesn't re-fire CONFLICT.
// - False-conflict collapse: when both sides changed but converged to the same
//   content (parallel edits ending in the same fix), one local + one remote
//   hash collapses to Synced and seeds the snapshot.
// - First-scan opportunistic equality: when no baseline AND sizes match AND
//   content hashes match, seed snapshot as Synced (no first-scan conflict).
// - Full ignore-rule parity via `crate::sync::ignore::should_ignore` (was
//   a basic stub in 1b).
//
// Critical data-safety guard preserved: empty remote listing + non-empty local
// → CONFIRM folder still exists before deciding; never auto-surface
// local files as `push (new)` against a transient SFTP failure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

use crate::sftp::{RemoteEntry, SftpClient};
use crate::state::sync_snapshot::{MTIME_TOLERANCE_SECS, SHA1_MAX_BYTES};
use crate::state::SyncSnapshot;
use crate::sync::ignore;

const MAX_DEPTH: usize = 12;
const REMOTE_HASH_BUDGET_PER_FOLDER: i32 = 25;

/// Lifetime cap for the `.rift-rebuild` tooling sentinel. Stale leftovers (older
/// than this) are ignored so a crashed build script can't lock sync forever.
const REBUILD_SENTINEL_MAX_AGE_SECS: u64 = 300;

/// True when the path exists AND mtime is within the sentinel's max-age window.
/// Used by both the drift scanner (skip folder) and the watch queue (skip event).
pub fn is_rebuild_sentinel_fresh(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    let Ok(modified) = meta.modified() else { return true };
    match modified.elapsed() {
        Ok(age) => age.as_secs() <= REBUILD_SENTINEL_MAX_AGE_SECS,
        Err(_) => true,
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftBucket {
    Synced,
    ToPush,
    ToPull,
    /// Delete from LOCAL — remote vanished while baseline says it existed.
    ToDelete,
    /// v0.2.53 Mirror mode: delete from REMOTE — local vanished (user deleted
    /// or moved a file/folder) while baseline + remote still have it. ONLY
    /// produced when scanner is in mirror mode; Normal mode keeps treating
    /// this as `ToPull` (the safer non-destructive choice).
    ToDeleteRemote,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEntry {
    pub resource_name: String,
    pub rel_path: String,
    pub local_path: String,
    pub remote_path: String,
    pub bucket: DriftBucket,
    pub local_exists: bool,
    pub remote_exists: bool,
    pub local_size: i64,
    pub remote_size: i64,
    pub local_mtime: Option<DateTime<Utc>>,
    pub remote_mtime: Option<DateTime<Utc>>,
    pub has_snapshot: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTarget {
    pub resource_name: String,
    pub local_root: String,
    pub remote_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortedShrunkFolder {
    pub resource_name: String,
    pub remote_root: String,
    pub baseline_count: u32,
    pub listing_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub entries: Vec<DriftEntry>,
    pub last_batch_listing_error: Option<String>,
    pub remote_folders_missing: Vec<String>,
    #[serde(default)]
    pub aborted_shrunk: Vec<AbortedShrunkFolder>,
    #[serde(default)]
    pub cancelled: bool,
}

pub struct DriftScanner<'a> {
    sftp: &'a SftpClient,
    snapshot: Option<&'a SyncSnapshot>,
    /// v0.2.53: Mirror mode toggle. When true, the `l.is_none() && r.is_some()
    /// && snap.is_some()` case buckets as `ToDeleteRemote` (propagate the
    /// local delete to remote) instead of `ToPull` (restore from remote).
    /// Default is false — destructive direction is opt-in only.
    mirror: bool,
}

impl<'a> DriftScanner<'a> {
    pub fn new(sftp: &'a SftpClient, snapshot: Option<&'a SyncSnapshot>) -> Self {
        Self { sftp, snapshot, mirror: false }
    }

    pub fn with_mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    pub async fn scan(&self, folders: &[FolderTarget]) -> ScanResult {
        self.scan_with_cancel(folders, None).await
    }

    /// Cancellable scan. When `cancel` fires between folders, returns the
    /// partial result with `cancelled: true` so callers can decide whether
    /// to act on whatever was discovered before the abort.
    pub async fn scan_with_cancel(
        &self,
        folders: &[FolderTarget],
        cancel: Option<&CancellationToken>,
    ) -> ScanResult {
        let mut entries = Vec::new();
        let mut last_batch_error: Option<String> = None;
        let mut remote_folders_missing = Vec::new();
        let mut aborted_shrunk: Vec<AbortedShrunkFolder> = Vec::new();
        let mut cancelled = false;

        let roots: Vec<String> = folders.iter().map(|f| f.remote_root.clone()).collect();
        // The SFTP batch listing is the slow part of a scan on high-latency
        // links (30-60s on Trey's Tailscale). Race it against the cancel
        // token so the modal's Cancel button takes effect immediately
        // instead of waiting for the listing to complete naturally.
        let listings = match cancel {
            Some(ct) => tokio::select! {
                res = self.sftp.list_recursive_batch(&roots, MAX_DEPTH, None, 4) => match res {
                    Ok(m) => m,
                    Err(e) => {
                        last_batch_error = Some(e);
                        HashMap::new()
                    }
                },
                _ = ct.cancelled() => {
                    return ScanResult {
                        entries,
                        last_batch_listing_error: None,
                        remote_folders_missing,
                        aborted_shrunk,
                        cancelled: true,
                    };
                }
            },
            None => match self
                .sftp
                .list_recursive_batch(&roots, MAX_DEPTH, None, 4)
                .await
            {
                Ok(m) => m,
                Err(e) => {
                    last_batch_error = Some(e);
                    HashMap::new()
                }
            },
        };

        let total = folders.len();
        for (idx, f) in folders.iter().enumerate() {
            if let Some(ct) = cancel {
                if ct.is_cancelled() {
                    cancelled = true;
                    break;
                }
            }
            crate::diagnostics::emit_with_fields(
                crate::diagnostics::DiagStage::DriftScanProgress,
                crate::diagnostics::DiagLevel::Debug,
                Some(&f.resource_name),
                None,
                format!("scanning folder {}/{}", idx + 1, total),
                serde_json::json!({
                    "current": idx + 1,
                    "total": total,
                    "resource": f.resource_name,
                }),
            );
            let remote_hits = listings
                .get(&f.remote_root)
                .cloned()
                .unwrap_or_default();
            let folder_result = self.scan_folder(f, &remote_hits, cancel).await;
            match folder_result {
                FolderScan::Drift(mut v) => entries.append(&mut v),
                FolderScan::RemoteMissing => {
                    remote_folders_missing.push(f.remote_root.clone());
                }
                FolderScan::SuspiciousEmptyAborted { baseline_count, listing_count } => {
                    aborted_shrunk.push(AbortedShrunkFolder {
                        resource_name: f.resource_name.clone(),
                        remote_root: f.remote_root.clone(),
                        baseline_count,
                        listing_count,
                    });
                }
            }
        }

        ScanResult {
            entries,
            last_batch_listing_error: last_batch_error,
            remote_folders_missing,
            aborted_shrunk,
            cancelled,
        }
    }

    async fn scan_folder(
        &self,
        f: &FolderTarget,
        remote_hits: &[RemoteEntry],
        cancel: Option<&CancellationToken>,
    ) -> FolderScan {
        let mut hash_budget: i32 = REMOTE_HASH_BUDGET_PER_FOLDER;
        // Local recursive walk. Full ignore-rule parity now via `ignore::should_ignore`.
        // Walk runs on a blocking thread so a multi-thousand-file resource
        // folder doesn't stall the tokio runtime mid-scan.
        let local_root = Path::new(&f.local_root);

        // Rebuild sentinel: tooling drops `.rift-rebuild` at the watched-folder
        // root while a build is in flight (Vite/Webpack/esbuild content-hashed
        // filenames produce unlink-old + write-new event pairs that otherwise
        // misclassify as remote-delete + local-new for two events). Skip the
        // whole folder for this scan when the sentinel is fresh.
        if is_rebuild_sentinel_fresh(&local_root.join(".rift-rebuild")) {
            crate::diagnostics::emit_for(
                crate::diagnostics::DiagStage::DriftScanProgress,
                crate::diagnostics::DiagLevel::Debug,
                Some(&f.resource_name),
                None,
                ".rift-rebuild present — folder skipped for this scan",
            );
            return FolderScan::Drift(Vec::new());
        }
        // #74: a panic inside walk_local previously fell back to .unwrap_or_default(),
        // producing an empty local_map that silently bypassed the data-safety guard
        // below (`!local_map.is_empty()`) and let the diff downstream mark every
        // remote file as ToPull — a mass-overwrite of local changes. Treat a join
        // error as SuspiciousEmptyAborted so the folder is reported to the UI and
        // skipped, never silently mass-pulled.
        let local_map: HashMap<String, LocalStat> = {
            let local_root_owned = local_root.to_path_buf();
            match tokio::task::spawn_blocking(move || {
                let mut m = HashMap::new();
                if local_root_owned.exists() {
                    walk_local(&local_root_owned, &local_root_owned, &mut m);
                }
                m
            })
            .await
            {
                Ok(m) => m,
                Err(e) => {
                    crate::diagnostics::emit_with_fields(
                        crate::diagnostics::DiagStage::DriftScanProgress,
                        crate::diagnostics::DiagLevel::Error,
                        Some(&f.resource_name),
                        None,
                        format!("local walk task panicked: {e}"),
                        serde_json::json!({ "remote_root": f.remote_root }),
                    );
                    let baseline_count = self
                        .snapshot
                        .map(|s| s.count_under(&f.remote_root) as u32)
                        .unwrap_or(0);
                    return FolderScan::SuspiciousEmptyAborted {
                        baseline_count,
                        listing_count: 0,
                    };
                }
            }
        };

        // Critical data-safety guard: empty remote + non-empty local. Confirm
        // remote folder existence before deciding (mirrors WPF v13.55.1 fix).
        if remote_hits.is_empty() && !local_map.is_empty() {
            let exists = self.sftp.remote_exists(&f.remote_root).await;
            if !exists {
                return FolderScan::RemoteMissing;
            } else {
                // Listing failed silently. Bail rather than false-push every local file.
                let baseline_count = self
                    .snapshot
                    .map(|s| s.count_under(&f.remote_root) as u32)
                    .unwrap_or(0);
                return FolderScan::SuspiciousEmptyAborted {
                    baseline_count,
                    listing_count: 0,
                };
            }
        }

        // Suspicious-shrink guard (v0.2.45 defense-in-depth): if the snapshot
        // had a substantial baseline for this prefix but the listing returned
        // less than half of it, assume the remote listing was truncated (russh
        // window/packet pressure, or any remaining edge case the exec-channel
        // drain fix didn't catch). Bail rather than emit phantom ToDeletes.
        // Threshold chosen conservatively — baseline ≥10 files AND listing
        // dropped >50% — so a legitimate workflow that bulk-deletes <50% of a
        // resource still propagates correctly.
        if let Some(snap) = self.snapshot {
            // #138: `count_under` returns the number of files (not dirs)
            // recorded in the snapshot under this prefix. Snapshot rows are
            // keyed by full remote path AND only files are ever inserted
            // (`SyncSnapshot::set` is called by entry-level dispatch, never
            // on directory metadata). The compared listing count below must
            // therefore filter out `is_dir` entries to make the >50%-shrink
            // arithmetic apples-to-apples — see filter on the next line.
            let baseline_n = snap.count_under(&f.remote_root);
            // Count just the file entries from remote_hits — dirs returned by
            // some listing paths (worker SFTP) inflate the count otherwise.
            let listing_files = remote_hits.iter().filter(|r| !r.is_dir).count();
            if baseline_n >= 10 && listing_files * 2 < baseline_n {
                eprintln!(
                    "[rift] drift scan: suspicious shrink for resource {} (baseline {} files, listing returned {} files) — aborting folder to prevent phantom deletes",
                    f.resource_name, baseline_n, listing_files,
                );
                crate::diagnostics::emit_with_fields(
                    crate::diagnostics::DiagStage::DriftScanProgress,
                    crate::diagnostics::DiagLevel::Warn,
                    Some(&f.resource_name),
                    None,
                    format!(
                        "suspicious listing shrink: baseline {baseline_n} files, listing {listing_files} — aborting"
                    ),
                    serde_json::json!({
                        "resource": f.resource_name,
                        "baseline_count": baseline_n,
                        "listing_count": listing_files,
                        "reason": "suspicious_shrink",
                    }),
                );
                return FolderScan::SuspiciousEmptyAborted {
                    baseline_count: baseline_n as u32,
                    listing_count: listing_files as u32,
                };
            }
        }

        let remote_root = f.remote_root.trim_end_matches('/');
        let mut remote_map: HashMap<String, RemoteStat> = HashMap::new();
        for r in remote_hits {
            if r.is_dir {
                continue;
            }
            let rel = if let Some(tail) = r.full_path.strip_prefix(remote_root) {
                tail.trim_start_matches('/').to_string()
            } else {
                r.name.clone()
            };
            if ignore::should_ignore(&rel) {
                continue;
            }
            remote_map.insert(
                rel,
                RemoteStat {
                    full_path: r.full_path.clone(),
                    size: r.size as i64,
                    mtime: r.last_modified,
                },
            );
        }

        let mut all_keys: HashSet<&String> = HashSet::new();
        for k in local_map.keys() {
            all_keys.insert(k);
        }
        for k in remote_map.keys() {
            all_keys.insert(k);
        }

        let mut entries = Vec::new();
        for rel in all_keys {
            // #73: per-entry cancel check. Each iteration may issue 0-2
            // `get_remote_sha1` calls (jitter detect + false-conflict
            // collapse); without this, an in-flight scan_folder would
            // complete fully even after the user clicked Cancel.
            if let Some(ct) = cancel {
                if ct.is_cancelled() {
                    break;
                }
            }
            let l = local_map.get(rel);
            let r = remote_map.get(rel);
            let has_local = l.is_some();
            let has_remote = r.is_some();

            let remote_path = match r {
                Some(rs) => rs.full_path.clone(),
                None => format!("{}/{}", f.remote_root.trim_end_matches('/'), rel),
            };
            let local_path = match l {
                Some(ls) => ls.full_path.to_string_lossy().to_string(),
                None => local_root
                    .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
                    .to_string_lossy()
                    .to_string(),
            };

            let snap = self
                .snapshot
                .and_then(|s| s.try_get(&remote_path));
            let has_snap = snap.is_some();

            let (bucket, reason): (DriftBucket, String) = if l.is_none() && r.is_some() {
                // v0.2.53 Mirror: if baseline says we previously had this
                // file locally and it's now gone, the user deleted it. In
                // Mirror mode propagate that delete to remote. In Normal
                // mode treat as remote-add and pull.
                if self.mirror && snap.is_some() {
                    (DriftBucket::ToDeleteRemote, "local deleted — removing remote (Mirror)".into())
                } else {
                    (DriftBucket::ToPull, "remote-only — pull".into())
                }
            } else if l.is_some() && r.is_none() {
                // Tombstone semantics: baseline IS the proof we'd previously
                // agreed this file existed remotely. Now remote doesn't have
                // it → that's a deletion propagation, NOT a "remote vanished
                // re-push" disaster. Pre-v0.2.33 this was misclassified as
                // ToPush, so teammates' deletes left ghost files behind +
                // risked accidental resurrection on next touch.
                if snap.is_some() {
                    (DriftBucket::ToDelete, "remote deleted — removing local".into())
                } else {
                    (DriftBucket::ToPush, "local-only — push".into())
                }
            } else if let (Some(ls), Some(rs), Some(snap_e)) = (l, r, snap.as_ref()) {
                // Both exist + baseline. Stat → optional hash → bucket.
                let mut local_changed = !SyncSnapshot::local_matches(snap_e, ls.size, ls.mtime);
                let mut remote_changed = !SyncSnapshot::remote_matches(snap_e, rs.size, rs.mtime);

                if let Some(snap_sha) = snap_e.sha1.as_deref() {
                    // Local jitter: stat-changed but size still matches snapshot column.
                    if local_changed && ls.size == snap_e.local_size && ls.size <= SHA1_MAX_BYTES {
                        if let Some(lh) = SyncSnapshot::compute_sha1(&ls.full_path) {
                            if lh.eq_ignore_ascii_case(snap_sha) {
                                local_changed = false;
                                if let Some(s) = self.snapshot {
                                    s.set(&remote_path, ls.size, ls.mtime,
                                        snap_e.remote_size, snap_e.remote_mtime_utc,
                                        Some(snap_sha.to_string()));
                                }
                            }
                        }
                    }
                    // Remote jitter: stat-changed but size still matches snapshot column.
                    if remote_changed && rs.size == snap_e.remote_size && hash_budget > 0 {
                        hash_budget -= 1;
                        if let Some(rh) = self.sftp.get_remote_sha1(&remote_path).await {
                            if rh.eq_ignore_ascii_case(snap_sha) {
                                remote_changed = false;
                                if let Some(s) = self.snapshot {
                                    s.set(&remote_path,
                                        snap_e.local_size, snap_e.local_mtime_utc,
                                        rs.size, rs.mtime,
                                        Some(snap_sha.to_string()));
                                }
                            }
                        }
                    }
                }

                if !local_changed && !remote_changed {
                    continue;
                }

                // False-conflict collapse: both sides "changed" but content matches.
                if local_changed && remote_changed && ls.size == rs.size && ls.size <= SHA1_MAX_BYTES
                    && hash_budget > 0
                {
                    if let Some(lh) = SyncSnapshot::compute_sha1(&ls.full_path) {
                        hash_budget -= 1;
                        if let Some(rh) = self.sftp.get_remote_sha1(&remote_path).await {
                            if lh.eq_ignore_ascii_case(&rh) {
                                if let Some(s) = self.snapshot {
                                    s.set(&remote_path, ls.size, ls.mtime, rs.size, rs.mtime, Some(lh));
                                }
                                continue;
                            }
                        }
                    }
                }

                if local_changed && !remote_changed {
                    (DriftBucket::ToPush, "local edited since last sync".into())
                } else if !local_changed && remote_changed {
                    (DriftBucket::ToPull, "remote edited since last sync".into())
                } else {
                    (DriftBucket::Conflict, "both sides changed since last sync".into())
                }
            } else if let (Some(ls), Some(rs), None) = (l, r, snap.as_ref()) {
                // First-scan opportunistic equality via content hash.
                if ls.size == rs.size && ls.size <= SHA1_MAX_BYTES && hash_budget > 0 {
                    if let Some(lh) = SyncSnapshot::compute_sha1(&ls.full_path) {
                        hash_budget -= 1;
                        if let Some(rh) = self.sftp.get_remote_sha1(&remote_path).await {
                            if lh.eq_ignore_ascii_case(&rh) {
                                if let Some(s) = self.snapshot {
                                    s.set(&remote_path, ls.size, ls.mtime, rs.size, rs.mtime, Some(lh));
                                }
                                continue;
                            }
                        }
                    }
                }
                // Fallback: looks-equal-by-stat (size + mtime tolerance).
                if ls.size == rs.size
                    && (ls.mtime - rs.mtime).num_seconds().abs() <= MTIME_TOLERANCE_SECS
                {
                    if let Some(s) = self.snapshot {
                        s.set(&remote_path, ls.size, ls.mtime, rs.size, rs.mtime, None);
                    }
                    continue;
                }
                // #75: sizes match but mtime tolerance failed AND we couldn't
                // confirm content-equality (hash failed / budget exhausted /
                // file too big). Arbitrary mtime-newer wins risks silent
                // overwrite of a re-extracted/rsync'd identical copy — surface
                // as Conflict so the user resolves explicitly.
                if ls.size == rs.size {
                    (
                        DriftBucket::Conflict,
                        "no baseline — sizes match but mtimes diverged (content unverified)".into(),
                    )
                } else if ls.mtime >= rs.mtime {
                    (DriftBucket::ToPush, "no baseline — local newer on first scan".into())
                } else {
                    (DriftBucket::ToPull, "no baseline — remote newer on first scan".into())
                }
            } else {
                unreachable!();
            };

            entries.push(DriftEntry {
                resource_name: f.resource_name.clone(),
                rel_path: rel.clone(),
                local_path,
                remote_path,
                bucket,
                local_exists: has_local,
                remote_exists: has_remote,
                local_size: l.map(|x| x.size).unwrap_or(0),
                remote_size: r.map(|x| x.size).unwrap_or(0),
                local_mtime: l.map(|x| x.mtime),
                remote_mtime: r.map(|x| x.mtime),
                has_snapshot: has_snap,
                reason,
            });
        }

        FolderScan::Drift(entries)
    }
}

#[allow(clippy::large_enum_variant)]
enum FolderScan {
    Drift(Vec<DriftEntry>),
    RemoteMissing,
    SuspiciousEmptyAborted { baseline_count: u32, listing_count: u32 },
}

struct LocalStat {
    size: i64,
    mtime: DateTime<Utc>,
    full_path: PathBuf,
}
struct RemoteStat {
    full_path: String,
    size: i64,
    mtime: DateTime<Utc>,
}

fn walk_local(root: &Path, dir: &Path, out: &mut HashMap<String, LocalStat>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        // Dirs: rel-path WITH trailing slash so segment rules in
        // ignore.rs:206-217 (which match `/{seg}/` or `{seg}/`) prune
        // `node_modules`, `.git`, `[disabled]`, etc. BEFORE we descend.
        // Bare-name `should_ignore("node_modules")` fails — input has no
        // trailing slash, so neither the contains nor starts_with branch
        // fires. Without this, the dir tree is walked even though the
        // per-file rel-path check at the bottom would later filter every
        // entry — wasted disk I/O on large `node_modules` etc. Mirrors
        // the #51 fix in `auto_sync::walk_local_rebaseline`.
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if meta.is_dir() {
            let mut probe = rel_s.clone();
            probe.push('/');
            if ignore::should_ignore(&probe) {
                continue;
            }
            walk_local(root, &path, out);
            continue;
        }
        if ignore::should_ignore(&rel_s) {
            continue;
        }
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
            .unwrap_or_else(Utc::now);
        out.insert(
            rel_s,
            LocalStat {
                size: meta.len() as i64,
                mtime,
                full_path: path.clone(),
            },
        );
    }
}

// (Phase 1f): local `should_ignore_basic` removed — full WPF ignore-rule parity
// is provided by `crate::sync::ignore::should_ignore`. Tests for the rule set
// live in `sync::ignore::tests`.
