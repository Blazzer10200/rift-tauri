//! Assistant — JetBrains-style AI partner page (beta).
//!
//! Architecture (locked 2026-05-14, see `docs/design/assistant-page.md`):
//! Rift's Rust side spawns the user's installed `claude` CLI in headless
//! streaming-JSON mode, parses NDJSON line-by-line, re-emits each event to the
//! frontend via Tauri events. The Agent SDK npm package is Node-only and cannot
//! run in the Tauri webview, so we drive the same CLI it would wrap.
//!
//! Auth model: piggyback on the user's existing `claude login` session by
//! default (CLI reads its own keychain). API-key fallback: when configured,
//! spawn with `--bare` + `ANTHROPIC_API_KEY` env so the CLI ignores OAuth.

pub mod ask_user;
pub mod git_local;
pub mod mcp_server;
pub mod permission;
pub mod remote_bridge;

pub use ask_user::AskUserRegistry;
pub use permission::PermissionRegistry;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

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

fn get_session_pid(session_id: &str) -> Option<u32> {
    with_session_pids(|m| m.get(session_id).copied()).flatten()
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

/// Cached absolute path to the user's `claude` CLI. Windows' `Command::new`
/// does NOT apply PATHEXT lookup (no auto-append of `.cmd`/`.exe`), so we
/// resolve via `where.exe claude` (or `which` on Unix) and reuse the
/// absolute path for all spawn sites. Outer `Option` = is-cached;
/// inner = path-or-not.
///
/// #64: previously `OnceLock<Option<PathBuf>>` — cached forever per process.
/// An upgrade or reinstall of the CLI required a full Rift restart. The
/// fast path now stats the cached file; a missing-file triggers a fresh
/// re-resolution, so CLI installs/moves take effect on the next spawn.
static CLAUDE_EXE: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

fn resolve_claude_exe_uncached() -> Option<PathBuf> {
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("where.exe", &["claude"])
    } else {
        ("which", &["claude"])
    };
    let mut cmd = std::process::Command::new(program);
    cmd.args(args).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    // `where.exe` prints one path per line. On Windows we MUST prefer
    // `.exe` over `.cmd`/`.bat` — Rust 1.77+ refuses to safely escape
    // newlines/special chars for batch-file invocation (CVE-2024-24576
    // mitigation, fails as "batch file arguments are invalid"). The
    // multi-line `--append-system-prompt` payload + Human:/Assistant:
    // history chain both contain newlines, so a `.cmd` shim is
    // unusable here. Native Claude Code installs `.exe` directly.
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if cfg!(windows) {
        // 1) .exe on PATH wins.
        if let Some(p) = lines.iter().find(|l| l.to_ascii_lowercase().ends_with(".exe")) {
            return Some(PathBuf::from(*p));
        }
        // 2) Native installer drops claude.exe at known LOCALAPPDATA
        // locations but doesn't always wire it into PATH (the npm
        // shim claims `where.exe claude` first). Probe directly.
        if let Some(lad) = std::env::var_os("LOCALAPPDATA") {
            let candidates = [
                PathBuf::from(&lad).join("AnthropicClaude").join("claude.exe"),
                PathBuf::from(&lad).join("Programs").join("AnthropicClaude").join("claude.exe"),
            ];
            for c in candidates {
                if c.is_file() {
                    return Some(c);
                }
            }
        }
        // 2b) npm-installed claude bundles the real claude.exe inside
        // its node_modules dir. The shim on PATH is `claude.cmd`
        // which forwards via `%*` — and cmd.exe arg forwarding
        // silently mangles `--output-format stream-json` so the CLI
        // downgrades to plain text output. Calling the bundled .exe
        // directly avoids the shim entirely.
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let bundled = PathBuf::from(&appdata)
                .join("npm")
                .join("node_modules")
                .join("@anthropic-ai")
                .join("claude-code")
                .join("bin")
                .join("claude.exe");
            if bundled.is_file() {
                return Some(bundled);
            }
        }
        // 3) Fall back to .cmd/.bat. assistant_send keeps spawn args
        // newline-free (system addendum is single-line, prompt goes
        // via stdin) so the Rust 1.77 batch-args validator accepts it.
        lines
            .iter()
            .find(|l| l.to_ascii_lowercase().ends_with(".cmd"))
            .or_else(|| lines.first())
            .map(|s| PathBuf::from(*s))
    } else {
        lines.first().map(|s| PathBuf::from(*s))
    }
}

fn resolve_claude_exe() -> Option<PathBuf> {
    // Fast path: return cached value if the file still exists. is_file()
    // catches the "CLI uninstalled or moved" case without forcing a full
    // re-resolution every call.
    {
        let g = match CLAUDE_EXE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(cached) = g.as_ref() {
            match cached {
                Some(p) if p.is_file() => return Some(p.clone()),
                None => return None, // cached "no CLI on PATH"
                _ => {} // cached path is stale → re-resolve below
            }
        }
    }
    // Slow path: re-resolve, then cache.
    let resolved = resolve_claude_exe_uncached();
    if let Ok(mut g) = CLAUDE_EXE.lock() {
        *g = Some(resolved.clone());
    }
    resolved
}

/// Build a `tokio::process::Command` for `claude`, hiding the console window
/// on Windows. Returns `None` if the CLI isn't on PATH. `pub(crate)` so the
/// `stt::cleanup` hop can reuse the same resolution + windowing path.
pub(crate) fn claude_command() -> Option<Command> {
    let exe = resolve_claude_exe()?;
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    // #63 follow-up: kill the CLI child if the spawning tokio task is
    // dropped before `wait()` returns (panic mid-turn, IPC handle teardown,
    // app shutdown). Without this the child outlives the spawn task and the
    // PID-tracker-based `assistant_stop` is the only kill path — which itself
    // depends on `set_session_pid` having completed.
    cmd.kill_on_drop(true);
    Some(cmd)
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
const ENHANCE_STREAM_EVENT: &str = "assistant://enhance-stream";

/// Output of `claude auth status` plus our locally-stored API-key flag.
/// All fields camelCase to match the CLI's JSON shape verbatim.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub cli_present: bool,
    pub cli_version: Option<String>,
    pub logged_in: bool,
    pub auth_method: Option<String>,
    pub api_provider: Option<String>,
    pub email: Option<String>,
    pub subscription_type: Option<String>,
    pub api_key_configured: bool,
    /// Pill color: "green" | "yellow" | "red".
    pub pill: String,
    /// One-line user-facing status.
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AssistantConfig {
    /// Legacy plaintext slot — Phase 6 (#37) moved the API key to the OS
    /// keychain. Still parsed (so old on-disk configs can be migrated) but
    /// never written: `skip_serializing_if` drops it once cleared, and
    /// `load_config()` runs a one-shot migration on read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
    /// Currently-open project folder for the Assistant. None = no folder open;
    /// Assistant falls back to AutoSync's server folders if any, else no-tools.
    /// Matches VS Code's "open folder" model — one root at a time.
    #[serde(default)]
    current_root: Option<PathBuf>,
    /// Last ~10 folders the user opened. Newest first. Surfaced in EmptyState
    /// so they can jump back. AutoSync folders are NOT mirrored here; they're
    /// a separate source the picker shows as a "Synced servers" group.
    #[serde(default)]
    recent_roots: Vec<PathBuf>,
    /// When true (the default), spawn the CLI without `--strict-mcp-config`
    /// and `--disable-slash-commands` so user MCP servers + slash commands
    /// layer alongside Rift's. CLAUDE.md / hooks always load via the CLI's
    /// own resolution; the `Skill` tool is explicitly added to the
    /// `--allowed-tools` allowlist so `/handoff`, `/check`, `/plan`, etc.
    /// can invoke. No opt-out short of `--bare`, which fires automatically
    /// in API-key mode.
    /// `None` = default (true). Switch off for a sandboxed Assistant.
    #[serde(default)]
    use_full_config: Option<bool>,
    /// Hard dollar cap per turn, passed as `--max-budget-usd <amount>`. The
    /// CLI exits non-zero if exceeded — we surface the failure as a chat
    /// notice. `None` or `<= 0` = no cap.
    #[serde(default)]
    max_budget_usd: Option<f64>,
    /// Gate for the `mcp__rift__remote_bash` tool. Off by default; flipping on
    /// exposes a single remote-shell tool to the model, scoped to the active
    /// AutoSync engine's russh session and workspace-locked against concurrent
    /// users. `None` = default (false).
    #[serde(default)]
    allow_remote_shell: Option<bool>,
    /// Effort tier for extended thinking on non-Haiku models. Mirrors Claude
    /// Code's own effort ladder. `"none"` skips extended thinking entirely
    /// (fastest TTFT); `"quick"` ~2K thinking tokens (default — balanced);
    /// `"deep"` 10K tokens (heavy reasoning, slowest). Haiku ignores this.
    /// Per-turn override rides the `assistant_send` arg; this is the default.
    #[serde(default)]
    thinking_effort: Option<String>,
    /// Permission mode passed to the CLI's `--permission-mode`. One of
    /// `default` / `acceptEdits` / `plan` / `auto` / `bypassPermissions`.
    /// `None` resolves to `bypassPermissions` (Rift's historical behavior).
    /// Per-turn override rides the `assistant_send` arg; this is the default.
    #[serde(default)]
    permission_mode: Option<String>,
    /// Assistant trust level gating the local git tools. One of `readonly` /
    /// `standard` / `full`. `None` derives from `allow_remote_shell` (true →
    /// full, else → readonly) so upgrades don't silently grant git-write.
    #[serde(default)]
    trust_level: Option<String>,
    /// Auto-compact threshold as fraction of context window (0.0-1.0). `None` =
    /// disabled (manual only). User has `DISABLE_AUTO_COMPACT=1` set globally
    /// so default to None — opt-in, not opt-out. See `docs/design/assistant-compaction.md`.
    #[serde(default)]
    auto_compact_threshold: Option<f32>,
    /// Model alias for the one-shot summarize call. `None` = "haiku" (cheap +
    /// fast; sufficient for prose summarization w/ explicit preservation prompt).
    /// $0.91 at 900K vs $2.73 on sonnet.
    #[serde(default)]
    compact_model: Option<String>,
}

const RECENT_ROOTS_MAX: usize = 10;

fn config_path() -> Result<PathBuf, String> {
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir ~/.rift/assistant: {e}"))?;
    Ok(dir.join("config.json"))
}

/// Directory holding one `<uuid>.json` per saved conversation.
fn conversations_dir() -> Result<PathBuf, String> {
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant").join("conversations");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir conversations: {e}"))?;
    Ok(dir)
}

