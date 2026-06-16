//! Loopback TCP bridge between the Assistant's MCP server child process and
//! the parent Rift Tauri process — UI-presentation ops only. The MCP child
//! runs in a separate process with no Tauri handle, so anything that must
//! touch the running app (emit an event, await a user answer) round-trips
//! through here. (The old remote/sync op set died with the pure-assistant
//! conversion; this is the minimal resurrection that brings `ask_user` back
//! and adds `open_browser` + `notify`.)
//!
//! Protocol: NDJSON over `127.0.0.1:<random-port>`, one request line → one
//! response line: `{"op":"...","token":"<b64url>", ...}` →
//! `{"ok":true,"data":{...}}` / `{"ok":false,"error":"..."}`.
//! Auth: per-launch random token handed to the MCP child via the
//! `RIFT_BRIDGE_TOKEN` env in the per-turn `--mcp-config` file.
//! Lifetime: spawned on app boot, lives for the rest of the process.
//!
//! Ops:
//!   * `ask_user`     — park until the user answers in the chat UI (≤10 min).
//!   * `open_browser` — show an http/https URL in the in-app browser dock.
//!   * `notify`       — toast in Rift's corner.

use std::sync::OnceLock;
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct BridgeInfo {
    pub port: u16,
    pub token: String,
}

static BRIDGE: OnceLock<BridgeInfo> = OnceLock::new();

/// Port + token for `write_mcp_config` to thread into the MCP child's env.
/// `None` until `start` has bound the listener (or if binding failed — the
/// bridge tools simply don't list in that case).
pub fn bridge_info() -> Option<&'static BridgeInfo> {
    BRIDGE.get()
}

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    token: String,
    /// `ask_user`: opaque id minted by the MCP child so the frontend can
    /// route the user's answer back to the matching pending bridge waiter.
    #[serde(default)]
    request_id: Option<String>,
    /// Convo session-id (the MCP child reads `RIFT_SESSION_ID`) — scopes
    /// events to the right chat tab.
    #[serde(default)]
    session_id: Option<String>,
    /// #37: originating window label (MCP child reads `RIFT_BRIDGE_WINDOW`) so
    /// bridge events emit_to that window instead of broadcasting app-wide.
    #[serde(default)]
    window_label: Option<String>,
    /// `ask_user`: pass-through questions payload — same shape the built-in
    /// `AskUserQuestion` tool emits.
    #[serde(default)]
    questions: Option<Value>,
    /// `open_browser`: target URL (http/https only — same allowlist as the
    /// dock's address bar).
    #[serde(default)]
    url: Option<String>,
    /// `notify`: toast fields.
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    severity: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

fn err(msg: impl Into<String>) -> Response {
    Response { ok: false, error: Some(msg.into()), ..Response::default() }
}

fn ok_with(data: Value) -> Response {
    Response { ok: true, data: Some(data), ..Response::default() }
}

/// #37: the window to emit_to — the request's label, or "main" when absent
/// (single-window installs / a child that predates the env var).
fn window_of(req: &Request) -> String {
    req.window_label
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("main")
        .to_string()
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
        // Race-loss: another initializer won — return its value, never panic.
        return BRIDGE
            .get()
            .cloned()
            .ok_or_else(|| "bridge OnceLock race-loss with no winning value".to_string());
    }
    log::info!("assistant bridge: listening on 127.0.0.1:{port}");

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let app = app.clone();
                    let token = token.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(app, token, stream).await {
                            log::debug!("assistant bridge conn closed: {e}");
                        }
                    });
                }
                Err(e) => {
                    log::warn!("assistant bridge accept: {e}");
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            }
        }
    });

    Ok(info)
}

async fn handle_conn(
    app: AppHandle,
    token: String,
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
        if req.token != token {
            // Write+flush the error, then orderly-shutdown so the client sees
            // "unauthorized" instead of a connection reset.
            write_line(&mut write_half, &err("unauthorized")).await?;
            let _ = write_half.shutdown().await;
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
        "ask_user" => ask_user_op(app, req).await,
        "open_browser" => open_browser_op(app, req),
        "notify" => notify_op(app, req),
        other => err(format!("unknown op `{other}`")),
    }
}

