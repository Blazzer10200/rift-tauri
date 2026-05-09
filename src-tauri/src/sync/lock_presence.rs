// Phase 1d: advisory presence locks — `.rift-lock` JSON files dropped on the
// remote alongside the file being edited. Mirrors WPF Services/Edit/LockPresence.cs.
//
// Acquire on first FSW dirty event for a path; release on successful flush.
// Poll loop walks watched roots every 10s, surfaces foreign locks (not ours)
// via Tauri event `autosync://locks` so the UI can render presence badges.
// Stale locks (>180s) get swept — usually a Rift crash mid-edit.
//
// Last-writer-wins (no atomic CAS over SFTP) — fine for advisory awareness.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

use crate::sftp::SftpClient;
use crate::transport::env::{current_user, hostname, short_id};

const STALE_SEC: i64 = 180;
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

pub struct LockPresence {
    sftp: Arc<SftpClient>,
    remote_root: String,
    my_user: String,
    my_host: String,
    my_locks: DashSet<String>,
    active_by_path: DashMap<String, RemoteLock>,
    scoped_provider: Mutex<Option<ScopedFoldersFn>>,
    poll_task: Mutex<Option<JoinHandle<()>>>,
    stop_tx: watch::Sender<bool>,
    app: AppHandle,
    disposed: AtomicBool,
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
            active_by_path: DashMap::new(),
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
        let sftp = self.sftp.clone();
        let cleanup = tokio::spawn(async move {
            for p in paths {
                let _ = sftp.delete(&p).await;
            }
        });
        let _ = tokio::time::timeout(Duration::from_secs(2), cleanup).await;
    }

    pub fn find_lock_by_other(&self, remote_file: &str) -> Option<RemoteLock> {
        self.active_by_path.get(remote_file).map(|kv| kv.clone())
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
        let _ = self.sftp.delete(&lock_path).await;
    }

    async fn poll_once(&self) -> Result<(), String> {
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
            for e in entries {
                if self.my_locks.contains(&e.full_path) {
                    continue;
                }
                let body = match self.try_read_lock(&e.full_path).await {
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

        // ADVISORY-ONLY: between clear() and the re-insert loop, concurrent
        // find_lock_by_other() callers see an empty map. Refreshes are 10s
        // apart so the window is bounded; an upload that races through this
        // gap may proceed without seeing a foreign lock for one cycle. Locks
        // are advisory anyway — last-writer-wins is the documented model.
        self.active_by_path.clear();
        for l in &found {
            self.active_by_path.insert(l.file_path.clone(), l.clone());
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

    /// Used by Drop-style explicit teardown when LockPresence is held by Arc.
    pub fn local_paths_owned(&self) -> Vec<PathBuf> {
        self.my_locks.iter().map(|s| PathBuf::from(s.as_str())).collect()
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
