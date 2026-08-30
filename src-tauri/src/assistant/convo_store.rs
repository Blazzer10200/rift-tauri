//! Conversation persistence; see `docs/ARCHITECTURE.md#backend-map`:
//! the on-disk convo JSON store (list/load/save/delete), the per-session
//! cwd + model sidecars, session-id validation, and the retired-JSONL
//! housekeeping sweep. Lifted verbatim from `assistant/mod.rs` 2026-06-09.
//! `dirs_home` + model-name validation stay on the parent, reached via `super::`.
//! Disk format is the contract — don't reshape any serialized field.

use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

use serde::{Deserialize, Serialize};

use super::{dirs_home, is_valid_model_name};

/// Serializes conversation saves so two windows saving the same convo id can't
/// race on a shared `.tmp` and silently install stale data. Mirrors
/// `config.rs::CONFIG_WRITE_LOCK`. Poison-recovered at the call site.
static CONVO_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// The session cwd is a security and correctness boundary: once a provider
/// session is associated with a workspace it must never move. Keep the
/// read/compare/create sequence in one critical section so two simultaneous
/// first turns cannot each decide that they own an unpinned session.
static SESSION_CWD_LOCK: Mutex<()> = Mutex::new(());
static SIDECAR_TMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A filesystem-level companion to `SESSION_CWD_LOCK`. The mutex protects
/// threads in this Rift process; this handle protects independent Rift
/// processes that share the same `.rift` data directory. On Windows an open
/// file with `share_mode(0)` is an exclusive OS lock and is released by the OS
/// even if its owning process crashes. The lock file deliberately remains on
/// disk after the handle closes: deleting it would let a second process create
/// a different file while another process still holds the old one.
struct SessionCwdFileLock {
    #[cfg(windows)]
    _file: std::fs::File,
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
    /// Real content activity (send / turn result). Every tab-switch save bumps
    /// `updated_at`, so relative-time labels must read THIS — omitting it from
    /// the list DTO is why every "Jump back in" row said "just now" (v0.131.0).
    /// Rides the record's serde-flatten catch-all on save; None for legacy
    /// records (frontend falls back to `updated_at`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity_at: Option<i64>,
    /// Σ of per-turn costs across the transcript (sum of messages[].costUsd).
    /// Matches the live session counter; 0.0 for convos predating cost capture.
    pub cost_usd: f64,
    /// Phase E5: flattened compaction summaries so HistoryDrawer search can
    /// match against the contents of long-running compacted convos without
    /// loading every transcript. Empty for convos that never compacted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compaction_summaries: Vec<String>,
    /// One-line preview of the newest text message (ui-audit #6) — feeds the
    /// Home "Jump back in" rows. None when no text block exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_snippet: Option<String>,
    /// Per-project scoping: the workspace folder this conversation belongs to,
    /// so the sidebar can show only the open project's chats. Sourced from the
    /// convo's own `workspace_root` (new convos), else backfilled from the
    /// session-cwd sidecar (legacy convos predating the field). None = unfiled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
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
    /// ChatGPT transport pinned by the first GPT turn (`codex` subscription or
    /// separately billed `openai` API). Optional for legacy conversations;
    /// the frontend infers those from provider-owned continuation state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_gpt_route: Option<String>,
    /// Per-project scoping: the workspace folder active when this conversation's
    /// turns run. Stamped by the frontend on save (`tab.workspaceRoot ?? activeRoot`).
    /// Legacy convos lack it (None) — the list backfills from the cwd sidecar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    /// Catch-all for conversation-level fields the frontend owns but Rust does
    /// not model — `forceNextFirstTurn` (F51) and `compactionHistory` (the
    /// latent round-trip bug). Without flatten, `from_value` → `to_string` on
    /// every save silently dropped them, so a post-restart load lost compaction
    /// state and re-sent `--resume` against a non-existent JSONL.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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
/// between turns (folder swap, root vanishes), the
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
pub(super) fn is_valid_session_id(s: &str) -> bool {
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

fn sidecar_tmp_path(path: &Path, extension: &str) -> PathBuf {
    let sequence = SIDECAR_TMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("{extension}.{}.{}.tmp", std::process::id(), sequence))
}

fn session_cwd_lock_path(path: &Path) -> PathBuf {
    path.with_extension("cwd.lock")
}

#[cfg(windows)]
fn acquire_session_cwd_file_lock(sidecar_path: &Path) -> Result<SessionCwdFileLock, String> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::{Duration, Instant};

