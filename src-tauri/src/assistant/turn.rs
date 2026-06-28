//! The live-turn nervous system — R8 split (2026-06-09) out of `assistant/mod.rs`.
//! Session registry (per-session child PIDs, stop flags, steer channels),
//! the `assistant://*` event consts, user-envelope build, control-response +
//! permission-request plumbing, and the `assistant_send` / `assistant_stop` /
//! `assistant_steer` commands. See docs/design/assistant-mod-split.md R8.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::BufReader;
use tokio::sync::{mpsc, oneshot};

use super::warm_pool;
use super::warm_pool::kill_child_tree;

use super::auth_update::assistant_auth_probe;
use super::cli_install::{claude_command, resolve_claude_exe};
use super::config::{
    clamp_effort, current_api_key, current_api_key_with, effective_trust_level, fable_unavailable,
    haiku_unavailable, HAIKU_FALLBACK_MODEL, HAIKU_MODEL,
    is_valid_effort_tier,
    is_valid_local_model_name, is_valid_model_name, is_valid_permission_mode, load_config,
    DEFAULT_MODEL, FABLE_FALLBACK_MODEL, FABLE_MODEL,
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
/// The per-PID `kill_child_tree` now lives in `warm_pool` (shared by this sweep,
/// the signature-drain, idle-evict, and shutdown-drain).
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
        kill_child_tree(pid);
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

/// 20 MiB total cap across all attachments — protects the CLI's JSON parser
/// from a runaway paste. Per-image cap equals the cumulative since one big
/// image is the realistic worst case.
const ATTACHMENT_BYTES_CAP: usize = 20 * 1024 * 1024;
/// Strict allowlist (not an `image/` prefix) — blocks e.g. image/svg+xml,
/// which can carry script, and any malformed `image/…\r\n…` smuggle.
const ALLOWED_IMAGE_MIMES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Validate inline image attachments: cumulative decoded size ≤ cap, mime in
/// the strict allowlist. Shared by the send path and `assistant_steer` so a
/// steered image gets the same gate as a first-turn one (#49).
fn validate_attachments(attachments: &[AssistantAttachment]) -> Result<(), String> {
    if attachments.is_empty() {
        return Ok(());
    }
    // #116: `len * 3 / 4` is approximate — pasted base64 can contain
    // whitespace/CRLF that inflates the encoded length but doesn't add to
    // decoded bytes. Strip whitespace before the divide so the cap reflects
    // real decoded size.
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
            "Those images are too large to send — keep attachments under {} MB total.",
            ATTACHMENT_BYTES_CAP / (1024 * 1024)
        ));
    }
    for a in attachments {
        if !ALLOWED_IMAGE_MIMES.contains(&a.mime.as_str()) {
            return Err(format!(
                "That file type isn't supported ({}). Attach a PNG, JPEG, GIF, or WebP image.",
                a.mime
            ));
        }
    }
    Ok(())
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
    if serde_json::to_string(&answer).map(|s| s.len()).unwrap_or(usize::MAX) > 64 * 1024 {
        return Err("answer payload too large".into());
    }
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
const RIFT_SYSTEM_ADDENDUM_TOOLS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app, working inside the user's open project folder (your working directory is already set to the workspace root, so relative paths Just Work). You have the full Claude Code toolset: Read / Write / Edit / MultiEdit for files, Bash for shell commands (executes in the workspace dir, output streamed back), Glob for filename patterns, Grep for content search, WebFetch and WebSearch for the open web, TaskCreate / TaskUpdate for multi-step plans (TodoWrite on older CLI builds), and Agent for delegating heavy lookups. Task output surfaces in a dedicated Tasks panel in the user's UI — create tasks proactively whenever a request involves three or more distinct steps, and update statuses (pending → in_progress → completed) as you go. Rift's MCP server also exposes read_file / list_dir / grep as scoped, workspace-rooted helpers, plus git_status / git_diff / git_log (and git_pull / git_commit / git_push when trust permits). Three more MCP tools drive the Rift app itself: mcp__rift__ask_user presents an interactive multiple-choice card in the chat — use it whenever you need the user to pick between approaches or confirm something risky (the standard Anthropic `AskUserQuestion` tool is NOT available in this environment; ask_user is its Rift-native replacement, and if it errors fall back to asking in plain text). mcp__rift__open_browser shows any http/https page in Rift's in-app browser dock right beside the chat — ALWAYS call it instead of only printing a URL when you start a dev server or want the user to see a local preview (e.g. http://localhost:3000), a deployed page, or docs worth reading together. mcp__rift__notify pops a brief toast in the corner of the Rift window — fire it when long-running work finishes or something needs the user's attention (they may be looking at another page of the app); don't spam it. BACKGROUND TASKS — CRITICAL: you run in single-turn headless mode. A Bash command started with run_in_background:true is KILLED a few seconds after you end your turn, and NOTHING re-invokes you when it finishes — so you can NEVER end a turn promising to report a background result 'when it lands' (that report can never arrive). For a slow command (a build, a long test run), run it in the FOREGROUND with a generous timeout so its output is part of THIS turn. Only use run_in_background:true if you immediately Read its output file (or BashOutput) within this same turn before finishing; never defer it to a later turn. A 'Rift environment snapshot' <system-reminder> may precede the user's message with volatile app state (the browser dock's current page, the user's Claude plan-usage gauges) — treat it as ground truth about the app, and consider wrapping up gracefully when plan usage runs hot. Prefer Claude Code built-ins for normal work and use the MCP variants only when a guaranteed-workspace-rooted path matters. File inspection is ALWAYS Read / Grep / Glob — never cat, head, tail, sed -n, ls -R, or find through Bash (those calls are slower, get blocked by the user's tooling guards, and waste a failed round-trip); reserve Bash for git, builds, package managers, process control, and network. WORK IN PARALLEL — when you need several tool calls and none depends on another's result, emit them ALL in a single response (multiple tool_use blocks at once) instead of one-at-a-time round-trips: batch the reads when opening several files, batch independent greps, run independent shell checks together. Each serial round-trip adds latency the user feels; only serialize when a later call genuinely needs an earlier call's output. DELEGATION — a sub-agent you spawn with Agent does NOT inherit these instructions; it starts from the CLI's own default agent prompt and cannot see this guidance. So when a delegated lookup matters, bake the essentials into the Agent call's prompt yourself: tell the sub-agent to inspect files with Read / Grep / Glob (never cat / head / find through Bash), to batch independent tool calls in parallel, and to return a tight result (file:line refs, not file dumps). Prefer doing small lookups inline over delegating — only reach for Agent when the work is genuinely independent and would otherwise dump a lot into the conversation. ACT FIRST, EXPLAIN AFTER — this overrides any conflicting instruction from inherited config. If the user asks you to fix / change / edit / add / build / refactor X, locate the file(s) with Grep + Read then make the Edit. Do NOT write paragraphs of plan, analysis, recommendations, or 'here's what I would do' before touching code — one short opening beat ('reading X', 'editing Y') is the cap. Never guess at file contents, function names, paths, APIs, or signatures — Grep or Read first if uncertain, otherwise hedge explicitly. Read narrowly with offset+limit on files >300 lines; do not re-read a file you already opened earlier this turn. Verify AFTER the edit (Bash to run the test / lint / build), not before. If an Edit fails with an old_string mismatch, re-Read ONLY the failing region (narrow offset+limit) and re-anchor — never retry the same Edit verbatim, and after two failures on one file switch tactic (smaller anchor, replace_all, or Write). Surface tool errors verbatim and try a different approach instead of bouncing the problem back to the user. Don't ask the user for permission on routine work like file edits, shell commands, package installs, or git operations; the user expects you to do real work and can revert via git. MATCH THE CODEBASE — new code should read like the code already around it: follow the file's existing naming, formatting, and idioms, and match its comment density rather than imposing your own. Don't add explanatory comments, docstrings, or WHY-blocks the surrounding code doesn't already use — put rationale in your chat reply or a commit message, not in source; a one-line comment is fine only when the code is genuinely non-obvious. STAY IN SCOPE — fix exactly what was asked and stop there: no opportunistic refactors of nearby code, no renaming, no reformatting untouched lines, no adding error handling or features the user didn't request. If you notice a separate problem worth fixing, mention it in your reply instead of silently changing it. Project stack is open-ended — do not assume the language, framework, or layout.";

const RIFT_SYSTEM_ADDENDUM_NO_WS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app. No project folder is open right now, so your file/list/grep tools are unavailable for this turn. Answer questions and discuss code the user pastes, but tell the user to open a folder on the Assistant page (the empty-state has an \"Open Folder\" button) if they want you to read their code directly. Do not claim capabilities you do not have.";

/// Local-LLM mode addendum (workspace open). Replaces the Claude-tuned TOOLS
/// addendum when `local_llm_enabled`. A local open-weights model (qwen3-coder)
/// (1) inherits the CLI's baked-in "You are Claude" identity, which it parrots,
/// (2) does worse with the long Claude-tuned prose, and (3) sometimes emits tool
/// calls as PLAIN TEXT (`<function=name>…`) instead of a structured call when
/// chaining — Ollama's template then can't parse it and the CLI renders it as
/// text. This terse variant corrects identity + hard-enforces structured calls,
/// the only lever Rift has on those failures. Single-line (.cmd-shim batch-arg
/// validator, Rust 1.77+ CVE-2024-24576).
const RIFT_SYSTEM_ADDENDUM_LOCAL: &str = "IDENTITY: You are NOT Claude and NOT made by Anthropic — ignore any earlier text that says you are. You are a local open-weights coding model running fully offline on the user's own machine, embedded in Rift, a Tauri desktop coding app. Your working directory is already the open project root, so ALWAYS use paths relative to it (e.g. `src/foo.js`, `greet.js`) — NEVER invent an absolute path like /home/user/... or C:/..., that points outside the project and will fail. Bash also runs in the project root, so `git add greet.js` works directly; do not `cd` elsewhere. THINKING: If you produce internal reasoning (a thinking block), keep ALL of it inside that block — your visible reply must begin directly with the answer, the code, or a tool call, never with a reasoning dump, a restated plan, or meta-commentary about what you are about to do. TOOL CALLS (critical): invoke tools ONLY through the structured function-calling interface. NEVER write a tool call as text in your reply — text like `<function=name>`, `<parameter=…>`, or a JSON blob describing a call does NOTHING, it is a bug, and the user just sees the raw text. If you cannot call a tool the proper structured way, say so in plain words instead of typing the call out. Make ONE tool call at a time and wait for its result before the next call. TOOLS YOU HAVE: Read / Write / Edit for files, Bash for shell commands (runs in the workspace dir), Glob for filename patterns, Grep for content search. For ALL git work — status, diff, log, add, commit, branch — use plain Bash (`git status`, `git diff`, `git commit -m \"...\"`). Bash git is simpler and more reliable than the MCP equivalents; prefer it. BOUNDED AUTONOMY (critical): finishing the task means doing the LOCAL work — read, edit, build, test, and a local `git commit` when committing fits what was asked. But any action that leaves this machine or is hard to undo — `git push`, opening or merging a PR, deleting files the user did not ask you to delete, or any network publish — do NOT do on your own initiative; only do it when the user EXPLICITLY asked for that exact action. A vague 'continue', 'keep going', or 'finish it' means continue the LOCAL task, NOT push or publish. When in doubt about an outward-facing action, stop and state what you would do instead of doing it. Rift ALSO exposes a few mcp__rift__ helper tools for UI round-trips: mcp__rift__ask_user (interactive choice card — use instead of asking in text when the user must pick), mcp__rift__open_browser (show an http/https page in Rift's in-app dock — call it when you start a dev server or have a URL worth showing, e.g. http://localhost:3000), and mcp__rift__notify (corner toast for finished long work). MCP TOOL NAMES ARE LITERAL: every mcp tool name has the exact form mcp__rift__NAME with TWO underscores before and after `rift` (e.g. mcp__rift__ask_user). Copy the name character-for-character — never collapse the double underscore to one (`mcp__rift_ask_user` is WRONG), never call the bare prefix `mcp__rift`, never invent a name like mcp__rift__git_commit (git goes through Bash, not mcp). If you are not 100% sure of an mcp name, use the plain native tool or Bash instead — a wrong mcp name just errors with 'No such tool available'. Inspect files with Read / Grep / Glob — never cat, head, tail, ls -R, or find through Bash. Reserve Bash for git, builds, package managers, and running things. IMAGES (critical): if the user attaches an image, it is the PRIMARY input — LOOK at it and describe what you actually see before doing anything else. A screenshot of a bug IS the bug report: the user is showing you the broken thing, not decorating the message. Never ignore an attached image, and never answer as if no image was sent. DIAGNOSE BEFORE FIXING: 'find the issue and fix it' means (1) reproduce / locate the actual defect — from the image, the described symptom, or the code — then (2) edit the code that causes it, then (3) verify. Running the build, linter, or tests and seeing them pass does NOT mean 'there is no issue' — a clean build with a visible bug means the bug is real and you have not found it yet. If everything you checked looks green but the user reports a problem, you looked in the wrong place: keep digging, do not declare there is nothing to fix. Only say 'no issue found' after you have genuinely searched the relevant code for the described symptom and can explain why it cannot occur. TESTING RIGOR: when you fix a bug, your fix must handle EVERY case in the same family, not just the one example given — if the bug is about extra dashes, handle leading, trailing, AND internal/doubled dashes; if it is about whitespace, handle tabs and multiple spaces too. Before claiming done, mentally run 3-4 varied inputs (empty string, multiple separators in a row, punctuation runs) through your fix and confirm each. Your test must assert on those varied cases, not re-test the single happy path. A fix that passes only the reported example is INCOMPLETE. FINISH THE WHOLE TASK (critical — do NOT stop early): the user's request is the WHOLE job, not the first step of it. A broad ask like 'debug the codebase and see what can be improved', 'audit this', 'review the project', or 'find issues' means: read ALL the relevant files (not 2-3), actually FIND concrete problems, and report a real list of findings WITH fixes — keep calling tools across as many turns as it takes. Describing one file you happened to read is NOT doing the task; it is quitting after step one. After every tool result, ask yourself 'is the user's actual goal fully done?' — if not, immediately make the next tool call. Only end your turn when the complete request is satisfied, never after a single file or a single observation. If a task is genuinely large, do the work in order and keep going; do not hand it back half-done. CONVERSATION MEMORY: you have the full prior conversation. When the user says 'you didn't do what I requested' or 'do it' or 'continue', the request is in an EARLIER message — re-read the conversation and act on that original request; never reply that you cannot see a request. BEHAVIOR: bias toward action — once you know the cause, make the Edit; don't pad with long 'here is what I would do' preambles. But finishing the task always beats being brief: write as many words and make as many tool calls as the job needs. Keep PROSE tight (no filler, no restating the plan) — that is about wordiness, never about doing less work. A short, real diagnosis of a reported bug is required work, not filler. Locate files with Grep + Read; never guess file contents, paths, function names, or signatures — Read or Grep first, otherwise say you are unsure. EDIT PRECISION: when you use Edit, the old_string must match the file EXACTLY — copy it verbatim from what Read returned, byte for byte, including the exact existing indentation (spaces vs tabs). Do NOT retype it from memory, do NOT add tabs or spaces the file does not have, do NOT guess the whitespace. Read shows line numbers as a `123\t` prefix — that prefix is NOT part of the file, never include it in old_string. If an Edit fails with 'old_string not found', do not retry the same text — re-Read that exact region and copy the real bytes, or rewrite the file with Write. After an edit, verify by running the build or tests. Do not ask permission for routine LOCAL work — file edits, shell commands, package installs, and local git commits — the user expects real work and can revert via git. (Outward-facing actions like `git push` are the exception above: those need an explicit request.)";

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
    window_label: &str,
    stdin: &mut tokio::process::ChildStdin,
    msg: &Value,
) -> std::io::Result<()> {
    let request_id = msg.get("request_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    // RR11: reject a missing/empty request_id (mirrors ask_user_op) — an empty key
    // collides in the PermissionRegistry, so two malformed messages would corrupt
    // both pending grants. Deny immediately rather than register the "" slot.
    if request_id.is_empty() {
        write_control_response(stdin, "", serde_json::json!({
            "behavior": "deny",
            "message": "permission request missing request_id"
        })).await?;
        return Ok(());
    }
    let req = msg.get("request").cloned().unwrap_or(Value::Null);
    let tool_use_id = req.get("tool_use_id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let original_input = req.get("input").cloned().unwrap_or(Value::Null);
    let tool_name = req.get("tool_name").and_then(|x| x.as_str()).unwrap_or_default();

    // Builtin AskUserQuestion has no headless surface (it stalls in `-p` mode)
    // and only reaches here because it's off the allowlist. Auto-deny with a
    // steer to mcp__rift__ask_user — never surface the raw Allow/Deny bar.
    if tool_name == "AskUserQuestion" {
        write_control_response(stdin, &request_id, serde_json::json!({
            "behavior": "deny",
            "message": "AskUserQuestion is unavailable here. Call the mcp__rift__ask_user tool instead — it presents the question(s) in the Rift UI and returns the user's selection.",
        })).await?;
        return Ok(());
    }

    let registry = match app.try_state::<std::sync::Arc<PermissionRegistry>>() {
        Some(r) => r.inner().clone(),
        None => {
            // Init bug — deny so the CLI doesn't hang forever.
            write_control_response(stdin, &request_id, serde_json::json!({
                "behavior": "deny", "message": "permission registry unavailable",
            })).await?;
            return Ok(());
        }
    };

    // Guard cancels the registry entry on drop — covers task-abort mid-await
    // (the explicit cancel below never runs when the future is cancelled).
    let (rx, _perm_guard) = registry.register_guarded(request_id.clone());
    // B4: if the UI is unreachable (window closed mid-turn) the user never sees
    // the prompt — deny immediately rather than hang for the full 120s timeout
    // while the CLI waits on us. `emit_to` returns Ok(()) for a missing/closed
    // label (zero webviews matched), so the error path below can't detect a gone
    // window — check existence explicitly first.
    if app.get_webview_window(window_label).is_none() {
        log::warn!("permission emit skipped for {session_id} — window `{window_label}` gone, denying");
        registry.cancel(&request_id);
        write_control_response(stdin, &request_id, serde_json::json!({
            "behavior": "deny", "message": "permission UI unreachable",
        })).await?;
        return Ok(());
    }
    log::info!(
        "permission ask: tool={tool_name} session={session_id} request_id={request_id} \
         tool_use_id={tool_use_id} — emitting card, awaiting user decision"
    );
    if let Err(e) = app.emit_to(window_label, PERMISSION_EVENT, serde_json::json!({
        "session_id": session_id,
        "request_id": request_id,
        "tool_use_id": tool_use_id,
        "tool_name": req.get("tool_name").cloned().unwrap_or(Value::Null),
        "input": req.get("input").cloned().unwrap_or(Value::Null),
        "suggestions": req.get("permission_suggestions").cloned().unwrap_or(Value::Null),
    })) {
        log::warn!("permission emit failed for {session_id} ({e}) — denying (UI unreachable)");
        registry.cancel(&request_id);
        write_control_response(stdin, &request_id, serde_json::json!({
            "behavior": "deny", "message": "permission UI unreachable",
        })).await?;
        return Ok(());
    }

    // Cap the wait so a forgotten prompt can't wedge the turn. 120s, NOT 30 min:
    // a real Allow/Deny click happens in seconds, so the only thing the long
    // ceiling ever bought was a half-hour silent freeze when the card failed to
    // render (the prompting-mode path that cont.202 made default — historically
    // untested, since bypassPermissions never raises an ask). Deny-on-timeout
    // with an actionable message lets the user recover instead of staring at a
    // frozen "Working…". The model sees the deny and can ask in plain text.
    let mut decision = match tokio::time::timeout(std::time::Duration::from_secs(120), rx).await {
        Ok(Ok(v)) => {
            log::info!("permission decision: tool={tool_name} session={session_id} → {}",
                v.get("behavior").and_then(|b| b.as_str()).unwrap_or("?"));
            v
        }
        _ => {
            registry.cancel(&request_id);
            log::warn!(
                "permission ask TIMED OUT after 120s: tool={tool_name} session={session_id} \
                 — no Allow/Deny answer arrived (card not shown or not clicked); auto-denying"
            );
            serde_json::json!({ "behavior": "deny",
                "message": "Permission prompt wasn't answered in time. Rift auto-denied this action. \
                            If you didn't see an Allow/Deny prompt, switch the permission mode to \
                            'Bypass' in the composer so tools run without asking." })
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
    write_control_response(stdin, &request_id, decision).await
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
    window: tauri::Window,
    prompt: String,
    session_id: String,
    is_first_turn: bool,
    model: Option<String>,
    attachments: Option<Vec<AssistantAttachment>>,
    dyslexia_mode: Option<bool>,
    thinking_effort: Option<String>,
    thinking_enabled: Option<bool>,
    permission_mode: Option<String>,
    prior_context_summary: Option<String>,
    root: Option<String>,
) -> Result<(), String> {
    run_or_prewarm(
        app, window, prompt, session_id, is_first_turn, model, attachments,
        dyslexia_mode, thinking_effort, thinking_enabled, permission_mode,
        prior_context_summary, root, false,
    ).await
}

/// #67 pre-warming: spawn a warm `claude` child for a chat tab BEFORE the user
/// hits send, so the first real turn skips the cold-boot + (full-config) the
/// ~6.3s SessionStart-hook tax measured in the cont.214 spike. The spare is a
/// NORMAL warm child registered under the tab's already-minted `session_id` —
/// no separate spare pool, no adoption codepath. When the real `assistant_send`
/// arrives, `dispatch_turn` finds it by session_id and reuses it via the
/// existing warm path IF the SpawnKey matches (model/effort/perm/root/etc.); a
/// mismatch (the user changed the picker before sending) drains it + cold-spawns
/// exactly as today — never worse than no pre-warm.
///
/// Cheap + idempotent: returns Ok immediately (no spawn) when a warm child
/// already exists for the session, the prompt would be tool-less (no root +
/// not full-config), or plan usage is hot. NEVER runs a model turn — it only
/// pays the process spawn + init handshake, then parks. The frontend triggers
/// it debounced on tab-ready-with-root; see prewarm.ts.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn assistant_prewarm(
    app: AppHandle,
    window: tauri::Window,
    session_id: String,
    model: Option<String>,
    thinking_effort: Option<String>,
    thinking_enabled: Option<bool>,
    permission_mode: Option<String>,
    root: Option<String>,
    // Re-warm support (2026-06-28 cold-start arc): a fresh tab pre-warms a
    // `--session-id` child; an EXISTING conversation whose warm child was idle-
    // evicted re-warms a `--resume` child so the next real turn is a warm hit
    // instead of a cold respawn. Defaults to true (fresh-tab) when the frontend
    // omits it, preserving the original single-call-site behaviour.
    is_first_turn: Option<bool>,
) -> Result<(), String> {
    // A spare for a session that already has a live warm child is pure waste —
    // bail before doing any work (cheap registry read, no child lock).
    if warm_pool::get(&session_id).is_some() {
        return Ok(());
    }
    // Cost guard: don't speculatively spawn (and re-run the user's SessionStart
    // hooks + handshake) when the plan's rolling window is nearly spent. None =
    // unknown (API-key users, no fetch yet) → allow; only skip on a KNOWN-hot
    // window so the common case never loses pre-warm to missing data.
    if let Some(snap) = crate::usage::limits::cached_snapshot() {
        let hot = snap.five_hour.as_ref().map(|w| w.utilization).unwrap_or(0.0) >= 90.0
            || snap.seven_day.as_ref().map(|w| w.utilization).unwrap_or(0.0) >= 95.0;
        if hot {
            log::debug!("assistant_prewarm: skipped for {session_id} — plan usage hot");
            return Ok(());
        }
    }
    run_or_prewarm(
        app, window, String::new(), session_id, is_first_turn.unwrap_or(true), model,
        /*attachments*/ None, /*dyslexia*/ None, thinking_effort, thinking_enabled,
        permission_mode, /*prior_context_summary*/ None, root, true,
    ).await
}

#[allow(clippy::too_many_arguments)]
async fn run_or_prewarm(
    app: AppHandle,
    window: tauri::Window,
    prompt: String,
    session_id: String,
    is_first_turn: bool,
    model: Option<String>,
    attachments: Option<Vec<AssistantAttachment>>,
    dyslexia_mode: Option<bool>,
    thinking_effort: Option<String>,
    thinking_enabled: Option<bool>,
    permission_mode: Option<String>,
    prior_context_summary: Option<String>,
    root: Option<String>,
    prewarm: bool,
) -> Result<(), String> {
    // #220: validate session_id is a canonical UUID (8-4-4-4-12 lowercase hex)
    // BEFORE any use. Renderer-supplied — must not flow into CLI args or
    // sidecar filename without check. Blocks leading-dash flag injection
    // into `--session-id`/`--resume` AND path-traversal segments in
    // save_session_cwd's filename derivation.
    // RR9: cap the renderer-supplied prompt + prior_context_summary before they
    // flow into build_user_envelope → blocking stdin write_all (mirrors the
    // attachment 20 MiB cap + steer 1 MiB cap). A renderer bug could otherwise
    // push tens of MB into a single synchronous pipe write, stalling the worker
    // + allocating heap proportional to the input. 2 MiB is generous for any
    // real prompt or compaction summary.
    const PROMPT_BYTES_CAP: usize = 2 * 1024 * 1024;
    if prompt.len() > PROMPT_BYTES_CAP {
        return Err(format!("prompt too large ({} bytes, max {})", prompt.len(), PROMPT_BYTES_CAP));
    }
    if prior_context_summary.as_deref().map(str::len).unwrap_or(0) > PROMPT_BYTES_CAP {
        return Err("prior_context_summary too large (max 2 MiB)".into());
    }
    if !is_valid_session_id(&session_id) {
        return Err(format!("invalid session_id: must be a UUID (got {} chars)", session_id.len()));
    }
    // #37: the window that fired this turn — all turn events (stream/done/error/
    // permission) emit_to this label so a second window never sees another's turn.
    let window_label = window.label().to_string();
    let cfg = load_config();
    let api_key = current_api_key_with(&cfg);
    let use_api_key = api_key.is_some();
    let mut model = model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
    if !is_valid_model_name(&model) {
        return Err(format!("invalid model: {model}"));
    }
    if cfg.local_llm_enabled {
        // Local-LLM mode (experimental): use the configured local model verbatim
        // and skip cloud-only machinery (model pin, Fable guard) — there are no
        // thinking-block signatures to preserve and no Anthropic model ids in
        // play. Env injection + `--effort` bypass happen at the spawn site below.
        if let Some(lm) = cfg.local_llm_model.as_deref().filter(|s| is_valid_local_model_name(s)) {
            model = lm.to_string();
        }
    } else {
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
        // Fable guard — after pin resolution so a pinned Fable session also falls
        // back when Fable is unavailable (manual kill-switch or past its sunset).
        if model == FABLE_MODEL && fable_unavailable() {
            log::info!("assistant_send: {FABLE_MODEL} unavailable — falling back to {FABLE_FALLBACK_MODEL}");
            model = FABLE_FALLBACK_MODEL.to_string();
        }
        // Haiku guard — pulled 2026-06-26; a pinned/stale Haiku session falls back
        // to sonnet before the id reaches the API (mirror helpers.ts).
        if model == HAIKU_MODEL && haiku_unavailable() {
            log::info!("assistant_send: {HAIKU_MODEL} unavailable — falling back to {HAIKU_FALLBACK_MODEL}");
            model = HAIKU_FALLBACK_MODEL.to_string();
        }
    }
    // Effort tier: per-turn override wins, else stored default, else "smart"
    // (--effort high, the API default — mirrors the frontend's loadEffort()).
    let effort = thinking_effort
        .or_else(|| cfg.thinking_effort.clone())
        .unwrap_or_else(|| "smart".to_string());
    // Extended-thinking master switch (per-send). When off on the cloud path we
    // route the CLI through the no-think shim — the CLI always sends a thinking
    // block and no flag disables it, so injecting `thinking:{disabled}` into
    // /v1/messages is the only real off switch.
    // Default OFF when the renderer omits the value — mirrors the frontend's
    // `loadThinkingEnabled` (off-by-default) and the documented behavior. The old
    // `unwrap_or(true)` silently re-enabled extended thinking on any send that
    // didn't carry the flag, adding a 6-8s silent Opus thinking gap before text
    // (Opus omits thinking text → reads as a hang) — the "everything is slow"
    // symptom. Lockstep: keep this `false` aligned with helpers.ts.
    let thinking_on = thinking_enabled.unwrap_or(false);

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
    // Per-tab root: the renderer passes the tab's chosen folder so each pane
    // (and each window) can run turns in a different directory. Wins over the
    // global `current_root` on the first turn; subsequent turns ride the
    // per-session pinned cwd above (which we save from this very value below).
    let tab_root: Option<PathBuf> = root
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_dir());
    // No folder open → fall back to the persistent local scratch workspace
    // (`%LOCALAPPDATA%\Rift\local`) so the standard OAuth no-folder turn gets the
    // full tool set + MCP boundary instead of a dead `--tools ""` chat. Gated to
    // the full-config OAuth path: API-key / local-LLM / (later) sandboxed branches
    // keep the empty roots → no-tools fallback intact (mirrors the `use_full_config`
    // computation below; recomputed here because that binding is resolved later).
    let scratch_eligible =
        cfg.use_full_config.unwrap_or(true) && !use_api_key && !cfg.local_llm_enabled;
    let roots: Vec<PathBuf> = if let Some(p) = pinned_cwd.clone() {
        vec![p]
    } else if let Some(r) = tab_root {
        vec![r]
    } else if let Some(root) = cfg.current_root.as_ref().filter(|p| p.is_dir()) {
        vec![root.clone()]
    } else if scratch_eligible {
        match super::workspace::local_scratch_dir() {
            Ok(scratch) => vec![scratch],
            Err(e) => {
                log::warn!("assistant: local scratch dir unavailable, falling back to no-tools: {e}");
                Vec::new()
            }
        }
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
    // Per-project file-pattern config: if the turn's root belongs to a defined
    // project, thread its include/exclude globs into the MCP child so the
    // workspace tools (read_file/list_dir/grep) honor the user's scoping. No
    // matching project → empty lists → SKIP_DIRS-only baseline (unchanged).
    let (proj_include, proj_exclude) = match roots.first() {
        Some(r) => super::projects::patterns_for_root(&cfg, r),
        None => (Vec::new(), Vec::new()),
    };
    let (mcp_config_path, _mcp_guard, addendum) = if roots.is_empty() {
        (None, None, RIFT_SYSTEM_ADDENDUM_NO_WS)
    } else {
        match write_mcp_config(&session_id, &roots, &trust_level, &window_label, &proj_include, &proj_exclude) {
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

    // Local-LLM mode: swap the Claude-tuned TOOLS addendum for the terse,
    // identity-correcting, structured-tool-call-enforcing local variant.
    // `mcp_config_path.is_some()` is exactly the "tools path" (workspace open +
    // MCP config provisioned); the no-workspace / fallback paths keep NO_WS.
    let addendum = if cfg.local_llm_enabled && mcp_config_path.is_some() {
        RIFT_SYSTEM_ADDENDUM_LOCAL
    } else {
        addendum
    };

    // Pipe the user's prompt via stdin instead of `-p <arg>`. The CLI accepts
    // prompt text on stdin when `-p` is bare; this keeps every arg short and
    // newline-free so .cmd shims work under Rust 1.77+ batch validation
    // (CVE-2024-24576). Addenda + MCP config path are single-line by design,
    // so they're safe as args.
    // "Piggyback" (`use_full_config`) mode: drop the two MCP/slash fences AND
    // add the `user` setting source (below) so Rift inherits the user's full
    // Claude Code setup — global ~/.claude CLAUDE.md, settings.json, hooks,
    // custom MCP servers (from `~/.claude.json`), and slash commands — running
    // their `claude` the same way a terminal would. (The global CLAUDE.md /
    // settings / hooks ride the `user` setting source specifically; an older
    // S71 comment claimed they loaded regardless — that was wrong once
    // `--setting-sources` started excluding `user`. Fixed here + at the
    // setting-sources branch below.)
    // API-key mode forces `--bare`, which suppresses user config wholesale,
    // so we runtime-disable piggyback in that path. Local-LLM mode also forces
    // `--bare` (below), so it disables piggyback for the same reason — keeps
    // Rift's `--mcp-config` the strict source instead of a contradictory
    // `--bare` + piggyback combo.
    let use_full_config =
        cfg.use_full_config.unwrap_or(true) && !use_api_key && !cfg.local_llm_enabled;

    let attachments = attachments.unwrap_or_default();
    validate_attachments(&attachments)?;
    // Prompting modes route per-action permission asks through the stream-json
    // control channel (`--permission-prompt-tool stdio` + the `can_use_tool`
    // round-trip below). bypass/auto never prompt, so they keep the wide
    // allowlist + auto-allow behavior unchanged.
    let prompting_mode = matches!(permission_mode.as_str(), "default" | "acceptEdits" | "plan");

    // Version-gate the spawn flags to the INSTALLED CLI, not the dev's
    // bleeding-edge one. Cached, 5s-bounded probe; unreadable version ⇒
    // conservative-old (every optional flag gated off — see cli_caps). Resolved
    // off the async hot path via spawn_blocking (the probe may shell out).
    let caps = tokio::task::spawn_blocking(super::cli_caps::CliCaps::active)
        .await
        .unwrap_or_else(|_| super::cli_caps::CliCaps::from_version(None));
    // Hard floor: below it the stream-json control handshake Rift relies on
    // doesn't exist, so a turn can't run. Surface an actionable update prompt
    // instead of a silent dead turn. A None version is "unsupported" too, but we
    // can only PROVE too-old when we actually read a sub-floor version; an
    // unreadable version still attempts the spawn at reduced features (the CLI
    // may simply not print --version yet still run).
    if let Some(v) = caps.version {
        if !caps.supported {
            return Err(format!(
                "This Claude Code CLI (v{}.{}.{}) is too old for Rift, which needs ≥ v{}.{}.{}. \
                 Update with `npm i -g @anthropic-ai/claude-code@latest` (or `claude update` for a \
                 native install), then reopen Rift.",
                v.0, v.1, v.2,
                super::cli_caps::MIN_SUPPORTED.0,
                super::cli_caps::MIN_SUPPORTED.1,
                super::cli_caps::MIN_SUPPORTED.2,
            ));
        }
    }

    let mut cmd = claude_command().ok_or_else(|| {
        "Claude CLI not found on this machine. Install Claude Code from https://claude.ai/download \
         (or run `irm https://claude.ai/install.ps1 | iex`), then reopen Rift — or add an Anthropic \
         API key in Settings → CLI session."
            .to_string()
    })?;
    // Default child cwd so the CLI (and any daemon its SessionStart hooks spawn) never inherits Rift's install dir — a live handle on `…\current\` blocks Velopack's update apply. Overridden to the workspace root below when one exists. Prefer LOCALAPPDATA (always a LOCAL path — GPO forbids redirecting it) over temp_dir(), which a corporate machine can redirect to a UNC share that breaks Node's cwd internals.
    let cwd_fallback = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    cmd.current_dir(cwd_fallback);
    cmd.arg("-p")
        .arg("--append-system-prompt").arg(addendum)
        .arg("--output-format").arg("stream-json")
        // Always stream-json input: we now always write a `{type:"user"}`
        // envelope (so the control channel and image attachments share one
        // path), and the `initialize` handshake below requires it.
        .arg("--input-format").arg("stream-json")
        .arg("--verbose")
        .arg("--model").arg(&model)
        .arg("--permission-mode").arg(&permission_mode);
    // The `user` setting source carries the global ~/.claude CLAUDE.md +
    // settings.json + hooks. `use_full_config` on = inherit them (full reskin);
    // off = sandbox (Rift MCP only). OAuth auth is a separate source, kept
    // either way. Latency: with the warm pool (#48) user SessionStart hooks run
    // once per warm child, not per turn. `use_full_config` is in the SpawnKey →
    // toggling it drains + respawns.
    if use_full_config {
        cmd.arg("--setting-sources").arg("user,project,local");
    } else {
        cmd.arg("--setting-sources").arg("project,local");
    }
    // Moves the CLI's own per-machine sections (cwd, env info, memory paths,
    // git status) out of the system prompt and into the first user message.
    // Keeps the cached system-prompt prefix stable across users and across our
    // per-turn workspace-context injection (which rides the user message via
    // <system-reminder>). Gated: an older CLI rejects the unknown flag — without
    // it the per-machine sections just stay in the system prompt (cache churn,
    // not a broken turn).
    if caps.exclude_dynamic_sections {
        cmd.arg("--exclude-dynamic-system-prompt-sections");
    }
    // Partial assistant deltas on the stream. Gated: older CLI rejects it —
    // without it the UI gets whole-message events instead of token-level
    // streaming (still a working turn, just chunkier rendering).
    if caps.include_partial_messages {
        cmd.arg("--include-partial-messages");
    }
    // Piece 2: route per-action permission asks over the stream-json control
    // channel. `stdio` makes the CLI emit a `can_use_tool` `control_request` on
    // stdout (instead of headless auto-deny) and block on a `control_response`
    // we write back to stdin. This is what the Agent SDK passes when a
    // `canUseTool` callback is set; undocumented in `--help` but present in
    // v2.1.152. Harmless for bypass/auto (they never trigger a permission
    // check). `--permission-mode` (set above) still drives WHICH tools ask.
    // Gated: on a CLI without it, prompting modes can't round-trip a permission
    // ask — the `--allowed-tools` allowlist below still gates tools, so the turn
    // runs but per-action prompts silently don't appear (acceptEdits-like).
    if caps.permission_prompt_tool {
        cmd.arg("--permission-prompt-tool").arg("stdio");
    }
    cmd.stdin(Stdio::piped())
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

    // Gated: older CLI rejects `--max-budget-usd`. Without it the per-turn spend
    // cap simply isn't enforced CLI-side (the turn runs uncapped) — a degraded
    // feature, not a broken spawn.
    if caps.max_budget_usd {
        if let Some(budget) = cfg.max_budget_usd.filter(|v| v.is_finite() && *v > 0.0) {
            cmd.arg("--max-budget-usd").arg(format!("{budget}"));
        }
    }

    if !use_full_config {
        // Both gated independently (different floors). Absent `--strict-mcp-config`
        // → the CLI may merge the user's `~/.claude.json` MCP servers (slightly
        // wider than the intended strict sandbox, not a broken turn). Absent
        // `--disable-slash-commands` (lands 2.1.170) → user slash commands stay
        // enabled. Both degrade gracefully on an older CLI rather than crashing.
        if caps.strict_mcp_config {
            cmd.arg("--strict-mcp-config");
        }
        if caps.disable_slash_commands {
            cmd.arg("--disable-slash-commands");
        }
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
        // `DesignSync` (claude.ai/design sync, driven by /design-sync) is the
        // built-in for the Claude Design integration; kept out of SAFE_BUILTINS
        // so its cloud writes ride the can_use_tool prompt. OAuth-path only —
        // it has no auth under --bare.
        // Task* are the CLI 2.1.18x+ rename of TodoWrite (the Tasks-dock driver):
        // TaskCreate/TaskUpdate/TaskList/TaskGet/TaskStop. Keep BOTH names — old
        // CLIs emit TodoWrite, new ones emit Task*; the FE (streaming.ts
        // applyTaskCreate/applyTaskUpdate) already renders both into the same Plan
        // card. Omitting Task* silently killed the Tasks panel on current CLI: the
        // model has the tools but the allowlist gated them out, so it fell back to
        // describing the plan in plain text. (Found in the 2026-06-25 stress test.)
        const BUILTINS: &str = "Agent,Bash,BashOutput,DesignSync,Edit,ExitPlanMode,Glob,Grep,KillBash,KillShell,MultiEdit,NotebookEdit,Read,Skill,SlashCommand,TaskCreate,TaskUpdate,TaskList,TaskGet,TaskStop,TaskOutput,TodoWrite,WebFetch,WebSearch,Write";
        // Read-only / non-mutating subset always auto-approved even in a
        // prompting mode — these shouldn't interrupt the user. Everything
        // omitted (Bash, Edit, Write, MultiEdit, NotebookEdit, Agent, Skill,
        // SlashCommand, ExitPlanMode, and the mutating mcp__rift__* tools)
        // falls through to the `can_use_tool` prompt.
        const SAFE_BUILTINS: &str = "BashOutput,Glob,Grep,KillBash,KillShell,Read,TaskCreate,TaskUpdate,TaskList,TaskGet,TaskStop,TaskOutput,TodoWrite,WebFetch,WebSearch";
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
            // variants — mutating MCP tools (git write) intentionally prompt here.
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
    } else if use_full_config && !prompting_mode {
        // No folder open, but the user runs Rift as their full Claude Code: still
        // expose the WORKSPACE-INDEPENDENT tools so global slash commands +
        // skills + web tools work without a project open (`/cost`, `/help`, the
        // user's own `/commands`, `Skill`, `WebSearch`). File/Bash/Edit tools
        // stay OFF here — there's no `--mcp-config` and no workspace root, so
        // there's no path-safety boundary to scope a write or a shell against;
        // those need an open folder (the `Some(p)` branch above). `mcp__*` is
        // omitted too (no Rift MCP server is spawned without a root). This makes
        // a no-folder chat behave like `claude` in an empty dir rather than a
        // tools-disabled sandbox.
        const NO_WS_TOOLS: &str = "Agent,ExitPlanMode,Skill,SlashCommand,TaskCreate,TaskUpdate,TaskList,TaskGet,TaskStop,TaskOutput,TodoWrite,WebFetch,WebSearch";
        cmd.arg("--allowed-tools").arg(NO_WS_TOOLS);
    } else {
        // No MCP config + sandboxed/prompting (or api-key/local-LLM, which force
        // !use_full_config) → keep the SDK's built-in tools off via empty set.
        // Pure conversational mode.
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

    // Local-LLM mode (experimental): redirect the CLI at a local Anthropic-
    // compatible endpoint (LiteLLM/Ollama). `--bare` forces env-key auth so the
    // CLI ignores OAuth/keychain (added above already if api-key mode). The base
    // URL + local key override anything set in the api-key branch — local wins.
    // Additive + flag-gated; off = the spawn is byte-identical to cloud. Yank =
    // delete this block + the model/effort guards above/below.
    if cfg.local_llm_enabled {
        if !use_api_key {
            cmd.arg("--bare");
        }
        // Re-validate at the sink: the setter guards writes, but a hand-edited
        // config.json could still carry a non-http(s) scheme. Skip if invalid.
        if let Some(base) = cfg
            .local_llm_base_url
            .as_deref()
            .filter(|s| !s.is_empty() && super::config::is_valid_local_base_url(s))
        {
            // Route through Rift's in-process no-think shim when it's bound: it
            // injects `thinking:{type:"disabled"}` into /v1/messages (the only
            // switch that suppresses Ollama's forced reasoning) and forwards to
            // `base`, read fresh per-request. This replaces the external
            // `rift-nothink-proxy.mjs`. If the shim failed to bind, fall back to
            // the raw base URL (user can still run the external proxy).
            let target = super::nothink::shim_base_url().unwrap_or_else(|| base.to_string());
            cmd.env("ANTHROPIC_BASE_URL", target);
        }
        let local_key = crate::secrets::get(crate::secrets::LOCAL_LLM_API_KEY)
            .unwrap_or_else(|| "local".to_string());
        cmd.env("ANTHROPIC_API_KEY", local_key);
        // Local models get a generous output cap. Without this the CLI applies
        // its conservative default `max_tokens` to the /v1/messages request, so
        // a multi-step local turn (explanation + several tool calls) gets
        // guillotined mid-reply — the user sees "Response cut off — reached
        // output length limit / Continue". 8192 lets a real turn finish while
        // staying well inside the model's 16384 num_ctx (input + output).
        cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8192");
    } else if !thinking_on && model != "haiku" {
        // Cloud "thinking off": point the CLI at the in-process shim, which
        // injects `thinking:{type:"disabled"}` into /v1/messages and forwards to
        // Anthropic. Haiku has no extended thinking, so there's nothing to
        // disable (and it would only add a needless hop). If the shim failed to
        // bind, leave ANTHROPIC_BASE_URL unset → falls back to normal thinking.
        if let Some(shim) = super::nothink::shim_base_url() {
            cmd.env("ANTHROPIC_BASE_URL", shim);
        }
    }

    // Effort-gated extended thinking via the CLI's `--effort` flag (the CLI
    // accepts low/medium/high/xhigh/max). Haiku skips wholesale — the API
    // rejects effort on Haiku 4.5. Tier mapping (MUST mirror frontend
    // `effortToFlag` in src/lib/state/assistant/helpers.ts):
    //   none  → --effort low     (minimal reasoning, fastest TTFT)
    //   quick → --effort medium  (light reasoning, leaner tool use)
    //   smart → --effort medium  (the responsive interactive default — Anthropic's
    //            recommended `medium`; the CLI default is `high` but their API
    //            guidance says to override it for interactive use to avoid the
    //            "UI appears frozen" long hidden pre-pass)
    //   deep  → --effort high    (the old API default — explicit heavy reasoning)
    //   ultra → --effort xhigh + the ultracode workflow settings key
    // `max` is deliberately not exposed — per Anthropic's guidance it shows
    // diminishing returns and is prone to overthinking vs xhigh.
    // Earlier impl set `MAX_THINKING_TOKENS` env, but the CLI doesn't honor
    // that env directly — `--effort` is the documented API.
    // Thinking-display caveat (verified 2026-06-15, CLI 2.1.177): whether we get
    // plaintext reasoning is NOT a -p-mode encryption thing — it's the model's
    // `thinking.display` default. Sonnet 4.6 defaults to "summarized" and DOES
    // stream `thinking_delta` text headless; Opus 4.8/4.7 default to "omitted"
    // and stream only `content_block_start{type:thinking}` + `signature_delta`
    // (empty text). The CLI exposes no flag to override display to "summarized"
    // (`claude --help` has only `--effort`/`--include-partial-messages`), so
    // Opus thinking text can't be surfaced today — gated upstream, not here.
    // #237: normalize effort BEFORE logging so newlines/ANSI in the raw
    // renderer-supplied string can't reach the log stream. The CLI flag itself
    // was safe (string-arg passthrough) but the log line was unredacted.
    // Clamp the requested tier to the model's ceiling before mapping to a flag,
    // and reject a tier the ladder doesn't define. Sonnet 4.6 tops out at
    // "smart" (medium); xhigh + the ultracode workflow key are Opus-tier only — so
    // a stale out-of-range tier (e.g. a workspace pinned to `ultra` under Opus,
    // then switched to Sonnet) can't push Sonnet to xhigh/ultracode. This is the
    // only point that builds the actual CLI args, so it's the authoritative
    // gate; the frontend coerces too. `clamp_effort`/`model_max_effort` mirror
    // MODEL_MAX_EFFORT in src/lib/state/assistant/helpers.ts.
    if !is_valid_effort_tier(&effort) {
        log::warn!("assistant_send: unknown effort tier {effort:?} — treating as deep (high)");
    }
    let effort_tier = clamp_effort(&effort, &model);
    let effort_level = match effort_tier {
        "none" => "low",
        "quick" | "smart" => "medium", // "smart" = the responsive interactive default (Anthropic's recommended medium); see effortToFlag in helpers.ts
        "ultra" => "xhigh",
        _ /* "deep" or unknown */ => "high",
    };
    log::info!("assistant_send: effort tier={effort_tier} flag={effort_level} model={model} session={session_id}");
    // Local-LLM mode skips `--effort` wholesale — local models/proxies don't
    // implement Anthropic extended-thinking tiers and 4xx or silently ignore it.
    //
    // #68 (cont.226): thinking-OFF must STILL send `--effort low`. The CLI has no
    // `--thinking`/disable flag (verified `claude --help` 2.1.195: `--effort` is
    // the only thinking control), so the no-think shim — which rewrites the
    // request body to `thinking:{disabled}` — was the only real off switch. But
    // the OAuth CLI ignores our `ANTHROPIC_BASE_URL` override (proven live: 0
    // shim connections, 12-13 direct conns to api.anthropic.com), so the shim is
    // bypassed. With no `--effort` sent, the CLI defaults to `high` → a `hello`
    // "reckons" ~12s. Sending `low` forces minimal reasoning regardless of
    // whether the shim is reachable. (low ≠ fully off, but it's the CLI floor and
    // kills the multi-second pre-pass; if the shim ever starts working again the
    // injected disabled-block still wins on top of this.)
    // `--effort` gated on caps.effort: an older CLI without the flag rejects it.
    let send_effort = if thinking_on { Some(effort_level) } else { Some("low") };
    if !cfg.local_llm_enabled && model != "haiku" && caps.effort {
        if let Some(level) = send_effort {
            cmd.arg("--effort").arg(level);
        }
        // Ultracode tier: xhigh effort + autonomous dynamic-workflow
        // orchestration. The workflow behavior rides the CLI's `ultracode`
        // settings key (a boolean read into app state, gated server-side by the
        // user's plan entitlement). `--settings` merges this additively over
        // user/project/local settings — when unentitled the CLI ignores it and
        // the session simply runs at xhigh effort. Haiku is excluded (it skips
        // extended thinking + workflow orchestration wholesale). Only when
        // thinking is actually on — a thinking-off turn never rides ultracode.
        if thinking_on && effort_tier == "ultra" && caps.settings_flag {
            cmd.arg("--settings").arg(r#"{"ultracode":true}"#);
        }
    }

    log::info!(
        "assistant_send: spawn session_id={} first_turn={} model={} effort={} thinking_on={} perm={} use_full_config={} mcp={} api_key={} local_llm={} cli_ver={:?} caps=[effort={} perm_tool={} excl_dyn={} partial={} budget={} settings={}]",
        session_id, is_first_turn, model, effort_level, thinking_on, permission_mode, use_full_config, mcp_config_path.is_some(), use_api_key, cfg.local_llm_enabled,
        caps.version, caps.effort, caps.permission_prompt_tool, caps.exclude_dynamic_sections, caps.include_partial_messages, caps.max_budget_usd, caps.settings_flag
    );
    log::debug!(
        "assistant_send: caps(cont) strict_mcp={} disable_slash={}",
        caps.strict_mcp_config, caps.disable_slash_commands
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
    // Phase C (forward-compat seam — currently never populated): seed the next
    // CLI session with a prior-conversation summary after a Rift-side compaction
    // remint, via a <system-reminder> so the cached system-prompt prefix stays
    // stable. The frontend hard-codes `priorContextSummary: null` (send.ts) — the
    // CLI now does compaction NATIVELY in-process (its own `compact_boundary`
    // event, surfaced by streaming.ts::appendCliCompaction) and the warm child
    // carries context across turns, so Rift never needs to re-inject it. This
    // block stays as a validated, capped hook in case a future Rift-driven remint
    // path wants it; until a caller passes a non-empty summary it's a no-op.
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
    // The per-turn user message — always a stream-json `user` envelope (text +
    // optional image blocks). Sent by the reader loop once the `initialize`
    // handshake is acknowledged. Shares build_user_envelope with steer injection.
    let user_line: Vec<u8> = build_user_envelope(&effective_prompt, &attachments)?;

    // #48 warm pool: the spawn "signature". Every field below is baked into the
    // child's argv/env at spawn and CANNOT change in-flight — so a turn whose
    // key differs from the warm child's must drain + cold-respawn (with
    // `--resume`). `addendum` is a `&'static str` (one of three constants), so
    // its pointer is a stable, cheap fingerprint of the system-prompt variant.
    let key = warm_pool::SpawnKey {
        model: model.clone(),
        root: roots.first().map(|p| p.to_string_lossy().into_owned()),
        permission_mode: permission_mode.clone(),
        prompting_mode,
        use_full_config,
        use_api_key,
        local_llm_enabled: cfg.local_llm_enabled,
        thinking_on,
        effort_level: effort_level.to_string(),
        trust_level: trust_level.clone(),
        addendum_ptr: addendum.as_ptr() as usize,
    };

    // #67 pre-warm path: spawn the child, run the init handshake, and PARK it in
    // the warm pool with NO first turn. The real `assistant_send` that follows
    // reuses it via `dispatch_turn`'s existing warm path. Returns as soon as the
    // child is registered (the handshake completes inside the reader loop).
    if prewarm {
        return prewarm_spawn(app, window_label, session_id, key, cmd, _mcp_guard, model);
    }

    // Dispatch through the warm pool: reuse a live child whose key matches, else
    // cold-spawn one and keep it warm. `_mcp_guard` outlives the process via the
    // WarmChild so the per-turn config file survives every reused turn. The
    // whole spawn + reader loop + reaping moved into `dispatch_turn`.
    dispatch_turn(
        app,
        window_label,
        session_id,
        key,
        cmd,
        user_line,
        _mcp_guard,
        is_first_turn,
        model,
    )
    .await
}

/// Reuse-or-cold-spawn a warm `claude` child for this turn, run the turn, and
/// return once its `result` frame lands (DONE already emitted by the reader
/// loop). The highest-risk function in the file — see warm_pool.rs + the design
/// doc. Keeps the existing per-turn streaming/permission/steer plumbing intact;
/// the only structural change vs the old inline path is that the reader loop is
/// LONG-LIVED (one per warm child) and parks on `result` instead of EOF-ing.
#[allow(clippy::too_many_arguments)]
async fn dispatch_turn(
    app: AppHandle,
    window_label: String,
    session_id: String,
    key: warm_pool::SpawnKey,
    cmd: tokio::process::Command,
    user_line: Vec<u8>,
    mcp_guard: Option<McpConfigGuard>,
    is_first_turn: bool,
    model: String,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    // Clear any stale stop marker for this session (e.g. retry after a previous
    // stop) before we touch the warm child.
    take_session_stopped(&session_id);

    // 1) Try the warm path: a live child whose signature matches.
    if let Some(arc) = warm_pool::get(&session_id) {
        // M5/M7: lock the child only to read tx + key + in-progress, then RELEASE
        // before any await. Never hold the WarmChild mutex across the turn.
        let reuse: Option<(mpsc::UnboundedSender<warm_pool::TurnCmd>, Arc<std::sync::atomic::AtomicBool>)> = {
            let mut g = match arc.lock() { Ok(g) => g, Err(p) => p.into_inner() };
            if g.key != key {
                // Signature changed (model/effort/perm/root/mode) → can't reuse.
                // Drain the old child + fall through to cold respawn (--resume).
                log::info!("warm_pool: signature changed for {session_id} — draining + cold respawn");
                None
            } else if g.turn_in_progress.load(Ordering::Acquire) {
                // Concurrent turn on one session (failure mode #1). The frontend
                // serializes turns per tab via the queue, so this is a bug/race;
                // reject rather than interleave two turns on one stdin.
                return Err("a turn is already in progress for this session".into());
            } else {
                // Mark in-progress BEFORE releasing the lock so a racing second
                // send sees it. The reader loop clears it on `result` (M6).
                g.turn_in_progress.store(true, Ordering::Release);
                g.last_used = std::time::Instant::now();
                Some((g.turn_tx.clone(), g.turn_in_progress.clone()))
            }
        };
        // Emit the warm-hit AFTER releasing the WarmChild lock above: emit_dispatch
        // calls warm_pool::pool_size() which locks the WARM_CHILDREN registry, and
        // acquiring the registry lock while holding a WarmChild guard inverts the
        // lock order the rest of the pool uses (registry → child) — a deadlock
        // hazard + extra contention on the hottest path. `reuse.is_some()` ⇒ hit.
        if reuse.is_some() {
            emit_dispatch(&session_id, "hit", &model, &key);
        }
        // `user_line` is moved into the TurnCmd only on the reuse path; on every
        // fall-through (mismatch / dead-on-send) it's recovered so the cold path
        // below still has it. `user_line` stays a binding the cold call consumes.
        let user_line = match reuse {
            Some((turn_tx, in_progress)) => {
                let (done_tx, done_rx) = oneshot::channel();
                let bg_evict = Arc::new(std::sync::atomic::AtomicBool::new(false));
                // Keep a copy: if the reused child turns out to be dead (killed
                // while parked), the loop reports DeadOnReuse and we retry cold —
                // which needs user_line again (the original was moved into the cmd).
                let retry_line = user_line.clone();
                let cmd_msg = warm_pool::TurnCmd {
                    user_line,
                    app: app.clone(),
                    window_label: window_label.clone(),
                    done: done_tx,
                    bg_evict,
                };
                match turn_tx.send(cmd_msg) {
                    Ok(()) => {
                        // Await the turn's completion signal from the reader loop.
                        match done_rx.await {
                            // Dead-on-reuse sentinel: the parked child was dead.
                            // The loop already dropped it; fall through to cold
                            // respawn with the preserved user_line — no UI error.
                            Ok(Err(ref s)) if s == RETRY_COLD_SENTINEL => {
                                in_progress.store(false, Ordering::Release);
                                retry_line
                            }
                            Ok(r) => return r,
                            // Reader dropped done_tx without sending (loop exited /
                            // panicked mid-turn). Surface so the UI unwedges.
                            Err(_) => {
                                in_progress.store(false, Ordering::Release);
                                let _ = app.emit_to(&window_label, ERROR_EVENT, serde_json::json!({
                                    "session_id": session_id,
                                    "message": "the warm CLI process ended unexpectedly — retry the turn",
                                }));
                                return Err("warm child reader ended before result".into());
                            }
                        }
                    }
                    Err(send_err) => {
                        // Reader loop gone (child died between our get + send).
                        // Clear in-progress, drop the dead child, recover user_line
                        // from the rejected message, fall through to cold respawn.
                        in_progress.store(false, Ordering::Release);
                        warm_pool::remove_if(&session_id, &arc);
                        log::info!("warm_pool: warm child for {session_id} dead on send — cold respawn");
                        send_err.0.user_line
                    }
                }
            }
            None => {
                // Signature mismatch → drain the old child so the cold spawn below
                // replaces it cleanly. The old reader loop holds a `turn_tx` clone
                // (via its WarmChild), so dropping the registry entry does NOT make
                // its `recv()` return None — we must reap the old child by PID, or
                // it leaks until the 30-min idle-evict. Kill BEFORE cold_spawn's
                // set_session_pid overwrites the SESSION_PIDS entry.
                let old_pid = warm_pool::pid_of(&session_id);
                warm_pool::remove_if(&session_id, &arc);
                if let Some(p) = old_pid {
                    kill_child_tree(p);
                    log::info!("warm_pool: drained + reaped old child pid={p} for {session_id} (signature change)");
                }
                emit_dispatch(&session_id, "signature_drain", &model, &key);
                user_line
            }
        };

        // 2) Cold path (warm child existed but couldn't be reused).
        return cold_spawn_and_run(app, window_label, session_id, key, cmd, user_line, mcp_guard, is_first_turn, model).await;
    }

    // 2) Cold path: no warm child at all — spawn one, register it, run turn 1.
    emit_dispatch(&session_id, "cold", &model, &key);
    cold_spawn_and_run(app, window_label, session_id, key, cmd, user_line, mcp_guard, is_first_turn, model).await
}

/// Emit a structured warm-pool dispatch event (Phase 2a). `outcome` ∈
/// "hit" | "signature_drain" | "cold" | "dead_on_send". Fire-and-forget; the bus
/// scrubs + rate-limits. Resource "warm_pool" so the console can isolate the
/// latency-critical path the cont.219 hunt had to hand-probe.
fn emit_dispatch(session_id: &str, outcome: &str, model: &str, key: &warm_pool::SpawnKey) {
    // Phase 5: also bump a per-outcome counter via the metric! primitive — gives
    // the health panel a running warm-hit total without re-scanning the ring.
    match outcome {
        "hit" => crate::metric!("warm_pool.hit"),
        "cold" => crate::metric!("warm_pool.cold"),
        "signature_drain" => crate::metric!("warm_pool.drain"),
        _ => {}
    }
    crate::diagnostics::emit_with_fields(
        crate::diagnostics::DiagStage::Log,
        crate::diagnostics::DiagLevel::Info,
        Some("warm_pool"),
        Some(file!()),
        "dispatch",
        serde_json::json!({
            "outcome": outcome,
            "session": session_id,
            "model": model,
            "effort": key.effort_level.as_str(),
            "pool_size": warm_pool::pool_size(),
        }),
    );
}

/// Cold-spawn a fresh `claude` child, register it in the warm pool, start its
/// long-lived reader loop, hand it the first turn, and await that turn's
/// result. The reader loop OWNS stdin/stdout/stderr for the child's whole life
/// and parks between turns — so this fn returns when the FIRST turn's `result`
/// lands, but the process stays alive for subsequent reused turns.
#[allow(clippy::too_many_arguments)]
async fn cold_spawn_and_run(
    app: AppHandle,
    window_label: String,
    session_id: String,
    key: warm_pool::SpawnKey,
    mut cmd: tokio::process::Command,
    user_line: Vec<u8>,
    mcp_guard: Option<McpConfigGuard>,
    is_first_turn: bool,
    model: String,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    // Make sure the idle-evict sweeper is running (idempotent; first cold spawn
    // starts it, so a process that never opens a chat pays nothing).
    warm_pool::ensure_evictor();

    // Capture effort before `key` is moved into the warm entry (perf grouping, WS5).
    let effort = key.effort_level.clone();
    let turn_start = std::time::Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("spawn `claude`: {e}"))?;
    let turn_pid = child.id();
    if let Some(pid) = turn_pid {
        set_session_pid(&session_id, pid);
    } else {
        // #67: `child.id()` returns None when the process already exited by the
        // time we ask (immediate-exit on bad args). Surface it so a later
        // assistant_stop's "no PID, return Ok" isn't mistaken for a real stop.
        log::warn!("cold_spawn: child PID unavailable for session {session_id} (process already exited?)");
    }

    // #39: a concurrent stop arriving in the spawn window would find no PID and
    // silently drop. Re-check now that the PID is registered.
    if take_session_stopped(&session_id) {
        log::info!("cold_spawn: stop arrived during spawn for {session_id} — killing child");
        if let Err(e) = child.start_kill() {
            log::warn!("cold_spawn: start_kill failed for {session_id} during stop-on-spawn: {e}");
        }
        if let Some(p) = turn_pid { clear_session_pid_if(&session_id, p); }
        let _ = app.emit_to(&window_label, DONE_EVENT, serde_json::json!({
            "session_id": session_id, "exit_code": -1,
        }));
        return Ok(());
    }

    // A1: take() + guard — None means the child died between spawn and now.
    let Some(stdin) = child.stdin.take() else {
        let _ = child.start_kill();
        clear_session_pid(&session_id);
        return Err("claude stdin unavailable — process killed".into());
    };
    let stdout = child.stdout.take().ok_or_else(|| "claude stdout missing".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "claude stderr missing".to_string())?;

    // Steer channel — registered for the WARM CHILD's whole life (not per-turn).
    // `assistant_steer` looks it up; the reader loop owns the receiver and gates
    // on `turn_in_progress` so an idle-between-turns steer answers no_active_turn.
    let (steer_tx, steer_rx) = mpsc::unbounded_channel::<SteerMsg>();
    register_steer_tx(&session_id, steer_tx.clone());

    // Per-child turn channel: dispatch_turn sends a TurnCmd; the loop runs it.
    let (turn_tx, turn_rx) = mpsc::unbounded_channel::<warm_pool::TurnCmd>();
    let turn_in_progress = Arc::new(std::sync::atomic::AtomicBool::new(true)); // first turn starts in-progress

    // Register the warm child BEFORE spawning the loop so a racing second send
    // (or the evictor) sees a coherent entry.
    let warm = Arc::new(std::sync::Mutex::new(warm_pool::WarmChild {
        turn_tx: turn_tx.clone(),
        key,
        turn_in_progress: turn_in_progress.clone(),
        last_used: std::time::Instant::now(),
        pid: turn_pid,
    }));
    warm_pool::insert(&session_id, warm.clone());

    // The first turn's completion signal.
    let (done_tx, done_rx) = oneshot::channel();
    let first_bg_evict = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let first_turn = warm_pool::TurnCmd {
        user_line,
        app: app.clone(),
        window_label: window_label.clone(),
        done: done_tx,
        bg_evict: first_bg_evict,
    };

    // Spawn the long-lived reader loop. It owns child/stdin/stdout/stderr, the
    // steer receiver, the turn receiver, the mcp guard, and the warm-pool Arc —
    // all dropped together when the loop exits (death / evict / signature drain).
    tokio::spawn(run_turn_loop(RunCtx {
        child,
        stdin,
        stdout,
        stderr,
        steer_rx,
        turn_rx,
        first_turn: Some(first_turn),
        session_id: session_id.clone(),
        turn_pid,
        turn_in_progress: turn_in_progress.clone(),
        warm: warm.clone(),
        steer_tx,
        mcp_guard,
        app: app.clone(),
        window_label: window_label.clone(),
        is_first_turn,
        model,
        effort,
        turn_start,
    }));

    // Await the FIRST turn's result. Subsequent turns ride the warm child.
    match done_rx.await {
        Ok(r) => r,
        Err(_) => {
            turn_in_progress.store(false, Ordering::Release);
            let _ = app.emit_to(&window_label, ERROR_EVENT, serde_json::json!({
                "session_id": session_id,
                "message": "the CLI process ended before producing a result — retry the turn",
            }));
            Err("warm child reader ended before first result".into())
        }
    }
}

/// #67 pre-warm: cold-spawn a `claude` child, register it in the warm pool with
/// NO first turn, start its long-lived reader loop (which runs the init
/// handshake then PARKS on `turn_rx`), and return IMMEDIATELY. The next real
/// `assistant_send` for this session reuses the parked child via the normal
/// warm path. A structural twin of `cold_spawn_and_run` minus the first turn +
/// the `done_rx.await` — the spare never runs a model turn until a real send
/// arrives. Synchronous (no `.await`) so the trigger returns instantly; the
/// handshake + any spawn failure plays out inside the spawned reader loop (a
/// dead spare just leaves the pool empty → the real send cold-respawns).
#[allow(clippy::too_many_arguments)]
fn prewarm_spawn(
    app: AppHandle,
    window_label: String,
    session_id: String,
    key: warm_pool::SpawnKey,
    mut cmd: tokio::process::Command,
    mcp_guard: Option<McpConfigGuard>,
    model: String,
) -> Result<(), String> {
    // Make sure the idle-evict sweeper is running so an unadopted spare ages out
    // on the same timers as any warm child (idempotent).
    warm_pool::ensure_evictor();

    let effort = key.effort_level.clone();
    let model_log = model.clone();
    let turn_start = std::time::Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("prewarm spawn `claude`: {e}"))?;
    let turn_pid = child.id();
    if let Some(pid) = turn_pid {
        set_session_pid(&session_id, pid);
    } else {
        log::warn!("prewarm_spawn: child PID unavailable for session {session_id} (process already exited?)");
    }

    // A1: same take()+guard as the cold path — None means the child died between
    // spawn and now.
    let Some(stdin) = child.stdin.take() else {
        let _ = child.start_kill();
        clear_session_pid(&session_id);
        return Err("prewarm: claude stdin unavailable — process killed".into());
    };
    let stdout = child.stdout.take().ok_or_else(|| "prewarm: claude stdout missing".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "prewarm: claude stderr missing".to_string())?;

    // Steer channel registered for the spare's whole life (the first real turn
    // rides this same registration — no re-register on adoption).
    let (steer_tx, steer_rx) = mpsc::unbounded_channel::<SteerMsg>();
    register_steer_tx(&session_id, steer_tx.clone());

    let (turn_tx, turn_rx) = mpsc::unbounded_channel::<warm_pool::TurnCmd>();
    // A spare is IDLE from birth: no turn in progress. (A racing real send sees
    // turn_in_progress=false and reuses it via dispatch_turn's warm path.)
    let turn_in_progress = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let warm = Arc::new(std::sync::Mutex::new(warm_pool::WarmChild {
        turn_tx: turn_tx.clone(),
        key,
        turn_in_progress: turn_in_progress.clone(),
        last_used: std::time::Instant::now(),
        pid: turn_pid,
    }));
    warm_pool::insert(&session_id, warm.clone());

    // Reader loop with first_turn: None → handshake then park.
    tokio::spawn(run_turn_loop(RunCtx {
        child,
        stdin,
        stdout,
        stderr,
        steer_rx,
        turn_rx,
        first_turn: None,
        session_id: session_id.clone(),
        turn_pid,
        turn_in_progress,
        warm,
        steer_tx,
        mcp_guard,
        app,
        window_label,
        is_first_turn: true,
        model,
        effort,
        turn_start,
    }));
    log::info!("prewarm_spawn: spare warm child spawned + parked for session {session_id} (model={model_log})");
    Ok(())
}

/// Everything the long-lived reader loop owns for one warm child. Bundled into
/// a struct so `run_turn_loop` takes a single arg (clippy + readability).
struct RunCtx {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    steer_rx: mpsc::UnboundedReceiver<SteerMsg>,
    turn_rx: mpsc::UnboundedReceiver<warm_pool::TurnCmd>,
    /// The first turn to run, or `None` for a #67 pre-warm spare: the loop runs
    /// the init handshake then parks on `turn_rx` for the real first turn.
    first_turn: Option<warm_pool::TurnCmd>,
    session_id: String,
    turn_pid: Option<u32>,
    turn_in_progress: Arc<std::sync::atomic::AtomicBool>,
    warm: Arc<std::sync::Mutex<warm_pool::WarmChild>>,
    steer_tx: mpsc::UnboundedSender<SteerMsg>,
    mcp_guard: Option<McpConfigGuard>,
    /// App handle + window label used ONLY to surface a handshake-write failure
    /// (the per-turn app/window ride each TurnCmd). For a pre-warm spare there's
    /// no turn to error yet, so an init failure just logs + tears the child down.
    app: AppHandle,
    window_label: String,
    is_first_turn: bool,
    model: String,
    effort: String,
    turn_start: std::time::Instant,
}

/// The long-lived per-warm-child reader loop. Owns stdin/stdout for the child's
/// whole life. Runs the `initialize` handshake ONCE, then for each turn: writes
/// the user envelope, streams NDJSON (forwarding + control-channel handling +
/// steer injection) exactly as the old inline path did, and on `result` emits
/// DONE, signals the turn's `done` channel, clears `turn_in_progress`, and
/// PARKS on the turn channel for the next envelope — instead of EOF-ing.
///
/// The loop exits (→ stdin drops → child EOFs → reaped) when: the child dies,
/// the turn channel closes (evict / signature drain — all senders dropped), a
/// turn requested bg-evict (M3), or a fatal stdin write error.
async fn run_turn_loop(mut ctx: RunCtx) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use std::sync::atomic::Ordering;

    let mut stdin = ctx.stdin;
    let mut lines = BufReader::new(ctx.stdout).lines();
    let mut steer_rx = ctx.steer_rx;
    let mut turn_rx = ctx.turn_rx;

    // Persistent stderr reader (M2): on the warm path `child.wait()` never
    // returns non-zero mid-life, so stderr can't be drained at exit. Keep a
    // rolling tail in a shared buffer that the EOF handler reads for auth/resume
    // failure remapping. Capped (64 KiB, keep the tail — fatal lines live at end).
    let stderr_tail: Arc<std::sync::Mutex<String>> = Arc::new(std::sync::Mutex::new(String::new()));
    let stderr_tail_task = stderr_tail.clone();
    let stderr = ctx.stderr;
    let stderr_handle = tokio::spawn(async move {
        const STDERR_CAP: usize = 64 * 1024;
        const STDERR_TRIM: usize = 32 * 1024;
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            let mut buf = match stderr_tail_task.lock() { Ok(g) => g, Err(p) => p.into_inner() };
            buf.push_str(&l);
            buf.push('\n');
            if buf.len() > STDERR_CAP {
                let cut = buf.as_bytes()[STDERR_TRIM..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|n| STDERR_TRIM + n + 1)
                    .unwrap_or_else(|| {
                        let mut c = STDERR_TRIM;
                        while c < buf.len() && !buf.is_char_boundary(c) { c += 1; }
                        c
                    });
                buf.drain(..cut);
            }
        }
    });

    // The first turn (if any) is in hand; pre-warm spares start with None and
    // park on turn_rx after the handshake. Take it out of ctx so a handshake
    // failure can consume its `done` channel.
    let mut current: Option<warm_pool::TurnCmd> = ctx.first_turn.take();

    // 1) initialize handshake — ONCE per process. Required so the CLI routes
    //    permission asks over the control channel as `can_use_tool`. A write
    //    failure here means the child is already dead. For a real first turn we
    //    surface the error to its `done` channel + emit to its window; for a
    //    pre-warm spare (current == None) there's no turn yet, so we emit to the
    //    fallback app/window and just tear the child down (the next real send
    //    cold-respawns cleanly — a dead spare is invisible to the user).
    const INIT: &[u8] = b"{\"type\":\"control_request\",\"request_id\":\"rift-init\",\"request\":{\"subtype\":\"initialize\",\"hooks\":{}}}\n";
    let init_result = match stdin.write_all(INIT).await {
        Ok(()) => stdin.flush().await.map_err(|e| format!("flush initialize: {e}")),
        Err(e) => Err(format!("write initialize: {e}")),
    };
    if let Err(msg) = init_result {
        let (app_err, win_err) = match current.as_ref() {
            Some(t) => (&t.app, t.window_label.as_str()),
            None => (&ctx.app, ctx.window_label.as_str()),
        };
        let _ = app_err.emit_to(win_err, ERROR_EVENT, serde_json::json!({
            "session_id": ctx.session_id, "message": msg.clone(),
        }));
        if let Some(t) = current.take() {
            let _ = t.done.send(Err(msg));
        }
        loop_cleanup(&ctx.session_id, ctx.turn_pid, &ctx.steer_tx, &ctx.warm, &mut ctx.child).await;
        stderr_handle.abort();
        drop(ctx.mcp_guard);
        return;
    }

    let mut handshake_done = false;
    // A pre-warm spare starts with NO in-hand turn (`current == None`): its real
    // first turn arrives on `turn_rx` and is, latency-wise, a WARM reuse (the
    // process is already spawned + handshaked), so it must NOT inherit the cold
    // `ctx.turn_start` (which is the spare-spawn instant, possibly minutes old) —
    // that would log a bogus multi-minute TTFT. Gate the cold-first timing on
    // whether we actually hold a first turn here.
    let has_inhand_first = current.is_some();
    let cold_first = ctx.is_first_turn && has_inhand_first;
    let mut first_turn_flag = ctx.is_first_turn && has_inhand_first;

    'turns: loop {
        // Park for a turn if we don't have one in hand.
        let turn = match current.take() {
            Some(t) => t,
            None => {
                ctx.turn_in_progress.store(false, Ordering::Release);
                match turn_rx.recv().await {
                    Some(t) => {
                        // dispatch_turn already set turn_in_progress = true before
                        // sending; re-assert for safety on the loop side.
                        ctx.turn_in_progress.store(true, Ordering::Release);
                        t
                    }
                    None => break 'turns, // all senders dropped → evict/drain → exit
                }
            }
        };
        let app_out = turn.app.clone();
        let win_label = turn.window_label.clone();
        let stream_sid = ctx.session_id.clone();
        let bg_evict = turn.bg_evict.clone();
        let done = turn.done;
        let user_line = turn.user_line;

        let turn_start = if first_turn_flag { ctx.turn_start } else { std::time::Instant::now() };

        // #67: a pre-warm spare started with NO in-hand first turn; the FIRST
        // turn it ever runs is the adopted real send. If that adopted turn hits
        // EOF before any output, the spare had silently died while parked — but
        // there's no user-visible prior turn here, so (like DeadOnReuse) we
        // retry it cold instead of surfacing a spawn-failure error. This guard
        // is true only on the spare's first adopted turn (handshake not yet
        // done + never had an in-hand turn); after the first turn completes the
        // handshake, normal warm semantics resume.
        let spare_first_adopt = !has_inhand_first && !handshake_done;
        if spare_first_adopt {
            log::info!("warm_pool: pre-warm spare ADOPTED by first real turn, session={stream_sid} (warm-start, cold-boot skipped)");
        }

        // Per-turn run: stream NDJSON until result/eof/fatal.
        let outcome = stream_one_turn(StreamCtx {
            stdin: &mut stdin,
            lines: &mut lines,
            steer_rx: &mut steer_rx,
            app_out: &app_out,
            win_label: &win_label,
            stream_sid: &stream_sid,
            model: &ctx.model,
            effort: &ctx.effort,
            user_line: &user_line,
            handshake_done: &mut handshake_done,
            bg_evict: &bg_evict,
            turn_start,
        }).await;

        // Clear in-progress the instant the turn ends (M6: reader-side).
        ctx.turn_in_progress.store(false, Ordering::Release);
        first_turn_flag = false;

        match outcome {
            TurnOutcome::Result => {
                log::info!("warm_pool: turn result {} ms session={}", turn_start.elapsed().as_millis(), stream_sid);
                let _ = done.send(Ok(()));
                // M3: a turn that spawned a background child taints the inherited
                // stdout pipe — don't keep it warm. Exit the loop (→ stdin drops).
                if bg_evict.load(Ordering::Acquire) {
                    log::info!("warm_pool: bg-spawn turn — evicting warm child {stream_sid}");
                    break 'turns;
                }
                continue 'turns; // reuse: park for the next turn
            }
            TurnOutcome::Fatal(msg) => {
                let _ = app_out.emit_to(&win_label, ERROR_EVENT, serde_json::json!({
                    "session_id": stream_sid, "message": msg.clone(),
                }));
                let _ = done.send(Err(msg));
                break 'turns;
            }
            TurnOutcome::DeadOnReuse => {
                // Warm child died while parked; this reused turn produced no
                // output. Drop the dead child + registry entry and tell the
                // dispatcher to retry cold (fresh spawn + --resume). NO UI error.
                log::info!("warm_pool: reused turn found dead child {stream_sid} — signalling cold retry");
                warm_pool::remove_if(&ctx.session_id, &ctx.warm);
                let _ = done.send(Err(RETRY_COLD_SENTINEL.into()));
                break 'turns;
            }
            TurnOutcome::Stalled => {
                // No stdout for the no-progress ceiling — the child is wedged
                // (hung init, stuck CLI MCP connect, dead-but-not-EOF'd pipe).
                // Honest error (NOT "the Anthropic API"), drop the child so the
                // next send cold-respawns, and end the turn so the UI unwedges.
                let msg = format!(
                    "Claude stopped responding — no output for {STREAM_NO_PROGRESS_SECS}s, so Rift ended the turn. \
                     This is the local Claude process stalling (a hung startup, a stuck tool/MCP connection, or a dropped pipe), not a slow model. \
                     Try the turn again; if it keeps happening, run `claude` in a terminal to confirm the CLI itself works."
                );
                warm_pool::remove_if(&ctx.session_id, &ctx.warm);
                let _ = app_out.emit_to(&win_label, ERROR_EVENT, serde_json::json!({
                    "session_id": stream_sid, "message": msg.clone(),
                }));
                let _ = done.send(Err(msg));
                break 'turns;
            }
            TurnOutcome::Eof => {
                // stdout closed without a result — the child exited (crash, bad
                // args, user stop, lost --resume). Disambiguate via exit status +
                // the stderr tail, exactly like the old post-wait path.
                warm_pool::remove_if(&ctx.session_id, &ctx.warm);
                // #67: a spare that died while parked surfaces as EOF on its first
                // adopted turn. Unless it's an AUTH/resume failure (a real,
                // user-actionable error the cold respawn would just repeat), retry
                // it cold like DeadOnReuse — the user never sees the dead spare.
                let stderr_buf = {
                    let g = match stderr_tail.lock() { Ok(g) => g, Err(p) => p.into_inner() };
                    g.clone()
                };
                if spare_first_adopt
                    && !is_auth_rejection(&stderr_buf)
                    && !stderr_buf.contains("No conversation found with session ID:")
                {
                    log::info!("warm_pool: pre-warm spare {stream_sid} died while parked — silent cold retry");
                    let _ = ctx.child.wait().await;
                    let _ = done.send(Err(RETRY_COLD_SENTINEL.into()));
                    break 'turns;
                }
                let status = ctx.child.wait().await.ok();
                emit_turn_end_error(
                    &app_out, &win_label, &stream_sid, cold_first, status, &stderr_buf, done,
                ).await;
                break 'turns;
            }
        }
    }

    // Loop exited — tear down. Clear registry entry (if still ours), PID, steer.
    loop_cleanup(&ctx.session_id, ctx.turn_pid, &ctx.steer_tx, &ctx.warm, &mut ctx.child).await;
    stderr_handle.abort();
    drop(ctx.mcp_guard); // delete the per-session MCP config file now (not per-turn)
    log::debug!("warm_pool: reader loop exited for {} (model={})", ctx.session_id, ctx.model);
}

/// Common loop-exit teardown: drop the warm registry entry if it's still ours,
/// clear the PID + steer-tx, and best-effort reap the child.
async fn loop_cleanup(
    session_id: &str,
    turn_pid: Option<u32>,
    steer_tx: &mpsc::UnboundedSender<SteerMsg>,
    warm: &Arc<std::sync::Mutex<warm_pool::WarmChild>>,
    child: &mut tokio::process::Child,
) {
    warm_pool::remove_if(session_id, warm);
    if let Some(p) = turn_pid { clear_session_pid_if(session_id, p); }
    clear_steer_tx_if(session_id, steer_tx);
    let _ = child.start_kill();
    let _ = child.wait().await;
}

/// How a single turn ended, as seen by the reader loop.
/// Internal sentinel passed through the turn's `done` channel when a reused turn
/// found a dead warm child. `dispatch_turn` recognises it and retries the turn
/// cold instead of surfacing it to the UI. Never user-visible.
const RETRY_COLD_SENTINEL: &str = "__rift_retry_cold__";

enum TurnOutcome {
    /// A `result` frame landed — DONE was emitted; the warm child can be reused.
    Result,
    /// stdout closed (None) before any `result` — the child exited. The caller
    /// reaps it + remaps the exit/stderr into an actionable error/session-lost.
    Eof,
    /// A fatal mid-turn condition (broken stdin while writing a control response
    /// or steer, or a stdout read error) — the child is unusable. The caller
    /// surfaces `msg` + drops the child.
    Fatal(String),
    /// A REUSED turn's first envelope write failed before any output — the warm
    /// child died while parked (process killed/crashed between turns). Not a
    /// turn failure: the loop drops the dead child and the dispatcher retries the
    /// turn cold (fresh spawn + `--resume`) so the user never sees an error.
    DeadOnReuse,
    /// The child produced ZERO stdout for the no-progress ceiling — wedged, not
    /// slow. A real prefill/queue wait yields its first frame in seconds; a quiet
    /// pipe past the ceiling means the CLI is stuck (hung init handshake, a CLI
    /// MCP server stuck connecting, a dead-but-not-EOF'd pipe). The child is
    /// unusable → surface an honest error + drop it. Never the Anthropic API.
    Stalled,
}

/// Borrowed per-turn context for `stream_one_turn` — keeps the long-lived
/// owned state (`stdin`, `lines`, `steer_rx`) borrowed mutably while the
/// per-turn values are passed by ref. One arg-bundle struct so the fn stays
/// under clippy's arg limit.
struct StreamCtx<'a> {
    stdin: &'a mut tokio::process::ChildStdin,
    lines: &'a mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    steer_rx: &'a mut mpsc::UnboundedReceiver<SteerMsg>,
    app_out: &'a AppHandle,
    win_label: &'a str,
    stream_sid: &'a str,
    /// Model + effort for THIS process (baked at spawn; per-process constants).
    /// Threaded only so the perf record can be grouped by model/effort (WS5).
    model: &'a str,
    effort: &'a str,
    user_line: &'a [u8],
    /// True once the per-process `initialize` handshake has been acked. The
    /// FIRST turn waits for the first `control_response` (the init ack) before
    /// sending its user envelope; every reused turn writes the envelope up front
    /// (the handshake already happened at process start).
    handshake_done: &'a mut bool,
    /// M3: set true if this turn launched a `run_in_background` Bash — the
    /// detached grandchild inherits the warm stdout write-end and never EOFs, so
    /// the warm child is tainted and must be evicted after `result` (the next
    /// turn would otherwise read interleaved junk from the bg process).
    bg_evict: &'a std::sync::atomic::AtomicBool,
    turn_start: std::time::Instant,
}

/// No-progress watchdog ceiling: max time the reader will park on `next_line()`
/// with the child producing NOTHING — no init ack, no token, no frame — before
/// declaring it wedged (`TurnOutcome::Stalled`). The deadline is RESET on every
/// stdout line, so it only bites a truly silent pipe; a streaming turn (frames
/// arriving) never trips it, and a legitimate permission wait runs inside the
/// read arm (its own 120s timeout governs) so the watchdog isn't even polled.
/// 180s is far above a real prefill/queue first-frame (<60s) yet bounds the
/// previously-infinite deadlock the UI used to mislabel as "the Anthropic API".
const STREAM_NO_PROGRESS_SECS: u64 = 180;

/// Consecutive dead-silent watchdog fires tolerated with a tool in flight before
/// declaring a stall — bounds the re-arm so a never-completing tool can't disable
/// the net forever (2 ≈ 6min). Any received line resets the counter.
const STREAM_TOOL_GRACE_WINDOWS: u32 = 2;

/// Stream ONE turn: write the user envelope, forward NDJSON, handle the control
/// channel (init ack, `can_use_tool` permission asks) + mid-turn steers, and
/// return when `result` lands / stdout EOFs / a fatal write fails. Ported
/// verbatim from the old inline reader task — the only structural change is that
/// it runs per-turn against borrowed long-lived stdin/stdout instead of owning
/// them, and on `result` it RETURNS (the loop parks) instead of dropping stdin.
async fn stream_one_turn(ctx: StreamCtx<'_>) -> TurnOutcome {
    use tokio::io::AsyncWriteExt;

    let StreamCtx {
        stdin, lines, steer_rx, app_out, win_label, stream_sid, model, effort, user_line,
        handshake_done, bg_evict, turn_start,
    } = ctx;

    // A reused turn entered with the handshake already done. If such a turn sees
    // stdout EOF before any `result`, the warm child died while parked (a write
    // to a dead pipe doesn't fail synchronously on Windows — the death only
    // surfaces as a read EOF). That's silently retryable: drop the dead child +
    // respawn cold. A FIRST turn EOF is a real spawn failure → surface it.
    let was_reused = *handshake_done;
    // Reused turns: the handshake already happened at process start, so write
    // the user envelope immediately. First turn: wait for the init ack below.
    let mut user_sent = if *handshake_done {
        // Reused turn: a write failure here means the warm child died while
        // parked. No output has been produced, so this is silently retryable —
        // signal the loop to drop the child and let the dispatcher respawn cold.
        if stdin.write_all(user_line).await.is_err() {
            return TurnOutcome::DeadOnReuse;
        }
        // Flush failure here = the warm child's pipe broke (died while parked);
        // no output produced yet, so it's retryable — drop + respawn cold. (#31)
        if stdin.flush().await.is_err() {
            return TurnOutcome::DeadOnReuse;
        }
        true
    } else {
        false
    };

    let mut first_line_logged = false;
    let mut first_think_logged = false;
    let mut first_text_logged = false;
    // WS6 latency attribution: turn-start → first ANY frame, and total tool time
    // accrued BEFORE first text. `tool_open_at` marks the most recent tool_use so
    // its result can add the elapsed gap to `pre_text_tool_ms`.
    let mut perf_first_line_ms: Option<u64> = None;
    let mut perf_pre_text_tool_ms: u64 = 0;
    let mut tool_open_at: Option<std::time::Instant> = None;
    // Steers that arrive before the (first-turn) handshake completes are
    // buffered, then flushed the instant the user turn is sent.
    let mut steer_pending: Vec<SteerMsg> = Vec::new();

    // B2: per-turn perf accumulators — filled at the existing TTFT log sites +
    // the result frame, finalised into a TurnPerf record before the result
    // return. Wall-clock start (for cost-by-day bucketing); elapsed milestones
    // use the existing `turn_start` Instant.
    let ts_start_ms: u64 = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    };
    let mut perf_ttft_thinking_ms: Option<u64> = None;
    let mut perf_ttft_text_ms: Option<u64> = None;

    // Tools in flight: a `tool_use` block (assistant frame) opens one, a
    // `tool_result` (user frame) closes it. A running tool legitimately produces
    // no stdout for a long time — a Bash build, a deep grep, or an `ask_user`
    // parked on the human — so the watchdog must NOT fire while count > 0. Mirrors
    // the frontend's `liveTool != null` rule for the stall indicator.
    let mut tools_in_flight: i32 = 0;
    // Consecutive watchdog fires survived purely because a tool was in flight.
    // Reset to 0 by ANY received line (real progress); when it exceeds the grace
    // cap with the pipe still dead, the "tool" is wedged → stall anyway.
    let mut tool_grace_fires: u32 = 0;

    // No-progress watchdog: a deadline that any received line pushes forward.
    // Parked on `next_line()` with a silent pipe → fires once at the ceiling and
    // ends the turn as Stalled (vs the old infinite hang). Pinned so the select
    // can re-poll the same future; reset by re-arming `deadline` after each line.
    let watchdog = tokio::time::sleep(std::time::Duration::from_secs(STREAM_NO_PROGRESS_SECS));
    tokio::pin!(watchdog);

    loop {
        tokio::select! {
        () = &mut watchdog => {
            // In-flight tool → silent-but-healthy, re-arm; but bound the grace so a
            // never-completing tool can't disable the net forever (see read arm: a
            // line resets tool_grace_fires).
            if tools_in_flight > 0 && tool_grace_fires < STREAM_TOOL_GRACE_WINDOWS {
                tool_grace_fires += 1;
                watchdog.as_mut().reset(
                    tokio::time::Instant::now() + std::time::Duration::from_secs(STREAM_NO_PROGRESS_SECS),
                );
                continue;
            }
            log::warn!(
                "warm_pool: no stdout for {STREAM_NO_PROGRESS_SECS}s (user_sent={user_sent}, tools_in_flight={tools_in_flight}, grace={tool_grace_fires}) — child wedged, session={stream_sid}"
            );
            return TurnOutcome::Stalled;
        }
        read = lines.next_line() => {
            // Any line (even an ignored control frame) = the child is alive and
            // making progress — push the no-progress deadline forward and clear
            // the tool-grace counter (a working tool just proved itself).
            watchdog.as_mut().reset(
                tokio::time::Instant::now() + std::time::Duration::from_secs(STREAM_NO_PROGRESS_SECS),
            );
            tool_grace_fires = 0;
        match read {
            Ok(Some(line)) => {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                    let ty = v.get("type").and_then(|x| x.as_str());
                    // First `control_response` on a first turn = the init ack →
                    // fire the user turn once. Don't forward it to the UI.
                    if !user_sent && ty == Some("control_response") {
                        user_sent = true;
                        *handshake_done = true;
                        if let Err(e) = stdin.write_all(user_line).await {
                            return TurnOutcome::Fatal(format!("write user turn: {e}"));
                        }
                        if let Err(e) = stdin.flush().await {
                            return TurnOutcome::Fatal(format!("flush user turn: {e}"));
                        }
                        for m in steer_pending.drain(..) {
                            match build_user_envelope(&m.text, &m.attachments) {
                                Ok(env) => {
                                    if let Err(e) = stdin.write_all(&env).await {
                                        return TurnOutcome::Fatal(format!("write steer: {e}"));
                                    }
                                }
                                Err(e) => {
                                    let _ = app_out.emit_to(win_label, ERROR_EVENT, serde_json::json!({
                                        "session_id": stream_sid, "message": e,
                                    }));
                                }
                            }
                        }
                        if let Err(e) = stdin.flush().await {
                            return TurnOutcome::Fatal(format!("flush steer: {e}"));
                        }
                        continue;
                    }
                    // A `control_response` on a REUSED turn (handshake already
                    // done) is the CLI re-acking — ignore, don't forward.
                    if *handshake_done && !user_sent && ty == Some("control_response") {
                        continue;
                    }
                    // Permission ask → resolve via the registry + UI, write the
                    // decision back as a `control_response`.
                    let is_perm = ty == Some("control_request")
                        && v.get("request").and_then(|r| r.get("subtype")).and_then(|s| s.as_str())
                            == Some("can_use_tool");
                    if is_perm {
                        if let Err(e) = handle_permission_request(app_out, stream_sid, win_label, stdin, &v).await {
                            return TurnOutcome::Fatal(format!("write permission response: {e}"));
                        }
                        // A permission ask parks on the USER (its own 120s
                        // timeout). That human time isn't a stalled child — re-arm
                        // the watchdog so a slow decision doesn't trip it on the
                        // next park; the CLI's first post-decision frame is what
                        // the fresh deadline now waits on.
                        watchdog.as_mut().reset(
                            tokio::time::Instant::now() + std::time::Duration::from_secs(STREAM_NO_PROGRESS_SECS),
                        );
                        continue;
                    }
                    // `result` is the last frame — forward it, emit DONE, and
                    // RETURN (the loop parks for the next turn; stdin stays open).
                    if ty == Some("result") {
                        let res_is_err = v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false)
                            || v.get("subtype").and_then(|s| s.as_str()).map(|s| s != "success").unwrap_or(false);
                        let res_text = v.get("result").and_then(|s| s.as_str()).unwrap_or("");
                        if res_is_err && is_auth_rejection(res_text) {
                            let _ = app_out.emit_to(win_label, ERROR_EVENT, serde_json::json!({
                                "session_id": stream_sid, "message": auth_rejection_message(),
                            }));
                        }
                        // B2: harvest the result frame's token / cache / cost data
                        // (the only frame that carries them) and persist a typed
                        // perf record. Fire-and-forget — never gates the DONE emit.
                        record_turn_perf(
                            &v, ts_start_ms, turn_start, perf_ttft_thinking_ms,
                            perf_ttft_text_ms, stream_sid, model, effort,
                            perf_first_line_ms, perf_pre_text_tool_ms, !was_reused,
                        );
                        let _ = app_out.emit_to(win_label, STREAM_EVENT, serde_json::json!({
                            "session_id": stream_sid, "line": trimmed,
                        }));
                        let _ = app_out.emit_to(win_label, DONE_EVENT, serde_json::json!({
                            "session_id": stream_sid, "exit_code": 0,
                            // B: a turn that backgrounded a Bash task ends with no way
                            // to auto-report its result (headless -p kills the shell ~5s
                            // after the turn). Flag it so the FE can warn the user once.
                            "bg_task": bg_evict.load(std::sync::atomic::Ordering::Acquire),
                        }));
                        return TurnOutcome::Result;
                    }
                    // M3: detect a `run_in_background` Bash tool_use so we evict
                    // this warm child after the turn (its detached grandchild
                    // inherits the stdout pipe → would taint the next turn).
                    if ty == Some("assistant") {
                        if let Some(content) = v.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
                            for block in content {
                                let block_ty = block.get("type").and_then(|t| t.as_str());
                                // A tool starts → suspend the watchdog (a long Bash
                                // build / grep / ask_user is silent-but-healthy).
                                if block_ty == Some("tool_use") {
                                    tools_in_flight += 1;
                                    // WS6: clock the tool open so its result adds the
                                    // gap to pre-text tool time (only matters pre-text).
                                    if !first_text_logged && tool_open_at.is_none() {
                                        tool_open_at = Some(std::time::Instant::now());
                                    }
                                }
                                if block_ty == Some("tool_use")
                                    && block.get("name").and_then(|n| n.as_str()) == Some("Bash")
                                    && block.get("input").and_then(|i| i.get("run_in_background")).and_then(|b| b.as_bool()) == Some(true)
                                {
                                    bg_evict.store(true, std::sync::atomic::Ordering::Release);
                                    log::info!("warm_pool: run_in_background Bash detected — will evict warm child after turn, session={stream_sid}");
                                }
                            }
                        }
                    }
                    // A tool finished → its result rides a `user` frame. Close out
                    // the in-flight count so the watchdog re-engages for the next
                    // quiet-pipe window. Saturating at 0 (never negative) in case a
                    // result arrives without a matching tracked tool_use.
                    if ty == Some("user") {
                        if let Some(content) = v.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
                            for block in content {
                                if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                                    tools_in_flight = (tools_in_flight - 1).max(0);
                                    // WS6: a tool finished before any text → bank the
                                    // round-trip into pre-text tool time. Only the
                                    // outermost open (the one we clocked) closes here.
                                    if !first_text_logged && tools_in_flight == 0 {
                                        if let Some(at) = tool_open_at.take() {
                                            perf_pre_text_tool_ms =
                                                perf_pre_text_tool_ms.saturating_add(at.elapsed().as_millis() as u64);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // #244 phase probe: first thinking vs text delta.
                    if ty == Some("stream_event") {
                        if let Some(ev) = v.get("event") {
                            let ev_ty = ev.get("type").and_then(|x| x.as_str());
                            let blk_ty = ev.get("content_block").and_then(|b| b.get("type")).and_then(|x| x.as_str());
                            let delta_ty = ev.get("delta").and_then(|d| d.get("type")).and_then(|x| x.as_str());
                            let is_think = blk_ty == Some("thinking")
                                || matches!(delta_ty, Some("thinking_delta") | Some("signature_delta"));
                            let is_text = blk_ty == Some("text") || delta_ty == Some("text_delta");
                            if is_think && !first_think_logged {
                                first_think_logged = true;
                                perf_ttft_thinking_ms = Some(turn_start.elapsed().as_millis() as u64);
                                log::info!("warm_pool: first-thinking {} ms session={}", turn_start.elapsed().as_millis(), stream_sid);
                            }
                            if is_text && !first_text_logged {
                                first_text_logged = true;
                                perf_ttft_text_ms = Some(turn_start.elapsed().as_millis() as u64);
                                log::info!("warm_pool: first-text {} ms ev={:?} session={}", turn_start.elapsed().as_millis(), ev_ty, stream_sid);
                            }
                        }
                    }
                }
                if !first_line_logged {
                    first_line_logged = true;
                    perf_first_line_ms = Some(turn_start.elapsed().as_millis() as u64);
                    log::info!("warm_pool: TTFT {} ms (turn-start→first-line) session={}", turn_start.elapsed().as_millis(), stream_sid);
                }
                let _ = app_out.emit_to(win_label, STREAM_EVENT,
                    serde_json::json!({ "session_id": stream_sid, "line": trimmed }));
            }
            Ok(None) => {
                // Reused turn, stdout closed with no result → child died while
                // parked. Retry cold instead of erroring the user's turn.
                return if was_reused { TurnOutcome::DeadOnReuse } else { TurnOutcome::Eof };
            }
            Err(e) => {
                let _ = app_out.emit_to(win_label, ERROR_EVENT, serde_json::json!({
                    "session_id": stream_sid, "message": format!("stdout read error: {e}"),
                }));
                return TurnOutcome::Fatal(format!("stdout read error: {e}"));
            }
        }
        }
        // Mid-turn steer: write the injected user message to the live stdin.
        Some(msg) = steer_rx.recv() => {
            if !user_sent {
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
                            let _ = app_out.emit_to(win_label, ERROR_EVENT, serde_json::json!({
                                "session_id": stream_sid, "message": format!("write steer: {e}"),
                            }));
                            return TurnOutcome::Fatal(format!("write steer: {e}"));
                        }
                        if let Err(e) = stdin.flush().await {
                            let _ = app_out.emit_to(win_label, ERROR_EVENT, serde_json::json!({
                                "session_id": stream_sid, "message": format!("flush steer: {e}"),
                            }));
                            return TurnOutcome::Fatal(format!("flush steer: {e}"));
                        }
                    }
                    Err(e) => {
                        let _ = app_out.emit_to(win_label, ERROR_EVENT, serde_json::json!({
                            "session_id": stream_sid, "message": e,
                        }));
                    }
                }
            }
        }
        }
    }
}

