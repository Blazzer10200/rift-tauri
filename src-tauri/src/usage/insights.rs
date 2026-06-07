//! Insight layer v1 (idea-phase-plan §2b) — deterministic, read-only queries
//! over the durable `turns` corpus. The seed of Pillar 3 ("grows-with-you"):
//! observational only, no auto-action. Each helper emits at most one `Insight`
//! and only when a real, non-trivial threshold is crossed — a blank or sparse
//! corpus yields an empty list, never filler. The frontend renders these under
//! a "Rift noticed…" panel.

use super::UsageDb;
use rusqlite::Connection;
use serde::Serialize;

/// The trustworthy per-turn cost (mirrors aggregate.rs): computed price wins,
/// CLI cost is the fallback, 0 when neither is known.
const COST: &str = "COALESCE(cost_usd_calc, cost_usd_cli, 0)";

/// One surfaced pattern. `kind` groups it; `severity` is purely cosmetic
/// (good / info / warn) — v1 never acts on these.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insight {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub severity: String,
}

fn usd(x: f64) -> String {
    if x >= 100.0 {
        format!("${x:.0}")
    } else if x >= 1.0 {
        format!("${x:.2}")
    } else {
        format!("${x:.3}")
    }
}

fn pct(part: f64, whole: f64) -> i64 {
    if whole <= 0.0 {
        0
    } else {
        (part / whole * 100.0).round() as i64
    }
}

fn short_model(id: &str) -> String {
    id.strip_prefix("claude-")
        .unwrap_or(id)
        .replace("-20", " 20") // keep readable; dated ids collapse a touch
        .split(' ')
        .next()
        .unwrap_or(id)
        .to_string()
}

fn workspace_leaf(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// Total lifetime turns + spend — the gate everything else hangs off. With too
/// few turns no insight is "non-trivial", so we bail the whole pass.
fn corpus_size(conn: &Connection) -> (i64, f64) {
    conn.query_row(
        &format!("SELECT COUNT(*), SUM({COST}) FROM turns"),
        [],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<f64>>(1)?.unwrap_or(0.0))),
    )
    .unwrap_or((0, 0.0))
}

/// A single model carries an outsized share of spend.
fn dominant_model(conn: &Connection, total_cost: f64) -> Option<Insight> {
    if total_cost <= 0.0 {
        return None;
    }
    let (model, cost): (Option<String>, f64) = conn
        .query_row(
            &format!(
                "SELECT model_id, SUM({COST}) c FROM turns
                 GROUP BY model_id ORDER BY c DESC LIMIT 1"
            ),
            [],
            |r| Ok((r.get(0)?, r.get::<_, Option<f64>>(1)?.unwrap_or(0.0))),
        )
        .ok()?;
    let model = model?;
    let share = pct(cost, total_cost);
    if share < 55 {
        return None;
    }
    let name = short_model(&model);
    Some(Insight {
        id: "dominant-model".into(),
        kind: "model".into(),
        title: format!("{name} drives most of your spend"),
        detail: format!(
            "{name} accounts for {share}% of all credit burned ({} of {}). Cheaper tiers on routine turns would move this number.",
            usd(cost),
            usd(total_cost)
        ),
        severity: if name.contains("opus") { "warn".into() } else { "info".into() },
    })
}

/// One workspace is the clear cost sink.
fn cost_sink_workspace(conn: &Connection, total_cost: f64) -> Option<Insight> {
    if total_cost <= 0.0 {
        return None;
    }
    // Need ≥2 distinct named workspaces for "sink" to mean anything.
    let distinct: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT workspace) FROM turns WHERE workspace IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if distinct < 2 {
        return None;
    }
    let (ws, cost): (Option<String>, f64) = conn
        .query_row(
            &format!(
                "SELECT workspace, SUM({COST}) c FROM turns
                 WHERE workspace IS NOT NULL GROUP BY workspace ORDER BY c DESC LIMIT 1"
            ),
            [],
            |r| Ok((r.get(0)?, r.get::<_, Option<f64>>(1)?.unwrap_or(0.0))),
        )
        .ok()?;
    let ws = ws?;
    let share = pct(cost, total_cost);
    if share < 45 {
        return None;
    }
    Some(Insight {
        id: "cost-sink-workspace".into(),
        kind: "workspace".into(),
        title: format!("{} is your costliest workspace", workspace_leaf(&ws)),
        detail: format!(
            "{} ({share}% of all spend) comes from work in {}.",
            usd(cost),
            workspace_leaf(&ws)
        ),
        severity: "info".into(),
    })
}

/// The 4-hour window of the day where the most credit burns.
fn peak_window(conn: &Connection, total_turns: i64, total_cost: f64) -> Option<Insight> {
    if total_turns < 20 || total_cost <= 0.0 {
        return None;
    }
    // Cost per local hour-of-day, folded into six 4h buckets.
    let mut stmt = conn
        .prepare(&format!(
            "SELECT CAST(strftime('%H', ts/1000, 'unixepoch', 'localtime') AS INTEGER) h,
                    SUM({COST}) c
             FROM turns GROUP BY h"
        ))
        .ok()?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<f64>>(1)?.unwrap_or(0.0)))
        })
        .ok()?;
    let mut buckets = [0.0_f64; 6];
    for row in rows.flatten() {
        let (h, c) = row;
        let b = ((h.clamp(0, 23)) / 4) as usize;
        buckets[b] += c;
    }
    let (idx, &peak) = buckets
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))?;
    let share = pct(peak, total_cost);
    if share < 35 {
        return None;
    }
    let start = idx * 4;
    let label = |h: usize| -> String {
        let suffix = if h < 12 { "am" } else { "pm" };
        let h12 = match h % 12 {
            0 => 12,
            x => x,
        };
        format!("{h12}{suffix}")
    };
    Some(Insight {
        id: "peak-window".into(),
        kind: "time".into(),
        title: format!("Most of your burn is {}–{}", label(start), label(start + 4)),
        detail: format!(
            "{share}% of spend ({}) lands between {} and {} local time.",
            usd(peak),
            label(start),
            label(start + 4)
        ),
        severity: "info".into(),
    })
}

