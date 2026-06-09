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
pub mod auth_update;
pub mod cli_install;
pub mod convo_store;
pub mod env_checks;
pub mod git_local;
pub mod mcp_server;
pub mod permission;
pub mod session_log;
pub mod workspace;

pub use ask_user::AskUserRegistry;
// R4 split (2026-06-09): auth probe + in-app sign-in + multi-install CLI
// updater in `auth_update.rs`. Glob re-export for the __cmd__ items.
pub use auth_update::*;
// R1 split (2026-06-09): CLI discovery/ranking/cache + spawn-command builder
// in `cli_install.rs`. Re-exports keep `assistant::ClaudeInstall` (DTO) and
// `crate::assistant::claude_command()` (stt/swarm callers) path-stable.
pub use cli_install::ClaudeInstall;
pub(crate) use cli_install::claude_command;
use cli_install::resolve_claude_exe;
// R3 split (2026-06-09): conversation persistence (convo JSON store, session
// cwd/model sidecars, session-id validation, retired-JSONL sweep) in
// `convo_store.rs`. Glob re-export for the __cmd__ items + Conversation DTOs
// + lib.rs's `assistant::cleanup_retired_jsonls`.
pub use convo_store::*;
use convo_store::{
    is_valid_session_id, load_session_cwd, load_session_model, save_session_cwd,
    save_session_model,
};
// R5 split (2026-06-09): compression toggle + proxy/host-tool probes in
// `env_checks.rs`. Glob re-export (like session_log) so the macro-generated
// `__cmd__*` items travel with the commands — a named re-export strands them.
pub use env_checks::*;
use env_checks::resolve_compression;
// R7 split (2026-06-09): workspace root state + @-mention file enumeration +
// branch probe in `workspace.rs`. Glob re-export for the same __cmd__ reason;
// `current_root` stays reachable as `crate::assistant::current_root` (stt).
pub use workspace::*;
pub(crate) use workspace::current_root;
pub use permission::PermissionRegistry;
// Flatten the session-log commands into `crate::assistant` so the wildcard
// `pub use crate::assistant::*` in commands/assistant.rs forwards them to the
// path `invoke_handler!` resolves.
pub use session_log::*;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

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

/// Tree-kill every tracked streaming `claude` child, draining the registry.
/// Best-effort + blocking: errors are logged, never returned. Used on the
/// update-apply path — each `claude` parents a `rift-tauri.exe` MCP child
/// (`RIFT_MCP_SERVER=1`) that holds an exclusive lock on `current/`, so a
/// `/T` tree-kill is required to release Velopack's swap target. `app.exit(0)`
/// is `std::process::exit` (skips `Drop`), so `kill_on_drop` never reaps these.
pub(crate) fn kill_all_session_children() {
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

fn clear_steer_tx(session_id: &str) {
    with_steer_tx(|m| { m.remove(session_id); });
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
    /// A system `ANTHROPIC_API_KEY` env var is present in Rift's environment.
    /// Rift deliberately ignores it (env keys are stripped from every spawn so
    /// the keychain/login model stays authoritative) — surfaced only so the UI
    /// can warn that an out-of-band key exists and is NOT being used.
    pub env_api_key_present: bool,
    /// How the resolved `claude` binary was installed — "npm" | "native" |
    /// "unknown". Drives the correct in-app update command. None when no CLI.
    pub install_method: Option<String>,
    /// Pill color: "green" | "yellow" | "red".
    pub pill: String,
    /// One-line user-facing status.
    pub summary: String,
    /// Every Claude CLI install detected on this machine (npm + native can
    /// coexist). The `active` one drives `cli_version`/`install_method`; the
    /// rest are surfaced so the UI can show + update all of them.
    #[serde(default)]
    pub installs: Vec<ClaudeInstall>,
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
    /// `standard` / `full`. `None` = `readonly`.
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
    /// LEGACY single-provider escape hatch (pre-2a). Migrated into `providers`
    /// on first `load_config()` and then cleared. Still parsed so old on-disk
    /// configs migrate cleanly; never re-written once `providers` is populated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    /// LEGACY companion to `base_url`. Migrated + cleared alongside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider_model: Option<String>,
    /// 2a multi-provider list (cc-switch pattern). Each profile is an
    /// Anthropic-compatible endpoint; the active one routes `assistant_send`.
    /// Empty = Anthropic only. Secrets live in the keychain (`key_ref`), never here.
    #[serde(default)]
    providers: Vec<ProviderProfile>,
    /// `id` of the provider that routes turns. `None` = Anthropic (no custom endpoint).
    #[serde(default)]
    active_provider_id: Option<String>,
    /// 3c compression toggle (headroom-style local proxy). When `true`,
    /// `assistant_send` points `ANTHROPIC_BASE_URL` at a local compression proxy
    /// that deterministically shrinks context before forwarding upstream. Opt-in,
    /// OFF by default. The Python proxy runtime is a SOFT dependency Rift never
    /// bundles or spawns — the user runs it (e.g. `headroom serve`); Rift only
    /// owns the env seam + a reachability check. An active custom provider wins
    /// the same seam, so compression is bypassed for custom-provider turns.
    #[serde(default)]
    compression_enabled: Option<bool>,
    /// Local compression proxy URL. `None` = the headroom default
    /// (`http://127.0.0.1:8787`). Only consulted when `compression_enabled`.
    #[serde(default)]
    compression_proxy_url: Option<String>,
}

/// One saved custom-provider endpoint. `key_ref` is the keychain entry name
/// holding this provider's API key — the secret itself is never serialized to
/// `config.json`. `model` (optional) overrides Rift's tier via `--model`;
/// gateways that map Rift's tiers can leave it blank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub key_ref: String,
}

