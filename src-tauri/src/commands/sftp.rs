//! SFTP + local-fs + edit-in-place command surface (#20).

use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::sync::auto_sync::{ActivityKind, ActivityRow};
use crate::{
    bootstrap, edit, local_fs, path_guard, profile, sftp, AutoSyncState, DownloadState,
    EditInPlaceState,
};
use super::{basename_for_log, reject_path_traversal, require_pinned_fingerprint};
use super::sync::validate_watched_local_path;

/// Browser-pane LocalEntry shape. Distinct from `local_fs::LocalEntry` because
/// the frontend pre-dates the canonical version and uses a flatter shape.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
}

#[tauri::command]
pub fn local_list_dir(path: String) -> Result<Vec<LocalEntry>, String> {
    let p = std::path::Path::new(&path);
    reject_path_traversal(p, "path")?;
    if !p.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    Ok(local_fs::list_directory(p)
        .into_iter()
        .map(|e| LocalEntry {
            name: e.name,
            path: e.full_path,
            is_dir: e.is_directory,
            size: e.size,
            mtime: e.last_modified.timestamp(),
        })
        .collect())
}

async fn open_sftp_for(server_key: &str) -> Result<sftp::SftpClient, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    open_sftp_for_server(&server).await
}

/// #112: variant taking a pre-loaded server so callers that already had to
/// load `RiftConfig` don't pay a second file read.
async fn open_sftp_for_server(server: &profile::ServerProfile) -> Result<sftp::SftpClient, String> {
    require_pinned_fingerprint(&server.key, server.fingerprint.as_deref())?;
    let key_path = std::path::PathBuf::from(&server.key_path);
    sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: server.fingerprint.as_deref(),
        write_probe_root: Some(&server.remote_root),
    })
    .await
}

#[tauri::command]
pub async fn remote_list_dir(
    server_key: String,
    path: String,
) -> Result<Vec<sftp::RemoteEntry>, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    let path = path_guard::validate_remote_listable(&server, &path)
        .map_err(|e| format!("remote list guard: {e}"))?;
    let client = open_sftp_for_server(&server).await?;
    let entries = client.list_directory(&path).await;
    client.close().await;
    entries
}

/// Expand a job list so directory inputs become recursive file jobs.
fn expand_upload_jobs(jobs: Vec<(String, String)>) -> Vec<(PathBuf, String)> {
    let mut expanded: Vec<(PathBuf, String)> = Vec::new();
    for (local, remote) in jobs {
        let local_path = PathBuf::from(&local);
        if local_path.is_dir() {
            let remote_root = remote.trim_end_matches('/').to_string();
            for entry in walkdir::WalkDir::new(&local_path).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let rel = match entry.path().strip_prefix(&local_path) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let rel_posix: String = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                if rel_posix.is_empty() {
                    continue;
                }
                let remote_target = format!("{}/{}", remote_root, rel_posix);
                expanded.push((entry.path().to_path_buf(), remote_target));
            }
        } else if local_path.is_file() {
            expanded.push((local_path, remote));
        }
    }
    expanded
}

#[tauri::command]
pub async fn upload_paths(
    app: tauri::AppHandle,
    server_key: String,
    jobs: Vec<(String, String)>,
) -> Result<Vec<bool>, String> {
    use tauri::Emitter;
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    for (local, remote) in &jobs {
        path_guard::validate_local_child(&server, local)
            .map_err(|e| format!("upload local guard: {e}"))?;
        path_guard::validate_remote_child(&server, remote)
            .map_err(|e| format!("upload remote guard: {e}"))?;
    }
    let client = open_sftp_for(&server_key).await?;
    let mapped = expand_upload_jobs(jobs);
    for (local, remote) in &mapped {
        let local_str = local.to_string_lossy().to_string();
        if let Err(e) = path_guard::validate_local_child(&server, &local_str) {
            client.close().await;
            return Err(format!("upload local guard (expanded): {e}"));
        }
        if let Err(e) = path_guard::validate_remote_child(&server, remote) {
            client.close().await;
            return Err(format!("upload remote guard (expanded): {e}"));
        }
    }
    if mapped.is_empty() {
        client.close().await;
        return Ok(vec![]);
    }
    let _ = app.emit("autosync://activity", &ActivityRow {
        at: chrono::Utc::now(),
        resource: "manual".to_string(),
        file: format!("{} files", mapped.len()),
        action: "upload started".to_string(),
        kind: ActivityKind::Sync,
        ..Default::default()
    });
    let result = client.upload_files_batch(&mapped, 4).await;
    client.close().await;
    let ok = result.iter().filter(|b| **b).count();
    let _ = app.emit("autosync://activity", &ActivityRow {
        at: chrono::Utc::now(),
        resource: "manual".to_string(),
        file: format!("{}/{} files", ok, result.len()),
        action: if ok == result.len() { "upload complete".to_string() } else { "upload partial".to_string() },
        kind: if ok == result.len() { ActivityKind::Sync } else { ActivityKind::Error },
        ..Default::default()
    });
    Ok(result)
}