    let lock_path = session_cwd_lock_path(sidecar_path);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        // `share_mode(0)` denies every sharing mode. Unlike create_new lock
        // files, it cannot become permanently stale: Windows closes the handle
        // during process teardown. It also works when the lock file already
        // exists from a prior successful turn.
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .open(&lock_path)
        {
            Ok(file) => return Ok(SessionCwdFileLock { _file: file }),
            // Windows reports ERROR_SHARING_VIOLATION / ERROR_LOCK_VIOLATION
            // as `Other` on some toolchains, so inspect the OS code as well as
            // the portable kinds before deciding this is a terminal failure.
            Err(error)
                if (matches!(
                    error.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::WouldBlock
                ) || matches!(error.raw_os_error(), Some(32 | 33)))
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return Err(format!(
                    "lock session workspace {}: {error}",
                    lock_path.display()
                ));
            }
        }
    }
}

// Rift currently targets Windows. Keep non-Windows test/dev builds usable; the
// in-process mutex above still protects their supported single-process mode.
#[cfg(not(windows))]
fn acquire_session_cwd_file_lock(_sidecar_path: &Path) -> Result<SessionCwdFileLock, String> {
    Ok(SessionCwdFileLock {})
}

fn write_session_cwd(path: &Path, cwd: &Path) -> Result<(), String> {
    let tmp = sidecar_tmp_path(path, "cwd");
    std::fs::write(&tmp, cwd.to_string_lossy().as_bytes())
        .map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("rename {}: {e}", path.display())
    })
}

pub(super) fn load_session_cwd(id: &str) -> Option<PathBuf> {
    let p = session_cwd_path(id).ok()?;
    let s = std::fs::read_to_string(&p).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

/// Compare a persisted conversation's declared workspace to its immutable
/// provider-session sidecar. A conversation JSON is user-local data and can
/// survive partial writes, manual edits, or an older Rift build; it must not be
/// trusted to re-home a session that was already pinned elsewhere.
///
/// Missing roots remain valid so a moved/deleted project can be reported to the
/// user as unavailable rather than being silently reassigned. When both paths
/// exist we canonicalize before comparing. If either disappeared, only an
/// equivalent Windows path spelling is accepted; any disagreement fails closed.
fn validate_conversation_session_binding(convo: &Conversation) -> Result<(), String> {
    let Some(declared) = convo.workspace_root.as_deref() else {
        return Ok(());
    };
    let session_id = convo.cli_session_id.as_deref().unwrap_or(&convo.id);
    let Some(pinned) = load_session_cwd(session_id) else {
        return Ok(());
    };

    let declared_path = PathBuf::from(declared.trim());
    let matches = if declared_path.is_dir() && pinned.is_dir() {
        super::canonicalize_root(declared_path.clone())?
            == super::canonicalize_root(pinned.clone())?
    } else {
        // `PathBuf` equality is case-sensitive, unlike Windows workspace
        // identity. Normalize the only spelling differences we can safely
        // resolve without touching a missing folder; do not collapse `..` or
        // otherwise guess at a moved directory.
        declared_path
            .to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&pinned.to_string_lossy().replace('/', "\\"))
    };
    if matches {
        Ok(())
    } else {
        Err(format!(
            "conversation workspace conflicts with its pinned session: record declares {}, session is bound to {}",
            declared_path.display(),
            pinned.display()
        ))
    }
}

/// Resolve a renderer-supplied workspace for a turn without ever consulting the
/// mutable application-wide `current_root`. A rooted session is immutable: a
/// later turn may name the same canonical folder, but may not silently move an
/// existing provider session into another project.
///
/// `None` on a new session is deliberately a rootless/local turn. It never means
/// "use whichever project happened to be selected last". Once a session has a
/// sidecar, omission keeps that immutable pin; this is required for no-folder
/// Claude turns whose first turn is assigned Rift's local scratch directory.
pub(super) fn resolve_session_workspace(id: &str, requested: Option<&str>) -> Result<Option<PathBuf>, String> {
    let requested = match requested.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => {
            let raw = PathBuf::from(value);
            if !raw.is_dir() {
                return Err(format!("workspace folder does not exist: {}", raw.display()));
            }
            Some(super::canonicalize_root(raw)?)
        }
        None => None,
    };
    resolve_or_establish_session_workspace(id, requested)
}

