//! Sync engine + diagnostics command surface (#20).

use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::sync::{self, AutoSyncEngine, AutoSyncStatus, ConflictResolution, FolderSpec};
use crate::sync::auto_sync::{ActivityKind, ActivityRow};
use crate::{
    bridge, diagnostics, profile, sftp, state, tunnel, AutoSyncState, TunnelState,
};
use super::{basename_for_log, reject_path_traversal, require_pinned_fingerprint};

// ─── Diagnostics (Sync Inspector) ────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagStateDto {
    at: String,
    autosync_state: Option<sync::AutoSyncState>,
    autosync_detail: String,
    watcher_count: usize,
    queue_pending: usize,
    queue_failed: usize,
    queue_dropped_total: u64,
    /// #45: FS-event drops on the watcher → tokio channel (per-engine), distinct
    /// from `queue_dropped_total` which counts diag-bus broadcaster overflow.
    fs_events_dropped: u64,
    ignored_total: u64,
    conflicts: usize,
    last_drift_scan_at: Option<String>,
    last_rescan_signal_at: Option<String>,
    bus_lag_total: u64,
    events_emitted_total: u64,
}

/// #108: shared DTO builder used by both `diag_get_state` (Tauri command) and
/// `diag_state_pump` (500ms emitter).
async fn collect_diag_dto(engine: Option<Arc<sync::AutoSyncEngine>>) -> DiagStateDto {
    let (autosync_state, autosync_detail, watches, pending, failed, ignored_total, conflicts, fs_dropped) =
        if let Some(engine) = engine.as_ref() {
            let s = engine.status().await;
            (
                Some(s.state),
                s.detail,
                s.watches,
                s.pending,
                s.failed,
                s.ignored_total,
                s.conflicts,
                s.dropped_events,
            )
        } else {
            (None, String::new(), 0, 0, 0, 0, 0, 0)
        };
    let bus = diagnostics::bus();
    DiagStateDto {
        at: chrono::Utc::now().to_rfc3339(),
        autosync_state,
        autosync_detail,
        watcher_count: watches,
        queue_pending: pending,
        queue_failed: failed,
        queue_dropped_total: bus.queue_dropped_total(),
        fs_events_dropped: fs_dropped,
        ignored_total,
        conflicts,
        last_drift_scan_at: bus.last_drift_scan_at().map(|d| d.to_rfc3339()),
        last_rescan_signal_at: bus.last_rescan_signal_at().map(|d| d.to_rfc3339()),
        bus_lag_total: bus.bus_lag_total(),
        events_emitted_total: bus.events_emitted_total(),
    }
}

#[tauri::command]
pub async fn diag_get_state(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<DiagStateDto, String> {
    let engine = { state.0.lock().await.clone() };
    Ok(collect_diag_dto(engine).await)
}

#[tauri::command]
pub fn diag_snapshot_path(server_key: String) -> Result<String, String> {
    state::paths::cache_path("snapshot", &server_key)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("snapshot path: {e}"))
}

