//! Native OpenAI Responses API provider.
//!
//! The permanent API key stays in the OS keychain and is only read by Rust.
//! Streaming Responses events are translated into Rift's existing internal
//! assistant envelopes, so the renderer remains provider-agnostic while the
//! older Claude wire decoder is incrementally retired.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, LazyLock, Mutex,
};
use std::time::Duration;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use super::{AssistantAttachment, PermissionRegistry};

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MODELS_URL: &str = "https://api.openai.com/v1/models";
const DEFAULT_MODEL: &str = "gpt-5.6";
const MAX_SSE_FRAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;
const MAX_TOOL_ROUNDS: usize = 16;
const MAX_WRITE_BYTES: usize = 8 * 1024 * 1024;
const MAX_COMMAND_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const STREAM_EVENT: &str = "assistant://stream";
const DONE_EVENT: &str = "assistant://done";
const PERMISSION_EVENT: &str = "assistant://permission-request";

#[derive(Debug, Clone)]
struct FunctionCall {
    call_id: String,
    name: String,
    arguments: String,
}

#[derive(Default)]
struct UsageTotals {
    input: u64,
    output: u64,
    cached: u64,
}

struct ResponseRound {
    response: Value,
    calls: Vec<FunctionCall>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiStatus {
    pub api_key_configured: bool,
    pub env_api_key_present: bool,
    pub ready: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiModel {
    pub id: String,
    pub label: String,
    pub family: String,
    pub context_window: Option<u64>,
    pub reasoning: bool,
    pub image_input: bool,
    pub available: bool,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelObject>,
}

#[derive(Debug, Deserialize)]
struct ModelObject {
    id: String,
}

fn current_api_key() -> Option<String> {
    crate::secrets::get(crate::secrets::OPENAI_API_KEY)
}

#[tauri::command]
pub fn assistant_openai_status() -> Result<OpenAiStatus, String> {
    let configured = current_api_key().is_some();
    let env_present = std::env::var_os("OPENAI_API_KEY").is_some();
    Ok(OpenAiStatus {
        api_key_configured: configured,
        env_api_key_present: env_present,
        ready: configured,
        summary: if configured {
            "OpenAI API key configured".into()
        } else {
            "Add an OpenAI API key in Settings to use GPT models".into()
        },
    })
}

#[tauri::command]
pub fn assistant_set_openai_api_key(api_key: Option<String>) -> Result<(), String> {
    let value = api_key.as_deref().map(str::trim).filter(|s| !s.is_empty());
    match value {
        Some(key) => {
            if key.len() < 20 || key.len() > 512 || key.chars().any(char::is_whitespace) {
                return Err("OpenAI API key format is invalid".into());
            }
            crate::secrets::set(crate::secrets::OPENAI_API_KEY, key)
        }
        None => crate::secrets::delete(crate::secrets::OPENAI_API_KEY),
    }
}

fn curated_models() -> Vec<OpenAiModel> {
    [
        ("gpt-5.6", "GPT-5.6", "gpt-5.6", 1_050_000),
        ("gpt-5.6-sol", "GPT-5.6 Sol", "gpt-5.6", 1_050_000),
        ("gpt-5.6-terra", "GPT-5.6 Terra", "gpt-5.6", 1_050_000),
        ("gpt-5.6-luna", "GPT-5.6 Luna", "gpt-5.6", 1_050_000),
        ("gpt-5.3-codex", "GPT-5.3 Codex", "gpt-5.3", 400_000),
    ]
    .into_iter()
    .map(|(id, label, family, context)| OpenAiModel {
        id: id.into(),
        label: label.into(),
        family: family.into(),
        context_window: Some(context),
        reasoning: true,
        image_input: true,
        available: false,
    })
    .collect()
}

#[tauri::command]
pub async fn assistant_openai_list_models() -> Result<Vec<OpenAiModel>, String> {
    let Some(key) = current_api_key() else {
        return Ok(curated_models());
    };
    let resp = crate::certs::usage_client()
        .get(MODELS_URL)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| format!("OpenAI model lookup failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = super::read_body_capped(resp, MAX_ERROR_BODY_BYTES).await;
        return Err(openai_http_error(status.as_u16(), &body));
    }
    let listed = resp
        .json::<ModelsResponse>()
        .await
        .map_err(|e| format!("OpenAI model list was unreadable: {e}"))?;
    let available: HashSet<String> = listed.data.into_iter().map(|m| m.id).collect();
    let mut out = curated_models();
    for model in &mut out {
        model.available = available.contains(&model.id);
    }
    // Keep account-visible GPT chat/reasoning models discoverable even before
    // Rift has richer capability metadata for a brand-new snapshot.
    let known: HashSet<String> = out.iter().map(|m| m.id.clone()).collect();
    for id in available
        .iter()
        .filter(|id| id.starts_with("gpt-") && !known.contains(*id))
    {
        out.push(OpenAiModel {
            id: id.clone(),
            label: id.clone(),
            family: id.split('-').take(3).collect::<Vec<_>>().join("-"),
            context_window: None,
            reasoning: true,
            image_input: true,
            available: true,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

type SessionEntry = (u64, CancellationToken);
static NEXT_TURN: AtomicU64 = AtomicU64::new(1);
static ACTIVE: LazyLock<Mutex<HashMap<String, SessionEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct ActiveGuard {
    session_id: String,
    generation: u64,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        let mut active = ACTIVE.lock().unwrap_or_else(|p| p.into_inner());
        if active.get(&self.session_id).map(|(g, _)| *g) == Some(self.generation) {
            active.remove(&self.session_id);
        }
    }
}

/// Cancel an active OpenAI response stream. Returns false when the session is
/// not owned by this provider, allowing `assistant_stop` to continue to Claude.
pub(super) fn cancel_session(session_id: &str) -> bool {
    let token = ACTIVE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(session_id)
        .map(|(_, token)| token);
    if let Some(token) = token {
        token.cancel();
        true
    } else {
        false
    }
}

fn register_session(session_id: &str) -> (CancellationToken, ActiveGuard) {
    let generation = NEXT_TURN.fetch_add(1, Ordering::Relaxed);
    let token = CancellationToken::new();
    let old = ACTIVE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(session_id.to_string(), (generation, token.clone()));
    if let Some((_, old_token)) = old {
        old_token.cancel();
    }
    (
        token,
        ActiveGuard {
            session_id: session_id.to_string(),
            generation,
        },
    )
}

/// Preserve Responses items exactly as the API returned them. Reasoning and
/// compaction items are opaque continuation state, not display text; reducing
/// them to chat bubbles breaks stateless (`store:false`) multi-turn runs.
fn validated_history(history: Vec<Value>) -> Result<Vec<Value>, String> {
    if history.len() > 4_000 {
        return Err("conversation history is too large to send safely".into());
    }
    let mut total = 0usize;
    let mut out = Vec::with_capacity(history.len());
    for item in history {
        if !item.is_object() {
            return Err("OpenAI conversation state contained an invalid item".into());
        }
        total = total.saturating_add(
            serde_json::to_vec(&item)
                .map_err(|_| "OpenAI conversation state could not be encoded")?
                .len(),
        );
        if total > 8 * 1024 * 1024 {
            return Err("conversation history exceeds the 8 MB safety limit".into());
        }
        out.push(item);
    }
    Ok(out)
}

fn reasoning_effort(enabled: bool, tier: Option<&str>) -> &'static str {
    if !enabled {
        return "none";
    }
    match tier {
        Some("none") => "low",
        Some("smart") => "medium",
        Some("ultra") => "xhigh",
        _ => "high",
    }
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn assistant_openai_send(
    app: AppHandle,
    window: tauri::Window,
    prompt: String,
    session_id: String,
    model: Option<String>,
    attachments: Option<Vec<AssistantAttachment>>,
    history: Option<Vec<Value>>,
    thinking_effort: Option<String>,
    thinking_enabled: Option<bool>,
    turn_epoch: Option<u64>,
    permission_mode: Option<String>,
    root: Option<String>,
) -> Result<(), String> {
    if !super::is_valid_session_id(&session_id) {
        return Err(format!(
            "invalid session_id: must be a UUID (got {} chars)",
            session_id.len()
        ));
    }
    let attachments = attachments.unwrap_or_default();
    super::turn::validate_attachments(&attachments)?;
    if prompt.trim().is_empty() && attachments.is_empty() {
        return Err("empty OpenAI message".into());
    }
    let key = current_api_key().ok_or_else(|| {
        "OpenAI isn't configured — add an API key in Settings → Providers".to_string()
    })?;
    let model = model.unwrap_or_else(|| DEFAULT_MODEL.into());
    if !super::is_valid_model_name(&model) {
        return Err("invalid OpenAI model id".into());
    }
    let cfg = super::config::load_config();
    let permission_mode = permission_mode
        .filter(|mode| super::config::is_valid_permission_mode(mode))
        .or(cfg.permission_mode.clone())
        .filter(|mode| super::config::is_valid_permission_mode(mode))
        .unwrap_or_else(|| "bypassPermissions".into());
    let trust_level = super::config::effective_trust_level(&cfg.trust_level);
    let roots = resolve_roots(root.as_deref(), cfg.current_root.as_deref())?;
    let epoch = turn_epoch.unwrap_or(0);
    let window_label = window.label().to_string();
    let mut input = validated_history(history.unwrap_or_default())?;
    let mut content = vec![json!({ "type": "input_text", "text": prompt })];
    for attachment in attachments {
        content.push(json!({
            "type": "input_image",
            "image_url": format!("data:{};base64,{}", attachment.mime, attachment.data_base64),
        }));
    }
    input.push(json!({ "role": "user", "content": content }));
    let tools = openai_tools(&trust_level);
    let instructions = provider_instructions(&roots, &permission_mode);
    let (cancel, _guard) = register_session(&session_id);
    let mut totals = UsageTotals::default();
    let mut init_emitted = false;

    for _round in 0..MAX_TOOL_ROUNDS {
        if cancel.is_cancelled() {
            emit_done(&app, &window_label, &session_id, epoch, -1);
            return Ok(());
        }
        let body = json!({
            "model": model,
            "instructions": instructions,
            "input": input,
            "tools": tools,
            "parallel_tool_calls": false,
            // Stateless/ZDR-safe continuation: GPT-5.6 returns encrypted
            // reasoning items, and replaying the response output below keeps
            // reasoning intact across function-call rounds without server-side
            // response storage.
            "include": ["reasoning.encrypted_content"],
            "stream": true,
            "store": false,
            // Server-side compaction stays compatible with `store:false`. The
            // returned opaque compaction item is persisted with the rest of the
            // local continuation state and automatically reduces the next input.
            "context_management": [{ "type": "compaction", "compact_threshold": 180_000 }],
            "reasoning": {
                "effort": reasoning_effort(thinking_enabled.unwrap_or(false), thinking_effort.as_deref())
            }
        });
        let round = match stream_response(
            &app,
            &window_label,
            &session_id,
            epoch,
            &model,
            &key,
            &body,
            &cancel,
            &mut init_emitted,
        )
        .await
        {
            Ok(round) => round,
            Err(_) if cancel.is_cancelled() => {
                emit_done(&app, &window_label, &session_id, epoch, -1);
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let round_usage = usage_snapshot(&round.response);
        add_usage(&mut totals, &round.response);

        if round.calls.is_empty() {
            let mut continuation = input;
            if let Some(output) = round.response.get("output").and_then(Value::as_array) {
                continuation.extend(output.iter().cloned());
            }
            prune_before_latest_compaction(&mut continuation);
            emit_usage_and_result(
                &app,
                &window_label,
                &session_id,
                epoch,
                &model,
                &round.response,
                &round_usage,
                &totals,
                &continuation,
            );
            emit_done(&app, &window_label, &session_id, epoch, 0);
            return Ok(());
        }

        if let Some(output) = round.response.get("output").and_then(Value::as_array) {
            input.extend(output.iter().cloned());
        }
        for call in round.calls {
            emit_tool_use(&app, &window_label, &session_id, epoch, &call);
            let args = serde_json::from_str::<Value>(&call.arguments).unwrap_or_else(|_| json!({}));
            let result = execute_tool(
                &app,
                &window_label,
                &session_id,
                &call,
                &args,
                &roots,
                &trust_level,
                &permission_mode,
                &cancel,
            )
            .await;
            if cancel.is_cancelled() {
                emit_done(&app, &window_label, &session_id, epoch, -1);
                return Ok(());
            }
            let (output, is_error) = match result {
                Ok(text) => (text, false),
                Err(message) => (message, true),
            };
            emit_tool_result(
                &app,
                &window_label,
                &session_id,
                epoch,
                &call.call_id,
                &output,
                is_error,
            );
            input.push(json!({
                "type": "function_call_output",
                "call_id": call.call_id,
                "output": output,
            }));
        }
    }

    Err(format!(
        "OpenAI exceeded Rift's {MAX_TOOL_ROUNDS}-round tool safety limit"
    ))
}

#[allow(clippy::too_many_arguments)]
async fn stream_response(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    requested_model: &str,
    key: &str,
    body: &Value,
    cancel: &CancellationToken,
    init_emitted: &mut bool,
) -> Result<ResponseRound, String> {
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err("OpenAI turn cancelled".into()),
        response = crate::certs::api_client()
            .post(RESPONSES_URL)
            .bearer_auth(key)
            .json(body)
            .send() => response.map_err(|e| format!("OpenAI request failed: {e}"))?,
    };
    let status = response.status();
    if !status.is_success() {
        let body = super::read_body_capped(response, MAX_ERROR_BODY_BYTES).await;
        return Err(openai_http_error(status.as_u16(), &body));
    }

    let mut decoder = SseDecoder::default();
    let mut stream = response.bytes_stream();
    let mut completed = None;
    let mut thinking_open = false;
    while let Some(next) = tokio::select! {
        _ = cancel.cancelled() => return Err("OpenAI turn cancelled".into()),
        next = stream.next() => next,
    } {
        let chunk = next.map_err(|e| format!("OpenAI response stream failed: {e}"))?;
        for data in decoder.push(&chunk)? {
            if data == "[DONE]" {
                continue;
            }
            let event: Value = serde_json::from_str(&data)
                .map_err(|e| format!("OpenAI sent an unreadable stream event: {e}"))?;
            match event.get("type").and_then(Value::as_str).unwrap_or("") {
                "response.created" if !*init_emitted => {
                    let response = event.get("response").cloned().unwrap_or(Value::Null);
                    let actual_model = response
                        .get("model")
                        .and_then(Value::as_str)
                        .unwrap_or(requested_model);
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type": "system", "subtype": "init", "model": actual_model,
                            "provider": "openai",
                        }),
                    );
                    *init_emitted = true;
                }
                "response.output_text.delta" | "response.refusal.delta" => {
                    if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                        emit_text_delta(app, window, session, epoch, delta);
                    }
                }
                "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                    if !thinking_open {
                        emit_line(
                            app,
                            window,
                            session,
                            epoch,
                            json!({
                                "type": "stream_event",
                                "event": { "type": "content_block_start", "index": 1,
                                    "content_block": { "type": "thinking", "thinking": "" } }
                            }),
                        );
                        thinking_open = true;
                    }
                    if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                        emit_line(
                            app,
                            window,
                            session,
                            epoch,
                            json!({
                                "type": "stream_event",
                                "event": { "type": "content_block_delta", "index": 1,
                                    "delta": { "type": "thinking_delta", "thinking": delta } }
                            }),
                        );
                    }
                }
                "response.completed" => {
                    if thinking_open {
                        emit_thinking_stop(app, window, session, epoch);
                        thinking_open = false;
                    }
                    completed = Some(event.get("response").cloned().unwrap_or(Value::Null));
                }
                "response.failed" | "response.incomplete" | "error" => {
                    return Err(openai_event_error(&event));
                }
                _ => {}
            }
        }
    }
    let response =
        completed.ok_or_else(|| "OpenAI stream ended before response.completed".to_string())?;
    let calls = response
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
        .map(|item| FunctionCall {
            call_id: item
                .get("call_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            name: item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            arguments: item
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}")
                .to_string(),
        })
        .filter(|call| !call.call_id.is_empty() && !call.name.is_empty())
        .collect();
    Ok(ResponseRound { response, calls })
}

