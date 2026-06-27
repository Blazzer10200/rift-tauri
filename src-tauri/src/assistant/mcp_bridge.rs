//! UI-bridge MCP tools — `ask_user` / `open_browser` / `notify`. Each makes a
//! single loopback NDJSON round-trip to the parent Rift process over the
//! `RIFT_BRIDGE_PORT`/`_TOKEN` env injected by `mod::write_mcp_config`. Split out
//! of `mcp_server.rs` (2026-06-27) — self-contained, no shared state with the
//! filesystem tools; `handle_request` dispatches to these via `bridge_enabled()`.

use std::fmt::Write as _;
use std::io::{self, BufRead, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};

/// Whether the loopback bridge to the parent Rift process is reachable from
/// this MCP-child spawn (env injected by `mod::write_mcp_config`).
pub(super) fn bridge_enabled() -> bool {
    std::env::var("RIFT_BRIDGE_PORT").is_ok() && std::env::var("RIFT_BRIDGE_TOKEN").is_ok()
}

/// Socket timeout setters return Err on platforms / states where the option
/// can't be applied. Dropping that silently turns a misbehaving bridge socket
/// into an indefinite blocked read on the stdio thread (one MCP request stalls
/// every subsequent tool call) — log the breadcrumb.
pub(super) fn apply_bridge_timeouts(stream: &TcpStream, read: Duration, write: Duration, label: &str) {
    if let Err(e) = stream.set_read_timeout(Some(read)) {
        log::warn!("{label}: set_read_timeout failed: {e}");
    }
    if let Err(e) = stream.set_write_timeout(Some(write)) {
        log::warn!("{label}: set_write_timeout failed: {e}");
    }
}

/// Single round-trip to the parent's loopback bridge: one NDJSON request line
/// out, one response line back. `extra` fields are merged over the base
/// `{op, token, session_id}` envelope. `read_timeout` is per-op — `ask_user`
/// parks for minutes while the user decides; the fire-and-forget ops use
/// seconds.
pub(super) fn bridge_call(op: &str, extra: Value, read_timeout: Duration) -> Result<Value, String> {
    let bridge_t0 = std::time::Instant::now();
    let result = bridge_call_inner(op, extra, read_timeout);
    let bridge_dur_ms = bridge_t0.elapsed().as_millis() as u64;
    let bridge_ok = result.is_ok();
    {
        use crate::diagnostics::{DiagLevel, DiagStage};
        let level = if bridge_ok { DiagLevel::Info } else { DiagLevel::Warn };
        crate::diagnostics::emit_with_fields(
            DiagStage::Log, level, Some("bridge"), Some(file!()),
            "bridge round-trip",
            serde_json::json!({ "op": op, "dur_ms": bridge_dur_ms, "ok": bridge_ok }),
        );
    }
    result
}

fn bridge_call_inner(op: &str, extra: Value, read_timeout: Duration) -> Result<Value, String> {
    let port_s = std::env::var("RIFT_BRIDGE_PORT")
        .map_err(|_| "RIFT_BRIDGE_PORT not set on this MCP child".to_string())?;
    let token = std::env::var("RIFT_BRIDGE_TOKEN")
        .map_err(|_| "RIFT_BRIDGE_TOKEN not set on this MCP child".to_string())?;
    let port: u16 = port_s
        .parse()
        .map_err(|e| format!("invalid RIFT_BRIDGE_PORT `{port_s}`: {e}"))?;

    let mut req = json!({
        "op": op,
        "token": token,
        "session_id": std::env::var("RIFT_SESSION_ID").unwrap_or_default(),
        // #37: originating window so the bridge emits_to it, not app-wide.
        "window_label": std::env::var("RIFT_BRIDGE_WINDOW").unwrap_or_default(),
    });
    if let (Value::Object(base), Value::Object(extra)) = (&mut req, extra) {
        for (k, v) in extra {
            base.insert(k, v);
        }
    }

    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|e| format!("bridge addr parse: {e}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| format!("bridge connect: {e}"))?;
    apply_bridge_timeouts(&stream, read_timeout, Duration::from_secs(5), op);

    let payload = format!("{}\n", req);
    stream
        .write_all(payload.as_bytes())
        .map_err(|e| format!("bridge write: {e}"))?;
    stream.flush().map_err(|e| format!("bridge flush: {e}"))?;

    // Cap the response read — a bridge response (ask_user answer, ack) is small;
    // a Take guard bounds the allocation if bridge.rs ever emits a huge line.
    use std::io::Read as _;
    let mut reader = io::BufReader::new((&stream).take(64 * 1024));
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("bridge read: {e}"))?;
    if line.trim().is_empty() {
        return Err("bridge closed connection without a response".into());
    }
    let resp: Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("bridge parse: {e} (raw: {})", line.trim()))?;
    if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let msg = resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown bridge error");
        return Err(msg.to_string());
    }
    Ok(resp.get("data").cloned().unwrap_or(Value::Null))
}

