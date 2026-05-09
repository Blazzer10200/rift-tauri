//! Rift v14 — Tauri + Svelte + russh backend.
//!
//! Velopack-Rust auto-update wired at `run()` — banner fires when a newer
//! version is released to `Blazzer10200/rift-tauri`. The Tauri command surface
//! lives at the bottom of this file (`run()`'s `invoke_handler!`) and is the
//! contract with the Svelte frontend.

pub mod bootstrap;
pub mod bridge;
pub mod edit;
pub mod local_fs;
pub mod profile;
pub mod sftp;
pub mod state;
pub mod sync;
pub mod transport;
pub mod tunnel;
pub mod update_service;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use crate::sync::{AutoSyncEngine, AutoSyncStatus, ConflictResolution, FolderSpec};

/// Tauri-managed handle to the active AutoSync engine. None until start_autosync.
pub struct AutoSyncState(pub AsyncMutex<Option<Arc<AutoSyncEngine>>>);

/// Tauri-managed handle to the active SSH local-port-forward (Phase 1g). Some
/// only when a profile w/ `bridge_port` is active. Lifecycle bound to AutoSync:
/// started before BridgeClient construction, stopped in stop_autosync.
pub struct TunnelState(pub AsyncMutex<Option<tunnel::SshTunnel>>);

/// Phase 4: per-server EditInPlaceManager. Keyed by server_key — each server
/// gets its own SftpClient + watcher + tmp root. Lazy-init on first begin_edit
/// (lock held across init in [`editor_for`] to prevent connection leaks under
/// concurrent open).
pub struct EditInPlaceState(
    pub AsyncMutex<std::collections::HashMap<String, Arc<edit::in_place::EditInPlaceManager>>>,
);

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Debug, serde::Deserialize)]
pub struct ScanFolderInput {
    pub resource_name: String,
    pub remote_subpath: String, // relative to server.remote_root
}

#[tauri::command]
async fn scan_drift(
    server_key: String,
    folders: Vec<ScanFolderInput>,
) -> Result<sync::ScanResult, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();

    let key_path = std::path::PathBuf::from(&server.key_path);
    let client = sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: server.fingerprint.as_deref(),
    })
    .await?;
    if server.fingerprint.as_deref().unwrap_or("").is_empty() {
        persist_fingerprint_if_new(&server_key, client.fingerprint());
    }

    let snap = state::SyncSnapshot::new(&server.key)
        .map_err(|e| format!("snapshot init: {e}"))?;
    let scanner = sync::DriftScanner::new(&client, Some(&snap));

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

    let result = scanner.scan(&targets).await;
    client.close().await;
    Ok(result)
}

// ─── AutoSync (Phase 1c) Tauri commands ──────────────────────────────────────

#[tauri::command]
async fn start_autosync(
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

    let key_path = std::path::PathBuf::from(&server.key_path);
    let client = sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: server.fingerprint.as_deref(),
    })
    .await?;
    if server.fingerprint.as_deref().unwrap_or("").is_empty() {
        persist_fingerprint_if_new(&server_key, client.fingerprint());
    }
    let sftp_arc = Arc::new(client);

    // LockPresence: always available; advisory regardless of bridge config.
    let lp = sync::LockPresence::new(sftp_arc.clone(), server.remote_root.clone(), app.clone());
    lp.start().await;

    // Phase 1g: spawn SSH local-port-forward when the profile has a bridge port.
    // BridgeClient then targets the tunnel's kernel-assigned local_port instead
    // of trying to dial bridge_port directly (which only works if the user has
    // an external `ssh -L` up — the WPF migration pain point).
    let active_tunnel = match (server.bridge_port, server.bridge_token.as_deref()) {
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

    // BridgeClient: only when tunnel is up. Hits the local forwarded port.
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

    // Engine start can fail (notify watcher init, snapshot init); on Err we MUST
    // tear down `lp` AND the tunnel so neither outlives this call.
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

    // try_watch can also Err (path traversal, FSW init). Same tear-down rule —
    // engine.stop() drops the locks Arc internally so we don't double-stop here.
    for spec in folders {
        if let Err(err) = engine.try_watch(spec).await {
            engine.stop().await;
            if let Some(t) = tunnel_handle {
                t.stop().await;
            }
            return Err(err);
        }
    }
    // Wire scoped folder provider so the lock-poll loop only walks watched roots.
    let engine_for_lp = engine.clone();
    lp.set_scoped_provider(std::sync::Arc::new(move || engine_for_lp.watched_remote_roots()))
        .await;
    let status = engine.status().await;

    let mut g = state.0.lock().await;
    if let Some(prev) = g.take() {
        prev.stop().await;
    }
    *g = Some(engine);
    drop(g);

    // Stash tunnel — replaces any stale handle from a prior session.
    let mut tg = tunnel_state.0.lock().await;
    if let Some(prev) = tg.take() {
        prev.stop().await;
    }
    *tg = tunnel_handle;
    Ok(status)
}