fn resolve_roots(
    explicit: Option<&str>,
    configured: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let selected = explicit
        .map(PathBuf::from)
        .or_else(|| configured.map(Path::to_path_buf));
    let Some(selected) = selected else {
        return Ok(Vec::new());
    };
    let canonical = std::fs::canonicalize(&selected)
        .map_err(|e| format!("workspace root is unavailable: {e}"))?;
    if !canonical.is_dir() {
        return Err("workspace root is not a directory".into());
    }
    Ok(vec![super::strip_unc(&canonical)])
}

fn provider_instructions(roots: &[PathBuf], permission_mode: &str) -> String {
    let workspace = roots
        .first()
        .map(|root| root.display().to_string())
        .unwrap_or_else(|| "No project folder is open".into());
    format!(
        "You are Rift, a local coding assistant. Work only inside the configured workspace. Read relevant files before editing, make focused changes, and verify behavior with the available tools. Never claim a command or edit succeeded unless its tool result confirms it. Workspace: {workspace}. Permission mode: {permission_mode}."
    )
}

fn openai_tools(trust_level: &str) -> Vec<Value> {
    let mut tools = super::mcp_server::openai_tool_definitions(trust_level);
    tools.extend([
        json!({
            "type": "function",
            "name": "write_file",
            "description": "Write a complete UTF-8 file inside the active Rift workspace. Use replace_text for a focused edit to an existing file.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or workspace-relative file path." },
                    "content": { "type": "string", "description": "Complete new UTF-8 file content." }
                },
                "required": ["path", "content"]
            },
            "strict": false
        }),
        json!({
            "type": "function",
            "name": "replace_text",
            "description": "Make a focused edit inside an existing UTF-8 workspace file by replacing exact text. By default the old text must occur exactly once.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or workspace-relative file path." },
                    "old_text": { "type": "string", "description": "Exact text to replace." },
                    "new_text": { "type": "string", "description": "Replacement text." },
                    "replace_all": { "type": "boolean", "description": "Replace every occurrence instead of requiring one unique match." }
                },
                "required": ["path", "old_text", "new_text"]
            },
            "strict": false
        }),
        json!({
            "type": "function",
            "name": "run_command",
            "description": "Run a PowerShell command in the active Rift workspace and return stdout, stderr, and exit code. Commands are permission-gated and time-limited.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "PowerShell command to run." },
                    "timeout_seconds": { "type": "integer", "minimum": 1, "maximum": 300, "description": "Timeout in seconds. Default 120." }
                },
                "required": ["command"]
            },
            "strict": false
        }),
    ]);
    tools
}