/// Remap a turn that ended WITHOUT a `result` (EOF) into the right frontend
/// event: session-lost (resume miss → auto-recover), stop-done (user stopped),
/// or an actionable error. Mirrors the old post-wait failure branch. `done`
/// signals `cold_spawn_and_run`'s awaiter; we send `Ok(())` for the recoverable
/// session-lost/stop cases (the UI already got its event) and `Err` otherwise.
async fn emit_turn_end_error(
    app: &AppHandle,
    window_label: &str,
    session_id: &str,
    is_first_turn: bool,
    status: Option<std::process::ExitStatus>,
    stderr_buf: &str,
    done: oneshot::Sender<Result<(), String>>,
) {
    // User clicked Stop → emit done (not error) so the UI clears + pops queue.
    if take_session_stopped(session_id) {
        let _ = app.emit_to(window_label, DONE_EVENT, serde_json::json!({
            "session_id": session_id,
            "exit_code": status.and_then(|s| s.code()).unwrap_or(-1),
        }));
        let _ = done.send(Ok(()));
        return;
    }
    // --resume index miss → session-lost so the frontend re-sends as first turn.
    if !is_first_turn && stderr_buf.contains("No conversation found with session ID:") {
        log::warn!("warm_pool: --resume {session_id} failed (no conversation) — emitting session-lost");
        let _ = app.emit_to(window_label, SESSION_LOST_EVENT, serde_json::json!({
            "session_id": session_id,
        }));
        let _ = done.send(Ok(()));
        return;
    }
    let raw = stderr_buf.trim();
    let msg = if raw.is_empty() {
        match assistant_auth_probe().await {
            Ok(s) if !s.cli_present => "Claude Code CLI not found on this machine — install it from claude.com/code (or add an API key in Settings), then try again.".to_string(),
            Ok(s) if !s.logged_in && !s.api_key_configured => "Claude CLI is installed but not logged in on this machine — open a terminal, run `claude`, and sign in (or add an API key in Settings), then try again.".to_string(),
            _ => format!(
                "claude exited with {} (no error output) — run `claude` in a terminal to confirm it works, then retry.",
                status.and_then(|s| s.code()).map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            ),
        }
    } else if is_auth_rejection(raw) {
        auth_rejection_message()
    } else {
        format!(
            "claude exited with {} — {}",
            status.and_then(|s| s.code()).map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            raw
        )
    };
    let _ = app.emit_to(window_label, ERROR_EVENT, serde_json::json!({
        "session_id": session_id, "message": msg.clone(),
    }));
    let _ = done.send(Err(msg));
}

