//! Loopback TCP bridge between the Assistant's MCP server child process and
//! the parent Rift Tauri process. Lets the MCP server reuse the auto-sync
//! engine's live russh session (cheaper than dialing a fresh SSH connection
//! per turn) and surface workspace-scoped advisory locks held by other users.
//!
//! Protocol: NDJSON over `127.0.0.1:<random-port>`. Each request is one line:
//!   `{"op":"remote_bash","token":"<hex>","command":"...","timeout_secs":60}`
//! Each response is one line:
//!   `{"ok":true,"stdout":"...","stderr":"...","exit_code":0,"truncated":false}`
//! Auth: shared hex token passed to the MCP child via `RIFT_BRIDGE_TOKEN` env.
//! Lifetime: spawned on app boot, lives for the rest of the process. The
//! listener binds to port 0 so we get a kernel-assigned port; the port +
//! token are written to a static accessible via `bridge_info()` and threaded
//! into the MCP server's env at spawn-time.

use std::sync::OnceLock;
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::sftp::remote_exec::ExecOutput;
use crate::AutoSyncState;

#[derive(Debug, Clone)]
pub struct BridgeInfo {
    pub port: u16,
    pub token: String,
}

static BRIDGE: OnceLock<BridgeInfo> = OnceLock::new();

pub fn bridge_info() -> Option<&'static BridgeInfo> {
    BRIDGE.get()
}

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    token: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

/// Flat response shape — every field optional so a single struct handles bash
/// output, lock status, and errors. Client distinguishes by `ok` + presence.
#[derive(Debug, Default, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    held_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<String>,
}

fn err(msg: impl Into<String>) -> Response {
    Response { ok: false, error: Some(msg.into()), ..Response::default() }
}

/// Spawn the bridge listener exactly once. Idempotent. Returns the cached
/// `BridgeInfo` after the first successful bind.
pub async fn start(app: AppHandle) -> Result<BridgeInfo, String> {
    if let Some(info) = BRIDGE.get() {
        return Ok(info.clone());
    }
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("bridge bind: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("bridge local_addr: {e}"))?
        .port();
    let mut token_bytes = [0u8; 24];
    rand::fill(&mut token_bytes);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes);

    let info = BridgeInfo { port, token: token.clone() };
    if BRIDGE.set(info.clone()).is_err() {
        return Ok(BRIDGE.get().unwrap().clone());
    }
    log::info!("assistant remote_bridge: listening on 127.0.0.1:{port}");

    let app_for_accept = app.clone();
    let token_for_accept = token;
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let app = app_for_accept.clone();
                    let token = token_for_accept.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(app, token, stream).await {
                            log::debug!("assistant remote_bridge conn closed: {e}");
                        }
                    });
                }
                Err(e) => {
                    log::warn!("assistant remote_bridge accept: {e}");
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            }
        }
    });

    Ok(info)
}

async fn handle_conn(
    app: AppHandle,
    expected_token: String,
    stream: tokio::net::TcpStream,
) -> Result<(), String> {
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let _ = write_line(&mut write_half, &err(format!("invalid request: {e}"))).await;
                continue;
            }
        };
        if req.token != expected_token {
            let _ = write_line(&mut write_half, &err("unauthorized")).await;
            return Err("unauthorized".into());
        }
        let resp = dispatch(&app, req).await;
        write_line(&mut write_half, &resp).await?;
    }
    Ok(())
}

async fn write_line(
    w: &mut tokio::net::tcp::OwnedWriteHalf,
    resp: &Response,
) -> Result<(), String> {
    let mut s = serde_json::to_string(resp).map_err(|e| e.to_string())?;
    s.push('\n');
    w.write_all(s.as_bytes()).await.map_err(|e| e.to_string())?;
    w.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn dispatch(app: &AppHandle, req: Request) -> Response {
    match req.op.as_str() {
        "remote_bash" => {
            let command = match req.command {
                Some(c) if !c.trim().is_empty() => c,
                _ => return err("missing or empty `command`"),
            };
            let timeout = Duration::from_secs(req.timeout_secs.unwrap_or(60).clamp(1, 600));
            run_remote_bash(app, command, timeout).await
        }
        "shell_lock_status" => shell_lock_status(app).await,
        other => err(format!("unknown op `{other}`")),
    }
}

async fn engine(app: &AppHandle) -> Option<std::sync::Arc<crate::sync::AutoSyncEngine>> {
    let state = app.state::<AutoSyncState>();
    let guard = state.0.lock().await;
    guard.clone()
}

fn shell_lock_key(remote_root: &str) -> String {
    let trimmed = remote_root.trim_end_matches('/');
    format!("{trimmed}/.rift-shell")
}

async fn shell_lock_status(app: &AppHandle) -> Response {
    let empty = Response { ok: true, ..Response::default() };
    let Some(eng) = engine(app).await else { return empty; };
    let Some(locks) = eng.locks() else { return empty; };
    let folders = eng.folders_clone();
    let Some(first) = folders.first() else { return empty; };
    let key = shell_lock_key(&first.remote_root);
    if let Some(l) = locks.find_lock_by_other(&key) {
        Response {
            ok: true,
            held_by: Some(l.user),
            host: Some(l.host),
            since: Some(l.since.to_rfc3339()),
            ..Response::default()
        }
    } else {
        empty
    }
}

async fn run_remote_bash(app: &AppHandle, command: String, timeout: Duration) -> Response {
    let Some(eng) = engine(app).await else {
        return err("no remote connection — start a server in Rift first");
    };
    let folders = eng.folders_clone();
    let Some(first) = folders.first() else {
        return err("no folders configured on the active connection");
    };
    let remote_root = first.remote_root.clone();
    let lock_key = shell_lock_key(&remote_root);

    if let Some(locks) = eng.locks() {
        if let Some(foreign) = locks.find_lock_by_other(&lock_key) {
            return err(format!(
                "remote shell is locked by {} on {} (since {}); retry shortly",
                foreign.user,
                foreign.host,
                foreign.since.to_rfc3339()
            ));
        }
        locks.acquire(&lock_key).await;
    }

    let sftp = eng.sftp();
    let exec_result = sftp.exec_bash(&command, timeout).await;

    if let Some(locks) = eng.locks() {
        locks.release(&lock_key).await;
    }

    use tauri::Emitter;
    let _ = app.emit("assistant://remote-shell-fired", serde_json::json!({
        "command": preview(&command),
        "remote_root": remote_root,
        "at": chrono::Utc::now().to_rfc3339(),
    }));

    match exec_result {
        Ok(out) => to_bash_response(out),
        Err(e) => err(e),
    }
}

fn to_bash_response(out: ExecOutput) -> Response {
    Response {
        ok: true,
        stdout: Some(out.stdout),
        stderr: Some(out.stderr),
        exit_code: out.exit_code,
        truncated: Some(out.truncated),
        ..Response::default()
    }
}

fn preview(cmd: &str) -> String {
    let first_line = cmd.lines().next().unwrap_or("").trim();
    if first_line.len() > 80 {
        format!("{}\u{2026}", &first_line[..80])
    } else {
        first_line.to_string()
    }
}