/// Establish a workspace pin for a locally-created scratch turn. This shares
/// the exact same atomic compare-and-establish path as renderer-supplied roots.
pub(super) fn establish_session_workspace(id: &str, requested: &Path) -> Result<PathBuf, String> {
    if !requested.is_dir() {
        return Err(format!("workspace folder does not exist: {}", requested.display()));
    }
    let requested = super::canonicalize_root(requested.to_path_buf())?;
    resolve_or_establish_session_workspace(id, Some(requested))?
        .ok_or_else(|| "workspace pin unexpectedly resolved empty".to_string())
}

fn resolve_or_establish_session_workspace(
    id: &str,
    requested: Option<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let _guard = SESSION_CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = session_cwd_path(id)?;
    let _file_lock = acquire_session_cwd_file_lock(&path)?;
    let pinned = match load_session_cwd(id) {
        Some(path) if !path.is_dir() => {
            return Err(format!(
                "conversation workspace is unavailable: {}",
                path.display()
            ));
        }
        Some(path) => Some(super::canonicalize_root(path)?),
        None => None,
    };
    match (pinned, requested) {
        (Some(pinned), Some(requested)) if pinned != requested => Err(format!(
            "conversation workspace mismatch: this session is bound to {}, not {}",
            pinned.display(),
            requested.display()
        )),
        (Some(pinned), Some(_)) => Ok(Some(pinned)),
        (Some(pinned), None) => Ok(Some(pinned)),
        (None, Some(requested)) => {
            write_session_cwd(&path, &requested)?;
            Ok(Some(requested))
        }
        (None, None) => Ok(None),
    }
}

fn delete_session_cwd(id: &str) {
    if let Ok(p) = session_cwd_path(id) {
        let _ = std::fs::remove_file(&p);
    }
}

/// Sidecar that pins the MODEL a conversation currently runs on. Originally a
/// hard guard against the model-bound thinking-signature 400 wedge (resuming a
/// session whose transcript held `thinking` + `tool_use` blocks signed by a
/// different model permanently wedged it) — the CLI now sanitizes cross-model
/// thinking blocks on resume (verified on 2.1.204, 2026-07-07), so a mid-chat
/// picker switch is honored and RE-pins this sidecar (turn.rs assistant_send).
/// The pin still does real work: it keeps the exact resolved id stable across
/// resumes when the picker selection hasn't changed (alias-stable — a legacy
/// `sonnet` pin stays on claude-sonnet-4-6), and it's the fallback target when
/// a send arrives without an explicit model.
fn session_model_path(id: &str) -> Result<PathBuf, String> {
    Ok(session_cwd_path(id)?.with_extension("model"))
}

pub(super) fn save_session_model(id: &str, model: &str) {
    if let Ok(p) = session_model_path(id) {
        // Per-writer tmp name — same cross-window race as save_session_cwd; a
        // torn model pin risks the documented resume-wedge (model-bound thinking
        // signatures). Atomic rename after a whole-value write.
        let tmp = sidecar_tmp_path(&p, "model");
        if let Err(e) = std::fs::write(&tmp, model.as_bytes()) {
            log::warn!("assistant: save session model {}: {e}", p.display());
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &p) {
            let _ = std::fs::remove_file(&tmp);
            log::warn!("assistant: save session model rename {}: {e}", p.display());
        }
    }
}

