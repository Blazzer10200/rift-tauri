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

pub mod mcp_server;
pub mod remote_bridge;

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// PID of the currently-streaming `claude` child, if any. Set on spawn,
/// cleared on exit. `assistant_stop` reads this to dispatch a kill — we use
/// PID + platform-native kill (taskkill on Win, SIGTERM on Unix) instead of
/// holding the `tokio::process::Child` across an await because the spawn
/// task owns the Child to call `.wait()` on it.
static CURRENT_CHILD_PID: Mutex<Option<u32>> = Mutex::new(None);
/// Set by `assistant_stop`, cleared on next spawn. Lets the wait-task tell
/// "user asked to stop" (emit done) apart from "CLI crashed silently w/ no
/// stderr" (emit error). Without this, a hung claude that exited 1 w/o any
/// stderr would silently look like a stop.
static USER_STOPPED: AtomicBool = AtomicBool::new(false);

fn set_current_pid(pid: Option<u32>) {
    if let Ok(mut g) = CURRENT_CHILD_PID.lock() {
        *g = pid;
    }
}

fn get_current_pid() -> Option<u32> {
    CURRENT_CHILD_PID.lock().ok().and_then(|g| *g)
}

/// Cached absolute path to the user's `claude` CLI. Windows' `Command::new`
/// does NOT apply PATHEXT lookup (no auto-append of `.cmd`/`.exe`), so we
/// resolve once via `where.exe claude` (or `which` on Unix) and reuse the
/// absolute path for all spawn sites. None = no CLI on PATH.
static CLAUDE_EXE: OnceLock<Option<PathBuf>> = OnceLock::new();

fn resolve_claude_exe() -> Option<PathBuf> {
    CLAUDE_EXE
        .get_or_init(|| {
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
        })
        .clone()
}

/// Build a `tokio::process::Command` for `claude`, hiding the console window
/// on Windows. Returns `None` if the CLI isn't on PATH.
fn claude_command() -> Option<Command> {
    let exe = resolve_claude_exe()?;
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    Some(cmd)
}

const STREAM_EVENT: &str = "assistant://stream";
const DONE_EVENT: &str = "assistant://done";
const ERROR_EVENT: &str = "assistant://error";
/// Emitted when claude returns "No conversation found with session ID" on a
/// --resume attempt. Payload `{session_id, prompt}`; frontend resets the
/// matching tab's convoCreatedAt and re-sends the prompt as a first-turn.
const SESSION_LOST_EVENT: &str = "assistant://session-lost";

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
    /// Plaintext API key. Keychain migration planned (see design brief).
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
}

fn convo_path(id: &str) -> Result<PathBuf, String> {
    // Guard against `..` / path separators in the id — only accept the
    // hex/uuid shape we generate (alphanumeric + dashes).
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!("invalid conversation id: {id}"));
    }
    Ok(conversations_dir()?.join(format!("{id}.json")))
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
        // Parse just enough to extract metadata; skip invalid files.
        let convo: Conversation = match serde_json::from_slice(&bytes) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let message_count = convo
            .messages
            .as_array()
            .map(|a| a.len() as u32)
            .unwrap_or(0);
        out.push(ConversationMeta {
            id: convo.id,
            title: convo.title,
            model: convo.model,
            message_count,
            created_at: convo.created_at,
            updated_at: convo.updated_at,
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
    match std::fs::remove_file(&p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("delete {}: {e}", p.display())),
    }
}

fn dirs_home() -> Result<PathBuf, String> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| "no USERPROFILE/HOME env var".to_string())
}

fn load_config() -> AssistantConfig {
    config_path()
        .and_then(|p| std::fs::read_to_string(&p).map_err(|e| e.to_string()))
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        .unwrap_or_default()
}