fn is_mutating_tool(name: &str) -> bool {
    matches!(
        name,
        "write_file"
            | "replace_text"
            | "run_command"
            | "git_pull"
            | "git_commit"
            | "git_push"
            | "gh_pr_create"
    )
}

fn permission_required(mode: &str, name: &str) -> Result<bool, String> {
    if !is_mutating_tool(name) {
        return Ok(false);
    }
    match mode {
        "bypassPermissions" => Ok(false),
        "acceptEdits" | "auto" if matches!(name, "write_file" | "replace_text") => Ok(false),
        "plan" => Err(format!("{name} is unavailable while Rift is in plan mode")),
        _ => Ok(true),
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_tool(
    app: &AppHandle,
    window: &str,
    session: &str,
    call: &FunctionCall,
    args: &Value,
    roots: &[PathBuf],
    trust_level: &str,
    permission_mode: &str,
    cancel: &CancellationToken,
) -> Result<String, String> {
    if permission_required(permission_mode, &call.name)?
        && !ask_openai_permission(app, window, session, call, args, cancel).await?
    {
        return Err("User declined this action.".into());
    }
    if cancel.is_cancelled() {
        return Err("OpenAI turn cancelled".into());
    }

    match call.name.as_str() {
        "write_file" => {
            let args = args.clone();
            let roots = roots.to_vec();
            tokio::task::spawn_blocking(move || tool_write_file(&args, &roots))
                .await
                .map_err(|e| format!("write_file task failed: {e}"))?
        }
        "replace_text" => {
            let args = args.clone();
            let roots = roots.to_vec();
            tokio::task::spawn_blocking(move || tool_replace_text(&args, &roots))
                .await
                .map_err(|e| format!("replace_text task failed: {e}"))?
        }
        "run_command" => tool_run_command(args, roots, cancel).await,
        _ => {
            let name = call.name.clone();
            let args = args.clone();
            let roots = roots.to_vec();
            let trust_level = trust_level.to_string();
            tokio::task::spawn_blocking(move || {
                super::mcp_server::invoke_workspace_tool(&name, &args, &roots, &trust_level)
            })
            .await
            .map_err(|e| format!("workspace tool task failed: {e}"))?
        }
    }
}

async fn ask_openai_permission(
    app: &AppHandle,
    window: &str,
    session: &str,
    call: &FunctionCall,
    args: &Value,
    cancel: &CancellationToken,
) -> Result<bool, String> {
    let registry = app
        .try_state::<Arc<PermissionRegistry>>()
        .ok_or_else(|| "permission registry unavailable".to_string())?
        .inner()
        .clone();
    if app.get_webview_window(window).is_none() {
        return Err("permission UI is unavailable".into());
    }
    let request_id = format!(
        "openai-{}-{}",
        call.call_id,
        NEXT_TURN.fetch_add(1, Ordering::Relaxed)
    );
    let (rx, _guard) = registry.register_guarded(request_id.clone(), session.to_string());
    app.emit_to(
        window,
        PERMISSION_EVENT,
        json!({
            "session_id": session,
            "request_id": request_id,
            "tool_use_id": call.call_id,
            "tool_name": call.name,
            "input": args,
            "suggestions": Value::Null,
            "kind": "tool",
        }),
    )
    .map_err(|e| format!("permission UI emit failed: {e}"))?;

    let decision = tokio::select! {
        _ = cancel.cancelled() => return Ok(false),
        result = tokio::time::timeout(Duration::from_secs(120), rx) => result,
    };
    match decision {
        Ok(Ok(value)) => Ok(value.get("behavior").and_then(Value::as_str) == Some("allow")),
        _ => Ok(false),
    }
}

fn resolve_write_path(path: &str, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let root = roots
        .first()
        .ok_or_else(|| "no workspace root configured".to_string())?;
    let raw = PathBuf::from(path);
    let candidate = if raw.is_absolute() {
        raw
    } else {
        root.join(raw)
    };
    let resolved = if candidate.exists() {
        std::fs::canonicalize(&candidate).map_err(|e| format!("cannot resolve target file: {e}"))?
    } else {
        let parent = candidate
            .parent()
            .ok_or_else(|| "target path has no parent directory".to_string())?;
        let parent = std::fs::canonicalize(parent)
            .map_err(|_| "target parent directory does not exist".to_string())?;
        parent.join(
            candidate
                .file_name()
                .ok_or_else(|| "target path has no file name".to_string())?,
        )
    };
    let resolved = super::strip_unc(&resolved);
    let inside = roots
        .iter()
        .map(|root| super::strip_unc(root))
        .any(|root| resolved.starts_with(root));
    if !inside {
        return Err("target path is outside the workspace".into());
    }
    if resolved.components().any(|component| {
        super::mcp_server::SKIP_DIRS.contains(&component.as_os_str().to_string_lossy().as_ref())
    }) {
        return Err("target path is inside an excluded workspace directory".into());
    }
    Ok(resolved)
}

fn tool_write_file(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("missing `path`")?;
    let content = args
        .get("content")
        .and_then(Value::as_str)
        .ok_or("missing `content`")?;
    if content.len() > MAX_WRITE_BYTES {
        return Err("file content exceeds the 8 MB safety limit".into());
    }
    let resolved = resolve_write_path(path, roots)?;
    std::fs::write(&resolved, content).map_err(|e| format!("write failed: {e}"))?;
    Ok(format!(
        "Wrote {} bytes to {}",
        content.len(),
        resolved.display()
    ))
}

fn tool_replace_text(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("missing `path`")?;
    let old = args
        .get("old_text")
        .and_then(Value::as_str)
        .ok_or("missing `old_text`")?;
    let new = args
        .get("new_text")
        .and_then(Value::as_str)
        .ok_or("missing `new_text`")?;
    if old.is_empty() {
        return Err("`old_text` cannot be empty".into());
    }
    let resolved = resolve_write_path(path, roots)?;
    let content = std::fs::read_to_string(&resolved)
        .map_err(|e| format!("read before replace failed: {e}"))?;
    if content.len() > MAX_WRITE_BYTES {
        return Err("file exceeds the 8 MB safety limit".into());
    }
    let count = content.matches(old).count();
    if count == 0 {
        return Err("old_text was not found".into());
    }
    let replace_all = args
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !replace_all && count != 1 {
        return Err(format!(
            "old_text matched {count} times; provide more context or set replace_all"
        ));
    }
    let updated = if replace_all {
        content.replace(old, new)
    } else {
        content.replacen(old, new, 1)
    };
    if updated.len() > MAX_WRITE_BYTES {
        return Err("edited file would exceed the 8 MB safety limit".into());
    }
    std::fs::write(&resolved, updated).map_err(|e| format!("replace write failed: {e}"))?;
    Ok(format!(
        "Replaced {count} occurrence(s) in {}",
        resolved.display()
    ))
}

async fn tool_run_command(
    args: &Value,
    roots: &[PathBuf],
    cancel: &CancellationToken,
) -> Result<String, String> {
    let root = roots
        .first()
        .ok_or_else(|| "no workspace root configured".to_string())?;
    let command = args
        .get("command")
        .and_then(Value::as_str)
        .ok_or("missing `command`")?;
    if command.is_empty() || command.len() > 32 * 1024 {
        return Err("command must be 1-32768 bytes".into());
    }
    let timeout_secs = args
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(120)
        .clamp(1, 300);
    let mut process = if cfg!(windows) {
        let mut process = tokio::process::Command::new("powershell");
        process.args(["-NoProfile", "-NonInteractive", "-Command", command]);
        process
    } else {
        let mut process = tokio::process::Command::new("sh");
        process.args(["-lc", command]);
        process
    };
    process
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::select! {
        _ = cancel.cancelled() => return Err("command cancelled".into()),
        result = tokio::time::timeout(Duration::from_secs(timeout_secs), process.output()) => {
            match result {
                Ok(result) => result.map_err(|e| format!("command launch failed: {e}"))?,
                Err(_) => return Err(format!("command timed out after {timeout_secs}s")),
            }
        }
    };
    let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if stdout.len() > MAX_COMMAND_OUTPUT_BYTES {
        stdout.truncate(MAX_COMMAND_OUTPUT_BYTES);
        stdout.push_str("\n[stdout truncated]");
    }
    if stderr.len() > MAX_COMMAND_OUTPUT_BYTES {
        stderr.truncate(MAX_COMMAND_OUTPUT_BYTES);
        stderr.push_str("\n[stderr truncated]");
    }
    Ok(format!(
        "exit_code: {}\nstdout:\n{}\nstderr:\n{}",
        output.status.code().unwrap_or(-1),
        stdout,
        stderr
    ))
}

fn emit_tool_use(app: &AppHandle, window: &str, session: &str, epoch: u64, call: &FunctionCall) {
    let input = serde_json::from_str::<Value>(&call.arguments).unwrap_or_else(|_| json!({}));
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type": "assistant",
            "message": { "content": [{
                "type": "tool_use", "id": call.call_id, "name": call.name, "input": input
            }] }
        }),
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_tool_result(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    call_id: &str,
    output: &str,
    is_error: bool,
) {
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type": "user",
            "message": { "content": [{
                "type": "tool_result", "tool_use_id": call_id,
                "content": output, "is_error": is_error
            }] }
        }),
    );
}

