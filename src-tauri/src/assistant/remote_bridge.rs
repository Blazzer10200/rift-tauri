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
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::sftp::remote_exec::ExecOutput;
use crate::AutoSyncState;

#[derive(Debug, Clone)]
pub struct BridgeInfo {
    pub port: u16,
    /// Write-scoped token. Required for ops that mutate remote state (`remote_bash`).
    /// Only injected into the MCP child env when `remote_shell_enabled` is true.
    pub token: String,
    /// Read-only token. Authorizes `sync_status` + `shell_lock_status` only.
    /// Always injected so the MCP child can report sync state regardless of the
    /// remote-shell toggle. #62: split so a compromised MCP tool that only has
    /// `sync_status` access can't escalate to `remote_bash`.
    pub readonly_token: String,
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
    /// Acknowledgement latch for destructive sync ops (`reconcile_apply`).
    /// The MCP tool layer instructs the model to call read-only ops first
    /// (`reconcile_preview` + `drift_snapshot`), show the user, then re-call
    /// with `confirm: true` to actually mutate.
    #[serde(default)]
    confirm: Option<bool>,
    /// `ask_user` only: opaque id minted by the MCP child so the frontend can
    /// route the user's answer back to the matching pending bridge waiter.
    #[serde(default)]
    request_id: Option<String>,
    /// `ask_user` only: convo session-id so the frontend can scope the event
    /// to the right chat tab (the MCP child reads it from `RIFT_SESSION_ID`).
    #[serde(default)]
    session_id: Option<String>,
    /// `ask_user` only: pass-through questions payload — same shape Claude
    /// emits for the built-in `AskUserQuestion` tool (array of `{question,
    /// header, multiSelect, options}` objects). The frontend chip reads this
    /// straight from the tool_use envelope; the field here just rides along
    /// in the event payload for parity.
    #[serde(default)]
    questions: Option<Value>,
}

/// Flat response shape — every field optional so a single struct handles bash
/// output, lock status, sync status, and errors. Client distinguishes by `ok` + presence.
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
    /// Structured payload for non-bash ops (e.g. sync_status). Serialized as a
    /// JSON object; the MCP tool layer formats it into a human-readable string.
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
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
    let mut ro_bytes = [0u8; 24];
    rand::fill(&mut ro_bytes);
    let readonly_token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(ro_bytes);

    let info = BridgeInfo {
        port,
        token: token.clone(),
        readonly_token: readonly_token.clone(),
    };
    if BRIDGE.set(info.clone()).is_err() {
        // #118: `unwrap` would panic if the racing initializer is somehow
        // still in flight (theoretically impossible w/ OnceLock semantics
        // but defensive — a future refactor could change the type and the
        // panic would slip through). Return the won race's value or a
        // clear error.
        return BRIDGE
            .get()
            .cloned()
            .ok_or_else(|| "bridge OnceLock race-loss with no winning value".to_string());
    }
    log::info!("assistant remote_bridge: listening on 127.0.0.1:{port}");

    let app_for_accept = app.clone();
    let write_for_accept = token;
    let ro_for_accept = readonly_token;
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let app = app_for_accept.clone();
                    let write_tok = write_for_accept.clone();
                    let ro_tok = ro_for_accept.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(app, write_tok, ro_tok, stream).await {
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

/// Authorization scope inferred from which token the client presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    /// `sync_status`, `shell_lock_status` only.
    ReadOnly,
    /// All ops including `remote_bash`.
    Write,
}

