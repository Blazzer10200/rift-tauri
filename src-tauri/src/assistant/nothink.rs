//! In-process "no-think" loopback shim — local-LLM mode + cloud "thinking off".
//!
//! Replaces the external `tools/rift-nothink-proxy.mjs` (Node, :11435→:11434).
//! Ollama forces thinking ON; the ONLY switch that suppresses it on the
//! `/v1/messages` API is a `"thinking":{"type":"disabled"}` block in the body
//! (top-level `think:false` does nothing). The Claude CLI sends a `thinking`
//! block on every real turn (interleaved-thinking beta, no flag disables it),
//! so without this rewrite Qwen-class models emit a ~700-token reasoning dump
//! before every tool call / reply (measured: 11.4s → 2.0s per turn, 5.6×).
//!
//! This is a TRANSPARENT forwarder, NOT a protocol transform (unlike the old
//! LiteLLM bridge): it only rewrites the `thinking` field on POST /v1/messages
//! and pipes everything else — including the SSE stream — straight through, so
//! Ollama's native Anthropic-protocol rendering (incl. tool calls) is unaffected.
//!
//! Wiring: bound once at boot on `127.0.0.1:<random-port>`; turn.rs points the
//! CLI's `ANTHROPIC_BASE_URL` at this shim instead of the real base URL in two
//! cases — local mode (always), and cloud mode when the user toggles thinking
//! OFF. The upstream is resolved fresh per-request: local endpoint in local
//! mode, else Anthropic (`RIFT_CLOUD_UPSTREAM`, default api.anthropic.com).
//! Default cloud (thinking on) never sets ANTHROPIC_BASE_URL → byte-identical.

use std::sync::OnceLock;
use std::time::Duration;

use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

static SHIM: OnceLock<u16> = OnceLock::new();

/// Loopback base URL (`http://127.0.0.1:<port>`) to hand the CLI as
/// `ANTHROPIC_BASE_URL`, or `None` until the listener has bound.
pub fn shim_base_url() -> Option<String> {
    SHIM.get().map(|p| format!("http://127.0.0.1:{p}"))
}

/// Bind the shim listener exactly once. Idempotent. Non-fatal on failure —
/// turn.rs falls back to the raw base URL (i.e. the external proxy is still
/// expected) when this is `None`.
pub async fn start() -> Result<u16, String> {
    if let Some(p) = SHIM.get() {
        return Ok(*p);
    }
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("nothink shim bind: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("nothink shim local_addr: {e}"))?
        .port();
    if SHIM.set(port).is_err() {
        return SHIM
            .get()
            .copied()
            .ok_or_else(|| "nothink shim OnceLock race-loss".to_string());
    }
    log::info!("assistant nothink shim: listening on 127.0.0.1:{port}");

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(stream).await {
                            log::debug!("nothink shim conn closed: {e}");
                        }
                    });
                }
                Err(e) => {
                    log::warn!("nothink shim accept: {e}");
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            }
        }
    });

    Ok(port)
}

/// Parsed HTTP/1.1 request head + body. The CLI talks HTTP/1.1 to its base URL,
/// so we only need a minimal Content-Length-framed parser here (no chunked
/// request bodies — Anthropic clients always send a sized JSON body).
struct ParsedReq {
    method: String,
    path: String,
    /// (name, value) preserving order; we rewrite content-length when the body
    /// changes and drop hop-by-hop headers when forwarding.
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

async fn read_request(stream: &mut TcpStream) -> Result<ParsedReq, String> {
    // Read until end-of-headers (\r\n\r\n), keeping any body bytes already read.
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        let n = stream.read(&mut tmp).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("client closed before headers complete".into());
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > 1 << 20 {
            return Err("request headers exceed 1 MiB".into());
        }
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or("empty request")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("no method")?.to_string();
    let path = parts.next().ok_or("no path")?.to_string();

    let mut headers = Vec::new();
    let mut content_length = 0usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
                // RR10: bound the body alloc — a misbehaving/garbled CLI sending a
                // huge Content-Length would otherwise pre-grow `body` unbounded.
                if content_length > 64 * 1024 * 1024 {
                    return Err(format!("Content-Length {content_length} exceeds 64 MiB cap"));
                }
            }
            headers.push((name, value));
        }
    }

    // Body: bytes already past the header terminator, then read the remainder.
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);

    Ok(ParsedReq { method, path, headers, body })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

