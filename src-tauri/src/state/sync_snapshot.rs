use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::paths::{atomic_write_json, cache_path};

// Per-file "synced snapshot" — records the LOCAL+REMOTE state at the moment
// we know both sides agreed (after a successful push, pull, or seeded on
// first scan when local mirrors remote). Drives the 3-way drift diff.
//
// File-format compat w/ WPF: ~/.rift/snapshot-<profileKey>.json. Top-level
// dict keyed by remote path. Field names = PascalCase (matches
// System.Text.Json default policy on the WPF side).

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Entry {
    pub local_size: i64,
    pub local_mtime_utc: DateTime<Utc>,
    pub remote_size: i64,
    pub remote_mtime_utc: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
}

pub struct SyncSnapshot {
    path: PathBuf,
    data: Mutex<HashMap<String, Entry>>,
}

pub const MTIME_TOLERANCE_SECS: i64 = 2;
/// Files over this size skip SHA1 hashing — both AutoSync's post-flush snapshot
/// refresh and DriftScanner's jitter-collapse paths consult this. Matches WPF
/// `AutoSync.Sha1MaxBytes` = 64 MiB.
pub const SHA1_MAX_BYTES: i64 = 64 * 1024 * 1024;

impl SyncSnapshot {
    pub fn new(profile_key: &str) -> std::io::Result<Self> {
        let path = cache_path("snapshot", profile_key)?;
        let data = load_or_default(&path);
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }

    pub fn try_get(&self, remote_path: &str) -> Option<Entry> {
        lock(&self.data).get(remote_path).cloned()
    }

    pub fn set(
        &self,
        remote_path: &str,
        local_size: i64,
        local_mtime_utc: DateTime<Utc>,
        remote_size: i64,
        remote_mtime_utc: DateTime<Utc>,
        sha1: Option<String>,
    ) {
        let mut g = lock(&self.data);
        g.insert(
            remote_path.to_string(),
            Entry {
                local_size,
                local_mtime_utc,
                remote_size,
                remote_mtime_utc,
                sha1,
            },
        );
        let _ = self.save_locked(&g);
    }

    pub fn forget(&self, remote_path: &str) {
        let mut g = lock(&self.data);
        if g.remove(remote_path).is_some() {
            let _ = self.save_locked(&g);
        }
    }

    pub fn count(&self) -> usize {
        lock(&self.data).len()
    }

    /// Count baseline entries whose remote_path falls under `remote_root_prefix`.
    /// Used by drift_scanner's suspicious-shrink guard to detect truncated
    /// remote listings before classifying missing files as ToDelete.
    pub fn count_under(&self, remote_root_prefix: &str) -> usize {
        let prefix = remote_root_prefix.trim_end_matches('/');
        let g = lock(&self.data);
        g.keys()
            .filter(|k| Self::path_under(k, prefix))
            .count()
    }

    fn path_under(key: &str, prefix: &str) -> bool {
        if let Some(rest) = key.strip_prefix(prefix) {
            rest.is_empty() || rest.starts_with('/')
        } else {
            false
        }
    }

    /// Atomically replace every snapshot row under `remote_root_prefix` with
    /// `new_entries` (keyed by full remote_path). Caller is responsible for
    /// constructing entries that reflect current ground truth — see
    /// `lib::sync_rebaseline_folder`. Returns the count delta (new − old) so
    /// the UI can report what changed.
    pub fn replace_under(
        &self,
        remote_root_prefix: &str,
        new_entries: HashMap<String, Entry>,
    ) -> std::io::Result<(usize, usize)> {
        let prefix = remote_root_prefix.trim_end_matches('/');
        let mut g = lock(&self.data);
        let old_count = g.keys().filter(|k| Self::path_under(k, prefix)).count();
        g.retain(|k, _| !Self::path_under(k, prefix));
        for (k, v) in new_entries {
            g.insert(k, v);
        }
        let new_count = g.keys().filter(|k| Self::path_under(k, prefix)).count();
        self.save_locked(&g)?;
        Ok((old_count, new_count))
    }

    pub fn local_matches(e: &Entry, size: i64, mtime_utc: DateTime<Utc>) -> bool {
        size == e.local_size
            && (mtime_utc - e.local_mtime_utc).num_milliseconds().abs()
                <= MTIME_TOLERANCE_SECS * 1000
    }

    pub fn remote_matches(e: &Entry, size: i64, mtime_utc: DateTime<Utc>) -> bool {
        size == e.remote_size
            && (mtime_utc - e.remote_mtime_utc).num_milliseconds().abs()
                <= MTIME_TOLERANCE_SECS * 1000
    }

