//! Local-LLM commands — test/list/context/optimize a self-hosted endpoint
//! (LiteLLM proxy speaking Anthropic /v1/messages, or raw Ollama for /api/*).
//! Split out of `oneshot.rs` (2026-06-27) — a self-contained cluster of
//! `#[tauri::command]`s; re-exported via `assistant/mod.rs` `pub use local_llm::*`
//! so the `commands::` paths in lib.rs stay unchanged.

use serde_json::Value;

use super::config::{
    is_valid_local_base_url, is_valid_local_model_name, load_config, save_config, CONFIG_WRITE_LOCK,
};

/// Read a response body, capped at 256KB. A hostile/misconfigured proxy could
/// stream an unbounded body into `.text()` and OOM us. Every probe's real body is
/// tiny (a short reply, a model list, an error), so 256KB is generous; surplus
/// bytes are dropped at the boundary. Decode lossily — these are JSON or
/// plain-text diagnostics, not exact binary.
async fn read_body_capped(resp: reqwest::Response) -> String {
    const BODY_CAP: usize = 256 * 1024;
    let mut resp = resp;
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < BODY_CAP {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let take = (BODY_CAP - buf.len()).min(chunk.len());
                buf.extend_from_slice(&chunk[..take]);
                if take < chunk.len() {
                    break;
                }
            }
            Ok(None) | Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Experimental: round-trip a one-line prompt through the configured local-LLM
/// endpoint so the Local LLM page can show a green/red "Test connection".
/// POSTs directly to `{base_url}/v1/messages` (the Anthropic API the CLI
/// targets) instead of spawning the CLI — a direct request surfaces the
/// upstream HTTP status + body verbatim, so an upstream 500 (e.g. LiteLLM
/// rejecting a `thinking` param Ollama can't honour) shows the real cause
/// instead of hanging behind a generic CLI timeout.
/// Returns the model's reply + output-token count on success, the upstream error
/// on failure. The renderer pairs `output_tokens` with its measured round-trip to
/// surface an approximate tok/s — the timing the user actually cares about.
#[derive(serde::Serialize)]
pub struct LocalTestResult {
    reply: String,
    output_tokens: Option<u64>,
}

#[tauri::command]
pub async fn assistant_test_local_llm() -> Result<LocalTestResult, String> {
    let cfg = load_config();
    let base_url = cfg
        .local_llm_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_valid_local_base_url(s))
        .ok_or("No (valid) base URL configured")?
        .trim_end_matches('/')
        .to_string();
    let model = cfg
        .local_llm_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_valid_local_model_name(s))
        .ok_or("No (valid) model configured")?
        .to_string();
    let local_key = crate::secrets::get(crate::secrets::LOCAL_LLM_API_KEY)
        .unwrap_or_else(|| "local".to_string());

    // Hit the Anthropic `/v1/messages` API directly. The CLI fires two parallel
    // calls and, on an upstream 500, hangs until our timeout — burying the real
    // cause behind a generic "timed out". A direct POST surfaces the upstream
    // status + body verbatim. Bounded at 20s so an unresponsive proxy fails
    // cleanly instead of hanging the spinner forever.
    let url = format!("{base_url}/v1/messages");
    // Include a `thinking` block: the spawned CLI sends one on EVERY real turn
    // (interleaved-thinking beta, no flag disables it), so a probe without it
    // would false-green — a plain request succeeds against proxies whose model
    // can't actually think, then every real turn 500s. Mimicking the real shape
    // makes the probe fail the same way real traffic does (e.g. LiteLLM →
    // `OllamaException - "qwen3-coder:30b" does not support thinking`).
    // Anthropic requires budget_tokens >= 1024 and max_tokens > budget_tokens.
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1280,
        "thinking": { "type": "enabled", "budget_tokens": 1024 },
        "messages": [{ "role": "user", "content": "Reply with exactly: OK" }],
    });

    let resp = reqwest::Client::new()
        .post(&url)
        .header("x-api-key", &local_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!("timed out after 20s reaching {url} — is the proxy up and responding?")
            } else if e.is_connect() {
                format!(
                    "can't connect to {url} — check the Base URL. It must point at a LiteLLM \
                     proxy speaking the Anthropic /v1/messages API; raw Ollama (:11434) won't work."
                )
            } else {
                format!("request to {url} failed: {e}")
            }
        })?;

    let status = resp.status();
    let text = read_body_capped(resp).await;
    if !status.is_success() {
        // Surface the upstream body (truncated) so the UI shows the real cause —
        // e.g. `OllamaException - "qwen3-coder:30b" does not support thinking`.
        let snippet: String = text.trim().chars().take(600).collect();
        return Err(if snippet.is_empty() {
            format!("proxy returned HTTP {}", status.as_u16())
        } else {
            format!("proxy returned HTTP {}: {snippet}", status.as_u16())
        });
    }

    // Anthropic Messages response: { "content": [ { "type": "text", "text": "OK" }, ... ],
    //   "usage": { "output_tokens": N } }
    let parsed = serde_json::from_str::<Value>(&text).ok();
    let reply = parsed
        .as_ref()
        .and_then(|v| {
            v.get("content")?.as_array().map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("")
            })
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let output_tokens = parsed
        .as_ref()
        .and_then(|v| v.get("usage")?.get("output_tokens")?.as_u64());

    Ok(LocalTestResult {
        reply: if reply.is_empty() { "(connected, empty reply)".to_string() } else { reply },
        output_tokens,
    })
}