#[tauri::command]
pub async fn sync_reconcile(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<bool, String> {
    log::debug!("sync_reconcile cmd: entry");
    let g = state.0.lock().await;
    log::debug!("sync_reconcile cmd: lock acquired");
    let Some(engine) = g.as_ref() else {
        log::debug!("sync_reconcile cmd: no engine bound");
        return Ok(false);
    };
    engine.kick_drift_reconcile();
    Ok(true)
}

#[tauri::command]
pub async fn sync_cancel(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<bool, String> {
    let g = state.0.lock().await;
    let Some(engine) = g.as_ref() else { return Ok(false) };
    engine.cancel_drift_reconcile();
    Ok(true)
}

#[tauri::command]
pub async fn sync_pull_pending(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<bool, String> {
    log::debug!("sync_pull_pending cmd: entry");
    let g = state.0.lock().await;
    log::debug!("sync_pull_pending cmd: lock acquired");
    let Some(engine) = g.as_ref() else {
        log::debug!("sync_pull_pending cmd: no engine bound");
        return Ok(false);
    };
    engine.force_pull_now();
    Ok(true)
}

#[tauri::command]
pub async fn sync_push_pending(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<bool, String> {
    log::debug!("sync_push_pending cmd: entry");
    let g = state.0.lock().await;
    log::debug!("sync_push_pending cmd: lock acquired");
    let Some(engine) = g.as_ref() else {
        log::debug!("sync_push_pending cmd: no engine bound — returning false");
        return Ok(false);
    };
    log::debug!("sync_push_pending cmd: calling force_push_now");
    engine.force_push_now();
    log::debug!("sync_push_pending cmd: returning Ok(true)");
    Ok(true)
}

#[tauri::command]
pub async fn sync_get_drift_snapshot(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Vec<crate::sync::DriftEntry>, String> {
    let g = state.0.lock().await;
    Ok(match g.as_ref() {
        Some(engine) => engine.drift_snapshot(),
        None => Vec::new(),
    })
}

#[derive(serde::Serialize)]
pub struct WatchedFolderInfo {
    pub name: String,
    pub remote_root: String,
    pub file_count: u64,
}

#[tauri::command]
pub async fn list_watched_folders(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Vec<WatchedFolderInfo>, String> {
    let g = state.0.lock().await;
    Ok(match g.as_ref() {
        Some(engine) => engine
            .watched_folders_dashboard()
            .into_iter()
            .map(|(name, remote_root, file_count)| WatchedFolderInfo {
                name,
                remote_root,
                file_count,
            })
            .collect(),
        None => Vec::new(),
    })
}

#[tauri::command]
pub async fn sync_get_aborted_shrunk(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Vec<crate::sync::AbortedShrunkFolder>, String> {
    let g = state.0.lock().await;
    Ok(match g.as_ref() {
        Some(engine) => engine.aborted_shrunk(),
        None => Vec::new(),
    })
}

#[tauri::command]
pub async fn sync_rebaseline_folder(
    state: tauri::State<'_, AutoSyncState>,
    remote_subpath: String,
) -> Result<(usize, usize, usize), String> {
    let g = state.0.lock().await;
    let Some(engine) = g.as_ref() else {
        return Err("not connected".into());
    };
    engine.rebaseline_folder(&remote_subpath).await
}

#[tauri::command]
pub async fn sync_apply_selected(
    state: tauri::State<'_, AutoSyncState>,
    local_paths: Vec<String>,
) -> Result<bool, String> {
    let g = state.0.lock().await;
    let Some(engine) = g.as_ref() else {
        return Ok(false);
    };
    engine.apply_selected(local_paths);
    Ok(true)
}

#[tauri::command]
pub async fn sync_set_mirror_mode(
    state: tauri::State<'_, AutoSyncState>,
    enabled: bool,
) -> Result<bool, String> {
    let engine = { state.0.lock().await.clone() };
    let Some(engine) = engine else {
        return Err("not connected".into());
    };
    Ok(engine.set_mirror_mode(enabled))
}

#[tauri::command]
pub async fn sync_get_mirror_mode(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<bool, String> {
    let engine = { state.0.lock().await.clone() };
    Ok(engine.map(|e| e.mirror_mode_enabled()).unwrap_or(false))
}

#[tauri::command]
pub async fn sync_sweep_stale_locks(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<usize, String> {
    let engine = { state.0.lock().await.clone() };
    let Some(engine) = engine else {
        return Err("not connected".into());
    };
    engine.sweep_stale_locks().await
}

#[tauri::command]
pub async fn diag_ignored_breakdown(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<std::collections::HashMap<String, u64>, String> {
    let g = state.0.lock().await;
    Ok(match g.as_ref() {
        Some(engine) => engine.ignored_by_rule_snapshot(),
        None => std::collections::HashMap::new(),
    })
}

/// Periodic pipeline-state snapshot emitter. 500ms cadence. #106 cancel-aware.
pub async fn diag_state_pump(app: tauri::AppHandle, cancel: CancellationToken) {
    use tauri::Emitter;
    use tauri::Manager;

    let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                log::debug!("diag_state_pump: cancellation received, exiting");
                return;
            }
            _ = tick.tick() => {}
        }
        let st = app.state::<AutoSyncState>();
        let engine = { st.0.lock().await.clone() };
        let dto = collect_diag_dto(engine).await;
        let _ = app.emit("diag://state", &dto);
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ScanFolderInput {
    pub resource_name: String,
    pub remote_subpath: String,
}

#[tauri::command]
pub async fn scan_drift(
    server_key: String,
    folders: Vec<ScanFolderInput>,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<sync::ScanResult, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();

    require_pinned_fingerprint(&server_key, server.fingerprint.as_deref())?;

    let targets: Vec<sync::FolderTarget> = folders
        .into_iter()
        .map(|f| {
            let remote_root = format!(
                "{}/{}",
                server.remote_root.trim_end_matches('/'),
                f.remote_subpath.trim_start_matches('/')
            );
            let local_root = std::path::Path::new(&server.local_root)
                .join(f.remote_subpath.replace('/', std::path::MAIN_SEPARATOR_STR))
                .to_string_lossy()
                .to_string();
            sync::FolderTarget {
                resource_name: f.resource_name,
                local_root,
                remote_root,
            }
        })
        .collect();

    let engine_match = {
        let g = state.0.lock().await;
        g.as_ref()
            .filter(|e| e.profile_key() == server_key.as_str())
            .cloned()
    };
    if let Some(engine) = engine_match {
        let sftp = engine.sftp();
        let snap = engine.snapshot();
        let scanner = sync::DriftScanner::new(&sftp, Some(&snap));
        return Ok(scanner.scan(&targets).await);
    }

    let key_path = std::path::PathBuf::from(&server.key_path);
    let client = sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: server.fingerprint.as_deref(),
        write_probe_root: Some(&server.remote_root),
    })
    .await?;

    let snap = match state::SyncSnapshot::new(&server.key) {
        Ok(s) => s,
        Err(e) => {
            client.close().await;
            return Err(format!("snapshot init: {e}"));
        }
    };
    let scanner = sync::DriftScanner::new(&client, Some(&snap));
    let result = scanner.scan(&targets).await;
    client.close().await;
    Ok(result)
}

#[tauri::command]
pub async fn start_autosync(
    server_key: String,
    folders: Vec<FolderSpec>,
    state: tauri::State<'_, AutoSyncState>,
    tunnel_state: tauri::State<'_, TunnelState>,
    app: tauri::AppHandle,
) -> Result<AutoSyncStatus, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();

    require_pinned_fingerprint(&server_key, server.fingerprint.as_deref())?;

    let key_path = std::path::PathBuf::from(&server.key_path);
    let client = sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: server.fingerprint.as_deref(),
        write_probe_root: Some(&server.remote_root),
    })
    .await?;
    let sftp_arc = Arc::new(client);

    let lp = sync::LockPresence::new(sftp_arc.clone(), server.remote_root.clone(), app.clone());
    lp.start().await;

    let bridge_token_kc = profile::server_bridge_token(&server.key);
    let active_tunnel = match (server.bridge_port, bridge_token_kc.as_deref()) {
        (Some(rport), Some(token)) if !token.is_empty() => {
            match tunnel::SshTunnel::start(tunnel::TunnelArgs {
                host: &server.host,
                port: server.port,
                user: &server.user,
                key_path: &key_path,
                trusted_fingerprint: server.fingerprint.as_deref(),
                remote_host: "127.0.0.1",
                remote_port: rport,
            })
            .await
            {
                Ok(t) => Some((t, token.to_string())),
                Err(e) => {
                    log::warn!("ssh tunnel start failed: {e}");
                    None
                }
            }
        }
        _ => None,
    };

    let bridge = match &active_tunnel {
        Some((t, token)) => match bridge::BridgeClient::local(t.local_port, token) {
            Ok(c) => Some(Arc::new(c)),
            Err(e) => {
                log::warn!("bridge client init failed: {e}");
                None
            }
        },
        None => None,
    };

    let tunnel_handle = active_tunnel.map(|(t, _)| t);
    let engine = match AutoSyncEngine::start_with(
        sftp_arc,
        server,
        app,
        Some(lp.clone()),
        bridge,
    )
    .await
    {
        Ok(e) => e,
        Err(err) => {
            lp.stop().await;
            if let Some(t) = tunnel_handle {
                t.stop().await;
            }
            return Err(err);
        }
    };

    for spec in folders {
        if let Err(err) = engine.try_watch(spec).await {
            engine.stop().await;
            if let Some(t) = tunnel_handle {
                t.stop().await;
            }
            return Err(err);
        }
    }
    let engine_for_lp = engine.clone();
    lp.set_scoped_provider(std::sync::Arc::new(move || engine_for_lp.watched_remote_roots()))
        .await;

    let mut g = state.0.lock().await;
    if let Some(prev) = g.take() {
        prev.stop().await;
    }
    *g = Some(engine.clone());
    drop(g);

    let mut tg = tunnel_state.0.lock().await;
    if let Some(prev) = tg.take() {
        prev.stop().await;
    }
    *tg = tunnel_handle;
    drop(tg);

    // #107: sample status only after the prev engine is stopped + slots replaced,
    // so the returned snapshot reflects the new engine's post-install state.
    let status = engine.status().await;
    Ok(status)
}

#[tauri::command]
pub async fn stop_autosync(
    state: tauri::State<'_, AutoSyncState>,
    tunnel_state: tauri::State<'_, TunnelState>,
) -> Result<(), String> {
    let mut g = state.0.lock().await;
    if let Some(engine) = g.take() {
        engine.stop().await;
    }
    drop(g);
    let mut tg = tunnel_state.0.lock().await;
    if let Some(t) = tg.take() {
        t.stop().await;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_autosync_status(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Option<AutoSyncStatus>, String> {
    let engine = { state.0.lock().await.clone() };
    if let Some(engine) = engine.as_ref() {
        Ok(Some(engine.status().await))
    } else {
        Ok(None)
    }
}

/// #55: shared canonicalize + ownership check.
fn canonicalize_owned_path(
    engine: &sync::AutoSyncEngine,
    raw: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let p = PathBuf::from(raw);
    reject_path_traversal(&p, label)?;
    let canon = if p.exists() {
        p.canonicalize()
            .map_err(|e| format!("canonicalize {label} '{}': {e}", p.display()))?
    } else {
        let parent = p
            .parent()
            .ok_or_else(|| format!("{label}: '{}' has no parent", p.display()))?;
        let name = p
            .file_name()
            .ok_or_else(|| format!("{label}: '{}' has no filename", p.display()))?;
        let parent = parent
            .canonicalize()
            .map_err(|e| format!("canonicalize {label} parent '{}': {e}", parent.display()))?;
        parent.join(name)
    };
    if canon.parent().is_none() {
        return Err(format!("{label}: refusing filesystem root '{}'", canon.display()));
    }
    if !engine.owns_local_path(&canon) {
        return Err(format!("{label}: '{}' is outside watched roots", canon.display()));
    }
    Ok(canon)
}

pub(crate) async fn validate_watched_local_path(
    state: &tauri::State<'_, AutoSyncState>,
    path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    reject_path_traversal(&p, label)?;
    let canon = if p.exists() {
        p.canonicalize()
            .map_err(|e| format!("canonicalize {label} '{}': {e}", p.display()))?
    } else {
        let parent = p
            .parent()
            .ok_or_else(|| format!("{label}: '{}' has no parent", p.display()))?;
        let name = p
            .file_name()
            .ok_or_else(|| format!("{label}: '{}' has no filename", p.display()))?;
        let parent = parent
            .canonicalize()
            .map_err(|e| format!("canonicalize {label} parent '{}': {e}", parent.display()))?;
        parent.join(name)
    };
    if canon.parent().is_none() {
        return Err(format!("{label}: refusing filesystem root '{}'", canon.display()));
    }
    let engine = { state.0.lock().await.clone() };
    let Some(engine) = engine.as_ref() else {
        return Err(format!("{label}: local operation requires an active watch"));
    };
    if !engine.owns_local_path(&canon) {
        return Err(format!("{label}: '{}' is outside watched roots", canon.display()));
    }
    Ok(canon)
}

#[tauri::command]
pub async fn enqueue_for_flush_batch(
    paths: Vec<String>,
    deleted: bool,
    bypass_preflight: bool,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<u32, String> {
    let bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    for p in &bufs {
        reject_path_traversal(p, "enqueue path")?;
    }
    let engine = { state.0.lock().await.clone() };
    let Some(engine) = engine.as_ref() else { return Ok(0) };
    for p in &bufs {
        if !engine.owns_local_path(p) {
            return Err(format!("enqueue path outside watched roots: {}", p.display()));
        }
    }
    Ok(engine.enqueue_for_flush_batch(bufs, deleted, bypass_preflight))
}

#[tauri::command]
pub async fn resolve_conflict(
    local_path: String,
    resolution: ConflictResolution,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<(), String> {
    let buf = PathBuf::from(local_path);
    reject_path_traversal(&buf, "local_path")?;
    let engine = { state.0.lock().await.clone() };
    if let Some(engine) = engine.as_ref() {
        if !engine.owns_local_path(&buf) {
            return Err(format!("local_path outside watched roots: {}", buf.display()));
        }
        engine.resolve_conflict(&buf, resolution).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn resolve_conflicts_bulk(
    app: tauri::AppHandle,
    local_paths: Vec<String>,
    resolution: ConflictResolution,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Vec<bool>, String> {
    use tauri::Emitter;
    let engine = { state.0.lock().await.clone() };
    let engine = match engine.as_ref() {
        Some(e) => e,
        None => return Err("autosync not running".to_string()),
    };
    let mut out = Vec::with_capacity(local_paths.len());
    for p in &local_paths {
        let canon = match canonicalize_owned_path(engine, p, "local_path") {
            Ok(c) => c,
            Err(msg) => {
                let _ = app.emit("autosync://activity", &ActivityRow {
                    at: chrono::Utc::now(),
                    resource: "bulk".to_string(),
                    file: basename_for_log(p),
                    action: format!("conflict resolve blocked: {msg}"),
                    kind: ActivityKind::Block,
                    ..Default::default()
                });
                out.push(false);
                continue;
            }
        };
        let res = engine.resolve_conflict(&canon, resolution).await;
        let ok = res.is_ok();
        let row = if ok {
            ActivityRow {
                at: chrono::Utc::now(),
                resource: "bulk".to_string(),
                file: basename_for_log(p),
                action: format!("conflict resolved as {:?}", resolution).to_lowercase(),
                kind: ActivityKind::ConflictResolved,
                ..Default::default()
            }
        } else {
            ActivityRow {
                at: chrono::Utc::now(),
                resource: "bulk".to_string(),
                file: basename_for_log(p),
                action: format!("conflict resolve failed: {}", res.err().unwrap_or_default()),
                kind: ActivityKind::Error,
                ..Default::default()
            }
        };
        let _ = app.emit("autosync://activity", &row);
        out.push(ok);
    }
    Ok(out)
}

#[tauri::command]
pub async fn retry_failed(state: tauri::State<'_, AutoSyncState>) -> Result<(), String> {
    let g = state.0.lock().await;
    if let Some(engine) = g.as_ref() {
        engine.retry_failed().await;
    }
    Ok(())
}
