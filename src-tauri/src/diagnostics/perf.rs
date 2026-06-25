//! Per-turn performance telemetry — structured capture + append-only NDJSON.
//!
//! `turn.rs` already times TTFT / first-thinking / first-text / result-duration,
//! but only as throwaway `log::info!` strings, and never captures the token /
//! cache / cost data the CLI streams on the `result` frame. This module gives
//! those numbers a typed home (`TurnPerf`), a persistent sink (`turns.ndjson`
//! beside `rift.log`), and a query path for AI Health aggregates (p50/p90
//! latency, cache-hit rate, cost trend).
//!
//! The sink mirrors the rotating file-log pattern in `mod.rs` (lazy `OnceLock`
//! init, size-based rotation, mutex poison-recovery) and is fire-and-forget:
//! `append_turn_perf` offloads to `spawn_blocking` so file I/O never touches the
//! turn's async hot path.

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

/// One record per completed assistant turn — one NDJSON line in `turns.ndjson`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnPerf {
    /// Unix epoch ms at turn start (wall clock; used for cost-by-day bucketing).
    pub ts_start_ms: u64,

    // ── Latency milestones (ms, relative to turn start) ──────────────────────
    /// Time to the first thinking token. None when the turn had no thinking.
    pub ttft_thinking_ms: Option<u64>,
    /// Time to the first text token. None when the turn produced no text.
    pub ttft_text_ms: Option<u64>,
    /// Total turn-start → result-frame elapsed.
    pub duration_ms: Option<u64>,

    // ── Token counts (result-frame `usage` object) ──────────────────────────
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_create_tokens: Option<u64>,

    // ── Cost (result-frame `total_cost_usd`) ─────────────────────────────────
    pub cost_usd: Option<f64>,

    // ── Derived (computed at finalisation, not from the CLI) ─────────────────
    /// cache_read / (cache_read + input) — fraction of input served from cache.
    pub cache_hit_rate: Option<f64>,

    // ── Metadata ─────────────────────────────────────────────────────────────
    pub session_id: String,
    /// Result-frame `subtype` (e.g. "success", "error_max_turns").
    pub result_subtype: Option<String>,

    // ── Turn config (known at turn time, not from the CLI) ────────────────────
    /// Model key for this turn ("opus"/"sonnet"/"haiku"/"fable"). `serde(default)`
    /// so historical NDJSON lines (pre-tagging) deserialize as None.
    #[serde(default)]
    pub model: Option<String>,
    /// Effort tier for this turn ("none"/"quick"/"smart"/"deep"/"ultra").
    #[serde(default)]
    pub effort: Option<String>,
}

static TURNS_LOG: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();

/// `<appLogDir>/turns.ndjson` — beside `rift.log` (mirrors `write_crash_report`).
fn turns_log_path() -> Option<std::path::PathBuf> {
    let log = super::app_log_path()?;
    Some(log.parent()?.join("turns.ndjson"))
}

/// Open (and lazily rotate) the NDJSON sink. Mirrors `init_file_log` in `mod.rs`:
/// size-based rotation to a single `.old` backup, truncate-on-open if the rotate
/// rename fails so a stuck backup can't grow the file unbounded.
fn init_turns_log() -> Option<Mutex<std::fs::File>> {
    let path = turns_log_path()?;
    const MAX_BYTES: u64 = 5 * 1024 * 1024;
    let oversized = std::fs::metadata(&path).map(|m| m.len() > MAX_BYTES).unwrap_or(false);
    let rotate_failed =
        oversized && std::fs::rename(&path, path.with_extension("ndjson.old")).is_err();
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true);
    if rotate_failed {
        opts.write(true).truncate(true);
    } else {
        opts.append(true);
    }
    opts.open(&path).ok().map(Mutex::new)
}