/// Experimental: list the models the configured local endpoint advertises, so
/// the Local LLM page can offer a picker instead of free-text. GETs the
/// OpenAI-style `{base_url}/v1/models` (LiteLLM exposes it; the `/v1/messages`
/// adapter shares the same proxy). The key stays backend-side — only the model
/// id strings cross to the renderer. Returns [] (not an error) when the endpoint
/// is unreachable or advertises nothing, so the picker degrades to free-text.
#[tauri::command]
pub async fn assistant_list_local_models() -> Result<Vec<String>, String> {
    let cfg = load_config();
    let base_url = cfg
        .local_llm_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_valid_local_base_url(s))
        .ok_or("No (valid) base URL configured")?
        .trim_end_matches('/')
        .to_string();
    let local_key = crate::secrets::get(crate::secrets::LOCAL_LLM_API_KEY)
        .unwrap_or_else(|| "local".to_string());

    let url = format!("{base_url}/v1/models");
    let resp = reqwest::Client::new()
        .get(&url)
        .header("x-api-key", &local_key)
        .header("authorization", format!("Bearer {local_key}"))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("can't reach {url}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("{url} returned HTTP {}", resp.status().as_u16()));
    }

    let text = read_body_capped(resp).await;
    // OpenAI list shape: { "data": [ { "id": "ollama/qwen3-coder:30b" }, ... ] }.
    // Keep only ids that pass the same name guard the model field enforces.
    let models = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.get("data")?.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(Value::as_str))
                .filter(|id| is_valid_local_model_name(id))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(models)
}

/// Effective context window for the configured local model, probed via Ollama's
/// native `/api/show`. THE dominant local-mode failure: Ollama defaults `num_ctx`
/// to 4096 regardless of the model's true ceiling, silently truncating Rift's
/// system prompt + tools + open files mid-turn — the model loses its instructions
/// and the user's question, then stalls or refuses edits. This surfaces the gap
/// so the Local LLM page can warn + offer the one-click fix below.
#[derive(serde::Serialize)]
pub struct LocalCtxInfo {
    /// False when the endpoint isn't Ollama (e.g. a LiteLLM proxy with no
    /// `/api/show`) — the UI then skips the Ollama-specific guidance.
    is_ollama: bool,
    model: String,
    /// `num_ctx` set via a Modelfile PARAMETER. `None` = falls back to the
    /// server default (`OLLAMA_CONTEXT_LENGTH`, 4096 unless overridden) — the bug.
    num_ctx: Option<u64>,
    /// The model's architectural ceiling (e.g. 262144). `None` if `/api/show`
    /// didn't advertise one.
    max_ctx: Option<u64>,
    /// Model-card facts from `/api/show` `details` — the "what am I running"
    /// readout. All `None` for non-Ollama endpoints.
    params: Option<String>,
    quant: Option<String>,
    family: Option<String>,
}

/// `parameters` is a flat text blob (`num_ctx   32768\ntemperature  0.7\n…`).
fn parse_num_ctx(params: &str) -> Option<u64> {
    params.lines().find_map(|line| {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("num_ctx") => it.next().and_then(|n| n.parse::<u64>().ok()),
            _ => None,
        }
    })
}

