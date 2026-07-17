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

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures::StreamExt;
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

static SHIM: OnceLock<u16> = OnceLock::new();

/// For surfacing upstream trouble (429/5xx) to the UI — the CLI retries these
/// SILENTLY (observed live: Moonshot 429 → up to 10 retries, minutes of dead
/// air the user reads as "Rift is broken"), and this shim is the only place
/// Rift actually sees the response status.
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

/// (status, provider-route, last-emit) — re-emit only on change or after 10s so
/// a retry storm doesn't flood the frontend with toasts.
static LAST_STATUS: Mutex<Option<(u16, String, Instant)>> = Mutex::new(None);

/// Consecutive retryable failures (408/429/5xx) per upstream route. The CLI
/// retries these SILENTLY up to ~10 times (minutes of dead air on a saturated
/// endpoint); after GIVE_UP_AFTER in a row we convert the next one into a
/// non-retryable 400 so the turn ENDS with a visible error instead of hanging.
/// Any 2xx resets the route's count.
static FAIL_STREAK: LazyLock<Mutex<HashMap<String, u32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const GIVE_UP_AFTER: u32 = 5;

/// Tell the frontend an upstream /v1/messages call came back non-2xx. The FE
/// maps it onto whichever tab is streaming on that provider.
fn emit_upstream_status(status: u16, provider: Option<&str>, target: &str) {
    let Some(app) = APP.get() else { return };
    let route = provider.unwrap_or("").to_string();
    {
        let mut last = LAST_STATUS.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((s, r, at)) = last.as_ref() {
            if *s == status && *r == route && at.elapsed() < Duration::from_secs(10) {
                return;
            }
        }
        *last = Some((status, route.clone(), Instant::now()));
    }
    log::warn!("nothink shim: upstream {status} on /v1/messages ({target})");
    let _ = app.emit(
        "assistant://provider-upstream",
        serde_json::json!({ "status": status, "provider": provider }),
    );
}

/// One pooled HTTP client for all shim→upstream requests. `reqwest::Client::new()`
/// per request opened a fresh connection (no keep-alive) on every /v1/messages
/// call — pure handshake overhead on the hot streaming path. A shared client
/// reuses connections across turns. Built lazily so a TLS-init failure can't
/// panic at module load.
static UPSTREAM: OnceLock<reqwest::Client> = OnceLock::new();

