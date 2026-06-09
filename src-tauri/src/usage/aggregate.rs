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
    let conn = lock(&db)?;
    daily_rows(&conn, days.unwrap_or(30), now_ms())
}

fn daily_rows(conn: &Connection, days: i64, now: i64) -> Result<Vec<DailyRow>, String> {
    let days = days.max(1);
    let cutoff_ms = now - days * 86_400_000;
    collect(
        conn,
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
    monthly_rows(&conn)
}

fn monthly_rows(conn: &Connection) -> Result<Vec<MonthlyRow>, String> {
    collect(
        conn,
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
    by_model_rows(&conn)
}

fn by_model_rows(conn: &Connection) -> Result<Vec<ModelRow>, String> {
    collect(
        conn,
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
    by_workspace_rows(&conn)
}

fn by_workspace_rows(conn: &Connection) -> Result<Vec<WorkspaceRow>, String> {
    collect(
        conn,
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
    let conn = lock(&db)?;
    block_rows(&conn, window.unwrap_or(5), now_ms())
}

fn block_rows(conn: &Connection, window: i64, now: i64) -> Result<Vec<BlockRow>, String> {
    let win_ms = window.clamp(1, 24) * 3_600_000;
    collect(
        conn,
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
    session_rows(&conn, &id)
}

fn session_rows(conn: &Connection, id: &str) -> Result<Vec<SessionTurnRow>, String> {
    collect(
        conn,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::store::{self, TurnRow};

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        store::init_schema(&c).unwrap();
        c
    }

    fn base(idx: i64, ts: i64) -> TurnRow {
        TurnRow {
            session_id: "s1".into(),
            turn_index: idx,
            ts,
            model_id: None,
            provider: None,
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cost_usd_cli: None,
            cost_usd_calc: None,
            ttfp_ms: None,
            duration_ms: None,
            workspace: None,
            tool_count: 0,
        }
    }

    fn put(c: &Connection, r: TurnRow) {
        store::upsert_turn(c, &r).unwrap();
    }

    #[test]
    fn by_model_priced_flag_and_cost_fallback() {
        let c = mem();
        // Priced model: computed cost present.
        put(&c, TurnRow { model_id: Some("claude-opus-4-8".into()), cost_usd_calc: Some(3.0), ..base(0, 1) });
        // Unpriced model: only CLI cost — COST falls back to it, priced=false.
        put(&c, TurnRow { model_id: Some("custom-x".into()), cost_usd_cli: Some(1.0), ..base(1, 1) });

        let rows = by_model_rows(&c).unwrap();
        assert_eq!(rows.len(), 2);
        // Sorted by COST desc → opus (3.0) first.
        assert_eq!(rows[0].model_id, "claude-opus-4-8");
        assert!(rows[0].priced, "computed-cost model is priced");
        assert!((rows[0].cost - 3.0).abs() < 1e-9);
        assert_eq!(rows[1].model_id, "custom-x");
        assert!(!rows[1].priced, "cli-only model is unpriced");
        assert!((rows[1].cost - 1.0).abs() < 1e-9, "COST falls back to cli cost");
    }

    #[test]
    fn cost_prefers_calc_over_cli() {
        let c = mem();
        // calc and cli disagree → COALESCE(calc, cli) must pick calc.
        put(&c, TurnRow { model_id: Some("m".into()), cost_usd_calc: Some(2.0), cost_usd_cli: Some(99.0), ..base(0, 1) });
        let rows = by_model_rows(&c).unwrap();
        assert!((rows[0].cost - 2.0).abs() < 1e-9, "calc cost must win over cli");
    }

    #[test]
    fn blocks_anchor_to_window_and_flag_active() {
        let c = mem();
        let win_ms = 5 * 3_600_000; // 5h
        // A turn whose ts is the "now" → its block must be the active one.
        let now = 100 * win_ms + 1234; // mid-block offset
        put(&c, TurnRow { cost_usd_calc: Some(1.0), ..base(0, now) });
        // A turn three windows earlier → a different, inactive block.
        put(&c, TurnRow { cost_usd_calc: Some(1.0), ..base(1, now - 3 * win_ms) });

        let rows = block_rows(&c, 5, now).unwrap();
        assert_eq!(rows.len(), 2);
        // Newest first; each block start is window-aligned.
        for r in &rows {
            assert_eq!(r.start % win_ms, 0, "block start must be window-aligned");
            assert_eq!(r.end - r.start, win_ms);
        }
        let active: Vec<bool> = rows.iter().map(|r| r.active).collect();
        assert_eq!(active, vec![true, false], "only the now-containing block is active");
    }

    #[test]
    fn daily_excludes_turns_before_cutoff() {
        let c = mem();
        let now = 30 * 86_400_000_i64 + 500;
        put(&c, TurnRow { cost_usd_calc: Some(1.0), ..base(0, now) }); // today
        put(&c, TurnRow { cost_usd_calc: Some(9.0), ..base(1, now - 40 * 86_400_000) }); // 40d ago
        let rows = daily_rows(&c, 30, now).unwrap();
        let total: f64 = rows.iter().map(|r| r.cost).sum();
        assert!((total - 1.0).abs() < 1e-9, "40-day-old turn must fall outside a 30-day window");
    }

    #[test]
    fn by_workspace_groups_null_and_sorts_by_cost() {
        let c = mem();
        put(&c, TurnRow { workspace: Some("/a".into()), cost_usd_calc: Some(5.0), ..base(0, 1) });
        put(&c, TurnRow { workspace: None, cost_usd_calc: Some(3.0), ..base(1, 1) });
        let rows = by_workspace_rows(&c).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].workspace.as_deref(), Some("/a")); // costliest first
        assert_eq!(rows[1].workspace, None); // null workspace kept as its own group
    }

    #[test]
    fn session_rows_filter_and_order_by_turn_index() {
        let c = mem();
        put(&c, TurnRow { turn_index: 2, ..base(2, 3) });
        put(&c, TurnRow { turn_index: 0, ..base(0, 1) });
        put(&c, TurnRow { turn_index: 1, ..base(1, 2) });
        put(&c, TurnRow { session_id: "other".into(), ..base(0, 1) });
        let rows = session_rows(&c, "s1").unwrap();
        let idx: Vec<i64> = rows.iter().map(|r| r.turn_index).collect();
        assert_eq!(idx, vec![0, 1, 2], "only s1, ordered by turn_index");
    }
}
