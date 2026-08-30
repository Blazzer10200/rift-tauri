//! Codex App Server adapter for ChatGPT subscription-backed turns.
//!
//! Authentication remains entirely inside the official Codex CLI. Rift speaks
//! the public JSONL App Server protocol, translates its events into Rift's
//! existing assistant stream envelopes, and persists only the opaque thread id.

use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio_util::sync::CancellationToken;

use super::{AskUserRegistry, AssistantAttachment, PermissionRegistry};

const STREAM_EVENT: &str = "assistant://stream";
const DONE_EVENT: &str = "assistant://done";
const ERROR_EVENT: &str = "assistant://error";
const PERMISSION_EVENT: &str = "assistant://permission-request";
const ASK_USER_EVENT: &str = "assistant://ask-user";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexServiceTier {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexModel {
    pub id: String,
    pub label: String,
    pub description: String,
    pub is_default: bool,
    pub default_reasoning_effort: String,
    pub supported_reasoning_efforts: Vec<String>,
    pub service_tiers: Vec<CodexServiceTier>,
    pub default_service_tier: Option<String>,
    pub upgrade_model: Option<String>,
    pub upgrade_copy: Option<String>,
    pub input_modalities: Vec<String>,
    pub supports_personality: bool,
    pub image_input: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateWindow {
    pub used_percent: f64,
    pub window_duration_mins: Option<u64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimit {
    pub id: String,
    pub name: Option<String>,
    pub plan_type: Option<String>,
    pub primary: Option<CodexRateWindow>,
    pub secondary: Option<CodexRateWindow>,
    pub reached_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsageSummary {
    pub lifetime_tokens: Option<u64>,
    pub peak_daily_tokens: Option<u64>,
    pub longest_running_turn_sec: Option<u64>,
    pub current_streak_days: Option<u64>,
    pub longest_streak_days: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSkill {
    pub name: String,
    pub description: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAccountOverview {
    pub models: Vec<CodexModel>,
    pub skills: Vec<CodexSkill>,
    pub auth_type: Option<String>,
    pub email: Option<String>,
    pub plan_type: Option<String>,
    pub requires_openai_auth: bool,
    pub rate_limits: Vec<CodexRateLimit>,
    pub rate_limits_error: Option<String>,
    pub reset_credits_available: Option<u64>,
    pub usage: Option<CodexUsageSummary>,
    pub usage_error: Option<String>,
}

struct AppServer {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    pending: Vec<Value>,
}

impl AppServer {
    async fn spawn() -> Result<Self, String> {
        let exe = super::codex::resolve_codex_cli().ok_or_else(|| {
            "No runnable Codex CLI was found. Install it from Settings → AI.".to_string()
        })?;
        let mut command = super::codex::command_for(&exe, &["app-server"]);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|e| format!("start Codex App Server: {e}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or("Codex App Server stdin unavailable")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Codex App Server stdout unavailable")?;
        let mut server = Self {
            child,
            stdin,
            lines: BufReader::new(stdout).lines(),
            next_id: 1,
            pending: Vec::new(),
        };
        server
            .request(
                "initialize",
                json!({
                    "clientInfo": { "name": "rift", "title": "Rift", "version": env!("CARGO_PKG_VERSION") },
                    "capabilities": { "experimentalApi": true }
                }),
            )
            .await?;
        server.notify("initialized", json!({})).await?;
        Ok(server)
    }

    async fn write(&mut self, value: &Value) -> Result<(), String> {
        let mut line =
            serde_json::to_vec(value).map_err(|e| format!("encode App Server message: {e}"))?;
        line.push(b'\n');
        self.stdin
            .write_all(&line)
            .await
            .map_err(|e| format!("write App Server message: {e}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("flush App Server message: {e}"))
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.write(&json!({ "method": method, "params": params }))
            .await
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({ "id": id, "method": method, "params": params }))
            .await?;
        loop {
            let line = tokio::time::timeout(Duration::from_secs(20), self.lines.next_line())
                .await
                .map_err(|_| format!("Codex App Server timed out waiting for {method}"))?
                .map_err(|e| format!("read Codex App Server: {e}"))?
                .ok_or_else(|| "Codex App Server exited unexpectedly".to_string())?;
            let msg: Value = serde_json::from_str(&line)
                .map_err(|e| format!("invalid Codex App Server JSON: {e}"))?;
            if msg.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = msg.get("error") {
                    return Err(app_server_error(error));
                }
                return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
            }
            if msg.get("method").is_some() {
                self.pending.push(msg);
            }
        }
    }

    async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
    }
}

fn app_server_error(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Codex App Server request failed")
        .to_string()
}

type ActiveTurn = (u64, CancellationToken);

fn active_turns() -> &'static Mutex<HashMap<String, ActiveTurn>> {
    static ACTIVE: OnceLock<Mutex<HashMap<String, ActiveTurn>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_TURN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

struct ActiveGuard {
    session_id: String,
    generation: u64,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        let mut active = active_turns().lock().unwrap_or_else(|p| p.into_inner());
        if active.get(&self.session_id).map(|(generation, _)| *generation) == Some(self.generation) {
            active.remove(&self.session_id);
        }
    }
}

fn register_turn(session_id: &str) -> (CancellationToken, ActiveGuard) {
    use std::sync::atomic::Ordering;

    let generation = NEXT_TURN.fetch_add(1, Ordering::Relaxed);
    let token = CancellationToken::new();
    let mut active = active_turns().lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, old)) = active.insert(session_id.to_string(), (generation, token.clone())) {
        old.cancel();
    }
    (
        token,
        ActiveGuard {
            session_id: session_id.to_string(),
            generation,
        },
    )
}

pub fn cancel_codex_session(session_id: &str) -> bool {
    let token = active_turns()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(session_id)
        .map(|(_, token)| token.clone());
    if let Some(token) = token {
        token.cancel();
        true
    } else {
        false
    }
}

pub fn cancel_all_codex_turns() {
    let tokens: Vec<_> = active_turns()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .values()
        .map(|(_, token)| token.clone())
        .collect();
    for token in tokens {
        token.cancel();
    }
}

#[tauri::command]
pub async fn assistant_codex_account_overview(
    root: Option<String>,
) -> Result<CodexAccountOverview, String> {
    let mut server = AppServer::spawn().await?;
    let models = server
        .request(
            "model/list",
            json!({ "includeHidden": false, "limit": 100 }),
        )
        .await;
    // Skills are pane/workspace scoped. An omitted root means user-level only;
    // never borrow whichever project happens to be globally selected.
    let root = resolve_root(root.as_deref())?;
    let skills = if let Some(root) = root.as_deref() {
        server
            .request(
                "skills/list",
                json!({ "cwds": [root], "forceReload": false }),
            )
            .await
    } else {
        Ok(json!({ "data": [] }))
    };
    let account = server
        .request("account/read", json!({ "refreshToken": false }))
        .await;
    let rate_limits = server.request("account/rateLimits/read", json!({})).await;
    let usage = server.request("account/usage/read", json!({})).await;
    server.shutdown().await;

    let mut overview = parse_account_overview(&account?, rate_limits, usage);
    let models = models?;
    overview.models = models
        .get("data")
        .and_then(Value::as_array)
        .ok_or("Codex returned an invalid model list")?
        .iter()
        .filter_map(parse_model)
        .collect();
    // Skills are an enhancement, not an account-readiness gate. Older App
    // Server builds can omit this method; keep models/account/limits usable.
    overview.skills = skills
        .as_ref()
        .map(|value| parse_skills(value, root.as_deref()))
        .unwrap_or_default();
    Ok(overview)
}

fn parse_skills(value: &Value, root: Option<&str>) -> Vec<CodexSkill> {
    value
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| {
            root.is_none_or(|root| entry.get("cwd").and_then(Value::as_str) == Some(root))
        })
        .flat_map(|entry| {
            entry
                .get("skills")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|skill| {
            Some(CodexSkill {
                name: skill.get("name")?.as_str()?.to_string(),
                description: skill
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                path: skill
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                enabled: skill.get("enabled").and_then(Value::as_bool).unwrap_or(true),
            })
        })
        .collect()
}

fn leading_skill_name(prompt: &str) -> Option<&str> {
    let token = prompt.split_whitespace().next()?;
    let name = token.strip_prefix('$')?;
    (!name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')))
    .then_some(name)
}

fn parse_rate_window(value: Option<&Value>) -> Option<CodexRateWindow> {
    let value = value?.as_object()?;
    Some(CodexRateWindow {
        used_percent: value.get("usedPercent").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 100.0),
        window_duration_mins: value.get("windowDurationMins").and_then(Value::as_u64),
        resets_at: value.get("resetsAt").and_then(Value::as_i64),
    })
}

fn parse_rate_limit(value: &Value, fallback_id: Option<&str>) -> Option<CodexRateLimit> {
    let id = value
        .get("limitId")
        .and_then(Value::as_str)
        .or(fallback_id)?
        .to_string();
    Some(CodexRateLimit {
        id,
        name: value.get("limitName").and_then(Value::as_str).map(str::to_string),
        plan_type: value.get("planType").and_then(Value::as_str).map(str::to_string),
        primary: parse_rate_window(value.get("primary")),
        secondary: parse_rate_window(value.get("secondary")),
        reached_type: value
            .get("rateLimitReachedType")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_usage_summary(value: &Value) -> Option<CodexUsageSummary> {
    let summary = value.get("summary")?.as_object()?;
    Some(CodexUsageSummary {
        lifetime_tokens: summary.get("lifetimeTokens").and_then(Value::as_u64),
        peak_daily_tokens: summary.get("peakDailyTokens").and_then(Value::as_u64),
        longest_running_turn_sec: summary.get("longestRunningTurnSec").and_then(Value::as_u64),
        current_streak_days: summary.get("currentStreakDays").and_then(Value::as_u64),
        longest_streak_days: summary.get("longestStreakDays").and_then(Value::as_u64),
    })
}

fn parse_account_overview(
    account_result: &Value,
    rate_limits_result: Result<Value, String>,
    usage_result: Result<Value, String>,
) -> CodexAccountOverview {
    let account = account_result.get("account");
    let auth_type = account
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let email = account
        .and_then(|value| value.get("email"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut plan_type = account
        .and_then(|value| value.get("planType"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let requires_openai_auth = account_result
        .get("requiresOpenaiAuth")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let (mut rate_limits, rate_limits_error, reset_credits_available) = match rate_limits_result {
        Ok(value) => {
            let mut parsed = Vec::new();
            if let Some(by_id) = value.get("rateLimitsByLimitId").and_then(Value::as_object) {
                parsed.extend(by_id.iter().filter_map(|(id, limit)| parse_rate_limit(limit, Some(id))));
            } else if let Some(limit) = value.get("rateLimits") {
                parsed.extend(parse_rate_limit(limit, None));
            }
            parsed.sort_by(|left, right| left.id.cmp(&right.id));
            if plan_type.is_none() {
                plan_type = parsed.iter().find_map(|limit| limit.plan_type.clone());
            }
            let credits = value
                .pointer("/rateLimitResetCredits/availableCount")
                .and_then(Value::as_u64);
            (parsed, None, credits)
        }
        Err(error) => (Vec::new(), Some(error), None),
    };
    rate_limits.shrink_to_fit();
    let (usage, usage_error) = match usage_result {
        Ok(value) => (parse_usage_summary(&value), None),
        Err(error) => (None, Some(error)),
    };

    CodexAccountOverview {
        models: Vec::new(),
        skills: Vec::new(),
        auth_type,
        email,
        plan_type,
        requires_openai_auth,
        rate_limits,
        rate_limits_error,
        reset_credits_available,
        usage,
        usage_error,
    }
}

fn parse_model(value: &Value) -> Option<CodexModel> {
    if value
        .get("hidden")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let id = value
        .get("model")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)?
        .to_string();
    let efforts = value
        .get("supportedReasoningEfforts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("reasoningEffort").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let input_modalities: Vec<String> = value
        .get("inputModalities")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|| vec!["text".into(), "image".into()]);
    let service_tiers = value
        .get("serviceTiers")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item.get("id").and_then(Value::as_str)?.to_string();
                    Some(CodexServiceTier {
                        name: item
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or(&id)
                            .to_string(),
                        description: item
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        id,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let upgrade = value.get("upgrade").or_else(|| value.get("upgradeInfo"));
    Some(CodexModel {
        id,
        label: value
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or("GPT")
            .to_string(),
        description: value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        is_default: value
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        default_reasoning_effort: value
            .get("defaultReasoningEffort")
            .and_then(Value::as_str)
            .unwrap_or("medium")
            .to_string(),
        supported_reasoning_efforts: efforts,
        service_tiers,
        default_service_tier: value
            .get("defaultServiceTier")
            .and_then(Value::as_str)
            .map(str::to_string),
        upgrade_model: upgrade
            .and_then(|item| item.get("model").or_else(|| item.get("modelId")))
            .and_then(Value::as_str)
            .map(str::to_string),
        upgrade_copy: upgrade
            .and_then(|item| {
                item.get("migrationMarkdown")
                    .or_else(|| item.get("copy"))
                    .or_else(|| item.get("description"))
            })
            .and_then(Value::as_str)
            .map(str::to_string),
        supports_personality: value
            .get("supportsPersonality")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        image_input: input_modalities.iter().any(|item| item == "image"),
        input_modalities,
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn assistant_codex_send(
    app: AppHandle,
    window: tauri::Window,
    prompt: String,
    session_id: String,
    codex_thread_id: Option<String>,
    model: Option<String>,
    attachments: Option<Vec<AssistantAttachment>>,
    thinking_effort: Option<String>,
    thinking_enabled: Option<bool>,
    fast_mode: Option<bool>,
    turn_epoch: Option<u64>,
    permission_mode: Option<String>,
    root: Option<String>,
) -> Result<(), String> {
    if !super::is_valid_session_id(&session_id) {
        return Err("invalid session_id: must be a UUID".into());
    }
    let attachments = attachments.unwrap_or_default();
    super::turn::validate_attachments(&attachments)?;
    if prompt.trim().is_empty() && attachments.is_empty() {
        return Err("empty ChatGPT message".into());
    }
    let model = model.ok_or("No ChatGPT model is selected")?;
    if !super::is_valid_model_name(&model) || !model.starts_with("gpt-") {
        return Err("invalid ChatGPT model id".into());
    }
    let cfg = super::config::load_config();
    let permission_mode = permission_mode
        .filter(|mode| super::config::is_valid_permission_mode(mode))
        .or(cfg.permission_mode)
        .unwrap_or_else(|| "default".into());
    let root = resolve_turn_root(&session_id, root.as_deref())?;
    let epoch = turn_epoch.unwrap_or(0);
    let window_label = window.label().to_string();
    let (cancel, _guard) = register_turn(&session_id);
    let mut server = AppServer::spawn().await?;
    let result = run_turn(
        &app,
        &window_label,
        &session_id,
        epoch,
        &mut server,
        codex_thread_id.as_deref(),
        &model,
        &prompt,
        &attachments,
        thinking_enabled.unwrap_or(false),
        thinking_effort.as_deref(),
        fast_mode.unwrap_or(false),
        &permission_mode,
        root.as_deref(),
        &cancel,
    )
    .await;
    server.shutdown().await;
    if let Err(error) = &result {
        let _ = app.emit_to(
            &window_label,
            ERROR_EVENT,
            json!({ "session_id": session_id, "message": error, "turn_epoch": epoch }),
        );
    }
    result
}

fn resolve_turn_root(session_id: &str, requested: Option<&str>) -> Result<Option<String>, String> {
    let root = super::resolve_session_workspace(session_id, requested)?;
    Ok(root.map(|path| path.to_string_lossy().into_owned()))
}

fn resolve_root(requested: Option<&str>) -> Result<Option<String>, String> {
    let candidate = requested
        .filter(|value| !value.trim().is_empty())
        .map(std::path::PathBuf::from);
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    if !candidate.is_dir() {
        return Err(format!(
            "workspace folder does not exist: {}",
            candidate.display()
        ));
    }
    Ok(Some(
        super::canonicalize_root(candidate)?
            .to_string_lossy()
            .into_owned(),
    ))
}

#[allow(clippy::too_many_arguments)]
async fn run_turn(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    server: &mut AppServer,
    prior_thread: Option<&str>,
    model: &str,
    prompt: &str,
    attachments: &[AssistantAttachment],
    thinking_enabled: bool,
    thinking_effort: Option<&str>,
    fast_mode: bool,
    permission_mode: &str,
    root: Option<&str>,
    cancel: &CancellationToken,
) -> Result<(), String> {
    let approval_policy = if permission_mode == "bypassPermissions" {
        "never"
    } else {
        "on-request"
    };
    let roots = root.map(|value| vec![value]).unwrap_or_default();
    let service_tier = requested_fast_tier(server, model, fast_mode).await?;
    let thread_result = if let Some(thread_id) = prior_thread.filter(|id| !id.trim().is_empty()) {
        let mut params = json!({
            "threadId": thread_id,
            "cwd": root,
            "model": model,
            "approvalPolicy": approval_policy,
            "runtimeWorkspaceRoots": roots,
        });
        insert_service_tier(&mut params, service_tier.as_deref());
        server
            .request("thread/resume", params)
            .await?
    } else {
        let mut params = json!({
            "cwd": root,
            "model": model,
            "approvalPolicy": approval_policy,
            "approvalsReviewer": "user",
            "runtimeWorkspaceRoots": roots,
            "serviceName": "rift",
            "ephemeral": false,
        });
        insert_service_tier(&mut params, service_tier.as_deref());
        server
            .request("thread/start", params)
            .await?
    };
    let fast_active = thread_confirmed_fast(&thread_result, service_tier.is_some());
    let thread_id = thread_result
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .or(prior_thread)
        .ok_or("Codex did not return a thread id")?
        .to_string();

    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type": "system", "subtype": "init", "model": model,
            "provider": "codex", "codex_thread_id": thread_id,
        }),
    );

    let effort = effort_name(thinking_enabled, thinking_effort);
    let sandbox = sandbox_policy(permission_mode, root);
    let mut input = vec![json!({ "type": "text", "text": prompt })];
    if let (Some(root), Some(skill_name)) = (root, leading_skill_name(prompt)) {
        if let Ok(result) = server
            .request(
                "skills/list",
                json!({ "cwds": [root], "forceReload": false }),
            )
            .await
        {
            if let Some(skill) = parse_skills(&result, Some(root))
                .into_iter()
                .find(|skill| skill.enabled && skill.name == skill_name && !skill.path.is_empty())
            {
                input.push(json!({ "type": "skill", "name": skill.name, "path": skill.path }));
            }
        }
    }
    for attachment in attachments {
        input.push(json!({
            "type": "image",
            "url": format!("data:{};base64,{}", attachment.mime, attachment.data_base64),
        }));
    }
    let collaboration = if permission_mode == "plan" {
        json!({ "mode": "plan", "settings": {
            "model": model, "reasoning_effort": effort, "developer_instructions": null
        }})
    } else {
        Value::Null
    };
    let mut turn_params = json!({
        "threadId": thread_id,
        "input": input,
        "cwd": root,
        "model": model,
        "effort": effort,
        "summary": "concise",
        "approvalPolicy": approval_policy,
        "sandboxPolicy": sandbox,
        "collaborationMode": collaboration,
        "runtimeWorkspaceRoots": roots,
    });
    insert_service_tier(&mut turn_params, service_tier.as_deref());
    let turn = server.request("turn/start", turn_params).await?;
    let turn_id = turn
        .pointer("/turn/id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    stream_turn(
        app,
        window,
        session,
        epoch,
        server,
        &thread_id,
        &turn_id,
        model,
        fast_active,
        permission_mode,
        cancel,
    )
    .await
}

fn effort_name(thinking_enabled: bool, effort: Option<&str>) -> &'static str {
    if !thinking_enabled {
        return "low";
    }
    match effort {
        Some("agentic") => "ultra",
        Some("max") => "max",
        Some("ultra") => "xhigh",
        Some("deep") => "high",
        Some("none" | "low") => "low",
        _ => "medium",
    }
}

fn is_fast_tier(value: &str) -> bool {
    value.eq_ignore_ascii_case("priority") || value.eq_ignore_ascii_case("fast")
}

fn insert_service_tier(params: &mut Value, service_tier: Option<&str>) {
    if let (Some(map), Some(tier)) = (params.as_object_mut(), service_tier) {
        map.insert("serviceTier".into(), Value::String(tier.to_string()));
    }
}

fn thread_confirmed_fast(thread_result: &Value, requested: bool) -> bool {
    requested
        && thread_result
            .pointer("/thread/serviceTier")
            .and_then(Value::as_str)
            .is_some_and(is_fast_tier)
}

async fn requested_fast_tier(
    server: &mut AppServer,
    model: &str,
    requested: bool,
) -> Result<Option<String>, String> {
    if !requested {
        return Ok(None);
    }
    let result = server
        .request(
            "model/list",
            json!({ "includeHidden": false, "limit": 100 }),
        )
        .await?;
    let tier = result
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(parse_model)
        .find(|candidate| candidate.id == model)
        .and_then(|candidate| {
            candidate
                .service_tiers
                .into_iter()
                .find(|tier| is_fast_tier(&tier.id))
                .map(|tier| tier.id)
        });
    tier.map(Some).ok_or_else(|| {
        format!("Fast mode is not available for {model} through this ChatGPT account")
    })
}

fn sandbox_policy(mode: &str, root: Option<&str>) -> Value {
    match mode {
        "bypassPermissions" => json!({ "type": "dangerFullAccess" }),
        "plan" => json!({ "type": "readOnly", "networkAccess": false }),
        _ => json!({
            "type": "workspaceWrite", "writableRoots": root.into_iter().collect::<Vec<_>>(),
            "networkAccess": false, "excludeTmpdirEnvVar": true, "excludeSlashTmp": true,
        }),
    }
}

#[derive(Default)]
struct StreamState {
    thinking: bool,
    agent_delta_items: HashSet<String>,
    usage: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
struct ToolUseSpec {
    id: String,
    name: String,
    input: Value,
}

fn file_change_tool_uses(id: &str, changes: Option<&Value>) -> Vec<ToolUseSpec> {
    let Some(changes) = changes.and_then(Value::as_array) else {
        return Vec::new();
    };
    changes
        .iter()
        .enumerate()
        .filter_map(|(index, change)| {
            let path = change.get("path").and_then(Value::as_str)?;
            let diff = change
                .get("diff")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let kind = change
                .pointer("/kind/type")
                .and_then(Value::as_str)
                .unwrap_or("update");
            let tool_id = if index == 0 {
                id.to_string()
            } else {
                format!("{id}:{index}")
            };
            let (name, input) = match kind {
                "add" => (
                    "Write",
                    json!({
                        "file_path": path,
                        "content": diff,
                        "codex_diff_kind": "add",
                    }),
                ),
                "delete" => (
                    "Delete",
                    json!({
                        "file_path": path,
                        "unified_diff": diff,
                        "codex_diff_kind": "delete",
                    }),
                ),
                _ => (
                    "Edit",
                    json!({
                        "file_path": path,
                        "unified_diff": diff,
                        "move_path": change.pointer("/kind/move_path").cloned(),
                        "codex_diff_kind": "update",
                    }),
                ),
            };
            Some(ToolUseSpec {
                id: tool_id,
                name: name.to_string(),
                input,
            })
        })
        .collect()
}

fn item_tool_uses(item: &Value) -> Vec<ToolUseSpec> {
    let Some(id) = item.get("id").and_then(Value::as_str) else {
        return Vec::new();
    };
    if item.get("type").and_then(Value::as_str) == Some("fileChange") {
        return file_change_tool_uses(id, item.get("changes"));
    }
    let (name, input) = match item.get("type").and_then(Value::as_str) {
        Some("commandExecution") => (
            "Bash",
            json!({
                "command": item.get("command").cloned().unwrap_or(Value::Null),
                "cwd": item.get("cwd").cloned().unwrap_or(Value::Null),
            }),
        ),
        Some("mcpToolCall") => (
            item.get("tool")
                .and_then(Value::as_str)
                .unwrap_or("MCP tool"),
            item.get("arguments").cloned().unwrap_or(Value::Null),
        ),
        Some("webSearch") => (
            "WebSearch",
            json!({ "query": item.get("query").cloned().unwrap_or(Value::Null) }),
        ),
        Some("dynamicToolCall") => (
            item.get("tool").and_then(Value::as_str).unwrap_or("Tool"),
            item.get("arguments").cloned().unwrap_or(Value::Null),
        ),
        Some("plan") => (
            "ExitPlanMode",
            json!({ "plan": item.get("text").cloned().unwrap_or(Value::Null) }),
        ),
        Some("imageView") => (
            "ViewImage",
            json!({ "file_path": item.get("path").cloned().unwrap_or(Value::Null) }),
        ),
        Some("sleep") => (
            "Sleep",
            json!({ "duration_ms": item.get("durationMs").cloned().unwrap_or(Value::Null) }),
        ),
        Some("imageGeneration") => (
            "ImageGeneration",
            json!({ "prompt": item.get("revisedPrompt").cloned().unwrap_or(Value::Null) }),
        ),
        Some("collabAgentToolCall") => (
            "Agent",
            json!({
                "description": item.get("prompt").cloned().unwrap_or_else(|| json!("Codex agent task")),
                "subagent_type": item.get("tool").cloned().unwrap_or_else(|| json!("agent")),
                "model": item.get("model").cloned().unwrap_or(Value::Null),
            }),
        ),
        Some("enteredReviewMode") => (
            "EnterReviewMode",
            json!({ "review": item.get("review").cloned().unwrap_or(Value::Null) }),
        ),
        Some("exitedReviewMode") => (
            "ExitReviewMode",
            json!({ "review": item.get("review").cloned().unwrap_or(Value::Null) }),
        ),
        _ => return Vec::new(),
    };
    vec![ToolUseSpec {
        id: id.to_string(),
        name: name.to_string(),
        input,
    }]
}

fn item_failed(item: &Value) -> bool {
    item.get("status").and_then(Value::as_str) == Some("failed")
        || item.get("success").and_then(Value::as_bool) == Some(false)
        || item.get("error").is_some_and(|error| !error.is_null())
}

fn turn_plan_markdown(params: &Value) -> Option<String> {
    if let Some(plan) = params.get("plan").and_then(Value::as_str) {
        return Some(plan.to_string());
    }
    let steps = params.get("plan").and_then(Value::as_array)?;
    let mut markdown = String::new();
    if let Some(explanation) = params.get("explanation").and_then(Value::as_str) {
        if !explanation.trim().is_empty() {
            markdown.push_str(explanation.trim());
            markdown.push_str("\n\n");
        }
    }
    for step in steps {
        let text = step.get("step").and_then(Value::as_str).unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        let done = step.get("status").and_then(Value::as_str) == Some("completed");
        markdown.push_str(if done { "- [x] " } else { "- [ ] " });
        markdown.push_str(text);
        markdown.push('\n');
    }
    (!markdown.is_empty()).then(|| markdown.trim_end().to_string())
}

#[allow(clippy::too_many_arguments)]
async fn stream_turn(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    server: &mut AppServer,
    thread_id: &str,
    turn_id: &str,
    model: &str,
    fast_active: bool,
    permission_mode: &str,
    cancel: &CancellationToken,
) -> Result<(), String> {
    let mut state = StreamState::default();
    let mut queued = std::mem::take(&mut server.pending);
    let mut interrupting = false;
    loop {
        let value = if !queued.is_empty() {
            queued.remove(0)
        } else if interrupting {
            let line = tokio::time::timeout(Duration::from_secs(4), server.lines.next_line())
                .await
                .map_err(|_| "ChatGPT cancellation timed out".to_string())?
                .map_err(|e| format!("read Codex stream: {e}"))?
                .ok_or_else(|| "Codex App Server ended during cancellation".to_string())?;
            serde_json::from_str::<Value>(&line)
                .map_err(|e| format!("invalid Codex stream JSON: {e}"))?
        } else {
            let line = tokio::select! {
                _ = cancel.cancelled() => {
                    interrupting = true;
                    let id = server.next_id;
                    server.next_id += 1;
                    server.write(&json!({
                        "id": id, "method": "turn/interrupt",
                        "params": { "threadId": thread_id, "turnId": turn_id }
                    })).await?;
                    continue;
                }
                line = server.lines.next_line() => {
                    line.map_err(|e| format!("read Codex stream: {e}"))?
                }
            }
            .ok_or_else(|| "Codex App Server ended before the turn completed".to_string())?;
            serde_json::from_str::<Value>(&line)
                .map_err(|e| format!("invalid Codex stream JSON: {e}"))?
        };
        if value.get("method").is_none() {
            continue;
        }
        if value.get("id").is_some() {
            handle_server_request(app, window, session, epoch, server, &value, permission_mode)
                .await?;
            continue;
        }
        let method = value
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = value.get("params").cloned().unwrap_or(Value::Null);
        if params
            .get("threadId")
            .and_then(Value::as_str)
            .is_some_and(|id| id != thread_id)
        {
            continue;
        }
        match method {
            "item/agentMessage/delta" => {
                if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                    if let Some(id) = params.get("itemId").and_then(Value::as_str) {
                        state.agent_delta_items.insert(id.to_string());
                    }
                    emit_text_delta(app, window, session, epoch, delta);
                }
            }
            "item/reasoning/summaryTextDelta" | "item/reasoning/textDelta" => {
                if !state.thinking {
                    state.thinking = true;
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"stream_event", "event": {"type":"content_block_start", "index":1,
                            "content_block":{"type":"thinking","thinking":"","signature":""}}
                        }),
                    );
                }
                if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"stream_event", "event": {"type":"content_block_delta", "index":1,
                            "delta":{"type":"thinking_delta","thinking":delta}}
                        }),
                    );
                }
            }
            "item/commandExecution/outputDelta" => {
                if let (Some(id), Some(delta)) = (
                    params.get("itemId").and_then(Value::as_str),
                    params.get("delta").and_then(Value::as_str),
                ) {
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"tool_progress", "tool_use_id":id,
                            "tool_name":"Bash", "output_delta":delta,
                        }),
                    );
                }
            }
            "item/commandExecution/terminalInteraction" => {
                if let Some(id) = params.get("itemId").and_then(Value::as_str) {
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"tool_progress", "tool_use_id":id,
                            "tool_name":"Bash", "message":"Interactive command input",
                        }),
                    );
                }
            }
            "item/fileChange/patchUpdated" => {
                if let Some(id) = params.get("itemId").and_then(Value::as_str) {
                    let item = json!({
                        "id": id,
                        "type": "fileChange",
                        "changes": params.get("changes").cloned().unwrap_or_else(|| json!([])),
                    });
                    emit_item_started(app, window, session, epoch, Some(&item));
                }
            }
            "item/mcpToolCall/progress" => {
                if let Some(id) = params.get("itemId").and_then(Value::as_str) {
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"tool_progress", "tool_use_id":id,
                            "message":params.get("message").cloned().unwrap_or(Value::Null),
                        }),
                    );
                }
            }
            "item/started" => emit_item_started(app, window, session, epoch, params.get("item")),
            "item/completed" => {
                if params.pointer("/item/type").and_then(Value::as_str) == Some("reasoning")
                    && state.thinking
                {
                    state.thinking = false;
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"stream_event", "event":{"type":"content_block_stop","index":1}
                        }),
                    );
                }
                if params.pointer("/item/type").and_then(Value::as_str)
                    == Some("contextCompaction")
                {
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"system", "subtype":"compact_boundary", "provider":"codex"
                        }),
                    );
                }
                emit_item_completed(
                    app,
                    window,
                    session,
                    epoch,
                    params.get("item"),
                    &state.agent_delta_items,
                );
            }
            "thread/tokenUsage/updated" => state.usage = params.get("tokenUsage").cloned(),
            "thread/compacted" => emit_line(
                app,
                window,
                session,
                epoch,
                json!({
                    "type":"system", "subtype":"compact_boundary", "provider":"codex"
                }),
            ),
            "turn/plan/updated" => {
                if let Some(plan) = turn_plan_markdown(&params) {
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"assistant", "message":{"content":[{
                                "type":"tool_use", "id":format!("codex-plan-{turn_id}"),
                                "name":"ExitPlanMode", "input":{"plan":plan}
                            }]}
                        }),
                    );
                }
            }
            "turn/completed" => {
                if state.thinking {
                    emit_line(
                        app,
                        window,
                        session,
                        epoch,
                        json!({
                            "type":"stream_event", "event":{"type":"content_block_stop","index":1}
                        }),
                    );
                }
                let status = params
                    .pointer("/turn/status")
                    .and_then(Value::as_str)
                    .unwrap_or("failed");
                let error = params
                    .pointer("/turn/error/message")
                    .and_then(Value::as_str);
                emit_result(
                    app,
                    window,
                    session,
                    epoch,
                    CodexResult {
                        model,
                        thread_id,
                        status,
                        error,
                        usage: state.usage.as_ref(),
                        fast_active,
                    },
                );
                emit_done(
                    app,
                    window,
                    session,
                    epoch,
                    if status == "completed" { 0 } else { -1 },
                );
                return Ok(());
            }
            _ => {}
        }
    }
}

