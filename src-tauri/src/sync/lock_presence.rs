// Phase 1d: advisory presence locks — `.rift-lock` JSON files dropped on the
// remote alongside the file being edited. Mirrors WPF Services/Edit/LockPresence.cs.
//
// Acquire on first FSW dirty event for a path; release on successful flush.
// Poll loop walks watched roots every 10s, surfaces foreign locks (not ours)
// via Tauri event `autosync://locks` so the UI can render presence badges.
// Stale locks (>180s) get swept — usually a Rift crash mid-edit.
//
// Heartbeat: own locks get re-stamped every HEARTBEAT_SEC so active long-form
// edits don't get their lock swept by another Rift's stale sweep.
//
// Last-writer-wins (no atomic CAS over SFTP) — fine for advisory awareness.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

use crate::sftp::SftpClient;
use crate::transport::env::{current_user, hostname, short_id};

const STALE_SEC: i64 = 180;
const HEARTBEAT_SEC: u64 = 60;
const POLL_INTERVAL_MS: u64 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteLock {
    pub file_path: String,
    pub user: String,
    pub host: String,
    pub since: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LockBody {
    user: String,
    host: String,
    since: String,
}

pub type ScopedFoldersFn = Arc<dyn Fn() -> Vec<String> + Send + Sync>;

/// Skip stale-lock delete retries after this many consecutive failures per
/// path. Prevents a permanently-unreachable lock (perms revoked, remote
/// dir gone) from generating warn-log noise on every 10s sweep cycle.
const STALE_DELETE_MAX_FAILS: u8 = 3;

pub struct LockPresence {
    sftp: Arc<SftpClient>,
    remote_root: String,
    my_user: String,
    my_host: String,
    my_locks: DashSet<String>,
    last_heartbeat: DashMap<String, Instant>,
    active_by_path: RwLock<HashMap<String, RemoteLock>>,
    scoped_provider: Mutex<Option<ScopedFoldersFn>>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    stop_tx: watch::Sender<bool>,
    app: AppHandle,
    disposed: AtomicBool,
    /// Per-path failed-delete counter for stale-lock sweep. Reset on success;
    /// skips further attempts once `STALE_DELETE_MAX_FAILS` is hit.
    stale_delete_fails: DashMap<String, u8>,
}

impl LockPresence {
    pub fn new(sftp: Arc<SftpClient>, remote_root: String, app: AppHandle) -> Arc<Self> {
        let my_user = current_user();
        let my_host = hostname().unwrap_or_else(|| "unknown".into());
        let (stop_tx, _) = watch::channel(false);
        Arc::new(Self {
            sftp,
            remote_root,
            my_user,
            my_host,
            my_locks: DashSet::new(),
            last_heartbeat: DashMap::new(),
            stale_delete_fails: DashMap::new(),
            active_by_path: RwLock::new(HashMap::new()),
            scoped_provider: Mutex::new(None),
            poll_task: Mutex::new(None),
            stop_tx,
            app,
            disposed: AtomicBool::new(false),
        })
    }

    pub async fn set_scoped_provider(&self, f: ScopedFoldersFn) {
        *self.scoped_provider.lock().await = Some(f);
    }

    pub async fn start(self: &Arc<Self>) {
        if self.disposed.load(Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        let mut stop = self.stop_tx.subscribe();
        let task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));
            // First poll fires immediately — skip the immediate firstwait so the first
            // tick happens after one full interval (matches WPF Task.Delay-then-poll).
            tick.tick().await;
            loop {
                tokio::select! {
                    _ = stop.changed() => { if *stop.borrow() { break; } }
                    _ = tick.tick() => {
                        let _ = me.poll_once().await;
                    }
                }
            }
        });
        *self.poll_task.lock().await = Some(task);
    }

    pub async fn stop(&self) {
        if self.disposed.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = self.stop_tx.send(true);
        if let Some(h) = self.poll_task.lock().await.take() {
            let _ = h.await;
        }
        // Bounded cleanup of our own locks.
        let paths: Vec<String> = self.my_locks.iter().map(|s| s.clone()).collect();
        self.my_locks.clear();
        self.last_heartbeat.clear();
        let sftp = self.sftp.clone();
        let cleanup = tokio::spawn(async move {
            for p in paths {
                let _ = sftp.delete(&p).await;
            }
        });
        let _ = tokio::time::timeout(Duration::from_secs(2), cleanup).await;
    }

    pub fn find_lock_by_other(&self, remote_file: &str) -> Option<RemoteLock> {
        self.active_by_path
            .read()
            .ok()
            .and_then(|g| g.get(remote_file).cloned())
    }

    /// Snapshot of currently-known foreign locks across the watched roots.
    /// Sourced from the poll cache; refreshed every POLL_INTERVAL_MS. Used by
    /// the Assistant `WorkspaceContext` addendum to surface multi-writer state
    /// in the system prompt.
    pub fn active_locks(&self) -> Vec<RemoteLock> {
        self.active_by_path
            .read()
            .ok()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn acquire(&self, remote_file: &str) {
        if self.disposed.load(Ordering::SeqCst) || remote_file.is_empty() {
            return;
        }
        let lock_path = format!("{remote_file}.rift-lock");
        if !self.my_locks.insert(lock_path.clone()) {
            return; // already own
        }
        let body = serde_json::json!({
            "user": self.my_user,
            "host": self.my_host,
            "since": Utc::now().to_rfc3339(),
        });
        let bytes = body.to_string();
        if self.sftp.upload_bytes(bytes.as_bytes(), &lock_path).await.is_err() {
            self.my_locks.remove(&lock_path);
        } else {
            self.last_heartbeat.insert(lock_path, Instant::now());
        }
    }

    pub async fn release(&self, remote_file: &str) {
        if self.disposed.load(Ordering::SeqCst) || remote_file.is_empty() {
            return;
        }
        let lock_path = format!("{remote_file}.rift-lock");
        if self.my_locks.remove(&lock_path).is_none() {
            return;
        }
        self.last_heartbeat.remove(&lock_path);
        let _ = self.sftp.delete(&lock_path).await;
    }

    /// Re-stamp each held lock's `since` field if it's been ≥ HEARTBEAT_SEC
    /// since the last refresh. Keeps long-running edits alive past the
    /// foreign-sweep threshold. Failed writes leave the lock as-is — next
    /// poll retries; if connectivity is gone the lock goes stale and gets
    /// reclaimed by another Rift, which is the desired behavior.
    /// Watch-attach cleanup: remove stale `.rift-lock` files created by this
    /// local user. Foreign stale locks remain poll-owned so badges stay honest.
    pub async fn sweep_stale_mine(&self, folder: &str, depth: usize) -> Result<usize, String> {
        let entries = self
            .sftp
            .list_recursive(folder, depth, Some(&[".rift-lock"]))
            .await?;
        let mut removed = 0usize;
        for e in entries {
            if self.my_locks.contains(&e.full_path) {
                continue;
            }
            // Skip paths that have already failed STALE_DELETE_MAX_FAILS
            // times — keeps repeated warn-log noise out of the diag bus.
            if self
                .stale_delete_fails
                .get(&e.full_path)
                .map(|c| *c >= STALE_DELETE_MAX_FAILS)
                .unwrap_or(false)
            {
                continue;
            }
            let Some(body) = self.try_read_lock(&e.full_path).await else { continue };
            if body.user != self.my_user {
                continue;
            }
            let since = chrono::DateTime::parse_from_rfc3339(&body.since)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            if (Utc::now() - since).num_seconds() <= STALE_SEC {
                continue;
            }
            if self.sftp.delete(&e.full_path).await.success {
                removed += 1;
                self.stale_delete_fails.remove(&e.full_path);
            } else {
                let count = {
                    let mut entry =
                        self.stale_delete_fails.entry(e.full_path.clone()).or_insert(0);
                    *entry = entry.saturating_add(1);
                    *entry
                };
                log::warn!(
                    "stale lock cleanup failed for {} (attempt {}/{})",
                    e.full_path,
                    count,
                    STALE_DELETE_MAX_FAILS
                );
            }
        }
        Ok(removed)
    }

    async fn refresh_my_locks(&self) {
        let due: Vec<String> = self
            .last_heartbeat
            .iter()
            .filter_map(|e| {
                if e.value().elapsed().as_secs() >= HEARTBEAT_SEC {
                    Some(e.key().clone())
                } else {
                    None
                }
            })
            .collect();
        use futures::stream::{FuturesUnordered, StreamExt};
        let due_mine: Vec<String> = due
            .into_iter()
            .filter(|lp| {
                if !self.my_locks.contains(lp) {
                    self.last_heartbeat.remove(lp);
                    false
                } else {
                    true
                }
            })
            .collect();
        let now_iso = Utc::now().to_rfc3339();
        let mut up_tasks: FuturesUnordered<_> = due_mine
            .into_iter()
            .map(|lock_path| {
                let body = serde_json::json!({
                    "user": self.my_user.clone(),
                    "host": self.my_host.clone(),
                    "since": now_iso.clone(),
                })
                .to_string();
                let sftp = Arc::clone(&self.sftp);
                async move {
                    let ok = sftp.upload_bytes(body.as_bytes(), &lock_path).await.is_ok();
                    (lock_path, ok)
                }
            })
            .collect();
        while let Some((lock_path, ok)) = up_tasks.next().await {
            if ok {
                self.last_heartbeat.insert(lock_path, Instant::now());
            }
        }
    }

    async fn poll_once(&self) -> Result<(), String> {
        self.refresh_my_locks().await;
        let scoped = {
            let g = self.scoped_provider.lock().await;
            g.as_ref().map(|f| f())
        };
        let (folders, depth): (Vec<String>, usize) = match scoped {
            Some(v) if !v.is_empty() => (v, 4),
            _ => {
                if self.remote_root.is_empty() {
                    return Ok(());
                }
                (vec![self.remote_root.clone()], 6)
            }
        };

        use futures::stream::{FuturesUnordered, StreamExt};
        let mut found: Vec<RemoteLock> = Vec::new();
        for folder in &folders {
            let entries = match self
                .sftp
                .list_recursive(folder, depth, Some(&[".rift-lock"]))
                .await
            {
                Ok(v) => v,
                Err(_) => continue,
            };
            let mut read_tasks: FuturesUnordered<_> = entries
                .into_iter()
                .filter(|e| !self.my_locks.contains(&e.full_path))
                .map(|e| async move {
                    let body = self.try_read_lock(&e.full_path).await;
                    (e, body)
                })
                .collect();
            while let Some((e, body_opt)) = read_tasks.next().await {
                let body = match body_opt {
                    Some(b) => b,
                    None => continue,
                };
                let since = chrono::DateTime::parse_from_rfc3339(&body.since)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                if (Utc::now() - since).num_seconds() > STALE_SEC {
                    let _ = self.sftp.delete(&e.full_path).await;
                    continue;
                }
                let file_being_edited = if e.full_path.ends_with(".rift-lock") {
                    e.full_path[..e.full_path.len() - ".rift-lock".len()].to_string()
                } else {
                    e.full_path.clone()
                };
                found.push(RemoteLock {
                    file_path: file_being_edited,
                    user: body.user,
                    host: body.host,
                    since,
                });
            }
        }

        let mut next = HashMap::with_capacity(found.len());
        for l in &found {
            next.insert(l.file_path.clone(), l.clone());
        }
        if let Ok(mut g) = self.active_by_path.write() {
            *g = next;
        }
        let _ = self.app.emit("autosync://locks", &found);
        Ok(())
    }

    async fn try_read_lock(&self, remote_lock_path: &str) -> Option<LockBody> {
        let scratch = std::env::temp_dir().join(format!(
            "rift-lock-{}-{}",
            std::process::id(),
            short_id()
        ));
        let _ = std::fs::create_dir_all(&scratch);
        let local = self.sftp.download_file(remote_lock_path, &scratch).await.ok()?;
        let text = std::fs::read_to_string(&local).ok();
        let _ = std::fs::remove_file(&local);
        let _ = std::fs::remove_dir(&scratch);
        let text = text?;
        serde_json::from_str::<LockBody>(&text).ok()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_path_suffix() {
        let lp = format!("{}{}", "/srv/foo.lua", ".rift-lock");
        assert!(lp.ends_with(".rift-lock"));
    }

    #[test]
    fn lock_body_roundtrip() {
        let raw = r#"{"user":"alice","host":"box","since":"2026-05-08T12:00:00Z"}"#;
        let b: LockBody = serde_json::from_str(raw).unwrap();
        assert_eq!(b.user, "alice");
        assert_eq!(b.host, "box");
    }
}