pub(super) fn load_session_model(id: &str) -> Option<String> {
    let p = session_model_path(id).ok()?;
    let s = std::fs::read_to_string(&p).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() || !is_valid_model_name(trimmed) {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn delete_session_model(id: &str) {
    if let Ok(p) = session_model_path(id) {
        let _ = std::fs::remove_file(&p);
    }
}

#[tauri::command]
pub async fn assistant_list_conversations() -> Result<Vec<ConversationMeta>, String> {
    // RR10: the directory scan + per-file JSON parse (potentially hundreds of
    // convos) is blocking I/O — keep it off the Tokio worker pool.
    tokio::task::spawn_blocking(list_conversations_sync)
        .await
        .map_err(|e| format!("list_conversations join error: {e}"))?
}

fn list_conversations_sync() -> Result<Vec<ConversationMeta>, String> {
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
            Err(e) => {
                // Fail loud-ish: a convo silently vanishing from the list is
                // indistinguishable from data loss — leave a trail.
                log::warn!("convo list: skipping unparseable {} — {e}", p.display());
                continue;
            }
        };
        // Extract the Value-only fields BEFORE the typed parse consumes `raw`,
        // so we deserialize once without cloning the whole Value per convo.
        let last_activity_at = raw.get("lastActivityAt").and_then(|v| v.as_i64());
        let compaction_summaries = raw
            .get("compactionHistory")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("summary").and_then(|s| s.as_str()).map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let convo: Conversation = match serde_json::from_value(raw) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("convo list: skipping malformed {} — {e}", p.display());
                continue;
            }
        };
        let message_count = convo
            .messages
            .as_array()
            .map(|a| a.len() as u32)
            .unwrap_or(0);
        let cost_usd = convo
            .messages
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("costUsd").and_then(|v| v.as_f64()))
                    .sum::<f64>()
            })
            .unwrap_or(0.0);
        let last_snippet = convo.messages.as_array().and_then(|arr| {
            arr.iter().rev().find_map(|m| {
                if m.get("role").and_then(|r| r.as_str()) == Some("system") {
                    return None;
                }
                m.get("blocks")?.as_array()?.iter().find_map(|b| {
                    if b.get("type").and_then(|t| t.as_str()) != Some("text") {
                        return None;
                    }
                    let t = b.get("text")?.as_str()?;
                    // RR10: only the first ~120 chars are kept — bound the input
                    // before the whitespace-collapse allocates (a multi-MB text
                    // block would otherwise be flattened in full just to slice 120).
                    let head = if t.len() > 2048 {
                        match t.char_indices().nth(512) {
                            Some((byte_idx, _)) => &t[..byte_idx],
                            None => t,
                        }
                    } else {
                        t
                    };
                    let flat = head.split_whitespace().collect::<Vec<_>>().join(" ");
                    if flat.is_empty() {
                        return None;
                    }
                    let mut s: String = flat.chars().take(120).collect();
                    if flat.chars().count() > 120 {
                        s.push('…');
                    }
                    Some(s)
                })
            })
        });
        // Per-project scope: prefer the convo's own root; else backfill from the
        // session-cwd sidecar (keyed by the cli session id, falling back to the
        // convo id) so chats predating the field still land in their folder.
        let workspace_root = convo.workspace_root.clone().or_else(|| {
            let sid = convo.cli_session_id.as_deref().unwrap_or(&convo.id);
            load_session_cwd(sid).map(|p| p.to_string_lossy().into_owned())
        });
        out.push(ConversationMeta {
            id: convo.id,
            title: convo.title,
            model: convo.model,
            message_count,
            created_at: convo.created_at,
            updated_at: convo.updated_at,
            last_activity_at,
            cost_usd,
            compaction_summaries,
            last_snippet,
            workspace_root,
        });
    }
    out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
    Ok(out)
}

/// Lightweight per-conversation summary for the Home stats dashboard. One row
/// per saved transcript; all day/hour bucketing happens frontend-side in LOCAL
/// time (timezone-correct active-days, streaks, peak hour). Block-level counting
/// (tool calls, words) stays in Rust so the frontend never loads full
/// transcripts over IPC just to tally them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvoStat {
    pub updated_at: i64,
    pub created_at: i64,
    pub model: String,
    /// user + assistant messages (system/boundary rows excluded).
    pub messages: u32,
    pub user_messages: u32,
    /// tool-use blocks across the transcript — Rift's agentic-activity metric.
    pub tool_calls: u32,
    /// whitespace-delimited word count across text blocks (both roles).
    pub words: u32,
    /// Σ of per-message costUsd — accurate; 0 for convos predating cost capture.
    pub cost_usd: f64,
}

/// Scan every saved conversation and return per-convo summaries for the Home
/// dashboard. Cheap DTO (no transcript bodies) so the frontend can aggregate
/// totals, per-model breakdowns, and the activity heatmap without re-reading
/// disk. Unparseable files are skipped, mirroring `assistant_list_conversations`.
#[tauri::command]
pub async fn assistant_stats() -> Result<Vec<ConvoStat>, String> {
    // Same blocking dir-scan + per-file parse as list_conversations (potentially
    // hundreds of convos) — keep it off the Tokio worker pool.
    tokio::task::spawn_blocking(assistant_stats_sync)
        .await
        .map_err(|e| format!("assistant_stats join error: {e}"))?
}

