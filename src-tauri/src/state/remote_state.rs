use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use super::paths::{atomic_write_json, cache_path};

// Per-file "last known remote state" — drives the conflict detector. After a
// successful push or pull, record (size, mtime) of the remote at that moment.
// Before the next push, AutoSync re-fetches remote state and compares against
// this cache; mismatch = another dev pushed since we last touched → conflict.
//
// File-format compat: ~/.rift/state-<profileKey>.json. PascalCase fields.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Entry {
    pub size: i64,
    pub mtime_utc: DateTime<Utc>,
}

pub struct RemoteStateCache {
    path: PathBuf,
    data: Mutex<HashMap<String, Entry>>,
}

impl RemoteStateCache {
    pub fn new(profile_key: &str) -> std::io::Result<Self> {
        let path = cache_path("state", profile_key)?;
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<HashMap<String, Entry>>(&s).ok())
            .unwrap_or_default();
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }

    pub fn try_get(&self, remote_path: &str) -> Option<Entry> {
        lock(&self.data).get(remote_path).cloned()
    }

    pub fn set(&self, remote_path: &str, size: i64, mtime_utc: DateTime<Utc>) {
        {
            let mut g = lock(&self.data);
            g.insert(remote_path.to_string(), Entry { size, mtime_utc });
        }
        let _ = self.save();
    }

    pub fn forget(&self, remote_path: &str) {
        let removed = {
            let mut g = lock(&self.data);
            g.remove(remote_path).is_some()
        };
        if removed {
            let _ = self.save();
        }
    }

    fn save(&self) -> std::io::Result<()> {
        let snapshot = lock(&self.data).clone();
        let json = serde_json::to_string(&snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        atomic_write_json(&self.path, &json)
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
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
            "test-rs-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64 + n
        )
    }
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn set_get_forget_roundtrip() {
        let key = unique_key();
        let c = RemoteStateCache::new(&key).unwrap();
        let _cu = Cleanup(c.path.clone());
        let mtime = Utc.with_ymd_and_hms(2026, 5, 7, 0, 0, 0).unwrap();
        c.set("/r/x.lua", 42, mtime);
        let got = c.try_get("/r/x.lua").unwrap();
        assert_eq!(got.size, 42);
        assert_eq!(got.mtime_utc, mtime);
        c.forget("/r/x.lua");
        assert!(c.try_get("/r/x.lua").is_none());
    }

    #[test]
    fn round_trip_via_disk() {
        let key = unique_key();
        let path = {
            let c = RemoteStateCache::new(&key).unwrap();
            c.set("/r/y.lua", 99, Utc.with_ymd_and_hms(2026, 5, 7, 1, 2, 3).unwrap());
            c.path.clone()
        };
        let c2 = RemoteStateCache::new(&key).unwrap();
        let got = c2.try_get("/r/y.lua").expect("loaded");
        assert_eq!(got.size, 99);
        let _ = std::fs::remove_file(&path);
    }
}