    pub fn compute_sha1(local_path: &Path) -> Option<String> {
        use std::io::{BufReader, Read};

        let f = std::fs::File::open(local_path).ok()?;
        let mut reader = BufReader::with_capacity(64 * 1024, f);
        let mut hasher = Sha1::new();
        let mut buf = [0u8; 8 * 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => hasher.update(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
        Some(hex_upper(&hasher.finalize()))
    }

    fn save_locked(&self, snapshot: &HashMap<String, Entry>) -> std::io::Result<()> {
        let json = serde_json::to_string(snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        atomic_write_json(&self.path, &json)
    }
}

/// Mutex lock helper — recovers a poisoned mutex by taking its inner state.
/// All sync_snapshot consumers (autosync flush loop, drift scan) treat the
/// snapshot as a recoverable cache, so cascading panics from one panic-while-
/// holding incident would be far worse than a stale-but-readable map.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn load_or_default(path: &Path) -> HashMap<String, Entry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // write! into a pre-allocated String avoids the per-byte format!
        // String allocation; about 20× cheaper for SHA1 (20 bytes).
        let _ = write!(s, "{:02X}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_key() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!(
            "test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64 + n
        )
    }

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn make() -> (SyncSnapshot, Cleanup) {
        let key = unique_key();
        let snap = SyncSnapshot::new(&key).expect("new");
        let p = snap.path.clone();
        (snap, Cleanup(p))
    }

    #[test]
    fn set_then_try_get_returns_stored_entry() {
        let (snap, _c) = make();
        let mtime = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        snap.set(
            "/remote/foo.lua",
            1024,
            mtime,
            1024,
            mtime,
            Some("abc123".into()),
        );
        let e = snap.try_get("/remote/foo.lua").expect("present");
        assert_eq!(e.local_size, 1024);
        assert_eq!(e.remote_size, 1024);
        assert_eq!(e.sha1.as_deref(), Some("abc123"));
    }

    #[test]
    fn try_get_missing_key_returns_none() {
        let (snap, _c) = make();
        assert!(snap.try_get("/nope").is_none());
    }

    #[test]
    fn forget_removes_entry() {
        let (snap, _c) = make();
        let mtime = Utc::now();
        snap.set("/remote/bar.lua", 100, mtime, 100, mtime, None);
        assert!(snap.try_get("/remote/bar.lua").is_some());
        snap.forget("/remote/bar.lua");
        assert!(snap.try_get("/remote/bar.lua").is_none());
    }

    #[test]
    fn local_matches_within_tolerance() {
        let mtime = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let e = Entry {
            local_size: 1024,
            local_mtime_utc: mtime,
            remote_size: 1024,
            remote_mtime_utc: mtime,
            sha1: None,
        };
        assert!(SyncSnapshot::local_matches(&e, 1024, mtime));
        assert!(SyncSnapshot::local_matches(
            &e,
            1024,
            mtime + chrono::Duration::milliseconds(1500)
        ));
    }

    #[test]
    fn local_matches_outside_tolerance() {
        let mtime = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let e = Entry {
            local_size: 1024,
            local_mtime_utc: mtime,
            remote_size: 1024,
            remote_mtime_utc: mtime,
            sha1: None,
        };
        assert!(!SyncSnapshot::local_matches(
            &e,
            1024,
            mtime + chrono::Duration::seconds(3)
        ));
        assert!(!SyncSnapshot::local_matches(&e, 2048, mtime));
    }

    #[test]
    fn remote_matches_uses_remote_size() {
        let mtime = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let e = Entry {
            local_size: 1024,
            local_mtime_utc: mtime,
            remote_size: 2048,
            remote_mtime_utc: mtime,
            sha1: None,
        };
        assert!(SyncSnapshot::remote_matches(&e, 2048, mtime));
        assert!(!SyncSnapshot::remote_matches(&e, 1024, mtime));
    }

    #[test]
    fn count_reflects_live_entries() {
        let (snap, _c) = make();
        assert_eq!(snap.count(), 0);
        let mtime = Utc::now();
        snap.set("/a", 1, mtime, 1, mtime, None);
        snap.set("/b", 2, mtime, 2, mtime, None);
        assert_eq!(snap.count(), 2);
        snap.forget("/a");
        assert_eq!(snap.count(), 1);
    }

    #[test]
    fn compute_sha1_known_input() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("rift-sha1-{}.txt", unique_key()));
        std::fs::write(&p, "rift").unwrap();
        let h = SyncSnapshot::compute_sha1(&p);
        let _ = std::fs::remove_file(&p);
        assert_eq!(h.as_deref(), Some("37A219101CAFE5AC76B86BCD4DBD2A3884A822DC"));
    }

    #[test]
    fn compute_sha1_missing_file_returns_none() {
        let h = SyncSnapshot::compute_sha1(Path::new("C:/nonexistent/rift-missing.txt"));
        assert!(h.is_none());
    }

    // Compat smoke test against the user's real ~/.rift/snapshot-endure-rp.json
    // produced by WPF Rift v13.55.x. Skipped automatically if absent so CI / fresh
    // checkouts don't fail. Run explicitly: `cargo test -- --ignored real_wpf`.
    #[test]
    #[ignore]
    fn real_wpf_snapshot_deserializes() {
        let p = super::super::paths::cache_path("snapshot", "endure-rp").unwrap();
        if !p.exists() {
            eprintln!("skip: {} not present", p.display());
            return;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        let map: HashMap<String, Entry> = serde_json::from_str(&text).expect("parse");
        assert!(!map.is_empty(), "real snapshot should have entries");
        let (k, v) = map.iter().next().unwrap();
        eprintln!(
            "compat OK — {} entries, sample: {} → size local={} remote={}",
            map.len(),
            k,
            v.local_size,
            v.remote_size
        );
    }

    #[test]
    fn round_trip_via_disk_preserves_entry() {
        let key = unique_key();
        let path = {
            let snap = SyncSnapshot::new(&key).unwrap();
            let mtime = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
            snap.set("/r/a.lua", 10, mtime, 20, mtime, Some("DEADBEEF".into()));
            snap.path.clone()
        };
        let snap2 = SyncSnapshot::new(&key).unwrap();
        let e = snap2.try_get("/r/a.lua").expect("loaded");
        assert_eq!(e.local_size, 10);
        assert_eq!(e.remote_size, 20);
        assert_eq!(e.sha1.as_deref(), Some("DEADBEEF"));
        let _ = std::fs::remove_file(&path);
    }
}