/// Conversation metadata returned by `assistant_list_conversations`. Kept
/// thin so listing 100s of convos doesn't load every transcript into memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMeta {
    pub id: String,
    pub title: String,
    pub model: String,
    pub message_count: u32,
    pub created_at: i64,
    pub updated_at: i64,
    /// Phase E5: flattened compaction summaries so HistoryDrawer search can
    /// match against the contents of long-running compacted convos without
    /// loading every transcript. Empty for convos that never compacted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compaction_summaries: Vec<String>,
}

/// Full conversation record persisted to disk. `messages` is the frontend's
/// `ChatMessage[]` shape — we don't reshape it server-side, just JSON
/// round-trip. `serde_json::Value` lets the schema evolve without touching
/// Rust types every time the frontend adds a block kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub messages: serde_json::Value,
    /// CLI session UUID (--session-id / --resume target). Decoupled from `id`
    /// in S103 so compaction can mint a fresh CLI session without breaking
    /// tab persistence. Legacy convos without this field deserialize cleanly
    /// (Option default = None); frontend falls back to `id` on load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_session_id: Option<String>,
}

fn convo_path(id: &str) -> Result<PathBuf, String> {
    // Guard against `..` / path separators in the id — only accept the
    // hex/uuid shape we generate (alphanumeric + dashes). #132: also cap
    // length so a hostile caller can't smuggle a path bomb past the charset
    // filter via a 10kB id.
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!("invalid conversation id: {id}"));
    }
    Ok(conversations_dir()?.join(format!("{id}.json")))
}

/// Sidecar holding the workspace cwd that was active when a session was first
/// created. The claude CLI stores its transcript JSONL under
/// `~/.claude/projects/<cwd-hash>/<uuid>.jsonl`, and `--resume <uuid>` only
/// searches the CURRENT cwd's hash dir — it does NOT fall back across dirs
/// (anthropics/claude-code#35226). So if the user's active workspace changes
/// between turns (folder swap, autosync engine flip, root vanishes), the
/// resume target moves and the session goes silently stale ("session lost"
/// → frontend pops messages → all history dropped). Pinning cwd per session
/// keeps every turn aimed at the same JSONL.
fn session_cwd_path(id: &str) -> Result<PathBuf, String> {
    // #132: length cap mirrors convo_path; UUIDs are 36 chars, give some
    // slack for future ID shape evolution but stop pathological inputs.
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!("invalid session id: {id}"));
    }
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant").join("sessions");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir sessions: {e}"))?;
    Ok(dir.join(format!("{id}.cwd")))
}

/// #220: canonical UUID shape check — 36 chars, 8-4-4-4-12 hex w/ hyphens at
/// fixed positions. Accepts uppercase + lowercase hex (Claude CLI is
/// case-insensitive). Rejects path-traversal segments, leading dashes,
/// whitespace, and anything else that could escape an argv slot or filename.
fn is_valid_session_id(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    for (i, b) in s.as_bytes().iter().enumerate() {
        match i {
            8 | 13 | 18 | 23 => {
                if *b != b'-' {
                    return false;
                }
            }
            _ => {
                if !b.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    true
}

/// #221: reject any model value that would be parsed as a flag by the CLI.
/// Allowlist `[A-Za-z0-9._-]+` w/ NO leading dash. Covers short aliases
/// (`sonnet`/`opus`/`haiku`) and full ids (`claude-opus-4-7`).
fn is_valid_model_name(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') {
        return false;
    }
    s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn save_session_cwd(id: &str, cwd: &Path) {
    if let Ok(p) = session_cwd_path(id) {
        let s = cwd.to_string_lossy();
        if let Err(e) = std::fs::write(&p, s.as_bytes()) {
            log::warn!("assistant: save session cwd {}: {e}", p.display());
        }
    }
}

fn load_session_cwd(id: &str) -> Option<PathBuf> {
    let p = session_cwd_path(id).ok()?;
    let s = std::fs::read_to_string(&p).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn delete_session_cwd(id: &str) {
    if let Ok(p) = session_cwd_path(id) {
        let _ = std::fs::remove_file(&p);
    }
}

/// Lexical common ancestor of a set of paths. Returns `None` if the paths
/// share nothing beyond filesystem root, if the result has no parent (drive
/// or fs root), or if the result is not a directory on disk.
///
/// Motivation: when AutoSync watches a FiveM server, each resource directory
/// (e.g. `[voice]/`, `[ox]/`, `qbx_core/`) becomes its own FolderWatch with
/// its own `local_root`. `roots[0]` ends up at whichever sorts first — for
/// FiveM that's an `[bracket]` resource (`[` = 0x5B, before letters) — and
/// the Assistant's cwd lands inside that single resource rather than at
/// `resources/` where every resource is visible. Substituting the common
/// ancestor in as `roots[0]` fixes the "workspace is just `[voice]`" gripe.
fn common_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut iter = paths.iter();
    let first = iter.next()?;
    let mut common: Vec<std::path::Component> = first.components().collect();
    for p in iter {
        let other: Vec<_> = p.components().collect();
        let new_len = common
            .iter()
            .zip(other.iter())
            .take_while(|(a, b)| a == b)
            .count();
        common.truncate(new_len);
        if common.is_empty() {
            return None;
        }
    }
    let mut result = PathBuf::new();
    for c in &common {
        result.push(c.as_os_str());
    }
    if result.as_os_str().is_empty() || result.parent().is_none() {
        return None;
    }
    if !result.is_dir() {
        // #133: silent None fell through to caller's `roots[0]` fallback. A
        // brand-new resource folder that doesn't yet exist on disk would
        // narrow cwd unexpectedly. Surface so DiagBus shows the regression.
        log::warn!(
            "common_ancestor: '{}' is not a directory; caller will fall back to roots[0]",
            result.display()
        );
        return None;
    }
    Some(result)
}

#[tauri::command]
pub fn assistant_list_conversations() -> Result<Vec<ConversationMeta>, String> {
    let dir = conversations_dir()?;
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(out),
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = match std::fs::read(&p) {
            Ok(b) => b,
            Err(_) => continue,
        };
        // Parse to a Value first so we can extract optional fields not modeled
        // on the typed Conversation struct (compactionHistory[*].summary —
        // shipped E5, ridden through serde_json::Value catch-all on save).
        let raw: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let convo: Conversation = match serde_json::from_value(raw.clone()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let message_count = convo
            .messages
            .as_array()
            .map(|a| a.len() as u32)
            .unwrap_or(0);
        let compaction_summaries = raw
            .get("compactionHistory")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("summary").and_then(|s| s.as_str()).map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        out.push(ConversationMeta {
            id: convo.id,
            title: convo.title,
            model: convo.model,
            message_count,
            created_at: convo.created_at,
            updated_at: convo.updated_at,
            compaction_summaries,
        });
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
pub fn assistant_load_conversation(id: String) -> Result<Conversation, String> {
    let p = convo_path(&id)?;
    let bytes = std::fs::read(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse conversation: {e}"))
}

#[tauri::command]
pub fn assistant_save_conversation(convo: Conversation) -> Result<(), String> {
    let p = convo_path(&convo.id)?;
    let s = serde_json::to_string(&convo).map_err(|e| e.to_string())?;
    // Atomic-ish write: write to .tmp then rename so a crash mid-write
    // doesn't leave a half-truncated transcript on disk.
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, s).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("rename {}: {e}", p.display()))?;
    Ok(())
}

#[tauri::command]
pub fn assistant_delete_conversation(id: String) -> Result<(), String> {
    let p = convo_path(&id)?;
    // #114: load BEFORE delete so we can find the decoupled `cli_session_id`
    // (post-S103 these can differ from the Rift convo id after a compaction).
    // Without this, the cwd sidecar under the cli session UUID never gets
    // cleaned up — orphan accumulation under `~/.rift/assistant/sessions/`.
    let cli_session_id = std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str::<Conversation>(&s).ok())
        .and_then(|c| c.cli_session_id);
    match std::fs::remove_file(&p) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("delete {}: {e}", p.display())),
    }
    // Delete sidecars only after the convo record is gone — a half-deleted
    // convo with intact sidecars is recoverable; a deleted sidecar with a
    // surviving convo would silently lose its pinned cwd.
    delete_session_cwd(&id);
    if let Some(cli_id) = cli_session_id {
        if cli_id != id {
            delete_session_cwd(&cli_id);
        }
    }
    Ok(())
}

fn dirs_home() -> Result<PathBuf, String> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| "no USERPROFILE/HOME env var".to_string())
}

fn load_config() -> AssistantConfig {
    let mut cfg: AssistantConfig = config_path()
        .and_then(|p| std::fs::read_to_string(&p).map_err(|e| e.to_string()))
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        .unwrap_or_default();
    // Phase 6 (#37): one-shot migration of any plaintext api_key into the
    // OS keychain. Failure is non-fatal — the field stays in JSON for a
    // future attempt, and runtime reads still see it via legacy fallback in
    // current_api_key().
    if let Some(k) = cfg.api_key.as_deref().filter(|s| !s.is_empty()) {
        match crate::secrets::set(crate::secrets::ASSISTANT_API_KEY, k) {
            Ok(()) => {
                cfg.api_key = None;
                if let Err(e) = save_config(&cfg) {
                    log::warn!("assistant: post-migration save_config failed: {e}");
                } else {
                    log::info!("assistant: migrated api_key to keychain");
                }
            }
            Err(e) => log::warn!("assistant: keychain migration for api_key failed: {e}"),
        }
    }
    cfg
}

