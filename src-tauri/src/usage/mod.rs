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
        let cost_usd_cli = turn.get("costUsd").and_then(|v| v.as_f64());
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
            .and_then(|m| prices.cost_for(m, input, output, cache_read, cache_write));

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