async fn handle_conn(
    app: AppHandle,
    write_token: String,
    readonly_token: String,
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
        let scope = if req.token == write_token {
            Scope::Write
        } else if req.token == readonly_token {
            Scope::ReadOnly
        } else {
            // #69: write+flush the error line, then orderly-shutdown the write
            // half so the MCP child sees the "unauthorized" response instead of
            // a connection-reset (Windows TCP drop after `let _ = ...` can race
            // ahead of the kernel sendq flush).
            write_line(&mut write_half, &err("unauthorized")).await?;
            let _ = write_half.shutdown().await;
            return Err("unauthorized".into());
        };
        let resp = dispatch(&app, scope, req).await;
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

async fn dispatch(app: &AppHandle, scope: Scope, req: Request) -> Response {
    match req.op.as_str() {
        "remote_bash" => {
            // #62: write-scoped only. Readonly token must NOT escalate to
            // remote shell exec even if it has a valid loopback connection.
            if scope != Scope::Write {
                return err("unauthorized: remote_bash requires write-scoped token");
            }
            let command = match req.command {
                Some(c) if !c.trim().is_empty() => c,
                _ => return err("missing or empty `command`"),
            };
            let timeout = Duration::from_secs(req.timeout_secs.unwrap_or(60).clamp(1, 600));
            run_remote_bash(app, command, timeout).await
        }
        "shell_lock_status" => shell_lock_status(app).await,
        "sync_status" => sync_status_op(app).await,
        "drift_snapshot" => drift_snapshot_op(app).await,
        "reconcile_preview" => reconcile_preview_op(app).await,
        "push_pending" => {
            if scope != Scope::Write {
                return err("unauthorized: push_pending requires write-scoped token");
            }
            push_pending_op(app).await
        }
        "pull_pending" => {
            if scope != Scope::Write {
                return err("unauthorized: pull_pending requires write-scoped token");
            }
            pull_pending_op(app).await
        }
        "reconcile_apply" => {
            if scope != Scope::Write {
                return err("unauthorized: reconcile_apply requires write-scoped token");
            }
            if req.confirm != Some(true) {
                return err(
                    "reconcile_apply requires confirm: true — call reconcile_preview + drift_snapshot first, show the user the diff, then re-call with confirm: true",
                );
            }
            reconcile_apply_op(app).await
        }
        "ask_user" => ask_user_op(app, req).await,
        other => err(format!("unknown op `{other}`")),
    }
}

/// Park the bridge call until the user answers the question in the chat UI.
/// Either scope authorizes — `ask_user` is presentation-only, no remote state
/// is touched. Emits `assistant://ask-user` so the chat tab's listener can
/// pair the request_id with the matching tool block, then `await`s the
/// registry oneshot. 10-min timeout on the user's side; on timeout the MCP
/// child gets an `ok: false` and surfaces "user did not answer" so the model
/// falls back to plain-text asking.
async fn ask_user_op(app: &AppHandle, req: Request) -> Response {
    let request_id = match req.request_id {
        Some(s) if !s.trim().is_empty() => s,
        _ => return err("ask_user: missing `request_id`"),
    };
    let session_id = req.session_id.unwrap_or_default();
    let questions = req.questions.unwrap_or(Value::Null);

    let registry = match app.try_state::<std::sync::Arc<crate::assistant::AskUserRegistry>>() {
        Some(r) => r.inner().clone(),
        None => return err("ask_user: registry not managed (init bug)"),
    };
    let rx = registry.register(request_id.clone());

    // Emit AFTER registering — guarantees the receiver is in the map before
    // the frontend can possibly fire an answer back.
    let _ = app.emit(
        "assistant://ask-user",
        serde_json::json!({
            "request_id": request_id,
            "session_id": session_id,
            "questions": questions,
        }),
    );

    let timeout = Duration::from_secs(600);
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(answer)) => Response {
            ok: true,
            data: Some(answer),
            ..Default::default()
        },
        Ok(Err(_)) => {
            // Sender dropped without sending — shouldn't happen via normal
            // resolve/cancel, but be explicit.
            registry.cancel(&request_id);
            err("ask_user: pending entry dropped before an answer arrived")
        }
        Err(_) => {
            registry.cancel(&request_id);
            err("ask_user: user did not answer within 10 minutes")
        }
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

/// Return a live `AutoSyncStatus` snapshot. No lock required — read-only.
/// Always available when the bridge is running, regardless of the remote-shell toggle.
async fn sync_status_op(app: &AppHandle) -> Response {
    let Some(eng) = engine(app).await else {
        return Response {
            ok: true,
            data: Some(serde_json::json!({ "connected": false })),
            ..Default::default()
        };
    };
    let status = eng.status().await;
    let data = match serde_json::to_value(&status) {
        Ok(v) => v,
        Err(e) => return err(format!("serialize status: {e}")),
    };
    Response { ok: true, data: Some(data), ..Default::default() }
}

/// Return the last drift-scan result as a structured list. Read-only.
/// No-engine case is reported in-band (`connected: false`) rather than as an
/// error so the model can render a clean "not connected" message.
async fn drift_snapshot_op(app: &AppHandle) -> Response {
    let Some(eng) = engine(app).await else {
        return Response {
            ok: true,
            data: Some(serde_json::json!({ "connected": false, "entries": [] })),
            ..Default::default()
        };
    };
    let entries = eng.drift_snapshot();
    let entries_v = match serde_json::to_value(&entries) {
        Ok(v) => v,
        Err(e) => return err(format!("serialize drift snapshot: {e}")),
    };
    Response {
        ok: true,
        data: Some(serde_json::json!({ "connected": true, "entries": entries_v })),
        ..Default::default()
    }
}

/// Kick a fresh drift scan without applying anything. The result lands in the
/// engine's `last_scan_entries` cache; the model reads it via `drift_snapshot`.
async fn reconcile_preview_op(app: &AppHandle) -> Response {
    let Some(eng) = engine(app).await else {
        return err("not connected — start a server in Rift first");
    };
    eng.kick_drift_reconcile();
    Response {
        ok: true,
        data: Some(serde_json::json!({ "scan_kicked": true })),
        ..Default::default()
    }
}

/// Trigger an immediate push of all pending local-side changes. Fire-and-forget.
/// The model should follow up with `sync_status` / `drift_snapshot` to confirm completion.
async fn push_pending_op(app: &AppHandle) -> Response {
    let Some(eng) = engine(app).await else {
        return err("not connected — start a server in Rift first");
    };
    eng.force_push_now();
    crate::diagnostics::emit(
        crate::diagnostics::DiagStage::System,
        crate::diagnostics::DiagLevel::Info,
        "assistant tool: push_pending triggered",
    );
    Response {
        ok: true,
        data: Some(serde_json::json!({ "triggered": "push" })),
        ..Default::default()
    }
}

/// Trigger an immediate pull of all pending remote-side changes. Fire-and-forget.
async fn pull_pending_op(app: &AppHandle) -> Response {
    let Some(eng) = engine(app).await else {
        return err("not connected — start a server in Rift first");
    };
    eng.force_pull_now();
    crate::diagnostics::emit(
        crate::diagnostics::DiagStage::System,
        crate::diagnostics::DiagLevel::Info,
        "assistant tool: pull_pending triggered",
    );
    Response {
        ok: true,
        data: Some(serde_json::json!({ "triggered": "pull" })),
        ..Default::default()
    }
}

/// Apply both push and pull in sequence. Requires explicit `confirm: true` —
/// the dispatch arm gates on this before calling here. Fire-and-forget at the
/// engine level (both `force_*_now` fns are non-blocking).
async fn reconcile_apply_op(app: &AppHandle) -> Response {
    let Some(eng) = engine(app).await else {
        return err("not connected — start a server in Rift first");
    };
    eng.force_push_now();
    eng.force_pull_now();
    crate::diagnostics::emit(
        crate::diagnostics::DiagStage::System,
        crate::diagnostics::DiagLevel::Info,
        "assistant tool: reconcile_apply (push + pull) triggered",
    );
    Response {
        ok: true,
        data: Some(serde_json::json!({ "applied": true, "ops": ["push", "pull"] })),
        ..Default::default()
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

    // #41: RAII guard ensures the lock is released even if `exec_bash` panics or
    // the surrounding future is cancelled mid-await. The previous explicit
    // release call ran only on the normal-return path and leaked the lock on
    // any abnormal exit, permanently blocking every other user on this remote.
    let _lock_guard = if let Some(locks) = eng.locks() {
        if let Some(foreign) = locks.find_lock_by_other(&lock_key) {
            return err(format!(
                "remote shell is locked by {} on {} (since {}); retry shortly",
                foreign.user,
                foreign.host,
                foreign.since.to_rfc3339()
            ));
        }
        locks.acquire(&lock_key).await;
        Some(BridgeLockGuard { locks, key: lock_key.clone() })
    } else {
        None
    };

    let sftp = eng.sftp();
    let exec_result = sftp.exec_bash(&command, timeout).await;

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

/// Drop-released wrapper around a held remote-bash lock. On drop, spawns an
/// async task to call `release` so the lock survives panics and future
/// cancellation without leaking. See #41.
struct BridgeLockGuard {
    locks: std::sync::Arc<crate::sync::lock_presence::LockPresence>,
    key: String,
}

impl Drop for BridgeLockGuard {
    fn drop(&mut self) {
        let locks = self.locks.clone();
        let key = std::mem::take(&mut self.key);
        tokio::spawn(async move {
            locks.release(&key).await;
        });
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

