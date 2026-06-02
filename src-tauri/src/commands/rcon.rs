//! FXServer RCON command surface — drives the live console in Settings → Server.
//!
//! RCON targets the server's FXServer game/HTTP port (`bridge_port`, default
//! 30120) over UDP — NOT the SSH port. The password lives in the OS keychain
//! (`secrets::rcon_password_key`), never on disk or the IPC boundary.

use crate::{profile, rcon};

const DEFAULT_FX_PORT: u16 = 30120;

fn resolve_target(server_key: &str) -> Result<(String, u16), String> {
    let cfg = profile::RiftConfig::load()?;
    let s = cfg
        .find(server_key)
        .ok_or_else(|| format!("no server with key '{server_key}'"))?;
    let port = s.bridge_port.unwrap_or(DEFAULT_FX_PORT);
    Ok((s.host.clone(), port))
}

/// True if an RCON password is stored in the keychain for this server.
#[tauri::command]
pub fn rcon_has_password(server_key: String) -> bool {
    profile::server_rcon_password(&server_key).is_some()
}

/// Store (when non-empty) or clear (when empty) the RCON password.
#[tauri::command]
pub fn rcon_set_password(server_key: String, password: String) -> Result<(), String> {
    let val = if password.is_empty() {
        None
    } else {
        Some(password.as_str())
    };
    profile::set_server_rcon_password(&server_key, val)
}

/// Send one RCON command to the server's FXServer port and return its output.
#[tauri::command]
pub async fn rcon_send(server_key: String, command: String) -> Result<String, String> {
    let command = command.trim().to_string();
    if command.is_empty() {
        return Err("empty command".into());
    }
    let password = profile::server_rcon_password(&server_key)
        .ok_or_else(|| "no rcon password set for this server".to_string())?;
    let (host, port) = resolve_target(&server_key)?;
    rcon::send(&host, port, &password, &command).await
}
