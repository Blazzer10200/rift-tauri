//! Durable usage/metrics store + pricing layer — foundation for the cost
//! cockpit (idea-phase-plan §1a/1b). `usage_ingest_turn` is called from the
//! frontend's debounced session-log write so every finalized turn lands a
//! SQLite row before the session-log ring buffer can prune it;
//! `usage_backfill_from_logs` one-shot imports the existing `session-logs/*`.

pub mod aggregate;
pub mod budget;
pub mod insights;
pub mod pricing;
pub mod store;

use rusqlite::Connection;
use serde_json::Value;
use std::sync::Mutex;

/// Managed Tauri state — the single usage-DB connection behind a mutex. The
/// ingest/backfill commands are sync, so no lock is ever held across an await.
pub struct UsageDb(pub Mutex<Connection>);

impl UsageDb {
    /// Open the on-disk DB; fall back to an in-memory DB if that fails so the
    /// commands always have a valid connection (telemetry is best-effort —
    /// never crash the app over it).
    pub fn new() -> Self {
        let conn = store::open().unwrap_or_else(|e| {
            log::error!("usage: failed to open rift.db ({e}); using in-memory fallback");
            let c = Connection::open_in_memory().expect("in-memory sqlite");
            let _ = store::init_schema(&c);
            c
        });
        UsageDb(Mutex::new(conn))
    }
}

