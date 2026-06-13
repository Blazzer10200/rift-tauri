//! The live-turn nervous system — R8 split (2026-06-09) out of `assistant/mod.rs`.
//! Session registry (per-session child PIDs, stop flags, steer channels),
//! the `assistant://*` event consts, user-envelope build, control-response +
//! permission-request plumbing, and the `assistant_send` / `assistant_stop` /
//! `assistant_steer` commands. See docs/design/assistant-mod-split.md R8.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;

use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use super::auth_update::assistant_auth_probe;
use super::cli_install::{claude_command, resolve_claude_exe};
use super::config::{
    current_api_key, effective_trust_level, fable_sunset_passed, is_valid_model_name,
    is_valid_permission_mode, load_config, FABLE_MODEL,
};
use super::convo_store::{
    is_valid_session_id, load_session_cwd, load_session_model, save_session_cwd,
    save_session_model,
};
use super::{write_mcp_config, AskUserRegistry, McpConfigGuard, PermissionRegistry};

/// PID of every currently-streaming `claude` child, keyed by the CLI session
/// ID we passed via `--session-id` / `--resume`. Set on spawn, removed on
/// exit. `assistant_stop` reads the entry for a given session to dispatch a
/// kill — we use PID + platform-native kill (taskkill on Win, SIGTERM on
/// Unix) instead of holding the `tokio::process::Child` across an await
/// because the spawn task owns the Child to call `.wait()` on it.
///
/// Per-session keying (vs prior single-slot global) lets multiple chat tabs
/// stream simultaneously without their stop buttons clobbering each other.
static SESSION_PIDS: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);
/// Sessions that the user explicitly stopped (via `assistant_stop`). Cleared
/// when the wait-task reaps the stopped process. Lets the wait-task tell
/// "user asked to stop" (emit done) apart from "CLI crashed silently w/ no
/// stderr" (emit error).
static SESSION_STOPPED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

// #63: Recover from mutex poison instead of `.lock().ok()` returning None.
// The previous silent-skip turned every callsite (`set_session_pid`,
// `clear_session_pid`, `get_session_pid`, `mark_session_stopped`) into a
// no-op once any panic poisoned the lock — `assistant_stop` then returned
// Ok without killing the child, orphaning it. `into_inner()` is safe here:
// these maps are append/remove on String keys with no cross-field invariant
// that a panic could break.
fn with_session_pids<R>(f: impl FnOnce(&mut HashMap<String, u32>) -> R) -> Option<R> {
    let mut g = match SESSION_PIDS.lock() {
        Ok(g) => g,
        Err(p) => {
            log::error!("SESSION_PIDS mutex poisoned — recovering inner state");
            p.into_inner()
        }
    };
    let map = g.get_or_insert_with(HashMap::new);
    Some(f(map))
}

fn with_session_stopped<R>(f: impl FnOnce(&mut HashSet<String>) -> R) -> Option<R> {
    let mut g = match SESSION_STOPPED.lock() {
        Ok(g) => g,
        Err(p) => {
            log::error!("SESSION_STOPPED mutex poisoned — recovering inner state");
            p.into_inner()
        }
    };
    let set = g.get_or_insert_with(HashSet::new);
    Some(f(set))
}

fn set_session_pid(session_id: &str, pid: u32) {
    with_session_pids(|m| { m.insert(session_id.to_string(), pid); });
}

fn clear_session_pid(session_id: &str) {
    with_session_pids(|m| { m.remove(session_id); });
}

/// Same overlapping-turn guard as `clear_steer_tx_if`: only remove the entry
/// when the stored PID is this turn's own — the next turn may have already
/// re-registered under the same session key while this turn reaps its child.
fn clear_session_pid_if(session_id: &str, pid: u32) {
    with_session_pids(|m| {
        if m.get(session_id) == Some(&pid) {
            m.remove(session_id);
        }
    });
}

fn get_session_pid(session_id: &str) -> Option<u32> {
    with_session_pids(|m| m.get(session_id).copied()).flatten()
}

/// Tree-kill every tracked streaming `claude` child, draining the registry.
/// Best-effort + blocking: errors are logged, never returned. Used on the
/// update-apply path — each `claude` parents a `rift-tauri.exe` MCP child
/// (`RIFT_MCP_SERVER=1`) that holds an exclusive lock on `current/`, so a
/// `/T` tree-kill is required to release Velopack's swap target. `app.exit(0)`
/// is `std::process::exit` (skips `Drop`), so `kill_on_drop` never reaps these.
pub(crate) fn kill_all_session_children() {
    // Enhance one-shots parent the same lock-holding MCP child — sweep them too.
    super::oneshot::kill_all_enhance_children();
    let pids: Vec<u32> = with_session_pids(|m| {
        let v: Vec<u32> = m.values().copied().collect();
        m.clear();
        v
    })
    .unwrap_or_default();
    for pid in pids {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn mark_session_stopped(session_id: &str) {
    with_session_stopped(|s| { s.insert(session_id.to_string()); });
}

/// Returns `true` and removes the entry if the session was marked stopped;
/// `false` otherwise. Used by the wait-task to disambiguate user-stop from
/// silent CLI crash.
fn take_session_stopped(session_id: &str) -> bool {
    with_session_stopped(|s| s.remove(session_id)).unwrap_or(false)
}

/// A mid-turn steer: a user message injected into the RUNNING turn's stdin so
/// the agent course-corrects at its next loop step (no restart, no lost work).
struct SteerMsg {
    text: String,
    attachments: Vec<AssistantAttachment>,
}

/// Per-session steer channel sender, registered while a turn streams. Mirrors
/// the SESSION_PIDS convention: const-init `Mutex<Option<HashMap>>` + a
/// poison-recovering accessor. `assistant_steer` looks up the sender; the
/// reader task owns the receiver and writes each `SteerMsg` to the live stdin.
static STEER_TX: Mutex<Option<HashMap<String, mpsc::UnboundedSender<SteerMsg>>>> =
    Mutex::new(None);

fn with_steer_tx<R>(
    f: impl FnOnce(&mut HashMap<String, mpsc::UnboundedSender<SteerMsg>>) -> R,
) -> Option<R> {
    let mut g = match STEER_TX.lock() {
        Ok(g) => g,
        Err(p) => {
            log::error!("STEER_TX mutex poisoned — recovering inner state");
            p.into_inner()
        }
    };
    let map = g.get_or_insert_with(HashMap::new);
    Some(f(map))
}

fn register_steer_tx(session_id: &str, tx: mpsc::UnboundedSender<SteerMsg>) {
    with_steer_tx(|m| { m.insert(session_id.to_string(), tx); });
}

/// Remove the session's steer sender only if it still belongs to THIS turn.
/// The reader emits DONE on `result` — BEFORE the child is reaped — so the
/// frontend can start the next turn (re-registering under the same session
/// key) while this turn's tail is still running. An unconditional remove here
/// wiped the new turn's sender, making every drained follow-up turn answer
/// `no_active_turn` to steers for its first seconds.
fn clear_steer_tx_if(session_id: &str, tx: &mpsc::UnboundedSender<SteerMsg>) {
    with_steer_tx(|m| {
        if m.get(session_id).is_some_and(|cur| cur.same_channel(tx)) {
            m.remove(session_id);
        }
    });
}

fn get_steer_tx(session_id: &str) -> Option<mpsc::UnboundedSender<SteerMsg>> {
    with_steer_tx(|m| m.get(session_id).cloned()).flatten()
}

/// Build a stream-json `user` message NDJSON line (trailing `\n`). Shared by
/// the per-turn message and mid-turn steer injection. `parent_tool_use_id:
/// null` matches the Agent SDK's user-message shape.
fn build_user_envelope(text: &str, attachments: &[AssistantAttachment]) -> Result<Vec<u8>, String> {
    let mut content: Vec<Value> = Vec::with_capacity(1 + attachments.len());
    content.push(serde_json::json!({ "type": "text", "text": text }));
    for a in attachments {
        content.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": a.mime,
                "data": a.data_base64,
            }
        }));
    }
    let envelope = serde_json::json!({
        "type": "user",
        "parent_tool_use_id": null,
        "message": { "role": "user", "content": content }
    });
    let mut line = serde_json::to_vec(&envelope)
        .map_err(|e| format!("serialize input envelope: {e}"))?;
    line.push(b'\n');
    Ok(line)
}