/// Phase 6 (#37): the live API key. Reads the keychain first; falls back to
/// any (un-migrated) plaintext value still in `config.json`. Returns None
/// when both are empty/absent.
fn current_api_key() -> Option<String> {
    crate::secrets::get(crate::secrets::ASSISTANT_API_KEY)
        .or_else(|| load_config().api_key.filter(|s| !s.is_empty()))
}

fn save_config(cfg: &AssistantConfig) -> Result<(), String> {
    let p = config_path()?;
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    // #65: tmp+rename to match assistant_save_conversation. Two Tauri-command
    // setters racing on read-modify-write (e.g. set_api_key + set_max_budget
    // back-to-back) previously produced a torn or empty config.json under a
    // direct std::fs::write — the second writer truncated mid-flight.
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, s).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("rename {}: {e}", p.display()))?;
    Ok(())
}

/// RAII guard that deletes the per-session MCP config file when dropped.
/// Ensures cleanup on normal exit, early-return errors, and panics alike.
struct McpConfigGuard(PathBuf);

impl Drop for McpConfigGuard {
    fn drop(&mut self) {
        if self.0.exists() {
            if let Err(e) = std::fs::remove_file(&self.0) {
                log::warn!("assistant: failed to remove per-session mcp-config on drop: {e}");
            } else {
                log::debug!("assistant: removed per-session mcp-config {:?}", self.0);
            }
        }
    }
}

/// Write the Rift MCP server config the CLI will read via `--mcp-config`.
/// Points at our own `current_exe()` with `RIFT_MCP_SERVER=1`; the binary
/// branches to `mcp_server::run_stdio` instead of launching Tauri. Workspace
/// roots are passed via `RIFT_MCP_ROOTS` (newline-separated) so the spawned
/// child knows the path-safety boundary at request time.
///
/// S73: when `bridge` is `Some` AND `remote_shell_enabled` is true, the MCP
/// child also gets `RIFT_BRIDGE_PORT` + `RIFT_BRIDGE_TOKEN` so its
/// `remote_bash` tool can dial the parent Tauri's loopback bridge. The bridge
/// itself reuses the AutoSync engine's live russh session for the exec.
///
/// `session_id` is appended to the filename so concurrent `assistant_send`
/// calls (multi-tab) each get their own file — no cross-tab cred leak.
fn write_mcp_config(
    session_id: &str,
    roots: &[PathBuf],
    bridge: Option<&remote_bridge::BridgeInfo>,
    remote_shell_enabled: bool,
    trust_level: &str,
) -> Result<PathBuf, String> {
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir ~/.rift/assistant: {e}"))?;
    // Per-session filename prevents concurrent assistant_send calls (multi-tab)
    // from racing over a single shared file. sanitize: replace path-unsafe
    // chars so the session UUID is a valid filename component on all OSes.
    let safe_id = session_id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
    let path = dir.join(format!("mcp-config-{safe_id}.json"));

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let roots_joined = roots
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let mut env_map = serde_json::Map::new();
    env_map.insert("RIFT_MCP_SERVER".into(), Value::from("1"));
    env_map.insert("RIFT_MCP_ROOTS".into(), Value::from(roots_joined));
    // Plumb the convo's session_id so the `ask_user` MCP tool can tag its
    // bridge request — the frontend pairs incoming `assistant://ask-user`
    // events against the correct chat tab by session_id.
    env_map.insert("RIFT_SESSION_ID".into(), Value::from(session_id.to_string()));
    // Trust level gates the local git tools in the MCP child. Always injected —
    // git is a local op, no bridge needed. See `mcp_server::trust_level`.
    env_map.insert("RIFT_TRUST_LEVEL".into(), Value::from(trust_level.to_string()));
    // #62: always pass the read-only token so sync_status / shell_lock_status
    // are available; only inject the write-scoped RIFT_BRIDGE_TOKEN when the
    // user has explicitly opted into remote-shell. A compromised MCP tool with
    // only the readonly token can't call `remote_bash` even if it tries —
    // the bridge server-side dispatch (remote_bridge::dispatch) rejects it.
    if let Some(b) = bridge {
        env_map.insert("RIFT_BRIDGE_PORT".into(), Value::from(b.port.to_string()));
        env_map.insert(
            "RIFT_BRIDGE_READONLY_TOKEN".into(),
            Value::from(b.readonly_token.clone()),
        );
        if remote_shell_enabled {
            env_map.insert("RIFT_BRIDGE_TOKEN".into(), Value::from(b.token.clone()));
            env_map.insert("RIFT_REMOTE_SHELL_ENABLED".into(), Value::from("1"));
        }
    }

    let payload = serde_json::json!({
        "mcpServers": {
            "rift": {
                "command": exe.to_string_lossy(),
                "args": [],
                "env": Value::Object(env_map),
            }
        }
    });
    let s = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&path, s).map_err(|e| format!("write mcp-config: {e}"))?;

    // #9.2 + #38: tighten permissions so the on-disk bridge token isn't world-
    // readable in the interval between write + delete-on-exit.
    // Unix: explicit 0600 on the file.
    // Windows: explicit DACL via `icacls` — strip inheritance and grant the
    // current user Full Control only. NTFS inheritance from `%USERPROFILE%\.rift\`
    // is user-only on a standalone profile but NOT on domain-joined / shared
    // setups where inheritance can grant SYSTEM/Administrators read.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let user = std::env::var("USERNAME").unwrap_or_default();
        if !user.is_empty() {
            // /inheritance:r — strip inherited ACEs; /grant:r — replace user grant.
            // Output discarded; failure is non-fatal (file is still delete-on-exit
            // and the embedded token rotates each app launch).
            let _ = std::process::Command::new("icacls")
                .arg(&path)
                .args(["/inheritance:r", "/grant:r", &format!("{user}:(F)")])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }

    Ok(path)
}

/// #9.2: best-effort removal of all per-session MCP config files on app exit
/// so bridge tokens don't sit on disk between sessions. Tokens become stale
/// the moment the process exits (new ones generated next run), but leaked
/// stale tokens are still strictly more information than non-existent files.
/// Globs `mcp-config-*.json` to catch any files left behind by crashed or
/// cancelled sessions, plus the legacy fixed `mcp-config.json` name.
/// Errors are logged + swallowed — cleanup failure must not block app shutdown.
pub fn cleanup_mcp_config_on_exit() {
    let Ok(home) = dirs_home() else { return };
    let dir = home.join(".rift").join("assistant");

    // Legacy fixed-path (pre-per-session fix) — remove if present.
    let legacy = dir.join("mcp-config.json");
    if legacy.exists() {
        match std::fs::remove_file(&legacy) {
            Ok(()) => log::info!("assistant: removed legacy mcp-config.json on exit"),
            Err(e) => log::warn!("assistant: failed to remove legacy mcp-config.json on exit: {e}"),
        }
    }

    // Per-session files: glob mcp-config-*.json.
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        if name.starts_with("mcp-config-") && name.ends_with(".json") {
            match std::fs::remove_file(entry.path()) {
                Ok(()) => log::info!("assistant: removed stale {} on exit", name),
                Err(e) => log::warn!("assistant: failed to remove {} on exit: {e}", name),
            }
        }
    }
}

/// Phase E4: housekeeping sweep for CLI JSONLs that belong to sessions
/// retired by compaction. After a compact, the old `<uuid>.jsonl` under
/// `~/.claude/projects/<cwd-hash>/` is dead weight — Rift's own convo JSON
/// keeps the user-facing history; the CLI JSONL is never read again.
///
/// Approach: scan every Rift convo, collect `compactionHistory[*].priorSessionId`,
/// then walk `~/.claude/projects/*/<uuid>.jsonl`. Delete a file iff:
///   1. Its filename stem matches a known retired session id, AND
///   2. Its mtime is older than 30 days (conservative — gives the user a
///      long window to manually `claude --resume <old>` if compaction
///      surprised them; e.g. rollback debugging).
///
/// Errors are logged + swallowed. Best-effort — startup must not block on
/// disk hiccups. Returns the number of files deleted (for the log line).
pub fn cleanup_retired_jsonls() -> usize {
    use std::collections::HashSet;
    let Ok(home) = dirs_home() else { return 0 };

    // Step 1: enumerate retired session ids across all convo JSONs.
    let convo_dir = home.join(".rift").join("assistant").join("conversations");
    let Ok(entries) = std::fs::read_dir(&convo_dir) else { return 0 };
    let mut retired: HashSet<String> = HashSet::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&p) else { continue };
        let Ok(raw) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };
        let Some(arr) = raw.get("compactionHistory").and_then(|v| v.as_array()) else {
            continue;
        };
        for entry in arr {
            if let Some(sid) = entry.get("priorSessionId").and_then(|s| s.as_str()) {
                if is_valid_session_id(sid) {
                    retired.insert(sid.to_string());
                }
            }
        }
    }
    if retired.is_empty() {
        return 0;
    }

    // Step 2: walk ~/.claude/projects/<cwd-hash>/*.jsonl and delete matches.
    let projects = home.join(".claude").join("projects");
    let Ok(project_dirs) = std::fs::read_dir(&projects) else { return 0 };
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(30 * 24 * 60 * 60));
    let mut deleted = 0usize;
    for cwd_dir in project_dirs.flatten() {
        let path = cwd_dir.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&path) else { continue };
        for f in files.flatten() {
            let fp = f.path();
            if fp.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }
            let Some(stem) = fp.file_stem().and_then(|s| s.to_str()) else { continue };
            if !retired.contains(stem) {
                continue;
            }
            // mtime guard — only delete files older than 30 days.
            let aged = f
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .zip(cutoff)
                .map(|(mt, c)| mt < c)
                .unwrap_or(false);
            if !aged {
                continue;
            }
            match std::fs::remove_file(&fp) {
                Ok(()) => {
                    deleted += 1;
                    log::info!("assistant: cleaned retired JSONL {}", fp.display());
                }
                Err(e) => log::warn!("assistant: failed to remove {}: {e}", fp.display()),
            }
        }
    }
    deleted
}