fn emit_item_started(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    item: Option<&Value>,
) {
    let Some(item) = item else { return };
    let content = item_tool_uses(item)
        .into_iter()
        .map(|tool| {
            json!({
                "type":"tool_use", "id":tool.id, "name":tool.name, "input":tool.input
            })
        })
        .collect::<Vec<_>>();
    if !content.is_empty() {
        emit_line(
            app,
            window,
            session,
            epoch,
            json!({ "type":"assistant", "message":{"content":content} }),
        );
    }
}

fn emit_item_completed(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    item: Option<&Value>,
    agent_delta_items: &HashSet<String>,
) {
    let Some(item) = item else { return };
    let Some(id) = item.get("id").and_then(Value::as_str) else {
        return;
    };
    let item_type = item.get("type").and_then(Value::as_str);
    match item_type {
        Some("agentMessage") if !agent_delta_items.contains(id) => {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                emit_text_delta(app, window, session, epoch, text);
            }
        }
        Some("commandExecution") => {
            emit_item_started(app, window, session, epoch, Some(item));
            let output = item
                .get("aggregatedOutput")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let failed = item_failed(item)
                || item
                    .get("exitCode")
                    .and_then(Value::as_i64)
                    .is_some_and(|code| code != 0);
            emit_tool_result(app, window, session, epoch, id, output, failed);
        }
        Some("fileChange") => {
            emit_item_started(app, window, session, epoch, Some(item));
            for tool in item_tool_uses(item) {
                emit_tool_result(
                    app,
                    window,
                    session,
                    epoch,
                    &tool.id,
                    if item_failed(item) {
                        "File change failed"
                    } else {
                        "File change applied"
                    },
                    item_failed(item),
                );
            }
        }
        Some("mcpToolCall") => {
            emit_item_started(app, window, session, epoch, Some(item));
            let output = item
                .get("result")
                .filter(|value| !value.is_null())
                .or_else(|| item.get("error").filter(|value| !value.is_null()))
                .cloned()
                .unwrap_or_else(|| json!({"status": item.get("status")}));
            emit_tool_result(
                app,
                window,
                session,
                epoch,
                id,
                &serde_json::to_string_pretty(&output).unwrap_or_default(),
                item_failed(item),
            );
        }
        Some("dynamicToolCall") => {
            emit_item_started(app, window, session, epoch, Some(item));
            let output = item
                .get("contentItems")
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or_else(|| json!({"status": item.get("status")}));
            emit_tool_result(
                app,
                window,
                session,
                epoch,
                id,
                &serde_json::to_string_pretty(&output).unwrap_or_default(),
                item_failed(item),
            );
        }
        Some("webSearch") => {
            emit_item_started(app, window, session, epoch, Some(item));
            let output = item
                .get("results")
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or_else(|| json!({"status":"completed"}));
            emit_tool_result(
                app,
                window,
                session,
                epoch,
                id,
                &serde_json::to_string_pretty(&output).unwrap_or_default(),
                item_failed(item),
            );
        }
        Some(
            "plan" | "imageView" | "sleep" | "imageGeneration" | "collabAgentToolCall"
            | "enteredReviewMode" | "exitedReviewMode",
        ) => {
            emit_item_started(app, window, session, epoch, Some(item));
            let output = item
                .get("result")
                .or_else(|| item.get("status"))
                .cloned()
                .unwrap_or_else(|| json!("Completed"));
            emit_tool_result(
                app,
                window,
                session,
                epoch,
                id,
                &serde_json::to_string_pretty(&output).unwrap_or_default(),
                item_failed(item),
            );
        }
        _ => {}
    }
}