/// Outward shape for `assistant_list_providers` — the profile plus derived
/// flags the UI needs (`has_key` so it can show "key set" without exposing it,
/// `active` for the one-click toggle). The secret + raw `key_ref` are still
/// withheld from the renderer beyond what it needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: Option<String>,
    pub has_key: bool,
    pub active: bool,
}

/// Renderer-supplied profile for `assistant_save_provider`. `id` empty/None on
/// create (server mints a stable slug); set on edit. The API key rides a
/// separate arg so it can be omitted on edit without clobbering the stored one.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub model: Option<String>,
}

const RECENT_ROOTS_MAX: usize = 10;

fn config_path() -> Result<PathBuf, String> {
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir ~/.rift/assistant: {e}"))?;
    Ok(dir.join("config.json"))
}


/// #221: reject any model value that would be parsed as a flag by the CLI.
/// Allowlist `[A-Za-z0-9._-]+` w/ NO leading dash. Covers short aliases
/// (`sonnet`/`opus`/`haiku`) and full ids (`claude-opus-4-7`).
pub(super) fn is_valid_model_name(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') {
        return false;
    }
    s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Claude Fable 5 — limited run Rift offers only through 2026-06-22. Past
/// sunset a stale pref/session pin falls back to `opus` instead of firing at
/// a retired model id.
const FABLE_MODEL: &str = "claude-fable-5";
const FABLE_SUNSET_EPOCH_SECS: u64 = 1_782_172_800; // 2026-06-23T00:00:00Z

fn fable_sunset_passed() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() >= FABLE_SUNSET_EPOCH_SECS)
        .unwrap_or(false)
}




pub(super) fn dirs_home() -> Result<PathBuf, String> {
    crate::state::paths::dirs_home().map_err(|e| e.to_string())
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
    // 2a: migrate the legacy single `base_url`/`provider_model` into the
    // `providers` list once. The legacy escape hatch reused the main Anthropic
    // key, so the migrated profile points `key_ref` at ASSISTANT_API_KEY — the
    // user keeps routing without re-entering anything. Gated on empty list so it
    // runs at most once; clears the legacy fields after so it never re-fires.
    if cfg.providers.is_empty() {
        if let Some(b) = cfg.base_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            cfg.providers.push(ProviderProfile {
                id: "legacy".into(),
                name: "Custom".into(),
                base_url: b.to_string(),
                model: cfg.provider_model.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                key_ref: crate::secrets::ASSISTANT_API_KEY.into(),
            });
            cfg.active_provider_id = Some("legacy".into());
            cfg.base_url = None;
            cfg.provider_model = None;
            if let Err(e) = save_config(&cfg) {
                log::warn!("assistant: provider migration save_config failed: {e}");
            } else {
                log::info!("assistant: migrated legacy base_url into providers list");
            }
        }
    }
    cfg
}

/// Keychain entry name for a provider's API key. Stable across renames since
/// it's keyed on the immutable `id`. The legacy migrated provider is the lone
/// exception — it reuses `ASSISTANT_API_KEY`.
fn provider_key_ref(id: &str) -> String {
    format!("assistant.provider.{id}")
}

