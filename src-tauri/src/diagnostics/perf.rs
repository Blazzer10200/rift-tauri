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

    // ── Latency cause attribution (WS6) ───────────────────────────────────────
    // The clocks below decompose `ttft_text_ms` into WHERE the wait went, so the
    // advisor can name the real lever instead of inferring it from an aggregate.
    // All `serde(default)` so pre-WS6 NDJSON lines deserialize as None.
    /// Turn-start → first ANY frame from the CLI. The gap before the model emits
    /// anything = prompt upload + (on a cold spawn) process warm-up.
    #[serde(default)]
    pub ttft_first_line_ms: Option<u64>,
    /// Total ms a tool was in-flight BEFORE the first text token. Large = tool
    /// round-trips, not the model, blocked the reply.
    #[serde(default)]
    pub pre_text_tool_ms: Option<u64>,
    /// This turn ran on a freshly-spawned child (cold), not a warm-pool reuse.
    #[serde(default)]
    pub was_cold: Option<bool>,
    /// The largest contributor to the first-reply wait, computed at finalisation:
    /// "thinking" | "upload" | "cold_start" | "tools" | "none". The advisor maps
    /// each to a concrete lever. None when `ttft_text_ms` is absent (no reply).
    #[serde(default)]
    pub dominant_cause: Option<String>,

    // ── CLI self-reported timing (result-frame, server-side truth) ───────────
    // The CLI stamps its OWN timing on the result frame: `ttft_ms` (turn-start →
    // first model token, the API's true latency) and `duration_api_ms` (total
    // wall-time spent in API calls). These are the model's cost, measured by the
    // CLI, independent of anything Rift does. Surfacing them lets the AI Health
    // pane separate "model API time" from "Rift overhead" (= duration_ms minus
    // cli_api_ms) instead of attributing the whole turn to one or the other.
    // `serde(default)` so pre-existing NDJSON lines deserialize as None.
    #[serde(default)]
    pub cli_ttft_ms: Option<u64>,
    #[serde(default)]
    pub cli_api_ms: Option<u64>,
}

