// Rift v14 — Tauri + Svelte + russh backend.
//
// Phase 0 stub: prove the toolchain end-to-end. One Tauri command (sftp_list)
// that connects to a server via russh + russh-sftp and lists a remote dir.
// No persistence, no auto-sync, no drift scan — those come in Phase 1+.
//
// Velopack-Rust auto-update wired at main() — banner fires when a newer
// version is released to Blazzer10200/rift-tauri.

use russh::client::{self, Handle};
use russh::keys::*;
use russh_sftp::client::SftpSession;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::ToSocketAddrs;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectArgs {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
    pub remote_path: String,
}

struct Client {}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Phase 0: trust-on-first-use, no fingerprint pinning yet.
        // Phase 1 will port the .NET Fingerprint check.
        Ok(true)
    }
}

async fn connect_sftp(args: &ConnectArgs) -> Result<(Handle<Client>, SftpSession), String> {
    let key_path = PathBuf::from(&args.key_path);
    let key_pair = load_secret_key(&key_path, None)
        .map_err(|e| format!("load key {:?}: {}", key_path, e))?;

    let config = Arc::new(client::Config::default());
    let sh = Client {};
    let addr = (args.host.as_str(), args.port);
    let mut session = client::connect(config, addr_to_string(addr), sh)
        .await
        .map_err(|e| format!("connect {}:{}: {}", args.host, args.port, e))?;

    let hash: Option<russh::keys::HashAlg> = match session.best_supported_rsa_hash().await {
        Ok(Some(Some(h))) => Some(h),
        _ => None,
    };
    let auth = session
        .authenticate_publickey(
            &args.user,
            russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash),
        )
        .await
        .map_err(|e| format!("auth: {}", e))?;

    if !auth.success() {
        return Err(format!("auth rejected for {}@{}", args.user, args.host));
    }

    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| format!("open channel: {}", e))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("request sftp: {}", e))?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("sftp init: {}", e))?;

    Ok((session, sftp))
}

fn addr_to_string<A: ToSocketAddrs + std::fmt::Debug>(a: A) -> String {
    format!("{:?}", a)
        .trim_start_matches('(')
        .trim_end_matches(')')
        .replace('"', "")
        .replace(", ", ":")
}

#[tauri::command]
async fn sftp_list(args: ConnectArgs) -> Result<Vec<ListEntry>, String> {
    let (_session, sftp) = connect_sftp(&args).await?;

    let mut entries = Vec::new();
    let mut dir = sftp
        .read_dir(&args.remote_path)
        .await
        .map_err(|e| format!("readdir {}: {}", args.remote_path, e))?;

    while let Some(entry) = dir.next() {
        let metadata = entry.metadata();
        entries.push(ListEntry {
            name: entry.file_name(),
            is_dir: metadata.is_dir(),
            size: metadata.size.unwrap_or(0),
            mtime: metadata.mtime.unwrap_or(0) as i64,
        });
    }

    sftp.close().await.ok();
    Ok(entries)
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Velopack first — handles --veloapp-install/--veloapp-updated/etc and
    // exits before Tauri spins up. Mirrors the WPF Main() pattern.
    velopack::VelopackApp::build().run();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![sftp_list, app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
