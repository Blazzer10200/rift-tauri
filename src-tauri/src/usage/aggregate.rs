//! Cross-session aggregation over the durable `turns` table (idea-phase-plan
//! §1c / D3). Mirrors ccusage's views — daily / monthly / per-model /
//! per-workspace / 5h-billing-block / per-session — as flat DTO arrays the
//! frontend renders directly. The heavy folds run in SQLite `GROUP BY`, not
//! Svelte. Cost prefers the recomputed `cost_usd_calc`, falling back to the
//! CLI's `cost_usd_cli` then 0 so custom-provider turns still count.

use super::UsageDb;
use rusqlite::{Connection, Row};
use serde::Serialize;

/// SQL fragment: the trustworthy per-turn cost. Computed table cost wins;
/// CLI cost is the fallback; 0 when neither is known.
const COST: &str = "COALESCE(cost_usd_calc, cost_usd_cli, 0)";

/// One calendar day's rollup (local time). `date` is `YYYY-MM-DD`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRow {
    pub date: String,
    pub cost: f64,
    pub turns: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
}

/// One calendar month's rollup (local time). `month` is `YYYY-MM`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyRow {
    pub month: String,
    pub cost: f64,
    pub turns: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
}

/// Per-model rollup. `priced` is false when every turn for the model lacked a
/// price-table entry (custom provider) — the UI flags it as estimated.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRow {
    pub model_id: String,
    pub provider: Option<String>,
    pub cost: f64,
    pub turns: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub priced: bool,
}

/// Per-workspace rollup. `workspace` is None for turns with no recorded root.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRow {
    pub workspace: Option<String>,
    pub cost: f64,
    pub turns: i64,
    pub input: i64,
    pub output: i64,
}

/// One fixed billing block (default 5h, anchored to the unix epoch). `active`
/// marks the block containing "now". `start`/`end` are epoch ms.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRow {
    pub start: i64,
    pub end: i64,
    pub cost: f64,
    pub turns: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub active: bool,
}

/// One turn of a single session (for the drill-down view).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTurnRow {
    pub turn_index: i64,
    pub ts: i64,
    pub model_id: Option<String>,
    pub cost: f64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub ttfp_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub tool_count: i64,
}

fn lock(db: &UsageDb) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
    db.0.lock().map_err(|e| format!("usage db lock: {e}"))
}

/// Collect every row of a prepared statement through a row-mapper.
fn collect<T>(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
    map: impl Fn(&Row) -> rusqlite::Result<T>,
) -> Result<Vec<T>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params, |r| map(r))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<T>>>()
        .map_err(|e| e.to_string())
}

/// Daily rollup over the last `days` days (default 30). `ts` is epoch ms, so
/// `ts/1000` feeds SQLite's `unixepoch`; grouping is by LOCAL calendar day.
#[tauri::command]
pub fn usage_daily(db: tauri::State<UsageDb>, days: Option<i64>) -> Result<Vec<DailyRow>, String> {
    let days = days.unwrap_or(30).max(1);
    let cutoff_ms = now_ms() - days * 86_400_000;
    let conn = lock(&db)?;
    collect(
        &conn,
        &format!(
            "SELECT strftime('%Y-%m-%d', ts/1000, 'unixepoch', 'localtime') AS d,
                    SUM({COST}), COUNT(*), SUM(input), SUM(output),
                    SUM(cache_read), SUM(cache_write)
             FROM turns WHERE ts >= ?1
             GROUP BY d ORDER BY d ASC"
        ),
        &[&cutoff_ms],
        |r| {
            Ok(DailyRow {
                date: r.get(0)?,
                cost: r.get(1)?,
                turns: r.get(2)?,
                input: r.get(3)?,
                output: r.get(4)?,
                cache_read: r.get(5)?,
                cache_write: r.get(6)?,
            })
        },
    )
}

/// Monthly rollup over all history (local calendar month).
#[tauri::command]
pub fn usage_monthly(db: tauri::State<UsageDb>) -> Result<Vec<MonthlyRow>, String> {
    let conn = lock(&db)?;
    collect(
        &conn,
        &format!(
            "SELECT strftime('%Y-%m', ts/1000, 'unixepoch', 'localtime') AS m,
                    SUM({COST}), COUNT(*), SUM(input), SUM(output),
                    SUM(cache_read), SUM(cache_write)
             FROM turns GROUP BY m ORDER BY m ASC"
        ),
        &[],
        |r| {
            Ok(MonthlyRow {
                month: r.get(0)?,
                cost: r.get(1)?,
                turns: r.get(2)?,
                input: r.get(3)?,
                output: r.get(4)?,
                cache_read: r.get(5)?,
                cache_write: r.get(6)?,
            })
        },
    )
}