/// CLI auth-status JSON shape from `claude auth status`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliAuthStatus {
    logged_in: Option<bool>,
    auth_method: Option<String>,
    api_provider: Option<String>,
    email: Option<String>,
    subscription_type: Option<String>,
}

#[tauri::command]
pub async fn assistant_auth_probe() -> Result<AuthStatus, String> {
    let mut out = AuthStatus::default();
    let _cfg = load_config(); // run keychain migration / surface any stale legacy field
    out.api_key_configured = current_api_key().is_some();

    // #134: spawn `claude --version` and `claude auth status` in parallel
    // instead of back-to-back. The prior sequential layout opened a small
    // TOCTOU window where the CLI on PATH could be swapped between the two
    // resolutions (uninstall mid-probe, antivirus quarantine, etc.) and the
    // returned `cli_version` would disagree with the auth result. `claude_command()`
    // re-stats the cached path on each call, so a CLI removal still surfaces —
    // it just happens to both child spawns at once now.
    let ver_fut = async {
        match claude_command() {
            Some(mut c) => c.arg("--version").stdout(Stdio::piped()).stderr(Stdio::null()).output().await.ok(),
            None => None,
        }
    };
    let auth_fut = async {
        match claude_command() {
            Some(mut c) => c.args(["auth", "status"]).stdout(Stdio::piped()).stderr(Stdio::null()).output().await.ok(),
            None => None,
        }
    };
    let (ver, auth_opt) = tokio::join!(ver_fut, auth_fut);

    match ver {
        Some(o) if o.status.success() => {
            out.cli_present = true;
            out.cli_version = Some(String::from_utf8_lossy(&o.stdout).trim().to_string());
        }
        _ => {
            out.cli_present = false;
            out.pill = if out.api_key_configured { "yellow".into() } else { "red".into() };
            out.summary = if out.api_key_configured {
                "API key configured — install Claude Code CLI for piggyback session".into()
            } else {
                "Not configured — install Claude Code CLI or add an API key in Settings".into()
            };
            return Ok(out);
        }
    }

    // `claude auth status` — JSON when stdout isn't a TTY (which it isn't from spawn).
    if let Some(auth) = auth_opt {
        if auth.status.success() {
            let text = String::from_utf8_lossy(&auth.stdout);
            if let Ok(parsed) = serde_json::from_str::<CliAuthStatus>(text.trim()) {
                out.logged_in = parsed.logged_in.unwrap_or(false);
                out.auth_method = parsed.auth_method;
                out.api_provider = parsed.api_provider;
                out.email = parsed.email;
                out.subscription_type = parsed.subscription_type;
            }
        }
    }

    // Priority: explicit API key shadows the OAuth session (env-var precedence).
    if out.api_key_configured {
        out.pill = "yellow".into();
        out.summary = "Using API key".into();
    } else if out.logged_in {
        out.pill = "green".into();
        let who = out.email.as_deref().unwrap_or("Claude account");
        let sub = out.subscription_type.as_deref().unwrap_or("");
        out.summary = if sub.is_empty() {
            format!("Using Claude Code session ({who})")
        } else {
            format!("Using Claude Code session ({who} · {sub})")
        };
    } else {
        out.pill = "red".into();
        out.summary = "Claude CLI found but not logged in — run `claude login` or add an API key".into();
    }
    Ok(out)
}

/// Phase 6 (#37): renderer must never see the secret value — only whether
/// one is configured. Replaces the legacy `assistant_get_api_key` cmd which
/// returned the plaintext value to JS.
#[tauri::command]
pub fn assistant_get_api_key_present() -> Result<bool, String> {
    Ok(current_api_key().is_some())
}

#[tauri::command]
pub fn assistant_get_use_full_config() -> Result<bool, String> {
    Ok(load_config().use_full_config.unwrap_or(true))
}

