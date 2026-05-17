// Faithful port of EditInPlace.cs. Workflow:
//
//   begin_edit(remote_path)
//     → download remote → unique tmp subdir
//     → spawn notify watcher on that file
//     → on save: 400ms debounce → mtime/size delta check
//        → emit `edit://changed` (UI shows reupload prompt)
//   reupload(remote_path) → upload tmp → emit `edit://reuploaded`
//   close(remote_path) → drop watcher + delete subdir
//   Drop → close all watches + delete _tmp_root
//
// Phase 4 will wire the reupload prompt UI on top of these events. The WPF
// `auto_reupload` and `prompting` state lives on the UI side now — backend
// just reports saves and reuploads on demand.

use chrono::{DateTime, Utc};
use notify::{EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::sftp::SftpClient;
use crate::state::paths::dirs_home;
use crate::transport::env::short_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedFileInfo {
    pub remote_path: String,
    pub tmp_path: String,
    pub display_name: String,
    pub last_saved_mtime: DateTime<Utc>,
    pub last_saved_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditChangedEvent {
    pub server_key: String,
    pub remote_path: String,
    pub tmp_path: String,
    pub display_name: String,
    pub mtime: DateTime<Utc>,
    pub size: u64,
}

struct WatchedFile {
    remote_path: String,
    tmp_path: PathBuf,
    subdir: PathBuf,
    display_name: String,
    last_saved_mtime: SystemTime,
    last_saved_size: u64,
    debounce_token: u64,
    watcher: Option<notify::RecommendedWatcher>,
}

pub struct EditInPlaceManager {
    server_key: String,
    sftp: Arc<SftpClient>,
    app: AppHandle,
    tmp_root: PathBuf,
    watched: Arc<Mutex<HashMap<String, WatchedFile>>>,
}

fn invalid_filename_chars(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'..='\x1f')
}

fn now_stamp() -> String {
    use chrono::Local;
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

impl EditInPlaceManager {
    /// Containment guard for both begin_edit + save. Loads the profile fresh
    /// each call so user edits to ~/.rift/rift.json take effect immediately.
    /// Without this, the `remote_path` argument flows straight to SFTP and
    /// can step outside `profile.remote_root` (audit scan-transport row 7).
    fn guard_remote_path(&self, remote_path: &str) -> Result<(), String> {
        let cfg = crate::profile::RiftConfig::load()?;
        let prof = cfg
            .find(&self.server_key)
            .ok_or_else(|| format!("unknown server: {}", self.server_key))?;
        crate::path_guard::validate_remote_child(prof, remote_path).map(|_| ())
    }

    pub fn new(server_key: String, sftp: Arc<SftpClient>, app: AppHandle) -> Result<Self, String> {
        let home = dirs_home().map_err(|e| format!("home dir: {e}"))?;
        let tmp_root = home
            .join(".rift")
            .join("rift-edits")
            .join(format!("{}-{}", now_stamp(), short_id()));
        std::fs::create_dir_all(&tmp_root)
            .map_err(|e| format!("create tmp root {}: {e}", tmp_root.display()))?;
        Ok(Self {
            server_key,
            sftp,
            app,
            tmp_root,
            watched: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Start watching `remote_path`. Downloads to a per-file subdir under the
    /// session tmp root, registers a notify watcher, returns the local tmp path.
    /// If already watching, re-returns the existing tmp path (idempotent).
    pub async fn begin_edit(&self, remote_path: &str) -> Result<PathBuf, String> {
        if remote_path.is_empty() {
            return Err("empty remote_path".into());
        }
        self.guard_remote_path(remote_path)?;

        {
            let g = self.watched.lock().await;
            if let Some(w) = g.get(remote_path) {
                if w.tmp_path.is_file() {
                    return Ok(w.tmp_path.clone());
                }
            }
        }

        let last_sep = remote_path.rfind(['/', '\\']);
        let file_name = match last_sep {
            Some(i) => &remote_path[i + 1..],
            None => remote_path,
        };
        if file_name.is_empty() || file_name == "." || file_name == ".." {
            return Err(format!("invalid filename in remote_path '{remote_path}'"));
        }
        if file_name.contains(invalid_filename_chars) {
            return Err(format!("filename '{file_name}' has invalid chars"));
        }

        let subdir = self.tmp_root.join(short_id());
        std::fs::create_dir_all(&subdir)
            .map_err(|e| format!("create subdir {}: {e}", subdir.display()))?;
        let local_path = subdir.join(file_name);

        let res = self.sftp.download_file_atomic(remote_path, &local_path).await;
        if !res.success || !local_path.is_file() {
            let _ = std::fs::remove_dir_all(&subdir);
            return Err(format!("download for edit failed: {}", res.error));
        }

        let meta = std::fs::metadata(&local_path)
            .map_err(|e| format!("stat tmp: {e}"))?;
        let last_mtime = meta.modified().unwrap_or(SystemTime::now());
        let last_size = meta.len();

        // Spawn watcher. notify's debouncer-full would be cleaner, but inline
        // 400ms coalescing via debounce_token matches WPF's behavior 1:1.
        let watched_arc = self.watched.clone();
        let app = self.app.clone();
        let server_key_for_event = self.server_key.clone();
        let key = remote_path.to_string();
        let watch_target = local_path.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            if !matches!(
                ev.kind,
                EventKind::Modify(_) | EventKind::Create(_)
            ) {
                return;
            }
            if !ev.paths.iter().any(|p| p == &watch_target) {
                return;
            }
            let key = key.clone();
            let watched_arc = watched_arc.clone();
            let app = app.clone();
            let server_key_for_event = server_key_for_event.clone();
            tokio::spawn(async move {
                let token = {
                    let mut g = watched_arc.lock().await;
                    let Some(w) = g.get_mut(&key) else { return };
                    w.debounce_token = w.debounce_token.wrapping_add(1);
                    w.debounce_token
                };
                tokio::time::sleep(Duration::from_millis(400)).await;
                let event = {
                    let mut g = watched_arc.lock().await;
                    let Some(w) = g.get_mut(&key) else { return };
                    if w.debounce_token != token {
                        return;
                    }
                    let meta = match std::fs::metadata(&w.tmp_path) {
                        Ok(m) => m,
                        Err(_) => return,
                    };
                    let mtime = meta.modified().unwrap_or(SystemTime::now());
                    let size = meta.len();
                    if mtime <= w.last_saved_mtime && size == w.last_saved_size {
                        return;
                    }
                    w.last_saved_mtime = mtime;
                    w.last_saved_size = size;
                    EditChangedEvent {
                        server_key: server_key_for_event.clone(),
                        remote_path: w.remote_path.clone(),
                        tmp_path: w.tmp_path.to_string_lossy().to_string(),
                        display_name: w.display_name.clone(),
                        mtime: DateTime::<Utc>::from(mtime),
                        size,
                    }
                };
                let _ = app.emit("edit://changed", &event);
            });
        })
        .map_err(|e| format!("notify watcher init: {e}"))?;

        watcher
            .watch(&subdir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("notify watch {}: {e}", subdir.display()))?;

        let mut g = self.watched.lock().await;
        g.insert(
            remote_path.to_string(),
            WatchedFile {
                remote_path: remote_path.to_string(),
                tmp_path: local_path.clone(),
                subdir,
                display_name: file_name.to_string(),
                last_saved_mtime: last_mtime,
                last_saved_size: last_size,
                debounce_token: 0,
                watcher: Some(watcher),
            },
        );
        Ok(local_path)
    }

    /// Reupload the watched local file. Emits `edit://reuploaded` w/
    /// {remotePath, success, error} so the UI can refresh its drift baseline.
    pub async fn save(&self, remote_path: &str) -> Result<(), String> {
        self.guard_remote_path(remote_path)?;
        let (tmp_path, _display) = {
            let g = self.watched.lock().await;
            let w = g
                .get(remote_path)
                .ok_or_else(|| format!("not watching {remote_path}"))?;
            (w.tmp_path.clone(), w.display_name.clone())
        };
        let res = self.sftp.upload_file_atomic(&tmp_path, remote_path).await;

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Ev<'a> {
            remote_path: &'a str,
            success: bool,
            error: &'a str,
        }
        let _ = self.app.emit(
            "edit://reuploaded",
            &Ev { remote_path, success: res.success, error: &res.error },
        );

        if !res.success {
            return Err(res.error);
        }

        // Refresh baseline so the next save delta is honest.
        if let Ok(meta) = std::fs::metadata(&tmp_path) {
            let mut g = self.watched.lock().await;
            if let Some(w) = g.get_mut(remote_path) {
                w.last_saved_mtime = meta.modified().unwrap_or(SystemTime::now());
                w.last_saved_size = meta.len();
            }
        }
        Ok(())
    }

    /// Stop watching `remote_path` and delete its tmp subdir.
    pub async fn close(&self, remote_path: &str) -> Result<(), String> {
        let mut g = self.watched.lock().await;
        let Some(mut w) = g.remove(remote_path) else { return Ok(()) };
        // Drop watcher before removing files so notify doesn't fire on cleanup.
        w.watcher.take();
        let _ = std::fs::remove_dir_all(&w.subdir);
        Ok(())
    }

    pub async fn list_watched(&self) -> Vec<WatchedFileInfo> {
        let g = self.watched.lock().await;
        g.values()
            .map(|w| WatchedFileInfo {
                remote_path: w.remote_path.clone(),
                tmp_path: w.tmp_path.to_string_lossy().to_string(),
                display_name: w.display_name.clone(),
                last_saved_mtime: DateTime::<Utc>::from(w.last_saved_mtime),
                last_saved_size: w.last_saved_size,
            })
            .collect()
    }
}

impl Drop for EditInPlaceManager {
    fn drop(&mut self) {
        // Best-effort cleanup; can't await inside Drop. Detach the
        // remove_dir_all onto a fresh OS thread so a slow filesystem
        // (e.g. AV scanner holding handles) can't block the async runtime
        // worker that's executing this Drop. Outstanding watchers fall off
        // w/ the dropped map either way.
        let path = self.tmp_root.clone();
        std::thread::spawn(move || {
            let _ = std::fs::remove_dir_all(&path);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_filename_chars_match() {
        assert!(invalid_filename_chars(':'));
        assert!(invalid_filename_chars('\\'));
        assert!(invalid_filename_chars('?'));
        assert!(!invalid_filename_chars('a'));
        assert!(!invalid_filename_chars('-'));
    }

    #[test]
    fn short_id_is_16_hex() {
        let s = short_id();
        assert_eq!(s.len(), 16);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

}