fn assistant_stats_sync() -> Result<Vec<ConvoStat>, String> {
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
        let convo: Conversation = match serde_json::from_slice(&bytes) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let Some(arr) = convo.messages.as_array() else {
            continue;
        };
        let mut s = ConvoStat {
            updated_at: convo.updated_at,
            created_at: convo.created_at,
            model: convo.model,
            messages: 0,
            user_messages: 0,
            tool_calls: 0,
            words: 0,
            cost_usd: 0.0,
        };
        for m in arr {
            match m.get("role").and_then(|r| r.as_str()) {
                Some("user") => s.user_messages += 1,
                Some("assistant") => {}
                _ => continue, // skip system / boundary rows
            }
            s.messages += 1;
            if let Some(c) = m.get("costUsd").and_then(|v| v.as_f64()) {
                s.cost_usd += c;
            }
            if let Some(blocks) = m.get("blocks").and_then(|b| b.as_array()) {
                for b in blocks {
                    match b.get("type").and_then(|t| t.as_str()) {
                        Some("tool") => s.tool_calls += 1,
                        Some("text") => {
                            if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                                s.words += t.split_whitespace().count() as u32;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        out.push(s);
    }
    Ok(out)
}

#[tauri::command]
pub async fn assistant_load_conversation(id: String) -> Result<Conversation, String> {
    // RR10: large-transcript read + parse is blocking I/O — off the Tokio worker,
    // same as list/stats/save.
    tokio::task::spawn_blocking(move || {
        let p = convo_path(&id)?;
        let bytes = std::fs::read(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
        let convo: Conversation = serde_json::from_slice(&bytes)
            .map_err(|e| format!("parse conversation: {e}"))?;
        validate_conversation(&convo)?;
        if convo.id != id {
            return Err("conversation record id does not match its filename".into());
        }
        Ok(convo)
    })
    .await
    .map_err(|e| format!("load_conversation join error: {e}"))?
}

/// #30: expose a session's pinned cwd so the UI can flag a resumed tab that
/// operates in a different folder than the currently selected workspace.
#[tauri::command]
pub fn assistant_session_cwd(id: String) -> Option<String> {
    load_session_cwd(&id).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn assistant_save_conversation(convo: Conversation) -> Result<(), String> {
    // RR10: serialize + tmp-write + rename is blocking I/O — off the Tokio worker.
    // The std CONVO_WRITE_LOCK is acquired+released entirely inside the closure,
    // never held across an await.
    tokio::task::spawn_blocking(move || save_conversation_sync(&convo))
        .await
        .map_err(|e| format!("save_conversation join error: {e}"))?
}

fn save_conversation_sync(convo: &Conversation) -> Result<(), String> {
    validate_conversation(convo)?;
    let p = convo_path(&convo.id)?;
    let s = serde_json::to_string(convo).map_err(|e| e.to_string())?;
    // Serialize saves (multi-window can race the same id) + per-call tmp suffix
    // so concurrent writers never clobber a shared .tmp. Atomic-ish: write tmp
    // then rename so a crash mid-write leaves no half-truncated transcript.
    let _guard = CONVO_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = p.with_extension(format!("json.tmp.{}", std::process::id()));
    std::fs::write(&tmp, s).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &p).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("rename {}: {e}", p.display())
    })?;
    Ok(())
}

fn validate_conversation(convo: &Conversation) -> Result<(), String> {
    let _ = convo_path(&convo.id)?;
    if let Some(session_id) = convo.cli_session_id.as_deref() {
        if !is_valid_session_id(session_id) {
            return Err("invalid CLI session id".into());
        }
    }
    if let Some(route) = convo.chat_gpt_route.as_deref() {
        if !matches!(route, "codex" | "openai") {
            return Err("invalid ChatGPT route".into());
        }
    }
    if let Some(root) = convo.workspace_root.as_deref() {
        let root = root.trim();
        if root.is_empty() || root.eq_ignore_ascii_case("all") || !Path::new(root).is_absolute() {
            return Err("workspaceRoot must be an absolute workspace path, not All".into());
        }
    }
    validate_conversation_session_binding(convo)?;
    Ok(())
}

#[tauri::command]
pub fn assistant_delete_conversation(id: String) -> Result<(), String> {
    let p = convo_path(&id)?;
    // #114: load BEFORE delete so we can find the decoupled `cli_session_id`
    // (post-S103 these can differ from the Rift convo id after a compaction).
    // Without this, the cwd sidecar under the cli session UUID never gets
    // cleaned up — orphan accumulation under `~/.rift/assistant/sessions/`.
    // Hold CONVO_WRITE_LOCK across read+delete so an in-flight save's
    // write-tmp→rename can't land between our remove and return, silently
    // resurrecting the deleted convo on disk.
    let (cli_session_id, retired) = {
        let _guard = CONVO_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let raw = std::fs::read_to_string(&p).ok();
        let cli_session_id = raw
            .as_deref()
            .and_then(|s| serde_json::from_str::<Conversation>(s).ok())
            .and_then(|c| c.cli_session_id);
        // #46: collect this convo's compaction-retired session ids BEFORE the
        // record is gone — cleanup_retired_jsonls rebuilds its retired-set from
        // SURVIVING convos only, so after this delete it could never learn these
        // ids again and their CLI JSONLs would leak forever.
        let retired: std::collections::HashSet<String> = raw
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| {
                v.get("compactionHistory").and_then(|a| a.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|e| e.get("priorSessionId").and_then(|s| s.as_str()))
                        .filter(|sid| is_valid_session_id(sid))
                        .map(str::to_string)
                        .collect()
                })
            })
            .unwrap_or_default();
        match std::fs::remove_file(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("delete {}: {e}", p.display())),
        }
        (cli_session_id, retired)
    };
    // Delete sidecars only after the convo record is gone — a half-deleted
    // convo with intact sidecars is recoverable; a deleted sidecar with a
    // surviving convo would silently lose its pinned cwd.
    delete_session_cwd(&id);
    delete_session_model(&id);
    if let Some(cli_id) = cli_session_id {
        if cli_id != id {
            delete_session_cwd(&cli_id);
            delete_session_model(&cli_id);
        }
    }
    // #46: sweep the compaction-retired CLI JSONLs now — an explicit user
    // delete needs no 30-day grace (unlike the conservative startup sweep).
    // The convo's own LIVE session JSONL is deliberately left alone: it's the
    // only remaining artifact a user could still `claude --resume` for
    // recovery, and the retired-set sweep never touches non-Rift sessions.
    if !retired.is_empty() {
        let n = delete_project_jsonls(&retired, None);
        if n > 0 {
            log::info!("assistant: convo delete swept {n} retired JSONL(s)");
        }
    }
    Ok(())
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

    // Step 2: walk ~/.claude/projects/<cwd-hash>/*.jsonl and delete matches
    // older than 30 days. (checked_sub underflow ⇒ no cutoff ⇒ delete nothing,
    // preserving the sweep's conservative bias.)
    let Some(cutoff) = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(30 * 24 * 60 * 60))
    else {
        return 0;
    };
    delete_project_jsonls(&retired, Some(cutoff))
}