async fn handle_server_request(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    server: &mut AppServer,
    request: &Value,
    permission_mode: &str,
) -> Result<(), String> {
    let id = request
        .get("id")
        .cloned()
        .ok_or("Codex request missing id")?;
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
            let auto_accept = permission_mode == "bypassPermissions"
                || (method.contains("fileChange")
                    && matches!(permission_mode, "acceptEdits" | "auto"));
            let decision = if auto_accept {
                "accept"
            } else {
                await_permission(app, window, session, &id, method, &params).await?
            };
            server
                .write(&json!({ "id": id, "result": { "decision": decision } }))
                .await?;
        }
        "item/tool/requestUserInput" => {
            let answers = await_user_input(app, window, session, epoch, &id, &params).await?;
            server
                .write(&json!({ "id": id, "result": { "answers": answers } }))
                .await?;
        }
        _ => {
            server.write(&json!({ "id": id, "error": { "code": -32601, "message": "Rift does not support this App Server request yet" } })).await?;
        }
    }
    Ok(())
}

async fn await_permission(
    app: &AppHandle,
    window: &str,
    session: &str,
    protocol_id: &Value,
    method: &str,
    params: &Value,
) -> Result<&'static str, String> {
    let registry = app
        .try_state::<std::sync::Arc<PermissionRegistry>>()
        .ok_or("permission registry unavailable")?
        .inner()
        .clone();
    let request_id = format!("codex:{session}:{}", protocol_id);
    let tool_id = params
        .get("itemId")
        .and_then(Value::as_str)
        .unwrap_or(&request_id);
    let is_file = method.contains("fileChange");
    let input = if is_file {
        json!({ "reason": params.get("reason"), "root": params.get("grantRoot") })
    } else {
        json!({ "command": params.get("command"), "cwd": params.get("cwd"), "reason": params.get("reason") })
    };
    let (rx, _guard) = registry.register_guarded(request_id.clone(), session.to_string());
    app.emit_to(
        window,
        PERMISSION_EVENT,
        json!({
            "session_id":session, "request_id":request_id, "tool_use_id":tool_id,
            "tool_name": if is_file { "Edit" } else { "Bash" }, "input":input,
            "suggestions":Value::Null, "kind":"tool",
        }),
    )
    .map_err(|e| format!("show permission request: {e}"))?;
    let decision = tokio::time::timeout(Duration::from_secs(120), rx)
        .await
        .map_err(|_| "permission request timed out".to_string())?
        .map_err(|_| "permission request was cancelled".to_string())?;
    Ok(
        if decision.get("behavior").and_then(Value::as_str) == Some("allow") {
            "accept"
        } else {
            "decline"
        },
    )
}