fn upstream_client() -> &'static reqwest::Client {
    UPSTREAM.get_or_init(|| {
        reqwest::Client::builder()
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Loopback base URL (`http://127.0.0.1:<port>`) to hand the CLI as
/// `ANTHROPIC_BASE_URL`, or `None` until the listener has bound.
pub fn shim_base_url() -> Option<String> {
    SHIM.get().map(|p| format!("http://127.0.0.1:{p}"))
}

/// Bind the shim listener exactly once. Idempotent. Non-fatal on failure —
/// turn.rs falls back to the raw base URL (i.e. the external proxy is still
/// expected) when this is `None`.
pub async fn start(app: tauri::AppHandle) -> Result<u16, String> {
    let _ = APP.set(app);
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
                // Fail loud on an unparseable value instead of silently coercing
                // to 0 — a 0 would truncate the forwarded body to empty and POST
                // an incomplete /v1/messages upstream, bypassing the short-body
                // guard below (which only runs once content_length is trusted).
                // Matches the >64 MiB arm's error treatment.
                content_length = value.parse().map_err(|_| {
                    format!("invalid Content-Length header: {value:?}")
                })?;
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
    // A short body (peer closed mid-send) must be an error — truncate() is a
    // no-op below content_length, so without this a truncated JSON body would
    // be forwarded upstream as if complete.
    if body.len() < content_length {
        return Err(format!(
            "client closed mid-body ({}/{content_length} bytes)",
            body.len()
        ));
    }
    body.truncate(content_length);

    Ok(ParsedReq { method, path, headers, body })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

async fn handle_conn(mut stream: TcpStream) -> Result<(), String> {
    // Bound the request read — a stalled peer would otherwise park this
    // spawned task forever (no socket timeouts anywhere on this path).
    let req = tokio::time::timeout(Duration::from_secs(30), read_request(&mut stream))
        .await
        .map_err(|_| "request read timed out (30s)".to_string())??;

    // Resolve the upstream target fresh each request — profiles/the Base-URL
    // setter can change mid-session. Three callers point the CLI here (turn.rs):
    //   • provider turn (per-tab route) → `/p/<id>/…` names the profile; we
    //     resolve THAT provider's base per request, so two panes streaming on
    //     DIFFERENT providers stay independent;
    //   • legacy local mode (bare path) → the global wire fields;
    //   • cloud "thinking off" (bare path) → forward to Anthropic (the injected
    //     `thinking:{disabled}` is the only switch that suppresses extended
    //     thinking, since the CLI always sends a thinking block).
    let (target, fwd_path, provider_route) = if let Some(rest) = req.path.strip_prefix("/p/") {
        let (id, tail) = match rest.split_once('/') {
            Some((id, tail)) => (id, format!("/{tail}")),
            None => (rest, "/".to_string()),
        };
        let cfg = super::config::load_config();
        match cfg
            .providers
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.base_url.trim().trim_end_matches('/').to_string())
            .filter(|b| super::config::is_valid_local_base_url(b))
        {
            Some(b) => (b, tail, Some(id.to_string())),
            None => {
                return write_plain(
                    &mut stream,
                    502,
                    &format!("nothink shim: unknown provider route: {id}"),
                )
                .await;
            }
        }
    } else {
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
                .filter(|s| !s.trim().is_empty() && super::config::is_valid_local_base_url(s.trim()))
                .unwrap_or_else(|| "https://api.anthropic.com".to_string())
                .trim_end_matches('/')
                .to_string()
        };
        (target, req.path.clone(), None)
    };

    let is_messages = req.method.eq_ignore_ascii_case("POST")
        && fwd_path.split(['?', '#']).next().unwrap_or("") == "/v1/messages";

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

    let url = format!("{target}{fwd_path}");
    let mut rb = upstream_client().request(
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

    // send() resolves on response HEADERS (SSE body streams after) — 180s
    // covers a queued/slow first response without letting the task leak.
    let upstream = match tokio::time::timeout(Duration::from_secs(180), rb.send()).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            return write_plain(&mut stream, 502, &format!("nothink shim: upstream error: {e}"))
                .await;
        }
        Err(_) => {
            return write_plain(&mut stream, 504, "nothink shim: upstream response timed out (180s)")
                .await;
        }
    };

    // Status line.
    let status = upstream.status();
    // Surface upstream trouble the CLI would swallow: auth failures, rate
    // limits, and server errors on the messages endpoint. 429 especially — the
    // CLI silently retries for minutes and the UI would otherwise show nothing.
    let code = status.as_u16();
    if is_messages && (matches!(code, 401 | 403 | 408 | 429) || code >= 500) {
        emit_upstream_status(code, provider_route.as_deref(), &target);
    }
    // Give-up guard: after GIVE_UP_AFTER consecutive retryable responses on
    // this route, answer the CLI with a 400 (non-retryable) carrying an honest
    // message — the turn ends with a visible error instead of silent retry
    // dead-air. Streak resets here so a manual retry gets a fresh window.
    if is_messages {
        let key = provider_route.clone().unwrap_or_else(|| target.clone());
        let retryable = matches!(code, 408 | 429) || code >= 500;
        let give_up = {
            let mut m = FAIL_STREAK.lock().unwrap_or_else(|p| p.into_inner());
            if retryable {
                let n = m.entry(key.clone()).or_insert(0);
                *n += 1;
                let hit = *n >= GIVE_UP_AFTER;
                if hit {
                    m.remove(&key);
                }
                hit
            } else {
                if status.is_success() {
                    m.remove(&key);
                }
                false
            }
        };
        if give_up {
            let who = provider_route.as_deref().unwrap_or("the upstream");
            log::warn!("nothink shim: {key} returned {code} {GIVE_UP_AFTER}x in a row — ending the turn");
            return write_anthropic_error(
                &mut stream,
                &format!(
                    "Rift: {who} endpoint returned {code} {GIVE_UP_AFTER} times in a row — ending this turn instead of retrying silently. Wait a minute, then send again."
                ),
            )
            .await;
        }
    }
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

    // Stream the (decoded) body straight through, byte-for-byte. Per-chunk gap
    // timeout (not total — a legit SSE turn runs for minutes) so a wedged
    // upstream can't park this task forever.
    let mut bytes = upstream.bytes_stream();
    loop {
        let Some(chunk) = tokio::time::timeout(Duration::from_secs(300), bytes.next())
            .await
            .map_err(|_| "upstream stream idle >300s".to_string())?
        else {
            break;
        };
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

/// A 400 in Anthropic's error envelope — the ONE status class the CLI treats
/// as terminal (4xx-not-408/429), so the turn surfaces the message and stops.
async fn write_anthropic_error(stream: &mut TcpStream, msg: &str) -> Result<(), String> {
    let body = serde_json::json!({
        "type": "error",
        "error": { "type": "invalid_request_error", "message": msg },
    })
    .to_string();
    let head = format!(
        "HTTP/1.1 400 Bad Request\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(body.as_bytes()).await.map_err(|e| e.to_string())?;
    let _ = stream.shutdown().await;
    Ok(())
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