#[tauri::command]
async fn stop_autosync(
    state: tauri::State<'_, AutoSyncState>,
    tunnel_state: tauri::State<'_, TunnelState>,
) -> Result<(), String> {
    let mut g = state.0.lock().await;
    if let Some(engine) = g.take() {
        engine.stop().await;
    }
    drop(g);
    // Tunnel teardown after engine — bridge calls (which used the tunnel) have
    // drained on engine.stop().
    let mut tg = tunnel_state.0.lock().await;
    if let Some(t) = tg.take() {
        t.stop().await;
    }
    Ok(())
}

#[tauri::command]
async fn get_autosync_status(
    state: tauri::State<'_, AutoSyncState>,
) -> Result<Option<AutoSyncStatus>, String> {
    let g = state.0.lock().await;
    if let Some(engine) = g.as_ref() {
        Ok(Some(engine.status().await))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn enqueue_for_flush_batch(
    paths: Vec<String>,
    deleted: bool,
    bypass_preflight: bool,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<u32, String> {
    let g = state.0.lock().await;
    let Some(engine) = g.as_ref() else { return Ok(0) };
    let bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    Ok(engine.enqueue_for_flush_batch(bufs, deleted, bypass_preflight).await)
}

#[tauri::command]
async fn resolve_conflict(
    local_path: String,
    resolution: ConflictResolution,
    state: tauri::State<'_, AutoSyncState>,
) -> Result<(), String> {
    let g = state.0.lock().await;
    if let Some(engine) = g.as_ref() {
        engine.resolve_conflict(&PathBuf::from(local_path), resolution).await?;
    }
    Ok(())
}

#[tauri::command]
async fn retry_failed(state: tauri::State<'_, AutoSyncState>) -> Result<(), String> {
    let g = state.0.lock().await;
    if let Some(engine) = g.as_ref() {
        engine.retry_failed().await;
    }
    Ok(())
}

// ─── Phase 2 (UI shell) — server picker / last-selected commands ─────────────

#[tauri::command]
fn list_servers() -> Result<Vec<profile::ServerProfile>, String> {
    Ok(profile::RiftConfig::load()?.servers)
}

#[tauri::command]
fn get_last_selected() -> Result<Option<String>, String> {
    Ok(profile::RiftConfig::load()?.last_selected)
}

#[tauri::command]
fn set_last_selected(key: String) -> Result<(), String> {
    let mut cfg = profile::RiftConfig::load()?;
    if !cfg.servers.iter().any(|s| s.key == key) {
        return Err(format!("no server with key '{key}'"));
    }
    cfg.last_selected = Some(key);
    cfg.save()
}

/// Phase 5.1 / 1i write-back. Add a new server (when `edit_key=None`) or update
/// an existing one. Slug-based key; enforces unique key on add. Round-trips
/// `RiftConfig` so `serde(flatten) extra` preserves any WPF-only fields.
#[tauri::command]
fn save_server(
    profile: profile::ServerProfile,
    edit_key: Option<String>,
) -> Result<profile::ServerProfile, String> {
    let mut cfg = profile::RiftConfig::load().or_else(|_| Ok::<_, String>(profile::RiftConfig::default()))?;

    let mut next = profile;
    if next.name.trim().is_empty() {
        return Err("name is required".into());
    }
    if next.host.trim().is_empty() {
        return Err("host is required".into());
    }
    if next.user.trim().is_empty() {
        return Err("user is required".into());
    }
    if next.added_at.as_deref().unwrap_or("").is_empty() {
        next.added_at = Some(chrono::Utc::now().to_rfc3339());
    }

    match edit_key {
        Some(key) => {
            let pos = cfg.servers.iter().position(|s| s.key == key)
                .ok_or_else(|| format!("no server with key '{key}'"))?;
            // preserve the original key (don't allow renames here — slug stays stable)
            next.key = key;
            // preserve fingerprint if the form didn't supply one (TOFU stays valid)
            if next.fingerprint.as_deref().unwrap_or("").is_empty() {
                next.fingerprint = cfg.servers[pos].fingerprint.clone();
            }
            cfg.servers[pos] = next.clone();
        }
        None => {
            let base = if next.key.trim().is_empty() {
                profile::slugify(&next.name)
            } else {
                next.key.clone()
            };
            let existing: Vec<String> = cfg.servers.iter().map(|s| s.key.clone()).collect();
            next.key = profile::unique_key(&base, &existing);
            cfg.servers.push(next.clone());
            if cfg.last_selected.is_none() {
                cfg.last_selected = Some(next.key.clone());
            }
        }
    }

    cfg.save()?;
    Ok(next)
}

#[tauri::command]
fn delete_server(key: String) -> Result<(), String> {
    let mut cfg = profile::RiftConfig::load()?;
    let before = cfg.servers.len();
    cfg.servers.retain(|s| s.key != key);
    if cfg.servers.len() == before {
        return Err(format!("no server with key '{key}'"));
    }
    if cfg.last_selected.as_deref() == Some(key.as_str()) {
        cfg.last_selected = cfg.servers.first().map(|s| s.key.clone());
    }
    cfg.save()
}

/// Persist a freshly-captured fingerprint to a server profile. Idempotent — no
/// write if the profile already had the same value. Closes the 1i TOFU loop.
fn persist_fingerprint_if_new(server_key: &str, fingerprint: &str) {
    if fingerprint.is_empty() {
        return;
    }
    let mut cfg = match profile::RiftConfig::load() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("persist_fingerprint_if_new load failed for {server_key}: {e}");
            return;
        }
    };
    let Some(pos) = cfg.servers.iter().position(|s| s.key == server_key) else { return };
    if cfg.servers[pos].fingerprint.as_deref().unwrap_or("") == fingerprint {
        return;
    }
    if cfg.servers[pos].fingerprint.is_some() {
        // Profile had a different pinned fingerprint — DON'T silently overwrite.
        // The connect would've been rejected by the substring match anyway.
        log::warn!(
            "fingerprint mismatch for {server_key}; keeping pinned value"
        );
        return;
    }
    cfg.servers[pos].fingerprint = Some(fingerprint.to_string());
    if let Err(e) = cfg.save() {
        log::warn!("persist fingerprint for {server_key}: {e}");
    }
}

// ─── Phase 3 (Browser) — local + remote dir listing + batch transfer ─────────

/// Browser-pane LocalEntry shape. Distinct from `local_fs::LocalEntry` because
/// the frontend pre-dates the canonical version and uses a flatter
/// `{path, is_dir, mtime: unix-seconds}` shape. Adapter constructed via the
/// canonical walker; frontend type stays stable until the UI redesign rewires.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
}