async fn expand_download_jobs(
    client: &sftp::SftpClient,
    jobs: Vec<(String, String)>,
) -> Result<Vec<(String, PathBuf)>, String> {
    let mut expanded: Vec<(String, PathBuf)> = Vec::new();
    for (remote, local) in jobs {
        let info = client.remote_stat(&remote).await;
        if !info.exists {
            continue;
        }
        let local_path = PathBuf::from(&local);
        if info.is_directory {
            if let Err(e) = std::fs::create_dir_all(&local_path) {
                log::warn!("download mkdir {}: {e}", local_path.display());
            }
            let files = client
                .list_recursive(&remote, 32, None)
                .await
                .map_err(|e| format!("list_recursive `{remote}`: {e}"))?;
            let prefix = format!("{}/", remote.trim_end_matches('/'));
            for f in files {
                if f.is_dir {
                    continue;
                }
                let rel = f.full_path.strip_prefix(&prefix).unwrap_or(&f.full_path);
                let mut dest = local_path.clone();
                for part in rel.split('/') {
                    if !part.is_empty() {
                        dest.push(part);
                    }
                }
                if let Some(p) = dest.parent() {
                    if let Err(e) = std::fs::create_dir_all(p) {
                        log::warn!("download mkdir {}: {e}", p.display());
                    }
                }
                expanded.push((f.full_path.clone(), dest));
            }
        } else {
            if let Some(p) = local_path.parent() {
                if let Err(e) = std::fs::create_dir_all(p) {
                    log::warn!("download mkdir {}: {e}", p.display());
                }
            }
            expanded.push((remote, local_path));
        }
    }
    Ok(expanded)
}