/// Append one record to `turns.ndjson`, fire-and-forget. File I/O is offloaded to
/// `spawn_blocking` so the caller (the turn's async streaming loop) never blocks
/// on a disk write. Best-effort: a write failure is dropped (telemetry must never
/// fail a user turn).
pub fn append_turn_perf(rec: TurnPerf) {
    tokio::task::spawn_blocking(move || {
        let Some(cell) = TURNS_LOG.get_or_init(init_turns_log).as_ref() else { return };
        // Poison recovery — a panic mid-write must not blackout the sink (same
        // pattern as file_log_write in mod.rs).
        let mut f = match cell.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let Ok(mut line) = serde_json::to_string(&rec) else { return };
        line.push('\n');
        use std::io::Write as _;
        let _ = f.write_all(line.as_bytes());
        let _ = f.flush();
    });
}

// ─── Query / aggregation ─────────────────────────────────────────────────────

/// AI Health aggregate over all persisted turns. Percentiles are None below the
/// sample floor (p90 needs ≥10 turns) so the UI can show "—" rather than a noisy
/// single-sample number.
#[derive(Debug, Clone, Serialize)]
pub struct TurnPerfStats {
    pub p50_ttft_text_ms: Option<u64>,
    pub p90_ttft_text_ms: Option<u64>,
    pub p50_duration_ms: Option<u64>,
    pub p90_duration_ms: Option<u64>,
    /// Aggregate cache-hit rate: Σcache_read / Σ(cache_read + input).
    pub cache_hit_rate: Option<f64>,
    /// Σ output tokens across all turns (a rough throughput proxy).
    pub total_output_tokens: u64,
    /// Per-day cost buckets (UTC "YYYY-MM-DD" → usd), most-recent first, ≤30 days.
    pub cost_by_day: Vec<(String, f64)>,
    /// Per-day p90 first-reply latency (UTC "YYYY-MM-DD" → ms), most-recent
    /// first, ≤14 days. Drives the trend sparkline. p90 with min_samples=1 so a
    /// single-turn day still shows a point (direction, not clinical accuracy).
    pub latency_p90_by_day: Vec<(String, Option<u64>)>,
    /// Per-(model, effort) latency breakdown, busiest group first. Lets the
    /// advisor cite "Opus turns are 22s vs Sonnet 4s" instead of a global p90.
    /// Groups whose model is unknown (pre-tagging history) are dropped.
    pub by_model: Vec<ModelPerfStats>,
    pub total_turns: usize,
}

/// One (model, effort) latency group within `TurnPerfStats::by_model`.
#[derive(Debug, Clone, Serialize)]
pub struct ModelPerfStats {
    pub model: String,
    pub effort: Option<String>,
    pub p50_ttft_text_ms: Option<u64>,
    pub p90_ttft_text_ms: Option<u64>,
    pub p50_duration_ms: Option<u64>,
    pub turn_count: usize,
}

/// `pct` in [0,1]. `min_samples` is the floor below which the result is None.
fn percentile(sorted: &[u64], pct: f64, min_samples: usize) -> Option<u64> {
    if sorted.len() < min_samples || sorted.is_empty() {
        return None;
    }
    let idx = ((sorted.len() as f64 * pct).ceil() as usize).saturating_sub(1);
    sorted.get(idx).copied()
}