#[tauri::command]
pub fn assistant_set_use_full_config(value: bool) -> Result<(), String> {
    let mut cfg = load_config();
    cfg.use_full_config = Some(value);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_max_budget_usd() -> Result<Option<f64>, String> {
    Ok(load_config().max_budget_usd.filter(|v| v.is_finite() && *v > 0.0))
}

#[tauri::command]
pub fn assistant_set_max_budget_usd(value: Option<f64>) -> Result<(), String> {
    let mut cfg = load_config();
    cfg.max_budget_usd = value.filter(|v| v.is_finite() && *v > 0.0);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_thinking_effort() -> Result<String, String> {
    Ok(load_config()
        .thinking_effort
        .filter(|v| matches!(v.as_str(), "none" | "quick" | "deep" | "ultra"))
        .unwrap_or_else(|| "quick".to_string()))
}

#[tauri::command]
pub fn assistant_set_thinking_effort(value: String) -> Result<(), String> {
    if !matches!(value.as_str(), "none" | "quick" | "deep" | "ultra") {
        return Err(format!("invalid thinking_effort: {value}"));
    }
    let mut cfg = load_config();
    cfg.thinking_effort = Some(value);
    save_config(&cfg)
}

/// The CLI's `--permission-mode` values Rift exposes. `dontAsk` (the CLI's
/// auto-DENY mode) is intentionally excluded — there's no Rift surface to
/// approve, so it would silently block everything (see the S92 note below).
fn is_valid_permission_mode(v: &str) -> bool {
    matches!(v, "default" | "acceptEdits" | "plan" | "auto" | "bypassPermissions")
}

#[tauri::command]
pub fn assistant_get_permission_mode() -> Result<String, String> {
    Ok(load_config()
        .permission_mode
        .filter(|v| is_valid_permission_mode(v))
        .unwrap_or_else(|| "bypassPermissions".to_string()))
}

#[tauri::command]
pub fn assistant_set_permission_mode(value: String) -> Result<(), String> {
    if !is_valid_permission_mode(&value) {
        return Err(format!("invalid permission_mode: {value}"));
    }
    let mut cfg = load_config();
    cfg.permission_mode = Some(value);
    save_config(&cfg)
}

/// The Assistant trust levels Rift exposes. Gates the local git tools in the
/// MCP server (`git_local.rs`): `readonly` → status/diff/log; `standard` →
/// adds pull/commit/push; `full` → reserved for RCON raw passthrough (phase 2).
fn is_valid_trust_level(v: &str) -> bool {
    matches!(v, "readonly" | "standard" | "full")
}

/// Resolve the effective trust level. Explicit setting wins; when unset, derive
/// from the legacy `allow_remote_shell` toggle so upgrades don't silently grant
/// git-write: remote-shell on → `full`, off → `readonly`.
fn effective_trust_level(trust_level: &Option<String>, allow_remote_shell: Option<bool>) -> String {
    trust_level
        .clone()
        .filter(|v| is_valid_trust_level(v))
        .unwrap_or_else(|| {
            if allow_remote_shell == Some(true) { "full".into() } else { "readonly".into() }
        })
}

#[tauri::command]
pub fn assistant_get_trust_level() -> Result<String, String> {
    let cfg = load_config();
    Ok(effective_trust_level(&cfg.trust_level, cfg.allow_remote_shell))
}

#[tauri::command]
pub fn assistant_set_trust_level(value: String) -> Result<(), String> {
    if !is_valid_trust_level(&value) {
        return Err(format!("invalid trust_level: {value}"));
    }
    let mut cfg = load_config();
    cfg.trust_level = Some(value);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_auto_compact_threshold() -> Result<Option<f32>, String> {
    Ok(load_config()
        .auto_compact_threshold
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0))
}

#[tauri::command]
pub fn assistant_set_auto_compact_threshold(value: Option<f32>) -> Result<(), String> {
    let mut cfg = load_config();
    cfg.auto_compact_threshold = value.filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_compact_model() -> Result<String, String> {
    Ok(load_config()
        .compact_model
        .filter(|v| matches!(v.as_str(), "haiku" | "sonnet" | "opus"))
        .unwrap_or_else(|| "haiku".to_string()))
}

#[tauri::command]
pub fn assistant_set_compact_model(value: String) -> Result<(), String> {
    if !matches!(value.as_str(), "haiku" | "sonnet" | "opus") {
        return Err(format!("invalid compact_model: {value}"));
    }
    let mut cfg = load_config();
    cfg.compact_model = Some(value);
    save_config(&cfg)
}

/// Output of a one-shot summarize call. Mirrors the design doc Phase B
/// shape — caller uses `summary` as the seed for the next CLI session
/// after a compaction remint, and surfaces the cost/token figures in the
/// boundary message pill.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummarizeResult {
    pub summary: String,
    pub model: String,
    pub cost_usd: f64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_create_tokens: u32,
}

/// Meta-prompt for the composer's "enhance prompt" wand. Deliberately
/// conservative: clarify + structure the user's rough draft, but never invent
/// scope. Over-enhancement (ballooning a one-line ask into a spec) is the
/// failure mode we're guarding against — a coding prompt that grows phantom
/// requirements is worse than the rough original.
const ENHANCE_META_PROMPT: &str = "You rewrite a developer's rough draft into a clear, actionable instruction for \
Claude Code — an agentic coding assistant that reads files, runs commands, and edits code directly. \
Expand the draft so Claude Code can act on it precisely and completely.\n\
\n\
Rules:\n\
- Lead with the concrete goal in direct imperative voice (Add…, Fix…, Refactor…, Investigate…).\n\
- Preserve EVERY technical specific verbatim: file names, paths, identifiers, versions, numbers, commands, error text.\n\
- Make the draft's implicit intent explicit AND add the concrete detail a developer would obviously expect — the specific mechanism, key bindings, states/edge cases to handle, and a brief acceptance check — so the instruction is directly executable. Even a clear one-liner should gain useful, actionable specifics; a bare punctuation/grammar pass is NOT enough.\n\
- Stay inside the draft's intent: flesh out HOW to accomplish exactly what was asked. Do NOT bolt on unrelated features, files, or scope the draft never implied — added detail must serve the stated goal.\n\
- For a bug, keep the stated symptom and any stated cause; you may point to likely places to look, but do not assert a fix or diagnosis the draft didn't state.\n\
- If the result has multiple parts, lay them out as a short bullet list; otherwise a tight paragraph. Add substance, not padding or restatement — no filler, no closing summary.\n\
- If the draft is not a coding task, just make it clear, direct, and complete — do not force a coding frame onto it.\n\
\n\
Output ONLY the rewritten prompt — no preamble, no explanation, no markdown code fences, no surrounding quotes.";

/// One-shot prompt enhancer for the composer wand. Spawns `claude -p` headless
/// on Haiku (fast + cheap), feeds the meta-prompt + the user's draft, and
/// returns the rewritten text. No session, no resume, no tools, no hooks — the
/// simplest possible call. The frontend shows the result as an editable
/// preview; this command never mutates conversation state.
#[tauri::command]
pub async fn assistant_enhance_prompt(
    app: AppHandle,
    request_id: String,
    prompt: String,
) -> Result<String, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("nothing to enhance".into());
    }
    // 8K-char guard bounds cost + arg length — a normal prompt is <1K chars;
    // this catches accidental full-file pastes before they fire a model call.
    if trimmed.chars().count() > 8000 {
        return Err("prompt too long to enhance (8000 character cap)".into());
    }
    let mut cmd = claude_command()
        .ok_or_else(|| "claude CLI not on PATH — install Claude Code or configure an API key".to_string())?;
    // Fence the draft as inert content + an explicit "do not answer it"
    // directive. Without this, a draft phrased like a message to an assistant
    // (e.g. "you're going to have to step out of your zone") gets mistaken for
    // a conversational turn — the model replies to it instead of rewriting it.
    let user_msg = format!(
        "Rewrite the rough prompt draft delimited by <draft></draft> below into a clear, well-structured prompt. \
         Do NOT answer the draft, do NOT respond to it conversationally, do NOT address me — treat everything inside \
         the tags purely as text to improve. Output ONLY the rewritten prompt.\n\n<draft>\n{trimmed}\n</draft>"
    );
    cmd.arg("-p").arg(&user_msg)
        // Meta-prompt rides the system prompt (stable across calls), so the
        // user draft is the only varying part — lets the server-side prompt
        // cache hit on repeat enhances within its ~5min TTL.
        .arg("--append-system-prompt").arg(ENHANCE_META_PROMPT)
        // Drop per-machine sections (cwd/env/git/memory) from the system
        // prompt — irrelevant to a rewrite, and keeps the cached prefix stable.
        .arg("--exclude-dynamic-system-prompt-sections")
        // Stream the rewrite token-by-token so the UI shows text within ~1-2s
        // (TTFT) instead of blocking on the full completion. `--verbose` is
        // required by stream-json; `--include-partial-messages` emits the
        // per-token `content_block_delta` frames we forward to the frontend.
        //
        // CRITICAL: the draft MUST ride text input (`-p <arg>`, null stdin) for
        // this to actually stream. Measured 2026-05-27: `--input-format
        // stream-json` (the SDK control path) makes the bundled claude.exe
        // block-buffer its whole stdout and dump it in one burst at exit — no
        // streaming. Plain text `-p` input uses the simple completion path that
        // flushes incrementally. (`--tools ""` is fine; only input mode matters.)
        .arg("--output-format").arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--model").arg("haiku")
        // Tight cap — a sub-1K-token rewrite on Haiku costs fractions of a cent.
        .arg("--max-budget-usd").arg("0.20")
        // Zero MCP servers, zero slash commands, zero tools — pure completion.
        .arg("--strict-mcp-config")
        .arg("--disable-slash-commands")
        .arg("--tools").arg("")
        .arg("--permission-mode").arg("bypassPermissions")
        // One-shot: never persist this throwaway rewrite to the session store.
        .arg("--no-session-persistence")
        // Neutral cwd so the CLI loads no project CLAUDE.md / .claude settings.
        .current_dir(std::env::temp_dir())
        // Latency killers: skip SessionStart hooks (~46K-token memory load),
        // the autoupdater check, and telemetry/error-reporting — all add
        // blocking startup time to a one-shot spawn.
        .env("CLAUDE_DISABLE_HOOKS", "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn `claude` (enhance): {e}"))?;
    let stdout = child.stdout.take().ok_or("enhancer stdout unavailable")?;
    let stderr = child.stderr.take().ok_or("enhancer stderr unavailable")?;

    // Drain stderr concurrently so a chatty CLI can't deadlock on a full pipe
    // while we read stdout. Bounded — the enhancer's stderr is tiny.
    let stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            buf.push_str(&l);
            buf.push('\n');
            if buf.len() > 8192 {
                break;
            }
        }
        buf
    });

    // Read NDJSON stdout, forward each `text_delta` to the UI as it lands, and
    // accumulate the full rewrite as the authoritative return value.
    let mut acc = String::new();
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let ty = v.get("type").and_then(|t| t.as_str());
        // `result` is the terminal frame — stop reading once it lands.
        if ty == Some("result") {
            break;
        }
        if ty != Some("stream_event") {
            continue;
        }
        let Some(ev) = v.get("event") else { continue };
        if ev.get("type").and_then(|t| t.as_str()) != Some("content_block_delta") {
            continue;
        }
        let delta = ev.get("delta");
        let is_text = delta.and_then(|d| d.get("type")).and_then(|t| t.as_str()) == Some("text_delta");
        if !is_text {
            continue;
        }
        if let Some(txt) = delta.and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
            if txt.is_empty() {
                continue;
            }
            acc.push_str(txt);
            let _ = app.emit(
                ENHANCE_STREAM_EVENT,
                serde_json::json!({ "request_id": request_id, "delta": txt }),
            );
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("await claude (enhance): {e}"))?;
    let stderr_buf = stderr_task.await.unwrap_or_default();
    if !status.success() {
        let msg = stderr_buf.trim();
        return Err(if msg.is_empty() {
            "enhancer exited with an error".to_string()
        } else {
            format!("enhance failed: {msg}")
        });
    }
    let text = acc.trim().to_string();
    if text.is_empty() {
        return Err("enhancer returned empty output".into());
    }
    // Terminal marker so the frontend can settle the reveal even though the
    // command's resolved return value is the canonical text.
    let _ = app.emit(
        ENHANCE_STREAM_EVENT,
        serde_json::json!({ "request_id": request_id, "done": true }),
    );
    Ok(text)
}

const SUMMARIZE_PROMPT_HEAD: &str = "The user is approaching their context window cap. Produce a structured summary of this conversation that another instance of you could read in under 2K tokens and pick up where we left off without losing critical state. Preserve verbatim: (1) the active TodoWrite list below, (2) file paths actively being worked on + the last revision direction for each, (3) decisions explicitly made by the user, (4) open questions or blockers. Drop: tool-call mechanics, exploratory dead-ends, verbose tool outputs. Output format: 4 sections — \"Active task\", \"Files in play\", \"Decisions\", \"Open questions\". No preamble or sign-off.";