#[tauri::command]
pub async fn download_paths(
    app: tauri::AppHandle,
    server_key: String,
    jobs: Vec<(String, String)>,
    dl_state: tauri::State<'_, DownloadState>,
) -> Result<Vec<bool>, String> {
    use tauri::Emitter;
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    for (remote, local) in &jobs {
        path_guard::validate_remote_child(&server, remote)
            .map_err(|e| format!("download remote guard: {e}"))?;
        path_guard::validate_local_child(&server, local)
            .map_err(|e| format!("download local guard: {e}"))?;
    }
    let client = open_sftp_for(&server_key).await?;
    let ct = CancellationToken::new();
    {
        let mut g = dl_state.0.lock().await;
        *g = Some(ct.clone());
    }
    let _ = app.emit("autosync://activity", &ActivityRow {
        at: chrono::Utc::now(),
        resource: "manual".to_string(),
        file: "expanding job list".to_string(),
        action: "download started".to_string(),
        kind: ActivityKind::Pull,
        ..Default::default()
    });
    let mapped = match expand_download_jobs(&client, jobs).await {
        Ok(m) => m,
        Err(e) => {
            client.close().await;
            let mut g = dl_state.0.lock().await;
            *g = None;
            let _ = app.emit("autosync://activity", &ActivityRow {
                at: chrono::Utc::now(),
                resource: "manual".to_string(),
                file: "expand failed".to_string(),
                action: format!("download aborted: {e}"),
                kind: ActivityKind::Error,
                ..Default::default()
            });
            return Err(e);
        }
    };
    for (remote, local) in &mapped {
        let remote_ok = path_guard::validate_remote_child(&server, remote);
        let local_str = local.to_string_lossy().to_string();
        let local_ok = remote_ok
            .as_ref()
            .map_err(|e| e.clone())
            .and_then(|_| path_guard::validate_local_child(&server, &local_str).map(|_| ()));
        if let Err(e) = local_ok {
            client.close().await;
            let mut g = dl_state.0.lock().await;
            *g = None;
            return Err(format!("download guard (expanded): {e}"));
        }
    }
    if mapped.is_empty() {
        client.close().await;
        let _ = app.emit("autosync://activity", &ActivityRow {
            at: chrono::Utc::now(),
            resource: "manual".to_string(),
            file: "0 files".to_string(),
            action: "download empty".to_string(),
            kind: ActivityKind::System,
            ..Default::default()
        });
        return Ok(vec![]);
    }
    let _ = app.emit("autosync://activity", &ActivityRow {
        at: chrono::Utc::now(),
        resource: "manual".to_string(),
        file: format!("{} files", mapped.len()),
        action: "downloading".to_string(),
        kind: ActivityKind::Pull,
        ..Default::default()
    });
    let result = tokio::select! {
        r = client.download_files_batch(&mapped, 4, ct.clone()) => r,
        _ = ct.cancelled() => {
            client.close().await;
            let _ = app.emit("autosync://activity", &ActivityRow {
                at: chrono::Utc::now(),
                resource: "manual".to_string(),
                file: format!("{} files", mapped.len()),
                action: "download cancelled".to_string(),
                kind: ActivityKind::Error,
                ..Default::default()
            });
            return Ok(vec![false; mapped.len()]);
        }
    };
    client.close().await;
    {
        let mut g = dl_state.0.lock().await;
        *g = None;
    }
    let ok = result.iter().filter(|b| **b).count();
    let _ = app.emit("autosync://activity", &ActivityRow {
        at: chrono::Utc::now(),
        resource: "manual".to_string(),
        file: format!("{}/{} files", ok, result.len()),
        action: if ok == result.len() { "download complete".to_string() } else { "download partial".to_string() },
        kind: if ok == result.len() { ActivityKind::Pull } else { ActivityKind::Error },
        ..Default::default()
    });
    Ok(result)
}

