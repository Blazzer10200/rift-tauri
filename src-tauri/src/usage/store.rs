//! SQLite durable usage store (idea-phase-plan §1a / D1). One append-only
//! `turns` table at `~/.rift/rift.db`, idempotent-upsert keyed on
//! `(session_id, turn_index)`. The session-log JSONs stay the live/replay
//! format; this is the historical truth that survives the ring-buffer prune.

use rusqlite::Connection;
use std::path::PathBuf;

fn db_path() -> Result<PathBuf, String> {
    let dir = crate::state::paths::rift_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("rift.db"))
}

/// Open (creating if absent) the usage DB and ensure the schema exists. WAL +
/// a busy timeout so the startup backfill connection and the per-turn ingest
/// connection don't collide.
pub fn open() -> Result<Connection, String> {
    let p = db_path()?;
    let conn = Connection::open(&p).map_err(|e| format!("open rift.db: {e}"))?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    init_schema(&conn)?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS turns (
            session_id    TEXT    NOT NULL,
            turn_index    INTEGER NOT NULL,
            ts            INTEGER NOT NULL,
            model_id      TEXT,
            provider      TEXT,
            input         INTEGER NOT NULL DEFAULT 0,
            output        INTEGER NOT NULL DEFAULT 0,
            cache_read    INTEGER NOT NULL DEFAULT 0,
            cache_write   INTEGER NOT NULL DEFAULT 0,
            cost_usd_cli  REAL,
            cost_usd_calc REAL,
            ttfp_ms       INTEGER,
            duration_ms   INTEGER,
            workspace     TEXT,
            tool_count    INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (session_id, turn_index)
         );
         CREATE INDEX IF NOT EXISTS idx_turns_ts ON turns(ts);
         CREATE INDEX IF NOT EXISTS idx_turns_model ON turns(model_id);",
    )
    .map_err(|e| format!("usage init schema: {e}"))
}

/// One normalized turn row. Token fields are absolute counts; cost fields are
/// USD. `cost_usd_calc` is `None` when the model isn't priced.
pub struct TurnRow {
    pub session_id: String,
    pub turn_index: i64,
    pub ts: i64,
    pub model_id: Option<String>,
    pub provider: Option<String>,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub cost_usd_cli: Option<f64>,
    pub cost_usd_calc: Option<f64>,
    pub ttfp_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub workspace: Option<String>,
    pub tool_count: i64,
}

/// Idempotent insert-or-replace on `(session_id, turn_index)`. Re-ingesting a
/// session (debounce coalescing, backfill re-run) overwrites in place.
pub fn upsert_turn(conn: &Connection, r: &TurnRow) -> Result<(), String> {
    conn.execute(
        "INSERT INTO turns
            (session_id, turn_index, ts, model_id, provider, input, output,
             cache_read, cache_write, cost_usd_cli, cost_usd_calc, ttfp_ms,
             duration_ms, workspace, tool_count)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
         ON CONFLICT(session_id, turn_index) DO UPDATE SET
            ts=excluded.ts, model_id=excluded.model_id, provider=excluded.provider,
            input=excluded.input, output=excluded.output, cache_read=excluded.cache_read,
            cache_write=excluded.cache_write, cost_usd_cli=excluded.cost_usd_cli,
            cost_usd_calc=excluded.cost_usd_calc, ttfp_ms=excluded.ttfp_ms,
            duration_ms=excluded.duration_ms, workspace=excluded.workspace,
            tool_count=excluded.tool_count",
        rusqlite::params![
            r.session_id,
            r.turn_index,
            r.ts,
            r.model_id,
            r.provider,
            r.input,
            r.output,
            r.cache_read,
            r.cache_write,
            r.cost_usd_cli,
            r.cost_usd_calc,
            r.ttfp_ms,
            r.duration_ms,
            r.workspace,
            r.tool_count,
        ],
    )
    .map(|_| ())
    .map_err(|e| format!("usage upsert turn: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().expect("in-memory sqlite");
        init_schema(&c).expect("schema");
        c
    }

    fn row(session: &str, idx: i64) -> TurnRow {
        TurnRow {
            session_id: session.into(),
            turn_index: idx,
            ts: 1_700_000_000_000,
            model_id: Some("claude-opus-4-8".into()),
            provider: Some("anthropic".into()),
            input: 10,
            output: 20,
            cache_read: 0,
            cache_write: 0,
            cost_usd_cli: Some(0.01),
            cost_usd_calc: Some(0.02),
            ttfp_ms: Some(100),
            duration_ms: Some(500),
            workspace: Some("/ws".into()),
            tool_count: 1,
        }
    }

    fn count(c: &Connection) -> i64 {
        c.query_row("SELECT COUNT(*) FROM turns", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn init_schema_is_idempotent() {
        let c = mem();
        // Second call must not error (IF NOT EXISTS) and must keep the table.
        init_schema(&c).expect("second init");
        assert_eq!(count(&c), 0);
    }

    #[test]
    fn upsert_inserts_then_replaces_on_same_key() {
        let c = mem();
        upsert_turn(&c, &row("s1", 0)).unwrap();
        assert_eq!(count(&c), 1);

        // Same (session_id, turn_index) → in-place update, still one row.
        let mut r = row("s1", 0);
        r.output = 999;
        upsert_turn(&c, &r).unwrap();
        assert_eq!(count(&c), 1);
        let out: i64 = c
            .query_row("SELECT output FROM turns WHERE session_id='s1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(out, 999, "conflicting upsert should overwrite output");
    }

    #[test]
    fn distinct_turn_indices_are_separate_rows() {
        let c = mem();
        upsert_turn(&c, &row("s1", 0)).unwrap();
        upsert_turn(&c, &row("s1", 1)).unwrap();
        upsert_turn(&c, &row("s2", 0)).unwrap();
        assert_eq!(count(&c), 3);
    }

    #[test]
    fn nullable_cost_fields_persist_as_null() {
        let c = mem();
        let mut r = row("s1", 0);
        r.cost_usd_cli = None;
        r.cost_usd_calc = None;
        upsert_turn(&c, &r).unwrap();
        let calc: Option<f64> = c
            .query_row("SELECT cost_usd_calc FROM turns", [], |r| r.get(0))
            .unwrap();
        assert!(calc.is_none(), "unpriced turn should store NULL calc cost");
    }
}