/// Phase B: one-shot summarize against an existing CLI session. Spawns
/// `claude -p --resume <sid> --model <m>` headless, pipes a structured
/// summarize prompt, parses the NDJSON stream for assistant text deltas +
/// the terminal `result` envelope. No state mutation, no UI events — the
/// caller decides what to do with the returned summary (Phase C wires it
/// into the compaction remint flow).
///
/// `tasks_json` is the frontend's current TodoWrite snapshot serialized as
/// a JSON string (e.g. `[{"content":"...","status":"in_progress"}, ...]`);
/// pass `"[]"` or `"(none)"` when empty. Interpolated server-side so the
/// frontend doesn't have to know the prompt template.
#[tauri::command]
pub async fn assistant_summarize_session(
    app: AppHandle,
    session_id: String,
    focus: Option<String>,
    tasks_json: Option<String>,
) -> Result<SummarizeResult, String> {
    let cfg = load_config();
    let model = cfg
        .compact_model
        .filter(|v| matches!(v.as_str(), "haiku" | "sonnet" | "opus"))
        .unwrap_or_else(|| "haiku".to_string());

    let focus_line = focus
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("Focus: {s}."))
        .unwrap_or_else(|| "Focus: general continuation.".into());
    let tasks_body = tasks_json
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "[]")
        .unwrap_or("(none)")
        .to_string();
    let prompt = format!(
        "{SUMMARIZE_PROMPT_HEAD}\n\n{focus_line}\n\nActive TodoWrite tasks (preserve verbatim under \"Active task\"):\n{tasks_body}\n"
    );

    let mut cmd = claude_command()
        .ok_or_else(|| "claude CLI not on PATH — install Claude Code or configure an API key".to_string())?;
    // S124 fix: `--resume <sid>` resolves against the CLI's project-hash dir
    // derived from cwd. Without setting current_dir to match the cwd the
    // original conversation ran under (persisted via the .cwd sidecar at
    // session-id mint time), the CLI looks in the wrong hash dir and errors
    // "No conversation found with session ID".
    if let Some(cwd) = load_session_cwd(&session_id).filter(|p| p.is_dir()) {
        cmd.current_dir(cwd);
    }
    cmd.arg("-p").arg(&prompt)
        .arg("--resume").arg(&session_id)
        .arg("--output-format").arg("stream-json")
        .arg("--input-format").arg("text")
        .arg("--verbose")
        .arg("--model").arg(&model)
        // Hard cost cap — Haiku at full 900K context is ~$0.91; 1.50 leaves
        // ~60% headroom for tokenizer drift. Sonnet runs above this should
        // be flagged before they fire.
        .arg("--max-budget-usd").arg("1.50")
        // Headless mode has no interactive surface for ANY tool — and a
        // summarize call shouldn't be running tools regardless. The CLI's
        // `--tools ""` disables the built-in tool set wholesale.
        .arg("--tools").arg("")
        .arg("--permission-mode").arg("bypassPermissions")
        // SessionStart hooks load ~46K tokens of memory/git context into
        // cache_creation per fresh CLI process — irrelevant for a one-shot
        // summarize and burns ~5% of the budget per call. Verified S103
        // probe 2026-05-19 ($0.0586 empty-resume baseline cost).
        .env("CLAUDE_DISABLE_HOOKS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("spawn `claude` (summarize): {e}"))?;

    let stdout = child.stdout.take().ok_or_else(|| "claude stdout missing".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "claude stderr missing".to_string())?;

    let progress_sid = session_id.clone();
    let progress_app = app.clone();
    let stdout_task = tokio::spawn(async move {
        let mut summary = String::new();
        // Rate-limit progress emits: at most every 150ms or every 64 new chars,
        // whichever comes first. Avoids flooding the frontend on dense streams.
        let mut last_emit_at = std::time::Instant::now();
        let mut last_emit_len: usize = 0;
        let mut cost_usd: f64 = 0.0;
        let mut input_tokens: u32 = 0;
        let mut output_tokens: u32 = 0;
        let mut cache_read: u32 = 0;
        let mut cache_create: u32 = 0;
        let mut result_model: Option<String> = None;
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let env: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let env_type = env.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match env_type {
                // S124: in current CLI (2.1.139) `-p` mode emits buffered
                // `assistant` envelopes w/ the full message.content array
                // instead of per-token stream_event deltas. Extract text
                // from each content block here. Multiple `assistant` events
                // can land per turn (one per content block); the `result`
                // envelope below is the final aggregated source-of-truth.
                "assistant" => {
                    let msg = env.get("message").unwrap_or(&Value::Null);
                    if let Some(blocks) = msg.get("content").and_then(|v| v.as_array()) {
                        for b in blocks {
                            if b.get("type").and_then(|v| v.as_str()) == Some("text") {
                                if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                                    summary.push_str(t);
                                }
                            }
                        }
                    }
                    // Stream the in-flight summary to the frontend.
                    let elapsed_ms = last_emit_at.elapsed().as_millis();
                    let new_chars = summary.len().saturating_sub(last_emit_len);
                    if new_chars > 0 && (new_chars >= 64 || elapsed_ms >= 150) {
                        last_emit_at = std::time::Instant::now();
                        last_emit_len = summary.len();
                        let _ = progress_app.emit(
                            "assistant://summarize-progress",
                            serde_json::json!({
                                "session_id": progress_sid,
                                "summary_so_far": summary,
                                "status": "streaming",
                            }),
                        );
                    }
                }
                // Per-token deltas (alternative CLI output shape).
                "stream_event" => {
                    let inner = env.get("event").unwrap_or(&Value::Null);
                    if inner.get("type").and_then(|v| v.as_str()) == Some("content_block_delta") {
                        let delta = inner.get("delta").unwrap_or(&Value::Null);
                        if delta.get("type").and_then(|v| v.as_str()) == Some("text_delta") {
                            if let Some(t) = delta.get("text").and_then(|v| v.as_str()) {
                                summary.push_str(t);
                                let elapsed_ms = last_emit_at.elapsed().as_millis();
                                let new_chars = summary.len().saturating_sub(last_emit_len);
                                if new_chars > 0 && (new_chars >= 64 || elapsed_ms >= 150) {
                                    last_emit_at = std::time::Instant::now();
                                    last_emit_len = summary.len();
                                    let _ = progress_app.emit(
                                        "assistant://summarize-progress",
                                        serde_json::json!({
                                            "session_id": progress_sid,
                                            "summary_so_far": summary,
                                            "status": "streaming",
                                        }),
                                    );
                                }
                            }
                        }
                    }
                }
                // Terminal envelope w/ aggregated usage + cost.
                "result" => {
                    // S124: also drains `result.result` as the canonical
                    // aggregated text — overrides accumulated assistant
                    // events if non-empty so the parser is robust to
                    // either CLI output shape.
                    if let Some(t) = env.get("result").and_then(|v| v.as_str()) {
                        let trimmed = t.trim();
                        if !trimmed.is_empty() {
                            summary = trimmed.to_string();
                        }
                    }
                    if let Some(c) = env.get("total_cost_usd").and_then(|v| v.as_f64()) {
                        cost_usd = c;
                    }
                    let u = env.get("usage").unwrap_or(&Value::Null);
                    let g = |k: &str| -> u32 {
                        u.get(k)
                            .and_then(|v| v.as_u64())
                            .map(|n| n as u32)
                            .unwrap_or(0)
                    };
                    input_tokens = g("input_tokens");
                    output_tokens = g("output_tokens");
                    cache_read = g("cache_read_input_tokens");
                    cache_create = g("cache_creation_input_tokens");
                    if let Some(m) = env.get("model").and_then(|v| v.as_str()) {
                        result_model = Some(m.to_string());
                    }
                    // Emit final aggregated summary so the frontend lands on
                    // the canonical text (covers cases where assistant-event
                    // streaming was empty and `result.result` is the source).
                    let _ = progress_app.emit(
                        "assistant://summarize-progress",
                        serde_json::json!({
                            "session_id": progress_sid,
                            "summary_so_far": summary,
                            "status": "done",
                        }),
                    );
                }
                _ => {}
            }
        }
        (
            summary,
            cost_usd,
            input_tokens,
            output_tokens,
            cache_read,
            cache_create,
            result_model,
        )
    });

    let stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            buf.push_str(&l);
            buf.push('\n');
            if buf.len() > 32 * 1024 {
                break;
            }
        }
        buf
    });

    let status = child
        .wait()
        .await
        .map_err(|e| format!("await claude (summarize): {e}"))?;
    let (summary, cost_usd, input_tokens, output_tokens, cache_read, cache_create, result_model) =
        stdout_task.await.unwrap_or_default();
    // #222: surface drain-task JoinError as a string instead of swallowing it.
    let stderr_buf = stderr_task.await.unwrap_or_else(|e| {
        log::error!("summarize stderr drain task panicked: {e}");
        format!("(stderr drain task panicked: {e})")
    });

    if !status.success() {
        return Err(format!(
            "claude (summarize) exited {} — {}",
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            stderr_buf.trim()
        ));
    }
    let summary = summary.trim().to_string();
    if summary.is_empty() {
        return Err("summarize call returned empty text".into());
    }

    Ok(SummarizeResult {
        summary,
        model: result_model.unwrap_or(model),
        cost_usd,
        input_tokens,
        output_tokens,
        cache_read_tokens: cache_read,
        cache_create_tokens: cache_create,
    })
}

/// Compaction Phase C: copy the cwd sidecar from an old CLI session id to a
/// freshly-minted one. The old sidecar is left in place during a transition
/// window so a failed/aborted compaction can still --resume the prior
/// session. Cleanup of stranded old sidecars happens lazily via the next
/// `save_session_cwd` overwrite or never (best-effort housekeeping is fine —
/// each sidecar is ~80 bytes).
///
/// Both ids are validated as canonical UUIDs (#220 shape) before touching
/// disk. Errors propagate so the frontend can surface them in `lastError`.
#[tauri::command]
pub fn assistant_remint_session(
    old_session_id: String,
    new_session_id: String,
) -> Result<(), String> {
    if !is_valid_session_id(&old_session_id) {
        return Err(format!("invalid old session id: {old_session_id}"));
    }
    if !is_valid_session_id(&new_session_id) {
        return Err(format!("invalid new session id: {new_session_id}"));
    }
    if old_session_id == new_session_id {
        return Err("remint requires distinct old + new session ids".into());
    }
    let Some(cwd) = load_session_cwd(&old_session_id) else {
        // Legacy convos lacked sidecars; nothing to copy is not an error.
        // The new session will get a sidecar on its first turn via the
        // existing save_session_cwd path in assistant_send.
        return Ok(());
    };
    save_session_cwd(&new_session_id, &cwd);
    Ok(())
}

#[tauri::command]
pub fn assistant_get_allow_remote_shell() -> Result<bool, String> {
    Ok(load_config().allow_remote_shell.unwrap_or(false))
}

#[tauri::command]
pub fn assistant_set_allow_remote_shell(value: bool) -> Result<(), String> {
    let mut cfg = load_config();
    cfg.allow_remote_shell = Some(value);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_set_api_key(api_key: Option<String>) -> Result<(), String> {
    // Phase 6 (#37): write the API key to the OS keychain, not config.json.
    // Empty/None → delete the keychain entry. Also clears any lingering
    // legacy plaintext field (load_config's migration handles the read side,
    // but a fresh set after a failed migration leaves the legacy slot stale).
    match api_key.as_deref().filter(|s| !s.is_empty()) {
        Some(k) => crate::secrets::set(crate::secrets::ASSISTANT_API_KEY, k)?,
        None => crate::secrets::delete(crate::secrets::ASSISTANT_API_KEY)?,
    }
    let mut cfg = load_config();
    if cfg.api_key.is_some() {
        cfg.api_key = None;
        save_config(&cfg)?;
    }
    Ok(())
}

/// Workspace state surfaced to the frontend. `current` is the open folder or
/// `None` if no folder is open. `recent` is the MRU list (newest first).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub current: Option<String>,
    pub recent: Vec<String>,
}

fn workspace_state_from(cfg: &AssistantConfig) -> WorkspaceState {
    WorkspaceState {
        current: cfg.current_root.as_ref().map(|p| p.to_string_lossy().into_owned()),
        recent: cfg.recent_roots.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
    }
}

#[tauri::command]
pub fn assistant_get_workspace() -> Result<WorkspaceState, String> {
    Ok(workspace_state_from(&load_config()))
}