/// The provider that should route turns, or `None` for plain Anthropic.
fn resolve_active_provider(cfg: &AssistantConfig) -> Option<ProviderProfile> {
    let id = cfg.active_provider_id.as_deref()?;
    cfg.providers.iter().find(|p| p.id == id).cloned()
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

/// Monotonic per-turn nonce for MCP config filenames. Prevents an outgoing
/// turn's cleanup guard from deleting the file a same-session resume just wrote
/// (the queue-drain race). Process-lifetime counter — uniqueness, not ordering.
static MCP_CFG_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// RAII guard that deletes the per-turn MCP config file when dropped.
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
/// `session_id` is appended to the filename so concurrent `assistant_send`
/// calls (multi-tab) each get their own file — no cross-tab cred leak.
fn write_mcp_config(
    session_id: &str,
    roots: &[PathBuf],
    trust_level: &str,
) -> Result<PathBuf, String> {
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir ~/.rift/assistant: {e}"))?;
    // Per-TURN filename, not per-session. A queued message drains into a fresh
    // `assistant_send` that --resumes the SAME session_id the instant the prior
    // turn's `result` lands — but that prior turn's McpConfigGuard hasn't dropped
    // yet (it waits on child.wait(), up to REAP_GRACE later). A shared per-session
    // name let the OUTGOING guard delete the file the INCOMING turn just wrote,
    // so claude2 read `--mcp-config` and found nothing → "exited with 1 — config
    // file not found". The monotonic nonce gives each turn its own file; the guard
    // deletes by the returned PathBuf, so it only ever removes its own. The
    // exit-time glob `mcp-config-*.json` still sweeps every variant. sanitize:
    // replace path-unsafe chars so the session UUID is a valid filename component.
    let safe_id = session_id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
    let nonce = MCP_CFG_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!("mcp-config-{safe_id}-{nonce}.json"));

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    // RIFT_MCP_ROOTS is newline-separated, so a root path containing an embedded
    // newline would split into two phantom roots in the MCP child — widening the
    // path-safety boundary. Drop any such path rather than corrupt the list (F59).
    let roots_joined = roots
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| !s.contains('\n') && !s.contains('\r'))
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
            let icacls_status = std::process::Command::new("icacls")
                .arg(&path)
                // Quote the principal: domain usernames can contain spaces, which
                // icacls would otherwise parse as separate ACL tokens.
                .args(["/inheritance:r", "/grant:r", &format!("\"{user}\":(F)")])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if !matches!(icacls_status, Ok(s) if s.success()) {
                log::warn!("icacls failed to lock down {} for user {user}", path.display());
            }
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

/// Serializes every `assistant_set_*` config write. Each setter does a
/// load→modify→save read-modify-write; without this, two concurrent setters
/// (the Settings page can fire several `invoke`s in one render) read the same
/// on-disk state and the second save silently clobbers the first's change.
static CONFIG_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tauri::command]
pub fn assistant_set_use_full_config(value: bool) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    cfg.max_budget_usd = value.filter(|v| v.is_finite() && *v > 0.0);
    save_config(&cfg)
}

// F48: assistant_{get,set}_thinking_effort and _permission_mode commands removed
// — the frontend persists both via localStorage and passes them per-send through
// assistant_send's args; the config-file round-trip these commands wrote was a
// dead second store the UI never read. The `thinking_effort`/`permission_mode`
// config fields + `is_valid_permission_mode` are still used as per-send fallbacks.

/// The CLI's `--permission-mode` values Rift exposes. `dontAsk` (the CLI's
/// auto-DENY mode) is intentionally excluded — there's no Rift surface to
/// approve, so it would silently block everything (see the S92 note below).
fn is_valid_permission_mode(v: &str) -> bool {
    matches!(v, "default" | "acceptEdits" | "plan" | "auto" | "bypassPermissions")
}

/// The Assistant trust levels Rift exposes. Gates the local git tools in the
/// MCP server (`git_local.rs`): `readonly` → status/diff/log; `standard` →
/// adds pull/commit/push; `full` → reserved for RCON raw passthrough (phase 2).
fn is_valid_trust_level(v: &str) -> bool {
    matches!(v, "readonly" | "standard" | "full")
}

/// Resolve the effective trust level. Explicit setting wins; when unset → `readonly`.
fn effective_trust_level(trust_level: &Option<String>) -> String {
    trust_level
        .clone()
        .filter(|v| is_valid_trust_level(v))
        .unwrap_or_else(|| "readonly".into())
}

#[tauri::command]
pub fn assistant_get_trust_level() -> Result<String, String> {
    let cfg = load_config();
    Ok(effective_trust_level(&cfg.trust_level))
}

#[tauri::command]
pub fn assistant_set_trust_level(value: String) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if !matches!(value.as_str(), "haiku" | "sonnet" | "opus") {
        return Err(format!("invalid compact_model: {value}"));
    }
    let mut cfg = load_config();
    cfg.compact_model = Some(value);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_base_url() -> Result<Option<String>, String> {
    Ok(load_config().base_url.filter(|s| !s.trim().is_empty()))
}

/// Custom Anthropic-compatible endpoint; routes headless `-p` turns off the metered pool (June-15).
#[tauri::command]
pub fn assistant_set_base_url(value: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let v = value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref u) = v {
        if !(u.starts_with("http://") || u.starts_with("https://")) {
            return Err("base_url must start with http:// or https://".into());
        }
    }
    let mut cfg = load_config();
    cfg.base_url = v;
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_provider_model() -> Result<Option<String>, String> {
    Ok(load_config().provider_model.filter(|s| !s.trim().is_empty()))
}

/// Model id passed to `--model` when a custom base_url is set (e.g. "deepseek-chat").
#[tauri::command]
pub fn assistant_set_provider_model(value: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let v = value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref m) = v {
        if !is_valid_model_name(m) {
            return Err(format!("invalid provider_model: {m}"));
        }
    }
    let mut cfg = load_config();
    cfg.provider_model = v;
    save_config(&cfg)
}

// ── 2a multi-provider escape hatch (cc-switch pattern) ──