/// B2: build a `TurnPerf` from the result frame + accumulated TTFT milestones,
/// persist it to `turns.ndjson` (fire-and-forget), and mirror it onto the diag
/// bus as a structured event. The CLI carries token/cache/cost data ONLY on the
/// `result` frame — `v` here is that parsed frame. Best-effort: a malformed or
/// usage-less frame just yields `None` fields, never an error.
#[allow(clippy::too_many_arguments)]
fn record_turn_perf(
    v: &Value,
    ts_start_ms: u64,
    turn_start: std::time::Instant,
    ttft_thinking_ms: Option<u64>,
    ttft_text_ms: Option<u64>,
    stream_sid: &str,
    model: &str,
    effort: &str,
    first_line_ms: Option<u64>,
    pre_text_tool_ms: u64,
    was_cold: bool,
) {
    use crate::diagnostics::{self, perf::TurnPerf, DiagLevel, DiagStage};

    let usage = v.get("usage");
    let input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|x| x.as_u64());
    let output_tokens = usage.and_then(|u| u.get("output_tokens")).and_then(|x| x.as_u64());
    let cache_read_tokens =
        usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|x| x.as_u64());
    let cache_create_tokens =
        usage.and_then(|u| u.get("cache_creation_input_tokens")).and_then(|x| x.as_u64());
    let cost_usd = v.get("total_cost_usd").and_then(|x| x.as_f64());
    let result_subtype = v.get("subtype").and_then(|s| s.as_str()).map(|s| s.to_owned());
    // CLI self-reported timing (server-side truth, independent of Rift): `ttft_ms`
    // = turn-start → first model token (the API's real latency), `duration_api_ms`
    // = total wall-time in API calls. Lets the Health pane attribute the model's
    // share vs Rift's. Best-effort — absent on older CLIs or error frames.
    let cli_ttft_ms = v.get("ttft_ms").and_then(|x| x.as_u64());
    let cli_api_ms = v.get("duration_api_ms").and_then(|x| x.as_u64());

    let cache_hit_rate = match (cache_read_tokens, input_tokens) {
        (Some(r), Some(i)) if r + i > 0 => Some(r as f64 / (r + i) as f64),
        _ => None,
    };

    // WS6: attribute the first-reply wait to its dominant phase so the advisor
    // names the lever instead of inferring it. Pure fn in `perf`, unit-tested.
    let pre_text_tool = if pre_text_tool_ms > 0 { Some(pre_text_tool_ms) } else { None };
    let dominant_cause = crate::diagnostics::perf::classify_latency_cause(
        ttft_text_ms, ttft_thinking_ms, first_line_ms, pre_text_tool, was_cold, cache_hit_rate,
    );

    let rec = TurnPerf {
        ts_start_ms,
        ttft_thinking_ms,
        ttft_text_ms,
        duration_ms: Some(turn_start.elapsed().as_millis() as u64),
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_create_tokens,
        cost_usd,
        cache_hit_rate,
        session_id: stream_sid.to_owned(),
        result_subtype,
        model: Some(model.to_owned()),
        effort: Some(effort.to_owned()),
        ttft_first_line_ms: first_line_ms,
        pre_text_tool_ms: pre_text_tool,
        was_cold: Some(was_cold),
        dominant_cause,
        cli_ttft_ms,
        cli_api_ms,
    };

    // Structured bus event — DiagStage::Log so it rides the normal 200/s cap,
    // not the System critical-bypass. Useful for the live diagnostics pane.
    let fields = serde_json::to_value(&rec).unwrap_or(Value::Null);
    diagnostics::emit_with_fields(
        DiagStage::Log,
        DiagLevel::Info,
        Some("assistant"),
        Some("turn.rs"),
        "turn-perf",
        fields,
    );
    diagnostics::perf::append_turn_perf(rec);

    // Latency attribution at a glance: the CLI's own API time vs Rift's wall-clock.
    // `overhead = duration - cli_api` is everything NOT spent in the model API
    // (Rift IPC, tool execution, stdin/stdout plumbing). A small overhead next to
    // a large cli_api proves the turn's cost is the model, not Rift.
    if let Some(api) = cli_api_ms {
        let dur = turn_start.elapsed().as_millis() as i64;
        let overhead = dur - api as i64;
        log::info!(
            "turn-attrib: cli_api={api}ms cli_ttft={}ms rift_wall={dur}ms non_api_overhead={overhead}ms session={stream_sid}",
            cli_ttft_ms.map(|v| v as i64).unwrap_or(-1),
        );
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
            "Authentication failed (401). Rift is using the Claude CLI ({}) — sign in by running `claude login` in a terminal, or switch installs in Settings → CLI session, then retry.",
            resolve_claude_exe()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "your active install".into())
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
pub async fn assistant_stop(
    session_id: String,
    ask_user: tauri::State<'_, std::sync::Arc<crate::assistant::AskUserRegistry>>,
) -> Result<(), String> {
    if !is_valid_session_id(&session_id) {
        return Err(format!("invalid session_id: must be a UUID (got {} chars)", session_id.len()));
    }
    mark_session_stopped(&session_id);
    // Unblock any parked ask_user for this session FIRST — independent of the
    // PID kill. A warm child blocked in the bridge on an ask_user oneshot can't
    // be reached by `taskkill` if its PID was already cleared (eviction /
    // prior-turn cleanup), so the kill alone would leave the bridge parked for
    // the full 600s timeout and the UI spinner stuck. Cancelling here drops the
    // sender → the bridge waiter resolves Err immediately → MCP child unblocks.
    let cancelled = ask_user.cancel_all_for_session(&session_id);
    if cancelled > 0 {
        log::info!("assistant_stop: cancelled {cancelled} pending ask_user for {session_id}");
    }
    let Some(pid) = get_session_pid(&session_id) else {
        return Ok(());
    };
    // RR9: compare-and-clear on the PID we observed (mirrors the turn loop at
    // its two cleanup points). A queued follow-up can call assistant_send →
    // set_session_pid with a NEW pid between our get_session_pid read and here;
    // an unconditional clear would wipe the new turn's pid, leaving it
    // un-stoppable. clear_session_pid_if only removes when the stored pid still
    // matches ours.
    clear_session_pid_if(&session_id, pid);

    // RR11: taskkill/kill .status() blocks until the child exits; under AV/process
    // contention this can stall a Tokio worker for seconds (and several concurrent
    // Stops could starve the pool). Run the blocking wait off-worker.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let out = tokio::task::spawn_blocking(move || {
            std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        })
        .await
        .map_err(|e| format!("taskkill join: {e}"))?;
        match out {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("taskkill exited {}", s.code().unwrap_or(-1))),
            Err(e) => Err(format!("spawn taskkill: {e}")),
        }
    }
    #[cfg(unix)]
    {
        // Avoid a libc dependency just for SIGTERM; shell out to `kill`.
        let out = tokio::task::spawn_blocking(move || {
            std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        })
        .await
        .map_err(|e| format!("kill join: {e}"))?;
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
pub async fn assistant_steer(
    session_id: String,
    text: String,
    attachments: Option<Vec<AssistantAttachment>>,
) -> Result<String, String> {
    if !is_valid_session_id(&session_id) {
        return Err(format!(
            "invalid session_id: must be a UUID (got {} chars)",
            session_id.len()
        ));
    }
    let trimmed = text.trim();
    let attachments = attachments.unwrap_or_default();
    // A steer with no text but with images is valid (#49: an image-only steer).
    if trimmed.is_empty() && attachments.is_empty() {
        return Err("empty steer text".into());
    }
    validate_attachments(&attachments)?;
    // RR7: cap a renderer-supplied steer message before it enters the unbounded
    // mpsc channel and gets serialized + written to the CLI child's stdin. A
    // multi-megabyte payload would otherwise allocate unbounded heap and force a
    // huge synchronous write on the stdin task. 1 MiB is far above any real steer.
    if trimmed.len() > 1_048_576 {
        return Err("steer text too large (max 1 MiB)".into());
    }
    let Some(tx) = get_steer_tx(&session_id) else {
        return Ok("no_active_turn".into());
    };
    match tx.send(SteerMsg { text: trimmed.to_string(), attachments }) {
        Ok(()) => Ok("steered".into()),
        // Receiver dropped between lookup and send → turn just ended.
        Err(_) => Ok("no_active_turn".into()),
    }
}
