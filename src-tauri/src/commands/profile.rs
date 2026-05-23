//! Server profile + SSH key + TOFU fingerprint command surface (#20).

use crate::{profile, sftp, transport, AutoSyncState, TunnelState};
use super::reject_path_traversal;

#[tauri::command]
pub fn list_servers() -> Result<Vec<profile::ServerProfilePublic>, String> {
    Ok(profile::RiftConfig::load()?.servers.iter().map(Into::into).collect())
}

#[tauri::command]
pub fn get_last_selected() -> Result<Option<String>, String> {
    Ok(profile::RiftConfig::load()?.last_selected)
}

#[tauri::command]
pub fn set_last_selected(key: String) -> Result<(), String> {
    let mut cfg = profile::RiftConfig::load()?;
    if !cfg.servers.iter().any(|s| s.key == key) {
        return Err(format!("no server with key '{key}'"));
    }
    cfg.last_selected = Some(key);
    cfg.save()
}

/// Phase 5.1 / 1i write-back. Add a new server (when `edit_key=None`) or update
/// an existing one. Slug-based key; enforces unique key on add.
#[tauri::command]
pub fn save_server(
    profile: profile::ServerProfile,
    edit_key: Option<String>,
) -> Result<profile::ServerProfilePublic, String> {
    let mut cfg = profile::RiftConfig::load()
        .map_err(|e| format!("failed to load rift config: {e}"))?;

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

    let incoming_token = next.bridge_token.take().filter(|s| !s.is_empty());

    match edit_key {
        Some(key) => {
            let pos = cfg.servers.iter().position(|s| s.key == key)
                .ok_or_else(|| format!("no server with key '{key}'"))?;
            next.key = key.clone();
            if next.fingerprint.as_deref().unwrap_or("").is_empty() {
                next.fingerprint = cfg.servers[pos].fingerprint.clone();
            }
            if let Some(tok) = incoming_token.as_deref() {
                profile::set_server_bridge_token(&key, Some(tok))?;
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
            if let Some(tok) = incoming_token.as_deref() {
                profile::set_server_bridge_token(&next.key, Some(tok))?;
            }
            cfg.servers.push(next.clone());
            if cfg.last_selected.is_none() {
                cfg.last_selected = Some(next.key.clone());
            }
        }
    }

    cfg.save()?;
    Ok((&next).into())
}

#[tauri::command]
pub async fn delete_server(
    key: String,
    state: tauri::State<'_, AutoSyncState>,
    tunnel_state: tauri::State<'_, TunnelState>,
) -> Result<(), String> {
    // #56: stop the active engine + tunnel BEFORE the profile is gone from
    // disk. Mirrors `stop_autosync`'s order: engine first, then tunnel.
    {
        let mut g = state.0.lock().await;
        let take_engine = g
            .as_ref()
            .map(|e| e.profile_key() == key.as_str())
            .unwrap_or(false);
        if take_engine {
            if let Some(engine) = g.take() {
                engine.stop().await;
            }
        }
    }
    {
        let mut tg = tunnel_state.0.lock().await;
        if let Some(t) = tg.take() {
            t.stop().await;
        }
    }

    let mut cfg = profile::RiftConfig::load()?;
    let before = cfg.servers.len();
    cfg.servers.retain(|s| s.key != key);
    if cfg.servers.len() == before {
        return Err(format!("no server with key '{key}'"));
    }
    if cfg.last_selected.as_deref() == Some(key.as_str()) {
        cfg.last_selected = cfg.servers.first().map(|s| s.key.clone());
    }
    cfg.save()?;
    if let Err(e) = profile::set_server_bridge_token(&key, None) {
        log::warn!("delete_server: keychain cleanup for '{key}' failed: {e}");
    }
    Ok(())
}

#[tauri::command]
pub fn generate_ssh_key(
    target_dir: String,
    filename: String,
    comment: String,
) -> Result<transport::KeyPaths, String> {
    let dir = std::path::PathBuf::from(target_dir);
    reject_path_traversal(&dir, "target_dir")?;
    transport::SshKeygen::generate(&dir, &filename, &comment)
}

#[tauri::command]
pub fn generate_default_ssh_key(comment: Option<String>) -> Result<transport::KeyPaths, String> {
    transport::SshKeygen::generate_default(comment.as_deref())
}

#[tauri::command]
pub fn default_ssh_key_exists() -> bool {
    transport::SshKeygen::default_key_exists()
}

#[tauri::command]
pub fn default_ssh_key_path() -> Option<String> {
    transport::SshKeygen::default_key_path().map(|p| p.to_string_lossy().to_string())
}

/// Audit C2 — TOFU prompt: probe the server's host-key fingerprint without
/// pinning. Capture what the server presents, return to UI for user confirm.
#[tauri::command]
pub async fn probe_server_fingerprint(server_key: String) -> Result<String, String> {
    let cfg = profile::RiftConfig::load()?;
    let server = cfg
        .find(&server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?
        .clone();
    let key_path = std::path::PathBuf::from(&server.key_path);
    reject_path_traversal(&key_path, "key_path")?;
    // #61: TOFU probe deliberately WITHOUT pinned fingerprint or write-probe.
    let client = sftp::SftpClient::connect(sftp::ConnectArgs {
        host: &server.host,
        port: server.port,
        user: &server.user,
        key_path: &key_path,
        trusted_fingerprint: None,
        write_probe_root: None,
    })
    .await?;
    let fp = client.fingerprint().to_string();
    client.close().await;
    Ok(fp)
}

#[tauri::command]
pub fn set_server_fingerprint(server_key: String, fingerprint: String) -> Result<(), String> {
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
pub fn read_default_ssh_pub_key() -> Option<String> {
    transport::SshKeygen::read_default_pub_key()
}