#[tauri::command]
fn local_list_dir(path: String) -> Result<Vec<LocalEntry>, String> {
    let p = std::path::Path::new(&path);
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
    let key_path = std::path::PathBuf::from(&server.key_path);
    let client = sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: server.fingerprint.as_deref(),
    })
    .await?;
    // 1i TOFU close — first-connect captures fingerprint, persist it.
    if server.fingerprint.as_deref().unwrap_or("").is_empty() {
        persist_fingerprint_if_new(server_key, client.fingerprint());
    }
    Ok(client)
}

#[tauri::command]
async fn remote_list_dir(
    server_key: String,
    path: String,
) -> Result<Vec<sftp::RemoteEntry>, String> {
    let client = open_sftp_for(&server_key).await?;
    let entries = client.list_directory(&path).await;
    client.close().await;
    entries
}

#[tauri::command]
async fn upload_paths(
    server_key: String,
    jobs: Vec<(String, String)>,
) -> Result<Vec<bool>, String> {
    let client = open_sftp_for(&server_key).await?;
    let mapped: Vec<(PathBuf, String)> = jobs
        .into_iter()
        .map(|(local, remote)| (PathBuf::from(local), remote))
        .collect();
    let result = client.upload_files_batch(&mapped, 4).await;
    client.close().await;
    Ok(result)
}