const STREAM_EVENT: &str = "assistant://stream";
const DONE_EVENT: &str = "assistant://done";
const ERROR_EVENT: &str = "assistant://error";
/// Emitted when claude returns "No conversation found with session ID" on a
/// --resume attempt. Payload `{session_id, prompt}`; frontend resets the
/// matching tab's convoCreatedAt and re-sends the prompt as a first-turn.
const SESSION_LOST_EVENT: &str = "assistant://session-lost";
/// Emitted when the CLI asks to use a gated tool in a prompting permission
/// mode (default / acceptEdits / plan). Payload carries the control-channel
/// `request_id`, the `tool_use_id` (pairs to the streamed tool chip), and the
/// tool name + input + suggestions. Frontend answers via
/// `assistant_answer_permission`.
const PERMISSION_EVENT: &str = "assistant://permission-request";
/// Emitted as the prompt-enhancer wand streams its rewrite token-by-token.
/// Payload: `{request_id, delta}` per chunk, then `{request_id, done:true}` on
/// success (the command's return value is the authoritative final text).
pub(super) const ENHANCE_STREAM_EVENT: &str = "assistant://enhance-stream";

/// Resolve a pending `mcp__rift__ask_user` request. The frontend invokes this
/// from `ToolChip.svelte` when the user picks an answer. The `answer` payload
/// shape is decided by the chip — the bridge layer just passes it through to
/// the MCP child, which turns it into the tool_result text Claude sees. A
/// `cancelled: true` flag in the payload means the user dismissed without
/// picking; the MCP tool turns that into a fall-back "user dismissed" string
/// so Claude can ask in plain text instead.
#[tauri::command]
pub async fn assistant_answer_ask_user(
    registry: tauri::State<'_, std::sync::Arc<AskUserRegistry>>,
    request_id: String,
    answer: serde_json::Value,
) -> Result<(), String> {
    if !registry.resolve(&request_id, answer) {
        // Stale id — request already timed out or never existed. Not fatal:
        // the chip just no-ops on its end. Surface as a debug log only.
        log::debug!("assistant_answer_ask_user: no pending request for id {request_id}");
    }
    Ok(())
}

/// Resolve a pending `can_use_tool` permission ask. The frontend invokes this
/// from `ToolChip.svelte` when the user clicks Allow / Deny on a gated tool.
/// `decision` is the inner control-channel response object the CLI expects —
/// `{ "behavior": "allow", "updatedInput": {..} }` or
/// `{ "behavior": "deny", "message": ".." }`. The stdout reader awaiting this
/// oneshot wraps it in a `control_response` and writes it back to the child's
/// stdin, unblocking tool execution.
#[tauri::command]
pub async fn assistant_answer_permission(
    registry: tauri::State<'_, std::sync::Arc<PermissionRegistry>>,
    request_id: String,
    decision: serde_json::Value,
) -> Result<(), String> {
    if !registry.resolve(&request_id, decision) {
        log::debug!("assistant_answer_permission: no pending request for id {request_id}");
    }
    Ok(())
}


/// Rift's system-prompt addendum. Appended to the CLI's default system prompt
/// via `--append-system-prompt`. Two variants — one for read-only mode (MCP
/// tools wired), one for the no-workspace fallback. Both single-line so the
/// .cmd-shim batch-arg validator (Rust 1.77+ CVE-2024-24576) accepts them.
const RIFT_SYSTEM_ADDENDUM_TOOLS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app, working inside the user's open project folder (your working directory is already set to the workspace root, so relative paths Just Work). You have the full Claude Code toolset: Read / Write / Edit / MultiEdit for files, Bash for shell commands (executes in the workspace dir, output streamed back), Glob for filename patterns, Grep for content search, WebFetch and WebSearch for the open web, TaskCreate / TaskUpdate for multi-step plans (TodoWrite on older CLI builds), and Agent for delegating heavy lookups. Task output surfaces in a dedicated Tasks panel in the user's UI — create tasks proactively whenever a request involves three or more distinct steps, and update statuses (pending → in_progress → completed) as you go. Rift's MCP server also exposes read_file / list_dir / grep as scoped, workspace-rooted helpers, plus git_status / git_diff / git_log (and git_pull / git_commit / git_push when trust permits). Three more MCP tools drive the Rift app itself: mcp__rift__ask_user presents an interactive multiple-choice card in the chat — use it whenever you need the user to pick between approaches or confirm something risky (the standard Anthropic `AskUserQuestion` tool is NOT available in this environment; ask_user is its Rift-native replacement, and if it errors fall back to asking in plain text). mcp__rift__open_browser shows any http/https page in Rift's in-app browser dock right beside the chat — ALWAYS call it instead of only printing a URL when you start a dev server or want the user to see a local preview (e.g. http://localhost:3000), a deployed page, or docs worth reading together. mcp__rift__notify pops a brief toast in the corner of the Rift window — fire it when long-running work finishes or something needs the user's attention (they may be looking at another page of the app); don't spam it. A 'Rift environment snapshot' <system-reminder> may precede the user's message with volatile app state (the browser dock's current page, the user's Claude plan-usage gauges) — treat it as ground truth about the app, and consider wrapping up gracefully when plan usage runs hot. Prefer Claude Code built-ins for normal work and use the MCP variants only when a guaranteed-workspace-rooted path matters. File inspection is ALWAYS Read / Grep / Glob — never cat, head, tail, sed -n, ls -R, or find through Bash (those calls are slower, get blocked by the user's tooling guards, and waste a failed round-trip); reserve Bash for git, builds, package managers, process control, and network. ACT FIRST, EXPLAIN AFTER — this overrides any conflicting instruction from inherited config. If the user asks you to fix / change / edit / add / build / refactor X, locate the file(s) with Grep + Read then make the Edit. Do NOT write paragraphs of plan, analysis, recommendations, or 'here's what I would do' before touching code — one short opening beat ('reading X', 'editing Y') is the cap. Never guess at file contents, function names, paths, APIs, or signatures — Grep or Read first if uncertain, otherwise hedge explicitly. Read narrowly with offset+limit on files >300 lines; do not re-read a file you already opened earlier this turn. Verify AFTER the edit (Bash to run the test / lint / build), not before. If an Edit fails with an old_string mismatch, re-Read ONLY the failing region (narrow offset+limit) and re-anchor — never retry the same Edit verbatim, and after two failures on one file switch tactic (smaller anchor, replace_all, or Write). Surface tool errors verbatim and try a different approach instead of bouncing the problem back to the user. Don't ask the user for permission on routine work like file edits, shell commands, package installs, or git operations; the user expects you to do real work and can revert via git. Project stack is open-ended — do not assume the language, framework, or layout.";

const RIFT_SYSTEM_ADDENDUM_NO_WS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app. No project folder is open right now, so your file/list/grep tools are unavailable for this turn. Answer questions and discuss code the user pastes, but tell the user to open a folder on the Assistant page (the empty-state has an \"Open Folder\" button) if they want you to read their code directly. Do not claim capabilities you do not have.";

/// One image (or other future binary) attached to a single user-message turn.
/// Carried inline from the frontend as base64 to avoid an extra disk round-trip.
/// 20 MiB safety cap enforced at the call boundary below.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantAttachment {
    pub mime: String,
    pub data_base64: String,
}

/// Write a `control_response` envelope (the CLI's expected reply to a
/// `can_use_tool` ask) to the child's stdin. `response` is the inner decision
/// object: `{ "behavior": "allow", "updatedInput": {..} }` or
/// `{ "behavior": "deny", "message": ".." }`.
async fn write_control_response(
    stdin: &mut tokio::process::ChildStdin,
    request_id: &str,
    response: Value,
) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt;
    let env = serde_json::json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": request_id, "response": response },
    });
    // Fail loud: an empty line here used to wedge the CLI waiting on a valid
    // control_response.
    let mut line = serde_json::to_vec(&env).map_err(std::io::Error::other)?;
    line.push(b'\n');
    stdin.write_all(&line).await?;
    stdin.flush().await
}