/// Read `turns.ndjson` (+ `.old` if present) and compute the aggregate. Skips
/// malformed lines rather than failing — a partial write at the tail must not
/// blank the whole panel.
fn aggregate(lines: impl Iterator<Item = String>) -> TurnPerfStats {
    use std::collections::BTreeMap;

    let mut ttft_text: Vec<u64> = Vec::new();
    let mut durations: Vec<u64> = Vec::new();
    let mut cache_read_sum: u64 = 0;
    let mut input_sum: u64 = 0;
    let mut total_output: u64 = 0;
    let mut cost_by_day: BTreeMap<String, f64> = BTreeMap::new();
    // Per-day first-reply latencies (for the trend sparkline) and per-(model,
    // effort) latency+duration vecs (for the breakdown).
    let mut ttft_by_day: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut by_group: BTreeMap<(String, Option<String>), (Vec<u64>, Vec<u64>)> = BTreeMap::new();
    let mut total = 0usize;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(rec) = serde_json::from_str::<TurnPerf>(trimmed) else { continue };
        total += 1;
        if let Some(v) = rec.ttft_text_ms {
            ttft_text.push(v);
        }
        if let Some(v) = rec.duration_ms {
            durations.push(v);
        }
        if let (Some(r), Some(i)) = (rec.cache_read_tokens, rec.input_tokens) {
            cache_read_sum = cache_read_sum.saturating_add(r);
            input_sum = input_sum.saturating_add(i);
        }
        if let Some(o) = rec.output_tokens {
            total_output = total_output.saturating_add(o);
        }
        let date = chrono::DateTime::from_timestamp((rec.ts_start_ms / 1000) as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        if let Some(cost) = rec.cost_usd {
            *cost_by_day.entry(date.clone()).or_default() += cost;
        }
        if let Some(t) = rec.ttft_text_ms {
            ttft_by_day.entry(date).or_default().push(t);
        }
        // Per-(model, effort) breakdown — only records carrying a model tag
        // (post-WS5); historical untagged lines are skipped, not bucketed.
        if let Some(model) = rec.model.clone() {
            let g = by_group.entry((model, rec.effort.clone())).or_default();
            if let Some(t) = rec.ttft_text_ms {
                g.0.push(t);
            }
            if let Some(d) = rec.duration_ms {
                g.1.push(d);
            }
        }
    }

    ttft_text.sort_unstable();
    durations.sort_unstable();

    let cache_hit_rate = if cache_read_sum + input_sum > 0 {
        Some(cache_read_sum as f64 / (cache_read_sum + input_sum) as f64)
    } else {
        None
    };

    // Most-recent first, last 30 days.
    let cost_by_day: Vec<(String, f64)> = cost_by_day.into_iter().rev().take(30).collect();

    // Per-day p90 latency, most-recent first, ≤14 days. min_samples=1: a
    // single-turn day still yields a point (trend direction, not clinical p90).
    let latency_p90_by_day: Vec<(String, Option<u64>)> = ttft_by_day
        .into_iter()
        .rev()
        .take(14)
        .map(|(day, mut vs)| {
            vs.sort_unstable();
            (day, percentile(&vs, 0.90, 1))
        })
        .collect();

    // Per-(model, effort) breakdown, busiest group first.
    let mut by_model: Vec<ModelPerfStats> = by_group
        .into_iter()
        .map(|((model, effort), (mut ttft, mut dur))| {
            ttft.sort_unstable();
            dur.sort_unstable();
            ModelPerfStats {
                model,
                effort,
                p50_ttft_text_ms: percentile(&ttft, 0.50, 1),
                p90_ttft_text_ms: percentile(&ttft, 0.90, 5),
                p50_duration_ms: percentile(&dur, 0.50, 1),
                turn_count: ttft.len().max(dur.len()),
            }
        })
        .collect();
    by_model.sort_by(|a, b| b.turn_count.cmp(&a.turn_count));

    TurnPerfStats {
        p50_ttft_text_ms: percentile(&ttft_text, 0.50, 1),
        p90_ttft_text_ms: percentile(&ttft_text, 0.90, 10),
        p50_duration_ms: percentile(&durations, 0.50, 1),
        p90_duration_ms: percentile(&durations, 0.90, 10),
        cache_hit_rate,
        total_output_tokens: total_output,
        cost_by_day,
        latency_p90_by_day,
        by_model,
        total_turns: total,
    }
}