async fn await_user_input(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    protocol_id: &Value,
    params: &Value,
) -> Result<Value, String> {
    let registry = app
        .try_state::<std::sync::Arc<AskUserRegistry>>()
        .ok_or("ask-user registry unavailable")?
        .inner()
        .clone();
    let request_id = format!("codex:{session}:{}", protocol_id);
    let tool_id = params
        .get("itemId")
        .and_then(Value::as_str)
        .unwrap_or(&request_id);
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type":"assistant", "message":{"content":[{
                "type":"tool_use", "id":tool_id, "name":"mcp__rift__ask_user",
                "input":{"questions":params.get("questions").cloned().unwrap_or_else(|| json!([]))}
            }]}
        }),
    );
    let (rx, _guard) = registry.register_guarded(request_id.clone(), session.to_string());
    app.emit_to(
        window,
        ASK_USER_EVENT,
        json!({
            "request_id":request_id, "session_id":session,
            "questions":params.get("questions").cloned().unwrap_or_else(|| json!([])),
        }),
    )
    .map_err(|e| format!("show user question: {e}"))?;
    let answer = tokio::time::timeout(Duration::from_secs(600), rx)
        .await
        .map_err(|_| "user question timed out".to_string())?
        .map_err(|_| "user question was cancelled".to_string())?;
    Ok(codex_answers(params.get("questions"), &answer))
}