impl Default for UsageDb {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse one persisted session snapshot (the same payload handed to
/// `assistant_save_session_log`: `snapshot()` + `{ model, workspace }`) into
/// priced turn rows. Mirrors the token/timing math in `telemetry.ts`
/// (`ttfp = firstPaintAt - ts`, `duration = doneAt - ts`).
fn rows_from_snapshot(record: &Value, prices: &pricing::PriceTable) -> Vec<store::TurnRow> {
    let session_id = record
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if session_id.is_empty() {
        return Vec::new();
    }
    let workspace = record
        .get("workspace")
        .and_then(|v| v.as_str())
        .map(String::from);
    let session_model = record.get("model").and_then(|v| v.as_str());
    let turns = match record.get("turns").and_then(|v| v.as_array()) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut out = Vec::with_capacity(turns.len());
    for (i, turn) in turns.iter().enumerate() {
        // Prefer the authoritative result usage; fall back to the progressive
        // envelope usage when the result event never carried tokens.
        let usage = turn
            .get("resultUsage")
            .filter(|v| !v.is_null())
            .or_else(|| turn.get("envelopeUsage").filter(|v| !v.is_null()));
        let u = |k: &str| {
            usage
                .and_then(|x| x.get(k))
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
        };
        let input = u("input");
        let output = u("output");
        let cache_read = u("cacheRead");
        let cache_write = u("cacheCreate");

        let ts = turn.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
        let model_id = turn
            .get("modelId")
            .and_then(|v| v.as_str())
            .or_else(|| turn.get("model").and_then(|v| v.as_str()))
            .or(session_model)
            .map(String::from);
        // Reject NaN/Infinity from the untrusted IPC payload — a non-finite cost
        // poisons every budget/projection aggregate downstream.
        let cost_usd_cli = turn
            .get("costUsd")
            .and_then(|v| v.as_f64())
            .filter(|c| c.is_finite());
        let ttfp_ms = turn
            .get("firstPaintAt")
            .and_then(|v| v.as_i64())
            .map(|fp| fp - ts);
        let duration_ms = turn
            .get("doneAt")
            .and_then(|v| v.as_i64())
            .map(|d| d - ts);
        let tool_count = turn
            .get("toolUses")
            .and_then(|v| v.as_array())
            .map(|a| a.len() as i64)
            .unwrap_or(0);
        let provider = model_id
            .as_deref()
            .map(pricing::provider_for_model)
            .map(String::from);
        let cost_usd_calc = model_id
            .as_deref()
            .and_then(|m| prices.cost_for(m, input, output, cache_read, cache_write))
            .filter(|c| c.is_finite());

        out.push(store::TurnRow {
            session_id: session_id.clone(),
            turn_index: i as i64,
            ts,
            model_id,
            provider,
            input,
            output,
            cache_read,
            cache_write,
            cost_usd_cli,
            cost_usd_calc,
            ttfp_ms,
            duration_ms,
            workspace: workspace.clone(),
            tool_count,
        });
    }
    out
}

/// Ingest one session snapshot's turns. Called from the frontend's
/// `recordSessionLog` debounce after every completed turn — idempotent, so
/// re-ingesting the growing snapshot just overwrites prior rows. Returns the
/// number of rows written.
#[tauri::command]
pub fn usage_ingest_turn(db: tauri::State<UsageDb>, record: Value) -> Result<u32, String> {
    let prices = pricing::PriceTable::load();
    let rows = rows_from_snapshot(&record, &prices);
    let conn = db.0.lock().map_err(|e| format!("usage db lock: {e}"))?;
    let mut n = 0u32;
    for row in &rows {
        store::upsert_turn(&conn, row)?;
        n += 1;
    }
    Ok(n)
}

/// One-shot history import: read every `~/.rift/assistant/session-logs/*.json`
/// and upsert its turns. Idempotent — safe to run on every launch (the ring
/// buffer bounds the file count). Returns the number of rows written.
#[tauri::command]
pub fn usage_backfill_from_logs(db: tauri::State<UsageDb>) -> Result<u32, String> {
    let conn = db.0.lock().map_err(|e| format!("usage db lock: {e}"))?;
    backfill(&conn)
}

/// Shared backfill body so the startup pass can run on its own connection
/// without contending for the managed mutex.
pub fn backfill(conn: &Connection) -> Result<u32, String> {
    let home = crate::state::paths::dirs_home().map_err(|e| e.to_string())?;
    let dir = home.join(".rift").join("assistant").join("session-logs");
    let prices = pricing::PriceTable::load();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(0),
    };
    let mut n = 0u32;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = match std::fs::read(&p) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let record: Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for row in rows_from_snapshot(&record, &prices) {
            if store::upsert_turn(conn, &row).is_ok() {
                n += 1;
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn prices() -> pricing::PriceTable {
        pricing::PriceTable::load()
    }

    #[test]
    fn empty_id_yields_no_rows() {
        let rec = json!({ "turns": [{ "ts": 1 }] });
        assert!(rows_from_snapshot(&rec, &prices()).is_empty());
    }

    #[test]
    fn missing_turns_array_yields_no_rows() {
        let rec = json!({ "id": "s1" });
        assert!(rows_from_snapshot(&rec, &prices()).is_empty());
    }

    #[test]
    fn parses_tokens_timing_and_tool_count() {
        let rec = json!({
            "id": "s1",
            "workspace": "/ws",
            "turns": [{
                "ts": 1000,
                "modelId": "claude-opus-4-8",
                "firstPaintAt": 1150,
                "doneAt": 1900,
                "costUsd": 0.05,
                "resultUsage": { "input": 100, "output": 200, "cacheRead": 5, "cacheCreate": 7 },
                "toolUses": [{}, {}, {}]
            }]
        });
        let rows = rows_from_snapshot(&rec, &prices());
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.session_id, "s1");
        assert_eq!(r.turn_index, 0);
        assert_eq!(r.input, 100);
        assert_eq!(r.output, 200);
        assert_eq!(r.cache_read, 5);
        assert_eq!(r.cache_write, 7);
        assert_eq!(r.ttfp_ms, Some(150)); // firstPaintAt - ts
        assert_eq!(r.duration_ms, Some(900)); // doneAt - ts
        assert_eq!(r.tool_count, 3);
        assert_eq!(r.workspace.as_deref(), Some("/ws"));
        assert_eq!(r.cost_usd_cli, Some(0.05));
        assert_eq!(r.provider.as_deref(), Some("anthropic"));
    }

    #[test]
    fn envelope_usage_is_fallback_when_result_usage_absent() {
        let rec = json!({
            "id": "s1",
            "turns": [{
                "ts": 1,
                "modelId": "claude-opus-4-8",
                "envelopeUsage": { "input": 42, "output": 9 }
            }]
        });
        let rows = rows_from_snapshot(&rec, &prices());
        assert_eq!(rows[0].input, 42);
        assert_eq!(rows[0].output, 9);
    }

    #[test]
    fn null_result_usage_falls_back_to_envelope() {
        let rec = json!({
            "id": "s1",
            "turns": [{
                "ts": 1,
                "modelId": "claude-opus-4-8",
                "resultUsage": null,
                "envelopeUsage": { "input": 7 }
            }]
        });
        assert_eq!(rows_from_snapshot(&rec, &prices())[0].input, 7);
    }

    #[test]
    fn model_id_falls_back_to_session_model() {
        let rec = json!({
            "id": "s1",
            "model": "claude-sonnet-4-6",
            "turns": [{ "ts": 1 }] // turn carries no modelId/model
        });
        let rows = rows_from_snapshot(&rec, &prices());
        assert_eq!(rows[0].model_id.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn custom_model_is_unpriced_and_non_anthropic() {
        let rec = json!({
            "id": "s1",
            "turns": [{
                "ts": 1,
                "modelId": "my-local-llm-7b",
                "resultUsage": { "input": 100, "output": 100 }
            }]
        });
        let rows = rows_from_snapshot(&rec, &prices());
        assert!(rows[0].cost_usd_calc.is_none(), "unknown model has no computed cost");
        assert_ne!(rows[0].provider.as_deref(), Some("anthropic"));
    }

    #[test]
    fn turn_indices_increment_in_order() {
        let rec = json!({
            "id": "s1",
            "turns": [
                { "ts": 1, "modelId": "claude-opus-4-8" },
                { "ts": 2, "modelId": "claude-opus-4-8" },
                { "ts": 3, "modelId": "claude-opus-4-8" }
            ]
        });
        let rows = rows_from_snapshot(&rec, &prices());
        let idx: Vec<i64> = rows.iter().map(|r| r.turn_index).collect();
        assert_eq!(idx, vec![0, 1, 2]);
    }
}