/// Decompose a turn's first-reply wait into its largest contributor. Pure so it
/// can be unit-tested independently of the turn loop. Returns one of
/// "thinking" | "upload" | "cold_start" | "tools" | "none".
///
/// Model: `ttft_text` = pre-model wait (first_line) + thinking time + tool time.
/// We attribute to whichever phase is largest AND clears a noise floor — a snappy
/// turn (every phase small) is "none", never a misleading culprit.
pub fn classify_latency_cause(
    ttft_text_ms: Option<u64>,
    ttft_thinking_ms: Option<u64>,
    ttft_first_line_ms: Option<u64>,
    pre_text_tool_ms: Option<u64>,
    was_cold: bool,
    cache_hit_rate: Option<f64>,
) -> Option<String> {
    let text = ttft_text_ms?;
    // Below this, the reply was fast enough that no phase is worth blaming.
    const FAST_FLOOR_MS: u64 = 4000;
    if text < FAST_FLOOR_MS {
        return Some("none".to_string());
    }
    let first_line = ttft_first_line_ms.unwrap_or(0);
    let tools = pre_text_tool_ms.unwrap_or(0);
    // Thinking phase = first-text minus first-thinking (the model reasoned before
    // it spoke). Only when thinking actually started before text.
    let thinking = match ttft_thinking_ms {
        Some(t) if text > t => text - t,
        _ => 0,
    };
    // Pre-model wait: time before the first frame, minus any tool time that fell
    // inside it (tools open after the first frame, so first_line is purely wait).
    let pre_model = first_line;

    // Pick the largest phase; ties resolve toward the cheapest-to-explain lever.
    let mut best = ("none", 0u64);
    for (name, ms) in [("thinking", thinking), ("tools", tools), ("upload", pre_model)] {
        if ms > best.1 {
            best = (name, ms);
        }
    }
    // A dominant pre-model wait on a cold spawn is warm-up, not context size.
    // A warm spawn with low cache-hit points at context re-upload instead.
    if best.0 == "upload" {
        if was_cold {
            return Some("cold_start".to_string());
        }
        // Warm but slow first frame with a healthy cache is just network/queue —
        // don't pin it on the user's context. Only call it "upload" when the
        // cache is actually missing (context being re-billed).
        if cache_hit_rate.map(|c| c >= 0.7).unwrap_or(false) {
            return Some("none".to_string());
        }
    }
    // The winning phase must be a real majority of the wait, else it's diffuse.
    if best.1 * 2 < text {
        return Some("none".to_string());
    }
    Some(best.0.to_string())
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
    /// p50/p90 first-reply over WARM turns only (was_cold=false). The cold turn
    /// of each session pays a one-time model warm-up + cache-create tax that is
    /// NOT steady-state latency — folding it into the health verdict makes the
    /// score cry wolf over a spawn cost the user can't act on. The UI drives the
    /// latency signal off these, falling back to the all-turns p90 only when no
    /// turn carries the (post-WS6) `was_cold` tag yet. None below the sample floor.
    #[serde(default)]
    pub p90_ttft_text_warm_ms: Option<u64>,
    #[serde(default)]
    pub p50_ttft_text_warm_ms: Option<u64>,
    /// Count of WARM turns that carried a first-reply measurement — the honest
    /// denominator for the latency verdict's sample floor (a red score over 3
    /// turns is noise, not a diagnosis).
    #[serde(default)]
    pub warm_turns_measured: usize,
    /// Count of turns explicitly tagged cold (was_cold=true). Lets the UI show a
    /// separate "first reply of a session" note instead of blaming the model.
    #[serde(default)]
    pub cold_turns_measured: usize,
    /// p90 first-reply over COLD turns only — the warm-up cost, shown as context
    /// (not a problem to fix). None below the sample floor.
    #[serde(default)]
    pub p90_ttft_text_cold_ms: Option<u64>,
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
    /// Modal latency cause for slow turns in this group ("thinking"/"upload"/
    /// "cold_start"/"tools"), with how many turns voted for it. None when no turn
    /// in the group carried a cause, or the modal cause is "none" (group is fast).
    /// Lets the advisor say "9 of your 12 slow Opus turns were thinking" — a
    /// measured fact, not an inference from the aggregate p90.
    pub dominant_cause: Option<String>,
    pub dominant_cause_turns: usize,
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
    // Warm/cold split of first-reply (drives the warm-aware latency signal). A
    // record only joins one of these when it carries the post-WS6 `was_cold` tag;
    // untagged history stays in the all-turns `ttft_text` fallback.
    let mut ttft_text_warm: Vec<u64> = Vec::new();
    let mut ttft_text_cold: Vec<u64> = Vec::new();
    let mut durations: Vec<u64> = Vec::new();
    let mut cache_read_sum: u64 = 0;
    let mut input_sum: u64 = 0;
    let mut total_output: u64 = 0;
    let mut cost_by_day: BTreeMap<String, f64> = BTreeMap::new();
    // Per-day first-reply latencies (for the trend sparkline) and per-(model,
    // effort) latency+duration vecs (for the breakdown).
    let mut ttft_by_day: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    // Per group: (ttft vec, duration vec, cause→count tally for the modal cause).
    let mut by_group: BTreeMap<
        (String, Option<String>),
        (Vec<u64>, Vec<u64>, BTreeMap<String, usize>),
    > = BTreeMap::new();
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
            // Warm/cold split — only when the turn was tagged (post-WS6). An
            // untagged record contributes to the all-turns fallback only.
            match rec.was_cold {
                Some(true) => ttft_text_cold.push(v),
                Some(false) => ttft_text_warm.push(v),
                None => {}
            }
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
            // Tally the cause, ignoring "none" (a fast turn has no culprit to vote).
            if let Some(c) = rec.dominant_cause.as_deref() {
                if c != "none" {
                    *g.2.entry(c.to_string()).or_default() += 1;
                }
            }
        }
    }

    ttft_text.sort_unstable();
    ttft_text_warm.sort_unstable();
    ttft_text_cold.sort_unstable();
    let warm_turns_measured = ttft_text_warm.len();
    let cold_turns_measured = ttft_text_cold.len();
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
        .map(|((model, effort), (mut ttft, mut dur, causes))| {
            ttft.sort_unstable();
            dur.sort_unstable();
            // Modal cause = the most-voted culprit; ties break by name order
            // (deterministic). None when no slow turn cast a vote.
            let (dominant_cause, dominant_cause_turns) = causes
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(c, n)| (Some(c.clone()), *n))
                .unwrap_or((None, 0));
            ModelPerfStats {
                model,
                effort,
                p50_ttft_text_ms: percentile(&ttft, 0.50, 1),
                p90_ttft_text_ms: percentile(&ttft, 0.90, 5),
                p50_duration_ms: percentile(&dur, 0.50, 1),
                turn_count: ttft.len().max(dur.len()),
                dominant_cause,
                dominant_cause_turns,
            }
        })
        .collect();
    by_model.sort_by(|a, b| b.turn_count.cmp(&a.turn_count));

    TurnPerfStats {
        p50_ttft_text_ms: percentile(&ttft_text, 0.50, 1),
        p90_ttft_text_ms: percentile(&ttft_text, 0.90, 10),
        p50_duration_ms: percentile(&durations, 0.50, 1),
        p90_duration_ms: percentile(&durations, 0.90, 10),
        // Warm p90 floored at 8 measured warm turns — the sample floor for a
        // health verdict (a red over a handful of turns is noise). p50 at 1.
        p90_ttft_text_warm_ms: percentile(&ttft_text_warm, 0.90, 8),
        p50_ttft_text_warm_ms: percentile(&ttft_text_warm, 0.50, 1),
        warm_turns_measured,
        cold_turns_measured,
        // Cold p90 is informational context — a single cold turn is a real point.
        p90_ttft_text_cold_ms: percentile(&ttft_text_cold, 0.90, 1),
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
            ttft_first_line_ms: None,
            pre_text_tool_ms: None,
            was_cold: None,
            dominant_cause: None,
            cli_ttft_ms: None,
            cli_api_ms: None,
        };
        serde_json::to_string(&r).unwrap()
    }

    // Like `rec` but stamps model/effort (+ optional cause) so by_model grouping
    // and the modal-cause rollup can be exercised.
    fn rec_m(model: &str, effort: &str, ttft: Option<u64>, dur: Option<u64>) -> String {
        rec_mc(model, effort, ttft, dur, None)
    }
    fn rec_mc(model: &str, effort: &str, ttft: Option<u64>, dur: Option<u64>, cause: Option<&str>) -> String {
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
            ttft_first_line_ms: None,
            pre_text_tool_ms: None,
            was_cold: None,
            dominant_cause: cause.map(|c| c.to_string()),
            cli_ttft_ms: None,
            cli_api_ms: None,
        };
        serde_json::to_string(&r).unwrap()
    }

    // Like `rec` but stamps `was_cold` so the warm/cold latency split can be tested.
    fn rec_cold(ttft: u64, was_cold: bool) -> String {
        let r = TurnPerf {
            ts_start_ms: 1_700_000_000_000,
            ttft_thinking_ms: None,
            ttft_text_ms: Some(ttft),
            duration_ms: Some(ttft * 2),
            input_tokens: Some(100),
            output_tokens: Some(100),
            cache_read_tokens: None,
            cache_create_tokens: None,
            cost_usd: Some(0.01),
            cache_hit_rate: None,
            session_id: "s".into(),
            result_subtype: Some("success".into()),
            model: Some("opus".into()),
            effort: Some("deep".into()),
            ttft_first_line_ms: None,
            pre_text_tool_ms: None,
            was_cold: Some(was_cold),
            dominant_cause: None,
            cli_ttft_ms: None,
            cli_api_ms: None,
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

    #[test]
    fn classify_fast_turn_is_none() {
        // Under the fast floor → no culprit even if a phase is nominally largest.
        assert_eq!(
            classify_latency_cause(Some(2500), Some(200), Some(300), None, false, None),
            Some("none".into())
        );
    }

    #[test]
    fn classify_thinking_dominates() {
        // 12s reply, thinking started at 1s → 11s thinking = the wait. Lever: effort.
        assert_eq!(
            classify_latency_cause(Some(12_000), Some(1000), Some(800), None, false, None),
            Some("thinking".into())
        );
    }

    #[test]
    fn classify_cold_start_vs_upload() {
        // Big pre-model wait, no thinking/tools. Cold spawn → cold_start.
        assert_eq!(
            classify_latency_cause(Some(10_000), None, Some(9000), None, true, None),
            Some("cold_start".into())
        );
        // Same shape but warm + cache missing → context re-upload.
        assert_eq!(
            classify_latency_cause(Some(10_000), None, Some(9000), None, false, Some(0.2)),
            Some("upload".into())
        );
        // Warm + healthy cache → just network/queue, not the user's fault → none.
        assert_eq!(
            classify_latency_cause(Some(10_000), None, Some(9000), None, false, Some(0.9)),
            Some("none".into())
        );
    }

    #[test]
    fn classify_tools_dominate() {
        // 11s reply, 8s of it tool round-trips before first text → tools.
        assert_eq!(
            classify_latency_cause(Some(11_000), None, Some(500), Some(8000), false, None),
            Some("tools".into())
        );
    }

    #[test]
    fn classify_diffuse_is_none() {
        // Slow but no single phase is a majority → none (not a misleading blame).
        assert_eq!(
            classify_latency_cause(Some(10_000), Some(7000), Some(2000), Some(1500), false, None),
            Some("none".into())
        );
    }

    #[test]
    fn aggregate_splits_warm_and_cold_latency() {
        // 8 warm turns (all 3s) + 2 cold turns (30s, the warm-up tax). The warm
        // p90 must reflect ONLY the snappy warm turns; cold is broken out separately
        // and never poisons the warm signal.
        let mut lines: Vec<String> = (0..8).map(|_| rec_cold(3000, false)).collect();
        lines.push(rec_cold(30_000, true));
        lines.push(rec_cold(28_000, true));
        // An untagged record (pre-WS6 history) joins neither warm nor cold.
        lines.push(rec(Some(50_000), Some(60_000), None, None));
        let s = aggregate(lines.into_iter());
        assert_eq!(s.warm_turns_measured, 8);
        assert_eq!(s.cold_turns_measured, 2);
        // Warm p90 over eight 3s turns is 3s — the cold 30s turns are excluded.
        assert_eq!(s.p90_ttft_text_warm_ms, Some(3000));
        assert_eq!(s.p50_ttft_text_warm_ms, Some(3000));
        // Cold p90 is the warm-up cost, surfaced as context not a problem.
        assert_eq!(s.p90_ttft_text_cold_ms, Some(30_000));
        // The all-turns p90 still includes everything (untagged history relies on it).
        assert!(s.p90_ttft_text_ms.unwrap() >= 28_000);
    }

    #[test]
    fn warm_p90_below_floor_is_none() {
        // Only 3 warm turns — below the 8-sample floor → warm p90 is None so the UI
        // shows "still learning" rather than a red verdict over noise.
        let lines: Vec<String> = (0..3).map(|_| rec_cold(3000, false)).collect();
        let s = aggregate(lines.into_iter());
        assert_eq!(s.warm_turns_measured, 3);
        assert_eq!(s.p90_ttft_text_warm_ms, None);
        assert_eq!(s.p50_ttft_text_warm_ms, Some(3000)); // p50 floor is 1
    }

    #[test]
    fn aggregate_rolls_up_modal_cause() {
        let lines = vec![
            rec_mc("opus", "deep", Some(9000), Some(40000), Some("thinking")),
            rec_mc("opus", "deep", Some(8000), Some(38000), Some("thinking")),
            rec_mc("opus", "deep", Some(7000), Some(30000), Some("upload")),
            rec_mc("opus", "deep", Some(2000), Some(9000), Some("none")), // fast → no vote
        ];
        let s = aggregate(lines.into_iter());
        let opus = &s.by_model[0];
        assert_eq!(opus.dominant_cause.as_deref(), Some("thinking"));
        assert_eq!(opus.dominant_cause_turns, 2); // "none" didn't vote
    }
}
