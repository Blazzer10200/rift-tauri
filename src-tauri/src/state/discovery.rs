// Persistent cache of "remote resource folders we've previously discovered" —
// drives fast-attach on connect. File-format compat: ~/.rift/discovery-<profileKey>.json
// w/ camelCase fields ({ folders: [], cachedAt: ISO-8601 }).
//
// NOTE: implemented + tested but not yet wired to any Tauri command. Reserved
// for the Phase 6 fast-reconnect path (skip full manifest scan when the
// previously-cached discovery is still fresh). Keep working — don't gut.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use super::paths::{atomic_write_json, cache_path};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub cached_at: String,
}

pub struct ResourceDiscoveryCache {
    path: PathBuf,
    inner: Mutex<Snapshot>,
}

impl ResourceDiscoveryCache {
    pub fn new(profile_key: &str) -> std::io::Result<Self> {
        let path = cache_path("discovery", profile_key)?;
        let inner = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Snapshot>(&s).ok())
            .unwrap_or_default();
        Ok(Self {
            path,
            inner: Mutex::new(inner),
        })
    }

    pub fn get(&self) -> (Vec<String>, Option<DateTime<Utc>>) {
        let g = lock(&self.inner);
        let when = DateTime::parse_from_rfc3339(&g.cached_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));
        (g.folders.clone(), when)
    }

    pub fn save(&self, folders: Vec<String>) -> std::io::Result<()> {
        let now = Utc::now();
        let cached_at = now.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
        let snap = Snapshot { folders, cached_at };
        {
            let mut g = lock(&self.inner);
            *g = snap.clone();
        }
        let json = serde_json::to_string(&snap)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        atomic_write_json(&self.path, &json)
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| {
        // Audit M5: log poison events so silent recovery doesn't hide
        // the panic that originally poisoned the mutex.
        log::error!("discovery: recovering from poisoned mutex: {e}");
        e.into_inner()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    fn unique_key() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!(
            "test-disc-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64 + n
        )
    }

    #[test]
    fn save_then_load_roundtrips() {
        let key = unique_key();
        let path = {
            let c = ResourceDiscoveryCache::new(&key).unwrap();
            c.save(vec!["/a".into(), "/b/c".into()]).unwrap();
            c.path.clone()
        };
        let c2 = ResourceDiscoveryCache::new(&key).unwrap();
        let (folders, when) = c2.get();
        assert_eq!(folders, vec!["/a".to_string(), "/b/c".to_string()]);
        assert!(when.is_some(), "cachedAt should round-trip parseable");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_cache_returns_default() {
        let c = ResourceDiscoveryCache::new(&unique_key()).unwrap();
        let (folders, when) = c.get();
        assert!(folders.is_empty());
        assert!(when.is_none());
    }
}