fn add_usage(totals: &mut UsageTotals, response: &Value) {
    let usage = response.get("usage").cloned().unwrap_or_else(|| json!({}));
    totals.input = totals.input.saturating_add(
        usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    );
    totals.output = totals.output.saturating_add(
        usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    );
    totals.cached = totals.cached.saturating_add(
        usage
            .pointer("/input_tokens_details/cached_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    );
}

fn usage_snapshot(response: &Value) -> UsageTotals {
    let mut usage = UsageTotals::default();
    add_usage(&mut usage, response);
    usage
}

/// A compaction item carries the state necessary to continue the conversation.
/// Keeping only its tail prevents an otherwise-correct stateless transcript from
/// growing forever after the server has already compacted it.
fn prune_before_latest_compaction(items: &mut Vec<Value>) {
    let latest = items.iter().rposition(|item| {
        item.get("type").and_then(Value::as_str) == Some("compaction")
    });
    if let Some(at) = latest {
        items.drain(..at);
    }
}

fn emit_text_delta(app: &AppHandle, window: &str, session: &str, epoch: u64, delta: &str) {
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type": "stream_event",
            "event": { "type": "content_block_delta", "index": 0,
                "delta": { "type": "text_delta", "text": delta } }
        }),
    );
}

fn emit_thinking_stop(app: &AppHandle, window: &str, session: &str, epoch: u64) {
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type": "stream_event",
            "event": { "type": "content_block_stop", "index": 1 }
        }),
    );
}

