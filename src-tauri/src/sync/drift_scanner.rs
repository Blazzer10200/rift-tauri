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

use crate::sftp::{RemoteEntry, SftpClient};
use crate::state::SyncSnapshot;
use crate::sync::ignore;

const MAX_DEPTH: usize = 12;
const REMOTE_HASH_BUDGET_PER_FOLDER: i32 = 25;
/// Match WPF AutoSync.Sha1MaxBytes — files over this size skip hashing.
const SHA1_MAX_BYTES: i64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftBucket {
    Synced,
    ToPush,
    ToPull,
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

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub entries: Vec<DriftEntry>,
    pub last_batch_listing_error: Option<String>,
    pub remote_folders_missing: Vec<String>,
}

pub struct DriftScanner<'a> {
    sftp: &'a SftpClient,
    snapshot: Option<&'a SyncSnapshot>,
}

impl<'a> DriftScanner<'a> {
    pub fn new(sftp: &'a SftpClient, snapshot: Option<&'a SyncSnapshot>) -> Self {
        Self { sftp, snapshot }
    }

    pub async fn scan(&self, folders: &[FolderTarget]) -> ScanResult {
        let mut entries = Vec::new();
        let mut last_batch_error: Option<String> = None;
        let mut remote_folders_missing = Vec::new();

        let roots: Vec<String> = folders.iter().map(|f| f.remote_root.clone()).collect();
        let listings = match self
            .sftp
            .list_recursive_batch(&roots, MAX_DEPTH, None, 4)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                last_batch_error = Some(e);
                HashMap::new()
            }
        };

        for f in folders {
            let remote_hits = listings
                .get(&f.remote_root)
                .cloned()
                .unwrap_or_default();
            let folder_result = self.scan_folder(f, &remote_hits).await;
            match folder_result {
                FolderScan::Drift(mut v) => entries.append(&mut v),
                FolderScan::RemoteMissing => {
                    remote_folders_missing.push(f.remote_root.clone());
                }
                FolderScan::SuspiciousEmptyAborted => {
                    // documented data-safety bail — no entries surfaced
                }
            }
        }

        ScanResult {
            entries,
            last_batch_listing_error: last_batch_error,
            remote_folders_missing,
        }
    }

    async fn scan_folder(&self, f: &FolderTarget, remote_hits: &[RemoteEntry]) -> FolderScan {
        let mut hash_budget: i32 = REMOTE_HASH_BUDGET_PER_FOLDER;
        // Local recursive walk. Full ignore-rule parity now via `ignore::should_ignore`.
        let local_root = Path::new(&f.local_root);
        let mut local_map: HashMap<String, LocalStat> = HashMap::new();
        if local_root.exists() {
            walk_local(local_root, local_root, &mut local_map);
        }

        // Critical data-safety guard: empty remote + non-empty local. Confirm
        // remote folder existence before deciding (mirrors WPF v13.55.1 fix).
        if remote_hits.is_empty() && !local_map.is_empty() {
            let exists = self.sftp.remote_exists(&f.remote_root).await;
            if !exists {
                return FolderScan::RemoteMissing;
            } else {
                // Listing failed silently. Bail rather than false-push every local file.
                return FolderScan::SuspiciousEmptyAborted;
            }
        }

        let rr_len = f.remote_root.trim_end_matches('/').len();
        let mut remote_map: HashMap<String, RemoteStat> = HashMap::new();
        for r in remote_hits {
            if r.is_dir {
                continue;
            }
            let rel = if r.full_path.len() > rr_len {
                r.full_path[rr_len..].trim_start_matches('/').to_string()
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
                (DriftBucket::ToPull, "remote-only — pull".into())
            } else if l.is_some() && r.is_none() {
                let reason = if snap.is_some() {
                    "remote vanished — re-pushing local".into()
                } else {
                    "local-only — push".into()
                };
                (DriftBucket::ToPush, reason)
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
                // Fallback: looks-equal-by-stat (size + 2s mtime tolerance).
                if ls.size == rs.size && (ls.mtime - rs.mtime).num_seconds().abs() <= 2 {
                    if let Some(s) = self.snapshot {
                        s.set(&remote_path, ls.size, ls.mtime, rs.size, rs.mtime, None);
                    }
                    continue;
                }
                (DriftBucket::Conflict, "no baseline — first scan with both sides present".into())
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

enum FolderScan {
    Drift(Vec<DriftEntry>),
    RemoteMissing,
    SuspiciousEmptyAborted,
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
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if ignore::should_ignore(name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk_local(root, &path, out);
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
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