/// Per-model rollup, costliest first. `priced` = at least one turn had a
/// non-null computed cost (the model is in the price table).
#[tauri::command]
pub fn usage_by_model(db: tauri::State<UsageDb>) -> Result<Vec<ModelRow>, String> {
    let conn = lock(&db)?;
    collect(
        &conn,
        &format!(
            "SELECT COALESCE(model_id,'unknown'), provider,
                    SUM({COST}), COUNT(*), SUM(input), SUM(output),
                    SUM(cache_read), SUM(cache_write),
                    MAX(cost_usd_calc IS NOT NULL)
             FROM turns GROUP BY model_id ORDER BY SUM({COST}) DESC"
        ),
        &[],
        |r| {
            Ok(ModelRow {
                model_id: r.get(0)?,
                provider: r.get(1)?,
                cost: r.get(2)?,
                turns: r.get(3)?,
                input: r.get(4)?,
                output: r.get(5)?,
                cache_read: r.get(6)?,
                cache_write: r.get(7)?,
                priced: r.get::<_, i64>(8)? != 0,
            })
        },
    )
}

/// Per-workspace rollup, costliest first.
#[tauri::command]
pub fn usage_by_workspace(db: tauri::State<UsageDb>) -> Result<Vec<WorkspaceRow>, String> {
    let conn = lock(&db)?;
    collect(
        &conn,
        &format!(
            "SELECT workspace, SUM({COST}), COUNT(*), SUM(input), SUM(output)
             FROM turns GROUP BY workspace ORDER BY SUM({COST}) DESC"
        ),
        &[],
        |r| {
            Ok(WorkspaceRow {
                workspace: r.get(0)?,
                cost: r.get(1)?,
                turns: r.get(2)?,
                input: r.get(3)?,
                output: r.get(4)?,
            })
        },
    )
}

/// Fixed billing blocks of `window` hours (default 5 — Claude's rolling-limit
/// window), anchored to the unix epoch so blocks are stable across calls.
/// Newest first; `active` marks the block containing now.
#[tauri::command]
pub fn usage_blocks(
    db: tauri::State<UsageDb>,
    window: Option<i64>,
) -> Result<Vec<BlockRow>, String> {
    let win_ms = window.unwrap_or(5).clamp(1, 24) * 3_600_000;
    let now = now_ms();
    let conn = lock(&db)?;
    collect(
        &conn,
        &format!(
            "SELECT (ts / ?1) * ?1 AS blk,
                    SUM({COST}), COUNT(*), SUM(input), SUM(output),
                    SUM(cache_read), SUM(cache_write)
             FROM turns GROUP BY blk ORDER BY blk DESC"
        ),
        &[&win_ms],
        |r| {
            let start: i64 = r.get(0)?;
            Ok(BlockRow {
                start,
                end: start + win_ms,
                cost: r.get(1)?,
                turns: r.get(2)?,
                input: r.get(3)?,
                output: r.get(4)?,
                cache_read: r.get(5)?,
                cache_write: r.get(6)?,
                active: now >= start && now < start + win_ms,
            })
        },
    )
}

/// All turns of one session, in order (drill-down for the cockpit).
#[tauri::command]
pub fn usage_session(
    db: tauri::State<UsageDb>,
    id: String,
) -> Result<Vec<SessionTurnRow>, String> {
    let conn = lock(&db)?;
    collect(
        &conn,
        &format!(
            "SELECT turn_index, ts, model_id, {COST}, input, output,
                    cache_read, cache_write, ttfp_ms, duration_ms, tool_count
             FROM turns WHERE session_id = ?1 ORDER BY turn_index ASC"
        ),
        &[&id],
        |r| {
            Ok(SessionTurnRow {
                turn_index: r.get(0)?,
                ts: r.get(1)?,
                model_id: r.get(2)?,
                cost: r.get(3)?,
                input: r.get(4)?,
                output: r.get(5)?,
                cache_read: r.get(6)?,
                cache_write: r.get(7)?,
                ttfp_ms: r.get(8)?,
                duration_ms: r.get(9)?,
                tool_count: r.get(10)?,
            })
        },
    )
}

/// Epoch milliseconds now (the same clock `TurnRecord.ts` is written in).
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