fn codex_answers(questions: Option<&Value>, answer: &Value) -> Value {
    let submitted = answer.get("answers").and_then(Value::as_array);
    let mut out = serde_json::Map::new();
    for question in questions.and_then(Value::as_array).into_iter().flatten() {
        let Some(id) = question.get("id").and_then(Value::as_str) else {
            continue;
        };
        let text = question
            .get("question")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let values = submitted
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("question").and_then(Value::as_str) == Some(text))
            })
            .and_then(|item| item.get("answer"))
            .map(|value| match value {
                Value::Array(items) => Value::Array(items.clone()),
                Value::String(value) => json!([value]),
                _ => json!([]),
            })
            .unwrap_or_else(|| json!([]));
        out.insert(id.to_string(), json!({ "answers": values }));
    }
    Value::Object(out)
}

fn emit_text_delta(app: &AppHandle, window: &str, session: &str, epoch: u64, delta: &str) {
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type":"stream_event", "event":{"type":"content_block_delta","index":0,
            "delta":{"type":"text_delta","text":delta}}
        }),
    );
}

fn emit_tool_result(
    app: &AppHandle,
    window: &str,
    session: &str,
    epoch: u64,
    id: &str,
    output: &str,
    is_error: bool,
) {
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type":"user", "message":{"content":[{
                "type":"tool_result", "tool_use_id":id, "content":output, "is_error":is_error
            }]}
        }),
    );
}