/// List saved custom providers with derived flags. Secrets are never returned —
/// only `has_key` so the UI can show "key set" without exposing the value.
#[tauri::command]
pub fn assistant_list_providers() -> Result<Vec<ProviderDto>, String> {
    let cfg = load_config();
    let active = cfg.active_provider_id.as_deref();
    Ok(cfg
        .providers
        .iter()
        .map(|p| ProviderDto {
            id: p.id.clone(),
            name: p.name.clone(),
            base_url: p.base_url.clone(),
            model: p.model.clone(),
            has_key: crate::secrets::get(&p.key_ref).is_some(),
            active: active == Some(p.id.as_str()),
        })
        .collect())
}

/// Create or update a provider. Empty/None `id` → create (server mints a stable
/// slug from the name); otherwise update in place. `api_key`, when present and
/// non-empty, is written to the keychain under the profile's `key_ref`; omit it
/// on edit to leave the stored key untouched. Returns the profile id.
#[tauri::command]
pub fn assistant_save_provider(profile: ProviderInput, api_key: Option<String>) -> Result<String, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let name = profile.name.trim().to_string();
    if name.is_empty() {
        return Err("provider name is required".into());
    }
    let base_url = profile.base_url.trim().to_string();
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("base_url must start with http:// or https://".into());
    }
    let model = profile.model.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref m) = model {
        if !is_valid_model_name(m) {
            return Err(format!("invalid provider model: {m}"));
        }
    }

    let mut cfg = load_config();
    let id = profile
        .id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| mint_provider_id(&name, &cfg.providers));

    // key_ref is stable per id; the legacy migrated provider keeps reusing the
    // main key, so don't reassign its key_ref to the provider-scoped slot.
    let key_ref = cfg
        .providers
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.key_ref.clone())
        .unwrap_or_else(|| provider_key_ref(&id));

    if let Some(k) = api_key.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        crate::secrets::set(&key_ref, k)?;
    }

    let prof = ProviderProfile { id: id.clone(), name, base_url, model, key_ref };
    match cfg.providers.iter_mut().find(|p| p.id == id) {
        Some(existing) => *existing = prof,
        None => cfg.providers.push(prof),
    }
    save_config(&cfg)?;
    Ok(id)
}

/// Delete a provider + its keychain entry. Clears `active_provider_id` if it
/// pointed here. The legacy provider's shared `ASSISTANT_API_KEY` is preserved
/// (it's also the Anthropic key) — only provider-scoped keys are removed.
#[tauri::command]
pub fn assistant_delete_provider(id: String) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    if let Some(pos) = cfg.providers.iter().position(|p| p.id == id) {
        let removed = cfg.providers.remove(pos);
        if removed.key_ref != crate::secrets::ASSISTANT_API_KEY {
            if let Err(e) = crate::secrets::delete(&removed.key_ref) {
                log::warn!("assistant: delete provider key {} failed: {e}", removed.key_ref);
            }
        }
        if cfg.active_provider_id.as_deref() == Some(id.as_str()) {
            cfg.active_provider_id = None;
        }
        save_config(&cfg)?;
    }
    Ok(())
}

/// Set the active provider (`None` = Anthropic). Rejects an unknown id.
#[tauri::command]
pub fn assistant_set_active_provider(id: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    let id = id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref want) = id {
        if !cfg.providers.iter().any(|p| &p.id == want) {
            return Err(format!("unknown provider id: {want}"));
        }
    }
    cfg.active_provider_id = id;
    save_config(&cfg)
}


