//! R3 (per `docs/design/assistant-mod-split.md`) — conversation persistence:
//! the on-disk convo JSON store (list/load/save/delete/export), the per-session
//! cwd + model sidecars, session-id validation, and the retired-JSONL
//! housekeeping sweep. Lifted verbatim from `assistant/mod.rs` 2026-06-09.
//! `dirs_home` + model-name validation stay on the parent, reached via `super::`.
//! Disk format is the contract — don't reshape any serialized field.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::{dirs_home, is_valid_model_name};

/// Serializes conversation saves so two windows saving the same convo id can't
/// race on a shared `.tmp` and silently install stale data. Mirrors
/// `config.rs::CONFIG_WRITE_LOCK`. Poison-recovered at the call site.
static CONVO_WRITE_LOCK: Mutex<()> = Mutex::new(());

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

pub(super) fn save_session_cwd(id: &str, cwd: &Path) {
    if let Ok(p) = session_cwd_path(id) {
        let s = cwd.to_string_lossy();
        if let Err(e) = std::fs::write(&p, s.as_bytes()) {
            log::warn!("assistant: save session cwd {}: {e}", p.display());
        }
    }
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

fn delete_session_cwd(id: &str) {
    if let Ok(p) = session_cwd_path(id) {
        let _ = std::fs::remove_file(&p);
    }
}

/// Sidecar that pins the MODEL a conversation was started with. Extended-thinking
/// blocks carry a model-bound cryptographic signature; when an assistant turn
/// emits `thinking` + `tool_use`, every later `--resume` replays that message to
/// the API. If the resume goes out under a different model (picker switched
/// mid-chat — Opus↔Sonnet, or worst-case →Haiku which drops `--effort` and flips
/// thinking off), the API rejects the replayed blocks with `400 ... thinking ...
/// blocks ... cannot be modified` and the conversation is permanently wedged.
/// Pinning the model per session keeps resume aimed at the model that signed the
/// blocks; switching models is a new conversation.
fn session_model_path(id: &str) -> Result<PathBuf, String> {
    Ok(session_cwd_path(id)?.with_extension("model"))
}

pub(super) fn save_session_model(id: &str, model: &str) {
    if let Ok(p) = session_model_path(id) {
        if let Err(e) = std::fs::write(&p, model.as_bytes()) {
            log::warn!("assistant: save session model {}: {e}", p.display());
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
        let cost_usd = convo
            .messages
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("costUsd").and_then(|v| v.as_f64()))
                    .sum::<f64>()
            })
            .unwrap_or(0.0);
        let compaction_summaries = raw
            .get("compactionHistory")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("summary").and_then(|s| s.as_str()).map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
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
                    let flat = t.split_whitespace().collect::<Vec<_>>().join(" ");
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
        out.push(ConversationMeta {
            id: convo.id,
            title: convo.title,
            model: convo.model,
            message_count,
            created_at: convo.created_at,
            updated_at: convo.updated_at,
            cost_usd,
            compaction_summaries,
            last_snippet,
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
pub fn assistant_stats() -> Result<Vec<ConvoStat>, String> {
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
pub fn assistant_load_conversation(id: String) -> Result<Conversation, String> {
    let p = convo_path(&id)?;
    let bytes = std::fs::read(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse conversation: {e}"))
}

/// #30: expose a session's pinned cwd so the UI can flag a resumed tab that
/// operates in a different folder than the currently selected workspace.
#[tauri::command]
pub fn assistant_session_cwd(id: String) -> Option<String> {
    load_session_cwd(&id).map(|p| p.to_string_lossy().into_owned())
}

/// Write an exported conversation to a user-chosen path. The markdown/json
/// string is built on the frontend (where the typed block schema lives); this
/// just commits the bytes. `dest` comes from the native save dialog, so the
/// arbitrary-path write is the intended user action.
#[tauri::command]
pub fn assistant_export_save(dest: String, contents: String) -> Result<(), String> {
    // Defense-in-depth: this command is IPC-reachable. Reject malformed paths.
    // `dest` normally comes from the native save dialog (any drive is valid, so
    // we deliberately do NOT clamp to a root — that would break save-to-USB etc.).
    if dest.contains('\0') {
        return Err("dest contains a null byte".into());
    }
    if !std::path::Path::new(&dest).is_absolute() {
        return Err("dest must be an absolute path".into());
    }
    // Allowlist export extensions — prevents a compromised WebView from using
    // this command to overwrite arbitrary files (e.g. .exe, config files).
    let ext = std::path::Path::new(&dest)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(ext.to_ascii_lowercase().as_str(), "md" | "json" | "txt") {
        return Err(format!("unsupported export extension: .{ext} (allowed: .md .json .txt)"));
    }
    std::fs::write(&dest, contents.as_bytes()).map_err(|e| format!("write {dest}: {e}"))
}

#[tauri::command]
pub fn assistant_save_conversation(convo: Conversation) -> Result<(), String> {
    let p = convo_path(&convo.id)?;
    let s = serde_json::to_string(&convo).map_err(|e| e.to_string())?;
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
    delete_session_model(&id);
    if let Some(cli_id) = cli_session_id {
        if cli_id != id {
            delete_session_cwd(&cli_id);
            delete_session_model(&cli_id);
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