struct CodexResult<'a> {
    model: &'a str,
    thread_id: &'a str,
    status: &'a str,
    error: Option<&'a str>,
    usage: Option<&'a Value>,
    fast_active: bool,
}

fn emit_result(app: &AppHandle, window: &str, session: &str, epoch: u64, result: CodexResult<'_>) {
    let CodexResult {
        model,
        thread_id,
        status,
        error,
        usage,
        fast_active,
    } = result;
    let last = usage
        .and_then(|value| value.get("last"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let total = usage
        .and_then(|value| value.get("total"))
        .cloned()
        .unwrap_or_else(|| last.clone());
    let rift_usage = |value: &Value| {
        json!({
            "input_tokens": value.get("inputTokens").and_then(Value::as_i64).unwrap_or(0),
            "output_tokens": value.get("outputTokens").and_then(Value::as_i64).unwrap_or(0),
            "cache_read_input_tokens": value.get("cachedInputTokens").and_then(Value::as_i64).unwrap_or(0),
            "cache_creation_input_tokens": value.get("cacheWriteInputTokens").and_then(Value::as_i64).unwrap_or(0),
        })
    };
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type":"assistant", "message":{"content":[], "usage":rift_usage(&last)}
        }),
    );
    let context = usage
        .and_then(|value| value.get("modelContextWindow"))
        .and_then(Value::as_i64);
    emit_line(
        app,
        window,
        session,
        epoch,
        json!({
            "type":"result", "subtype":if status == "completed" {"success"} else {status},
            "is_error":status == "failed", "errors":error.into_iter().collect::<Vec<_>>(),
            "result":"", "usage":rift_usage(&total), "provider":"codex",
            "codex_thread_id":thread_id,
            "fast_mode_state":if fast_active {Value::String("on".into())} else {Value::Null},
            "modelUsage":context.map(|window| json!({(model):{"contextWindow":window}})).unwrap_or_else(|| json!({})),
        }),
    );
}