/// Synchronous read+aggregate. Reads the rotated `.old` first (older history)
/// then the live file so day buckets accumulate across a rotation boundary.
pub fn query_turn_perf_sync() -> TurnPerfStats {
    use std::io::{BufRead, BufReader};

    let mut all: Vec<String> = Vec::new();
    if let Some(path) = turns_log_path() {
        for p in [path.with_extension("ndjson.old"), path] {
            if let Ok(f) = std::fs::File::open(&p) {
                for line in BufReader::new(f).lines().map_while(Result::ok) {
                    all.push(line);
                }
            }
        }
    }
    aggregate(all.into_iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(ttft: Option<u64>, dur: Option<u64>, cr: Option<u64>, inp: Option<u64>) -> String {
        let r = TurnPerf {
            ts_start_ms: 1_700_000_000_000,
            ttft_thinking_ms: None,
            ttft_text_ms: ttft,
            duration_ms: dur,
            input_tokens: inp,
            output_tokens: Some(100),
            cache_read_tokens: cr,
            cache_create_tokens: None,
            cost_usd: Some(0.01),
            cache_hit_rate: None,
            session_id: "s".into(),
            result_subtype: Some("success".into()),
            model: None,
            effort: None,
        };
        serde_json::to_string(&r).unwrap()
    }

    // Like `rec` but stamps model/effort so by_model grouping can be exercised.
    fn rec_m(model: &str, effort: &str, ttft: Option<u64>, dur: Option<u64>) -> String {
        let r = TurnPerf {
            ts_start_ms: 1_700_000_000_000,
            ttft_thinking_ms: None,
            ttft_text_ms: ttft,
            duration_ms: dur,
            input_tokens: Some(100),
            output_tokens: Some(100),
            cache_read_tokens: None,
            cache_create_tokens: None,
            cost_usd: Some(0.01),
            cache_hit_rate: None,
            session_id: "s".into(),
            result_subtype: Some("success".into()),
            model: Some(model.into()),
            effort: Some(effort.into()),
        };
        serde_json::to_string(&r).unwrap()
    }

    #[test]
    fn percentile_floor_and_index() {
        let v = vec![100, 200, 300, 400, 500];
        assert_eq!(percentile(&v, 0.50, 1), Some(300));
        assert_eq!(percentile(&v, 0.90, 1), Some(500));
        // p90 floored at 10 samples → None with only 5.
        assert_eq!(percentile(&v, 0.90, 10), None);
        assert_eq!(percentile(&[], 0.5, 1), None);
    }

    #[test]
    fn aggregate_computes_cache_and_cost() {
        let lines = vec![
            rec(Some(500), Some(2000), Some(900), Some(100)),
            rec(Some(300), Some(1000), Some(800), Some(200)),
            "garbage-not-json".to_string(),
            "".to_string(),
        ];
        let s = aggregate(lines.into_iter());
        assert_eq!(s.total_turns, 2);
        assert_eq!(s.p50_ttft_text_ms, Some(300)); // ceil(2*.5)-1 = idx0 of sorted [300,500]
        // cache_read=1700, input=300 → 1700/2000 = 0.85
        assert!((s.cache_hit_rate.unwrap() - 0.85).abs() < 1e-9);
        assert_eq!(s.total_output_tokens, 200);
        assert_eq!(s.cost_by_day.len(), 1); // both records same day
    }

    #[test]
    fn aggregate_groups_by_model() {
        let lines = vec![
            rec_m("opus", "deep", Some(8000), Some(40000)),
            rec_m("opus", "deep", Some(4000), Some(20000)),
            rec_m("sonnet", "smart", Some(1000), Some(3000)),
            rec(Some(500), Some(2000), Some(900), Some(100)), // no model → excluded
        ];
        let s = aggregate(lines.into_iter());
        assert_eq!(s.total_turns, 4);
        // Only the 3 model-stamped records form groups; the unstamped one drops out.
        assert_eq!(s.by_model.len(), 2);
        // Sorted by turn_count desc → opus (2) leads sonnet (1).
        let opus = &s.by_model[0];
        assert_eq!(opus.model, "opus");
        assert_eq!(opus.effort.as_deref(), Some("deep"));
        assert_eq!(opus.turn_count, 2);
        assert_eq!(opus.p50_ttft_text_ms, Some(4000)); // ceil(2*.5)-1 = idx0 of [4000,8000]
        assert_eq!(opus.p90_ttft_text_ms, None); // p90 floored at 5 samples
        assert_eq!(s.by_model[1].model, "sonnet");
        assert_eq!(s.by_model[1].turn_count, 1);
    }
}