fn save_config(cfg: &AssistantConfig) -> Result<(), String> {
    let p = config_path()?;
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&p, s).map_err(|e| format!("write {}: {e}", p.display()))
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
fn write_mcp_config(
    roots: &[PathBuf],
    bridge: Option<&remote_bridge::BridgeInfo>,
    remote_shell_enabled: bool,
) -> Result<PathBuf, String> {
    let home = dirs_home()?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir ~/.rift/assistant: {e}"))?;
    let path = dir.join("mcp-config.json");

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let roots_joined = roots
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let mut env_map = serde_json::Map::new();
    env_map.insert("RIFT_MCP_SERVER".into(), Value::from("1"));
    env_map.insert("RIFT_MCP_ROOTS".into(), Value::from(roots_joined));
    if remote_shell_enabled {
        if let Some(b) = bridge {
            env_map.insert("RIFT_BRIDGE_PORT".into(), Value::from(b.port.to_string()));
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
    Ok(path)
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
    let cfg = load_config();
    out.api_key_configured = cfg.api_key.as_deref().map(|s| !s.is_empty()).unwrap_or(false);

    // `claude --version`. Resolve absolute path first — Windows can't find
    // `claude` from PATH alone (PATHEXT isn't applied by Command::new).
    let ver = match claude_command() {
        Some(mut c) => c.arg("--version").stdout(Stdio::piped()).stderr(Stdio::null()).output().await.ok(),
        None => None,
    };
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
    let auth = claude_command()
        .ok_or_else(|| "claude CLI not on PATH".to_string())?
        .args(["auth", "status"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .map_err(|e| format!("spawn `claude auth status`: {e}"))?;

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

#[tauri::command]
pub fn assistant_get_api_key() -> Result<Option<String>, String> {
    Ok(load_config().api_key.filter(|s| !s.is_empty()))
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
    let mut cfg = load_config();
    cfg.api_key = api_key.filter(|s| !s.is_empty());
    save_config(&cfg)
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
const RIFT_SYSTEM_ADDENDUM_TOOLS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app, working inside the user's open project folder (your working directory is already set to the workspace root, so relative paths Just Work). You have the full Claude Code toolset: Read / Write / Edit for files, Bash for shell commands (executes in the workspace dir, output streamed back), Glob for filename patterns, Grep for content search, WebFetch and WebSearch for the open web, and TodoWrite for multi-step plans. TodoWrite output surfaces in a dedicated Tasks panel in the user's UI — use it proactively whenever a request involves three or more distinct steps, and update statuses (pending → in_progress → completed) as you go. Rift's MCP server also exposes read_file / list_dir / grep as scoped helpers; prefer Claude Code built-ins for normal work and use the MCP variants only when a guaranteed-workspace-rooted path matters. Act directly — build it, run it, fix it. Don't ask the user for permission on routine work like file edits, shell commands, package installs, or git operations; the user expects you to do real work and can revert via git. Surface tool errors verbatim and try a different approach instead of bouncing the problem back to the user. Project stack is open-ended — do not assume the language, framework, or layout.";

const RIFT_SYSTEM_ADDENDUM_NO_WS: &str = "You are Rift's Assistant — a coding partner embedded in a Tauri desktop app. No project folder is open right now, so your file/list/grep tools are unavailable for this turn. Answer questions and discuss code the user pastes, but tell the user to open a folder on the Assistant page (the empty-state has an \"Open Folder\" button) if they want you to read their code directly. Do not claim capabilities you do not have.";

/// Build a single-line per-turn addendum that describes the live AutoSync /
/// LockPresence state — foreign locks held by other users, sync queue depth,
/// recent DiagBus stage events. Concatenated onto `RIFT_SYSTEM_ADDENDUM_TOOLS`
/// per turn so the model sees fresh multi-writer state. Returns an empty
/// string when no AutoSync engine is active (no signal worth surfacing).
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

    // Force single-line for the Rust 1.77 batch-arg validator (CVE-2024-24576).
    let joined = parts.join(" ");
    joined.replace('\n', " ").replace('\r', " ")
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
) -> Result<(), String> {
    let cfg = load_config();
    let use_api_key = cfg.api_key.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    let model = model.unwrap_or_else(|| "sonnet".to_string());

    // Workspace root resolution — priority order:
    //   1. The user's explicitly-opened folder (`current_root` in config).
    //      Set by `assistant_set_root`; cleared by `assistant_clear_root`.
    //      Matches VS Code's "open folder" model — one root at a time.
    //   2. AutoSync server folders if any are connected. Keeps the FiveM/RedM
    //      flow working unchanged when the user hasn't opened a folder.
    //   3. Empty → no-tools turn + no-workspace addendum.
    // Validate the configured root still exists; if it was deleted/moved
    // out-of-band, fall through to AutoSync rather than fail the turn.
    let roots: Vec<PathBuf> = if let Some(root) = cfg.current_root.as_ref().filter(|p| p.is_dir()) {
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

    // Remote-shell tool only fires when the user toggled it on AND the parent
    // can stand up the loopback bridge. Bridge `start` is idempotent — first
    // call binds the listener; later calls return the cached info.
    let allow_remote_shell = cfg.allow_remote_shell.unwrap_or(false);
    let bridge_info = if allow_remote_shell {
        match remote_bridge::start(app.clone()).await {
            Ok(info) => Some(info),
            Err(e) => {
                log::warn!("assistant: bridge start failed, remote_bash disabled: {e}");
                None
            }
        }
    } else {
        None
    };
    let remote_shell_enabled = allow_remote_shell && bridge_info.is_some();

    // Provision a temp MCP config when we have at least one root. Addendum is
    // a `String` (not &'static str) so we can append per-turn workspace state.
    let (mcp_config_path, mut addendum) = if roots.is_empty() {
        (None, RIFT_SYSTEM_ADDENDUM_NO_WS.to_string())
    } else {
        match write_mcp_config(&roots, bridge_info.as_ref(), remote_shell_enabled) {
            Ok(p) => (Some(p), RIFT_SYSTEM_ADDENDUM_TOOLS.to_string()),
            Err(e) => {
                log::warn!("assistant: failed to provision MCP config, falling back to no-tools: {e}");
                (None, RIFT_SYSTEM_ADDENDUM_NO_WS.to_string())
            }
        }
    };

    // S73: splice live multi-writer state onto the addendum. Single-line so
    // the .cmd-shim batch-arg validator (Rust 1.77+) accepts it.
    let ws_ctx = gather_workspace_context(&state).await;
    if !ws_ctx.is_empty() {
        addendum.push(' ');
        addendum.push_str(&ws_ctx);
    }
    if remote_shell_enabled {
        addendum.push_str(" A `mcp__rift__remote_bash` tool is available — runs a shell command over SSH against the active remote server (reuses the auto-sync engine's russh session). Use sparingly for ops work (status checks, pm2 restart, etc.); a workspace-scoped advisory lock serializes calls between users.");
    }

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
        let total: usize = attachments.iter().map(|a| a.data_base64.len() * 3 / 4).sum();
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
    let has_attachments = !attachments.is_empty();

    let mut cmd = claude_command()
        .ok_or_else(|| "claude CLI not on PATH — install Claude Code or configure an API key".to_string())?;
    cmd.arg("-p")
        .arg("--append-system-prompt").arg(addendum)
        .arg("--output-format").arg("stream-json")
        .arg("--input-format").arg(if has_attachments { "stream-json" } else { "text" })
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--model").arg(&model)
        .arg("--permission-mode").arg("dontAsk")
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
        // `MultiEdit`, `NotebookEdit`, `SlashCommand`, `AskUserQuestion`,
        // `ExitPlanMode`. Wider built-in coverage = fewer denial pop-ups.
        // MCP scope still restricts to rift's tools in the scoped branches.
        const BUILTINS: &str = "Agent,AskUserQuestion,Bash,BashOutput,Edit,ExitPlanMode,Glob,Grep,KillBash,KillShell,MultiEdit,NotebookEdit,Read,Skill,SlashCommand,TodoWrite,WebFetch,WebSearch,Write";
        let allowed: String = if use_full_config {
            // `mcp__*` admits any tool from user MCP servers that the CLI
            // merged in (no `--strict-mcp-config`). Rift's tools stay scoped
            // via the explicit-name entries.
            format!("{BUILTINS},mcp__*")
        } else if remote_shell_enabled {
            format!("{BUILTINS},mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep,mcp__rift__remote_bash")
        } else {
            format!("{BUILTINS},mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep")
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
        if let Some(k) = cfg.api_key.as_deref() {
            cmd.env("ANTHROPIC_API_KEY", k);
        }
    }

    // Enable extended thinking for models that support it (opus, sonnet on
    // 4.x+). Haiku skips silently. The plaintext reasoning is encrypted by
    // the API in -p mode — what reaches us is `content_block_start` of type
    // `thinking` + a `signature_delta` blob, with `thinking_delta` text
    // arriving in some scenarios. UI surfaces the indicator + duration even
    // when the text itself is redacted.
    if model != "haiku" {
        cmd.env("MAX_THINKING_TOKENS", "10000");
    }

    log::info!(
        "assistant_send: spawn session_id={} first_turn={} model={} use_full_config={} mcp={} api_key={} remote_shell={}",
        session_id, is_first_turn, model, use_full_config, mcp_config_path.is_some(), use_api_key, remote_shell_enabled
    );

    USER_STOPPED.store(false, Ordering::SeqCst);
    let mut child = cmd.spawn().map_err(|e| format!("spawn `claude`: {e}"))?;
    set_current_pid(child.id());

    // Write the user's prompt to stdin, then close it so the CLI knows the
    // input stream is complete and starts streaming back. With attachments,
    // serialize a stream-json `user` envelope (text + image blocks);
    // otherwise pipe the bare prompt for text input-format.
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let payload: Vec<u8> = if has_attachments {
            let mut content: Vec<Value> = Vec::with_capacity(1 + attachments.len());
            content.push(serde_json::json!({ "type": "text", "text": prompt }));
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
        } else {
            prompt.as_bytes().to_vec()
        };
        if let Err(e) = stdin.write_all(&payload).await {
            return Err(format!("write prompt to stdin: {e}"));
        }
        drop(stdin); // EOF
    }

    let stdout = child.stdout.take().ok_or_else(|| "claude stdout missing".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "claude stderr missing".to_string())?;

    let app_out = app.clone();
    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    // Forward raw NDJSON line as-is; frontend parses.
                    let _ = app_out.emit(STREAM_EVENT, trimmed);
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = app_out.emit(ERROR_EVENT, format!("stdout read error: {e}"));
                    break;
                }
            }
        }
    });

    // Drain stderr to a buffer for error-event surfacing on non-zero exit.
    let stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            buf.push_str(&l);
            buf.push('\n');
        }
        buf
    });

    let status = child.wait().await.map_err(|e| format!("await claude: {e}"))?;
    set_current_pid(None);
    let _ = stdout_task.await;
    let stderr_buf = stderr_task.await.unwrap_or_default();

    if status.success() {
        let _ = app.emit(DONE_EVENT, serde_json::json!({ "exit_code": 0 }));
        Ok(())
    } else if USER_STOPPED.swap(false, Ordering::SeqCst) {
        // User clicked Stop → assistant_stop killed the child. Emit done
        // (not error) so the UI clears the streaming flag and pops the
        // next queued message cleanly.
        let _ = app.emit(DONE_EVENT, serde_json::json!({ "exit_code": status.code().unwrap_or(-1) }));
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
            let _ = app.emit(
                SESSION_LOST_EVENT,
                serde_json::json!({
                    "session_id": session_id,
                    "prompt": prompt,
                }),
            );
            return Ok(());
        }
        let msg = format!(
            "claude exited with {} — {}",
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            stderr_buf.trim()
        );
        let _ = app.emit(ERROR_EVENT, &msg);
        Err(msg)
    }
}

/// Kill the currently-streaming `claude` child, if one is running.
/// Platform-native: taskkill /F /PID on Windows, SIGTERM via libc on Unix.
/// No-op (returns Ok) if no child is active.
#[tauri::command]
pub async fn assistant_stop() -> Result<(), String> {
    let Some(pid) = get_current_pid() else {
        return Ok(());
    };
    USER_STOPPED.store(true, Ordering::SeqCst);
    set_current_pid(None);

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