/// Handle a `can_use_tool` control_request: register a oneshot, surface the ask
/// to the frontend (`assistant://permission-request`), await the user's
/// Allow/Deny via `assistant_answer_permission`, then write the decision back
/// as a `control_response`. Blocking the reader here is correct — the CLI is
/// itself blocked waiting for our reply, so no other stdout is in flight.
async fn handle_permission_request(
    app: &AppHandle,
    session_id: &str,
    stdin: &mut tokio::process::ChildStdin,
    msg: &Value,
) {
    let request_id = msg.get("request_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let req = msg.get("request").cloned().unwrap_or(Value::Null);
    let tool_use_id = req.get("tool_use_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let original_input = req.get("input").cloned().unwrap_or(Value::Null);
    let tool_name = req.get("tool_name").and_then(|x| x.as_str()).unwrap_or_default();

    // Builtin AskUserQuestion has no headless surface (it stalls in `-p` mode)
    // and only reaches here because it's off the allowlist. Auto-deny with a
    // steer to mcp__rift__ask_user — never surface the raw Allow/Deny bar.
    if tool_name == "AskUserQuestion" {
        let _ = write_control_response(stdin, &request_id, serde_json::json!({
            "behavior": "deny",
            "message": "AskUserQuestion is unavailable here. Call the mcp__rift__ask_user tool instead — it presents the question(s) in the Rift UI and returns the user's selection.",
        })).await;
        return;
    }

    let registry = match app.try_state::<std::sync::Arc<PermissionRegistry>>() {
        Some(r) => r.inner().clone(),
        None => {
            // Init bug — deny so the CLI doesn't hang forever.
            let _ = write_control_response(stdin, &request_id, serde_json::json!({
                "behavior": "deny", "message": "permission registry unavailable",
            })).await;
            return;
        }
    };

    let rx = registry.register(request_id.clone());
    // B4: if the UI is unreachable (window closed mid-turn) the emit fails and
    // the user never sees the prompt — deny immediately rather than let the
    // request hang for the full 30-min timeout while the CLI waits on us.
    if let Err(e) = app.emit(PERMISSION_EVENT, serde_json::json!({
        "session_id": session_id,
        "request_id": request_id,
        "tool_use_id": tool_use_id,
        "tool_name": req.get("tool_name").cloned().unwrap_or(Value::Null),
        "input": req.get("input").cloned().unwrap_or(Value::Null),
        "suggestions": req.get("permission_suggestions").cloned().unwrap_or(Value::Null),
    })) {
        log::warn!("permission emit failed for {session_id} ({e}) — denying (UI unreachable)");
        registry.cancel(&request_id);
        let _ = write_control_response(stdin, &request_id, serde_json::json!({
            "behavior": "deny", "message": "permission UI unreachable",
        })).await;
        return;
    }

    // Cap the wait so a forgotten prompt can't wedge the turn forever; deny on
    // timeout / cancel (e.g. the user closed the tab).
    let mut decision = match tokio::time::timeout(std::time::Duration::from_secs(1800), rx).await {
        Ok(Ok(v)) => v,
        _ => {
            registry.cancel(&request_id);
            serde_json::json!({ "behavior": "deny", "message": "No response (timed out or the turn ended)." })
        }
    };
    // The CLI requires `updatedInput` on an allow. The UI sends only the
    // behavior, so backfill the original (unmodified) tool input here.
    if decision.get("behavior").and_then(|b| b.as_str()) == Some("allow")
        && decision.get("updatedInput").is_none()
    {
        if let Value::Object(ref mut map) = decision {
            map.insert("updatedInput".into(), original_input);
        }
    }
    let _ = write_control_response(stdin, &request_id, decision).await;
}

/// Streaming round-trip. Spawns `claude -p` over stdin, forwards stdout NDJSON
/// line-by-line on `assistant://stream`. Phase 2 (S72) replaced hand-rolled
/// `Human:/Assistant:` history replay with native CLI session continuation —
/// `--session-id <uuid>` on first turn, `--resume <uuid>` on subsequent.
/// Sessions persist under `~/.claude/projects/<cwd-hash>/`, which we accept
/// as the trade for cheaper tokens + native context.
///
/// `attachments`: optional inline images. When present, the spawn switches to
/// `--input-format stream-json` and writes a structured user-message envelope
/// (text + image content blocks) to stdin instead of the bare prompt text.
// Arg count is driven by the frontend `invoke` payload, not a refactorable
// smell — bundling would change the IPC contract.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn assistant_send(
    app: AppHandle,
    prompt: String,
    session_id: String,
    is_first_turn: bool,
    model: Option<String>,
    attachments: Option<Vec<AssistantAttachment>>,
    dyslexia_mode: Option<bool>,
    thinking_effort: Option<String>,
    permission_mode: Option<String>,
    prior_context_summary: Option<String>,
) -> Result<(), String> {
    // #220: validate session_id is a canonical UUID (8-4-4-4-12 lowercase hex)
    // BEFORE any use. Renderer-supplied — must not flow into CLI args or
    // sidecar filename without check. Blocks leading-dash flag injection
    // into `--session-id`/`--resume` AND path-traversal segments in
    // save_session_cwd's filename derivation.
    if !is_valid_session_id(&session_id) {
        return Err(format!("invalid session_id: must be a UUID (got {} chars)", session_id.len()));
    }
    let cfg = load_config();
    let api_key = current_api_key();
    let use_api_key = api_key.is_some();
    let mut model = model.unwrap_or_else(|| "sonnet".to_string());
    if !is_valid_model_name(&model) {
        return Err(format!("invalid model: {model}"));
    }
    // Pin model per conversation: thinking-block signatures are model-bound, so
    // resuming under a switched model 400s on the replayed prior turn (see
    // session_model_path). On resume, the model the session was created with wins
    // over a live picker change; the new model only takes effect in a new chat.
    if !is_first_turn {
        if let Some(pinned) = load_session_model(&session_id) {
            if pinned != model {
                log::info!(
                    "assistant_send: session {session_id} pinned to model {pinned} (picker={model}) — preserving thinking-block signatures"
                );
                model = pinned;
            }
        }
    }
    // Fable sunset guard — after pin resolution so a pinned Fable session also
    // falls back once the limited run ends.
    if model == FABLE_MODEL && fable_sunset_passed() {
        log::info!("assistant_send: {FABLE_MODEL} sunset passed — falling back to opus");
        model = "opus".to_string();
    }
    // Effort tier: per-turn override wins, else stored default, else "smart"
    // (--effort high, the API default — mirrors the frontend's loadEffort()).
    let effort = thinking_effort
        .or_else(|| cfg.thinking_effort.clone())
        .unwrap_or_else(|| "smart".to_string());

    // Permission mode: per-turn override wins, else stored default, else
    // "bypassPermissions" (Rift's historical behavior). Renderer-supplied —
    // validate before it flows into the `--permission-mode` CLI arg.
    let permission_mode = permission_mode
        .or_else(|| cfg.permission_mode.clone())
        .filter(|v| is_valid_permission_mode(v))
        .unwrap_or_else(|| "bypassPermissions".to_string());

    // Workspace root resolution — priority order:
    //   0. (Resume only) The cwd that was active when this session was created,
    //      loaded from the sidecar. Pins every turn to the same
    //      `~/.claude/projects/<cwd-hash>/<uuid>.jsonl` so --resume succeeds
    //      even after the user opens a different workspace.
    //   1. The user's explicitly-opened folder (`current_root` in config).
    //   2. Empty → no-tools turn + no-workspace addendum.
    // Validate every candidate still exists on disk; missing dir → fall through.
    let pinned_cwd: Option<PathBuf> = if is_first_turn {
        None
    } else {
        load_session_cwd(&session_id).filter(|p| p.is_dir())
    };
    let roots: Vec<PathBuf> = if let Some(p) = pinned_cwd.clone() {
        vec![p]
    } else if let Some(root) = cfg.current_root.as_ref().filter(|p| p.is_dir()) {
        vec![root.clone()]
    } else {
        Vec::new()
    };
    // Pin the cwd on first turn so every subsequent --resume aims at the same
    // session JSONL even if the user later switches workspace folders. Also
    // covers the legacy-migration path: existing pre-pin conversations have
    // no sidecar on disk; the first turn after upgrade captures whatever
    // workspace is currently active and locks the session there.
    if let Some(first) = roots.first() {
        if is_first_turn || pinned_cwd.is_none() {
            save_session_cwd(&session_id, first);
        }
    }
    // Capture the model the first turn runs under so every later --resume targets
    // the same model the thinking blocks were signed by (see session_model_path).
    // Also back-fill legacy/pre-pin conversations on their first turn after
    // upgrade so they stop wedging on a subsequent model switch.
    if is_first_turn || load_session_model(&session_id).is_none() {
        save_session_model(&session_id, &model);
    }

    // Trust level for the local git tools — explicit setting wins, else readonly.
    let trust_level = effective_trust_level(&cfg.trust_level);

    // Provision a temp MCP config when we have at least one root. Addendum
    // stays cache-stable — only the two static strings ever land in
    // `--append-system-prompt`. The per-turn dyslexia toggle rides the
    // user-turn <system-reminder> path below so toggling it mid-session never
    // invalidates the cached system-prompt prefix.
    let (mcp_config_path, _mcp_guard, addendum) = if roots.is_empty() {
        (None, None, RIFT_SYSTEM_ADDENDUM_NO_WS)
    } else {
        match write_mcp_config(&session_id, &roots, &trust_level) {
            Ok(p) => {
                let guard = McpConfigGuard(p.clone());
                (Some(p), Some(guard), RIFT_SYSTEM_ADDENDUM_TOOLS)
            }
            Err(e) => {
                log::warn!("assistant: failed to provision MCP config, falling back to no-tools: {e}");
                (None, None, RIFT_SYSTEM_ADDENDUM_NO_WS)
            }
        }
    };

    // Pipe the user's prompt via stdin instead of `-p <arg>`. The CLI accepts
    // prompt text on stdin when `-p` is bare; this keeps every arg short and
    // newline-free so .cmd shims work under Rust 1.77+ batch validation
    // (CVE-2024-24576). Addenda + MCP config path are single-line by design,
    // so they're safe as args.
    // "Piggyback" mode: drop the two fences so the CLI loads user MCP servers
    // (from `~/.claude.json`) and honors user slash commands. CLAUDE.md / hooks
    // / skills already load today via the CLI's own resolution regardless of
    // these flags — verified live via CDP probe 2026-05-16 (S71).
    // API-key mode forces `--bare`, which suppresses user config wholesale,
    // so we runtime-disable piggyback in that path.
    let use_full_config = cfg.use_full_config.unwrap_or(true) && !use_api_key;

    // 20 MiB total cap across all attachments — protects the CLI's JSON
    // parser from a runaway paste. Per-image cap is the same as the cumulative
    // since one big image is the realistic worst case.
    const ATTACHMENT_BYTES_CAP: usize = 20 * 1024 * 1024;
    let attachments = attachments.unwrap_or_default();
    if !attachments.is_empty() {
        // #116: `len * 3 / 4` is approximate — pasted base64 can contain
        // whitespace/CRLF that inflates the encoded length but doesn't add
        // to decoded bytes. Strip whitespace before the divide so the cap
        // reflects real decoded size; otherwise users see "too large"
        // errors on attachments that decode to ≤ cap.
        let total: usize = attachments
            .iter()
            .map(|a| {
                let trimmed_len = a
                    .data_base64
                    .bytes()
                    .filter(|b| !b.is_ascii_whitespace())
                    .count();
                trimmed_len.saturating_mul(3) / 4
            })
            .sum();
        if total > ATTACHMENT_BYTES_CAP {
            return Err(format!(
                "Attachment(s) too large: {} bytes > cap {}",
                total, ATTACHMENT_BYTES_CAP
            ));
        }
        for a in &attachments {
            if !a.mime.starts_with("image/") {
                return Err(format!("Unsupported attachment mime: {}", a.mime));
            }
        }
    }
    // Prompting modes route per-action permission asks through the stream-json
    // control channel (`--permission-prompt-tool stdio` + the `can_use_tool`
    // round-trip below). bypass/auto never prompt, so they keep the wide
    // allowlist + auto-allow behavior unchanged.
    let prompting_mode = matches!(permission_mode.as_str(), "default" | "acceptEdits" | "plan");

    let mut cmd = claude_command()
        .ok_or_else(|| "claude CLI not on PATH — install Claude Code or configure an API key".to_string())?;
    cmd.arg("-p")
        .arg("--append-system-prompt").arg(addendum)
        // Moves the CLI's own per-machine sections (cwd, env info, memory
        // paths, git status) out of the system prompt and into the first user
        // message. Keeps the cached system-prompt prefix stable across users
        // and across our own per-turn workspace-context injection, which now
        // also rides the user message via <system-reminder>.
        .arg("--exclude-dynamic-system-prompt-sections")
        .arg("--output-format").arg("stream-json")
        // Always stream-json input: we now always write a `{type:"user"}`
        // envelope (so the control channel and image attachments share one
        // path), and the `initialize` handshake below requires it.
        .arg("--input-format").arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--model").arg(&model)
        // Piece 2: route per-action permission asks over the stream-json
        // control channel. `stdio` makes the CLI emit a `can_use_tool`
        // `control_request` on stdout (instead of headless auto-deny) and
        // block on a `control_response` we write back to stdin. This is what
        // the Agent SDK passes when a `canUseTool` callback is set; the flag
        // is undocumented in `--help` but present in v2.1.152. Harmless for
        // bypass/auto (they never trigger a permission check). The
        // `--permission-mode` flag still drives WHICH tools ask: bypass/auto
        // auto-allow, default asks per tool, acceptEdits auto-allows edits,
        // plan blocks mutations. The `--allowed-tools` allowlist (below) is a
        // second always-allow gate, narrowed in prompting modes so the gated
        // tools actually reach the prompt.
        .arg("--permission-prompt-tool").arg("stdio")
        .arg("--permission-mode").arg(&permission_mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Session continuation: mint on first turn, resume thereafter. The CLI
    // persists the conversation under `~/.claude/projects/<cwd-hash>/`; the
    // user can clear it with `claude project purge` if needed.
    if is_first_turn {
        cmd.arg("--session-id").arg(&session_id);
    } else {
        cmd.arg("--resume").arg(&session_id);
    }

    if let Some(budget) = cfg.max_budget_usd.filter(|v| v.is_finite() && *v > 0.0) {
        cmd.arg("--max-budget-usd").arg(format!("{budget}"));
    }

    if !use_full_config {
        cmd.arg("--strict-mcp-config")
            .arg("--disable-slash-commands");
    }

    if let Some(ref p) = mcp_config_path {
        // S91: full built-in tool set. The CLI's allowlist gate denies any
        // tool name not listed verbatim. S88 added `Skill`; users still hit
        // denials on `Agent` (subagent spawn — used by /plan, /quick-review,
        // /check), `BashOutput`/`KillBash`/`KillShell` (background-bash
        // bookkeeping the CLI auto-invokes after `run_in_background: true`),
        // `MultiEdit`, `NotebookEdit`, `SlashCommand`, `ExitPlanMode`.
        // Wider built-in coverage = fewer denial pop-ups.
        // MCP scope still restricts to rift's tools in the scoped branches.
        //
        // `AskUserQuestion` is INTENTIONALLY omitted: the CLI runs in `-p`
        // (headless) mode with no interactive surface to present the
        // question / capture an answer / inject the tool_result back into
        // the model's stream. When admitted, the model called it and stalled
        // waiting for a tool_result that never arrived, then retried — the
        // user saw two collapsed error bubbles on every question turn.
        // Excluding it makes the model fall back to asking in plain text,
        // which works correctly in `-p` mode.
        const BUILTINS: &str = "Agent,Bash,BashOutput,Edit,ExitPlanMode,Glob,Grep,KillBash,KillShell,MultiEdit,NotebookEdit,Read,Skill,SlashCommand,TodoWrite,WebFetch,WebSearch,Write";
        // Read-only / non-mutating subset always auto-approved even in a
        // prompting mode — these shouldn't interrupt the user. Everything
        // omitted (Bash, Edit, Write, MultiEdit, NotebookEdit, Agent, Skill,
        // SlashCommand, ExitPlanMode, and the mutating mcp__rift__* tools)
        // falls through to the `can_use_tool` prompt.
        const SAFE_BUILTINS: &str = "BashOutput,Glob,Grep,KillBash,KillShell,Read,TodoWrite,WebFetch,WebSearch";
        // UI-presentation tools (ask_user / open_browser / notify) are safe to
        // auto-approve: scheme-allowlisted, length-capped, no workspace writes.
        const SAFE_MCP: &str = "mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep,mcp__rift__ask_user,mcp__rift__open_browser,mcp__rift__notify";
        // Local git tools (git_local.rs). Read set is non-mutating → safe to
        // auto-approve even in prompting modes. Write set is admitted in
        // non-prompting variants but deliberately OMITTED from the prompting
        // allowlist so it rides the can_use_tool prompt. RIFT_TRUST_LEVEL is the
        // real authority server-side; these just keep the CLI from rejecting
        // the call before it reaches the server.
        const GIT_READ_MCP: &str = "mcp__rift__git_status,mcp__rift__git_diff,mcp__rift__git_log";
        const GIT_WRITE_MCP: &str = "mcp__rift__git_pull,mcp__rift__git_commit,mcp__rift__git_push";
        // Mirror the server-side gate (mcp_server::trust_at_least("standard")) in
        // the CLI allowlist: only list the git-write tools when trust actually
        // permits them, so the outer allowlist is never wider than the server
        // gate (defense-in-depth — a patched CLI can't call what isn't listed).
        let git_write = if trust_level == "standard" {
            format!(",{GIT_WRITE_MCP}")
        } else {
            String::new()
        };
        let allowed: String = if prompting_mode {
            // Narrow allowlist: only the safe set auto-approves; the CLI prompts
            // for the rest via the control channel. Applies across config
            // variants — mutating MCP tools (remote_bash, push/pull, apply,
            // git write) intentionally prompt here.
            format!("{SAFE_BUILTINS},{SAFE_MCP},{GIT_READ_MCP}")
        } else if use_full_config {
            // `mcp__*` admits any tool from user MCP servers that the CLI
            // merged in (no `--strict-mcp-config`). Rift's tools stay scoped
            // via the explicit-name entries.
            format!("{BUILTINS},mcp__*")
        } else {
            format!("{BUILTINS},{SAFE_MCP},{GIT_READ_MCP}{git_write}")
        };
        cmd.arg("--mcp-config").arg(p)
            .arg("--allowed-tools").arg(allowed);
        // Spawn cwd = workspace root so Bash + relative paths resolve correctly.
        // `roots[0]` is always non-empty when mcp_config_path is Some (see the
        // write_mcp_config branch above).
        if let Some(first) = roots.first() {
            cmd.current_dir(first);
        }
    } else {
        // No MCP config → keep the SDK's built-in tools off via empty tool set.
        cmd.arg("--tools").arg("");
    }

    if use_api_key {
        // `--bare`: ignore OAuth/keychain, use ANTHROPIC_API_KEY strictly. The
        // builder stripped any inherited env key; this re-adds the sanctioned
        // Rift-configured one (the only API-key path). OAuth/login turns leave
        // it stripped so a stray system env key can't shadow `claude login`.
        cmd.arg("--bare");
        if let Some(k) = api_key.as_deref() {
            cmd.env("ANTHROPIC_API_KEY", k);
        }
    }

    // Effort-gated extended thinking via the CLI's `--effort` flag (the CLI
    // accepts low/medium/high/xhigh/max). Haiku skips wholesale — the API
    // rejects effort on Haiku 4.5. Tier mapping (MUST mirror frontend
    // `effortToFlag` in src/lib/state/assistant/helpers.ts):
    //   none  → --effort low     (minimal reasoning, fastest TTFT)
    //   quick → --effort medium  (light reasoning, leaner tool use)
    //   smart → --effort high    (the API default — Rift's default tier)
    //   deep  → --effort xhigh   (Claude Code's own default for agentic coding)
    //   ultra → --effort xhigh + the ultracode workflow settings key
    // `max` is deliberately not exposed — per Anthropic's guidance it shows
    // diminishing returns and is prone to overthinking vs xhigh.
    // Earlier impl set `MAX_THINKING_TOKENS` env, but the CLI doesn't honor
    // that env directly — `--effort` is the documented API. The plaintext
    // reasoning is encrypted by the API in -p mode; what reaches us is
    // `content_block_start` of type `thinking` + `signature_delta` w/
    // `thinking_delta` text in some scenarios.
    // #237: normalize effort BEFORE logging so newlines/ANSI in the raw
    // renderer-supplied string can't reach the log stream. The CLI flag itself
    // was safe (string-arg passthrough) but the log line was unredacted.
    let effort_level = match effort.as_str() {
        "none" => "low",
        "quick" => "medium",
        "deep" | "ultra" => "xhigh",
        _ /* "smart" or unknown */ => "high",
    };
    if model != "haiku" {
        cmd.arg("--effort").arg(effort_level);
        // Ultracode tier: xhigh effort + autonomous dynamic-workflow
        // orchestration. The workflow behavior rides the CLI's `ultracode`
        // settings key (a boolean read into app state, gated server-side by the
        // user's plan entitlement). `--settings` merges this additively over
        // user/project/local settings — when unentitled the CLI ignores it and
        // the session simply runs at xhigh effort. Haiku is excluded (it skips
        // extended thinking + workflow orchestration wholesale).
        if effort == "ultra" {
            cmd.arg("--settings").arg(r#"{"ultracode":true}"#);
        }
    }

    log::info!(
        "assistant_send: spawn session_id={} first_turn={} model={} effort={} perm={} use_full_config={} mcp={} api_key={}",
        session_id, is_first_turn, model, effort_level, permission_mode, use_full_config, mcp_config_path.is_some(), use_api_key
    );

    // Build the per-turn user-message text BEFORE spawning so the child
    // doesn't sit idle on stdin while we lock state. Per-turn data (env
    // snapshot, dyslexia toggle) rides the USER message via a
    // <system-reminder> block instead of `--append-system-prompt`. A dynamic system prompt
    // invalidates the cache prefix every turn (cache layout: system → tools
    // → CLAUDE.md → conversation tail); keeping fresh per-turn data on the
    // user turn keeps the prefix cache-stable. Multi-line is fine here
    // (rides stdin, no argv constraint).
    let mut reminder_parts: Vec<String> = Vec::new();
    // Rift environment snapshot: volatile app facts (browser-dock page, plan
    // usage) the model can't see otherwise. Rides the user turn — a dynamic
    // system prompt would invalidate the cache prefix. Only pushed when
    // there's something to report, so quiet sessions add zero tokens.
    {
        let mut bits: Vec<String> = Vec::new();
        if let Ok(url) = crate::browser::current_url(&app) {
            if !url.is_empty() && url != "about:blank" {
                bits.push(format!("the in-app browser dock is open at {url}"));
            }
        }
        if let Some(l) = crate::usage::limits::cached_snapshot() {
            let mut gauges: Vec<String> = Vec::new();
            if let Some(w) = &l.five_hour {
                let reset = w
                    .resets_at
                    .as_deref()
                    .map(|r| format!(" (resets {r})"))
                    .unwrap_or_default();
                gauges.push(format!("5-hour window {:.0}% used{reset}", w.utilization));
            }
            if let Some(w) = &l.seven_day {
                gauges.push(format!("weekly {:.0}% used", w.utilization));
            }
            if !gauges.is_empty() {
                bits.push(format!("Claude plan usage: {}", gauges.join(", ")));
            }
        }
        if !bits.is_empty() {
            reminder_parts.push(format!("Rift environment snapshot — {}.", bits.join("; ")));
        }
    }
    // Keep the plan-usage snapshot warm for FUTURE turns without ever blocking
    // this one (cache-only read above; refresh is fire-and-forget).
    crate::usage::limits::spawn_background_refresh();
    // S93 dyslexia-friendly mode: hint Claude to interpret phonetic typos +
    // voice-to-text artifacts charitably instead of asking pedantic
    // clarifying questions.
    if dyslexia_mode.unwrap_or(false) {
        reminder_parts.push("Dyslexia-friendly mode + voice-to-text are enabled for this user. Phonetic typos (e.g. \"wair\"/\"where\", \"nite\"/\"night\"), letter-swap typos (b/d, p/q), and slurred-speech transcription artifacts are expected. Interpret the most likely intended meaning charitably and proceed; only ask for clarification when meaning is genuinely ambiguous. Don't comment on spelling/grammar unless the user asks.".into());
    }
    // Phase C: seed the next CLI session with the prior conversation's
    // summary after a compaction remint. Frontend tracks
    // `pendingCompactionSummary` and passes it on the FIRST send into the
    // newly-minted session; the summary lives inside <system-reminder> so
    // the cached system-prompt prefix stays stable. Cleared after the send
    // returns — never persists across turns.
    if let Some(s) = prior_context_summary.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        reminder_parts.push(format!(
            "Prior conversation summary (compacted; the CLI session this turn runs against is fresh — this summary IS your context for what came before):\n{s}"
        ));
    }
    let effective_prompt = if reminder_parts.is_empty() {
        prompt.clone()
    } else {
        format!(
            "<system-reminder>\n{}\n</system-reminder>\n\n{}",
            reminder_parts.join("\n\n"),
            prompt
        )
    };

    // Clear any stale stop marker for this session (e.g. retry after a
    // previous stop) before we spawn.
    take_session_stopped(&session_id);
    // #241: coarse turn-latency profile. spawn → first-stream-line (TTFT proxy:
    // process spawn + handshake + SessionStart hooks + model prefill) and
    // spawn → result are the two numbers that reveal whether per-turn cost is
    // harness overhead vs model time. Logged at INFO so a dev session surfaces
    // the breakdown without a debugger. `Instant` is Copy → safe to read in the
    // stdout task and again after child.wait().
    let turn_start = std::time::Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("spawn `claude`: {e}"))?;
    let turn_pid = child.id();
    if let Some(pid) = turn_pid {
        set_session_pid(&session_id, pid);
    } else {
        // #67: `child.id()` returns None when the process already exited by
        // the time we ask (immediate-exit on bad args is the usual cause).
        // Without surfacing this, `assistant_stop` later returns Ok with no
        // PID found and looks like a successful stop while the child kept
        // running. Logging makes the orphan-or-instant-exit case diagnosable.
        log::warn!("assistant_send: child PID unavailable for session {session_id} (process already exited?)");
    }

    // #39: race window between the pre-spawn clear and set_session_pid means a
    // concurrent `assistant_stop` arriving in that window would find no PID,
    // return Ok, and silently drop the stop intent. Re-check the stopped flag
    // now that the PID is registered — if a stop landed during spawn, honor
    // it by killing the child immediately so the wait loop sees the exit and
    // emits the normal stop-path done event.
    if take_session_stopped(&session_id) {
        log::info!("assistant_send: stop arrived during spawn for {session_id} — killing child");
        // B3: surface a failed kill. This arm intentionally keeps the pid
        // registered (so a retry can stop the child), so a silent start_kill
        // failure is exactly the case where a later stale-pid kill could hit a
        // recycled pid — log it so it's diagnosable.
        if let Err(e) = child.start_kill() {
            log::warn!("assistant_send: start_kill failed for {session_id} during stop-on-spawn: {e}");
        }
        // Re-set the marker so the post-wait take_ at the failure branch
        // recognizes this as a user-initiated stop, not a crash.
        mark_session_stopped(&session_id);
    }

    // stdin stays OPEN for the whole turn: the control channel writes a
    // `control_response` back mid-stream after each `can_use_tool` ask, so we
    // can't EOF up front like the old text-input path did. The reader task
    // below owns stdin and drops it (EOF) once the turn's `result` lands.
    // #117: a None stdin would otherwise leave the child waiting forever —
    // fail loudly + kill so the wait loop unblocks.
    // A1: take() + guard in one — no `.expect()` panic path. None means the
    // child died between spawn and now; kill + clear the registered pid so a
    // later assistant_stop can't taskkill a since-recycled pid (#39), then fail
    // loudly so the wait loop unblocks (#117).
    let Some(stdin) = child.stdin.take() else {
        let _ = child.start_kill();
        clear_session_pid(&session_id);
        return Err("claude stdin unavailable — process killed".into());
    };

    // The per-turn user message — always a stream-json `user` envelope (text +
    // optional image blocks). Sent by the reader task once the `initialize`
    // handshake is acknowledged. Shares build_user_envelope with steer injection.
    let user_line: Vec<u8> = build_user_envelope(&effective_prompt, &attachments)?;

    // Steer channel: register the sender while this turn streams so
    // `assistant_steer` can inject mid-turn user messages; the reader task owns
    // the receiver. Cleared at the same points as the session PID (turn end).
    let (steer_tx, mut steer_rx) = mpsc::unbounded_channel::<SteerMsg>();
    register_steer_tx(&session_id, steer_tx.clone());

    let stdout = child.stdout.take().ok_or_else(|| "claude stdout missing".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "claude stderr missing".to_string())?;

    let app_out = app.clone();
    let stream_sid = session_id.clone();
    // #242: turn-completion is signaled by the `result` frame, NOT process exit.
    // A `run_in_background` child (e.g. a dev server / localhost) keeps `claude`
    // alive for as long as it runs, so `child.wait()` below would block for
    // minutes and the UI's DONE_EVENT (which drains the queue) would never fire.
    // The reader sets this the instant `result` lands and emits DONE itself; the
    // main task then reaps a lingering claude instead of waiting it out.
    let result_seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result_seen_task = result_seen.clone();
    let done_sid = session_id.clone();
    let done_app = app.clone();
    let mut stdout_task = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        use std::sync::atomic::Ordering;
        let mut stdin = stdin; // owned by the task; dropped → EOF on turn end
        let mut lines = BufReader::new(stdout).lines();

        // 1) initialize handshake — required so the CLI routes permission asks
        //    over the control channel as `can_use_tool` instead of the headless
        //    auto-deny short-circuit. Mirrors what the Agent SDK sends.
        const INIT: &[u8] = b"{\"type\":\"control_request\",\"request_id\":\"rift-init\",\"request\":{\"subtype\":\"initialize\",\"hooks\":{}}}\n";
        if let Err(e) = stdin.write_all(INIT).await {
            let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                "session_id": stream_sid, "message": format!("write initialize: {e}"),
            }));
            return;
        }
        let _ = stdin.flush().await;

        let mut user_sent = false;
        let mut first_line_logged = false;
        // Steers that arrive before the init handshake completes are buffered,
        // then flushed the instant the user turn is sent (see user_sent branch).
        let mut steer_pending: Vec<SteerMsg> = Vec::new();
        loop {
            tokio::select! {
            read = lines.next_line() => {
            match read {
                Ok(Some(line)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    // Intercept control-channel frames before forwarding.
                    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                        let ty = v.get("type").and_then(|x| x.as_str());
                        // The first `control_response` is the init ack → fire
                        // the user turn once. Don't forward it to the UI.
                        if !user_sent && ty == Some("control_response") {
                            user_sent = true;
                            if let Err(e) = stdin.write_all(&user_line).await {
                                let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                                    "session_id": stream_sid, "message": format!("write user turn: {e}"),
                                }));
                                break;
                            }
                            let _ = stdin.flush().await;
                            // Flush steers that landed during the handshake.
                            // RR-6: surface write/build failures instead of
                            // dropping them silently — mirrors the live steer
                            // path below so a lost steer always signals.
                            for m in steer_pending.drain(..) {
                                match build_user_envelope(&m.text, &m.attachments) {
                                    Ok(env) => {
                                        if let Err(e) = stdin.write_all(&env).await {
                                            let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                                                "session_id": stream_sid,
                                                "message": format!("write steer: {e}"),
                                            }));
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                                            "session_id": stream_sid, "message": e,
                                        }));
                                    }
                                }
                            }
                            let _ = stdin.flush().await;
                            continue;
                        }
                        // Permission ask → resolve via the registry + UI, write
                        // the decision back as a `control_response`.
                        let is_perm = ty == Some("control_request")
                            && v.get("request")
                                .and_then(|r| r.get("subtype"))
                                .and_then(|s| s.as_str())
                                == Some("can_use_tool");
                        if is_perm {
                            handle_permission_request(&app_out, &stream_sid, &mut stdin, &v).await;
                            continue;
                        }
                        // `result` is the last frame — forward it, signal DONE
                        // immediately (the turn is semantically over; don't wait
                        // for process exit, which a background child can defer for
                        // minutes), then break so stdin drops (EOF).
                        if ty == Some("result") {
                            // An auth rejection (401) surfaces as an error result
                            // frame carrying the raw "API Error: 401 Invalid
                            // authentication credentials" — forwarded verbatim it's
                            // a dead-end. Detect it and emit an actionable error too,
                            // mirroring the stderr-exit remap below, so a genuine
                            // auth failure always tells the user what to do.
                            let res_is_err = v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false)
                                || v.get("subtype").and_then(|s| s.as_str()).map(|s| s != "success").unwrap_or(false);
                            let res_text = v.get("result").and_then(|s| s.as_str()).unwrap_or("");
                            if res_is_err && is_auth_rejection(res_text) {
                                let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                                    "session_id": stream_sid, "message": auth_rejection_message(),
                                }));
                            }
                            let _ = app_out.emit(STREAM_EVENT, serde_json::json!({
                                "session_id": stream_sid, "line": trimmed,
                            }));
                            result_seen_task.store(true, Ordering::SeqCst);
                            let _ = done_app.emit(DONE_EVENT, serde_json::json!({
                                "session_id": done_sid, "exit_code": 0,
                            }));
                            break;
                        }
                    }
                    // #241: first forwarded content line ≈ TTFT. Everything
                    // before it (spawn, init handshake, SessionStart hooks,
                    // model prefill) is fixed per-turn overhead.
                    if !first_line_logged {
                        first_line_logged = true;
                        log::info!(
                            "assistant_send: TTFT {} ms (spawn→first-stream-line) session={}",
                            turn_start.elapsed().as_millis(), stream_sid
                        );
                    }
                    // Forward raw NDJSON line, tagged with the CLI session_id
                    // so multi-tab UIs route the event to the right bubble.
                    let _ = app_out.emit(
                        STREAM_EVENT,
                        serde_json::json!({ "session_id": stream_sid, "line": trimmed }),
                    );
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = app_out.emit(
                        ERROR_EVENT,
                        serde_json::json!({
                            "session_id": stream_sid,
                            "message": format!("stdout read error: {e}"),
                        }),
                    );
                    break;
                }
            }
            }
            // Mid-turn steer: write the injected user message to the live stdin.
            // The CLI folds it into the running turn at the next agent-loop step.
            // The STEER_TX registry holds a sender for the whole turn, so recv()
            // never yields None mid-turn (no busy-loop); the branch just parks.
            Some(msg) = steer_rx.recv() => {
                if !user_sent {
                    // Init handshake not yet acked — buffer until the turn is sent.
                    // Cap the buffer: the window is ~100ms, so a flood here can only
                    // be a frontend bug; drop extras rather than grow unbounded.
                    const STEER_PENDING_CAP: usize = 8;
                    if steer_pending.len() < STEER_PENDING_CAP {
                        steer_pending.push(msg);
                    } else {
                        log::warn!("steer_pending cap reached — dropping steer during init handshake");
                    }
                } else {
                    match build_user_envelope(&msg.text, &msg.attachments) {
                        Ok(env) => {
                            if let Err(e) = stdin.write_all(&env).await {
                                let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                                    "session_id": stream_sid,
                                    "message": format!("write steer: {e}"),
                                }));
                                break;
                            }
                            let _ = stdin.flush().await;
                        }
                        Err(e) => {
                            let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                                "session_id": stream_sid, "message": e,
                            }));
                        }
                    }
                }
            }
            }
        }
        // stdin dropped here → EOF.
    });

    // Drain stderr to a buffer for error-event surfacing on non-zero exit.
    // #66: cap at 64 KiB so a wedged CLI streaming error spew doesn't grow
    // the heap unboundedly. When the buffer crosses the cap, drop the first
    // 32 KiB and keep the tail — error context lives at the END of a stderr
    // stream (the panic / fatal-error line), not at the start.
    const STDERR_CAP: usize = 64 * 1024;
    const STDERR_TRIM: usize = 32 * 1024;
    let mut stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        let mut truncated = false;
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            buf.push_str(&l);
            buf.push('\n');
            if buf.len() > STDERR_CAP {
                truncated = true;
                // Find the first newline >= STDERR_TRIM bytes in so we drop on
                // a line boundary, not mid-line. Safe `String::drain` requires
                // a char boundary; newline is always one.
                // F70: index the BYTES, not the str — `buf[STDERR_TRIM..]` on a
                // String panics when STDERR_TRIM lands inside a multi-byte
                // codepoint. A byte slice is always valid; the cut (just past a
                // `\n`, or the next char boundary) stays drain-safe.
                let cut = buf.as_bytes()[STDERR_TRIM..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|n| STDERR_TRIM + n + 1)
                    .unwrap_or_else(|| {
                        let mut c = STDERR_TRIM;
                        while c < buf.len() && !buf.is_char_boundary(c) {
                            c += 1;
                        }
                        c
                    });
                buf.drain(..cut);
            }
        }
        if truncated {
            buf.insert_str(0, "[... earlier stderr dropped (>64 KiB) ...]\n");
        }
        buf
    });

    // #242: wait for claude to exit — but the `result` frame already ended the
    // turn for the UI (the reader emitted DONE). If claude lingers past a short
    // grace AFTER result (a run_in_background child is pinning it alive), kill
    // its PID — NOT the tree, so the detached background process survives — and
    // stop waiting. Without `result` we keep waiting: claude may legitimately be
    // mid-turn on a long task and must not be killed out from under itself.
    const REAP_GRACE: std::time::Duration = std::time::Duration::from_secs(5);
    let mut reap_deadline: Option<std::time::Instant> = None;
    let status: Option<std::process::ExitStatus> = loop {
        match tokio::time::timeout(std::time::Duration::from_millis(150), child.wait()).await {
            Ok(Ok(s)) => break Some(s),
            Ok(Err(e)) => {
                // F6: don't leak the two pipe-drain tasks on the wait()-error
                // path — abort them before bailing.
                stdout_task.abort();
                stderr_task.abort();
                if let Some(p) = turn_pid { clear_session_pid_if(&session_id, p); }
                clear_steer_tx_if(&session_id, &steer_tx);
                return Err(format!("await claude: {e}"));
            }
            Err(_) => {
                if result_seen.load(std::sync::atomic::Ordering::SeqCst) {
                    let dl = *reap_deadline
                        .get_or_insert_with(|| std::time::Instant::now() + REAP_GRACE);
                    if std::time::Instant::now() >= dl {
                        log::info!(
                            "assistant_send: claude lingering {} ms past result (background child pinning it) — killing PID, session={}",
                            turn_start.elapsed().as_millis(), session_id
                        );
                        let _ = child.start_kill();
                        let _ = child.wait().await;
                        break None;
                    }
                }
            }
        }
    };
    if let Some(p) = turn_pid { clear_session_pid_if(&session_id, p); }
    clear_steer_tx_if(&session_id, &steer_tx);
    // #241: total turn wall-clock (spawn → claude exit). Compare against the
    // TTFT line above: large TTFT w/ small (total−TTFT) = harness/prefill bound;
    // small TTFT w/ large remainder = model generation bound.
    log::info!(
        "assistant_send: turn total {} ms (spawn→exit) first_turn={} model={} session={}",
        turn_start.elapsed().as_millis(), is_first_turn, model, session_id
    );

    // #240: both drain tasks read the child's piped stdout/stderr. A background
    // process the turn spawned (e.g. a dev server / localhost) inherits those
    // pipe write-ends on Windows, so the reader never sees EOF and a bare
    // `.await` here blocks FOREVER — stranding the DONE_EVENT below and hanging
    // the frontend queue in "Queued". claude itself has already exited (wait()
    // returned above), so anything still pending is a leaked fd with nothing
    // left to deliver: bound each await and abort the task on elapse. stdout has
    // the `result`-frame break so it usually finishes instantly; stderr drains
    // to EOF with no escape hatch, so it's the one that actually wedges.
    const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);
    if tokio::time::timeout(DRAIN_TIMEOUT, &mut stdout_task).await.is_err() {
        log::warn!("assistant_send: stdout drain timed out (inherited pipe held by a background process?) for {session_id}");
        stdout_task.abort();
    }
    // #222: surface stderr-drain JoinError so a panicked drain task doesn't
    // turn into a blank stderr at the call site (which then shows up as
    // "claude exited with 1 — " with no diagnosis).
    let stderr_buf = match tokio::time::timeout(DRAIN_TIMEOUT, &mut stderr_task).await {
        Ok(Ok(buf)) => buf,
        Ok(Err(e)) => {
            log::error!("stderr drain task panicked: {e}");
            format!("(stderr drain task panicked: {e})")
        }
        Err(_) => {
            log::warn!("assistant_send: stderr drain timed out (inherited pipe held by a background process?) for {session_id}");
            stderr_task.abort();
            String::new()
        }
    };

    // #242: a `result` frame means the turn succeeded and the reader already
    // emitted DONE — whether claude then exited cleanly or we killed a pinned
    // process, there is nothing more to signal.
    if result_seen.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }
    // No `result` → claude ended without finishing the turn (crash, bad args,
    // user Stop, or a lost --resume). `status` is always Some on this path: we
    // only break the wait loop with None after observing result_seen above.
    let status = match status {
        Some(s) => s,
        None => {
            let _ = app.emit(ERROR_EVENT, serde_json::json!({
                "session_id": session_id,
                "message": "claude was killed before producing a result",
            }));
            return Err("claude killed before result".into());
        }
    };

    if status.success() {
        let _ = app.emit(
            DONE_EVENT,
            serde_json::json!({ "session_id": session_id, "exit_code": 0 }),
        );
        Ok(())
    } else if take_session_stopped(&session_id) {
        // User clicked Stop → assistant_stop killed the child. Emit done
        // (not error) so the UI clears the streaming flag and pops the
        // next queued message cleanly.
        let _ = app.emit(
            DONE_EVENT,
            serde_json::json!({
                "session_id": session_id,
                "exit_code": status.code().unwrap_or(-1),
            }),
        );
        Ok(())
    } else {
        // Auto-recovery: claude's resume index sometimes loses track of valid
        // session JSONLs (transient — observed after long-idle tabs / app
        // rebuilds even when the JSONL is on disk). Emit a session-lost
        // event so the frontend can null convoCreatedAt + re-send the same
        // prompt as a fresh first-turn. Only fires on --resume failures
        // (first-turn failures still go through the normal error path).
        if !is_first_turn
            && stderr_buf.contains("No conversation found with session ID:")
        {
            log::warn!(
                "assistant_send: --resume {} failed (no conversation found) — emitting session-lost for frontend auto-recovery",
                session_id
            );
            // #115: emit only the recovery signal. The full prompt is buffered
            // in the frontend's last-message slot; re-broadcasting it over the
            // Tauri bus risks leaking via diag listeners and inflates the
            // event payload for no benefit.
            let _ = app.emit(
                SESSION_LOST_EVENT,
                serde_json::json!({ "session_id": session_id }),
            );
            return Ok(());
        }
        // A non-zero exit with EMPTY stderr is almost always a startup failure —
        // a missing CLI or an unauthenticated session — both of which claude
        // reports on stdout/JSON, leaving the bare "claude exited with 1 — " with
        // no diagnosis (the exact dead-end a fresh collaborator hits). Reuse the
        // auth probe (already distinguishes not-installed vs not-logged-in) to
        // turn it into something the user can act on.
        let raw = stderr_buf.trim();
        let msg = if raw.is_empty() {
            match assistant_auth_probe().await {
                Ok(s) if !s.cli_present => "Claude Code CLI not found on this machine — install it from claude.com/code (or add an API key in Settings), then try again.".to_string(),
                Ok(s) if !s.logged_in && !s.api_key_configured => "Claude CLI is installed but not logged in on this machine — open a terminal, run `claude`, and sign in (or add an API key in Settings), then try again.".to_string(),
                _ => format!(
                    "claude exited with {} (no error output) — run `claude` in a terminal to confirm it works, then retry.",
                    status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
                ),
            }
        } else if is_auth_rejection(raw) {
            auth_rejection_message()
        } else {
            format!(
                "claude exited with {} — {}",
                status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
                raw
            )
        };
        let _ = app.emit(
            ERROR_EVENT,
            serde_json::json!({ "session_id": session_id, "message": msg.clone() }),
        );
        Err(msg)
    }
}