#[tauri::command]
pub async fn assistant_local_model_context() -> Result<LocalCtxInfo, String> {
    let cfg = load_config();
    let base_url = cfg
        .local_llm_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_valid_local_base_url(s))
        .ok_or("No (valid) base URL configured")?
        .trim_end_matches('/')
        .to_string();
    let model = cfg
        .local_llm_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_valid_local_model_name(s))
        .ok_or("No (valid) model configured")?
        .to_string();

    let url = format!("{base_url}/api/show");
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({ "model": model }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                format!("can't reach {base_url} — is the endpoint up?")
            } else {
                format!("request to {url} failed: {e}")
            }
        })?;

    // Non-Ollama endpoints (LiteLLM proxy) have no `/api/show` → 404/405. Treat
    // as "not Ollama" rather than an error so the page degrades gracefully.
    if resp.status() == reqwest::StatusCode::NOT_FOUND
        || resp.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED
    {
        return Ok(LocalCtxInfo {
            is_ollama: false,
            model,
            num_ctx: None,
            max_ctx: None,
            params: None,
            quant: None,
            family: None,
        });
    }
    if !resp.status().is_success() {
        return Err(format!("/api/show returned HTTP {}", resp.status().as_u16()));
    }

    let text = read_body_capped(resp).await;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("bad /api/show JSON: {e}"))?;

    let num_ctx = v
        .get("parameters")
        .and_then(Value::as_str)
        .and_then(parse_num_ctx);
    // The arch prefix varies (`qwen3moe.context_length`, `llama.context_length`,
    // …) — take the first `*.context_length` key.
    let max_ctx = v.get("model_info").and_then(Value::as_object).and_then(|mi| {
        mi.iter()
            .find(|(k, _)| k.ends_with(".context_length"))
            .and_then(|(_, val)| val.as_u64())
    });

    // `details`: model card. `parameter_size` ("30.5B"), `quantization_level`
    // ("Q4_K_M"), `family` ("qwen3moe") — the "what am I running" readout.
    let details = v.get("details").and_then(Value::as_object);
    let card = |key: &str| {
        details
            .and_then(|d| d.get(key))
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|s| !s.is_empty())
    };

    Ok(LocalCtxInfo {
        is_ollama: true,
        model,
        num_ctx,
        max_ctx,
        params: card("parameter_size"),
        quant: card("quantization_level"),
        family: card("family"),
    })
}

/// One-click "Optimize for Rift": create an Ollama variant of the configured
/// model baking in a Rift-sized `num_ctx`, then repoint the config at it. This
/// is the fix for the 4096 truncation — Ollama can't take `num_ctx` per-request
/// over the Anthropic `/v1/messages` adapter, so a baked-in Modelfile variant is
/// the only lever Rift has. Variant name = `<base-without-tag>-rift` (idempotent:
/// re-running rebuilds it). `target_ctx` clamps to [8192, min(max, 131072)].
#[tauri::command]
pub async fn assistant_optimize_local_model(target_ctx: Option<u64>) -> Result<String, String> {
    let cfg = load_config();
    let base_url = cfg
        .local_llm_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_valid_local_base_url(s))
        .ok_or("No (valid) base URL configured")?
        .trim_end_matches('/')
        .to_string();
    let from = cfg
        .local_llm_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && is_valid_local_model_name(s))
        .ok_or("No (valid) model configured")?
        .to_string();

    // Strip any `:tag`, append `-rift` (skip if already a `-rift` variant).
    let stem = from.split(':').next().unwrap_or(&from);
    let variant = if stem.ends_with("-rift") {
        stem.to_string()
    } else {
        format!("{stem}-rift")
    };
    if !is_valid_local_model_name(&variant) {
        return Err(format!("derived variant name is invalid: {variant}"));
    }

    // Clamp into a sane band. 131072 caps the upper end so a 262k-ceiling model
    // doesn't allocate a KV cache that won't fit in VRAM.
    let target = target_ctx.unwrap_or(32768).clamp(8192, 131072);

    let url = format!("{base_url}/api/create");
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({
            "model": variant,
            "from": from,
            "parameters": { "num_ctx": target },
            "stream": false,
        }))
        // Create copies a manifest (blobs are shared) — usually fast, but bound
        // generously so a cold model pull doesn't trip the timeout.
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                format!("can't reach {base_url} — is Ollama up?")
            } else {
                format!("/api/create failed: {e}")
            }
        })?;

    let status = resp.status();
    let text = read_body_capped(resp).await;
    if !status.is_success() {
        let snippet: String = text.trim().chars().take(400).collect();
        return Err(if snippet.is_empty() {
            format!("/api/create returned HTTP {}", status.as_u16())
        } else {
            format!("/api/create HTTP {}: {snippet}", status.as_u16())
        });
    }
    // With `stream:false` Ollama returns a single `{"status":"success"}`; an
    // error object means the create didn't complete.
    let ok = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.get("status").and_then(Value::as_str).map(|s| s == "success"))
        .unwrap_or(false);
    if !ok {
        let snippet: String = text.trim().chars().take(400).collect();
        return Err(format!("/api/create did not report success: {snippet}"));
    }

    // Repoint config at the new variant so the next turn uses it.
    {
        let _guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let mut cfg = load_config();
        cfg.local_llm_model = Some(variant.clone());
        save_config(&cfg)?;
    }

    Ok(variant)
}