#[tauri::command]
async fn download_paths(
    server_key: String,
    jobs: Vec<(String, String)>,
) -> Result<Vec<bool>, String> {
    let client = open_sftp_for(&server_key).await?;
    let mapped: Vec<(String, PathBuf)> = jobs
        .into_iter()
        .map(|(remote, local)| (remote, PathBuf::from(local)))
        .collect();
    let result = client.download_files_batch(&mapped, 4).await;
    client.close().await;
    Ok(result)
}

// ─── Phase 1j (tail services) — bootstrap detect / keygen / update check ─────

#[tauri::command]
async fn detect_bootstrap(
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

/// Phase 5.2 — list every file under remote_root (recursive, skip [disabled])
/// and pre-compute the local destination paths under `local_root`. Returns
/// (remote_full_path, local_full_path) pairs ready to feed into `download_paths`.
#[tauri::command]
async fn bootstrap_list_files(
    server_key: String,
    local_root: String,
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

    let local_root_path = std::path::PathBuf::from(&local_root);
    let mut jobs = Vec::with_capacity(entries.len());
    for e in entries {
        if e.is_dir { continue; }
        if e.full_path.contains("/[disabled]/") { continue; }
        // Case-insensitive prefix match on the original byte-string. Avoids
        // slicing the lowercased copy (which has a different byte length for
        // Turkish `İ`→`i` and similar Unicode pairs and would panic on a
        // non-char-boundary slice).
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

/// Audit H11: reject paths that can escape via `..`. Rift's identity files
/// and key-output dirs come from JS; without this, a malicious profile
/// could point key reads at arbitrary filesystem locations.
fn reject_path_traversal(p: &std::path::Path, label: &str) -> Result<(), String> {
    use std::path::Component;
    for c in p.components() {
        if matches!(c, Component::ParentDir) {
            return Err(format!("{label}: '..' components not allowed"));
        }
    }
    Ok(())
}

#[tauri::command]
fn generate_ssh_key(
    target_dir: String,
    filename: String,
    comment: String,
) -> Result<transport::KeyPaths, String> {
    let dir = std::path::PathBuf::from(target_dir);
    reject_path_traversal(&dir, "target_dir")?;
    transport::SshKeygen::generate(&dir, &filename, &comment)
}

#[tauri::command]
fn generate_default_ssh_key(comment: Option<String>) -> Result<transport::KeyPaths, String> {
    transport::SshKeygen::generate_default(comment.as_deref())
}

#[tauri::command]
fn default_ssh_key_exists() -> bool {
    transport::SshKeygen::default_key_exists()
}

/// Returns the canonical default ed25519 key path for this user
/// (typically `~/.ssh/id_ed25519` resolved against the home dir).
/// Used by the AddServer dialog to pre-fill the Identity file field
/// instead of guessing with a literal `~` that russh can't expand.
#[tauri::command]
fn default_ssh_key_path() -> Option<String> {
    transport::SshKeygen::default_key_path().map(|p| p.to_string_lossy().to_string())
}

/// Audit C2 — TOFU prompt: probe the server's host-key fingerprint by
/// opening a one-shot SFTP session WITHOUT a pinned fingerprint, capturing
/// what the server presents, then closing immediately. The fingerprint is
/// returned to the UI so the user can confirm before we save it to the
/// profile. Avoids the prior blind-TOFU silent persist.
#[tauri::command]
async fn probe_server_fingerprint(server_key: String) -> Result<String, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    let key_path = std::path::PathBuf::from(&server.key_path);
    reject_path_traversal(&key_path, "key_path")?;
    let client = sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: None,
    })
    .await?;
    let fp = client.fingerprint().to_string();
    client.close().await;
    Ok(fp)
}