/// Park the bridge call until the user answers the question in the chat UI.
/// Emits `assistant://ask-user` so the chat tab's listener can pair the
/// request_id with the matching tool block, then `await`s the registry
/// oneshot. 10-min timeout; on timeout the MCP child gets `ok: false` and
/// surfaces "user did not answer" so the model falls back to plain-text asking.
/// Bridge events carry the originating `session_id` so the frontend routes them
/// to the right chat tab. A missing id (shouldn't happen — the MCP child always
/// sets it) would silently misroute or drop the event, so log the fallback
/// instead of swallowing it with `unwrap_or_default()`. (B7)
fn session_id_or_warn(id: Option<String>, op: &str) -> String {
    match id {
        Some(s) if !s.trim().is_empty() => s,
        _ => {
            log::warn!("{op}: bridge request missing session_id — event may misroute");
            String::new()
        }
    }
}

async fn ask_user_op(app: &AppHandle, req: Request) -> Response {
    let window = window_of(&req);
    let request_id = match req.request_id {
        Some(s) if !s.trim().is_empty() => s,
        _ => return err("ask_user: missing `request_id`"),
    };
    let session_id = session_id_or_warn(req.session_id, "ask_user");
    let questions = req.questions.unwrap_or(Value::Null);

    let registry = match app.try_state::<std::sync::Arc<crate::assistant::AskUserRegistry>>() {
        Some(r) => r.inner().clone(),
        None => return err("ask_user: registry not managed (init bug)"),
    };
    let rx = registry.register(request_id.clone());

    // Emit AFTER registering — guarantees the receiver is in the map before
    // the frontend can possibly fire an answer back.
    let _ = app.emit_to(
        window.as_str(),
        "assistant://ask-user",
        serde_json::json!({
            "request_id": request_id,
            "session_id": session_id,
            "questions": questions,
        }),
    );

    match tokio::time::timeout(Duration::from_secs(600), rx).await {
        Ok(Ok(answer)) => ok_with(answer),
        Ok(Err(_)) => {
            registry.cancel(&request_id);
            err("ask_user: pending entry dropped before an answer arrived")
        }
        Err(_) => {
            registry.cancel(&request_id);
            err("ask_user: user did not answer within 10 minutes")
        }
    }
}

/// Validate the scheme, then hand the URL to the frontend via
/// `assistant://open-browser`. The frontend owns dock visibility + the stage
/// rect the native webview is positioned against, so it drives the actual
/// `browser_open` — the bridge never touches the webview directly.
fn open_browser_op(app: &AppHandle, req: Request) -> Response {
    let url = match req.url.as_deref().map(str::trim) {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => return err("open_browser: missing `url`"),
    };
    if let Err(e) = crate::browser::parse_url(&url) {
        return err(format!("open_browser: {e}"));
    }
    let _ = app.emit_to(
        &window_of(&req),
        "assistant://open-browser",
        serde_json::json!({
            "url": url,
            "session_id": session_id_or_warn(req.session_id, "open_browser"),
        }),
    );
    ok_with(serde_json::json!({ "opened": url }))
}

/// Surface a toast in Rift's corner via `assistant://notify`. Length-capped
/// and severity-allowlisted here so the frontend can render the payload as-is.
fn notify_op(app: &AppHandle, req: Request) -> Response {
    let title: String = match req.title.as_deref().map(str::trim) {
        Some(t) if !t.is_empty() => t.chars().take(200).collect(),
        _ => return err("notify: missing `title`"),
    };
    let detail: Option<String> = req
        .detail
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|d| d.chars().take(500).collect());
    let severity = match req.severity.as_deref() {
        Some("ok") => "ok",
        Some("warn") => "warn",
        Some("danger") => "danger",
        _ => "info",
    };
    let _ = app.emit_to(
        &window_of(&req),
        "assistant://notify",
        serde_json::json!({
            "title": title,
            "detail": detail,
            "severity": severity,
            "session_id": session_id_or_warn(req.session_id, "notify"),
        }),
    );
    ok_with(serde_json::json!({ "shown": true }))
}