/// Walk `~/.claude/projects/<cwd-hash>/*.jsonl` and delete files whose stem is
/// in `retired`. `cutoff`: only delete files modified BEFORE it; `None` = no
/// age guard (the explicit convo-delete sweep, #46 — the user just deleted the
/// convo, so its retired JSONLs need no grace window). Best-effort; errors are
/// logged + swallowed. Returns the number of files deleted.
fn delete_project_jsonls(
    retired: &std::collections::HashSet<String>,
    cutoff: Option<std::time::SystemTime>,
) -> usize {
    let Ok(home) = dirs_home() else { return 0 };
    let projects = home.join(".claude").join("projects");
    let Ok(project_dirs) = std::fs::read_dir(&projects) else { return 0 };
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
            if let Some(c) = cutoff {
                let aged = f
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .map(|mt| mt < c)
                    .unwrap_or(false);
                if !aged {
                    continue;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_gpt_route_round_trips_and_remains_optional_for_legacy_records() {
        let raw = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Pinned route",
            "model": "gpt-5.6-sol",
            "createdAt": 1,
            "updatedAt": 2,
            "messages": [],
            "chatGptRoute": "codex",
            "futureField": true
        });
        let convo: Conversation = serde_json::from_value(raw).expect("route record parses");
        assert_eq!(convo.chat_gpt_route.as_deref(), Some("codex"));
        assert_eq!(convo.extra.get("futureField"), Some(&serde_json::Value::Bool(true)));

        let saved = serde_json::to_value(&convo).expect("route record serializes");
        assert_eq!(saved.get("chatGptRoute").and_then(|v| v.as_str()), Some("codex"));

        let legacy: Conversation = serde_json::from_value(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "title": "Legacy",
            "model": "gpt-5.6-sol",
            "createdAt": 1,
            "updatedAt": 2,
            "messages": []
        }))
        .expect("legacy route-less record parses");
        assert_eq!(legacy.chat_gpt_route, None);
    }

    #[test]
    fn convo_path_rejects_traversal_and_separators() {
        // The charset/length guard must fire before any path join, so these
        // never reach disk. Path-traversal + separator + null-ish inputs all
        // rejected; only the hex/uuid-dash shape is accepted.
        for bad in [
            "",
            "..",
            "../etc/passwd",
            "a/b",
            "a\\b",
            "id.json",       // dot not allowed
            "id with space",
            "héllo",          // non-ascii
            &"a".repeat(65),  // over the 64-char cap
        ] {
            assert!(convo_path(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn convo_path_accepts_uuid_shape_and_lands_in_dir() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let p = convo_path(id).expect("valid uuid id accepted");
        // Confinement: the resolved file is exactly <id>.json inside the
        // conversations dir — no traversal out of it.
        assert_eq!(
            p.file_name().and_then(|s| s.to_str()),
            Some("550e8400-e29b-41d4-a716-446655440000.json")
        );
        assert_eq!(p.parent(), Some(conversations_dir().unwrap().as_path()));
    }

    #[test]
    fn session_cwd_path_applies_same_guard() {
        assert!(session_cwd_path("../escape").is_err());
        assert!(session_cwd_path("a/b").is_err());
        assert!(session_cwd_path(&"x".repeat(65)).is_err());
        // A clean uuid passes the guard.
        assert!(session_cwd_path("550e8400-e29b-41d4-a716-446655440000").is_ok());
    }

    #[test]
    fn is_valid_session_id_enforces_uuid_shape() {
        // Canonical 8-4-4-4-12 hex, both cases.
        assert!(is_valid_session_id("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_session_id("550E8400-E29B-41D4-A716-446655440000"));
        // Wrong length, missing/displaced hyphens, non-hex, traversal bytes.
        assert!(!is_valid_session_id(""));
        assert!(!is_valid_session_id("550e8400e29b41d4a716446655440000")); // no hyphens
        assert!(!is_valid_session_id("550e8400-e29b-41d4-a716-44665544000")); // 35 chars
        assert!(!is_valid_session_id("550e8400-e29b-41d4-a716-4466554400000")); // 37 chars
        assert!(!is_valid_session_id("550e8400xe29bx41d4xa716x446655440000")); // hyphens→x
        assert!(!is_valid_session_id("g50e8400-e29b-41d4-a716-446655440000")); // non-hex
        assert!(!is_valid_session_id("../0e8400-e29b-41d4-a716-446655440000")); // traversal
    }

    #[test]
    fn session_workspace_is_pinned_and_never_falls_back_to_current_workspace() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let id = "550e8400-e29b-41d4-a716-4466554400a1";
        let first_root = super::super::canonicalize_root(first.path().to_path_buf()).unwrap();
        delete_session_cwd(id);
        assert_eq!(
            resolve_session_workspace(id, Some(first.path().to_str().unwrap())).unwrap(),
            Some(first_root.clone())
        );

        assert_eq!(
            resolve_session_workspace(id, Some(first.path().to_str().unwrap())).unwrap(),
            Some(first_root.clone())
        );
        assert!(resolve_session_workspace(id, Some(second.path().to_str().unwrap())).is_err());
        assert_eq!(resolve_session_workspace(id, None).unwrap(), Some(first_root));
        assert!(resolve_session_workspace(id, Some("definitely-not-a-workspace")).is_err());
        delete_session_cwd(id);
    }

    #[test]
    fn concurrent_first_turns_cannot_establish_different_workspaces() {
        use std::sync::{Arc, Barrier};

        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let id = "550e8400-e29b-41d4-a716-4466554400b1";
        delete_session_cwd(id);

        let barrier = Arc::new(Barrier::new(3));
        let roots = [
            first.path().to_string_lossy().into_owned(),
            second.path().to_string_lossy().into_owned(),
        ];
        let mut workers = Vec::new();
        for root in roots {
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                resolve_session_workspace(id, Some(&root))
            }));
        }
        barrier.wait();
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().expect("workspace worker panicked"))
            .collect();

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        let pinned = load_session_cwd(id).expect("first successful turn pinned a workspace");
        let successful_root = results
            .into_iter()
            .find_map(Result::ok)
            .expect("one turn succeeds");
        assert_eq!(successful_root, Some(pinned));
        delete_session_cwd(id);
    }

    #[cfg(windows)]
    #[test]
    fn session_workspace_file_lock_excludes_another_opener_until_released() {
        use std::sync::mpsc;
        use std::time::Duration;

        let id = "550e8400-e29b-41d4-a716-4466554400b2";
        let sidecar = session_cwd_path(id).unwrap();
        let held = acquire_session_cwd_file_lock(&sidecar).expect("first process lock");
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let worker_sidecar = sidecar.clone();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let lock = acquire_session_cwd_file_lock(&worker_sidecar);
            acquired_tx.send(lock.is_ok()).unwrap();
        });

        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        // This is the same kernel sharing primitive used across processes, so
        // a second opener must remain blocked while the first handle is live.
        assert!(acquired_rx.recv_timeout(Duration::from_millis(80)).is_err());
        drop(held);
        assert!(acquired_rx.recv_timeout(Duration::from_secs(2)).unwrap());
        worker.join().unwrap();
        delete_session_cwd(id);
    }

    #[test]
    fn sidecar_temp_paths_are_unique_within_a_process() {
        let path = std::env::temp_dir().join("rift-session-pin.cw");
        let first = sidecar_tmp_path(&path, "cwd");
        let second = sidecar_tmp_path(&path, "cwd");
        assert_ne!(first, second);
    }

    #[test]
    fn conversation_validation_allows_moved_roots_but_rejects_ambiguous_identity() {
        let valid = Conversation {
            id: "550e8400-e29b-41d4-a716-4466554400a2".into(),
            title: "Moved workspace".into(),
            model: "gpt-5.6".into(),
            created_at: 0,
            updated_at: 0,
            messages: serde_json::json!([]),
            cli_session_id: Some("550e8400-e29b-41d4-a716-4466554400a3".into()),
            chat_gpt_route: Some("codex".into()),
            workspace_root: Some(
                std::env::temp_dir()
                    .join("rift-moved-workspace")
                    .to_string_lossy()
                    .into_owned(),
            ),
            extra: serde_json::Map::new(),
        };
        assert!(validate_conversation(&valid).is_ok());

        let mut invalid = valid.clone();
        invalid.workspace_root = Some("All".into());
        assert!(validate_conversation(&invalid).is_err());
        invalid.workspace_root = Some("relative/workspace".into());
        assert!(validate_conversation(&invalid).is_err());
        invalid.workspace_root = valid.workspace_root.clone();
        invalid.chat_gpt_route = Some("unknown".into());
        assert!(validate_conversation(&invalid).is_err());
        invalid.chat_gpt_route = valid.chat_gpt_route.clone();
        invalid.cli_session_id = Some("not-a-session".into());
        assert!(validate_conversation(&invalid).is_err());
    }

    #[test]
    fn conversation_rejects_workspace_conflicting_with_session_pin() {
        let pinned = tempfile::tempdir().unwrap();
        let conflicting = tempfile::tempdir().unwrap();
        let session_id = "550e8400-e29b-41d4-a716-4466554400c1";
        delete_session_cwd(session_id);
        let pinned_root = establish_session_workspace(session_id, pinned.path()).unwrap();

        let convo = Conversation {
            id: "550e8400-e29b-41d4-a716-4466554400c2".into(),
            title: "Wrong project".into(),
            model: "gpt-5.6".into(),
            created_at: 0,
            updated_at: 0,
            messages: serde_json::json!([]),
            cli_session_id: Some(session_id.into()),
            chat_gpt_route: Some("codex".into()),
            workspace_root: Some(conflicting.path().to_string_lossy().into_owned()),
            extra: serde_json::Map::new(),
        };
        assert!(validate_conversation(&convo).is_err());

        let mut matching = convo;
        matching.workspace_root = Some(pinned_root.to_string_lossy().into_owned());
        assert!(validate_conversation(&matching).is_ok());
        delete_session_cwd(session_id);
    }
}