/// Mint a stable, filesystem/keychain-safe id from the provider name, deduped
/// against existing ids. No uuid dep — a slug + numeric suffix is stable enough
/// (ids are immutable once created).
fn mint_provider_id(name: &str, existing: &[ProviderProfile]) -> String {
    let base: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() { "provider".to_string() } else { base };
    if !existing.iter().any(|p| p.id == base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|cand| !existing.iter().any(|p| &p.id == cand))
        .unwrap_or(base)
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
Claude Code — an agentic coding assistant that reads files, runs commands, and edits code directly.\n\
\n\
Calibrate the effort to the draft — match its need, do not inflate it:\n\
- Already clear + specific: tighten the wording, make the goal explicit, and add only the obvious missing specific (a key edge case or a one-line acceptance check). Resist expanding it.\n\
- Vague or terse: infer the concrete intent and lay out what Claude Code needs to act — the specific mechanism, the files/areas likely involved, states + edge cases, and a brief acceptance check.\n\
\n\
Always:\n\
- Lead with the concrete goal in direct imperative voice (Add…, Fix…, Refactor…, Investigate…).\n\
- Preserve EVERY technical specific verbatim: file names, paths, identifiers, versions, numbers, commands, error text.\n\
- Stay strictly inside the draft's intent — flesh out HOW to do exactly what was asked. Never bolt on unrelated features, files, or scope the draft never implied.\n\
- For a bug: keep the stated symptom and any stated cause; you may point to likely places to look, but do not assert a fix or diagnosis the draft didn't state.\n\
- Format by shape: multiple parts → a short bullet list; otherwise one tight paragraph. No filler, no restatement, no closing summary.\n\
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
    model: Option<String>,
    directive: Option<String>,
    cwd: Option<String>,
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
    // Sonnet by default — the quality lever for a nuanced rewrite. Caller may
    // override (e.g. "haiku" for a fast pass).
    let model = model
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "sonnet".to_string());
    // Optional steering for the refine loop (Concise / Detailed / freeform).
    let directive_line = match directive.as_deref().map(str::trim).filter(|d| !d.is_empty()) {
        Some(d) => format!(" Adjustment for this rewrite: {d}."),
        None => String::new(),
    };
    // Grounded mode: a valid workspace dir lets the enhancer consult the real
    // code (read-only) so it names actual files + symbols. Absent → the fast,
    // context-free pure-completion path.
    let ground_root: Option<PathBuf> = cwd
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_dir());
    let ground_line = if ground_root.is_some() {
        " You have read-only access to the user's codebase via the read_file, grep, and list_dir tools. \
Make a few targeted lookups ONLY when they ground the rewrite in real specifics (actual paths, function/symbol \
names), then output the rewritten prompt. Keep lookups minimal."
    } else {
        ""
    };

    let mut cmd = claude_command()
        .ok_or_else(|| "claude CLI not on PATH — install Claude Code or configure an API key".to_string())?;
    // Fence the draft as inert content + an explicit "do not answer it"
    // directive, so a draft phrased like a message to an assistant isn't
    // mistaken for a conversational turn.
    let user_msg = format!(
        "Rewrite the rough prompt draft delimited by <draft></draft> below into a clear, well-structured prompt. \
         Do NOT answer the draft, do NOT respond to it conversationally, do NOT address me — treat everything inside \
         the tags purely as text to improve. Output ONLY the rewritten prompt.{directive_line}{ground_line}\n\n<draft>\n{trimmed}\n</draft>"
    );
    cmd.arg("-p").arg(&user_msg)
        // Meta-prompt rides the system prompt (stable across calls) so the
        // server-side prompt cache hits on repeat enhances within its ~5min TTL.
        .arg("--append-system-prompt").arg(ENHANCE_META_PROMPT)
        // Drop per-machine sections (cwd/env/git/memory) from the system prompt.
        .arg("--exclude-dynamic-system-prompt-sections")
        // Stream the rewrite token-by-token. CRITICAL: the draft MUST ride text
        // input (`-p <arg>`, null stdin) — `--input-format stream-json`
        // block-buffers the bundled claude.exe (measured 2026-05-27); plain `-p`
        // flushes incrementally. (Tools are fine; only input mode breaks it.)
        .arg("--output-format").arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--model").arg(&model)
        // Tight cap — a sub-1K-token rewrite costs a cent or two.
        .arg("--max-budget-usd").arg("0.20")
        .arg("--disable-slash-commands")
        .arg("--permission-mode").arg("bypassPermissions")
        // One-shot: never persist this throwaway rewrite to the session store.
        .arg("--no-session-persistence")
        // Latency killers: skip SessionStart hooks, autoupdater, telemetry.
        .env("CLAUDE_DISABLE_HOOKS", "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Tool + cwd wiring differs by mode. The guard cleans up the per-request MCP
    // config file after the child exits (held until this fn returns).
    let _mcp_guard: Option<McpConfigGuard> = if let Some(root) = ground_root {
        let trust = effective_trust_level(&None);
        match write_mcp_config(&request_id, std::slice::from_ref(&root), &trust) {
            Ok(p) => {
                cmd.arg("--strict-mcp-config")
                    .arg("--mcp-config").arg(&p)
                    .arg("--allowed-tools")
                    .arg("mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep")
                    // Bound the agentic pass so a rewrite can't spiral.
                    .arg("--max-turns").arg("6")
                    .current_dir(&root);
                Some(McpConfigGuard(p))
            }
            Err(e) => {
                log::warn!("assistant: enhance grounding unavailable, using context-free: {e}");
                cmd.arg("--strict-mcp-config")
                    .arg("--tools").arg("")
                    .current_dir(std::env::temp_dir());
                None
            }
        }
    } else {
        // Fast path: zero tools, neutral cwd — the original pure-completion call.
        cmd.arg("--strict-mcp-config")
            .arg("--tools").arg("")
            .current_dir(std::env::temp_dir());
        None
    };

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
            // F4: keep draining to EOF even past the cap — `break`ing would let
            // the child's stderr pipe fill and deadlock it on wait().
            if buf.len() <= 8192 {
                buf.push_str(&l);
                buf.push('\n');
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

/// System prompt for conversation-title generation. Tight constraints: a short
/// Title-Case phrase capturing the task, never a sentence or the raw message.
const TITLE_META_PROMPT: &str = "You generate a concise title for a chat, given the user's opening message. \
Output a 3-to-6 word phrase in Title Case that captures the core task or topic. \
No surrounding quotes, no trailing punctuation, no preamble, no explanation. \
Examples: 'Run this bash command: echo hi' -> Bash Echo Command Test; \
'ok so where do we leave off on this project' -> Project Status Check-In; \
'fix the login bug where users cant sign in' -> Fix Login Sign-In Bug. \
Output ONLY the title.";

/// One-shot conversation-title generator. Same headless `claude -p` path as
/// `assistant_enhance_prompt` (Haiku, no session, no tools, neutral cwd), but
/// returns a short Title-Case phrase and emits no stream events — the frontend
/// fires this after the first assistant turn and patches the conversation
/// title in place. Cheap enough (sub-100-token completion) to run per convo.
#[tauri::command]
pub async fn assistant_generate_title(prompt: String) -> Result<String, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("nothing to title".into());
    }
    // Only the opening message seeds the title — cap the slice so a giant first
    // paste can't balloon cost or arg length.
    let snippet: String = trimmed.chars().take(2000).collect();
    let mut cmd = claude_command()
        .ok_or_else(|| "claude CLI not on PATH — install Claude Code or configure an API key".to_string())?;
    let user_msg = format!(
        "Generate a short title for a chat that opens with the message delimited by <msg></msg>. \
         Do NOT answer or respond to it — treat everything inside the tags purely as text to title. \
         Output ONLY the title.\n\n<msg>\n{snippet}\n</msg>"
    );
    cmd.arg("-p").arg(&user_msg)
        .arg("--append-system-prompt").arg(TITLE_META_PROMPT)
        .arg("--exclude-dynamic-system-prompt-sections")
        // Plain `-p` text input streams incrementally (see enhance_prompt note).
        .arg("--output-format").arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--model").arg("haiku")
        // A title is tens of tokens — pennies-fraction cap.
        .arg("--max-budget-usd").arg("0.05")
        .arg("--strict-mcp-config")
        .arg("--disable-slash-commands")
        .arg("--tools").arg("")
        .arg("--permission-mode").arg("bypassPermissions")
        .arg("--no-session-persistence")
        .current_dir(std::env::temp_dir())
        .env("CLAUDE_DISABLE_HOOKS", "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn `claude` (title): {e}"))?;
    let stdout = child.stdout.take().ok_or("title stdout unavailable")?;
    let stderr = child.stderr.take().ok_or("title stderr unavailable")?;

    let stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            // F4: keep draining to EOF even past the cap — `break`ing would let
            // the child's stderr pipe fill and deadlock it on wait().
            if buf.len() <= 8192 {
                buf.push_str(&l);
                buf.push('\n');
            }
        }
        buf
    });

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
            acc.push_str(txt);
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("await claude (title): {e}"))?;
    let stderr_buf = stderr_task.await.unwrap_or_default();
    if !status.success() {
        let msg = stderr_buf.trim();
        return Err(if msg.is_empty() {
            "title generation failed".to_string()
        } else {
            format!("title failed: {msg}")
        });
    }
    // Sanitize: first line only, strip wrapping quotes, cap length. A
    // well-behaved Haiku returns exactly the phrase, but guard against a
    // stray quote or trailing newline.
    let title = acc
        .trim()
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .trim()
        .chars()
        .take(80)
        .collect::<String>();
    if title.is_empty() {
        return Err("title generation returned empty output".into());
    }
    Ok(title)
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
        // Fence off user MCP servers + slash commands (mirror enhance_prompt /
        // generate_title): a one-shot summarize must not merge ~/.claude.json
        // MCP entries onto an already-near-full-context call.
        .arg("--strict-mcp-config")
        .arg("--disable-slash-commands")
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

    // API-key users: claude_command() stripped ANTHROPIC_API_KEY, so without
    // re-adding it (+ `--bare`) this spawn has no credentials and every
    // compaction 401s. Mirrors the assistant_send `use_api_key` branch.
    if let Some(k) = current_api_key() {
        cmd.arg("--bare");
        cmd.env("ANTHROPIC_API_KEY", &k);
    }

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
            // F4: keep draining to EOF past the cap so the child never blocks
            // on a full stderr pipe at wait().
            if buf.len() <= 32 * 1024 {
                buf.push_str(&l);
                buf.push('\n');
            }
        }
        buf
    });

    let status = child
        .wait()
        .await
        .map_err(|e| format!("await claude (summarize): {e}"))?;
    // F3/F5/F66/F68: surface a panicked stdout drain instead of `unwrap_or_default`
    // silently zeroing the whole tuple (empty summary + zero tokens read as a
    // successful-but-blank turn).
    let (summary, cost_usd, input_tokens, output_tokens, cache_read, cache_create, result_model) =
        match stdout_task.await {
            Ok(t) => t,
            Err(e) => {
                log::error!("summarize stdout drain task panicked: {e}");
                return Err(format!("summarize stdout drain task panicked: {e}"));
            }
        };
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
    // Carry the model pin across compaction so the reminted session keeps
    // resuming under the model its (replayed) thinking blocks were signed by.
    if let Some(m) = load_session_model(&old_session_id) {
        save_session_model(&new_session_id, &m);
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
pub fn assistant_set_api_key(api_key: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
const RIFT_SYSTEM_ADDENDUM_TOOLS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app, working inside the user's open project folder (your working directory is already set to the workspace root, so relative paths Just Work). You have the full Claude Code toolset: Read / Write / Edit / MultiEdit for files, Bash for shell commands (executes in the workspace dir, output streamed back), Glob for filename patterns, Grep for content search, WebFetch and WebSearch for the open web, TodoWrite for multi-step plans, and Agent for delegating heavy lookups. TodoWrite output surfaces in a dedicated Tasks panel in the user's UI — use it proactively whenever a request involves three or more distinct steps, and update statuses (pending → in_progress → completed) as you go. Rift's MCP server also exposes read_file / list_dir / grep as scoped, workspace-rooted helpers, plus git_status / git_diff / git_log (and git_pull / git_commit / git_push when trust permits). The standard Anthropic `AskUserQuestion` tool is NOT available in this environment; if you need a decision from the user, ask in plain text and proceed with the most reasonable default. Prefer Claude Code built-ins for normal work and use the MCP variants only when a guaranteed-workspace-rooted path matters. ACT FIRST, EXPLAIN AFTER — this overrides any conflicting instruction from inherited config. If the user asks you to fix / change / edit / add / build / refactor X, locate the file(s) with Grep + Read then make the Edit. Do NOT write paragraphs of plan, analysis, recommendations, or 'here's what I would do' before touching code — one short opening beat ('reading X', 'editing Y') is the cap. Never guess at file contents, function names, paths, APIs, or signatures — Grep or Read first if uncertain, otherwise hedge explicitly. Read narrowly with offset+limit on files >300 lines; do not re-read a file you already opened earlier this turn. Verify AFTER the edit (Bash to run the test / lint / build), not before. Surface tool errors verbatim and try a different approach instead of bouncing the problem back to the user. Don't ask the user for permission on routine work like file edits, shell commands, package installs, or git operations; the user expects you to do real work and can revert via git. Project stack is open-ended — do not assume the language, framework, or layout.";

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
    // June-15 hedge: the active custom provider routes this turn to a cheaper
    // endpoint, off the metered pool. None = plain Anthropic.
    let active_provider = resolve_active_provider(&cfg);
    let custom_base: Option<String> = active_provider.as_ref().map(|p| p.base_url.clone());
    // Custom-provider auth: its own keychain key, falling back to the main
    // Anthropic key (covers gateways that reuse it + the legacy migrated profile).
    let provider_key: Option<String> = active_provider
        .as_ref()
        .and_then(|p| crate::secrets::get(&p.key_ref))
        .or_else(|| api_key.clone());
    // Spawn with `--bare` + injected key whenever we have credentials for the
    // route: the provider key in custom mode, the Anthropic key otherwise.
    let use_api_key = if custom_base.is_some() {
        provider_key.is_some()
    } else {
        api_key.is_some()
    };
    let mut model = model.unwrap_or_else(|| "sonnet".to_string());
    // Custom endpoints use their own model ids; the profile's model overrides the Rift tier.
    if let Some(ref p) = active_provider {
        if let Some(pm) = p.model.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            model = pm.to_string();
        }
    }
    if !is_valid_model_name(&model) {
        return Err(format!("invalid model: {model}"));
    }
    // Pin model per conversation: thinking-block signatures are model-bound, so
    // resuming under a switched model 400s on the replayed prior turn (see
    // session_model_path). On resume, the model the session was created with wins
    // over a live picker change; the new model only takes effect in a new chat.
    // Pin is Anthropic-only (thinking-signature preservation); a custom endpoint
    // must keep its own provider_model, not a stale Anthropic tier.
    if !is_first_turn && custom_base.is_none() {
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
    // falls back once the limited run ends. Anthropic route only.
    if custom_base.is_none() && model == FABLE_MODEL && fable_sunset_passed() {
        log::info!("assistant_send: {FABLE_MODEL} sunset passed — falling back to opus");
        model = "opus".to_string();
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
        } else {
            format!("{BUILTINS},mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep,{GIT_READ_MCP}{git_write}")
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

    // June-15 hedge: route this turn to a custom Anthropic-compatible endpoint when set.
    // 3c: with no custom provider active, an enabled compression toggle routes the turn
    // through the local compression proxy instead (it forwards upstream to Anthropic).
    // Both can't own ANTHROPIC_BASE_URL — the custom provider wins the seam.
    let compression_base = if custom_base.is_none() { resolve_compression(&cfg) } else { None };
    if let Some(ref base) = custom_base {
        cmd.env("ANTHROPIC_BASE_URL", base);
    } else if let Some(ref base) = compression_base {
        cmd.env("ANTHROPIC_BASE_URL", base);
        log::info!("assistant_send: routing turn through compression proxy {base}");
    }
    if use_api_key {
        // `--bare`: ignore OAuth/keychain, use ANTHROPIC_API_KEY strictly. The
        // builder stripped any inherited env key; this re-adds the sanctioned
        // Rift-configured one (the only API-key path). OAuth/login turns leave
        // it stripped so a stray system env key can't shadow `claude login`.
        cmd.arg("--bare");
        // Custom routes use the active provider's key; Anthropic uses the main key.
        let auth = if custom_base.is_some() { provider_key.as_deref() } else { api_key.as_deref() };
        if let Some(k) = auth {
            // Custom endpoints want a bearer token (ANTHROPIC_AUTH_TOKEN); set both for gateway compat.
            if custom_base.is_some() {
                cmd.env("ANTHROPIC_AUTH_TOKEN", k);
            }
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
    if model != "haiku" && custom_base.is_none() {
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
    // doesn't sit idle on stdin while we lock state. Live workspace state
    // (foreign locks, sync queue, recent diag events) + per-session toggles
    // (remote_shell, dyslexia) ride the USER message via a <system-reminder>
    // block instead of `--append-system-prompt`. A dynamic system prompt
    // invalidates the cache prefix every turn (cache layout: system → tools
    // → CLAUDE.md → conversation tail); keeping fresh per-turn data on the
    // user turn keeps the prefix cache-stable. Multi-line is fine here
    // (rides stdin, no argv constraint).
    let mut reminder_parts: Vec<String> = Vec::new();
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
        // #39: the pid was registered above; clear it so a later assistant_stop
        // can't taskkill a since-recycled pid.
        clear_session_pid(&session_id);
        return Err("claude stdin unavailable — process killed".into());
    }
    let stdin = child.stdin.take().expect("stdin checked is_some above");

    // The per-turn user message — always a stream-json `user` envelope (text +
    // optional image blocks). Sent by the reader task once the `initialize`
    // handshake is acknowledged. Shares build_user_envelope with steer injection.
    let user_line: Vec<u8> = build_user_envelope(&effective_prompt, &attachments)?;

    // Steer channel: register the sender while this turn streams so
    // `assistant_steer` can inject mid-turn user messages; the reader task owns
    // the receiver. Cleared at the same points as the session PID (turn end).
    let (steer_tx, mut steer_rx) = mpsc::unbounded_channel::<SteerMsg>();
    register_steer_tx(&session_id, steer_tx);

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
                            for m in steer_pending.drain(..) {
                                if let Ok(env) = build_user_envelope(&m.text, &m.attachments) {
                                    let _ = stdin.write_all(&env).await;
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
                            if res_is_err
                                && (res_text.contains("401")
                                    || res_text.contains("authentication_error")
                                    || res_text.contains("Invalid authentication")
                                    || res_text.contains("invalid x-api-key"))
                            {
                                let friendly = if current_api_key().is_some() {
                                    "Your configured API key was rejected (401). Clear it in Settings → CLI session to fall back to your `claude login`, or paste a valid key.".to_string()
                                } else {
                                    format!(
                                        "Authentication failed (401). Rift is using the Claude CLI at {} — sign in there by running `claude login` in a terminal, or switch installs in Settings → CLI session, then retry.",
                                        resolve_claude_exe().map(|p| p.display().to_string()).unwrap_or_else(|| "your active install".into())
                                    )
                                };
                                let _ = app_out.emit(ERROR_EVENT, serde_json::json!({
                                    "session_id": stream_sid, "message": friendly,
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
                clear_session_pid(&session_id);
                clear_steer_tx(&session_id);
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
    clear_session_pid(&session_id);
    clear_steer_tx(&session_id);
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
        } else if raw.contains("401")
            || raw.contains("authentication_error")
            || raw.contains("Invalid authentication")
            || raw.contains("invalid x-api-key")
        {
            // A rejected credential. The bare "claude exited with 1 — API Error:
            // 401" leaves the user with nothing to do; route them to the exact
            // field to fix based on which auth path is active.
            if current_api_key().is_some() {
                "Your configured API key was rejected (401). Clear it in Settings → CLI session to fall back to your `claude login`, or paste a valid key.".to_string()
            } else {
                "Authentication failed (401) — your `claude login` session was rejected or expired. Run `claude` in a terminal and sign in again, then retry.".to_string()
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_level_allowlist_and_resolution() {
        assert!(is_valid_trust_level("readonly"));
        assert!(is_valid_trust_level("standard"));
        assert!(is_valid_trust_level("full"));
        assert!(!is_valid_trust_level("admin"));
        assert!(!is_valid_trust_level(""));
        // Unset or garbage must floor to readonly — never escalate.
        assert_eq!(effective_trust_level(&None), "readonly");
        assert_eq!(effective_trust_level(&Some("garbage".into())), "readonly");
        assert_eq!(effective_trust_level(&Some("standard".into())), "standard");
    }

    #[test]
    fn permission_mode_allowlist_excludes_dontask() {
        for ok in ["default", "acceptEdits", "plan", "auto", "bypassPermissions"] {
            assert!(is_valid_permission_mode(ok), "{ok} should be valid");
        }
        // `dontAsk` is the CLI's auto-DENY mode — must NOT be exposed.
        assert!(!is_valid_permission_mode("dontAsk"));
        assert!(!is_valid_permission_mode(""));
    }
}