#[tauri::command]
pub async fn cancel_download(dl_state: tauri::State<'_, DownloadState>) -> Result<(), String> {
    let g = dl_state.0.lock().await;
    if let Some(ct) = g.as_ref() {
        ct.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn remote_rename_path(
    app: tauri::AppHandle,
    server_key: String,
    from: String,
    to: String,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<(), String> {
    use tauri::Emitter;
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    let from = path_guard::validate_remote_child(&server, &from)
        .map_err(|e| format!("remote rename from guard: {e}"))?;
    let to = path_guard::validate_remote_child(&server, &to)
        .map_err(|e| format!("remote rename to guard: {e}"))?;
    if let Some(engine) = { state.0.lock().await.clone() } {
        if let Some(locks) = engine.locks() {
            if let Some(lock) = locks.find_lock_by_other(&from) {
                return Err(format!("remote rename blocked by {}@{}", lock.user, lock.host));
            }
        }
    }
    let client = open_sftp_for(&server_key).await?;
    let result = client.rename(&from, &to).await;
    client.close().await;
    let row = match &result {
        Ok(()) => ActivityRow {
            at: chrono::Utc::now(),
            resource: "manual".to_string(),
            file: format!("{} → {}", basename_for_log(&from), basename_for_log(&to)),
            action: "remote rename".to_string(),
            kind: ActivityKind::Sync,
            ..Default::default()
        },
        Err(e) => ActivityRow {
            at: chrono::Utc::now(),
            resource: "manual".to_string(),
            file: basename_for_log(&from),
            action: format!("remote rename failed: {e}"),
            kind: ActivityKind::Error,
            ..Default::default()
        },
    };
    let _ = app.emit("autosync://activity", &row);
    result
}

#[tauri::command]
pub async fn remote_delete_paths(
    app: tauri::AppHandle,
    server_key: String,
    paths: Vec<String>,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Vec<path_guard::OpStatus>, String> {
    use tauri::Emitter;
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    let engine = { state.0.lock().await.clone() };
    let client = open_sftp_for(&server_key).await?;
    let mut out = Vec::with_capacity(paths.len());
    for p in &paths {
        let guarded = path_guard::validate_remote_child(&server, p)
            .map_err(|e| format!("remote delete guard: {e}"));
        let res = match guarded {
            Ok(remote) => {
                if let Some(engine) = engine.as_ref() {
                    if let Some(locks) = engine.locks() {
                        if let Some(lock) = locks.find_lock_by_other(&remote) {
                            Err(format!("blocked by {}@{}", lock.user, lock.host))
                        } else {
                            client.delete_recursive(&remote).await
                        }
                    } else {
                        client.delete_recursive(&remote).await
                    }
                } else {
                    client.delete_recursive(&remote).await
                }
            }
            Err(e) => Err(e),
        };
        let ok = res.is_ok();
        let err = res.err();
        let row = if ok {
            ActivityRow {
                at: chrono::Utc::now(),
                resource: "manual".to_string(),
                file: basename_for_log(p),
                action: "remote delete".to_string(),
                kind: ActivityKind::Delete,
                ..Default::default()
            }
        } else {
            ActivityRow {
                at: chrono::Utc::now(),
                resource: "manual".to_string(),
                file: basename_for_log(p),
                action: format!("remote delete failed: {}", err.clone().unwrap_or_default()),
                kind: ActivityKind::Error,
                ..Default::default()
            }
        };
        let _ = app.emit("autosync://activity", &row);
        out.push(if ok {
            path_guard::OpStatus::ok()
        } else {
            path_guard::OpStatus::err(err.unwrap_or_else(|| "remote delete failed".into()))
        });
    }
    client.close().await;
    Ok(out)
}

#[tauri::command]
pub async fn local_rename_path(
    app: tauri::AppHandle,
    from: String,
    to: String,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<(), String> {
    use tauri::Emitter;
    let result = async {
        let from_p = validate_watched_local_path(&state, &from, "from").await?;
        let to_p = validate_watched_local_path(&state, &to, "to").await?;
        if to_p.exists() {
            return Err(format!("target already exists: {}", to_p.display()));
        }
        std::fs::rename(&from_p, &to_p)
            .map_err(|e| format!("rename {} -> {}: {e}", from_p.display(), to_p.display()))
    }
    .await;
    let row = match &result {
        Ok(()) => ActivityRow {
            at: chrono::Utc::now(),
            resource: "manual".to_string(),
            file: format!("{} → {}", basename_for_log(&from), basename_for_log(&to)),
            action: "local rename".to_string(),
            kind: ActivityKind::Sync,
            ..Default::default()
        },
        Err(e) => ActivityRow {
            at: chrono::Utc::now(),
            resource: "manual".to_string(),
            file: basename_for_log(&from),
            action: format!("local rename failed: {e}"),
            kind: ActivityKind::Error,
            ..Default::default()
        },
    };
    let _ = app.emit("autosync://activity", &row);
    result
}

#[tauri::command]
pub async fn local_delete_paths(
    app: tauri::AppHandle,
    paths: Vec<String>,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Vec<path_guard::OpStatus>, String> {
    use tauri::Emitter;
    let mut out = Vec::with_capacity(paths.len());
    for p in &paths {
        let guarded = validate_watched_local_path(&state, p, "path").await;
        let path = match guarded {
            Ok(path) => path,
            Err(e) => {
                let _ = app.emit("autosync://activity", &ActivityRow {
                    at: chrono::Utc::now(),
                    resource: "manual".to_string(),
                    file: basename_for_log(p),
                    action: format!("local delete blocked: {e}"),
                    kind: ActivityKind::Block,
                    ..Default::default()
                });
                out.push(path_guard::OpStatus::err(e));
                continue;
            }
        };
        let res = if path.symlink_metadata().map(|m| m.is_dir()).unwrap_or(false) {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        let ok = res.is_ok();
        let err = res.err().map(|e| e.to_string());
        let row = if ok {
            ActivityRow {
                at: chrono::Utc::now(),
                resource: "manual".to_string(),
                file: basename_for_log(p),
                action: "local delete".to_string(),
                kind: ActivityKind::Delete,
                ..Default::default()
            }
        } else {
            ActivityRow {
                at: chrono::Utc::now(),
                resource: "manual".to_string(),
                file: basename_for_log(p),
                action: format!("local delete failed: {}", err.clone().unwrap_or_default()),
                kind: ActivityKind::Error,
                ..Default::default()
            }
        };
        let _ = app.emit("autosync://activity", &row);
        out.push(if ok {
            path_guard::OpStatus::ok()
        } else {
            path_guard::OpStatus::err(err.unwrap_or_else(|| "local delete failed".into()))
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn detect_bootstrap(
    server_key: String,
) -> Result<bootstrap::BootstrapDetection, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    let client = open_sftp_for(&server_key).await?;
    let res = bootstrap::detect(&client, &server.remote_root, &server.local_root).await;
    client.close().await;
    res
}

#[tauri::command]
pub async fn bootstrap_list_files(
    server_key: String,
) -> Result<Vec<(String, String)>, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    let client = open_sftp_for(&server_key).await?;
    let remote_root = server.remote_root.trim_end_matches('/').to_string();
    let entries = client
        .list_recursive(&remote_root, 8, None)
        .await
        .map_err(|e| format!("list recursive: {e}"))?;
    client.close().await;

    let local_root_path = std::path::PathBuf::from(&server.local_root);
    let mut jobs = Vec::with_capacity(entries.len());
    for e in entries {
        if e.is_dir { continue; }
        if e.full_path.contains("/[disabled]/") { continue; }
        if e.full_path.len() < remote_root.len() { continue; }
        let (head, tail) = e.full_path.split_at(remote_root.len());
        if !head.eq_ignore_ascii_case(&remote_root) { continue; }
        let original_rel = tail.trim_start_matches('/');
        if original_rel.is_empty() { continue; }
        let local = local_root_path.join(original_rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        jobs.push((e.full_path, local.to_string_lossy().to_string()));
    }
    Ok(jobs)
}

async fn editor_for(
    server_key: &str,
    state: &EditInPlaceState,
    app: &tauri::AppHandle,
) -> Result<Arc<edit::in_place::EditInPlaceManager>, String> {
    {
        let g = state.0.lock().await;
        if let Some(m) = g.get(server_key) {
            return Ok(m.clone());
        }
    }
    let client = open_sftp_for(server_key).await?;
    let sftp_arc = Arc::new(client);
    let mgr = Arc::new(edit::in_place::EditInPlaceManager::new(server_key.to_string(), sftp_arc.clone(), app.clone())?);
    let mut g = state.0.lock().await;
    if let Some(existing) = g.get(server_key) {
        log::warn!(
            "editor_for: race lost on '{server_key}' — discarding just-opened SFTP handle (another task initialized first)"
        );
        let losing_sftp = sftp_arc.clone();
        drop(mgr);
        tokio::spawn(async move {
            if let Some(owned) = Arc::try_unwrap(losing_sftp).ok() {
                owned.close().await;
            }
        });
        return Ok(existing.clone());
    }
    g.insert(server_key.to_string(), mgr.clone());
    Ok(mgr)
}

#[tauri::command]
pub async fn begin_edit_in_place(
    server_key: String,
    remote_path: String,
    state: tauri::State<'_, EditInPlaceState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let mgr = editor_for(&server_key, &state, &app).await?;
    let local = mgr.begin_edit(&remote_path).await?;
    Ok(local.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn save_edit_in_place(
    server_key: String,
    remote_path: String,
    state: tauri::State<'_, EditInPlaceState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mgr = editor_for(&server_key, &state, &app).await?;
    mgr.save(&remote_path).await
}

#[tauri::command]
pub async fn close_edit_in_place(
    server_key: String,
    remote_path: String,
    state: tauri::State<'_, EditInPlaceState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mgr = editor_for(&server_key, &state, &app).await?;
    mgr.close(&remote_path).await
}

#[tauri::command]
pub async fn list_watched_edits(
    state: tauri::State<'_, EditInPlaceState>,
) -> Result<Vec<edit::in_place::WatchedFileInfo>, String> {
    let g = state.0.lock().await;
    let mut all = Vec::new();
    for mgr in g.values() {
        let mut v = mgr.list_watched().await;
        all.append(&mut v);
    }
    Ok(all)
}