/// Interactive question to the user via the chat UI. The parent emits an event
/// to the frontend and holds the TCP connection open until the user clicks an
/// answer (or 10-min timeout). 11-min read timeout here — 1 min headroom over
/// the parent's 10-min await so the parent times out first with a clean error.
pub(super) fn tool_ask_user(args: &Value) -> Result<String, String> {
    let questions = args
        .get("questions")
        .and_then(|v| v.as_array())
        .ok_or("missing `questions` array")?;
    if questions.is_empty() {
        return Err("`questions` must contain at least one question".into());
    }
    // RR10: bound the model-supplied payload before it crosses the loopback
    // bridge (the parent's read path buffers a whole line). The schema says
    // maxItems:4 but MCP does not enforce JSON Schema, so cap here.
    if questions.len() > 4 {
        return Err("too many questions (max 4)".into());
    }
    if serde_json::to_string(questions).map(|s| s.len()).unwrap_or(usize::MAX) > 16 * 1024 {
        return Err("questions payload too large (max 16 KiB)".into());
    }

    // 16 random bytes → 22-char base64url pending-request key. Collision
    // space is the in-flight set, never more than ~1 at a time per session.
    let mut id_bytes = [0u8; 16];
    rand::fill(&mut id_bytes);
    let request_id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id_bytes);

    let data = bridge_call(
        "ask_user",
        json!({ "request_id": request_id, "questions": questions }),
        Duration::from_secs(660),
    )?;
    Ok(format_ask_user_result(&data))
}

/// Turn the answer envelope from the frontend into a plain-text tool_result
/// Claude can read. Three shapes:
///   1. `{cancelled: true}` — user dismissed.
///   2. `{answers: [{question, answer}, ...]}` — normal path.
///   3. Anything else — serialize verbatim as JSON.
pub(super) fn format_ask_user_result(data: &Value) -> String {
    if data.get("cancelled").and_then(|v| v.as_bool()) == Some(true) {
        return "User dismissed the question without answering. Fall back to asking in plain text, or proceed with the most-likely-correct default and note your assumption.".into();
    }
    let Some(answers) = data.get("answers").and_then(|v| v.as_array()) else {
        return data.to_string();
    };
    if answers.is_empty() {
        return "User submitted an empty answer set.".into();
    }
    let mut out = String::new();
    for (i, a) in answers.iter().enumerate() {
        let q = a.get("question").and_then(|v| v.as_str()).unwrap_or("?");
        let ans_val = a.get("answer");
        let label = match ans_val {
            Some(Value::String(s)) => s.clone(),
            // Join multi-select labels with US (\x1F) so the frontend can split
            // unambiguously even when a label itself contains ", " (A1). The
            // model still reads it as readable prose (US renders as nothing).
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join("\u{1f}"),
            Some(other) => other.to_string(),
            None => "(no answer)".into(),
        };
        if i > 0 {
            out.push('\n');
        }
        let _ = write!(out, "Q: {q}\nA: {label}");
    }
    out
}

/// Show an http/https URL in Rift's in-app browser dock (the embedded webview
/// next to the chat). The parent validates the scheme and the frontend opens
/// the dock — this call returns as soon as the parent accepted the request.
pub(super) fn tool_open_browser(args: &Value) -> Result<String, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .ok_or("missing `url`")?;
    bridge_call("open_browser", json!({ "url": url }), Duration::from_secs(10))?;
    Ok(format!(
        "Opened {url} in Rift's in-app browser dock — the page is now visible to the user next to the chat."
    ))
}

/// Pop a toast notification in Rift's corner. Fire-and-forget presentation —
/// no user response comes back.
pub(super) fn tool_notify(args: &Value) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or("missing `title`")?;
    // RR10: a toast is bounded UI — cap the model-supplied strings (char
    // boundary safe) before they cross the bridge into the parent heap.
    let title: String = title.chars().take(200).collect();
    let mut extra = json!({ "title": title });
    if let Some(d) = args.get("detail").and_then(|v| v.as_str()) {
        let detail: String = d.chars().take(2000).collect();
        extra["detail"] = Value::from(detail);
    }
    if let Some(s) = args.get("severity").and_then(|v| v.as_str()) {
        extra["severity"] = Value::from(s);
    }
    bridge_call("notify", extra, Duration::from_secs(10))?;
    Ok("Notification shown to the user.".into())
}