/// Audit C2 — write a user-confirmed fingerprint to a profile. Distinct from
/// `persist_fingerprint_if_new` (silent TOFU): this is the explicit-trust path
/// triggered by the confirmation dialog, and it overwrites any prior value
/// because the user has just decided.
#[tauri::command]
fn set_server_fingerprint(server_key: String, fingerprint: String) -> Result<(), String> {
    if fingerprint.trim().is_empty() {
        return Err("empty fingerprint".into());
    }
    let mut cfg = profile::RiftConfig::load()?;
    let pos = cfg
        .servers
        .iter()
        .position(|s| s.key == server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?;
    cfg.servers[pos].fingerprint = Some(fingerprint);
    cfg.save().map_err(|e| format!("save profile: {e}"))?;
    Ok(())
}

#[tauri::command]
fn read_default_ssh_pub_key() -> Option<String> {
    transport::SshKeygen::read_default_pub_key()
}

#[tauri::command]
async fn check_for_updates() -> Result<Option<update_service::UpdateInfoDto>, String> {
    // Velopack's check is blocking network I/O — run it on a blocking thread
    // so the runtime isn't parked on the github roundtrip.
    tokio::task::spawn_blocking(|| update_service::UpdateService::new().check())
        .await
        .map_err(|e| format!("update check task: {e}"))?
}

// ─── Phase 4 (Sync surfaces) ─────────────────────────────────────────────────

async fn editor_for(
    server_key: &str,
    state: &EditInPlaceState,
    app: &tauri::AppHandle,
) -> Result<Arc<edit::in_place::EditInPlaceManager>, String> {
    // Fast-path read under the lock, then drop it before the SFTP open so a
    // slow connection on one server can't stall begin_edit_in_place calls
    // on a different server (or the same server's other editor methods).
    {
        let g = state.0.lock().await;
        if let Some(m) = g.get(server_key) {
            return Ok(m.clone());
        }
    }
    let client = open_sftp_for(server_key).await?;
    let sftp_arc = Arc::new(client);
    let mgr = Arc::new(edit::in_place::EditInPlaceManager::new(server_key.to_string(), sftp_arc, app.clone())?);
    // Two concurrent first-time inits for the same server may both reach
    // here; `or_insert` ensures one wins and the other Arc<EditInPlaceManager>
    // (with its SFTP client) is dropped immediately. Worst case: one
    // wasted SFTP handshake. Better than UI-wide stall.
    let mut g = state.0.lock().await;
    Ok(g.entry(server_key.to_string()).or_insert(mgr).clone())
}

#[tauri::command]
async fn begin_edit_in_place(
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
async fn save_edit_in_place(
    server_key: String,
    remote_path: String,
    state: tauri::State<'_, EditInPlaceState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mgr = editor_for(&server_key, &state, &app).await?;
    mgr.save(&remote_path).await
}

#[tauri::command]
async fn close_edit_in_place(
    server_key: String,
    remote_path: String,
    state: tauri::State<'_, EditInPlaceState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mgr = editor_for(&server_key, &state, &app).await?;
    mgr.close(&remote_path).await
}

#[tauri::command]
async fn list_watched_edits(
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

/// Application entry point. Velopack hooks run FIRST (before Tauri spins up)
/// so install/update commands like `--veloapp-install` exit cleanly without
/// dragging the whole UI runtime through the lifecycle event. Mirrors the WPF
/// `Main()` pattern. After Velopack, registers managed state + Tauri commands
/// and blocks on the event loop.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    velopack::VelopackApp::build().run();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AutoSyncState(AsyncMutex::new(None)))
        .manage(TunnelState(AsyncMutex::new(None)))
        .manage(EditInPlaceState(AsyncMutex::new(std::collections::HashMap::new())))
        .invoke_handler(tauri::generate_handler![
            app_version,
            scan_drift,
            start_autosync,
            stop_autosync,
            get_autosync_status,
            enqueue_for_flush_batch,
            resolve_conflict,
            retry_failed,
            list_servers,
            get_last_selected,
            set_last_selected,
            save_server,
            delete_server,
            local_list_dir,
            remote_list_dir,
            upload_paths,
            download_paths,
            detect_bootstrap,
            bootstrap_list_files,
            generate_ssh_key,
            generate_default_ssh_key,
            default_ssh_key_exists,
            default_ssh_key_path,
            read_default_ssh_pub_key,
            probe_server_fingerprint,
            set_server_fingerprint,
            check_for_updates,
            begin_edit_in_place,
            save_edit_in_place,
            close_edit_in_place,
            list_watched_edits,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