/// A rejected credential (401) arrives either as a stdout error-result frame
/// or as stderr at exit — one detector + one remap for both sites (#31; the
/// two inline copies had started to diverge).
fn is_auth_rejection(text: &str) -> bool {
    text.contains("401")
        || text.contains("authentication_error")
        || text.contains("Invalid authentication")
        || text.contains("invalid x-api-key")
}

/// Actionable replacement for the raw "API Error: 401 …" dead-end, routed by
/// which auth path is active.
fn auth_rejection_message() -> String {
    if current_api_key().is_some() {
        "Your configured API key was rejected (401). Clear it in Settings → CLI session to fall back to your `claude login`, or paste a valid key.".to_string()
    } else {
        format!(
            "Authentication failed (401). Rift is using the Claude CLI at {} — sign in by running `claude login` in a terminal, or switch installs in Settings → CLI session, then retry.",
            resolve_claude_exe().map(|p| p.display().to_string()).unwrap_or_else(|| "your active install".into())
        )
    }
}

/// Kill the streaming `claude` child for a specific CLI session, if any.
/// Platform-native: taskkill /F /PID on Windows, SIGTERM via libc on Unix.
/// No-op (returns Ok) if no child is active for that session.
///
/// Per-session (vs the prior single-slot global) so a tab pressing Stop kills
/// only its own stream — never another tab's.
#[tauri::command]
pub async fn assistant_stop(session_id: String) -> Result<(), String> {
    if !is_valid_session_id(&session_id) {
        return Err(format!("invalid session_id: must be a UUID (got {} chars)", session_id.len()));
    }
    let Some(pid) = get_session_pid(&session_id) else {
        return Ok(());
    };
    mark_session_stopped(&session_id);
    clear_session_pid(&session_id);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let out = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match out {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("taskkill exited {}", s.code().unwrap_or(-1))),
            Err(e) => Err(format!("spawn taskkill: {e}")),
        }
    }
    #[cfg(unix)]
    {
        // Avoid a libc dependency just for SIGTERM; shell out to `kill`.
        let out = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match out {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("kill exited {}", s.code().unwrap_or(-1))),
            Err(e) => Err(format!("spawn kill: {e}")),
        }
    }
}

/// Inject a steer message into the RUNNING turn for `session_id`. Unlike the
/// queue (which fires a fresh turn after `result`), a steer is written to the
/// live CLI stdin and folded into the current turn at the agent's next loop
/// step — no restart, no lost work. Returns `"steered"` when an active turn
/// accepted it, or `"no_active_turn"` when the turn already ended (the caller
/// should fall back to queueing a fresh turn).
#[tauri::command]
pub async fn assistant_steer(session_id: String, text: String) -> Result<String, String> {
    if !is_valid_session_id(&session_id) {
        return Err(format!(
            "invalid session_id: must be a UUID (got {} chars)",
            session_id.len()
        ));
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty steer text".into());
    }
    let Some(tx) = get_steer_tx(&session_id) else {
        return Ok("no_active_turn".into());
    };
    match tx.send(SteerMsg { text: trimmed.to_string(), attachments: Vec::new() }) {
        Ok(()) => Ok("steered".into()),
        // Receiver dropped between lookup and send → turn just ended.
        Err(_) => Ok("no_active_turn".into()),
    }
}