fn emit_line(app: &AppHandle, window: &str, session: &str, epoch: u64, value: Value) {
    if let Ok(line) = serde_json::to_string(&value) {
        let _ = app.emit_to(
            window,
            STREAM_EVENT,
            json!({
                "session_id":session, "line":line, "turn_epoch":epoch,
            }),
        );
    }
}

fn emit_done(app: &AppHandle, window: &str, session: &str, epoch: u64, exit_code: i32) {
    let _ = app.emit_to(
        window,
        DONE_EVENT,
        json!({
            "session_id":session, "exit_code":exit_code, "turn_epoch":epoch,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        cancel_codex_session, codex_answers, effort_name, file_change_tool_uses,
        insert_service_tier, item_failed, leading_skill_name, parse_account_overview, parse_model,
        register_turn, sandbox_policy, thread_confirmed_fast, turn_plan_markdown,
    };
    use serde_json::json;

    #[test]
    fn model_list_uses_live_catalog_fields() {
        let model = parse_model(&json!({
            "model":"gpt-5.6-sol", "displayName":"GPT-5.6 Sol", "description":"Agentic",
            "hidden":false, "isDefault":true, "defaultReasoningEffort":"low",
            "supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"high"}],
            "serviceTiers":[{"id":"priority","name":"Fast","description":"1.5x speed"}],
            "defaultServiceTier":"default",
            "upgrade":{"model":"gpt-5.6-terra","migrationMarkdown":"Move to Terra"},
            "inputModalities":["text","image"], "supportsPersonality":true
        }))
        .unwrap();
        assert_eq!(model.id, "gpt-5.6-sol");
        assert!(model.is_default);
        assert!(model.image_input);
        assert_eq!(model.supported_reasoning_efforts, ["low", "high"]);
        assert_eq!(model.service_tiers[0].id, "priority");
        assert_eq!(model.upgrade_model.as_deref(), Some("gpt-5.6-terra"));
        assert!(model.supports_personality);
    }

    #[test]
    fn permission_modes_map_to_safe_sandboxes() {
        assert_eq!(sandbox_policy("plan", Some("C:\\repo"))["type"], "readOnly");
        assert_eq!(
            sandbox_policy("default", Some("C:\\repo"))["type"],
            "workspaceWrite"
        );
        assert_eq!(
            sandbox_policy("bypassPermissions", None)["type"],
            "dangerFullAccess"
        );
        assert_eq!(effort_name(true, Some("ultra")), "xhigh");
        assert_eq!(effort_name(true, Some("max")), "max");
        assert_eq!(effort_name(true, Some("agentic")), "ultra");
        assert_eq!(effort_name(false, Some("ultra")), "low");
    }

    #[test]
    fn fast_tier_uses_camel_case_and_requires_provider_confirmation() {
        let mut params = json!({ "threadId": "thread-1" });
        insert_service_tier(&mut params, Some("priority"));
        assert_eq!(params["serviceTier"], "priority");
        assert!(thread_confirmed_fast(
            &json!({ "thread": { "serviceTier": "priority" } }),
            true
        ));
        assert!(!thread_confirmed_fast(
            &json!({ "thread": { "serviceTier": "default" } }),
            true
        ));
        assert!(!thread_confirmed_fast(
            &json!({ "thread": { "serviceTier": "priority" } }),
            false
        ));
    }

    #[test]
    fn ask_user_answers_are_keyed_for_app_server() {
        let questions = json!([{"id":"choice","question":"Pick one"}]);
        let answer = json!({"answers":[{"question":"Pick one","answer":"A"}]});
        assert_eq!(
            codex_answers(Some(&questions), &answer),
            json!({"choice":{"answers":["A"]}})
        );
    }

    #[test]
    fn account_overview_preserves_plan_limits_and_usage() {
        let overview = parse_account_overview(
            &json!({
                "account":{"type":"chatgpt","email":"user@example.com","planType":"free"},
                "requiresOpenaiAuth":true
            }),
            Ok(json!({
                "rateLimitsByLimitId":{"codex":{"limitId":"codex","primary":{
                    "usedPercent":25.5,"windowDurationMins":300,"resetsAt":1785654000
                }}},
                "rateLimitResetCredits":{"availableCount":2}
            })),
            Ok(json!({"summary":{"lifetimeTokens":1234,"currentStreakDays":8}})),
        );

        assert_eq!(overview.plan_type.as_deref(), Some("free"));
        assert_eq!(overview.email.as_deref(), Some("user@example.com"));
        assert_eq!(overview.rate_limits[0].primary.as_ref().unwrap().used_percent, 25.5);
        assert_eq!(overview.reset_credits_available, Some(2));
        assert_eq!(overview.usage.unwrap().current_streak_days, Some(8));
    }

    #[test]
    fn account_overview_keeps_partial_failures_visible() {
        let overview = parse_account_overview(
            &json!({"account":{"type":"chatgpt"},"requiresOpenaiAuth":true}),
            Err("rate limit probe failed".into()),
            Err("usage probe failed".into()),
        );

        assert!(overview.rate_limits.is_empty());
        assert_eq!(overview.rate_limits_error.as_deref(), Some("rate limit probe failed"));
        assert_eq!(overview.usage_error.as_deref(), Some("usage probe failed"));
    }

    #[test]
    fn only_a_leading_safe_dollar_token_invokes_a_skill() {
        assert_eq!(leading_skill_name("  $quick-review inspect this"), Some("quick-review"));
        assert_eq!(leading_skill_name("explain $quick-review"), None);
        assert_eq!(leading_skill_name("$bad/path inspect"), None);
    }

    #[test]
    fn file_changes_become_one_visible_tool_per_path() {
        let changes = json!([
            {"path":"src/new.ts","diff":"export const ready = true;\n","kind":{"type":"add"}},
            {"path":"src/edit.ts","diff":"@@ -1 +1 @@\n-old\n+new\n","kind":{"type":"update","move_path":null}},
            {"path":"src/old.ts","diff":"gone\n","kind":{"type":"delete"}}
        ]);
        let tools = file_change_tool_uses("change-1", Some(&changes));

        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].id, "change-1");
        assert_eq!(tools[0].name, "Write");
        assert_eq!(tools[0].input["file_path"], "src/new.ts");
        assert_eq!(tools[1].id, "change-1:1");
        assert_eq!(tools[1].name, "Edit");
        assert_eq!(tools[1].input["unified_diff"], "@@ -1 +1 @@\n-old\n+new\n");
        assert_eq!(tools[2].id, "change-1:2");
        assert_eq!(tools[2].name, "Delete");
        assert_eq!(tools[2].input["codex_diff_kind"], "delete");
    }

    #[test]
    fn nullable_mcp_error_does_not_turn_success_red() {
        assert!(!item_failed(&json!({"status":"completed","error":null})));
        assert!(item_failed(&json!({"status":"failed","error":null})));
        assert!(item_failed(&json!({"status":"completed","error":{"message":"boom"}})));
        assert!(item_failed(&json!({"status":"completed","success":false})));
    }

    #[test]
    fn current_turn_plan_arrays_render_as_markdown() {
        let markdown = turn_plan_markdown(&json!({
            "explanation":"Release work",
            "plan":[
                {"step":"Implement","status":"completed"},
                {"step":"Verify","status":"inProgress"}
            ]
        }))
        .unwrap();

        assert_eq!(markdown, "Release work\n\n- [x] Implement\n- [ ] Verify");
    }

    #[test]
    fn stale_active_guard_cannot_remove_a_newer_turn() {
        let id = "550e8400-e29b-41d4-a716-4466554400b1";
        let (_old_token, old_guard) = register_turn(id);
        let (new_token, new_guard) = register_turn(id);
        drop(old_guard);

        assert!(cancel_codex_session(id));
        assert!(new_token.is_cancelled());
        drop(new_guard);
    }
}