async fn handle_conn(mut stream: TcpStream) -> Result<(), String> {
    let req = read_request(&mut stream).await?;

    // Resolve the upstream target fresh each request — the Base-URL setter can
    // change it mid-session. Two callers point the CLI here (turn.rs):
    //   • local mode → forward to the configured local endpoint;
    //   • cloud "thinking off" → forward to Anthropic (the injected
    //     `thinking:{disabled}` is the only switch that suppresses extended
    //     thinking, since the CLI always sends a thinking block).
    let cfg = super::config::load_config();
    let target = if cfg.local_llm_enabled {
        match cfg
            .local_llm_base_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && super::config::is_valid_local_base_url(s))
        {
            Some(t) => t.trim_end_matches('/').to_string(),
            None => {
                return write_plain(&mut stream, 502, "nothink shim: no valid local_llm_base_url").await;
            }
        }
    } else {
        std::env::var("RIFT_CLOUD_UPSTREAM")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "https://api.anthropic.com".to_string())
            .trim_end_matches('/')
            .to_string()
    };

    let is_messages = req.method.eq_ignore_ascii_case("POST")
        && req.path.split(['?', '#']).next().unwrap_or("") == "/v1/messages";

    // Rewrite the body's `thinking` field to disabled (the one switch that works
    // on Ollama's /v1/messages). Non-JSON or unparseable → forward unchanged.
    let mut body = req.body;
    if is_messages && !body.is_empty() {
        if let Ok(mut j) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(obj) = j.as_object_mut() {
                obj.insert(
                    "thinking".into(),
                    serde_json::json!({ "type": "disabled" }),
                );
            }
            // The CLI also ships a `clear_thinking_*` context-management strategy,
            // which 400s ("requires thinking to be enabled or adaptive") once we
            // force thinking off. Strip any clear_thinking edit from the body.
            strip_clear_thinking(&mut j);
            if let Ok(reser) = serde_json::to_vec(&j) {
                body = reser;
            }
        }
    }

    let url = format!("{target}{}", req.path);
    let mut rb = reqwest::Client::new().request(
        reqwest::Method::from_bytes(req.method.as_bytes())
            .map_err(|e| format!("bad method: {e}"))?,
        &url,
    );
    // Forward request headers verbatim except Host (reqwest sets it from the URL)
    // and Content-Length (the body length may have changed after the rewrite).
    for (name, value) in &req.headers {
        if name.eq_ignore_ascii_case("host") || name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        rb = rb.header(name, value);
    }
    rb = rb.body(body);

    let upstream = match rb.send().await {
        Ok(r) => r,
        Err(e) => {
            return write_plain(&mut stream, 502, &format!("nothink shim: upstream error: {e}"))
                .await;
        }
    };

    // Status line.
    let status = upstream.status();
    let reason = status.canonical_reason().unwrap_or("");
    let mut head = format!("HTTP/1.1 {} {}\r\n", status.as_u16(), reason);
    // Pass response headers through, dropping framing headers we re-derive:
    // upstream may send Content-Length OR Transfer-Encoding: chunked; we stream
    // the decoded body and close the connection to delimit it, so we strip both
    // and signal close. (SSE is the common case — Connection: close is fine.)
    for (name, value) in upstream.headers() {
        let n = name.as_str();
        if n.eq_ignore_ascii_case("content-length")
            || n.eq_ignore_ascii_case("transfer-encoding")
            || n.eq_ignore_ascii_case("connection")
        {
            continue;
        }
        if let Ok(v) = value.to_str() {
            head.push_str(&format!("{n}: {v}\r\n"));
        }
    }
    head.push_str("connection: close\r\n\r\n");
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;

    // Stream the (decoded) body straight through, byte-for-byte.
    let mut bytes = upstream.bytes_stream();
    while let Some(chunk) = bytes.next().await {
        let chunk = chunk.map_err(|e| format!("upstream stream: {e}"))?;
        stream
            .write_all(&chunk)
            .await
            .map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;
    }
    let _ = stream.shutdown().await;
    Ok(())
}

/// Recursively remove any `clear_thinking*` context-management strategy from a
/// /v1/messages body. The CLI emits `context_management.edits[{type:
/// "clear_thinking_20251015", …}]`; that strategy requires thinking to be
/// enabled/adaptive and 400s once we force `thinking:{disabled}`. We walk the
/// whole value (structure-agnostic) and drop array elements whose `type` starts
/// with `clear_thinking`, then prune any `edits`/`context_management` left empty.
fn strip_clear_thinking(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Array(arr) => {
            arr.retain(|el| {
                !el.get("type")
                    .and_then(|t| t.as_str())
                    .is_some_and(|t| t.starts_with("clear_thinking"))
            });
            for el in arr.iter_mut() {
                strip_clear_thinking(el);
            }
        }
        serde_json::Value::Object(obj) => {
            for val in obj.values_mut() {
                strip_clear_thinking(val);
            }
            obj.retain(|k, val| match k.as_str() {
                "edits" => !val.as_array().is_some_and(|a| a.is_empty()),
                // Drop context_management if its edits list is now empty OR the
                // object itself is empty (its only `edits` key was just pruned
                // above, since values recurse before this retain runs).
                "context_management" => !val.as_object().is_some_and(|o| {
                    o.is_empty()
                        || matches!(o.get("edits").and_then(|e| e.as_array()), Some(a) if a.is_empty())
                }),
                _ => true,
            });
        }
        _ => {}
    }
}

async fn write_plain(stream: &mut TcpStream, code: u16, msg: &str) -> Result<(), String> {
    let body = msg.as_bytes();
    let head = format!(
        "HTTP/1.1 {code} Bad Gateway\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(body).await.map_err(|e| e.to_string())?;
    let _ = stream.shutdown().await;
    Ok(())
}