fn emit_usage_and_result(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    requested_model: &str,
    response: &Value,
    final_usage: &UsageTotals,
    totals: &UsageTotals,
    continuation: &[Value],
) {
    let model = response
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(requested_model);
    let final_rift_usage = json!({
        "input_tokens": final_usage.input,
        "output_tokens": final_usage.output,
        "cache_read_input_tokens": final_usage.cached,
        "cache_creation_input_tokens": 0,
    });
    let total_rift_usage = json!({
        "input_tokens": totals.input,
        "output_tokens": totals.output,
        "cache_read_input_tokens": totals.cached,
        "cache_creation_input_tokens": 0,
    });
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type": "assistant", "message": { "content": [], "usage": final_rift_usage }
        }),
    );
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type": "result", "subtype": "success", "result": "",
            "usage": total_rift_usage,
            "modelUsage": context_window(model).map(|window| json!({ (model): { "contextWindow": window } })).unwrap_or_else(|| json!({})),
            "provider": "openai",
            "response_id": response.get("id").cloned().unwrap_or(Value::Null),
            "openai_history": continuation,
        }),
    );
}

fn context_window(model: &str) -> Option<u64> {
    if model.starts_with("gpt-5.6") {
        Some(1_050_000)
    } else if model.starts_with("gpt-5.3-codex") {
        Some(400_000)
    } else {
        // `/v1/models` does not expose model limits. Omit an unknown model's
        // window rather than fabricate a value that would make the gauge lie.
        None
    }
}