/// Cache hit-rate this week vs the prior week (trend), else the overall level.
fn cache_efficiency(conn: &Connection) -> Option<Insight> {
    let now = super::aggregate::now_ms();
    let week = 7 * 86_400_000_i64;
    let (recent_cr, recent_in, prior_cr, prior_in): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT
                SUM(CASE WHEN ts >= ?1 THEN cache_read ELSE 0 END),
                SUM(CASE WHEN ts >= ?1 THEN input ELSE 0 END),
                SUM(CASE WHEN ts < ?1 AND ts >= ?2 THEN cache_read ELSE 0 END),
                SUM(CASE WHEN ts < ?1 AND ts >= ?2 THEN input ELSE 0 END)
             FROM turns WHERE ts >= ?2",
            [now - week, now - 2 * week],
            |r| {
                Ok((
                    r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                ))
            },
        )
        .ok()?;
    let hit = |cr: i64, inp: i64| -> Option<i64> {
        let denom = cr + inp;
        if denom < 50_000 {
            None
        } else {
            Some(pct(cr as f64, denom as f64))
        }
    };
    let recent = hit(recent_cr, recent_in);
    let prior = hit(prior_cr, prior_in);
    match (recent, prior) {
        (Some(r), Some(p)) if (r - p).abs() >= 6 => {
            let up = r > p;
            Some(Insight {
                id: "cache-trend".into(),
                kind: "cache".into(),
                title: format!(
                    "Cache hit-rate {} to {r}%",
                    if up { "climbed" } else { "slipped" }
                ),
                detail: format!(
                    "Your prompt-cache reuse is {r}% this week vs {p}% last — {} {} points. Higher reuse means cheaper, faster turns.",
                    if up { "up" } else { "down" },
                    (r - p).abs()
                ),
                severity: if up { "good".into() } else { "warn".into() },
            })
        }
        (Some(r), _) if r < 40 => Some(Insight {
            id: "cache-low".into(),
            kind: "cache".into(),
            title: format!("Cache reuse is low ({r}%)"),
            detail: format!(
                "Only {r}% of recent context came from the prompt cache. Long stable preambles and fewer context resets raise this."
            ),
            severity: "warn".into(),
        }),
        _ => None,
    }
}

/// A workspace whose turns are far more tool-heavy than your overall average.
fn tool_intensity(conn: &Connection, total_turns: i64) -> Option<Insight> {
    if total_turns < 30 {
        return None;
    }
    let overall: f64 = conn
        .query_row("SELECT AVG(tool_count) FROM turns", [], |r| {
            Ok(r.get::<_, Option<f64>>(0)?.unwrap_or(0.0))
        })
        .unwrap_or(0.0);
    if overall <= 0.0 {
        return None;
    }
    let (ws, avg, n): (Option<String>, f64, i64) = conn
        .query_row(
            "SELECT workspace, AVG(tool_count) a, COUNT(*) n FROM turns
             WHERE workspace IS NOT NULL
             GROUP BY workspace HAVING n >= 10 ORDER BY a DESC LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                    r.get(2)?,
                ))
            },
        )
        .ok()?;
    let ws = ws?;
    if avg < overall * 1.6 || avg < 3.0 {
        return None;
    }
    let _ = n;
    Some(Insight {
        id: "tool-intensity".into(),
        kind: "tools".into(),
        title: format!("{} is your most tool-heavy workspace", workspace_leaf(&ws)),
        detail: format!(
            "Turns there average {avg:.1} tool calls — {:.1}× your overall {overall:.1}. Agentic, multi-step work.",
            avg / overall
        ),
        severity: "info".into(),
    })
}

/// Spend routed off the metered Anthropic pool through a custom provider.
fn custom_provider_spend(conn: &Connection, total_cost: f64) -> Option<Insight> {
    if total_cost <= 0.0 {
        return None;
    }
    let off: f64 = conn
        .query_row(
            &format!(
                "SELECT SUM(CASE WHEN provider IS NOT NULL AND provider != 'anthropic'
                                 THEN {COST} ELSE 0 END) FROM turns"
            ),
            [],
            |r| Ok(r.get::<_, Option<f64>>(0)?.unwrap_or(0.0)),
        )
        .ok()?;
    if off <= 0.0 {
        return None;
    }
    let share = pct(off, total_cost);
    Some(Insight {
        id: "custom-provider-spend".into(),
        kind: "cost".into(),
        title: "You're routing turns off the metered pool".into(),
        detail: format!(
            "{} ({share}% of spend) ran through a custom provider instead of your Anthropic subscription.",
            usd(off)
        ),
        severity: "good".into(),
    })
}

/// Run every deterministic probe over the corpus and return the ones that
/// fired. Order is stable (probe order); the UI caps how many it shows.
#[tauri::command]
pub fn usage_insights(db: tauri::State<UsageDb>) -> Result<Vec<Insight>, String> {
    let conn = db.0.lock().map_err(|e| format!("usage db lock: {e}"))?;
    let (turns, cost) = corpus_size(&conn);
    if turns < 10 {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    out.extend(dominant_model(&conn, cost));
    out.extend(cost_sink_workspace(&conn, cost));
    out.extend(peak_window(&conn, turns, cost));
    out.extend(cache_efficiency(&conn));
    out.extend(tool_intensity(&conn, turns));
    out.extend(custom_provider_spend(&conn, cost));
    Ok(out)
}
