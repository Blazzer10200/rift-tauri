//! Persistent multi-session telemetry log. Each Rift launch produces one
//! `SessionTelemetry` snapshot (frontend: `src/lib/state/assistant/telemetry.ts`);
//! this module commits those snapshots to
//! `~/.rift/assistant/session-logs/<id>.json` so the Harness page can browse
//! past sessions across restarts. Mirrors the conversation-persistence pattern
//! in `mod.rs`: atomic tmp+rename writes, a thin meta list for cheap browsing,
//! and the same id charset guard.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

fn session_logs_dir() -> Result<PathBuf, String> {
    let home = super::dirs_home()?;
    let dir = home.join(".rift").join("assistant").join("session-logs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir session-logs: {e}"))?;
    Ok(dir)
}

fn session_log_path(id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!("invalid session log id: {id}"));
    }
    Ok(session_logs_dir()?.join(format!("{id}.json")))
}

/// Thin per-session metadata returned by `assistant_list_session_logs`. Folded
/// out of each stored snapshot so the picker can list 100s of sessions without
/// loading every turn array into memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLogMeta {
    pub id: String,
    pub started_at: i64,
    pub captured_at: i64,
    pub duration_ms: i64,
    pub turn_count: u32,
    pub total_cost_usd: f64,
    pub total_turns: u32,
    pub tool_call_total: u32,
    pub model: Option<String>,
    pub workspace: Option<String>,
}

/// Persist one session snapshot. `record` is the frontend's `snapshot()` JSON
/// merged with `{ id, model, workspace }`; stored verbatim as a `Value` so the
/// telemetry schema can evolve without touching Rust types (same contract as
/// `Conversation.messages`).
#[tauri::command]
pub fn assistant_save_session_log(record: Value) -> Result<(), String> {
    let id = record
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("session log record missing id")?;
    let p = session_log_path(id)?;
    let s = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, s).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("rename {}: {e}", p.display()))?;
    Ok(())
}

#[tauri::command]
pub fn assistant_list_session_logs() -> Result<Vec<SessionLogMeta>, String> {
    let dir = session_logs_dir()?;
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
        let raw: Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(id) = raw.get("id").and_then(|v| v.as_str()).map(String::from) else {
            continue;
        };
        let g_i64 = |k: &str| raw.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
        let summary = raw.get("summary");
        let s_f64 = |k: &str| {
            summary
                .and_then(|s| s.get(k))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        };
        let s_u32 = |k: &str| {
            summary
                .and_then(|s| s.get(k))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32
        };
        out.push(SessionLogMeta {
            id,
            started_at: g_i64("startedAt"),
            captured_at: g_i64("capturedAt"),
            duration_ms: g_i64("durationMs"),
            turn_count: raw.get("turnCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_cost_usd: s_f64("totalCostUsd"),
            total_turns: s_u32("totalTurns"),
            tool_call_total: s_u32("toolCallTotal"),
            model: raw.get("model").and_then(|v| v.as_str()).map(String::from),
            workspace: raw.get("workspace").and_then(|v| v.as_str()).map(String::from),
        });
    }
    // Newest first — the picker reads top-down.
    out.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(out)
}

#[tauri::command]
pub fn assistant_load_session_log(id: String) -> Result<Value, String> {
    let p = session_log_path(&id)?;
    let bytes = std::fs::read(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse session log: {e}"))
}

#[tauri::command]
pub fn assistant_delete_session_log(id: String) -> Result<(), String> {
    let p = session_log_path(&id)?;
    match std::fs::remove_file(&p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("delete {}: {e}", p.display())),
    }
}

/// Ring-buffer trim: keep the `keep` most-recent sessions (by `startedAt`),
/// delete the rest. Called once on launch so a long-running install doesn't
/// accumulate thousands of session files. Returns the count removed.
#[tauri::command]
pub fn assistant_prune_session_logs(keep: usize) -> Result<u32, String> {
    let dir = session_logs_dir()?;
    let mut files: Vec<(i64, PathBuf)> = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(0),
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let started = std::fs::read(&p)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v.get("startedAt").and_then(|x| x.as_i64()))
            .unwrap_or(0);
        files.push((started, p));
    }
    if files.len() <= keep {
        return Ok(0);
    }
    files.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
    let mut removed = 0u32;
    for (_, p) in files.into_iter().skip(keep) {
        if std::fs::remove_file(&p).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}