fn emit_line(app: &AppHandle, window: &str, session: &str, epoch: u64, value: Value) {
    if let Ok(line) = serde_json::to_string(&value) {
        let _ = app.emit_to(
            window,
            STREAM_EVENT,
            json!({
                "session_id": session, "line": line, "turn_epoch": epoch,
            }),
        );
    }
}

fn emit_done(app: &AppHandle, window: &str, session: &str, epoch: u64, exit_code: i32) {
    let _ = app.emit_to(
        window,
        DONE_EVENT,
        json!({
            "session_id": session, "exit_code": exit_code, "turn_epoch": epoch,
        }),
    );
}

fn openai_http_error(status: u16, body: &str) -> String {
    let message = serde_json::from_str::<Value>(body).ok().and_then(|v| {
        v.pointer("/error/message")
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    match status {
        401 => "OpenAI rejected the API key. Update it in Settings → Providers.".into(),
        429 => {
            message.unwrap_or_else(|| "OpenAI rate limit or account spending limit reached.".into())
        }
        _ => message.unwrap_or_else(|| format!("OpenAI returned HTTP {status}")),
    }
}

fn openai_event_error(event: &Value) -> String {
    event
        .pointer("/response/error/message")
        .or_else(|| event.pointer("/error/message"))
        .or_else(|| event.get("message"))
        .and_then(Value::as_str)
        .map(|s| format!("OpenAI response failed: {s}"))
        .unwrap_or_else(|| "OpenAI response failed before completion".into())
}

#[derive(Default)]
struct SseDecoder {
    bytes: Vec<u8>,
}

impl SseDecoder {
    fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>, String> {
        self.bytes.extend_from_slice(chunk);
        if self.bytes.len() > MAX_SSE_FRAME_BYTES {
            return Err("OpenAI stream frame exceeded the 8 MB safety limit".into());
        }
        let mut out = Vec::new();
        while let Some((at, sep_len)) = find_frame_end(&self.bytes) {
            let frame = self.bytes.drain(..at).collect::<Vec<_>>();
            self.bytes.drain(..sep_len);
            let text = std::str::from_utf8(&frame)
                .map_err(|_| "OpenAI stream contained invalid UTF-8".to_string())?;
            let data = text
                .lines()
                .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
                .collect::<Vec<_>>()
                .join("\n");
            if !data.is_empty() {
                out.push(data);
            }
        }
        Ok(out)
    }
}

fn find_frame_end(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes.windows(2).position(|w| w == b"\n\n").map(|i| (i, 2));
    let crlf = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| (i, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_decoder_handles_split_and_crlf_frames() {
        let mut d = SseDecoder::default();
        assert!(d
            .push(b"event: response.output_text.delta\r\ndata: {\"type\":\"response.")
            .unwrap()
            .is_empty());
        let out = d
            .push(b"output_text.delta\",\"delta\":\"hi\"}\r\n\r\n")
            .unwrap();
        assert_eq!(
            out,
            vec![r#"{"type":"response.output_text.delta","delta":"hi"}"#]
        );
    }

    #[test]
    fn sse_decoder_joins_multiple_data_lines() {
        let mut d = SseDecoder::default();
        let out = d.push(b"data: one\ndata: two\n\n").unwrap();
        assert_eq!(out, vec!["one\ntwo"]);
    }

    #[test]
    fn effort_mapping_is_provider_specific() {
        assert_eq!(reasoning_effort(false, Some("ultra")), "none");
        assert_eq!(reasoning_effort(true, Some("smart")), "medium");
        assert_eq!(reasoning_effort(true, Some("ultra")), "xhigh");
    }

    #[test]
    fn stateless_history_keeps_opaque_items_and_prunes_only_before_compaction() {
        let opaque = json!({ "type": "reasoning", "encrypted_content": "opaque" });
        let compact = json!({ "type": "compaction", "encrypted_content": "carry" });
        let tail = json!({ "type": "message", "role": "assistant", "content": "done" });
        let mut items = vec![json!({ "role": "user", "content": "old" }), opaque.clone(), compact.clone(), tail.clone()];
        assert_eq!(validated_history(items.clone()).unwrap(), items);
        prune_before_latest_compaction(&mut items);
        assert_eq!(items, vec![compact, tail]);
    }

    #[test]
    fn context_windows_are_only_reported_when_known() {
        assert_eq!(context_window("gpt-5.6"), Some(1_050_000));
        assert_eq!(context_window("gpt-5.3-codex"), Some(400_000));
        assert_eq!(context_window("gpt-unlisted-preview"), None);
    }

    #[test]
    fn http_errors_do_not_echo_raw_bodies_for_auth() {
        assert_eq!(
            openai_http_error(401, r#"{"error":{"message":"secret-shaped server text"}}"#),
            "OpenAI rejected the API key. Update it in Settings → Providers."
        );
    }

    #[test]
    fn permission_modes_keep_plan_read_only_and_auto_allow_edits() {
        assert_eq!(permission_required("default", "write_file"), Ok(true));
        assert_eq!(permission_required("acceptEdits", "write_file"), Ok(false));
        assert_eq!(permission_required("auto", "replace_text"), Ok(false));
        assert_eq!(
            permission_required("bypassPermissions", "run_command"),
            Ok(false)
        );
        assert!(permission_required("plan", "write_file").is_err());
        assert_eq!(permission_required("plan", "read_file"), Ok(false));
    }

    #[test]
    fn native_file_tools_are_workspace_scoped_and_exact() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        let roots = vec![super::super::strip_unc(&root)];

        tool_write_file(
            &json!({ "path": "sample.txt", "content": "alpha beta" }),
            &roots,
        )
        .unwrap();
        tool_replace_text(
            &json!({ "path": "sample.txt", "old_text": "beta", "new_text": "gamma" }),
            &roots,
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(temp.path().join("sample.txt")).unwrap(),
            "alpha gamma"
        );
        assert!(tool_write_file(
            &json!({ "path": "../outside.txt", "content": "nope" }),
            &roots,
        )
        .is_err());
    }
}