/// Set the active project folder. Validates the path exists and is a directory,
/// canonicalizes it (so `..`/symlinks don't drift), prepends to recent_roots
/// (dedup, capped at RECENT_ROOTS_MAX), and persists.
#[tauri::command]
pub fn assistant_set_root(path: String) -> Result<WorkspaceState, String> {
    let raw = PathBuf::from(&path);
    if !raw.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let canonical = std::fs::canonicalize(&raw).unwrap_or(raw);
    let mut cfg = load_config();
    // Dedup: pull existing entry then re-insert at the front.
    cfg.recent_roots.retain(|p| p != &canonical);
    cfg.recent_roots.insert(0, canonical.clone());
    if cfg.recent_roots.len() > RECENT_ROOTS_MAX {
        cfg.recent_roots.truncate(RECENT_ROOTS_MAX);
    }
    cfg.current_root = Some(canonical);
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

#[tauri::command]
pub fn assistant_clear_root() -> Result<WorkspaceState, String> {
    let mut cfg = load_config();
    cfg.current_root = None;
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

#[tauri::command]
pub fn assistant_remove_recent_root(path: String) -> Result<WorkspaceState, String> {
    let target = PathBuf::from(&path);
    let mut cfg = load_config();
    cfg.recent_roots.retain(|p| p != &target);
    save_config(&cfg)?;
    Ok(workspace_state_from(&cfg))
}

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

/// Enumerate file paths under the active workspace root, relative to the root,
/// forward-slash normalized. Drives the composer's `@`-file mention picker.
/// Skip set mirrors `mcp_server::SKIP_DIRS`. Capped at `MENTION_LIMIT` files.
#[tauri::command]
pub fn assistant_list_workspace_files() -> Result<Vec<String>, String> {
    const MENTION_LIMIT: usize = 4000;
    const SKIP_DIRS: &[&str] = &[
        "node_modules", ".git", ".svelte-kit", "build", "dist", "target",
        ".rift-trail", ".rift-tmp", "__pycache__", ".venv", ".next",
    ];
    let cfg = load_config();
    let root = match cfg.current_root {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::with_capacity(512);
    for entry in walkdir::WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()) {
                return false;
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
        out.push(rel.to_string_lossy().replace('\\', "/"));
        if out.len() >= MENTION_LIMIT {
            break;
        }
    }
    Ok(out)
}

/// Rift's system-prompt addendum. Appended to the CLI's default system prompt
/// via `--append-system-prompt`. Two variants — one for read-only mode (MCP
/// tools wired), one for the no-workspace fallback. Both single-line so the
/// .cmd-shim batch-arg validator (Rust 1.77+ CVE-2024-24576) accepts them.
const RIFT_SYSTEM_ADDENDUM_TOOLS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app, working inside the user's open project folder (your working directory is already set to the workspace root, so relative paths Just Work). You have the full Claude Code toolset: Read / Write / Edit / MultiEdit for files, Bash for shell commands (executes in the workspace dir, output streamed back), Glob for filename patterns, Grep for content search, WebFetch and WebSearch for the open web, TodoWrite for multi-step plans, and Agent for delegating heavy lookups. TodoWrite output surfaces in a dedicated Tasks panel in the user's UI — use it proactively whenever a request involves three or more distinct steps, and update statuses (pending → in_progress → completed) as you go. Rift's MCP server also exposes read_file / list_dir / grep as scoped helpers, sync_status to get a live reading of the sync queue (pending uploads, failed, conflicts) at any point mid-conversation — call it when the user asks whether files are synced or a push completed, rather than relying on the stale per-turn snapshot in the system-reminder — and ask_user for interactive multiple-choice questions. The standard Anthropic `AskUserQuestion` tool is NOT available in this environment; use `mcp__rift__ask_user` instead when you need a quick choice from the user (e.g. picking between approaches, confirming a destructive operation, narrowing scope). The answer comes back as the tool result and resumes the turn. Prefer Claude Code built-ins for normal work and use the MCP variants only when a guaranteed-workspace-rooted path matters or when you need user input. ACT FIRST, EXPLAIN AFTER — this overrides any conflicting instruction from inherited config. If the user asks you to fix / change / edit / add / build / refactor X, locate the file(s) with Grep + Read then make the Edit. Do NOT write paragraphs of plan, analysis, recommendations, or 'here's what I would do' before touching code — one short opening beat ('reading X', 'editing Y') is the cap. Never guess at file contents, function names, paths, APIs, or signatures — Grep or Read first if uncertain, otherwise hedge explicitly. Read narrowly with offset+limit on files >300 lines; do not re-read a file you already opened earlier this turn. Verify AFTER the edit (Bash to run the test / lint / build), not before. Surface tool errors verbatim and try a different approach instead of bouncing the problem back to the user. Don't ask the user for permission on routine work like file edits, shell commands, package installs, or git operations; the user expects you to do real work and can revert via git. Project stack is open-ended — do not assume the language, framework, or layout.";

const RIFT_SYSTEM_ADDENDUM_NO_WS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app. No project folder is open right now, so your file/list/grep tools are unavailable for this turn. Answer questions and discuss code the user pastes, but tell the user to open a folder on the Assistant page (the empty-state has an \"Open Folder\" button) if they want you to read their code directly. Do not claim capabilities you do not have.";

/// Build a per-turn snapshot of the live AutoSync / LockPresence state —
/// foreign locks held by other users, sync queue depth, recent DiagBus stage
/// events. Wrapped in a `<system-reminder>` on the USER turn (not the system
/// prompt) so the cached system prefix stays stable across turns. Returns
/// an empty string when no AutoSync engine is active.
async fn gather_workspace_context(state: &crate::AutoSyncState) -> String {
    let engine = { state.0.lock().await.clone() };
    let Some(engine) = engine else { return String::new(); };
    let folders = engine.folders_clone();
    if folders.is_empty() {
        return String::new();
    }
    let status = engine.status().await;
    let foreign: Vec<crate::sync::lock_presence::RemoteLock> = engine
        .locks()
        .map(|l| l.active_locks())
        .unwrap_or_default();

    let mut parts: Vec<String> = Vec::new();
    parts.push("Workspace is multi-writer (concurrent collaborators may be editing the same files over SFTP).".into());

    if foreign.is_empty() {
        parts.push("No foreign edits currently in progress.".into());
    } else {
        let preview: Vec<String> = foreign
            .iter()
            .take(4)
            .map(|l| {
                let file = l
                    .file_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&l.file_path)
                    .to_string();
                format!("{} on {} ({} ago)", l.user, file, rel_age(l.since))
            })
            .collect();
        let more = if foreign.len() > preview.len() {
            format!(" +{} more", foreign.len() - preview.len())
        } else {
            String::new()
        };
        parts.push(format!("Foreign edits in progress: {}{}.", preview.join(", "), more));
    }

    parts.push(format!(
        "Sync queue: {} pending, {} failed, {} conflicts.",
        status.pending, status.failed, status.conflicts
    ));

    let events = crate::diagnostics::bus().recent_events(20);
    let event_summary = summarize_events(&events);
    if !event_summary.is_empty() {
        parts.push(format!("Recent sync activity: {}.", event_summary));
    }

    parts.push("If you read a file more than ~30 s ago, re-read before editing — another writer may have changed it.".into());

    // Newline-separated for readability inside the <system-reminder> block.
    // No argv constraint here (rides stdin, not process args).
    parts.join("\n")
}

fn rel_age(when: chrono::DateTime<chrono::Utc>) -> String {
    let secs = (chrono::Utc::now() - when).num_seconds().max(0);
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

fn summarize_events(events: &[crate::diagnostics::DiagEvent]) -> String {
    use crate::diagnostics::{DiagLevel, DiagStage};
    let mut uploads_ok = 0usize;
    let mut uploads_fail = 0usize;
    let mut drift_scans = 0usize;
    let mut conflicts = 0usize;
    let mut pulls = 0usize;
    let mut wedged = 0usize;
    let mut last_log_err: Option<String> = None;
    for ev in events {
        match ev.stage {
            DiagStage::UploadDone => uploads_ok += 1,
            DiagStage::UploadFail => uploads_fail += 1,
            DiagStage::DriftScanResult => drift_scans += 1,
            DiagStage::RemotePullDone => pulls += 1,
            DiagStage::ConnectionWedged => wedged += 1,
            DiagStage::Log if matches!(ev.level, DiagLevel::Error | DiagLevel::Warn) => {
                if last_log_err.is_none() {
                    last_log_err = Some(ev.message.clone());
                }
            }
            _ => {}
        }
        if matches!(
            ev.stage,
            DiagStage::DriftScanResult
        ) && ev.message.to_lowercase().contains("conflict")
        {
            conflicts += 1;
        }
    }
    let mut tokens: Vec<String> = Vec::new();
    if uploads_ok > 0 {
        tokens.push(format!("{uploads_ok} uploads ok"));
    }
    if uploads_fail > 0 {
        tokens.push(format!("{uploads_fail} uploads failed"));
    }
    if pulls > 0 {
        tokens.push(format!("{pulls} pulls ok"));
    }
    if drift_scans > 0 {
        tokens.push(format!("{drift_scans} drift scans"));
    }
    if conflicts > 0 {
        tokens.push(format!("{conflicts} conflicts"));
    }
    if wedged > 0 {
        tokens.push(format!("{wedged} connection wedges"));
    }
    if let Some(msg) = last_log_err {
        let trimmed = if msg.len() > 80 {
            format!("{}\u{2026}", &msg[..80])
        } else {
            msg
        };
        tokens.push(format!("recent warn: {}", trimmed.replace('\n', " ")));
    }
    tokens.join(", ")
}

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
    let mut line = serde_json::to_vec(&env).unwrap_or_default();
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
    let _ = app.emit(PERMISSION_EVENT, serde_json::json!({
        "session_id": session_id,
        "request_id": request_id,
        "tool_use_id": tool_use_id,
        "tool_name": req.get("tool_name").cloned().unwrap_or(Value::Null),
        "input": req.get("input").cloned().unwrap_or(Value::Null),
        "suggestions": req.get("permission_suggestions").cloned().unwrap_or(Value::Null),
    }));

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
#[tauri::command]
pub async fn assistant_send(
    app: AppHandle,
    state: tauri::State<'_, crate::AutoSyncState>,
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
    let model = model.unwrap_or_else(|| "sonnet".to_string());
    if !is_valid_model_name(&model) {
        return Err(format!("invalid model: {model}"));
    }
    // Effort tier: per-turn override wins, else stored default, else "quick".
    let effort = thinking_effort
        .or_else(|| cfg.thinking_effort.clone())
        .unwrap_or_else(|| "quick".to_string());

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
    //   2. AutoSync server folders if any are connected.
    //   3. Empty → no-tools turn + no-workspace addendum.
    // Validate every candidate still exists on disk; missing dir → fall through.
    let pinned_cwd: Option<PathBuf> = if is_first_turn {
        None
    } else {
        load_session_cwd(&session_id).filter(|p| p.is_dir())
    };
    let mut roots: Vec<PathBuf> = if let Some(p) = pinned_cwd.clone() {
        vec![p]
    } else if let Some(root) = cfg.current_root.as_ref().filter(|p| p.is_dir()) {
        vec![root.clone()]
    } else {
        let guard = state.0.lock().await;
        guard
            .as_ref()
            .map(|eng| {
                eng.folders_clone()
                    .into_iter()
                    .map(|f| f.local_root)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    // When AutoSync surfaces N folders (one per FiveM resource), the
    // alphabetically-first wins as cwd — typically a `[bracket]` resource
    // ('[' = 0x5B). Prepend the lexical common ancestor so the model's cwd
    // lands at the parent (e.g. `<server>/resources/`) and every resource is
    // visible. Only applies to the AutoSync path (multiple roots, no pin,
    // no explicit current_root) — existing pinned conversations keep their
    // captured cwd to preserve session continuity even if it's narrower.
    if pinned_cwd.is_none() && cfg.current_root.is_none() && roots.len() > 1 {
        if let Some(anc) = common_ancestor(&roots) {
            if !roots.iter().any(|r| r == &anc) {
                roots.insert(0, anc);
            }
        }
    }
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

    // Remote-shell tool only fires when the user toggled it on AND the parent
    // can stand up the loopback bridge. Bridge `start` is idempotent — first
    // call binds the listener; later calls return the cached info.
    // Start bridge unconditionally — sync_status uses it even when remote_bash is off.
    // Bridge is a read-only loopback IPC channel; remote_bash is gated separately.
    let bridge_info = match remote_bridge::start(app.clone()).await {
        Ok(info) => Some(info),
        Err(e) => {
            log::warn!("assistant: bridge start failed, sync_status + remote_bash disabled: {e}");
            None
        }
    };
    let allow_remote_shell = cfg.allow_remote_shell.unwrap_or(false);
    let remote_shell_enabled = allow_remote_shell && bridge_info.is_some();
    // Trust level for the local git tools — explicit setting wins, else derived
    // from the remote-shell toggle so upgrades don't silently grant git-write.
    let trust_level = effective_trust_level(&cfg.trust_level, cfg.allow_remote_shell);

    // Provision a temp MCP config when we have at least one root. Addendum
    // stays cache-stable — only the two static strings ever land in
    // `--append-system-prompt`. Per-session/per-turn toggles (remote_shell,
    // dyslexia) ride the user-turn <system-reminder> path below so toggling
    // them mid-session never invalidates the cached system-prompt prefix.
    let (mcp_config_path, _mcp_guard, addendum) = if roots.is_empty() {
        (None, None, RIFT_SYSTEM_ADDENDUM_NO_WS)
    } else {
        match write_mcp_config(&session_id, &roots, bridge_info.as_ref(), remote_shell_enabled, &trust_level) {
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
        // S73: when the remote-shell toggle is on, the explicit-named path adds
        // `mcp__rift__remote_bash`. Piggyback already admits `mcp__*` so the
        // tool is reachable there unconditionally — the gate is server-side
        // (RIFT_REMOTE_SHELL_ENABLED env on the MCP child).
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
        const SAFE_MCP: &str = "mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep,mcp__rift__sync_status,mcp__rift__drift_snapshot,mcp__rift__reconcile_preview,mcp__rift__ask_user";
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
        let git_write = if matches!(trust_level.as_str(), "standard" | "full") {
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
        } else if remote_shell_enabled {
            format!("{BUILTINS},mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep,mcp__rift__sync_status,mcp__rift__drift_snapshot,mcp__rift__reconcile_preview,mcp__rift__remote_bash,mcp__rift__push_pending,mcp__rift__pull_pending,mcp__rift__reconcile_apply,mcp__rift__ask_user,{GIT_READ_MCP}{git_write}")
        } else {
            format!("{BUILTINS},mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep,mcp__rift__sync_status,mcp__rift__drift_snapshot,mcp__rift__reconcile_preview,mcp__rift__ask_user,{GIT_READ_MCP}{git_write}")
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
        // `--bare`: ignore OAuth/keychain, use ANTHROPIC_API_KEY strictly.
        cmd.arg("--bare");
        if let Some(k) = api_key.as_deref() {
            cmd.env("ANTHROPIC_API_KEY", k);
        }
    }

    // Effort-gated extended thinking via the CLI's `--effort` flag. Haiku
    // skips wholesale. Tier mapping mirrors Claude Code's own ladder:
    //   none  → --effort low      (minimal thinking, fastest TTFT)
    //   quick → --effort medium   (CC's default — balanced)
    //   deep  → --effort high     (heavy reasoning)
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
        "deep" => "high",
        "ultra" => "xhigh",
        _ /* "quick" or unknown */ => "medium",
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
        "assistant_send: spawn session_id={} first_turn={} model={} effort={} perm={} use_full_config={} mcp={} api_key={} remote_shell={}",
        session_id, is_first_turn, model, effort_level, permission_mode, use_full_config, mcp_config_path.is_some(), use_api_key, remote_shell_enabled
    );

    // Build the per-turn user-message text BEFORE spawning so the child
    // doesn't sit idle on stdin while we lock state. Live workspace state
    // (foreign locks, sync queue, recent diag events) + per-session toggles
    // (remote_shell, dyslexia) ride the USER message via a <system-reminder>
    // block instead of `--append-system-prompt`. A dynamic system prompt
    // invalidates the cache prefix every turn (cache layout: system → tools
    // → CLAUDE.md → conversation tail); keeping fresh per-turn data on the
    // user turn keeps the prefix cache-stable. Multi-line is fine here
    // (rides stdin, no argv constraint).
    let mut reminder_parts: Vec<String> = Vec::new();
    let ws_ctx = gather_workspace_context(&state).await;
    if !ws_ctx.is_empty() {
        reminder_parts.push(ws_ctx);
    }
    if remote_shell_enabled {
        reminder_parts.push("Remote-shell tool `mcp__rift__remote_bash` is available — runs over the auto-sync engine's russh session against the active SFTP server. Use sparingly for ops work (status checks, pm2 restart, etc.); a workspace-scoped advisory lock serializes calls between users.".into());
    }
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
    let mut child = cmd.spawn().map_err(|e| format!("spawn `claude`: {e}"))?;
    if let Some(pid) = child.id() {
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
        let _ = child.start_kill();
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
    if child.stdin.is_none() {
        let _ = child.start_kill();
        return Err("claude stdin unavailable — process killed".into());
    }
    let stdin = child.stdin.take().expect("stdin checked is_some above");

    // The per-turn user message — always a stream-json `user` envelope now
    // (text + optional image blocks). Sent by the reader task once the
    // `initialize` handshake is acknowledged.
    let user_line: Vec<u8> = {
        let mut content: Vec<Value> = Vec::with_capacity(1 + attachments.len());
        content.push(serde_json::json!({ "type": "text", "text": effective_prompt }));
        for a in &attachments {
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
            "message": { "role": "user", "content": content }
        });
        let mut line = serde_json::to_vec(&envelope)
            .map_err(|e| format!("serialize input envelope: {e}"))?;
        line.push(b'\n');
        line
    };

    let stdout = child.stdout.take().ok_or_else(|| "claude stdout missing".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "claude stderr missing".to_string())?;

    let app_out = app.clone();
    let stream_sid = session_id.clone();
    let stdout_task = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
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
        loop {
            match lines.next_line().await {
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
                        // `result` is the last frame — forward it, then break so
                        // stdin drops (EOF) and the CLI exits even if it would
                        // otherwise wait for more stream-json input.
                        if ty == Some("result") {
                            let _ = app_out.emit(STREAM_EVENT, serde_json::json!({
                                "session_id": stream_sid, "line": trimmed,
                            }));
                            break;
                        }
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
        // stdin dropped here → EOF.
    });

    // Drain stderr to a buffer for error-event surfacing on non-zero exit.
    // #66: cap at 64 KiB so a wedged CLI streaming error spew doesn't grow
    // the heap unboundedly. When the buffer crosses the cap, drop the first
    // 32 KiB and keep the tail — error context lives at the END of a stderr
    // stream (the panic / fatal-error line), not at the start.
    const STDERR_CAP: usize = 64 * 1024;
    const STDERR_TRIM: usize = 32 * 1024;
    let stderr_task = tokio::spawn(async move {
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
                let cut = buf[STDERR_TRIM..]
                    .find('\n')
                    .map(|n| STDERR_TRIM + n + 1)
                    .unwrap_or(STDERR_TRIM);
                buf.drain(..cut);
            }
        }
        if truncated {
            buf.insert_str(0, "[... earlier stderr dropped (>64 KiB) ...]\n");
        }
        buf
    });

    let status = child.wait().await.map_err(|e| format!("await claude: {e}"))?;
    clear_session_pid(&session_id);
    let _ = stdout_task.await;
    // #222: surface stderr-drain JoinError so a panicked drain task doesn't
    // turn into a blank stderr at the call site (which then shows up as
    // "claude exited with 1 — " with no diagnosis).
    let stderr_buf = stderr_task.await.unwrap_or_else(|e| {
        log::error!("stderr drain task panicked: {e}");
        format!("(stderr drain task panicked: {e})")
    });

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
        let msg = format!(
            "claude exited with {} — {}",
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            stderr_buf.trim()
        );
        let _ = app.emit(
            ERROR_EVENT,
            serde_json::json!({ "session_id": session_id, "message": msg.clone() }),
        );
        Err(msg)
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
