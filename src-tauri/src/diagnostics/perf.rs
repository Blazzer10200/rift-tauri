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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

/// Per-(model, effort) accumulator: (ttft vec, duration vec, cause→count tally).
type GroupAcc = std::collections::BTreeMap<
    (String, Option<String>),
    (Vec<u64>, Vec<u64>, std::collections::BTreeMap<String, usize>),
>;

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
    /// The effort FLAG this turn's child was keyed on ("low"/"medium"/"high"/
    /// "xhigh") — i.e. tier→flag, BEFORE the thinking gate. Not the tier string.
    #[serde(default)]
    pub effort: Option<String>,
    /// The effort actually sent as `--effort` after the thinking gate: when
    /// thinking is off, `send_effort_flag` floors any tier to "low". So a turn
    /// keyed `effort:"high"` with thinking off really ran `--effort low`. Without
    /// this the latency analysis can't tell a deep turn from a floored one.
    #[serde(default)]
    pub send_effort: Option<String>,
    /// Whether extended thinking was on for this turn. Pairs with `send_effort`
    /// to disambiguate slow-because-thinking from slow-because-cold.
    #[serde(default)]
    pub thinking_on: Option<bool>,

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
/// Bytes written to `turns.ndjson` since it was last (re)opened — tracked in
/// memory so a long-lived process can rotate periodically without a `stat` on
/// every write. `init_turns_log`'s own metadata check only ever runs once
/// (`OnceLock::get_or_init` semantics), so this is what makes rotation actually
/// happen for a session that outlives one 5MB fill.
static TURNS_LOG_BYTES: AtomicU64 = AtomicU64::new(0);
const TURNS_LOG_MAX_BYTES: u64 = 5 * 1024 * 1024;

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
    let start_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let oversized = start_len > TURNS_LOG_MAX_BYTES;
    let rotate_failed =
        oversized && std::fs::rename(&path, path.with_extension("ndjson.old")).is_err();
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true);
    if rotate_failed {
        opts.write(true).truncate(true);
    } else {
        opts.append(true);
    }
    let file = opts.open(&path).ok()?;
    // Seed the tracked byte-count from the just-opened file's actual size (0
    // after a rotate/truncate, else the pre-existing size we're appending to)
    // so `append_turn_perf` can rotate on crossing the threshold without a
    // `stat` per write.
    let seed = if rotate_failed || oversized { 0 } else { start_len };
    TURNS_LOG_BYTES.store(seed, Ordering::Relaxed);
    Some(Mutex::new(file))
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
        use std::io::{Seek, Write as _};
        let written = TURNS_LOG_BYTES.fetch_add(line.len() as u64, Ordering::Relaxed) + line.len() as u64;
        // Periodic rotation: the OnceLock init above only ever runs its size
        // check ONCE per process, so a long-lived session would otherwise never
        // rotate again. Re-check the cheap in-memory counter (no `stat`) on
        // every write instead; when it crosses the cap, truncate the live
        // handle in place and reset the counter.
        if written > TURNS_LOG_MAX_BYTES && f.set_len(0).is_ok() && f.seek(std::io::SeekFrom::Start(0)).is_ok() {
            TURNS_LOG_BYTES.store(0, Ordering::Relaxed);
        }
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
    /// p90 first-reply over WARM turns that STARTED in the last 24h — the
    /// "how is it running right now" basis. The lifetime warm p90 answers a
    /// different question; driving a "right now" banner off it keeps one slow
    /// afternoon on screen for weeks (found live 2026-07-10). None below 3
    /// recent samples.
    #[serde(default)]
    pub p90_ttft_text_recent_ms: Option<u64>,
    /// Count of warm turns with a first-reply measurement in the last 24h —
    /// the recent verdict's honest denominator.
    #[serde(default)]
    pub recent_turns_measured: usize,
    /// Aggregate cache-hit rate: Σcache_read / Σ(cache_read + input).
    pub cache_hit_rate: Option<f64>,
    /// Σ output tokens across all turns (a rough throughput proxy).
    pub total_output_tokens: u64,
    /// Median per-turn NON-API overhead (ms) = duration - cli_api: what Rift adds
    /// on top of the model's own API time (IPC, tool exec, plumbing). The honest
    /// answer to "is the latency Rift or the model" — a small value next to a
    /// large `avg_cli_api_ms` proves the wait is the model. None until turns carry
    /// the CLI's `duration_api_ms` (post-attrib history).
    #[serde(default)]
    pub p50_non_api_overhead_ms: Option<u64>,
    /// Mean model-API time (ms) over turns that reported it — the model's share,
    /// to read against `p50_non_api_overhead_ms`. None below any sample.
    #[serde(default)]
    pub avg_cli_api_ms: Option<u64>,
    /// Per-day cost buckets (UTC "YYYY-MM-DD" → usd), most-recent first, ≤30 days.
    pub cost_by_day: Vec<(String, f64)>,
    /// Per-model spend (model tag → Σ usd), highest first. Message share ≠
    /// dollar share — this is the honest "where the money goes" split.
    #[serde(default)]
    pub cost_by_model: Vec<(String, f64)>,
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

/// The "right now" window for the recent latency verdict. 24h: wide enough to
/// have samples on a normal dev day, narrow enough that yesterday's slow
/// afternoon ages out of the banner overnight.
const RECENT_WINDOW_MS: u64 = 24 * 60 * 60 * 1000;

/// Read `turns.ndjson` (+ `.old` if present) and compute the aggregate. Skips
/// malformed lines rather than failing — a partial write at the tail must not
/// blank the whole panel.
fn aggregate(lines: impl Iterator<Item = String>) -> TurnPerfStats {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    aggregate_at(lines, now_ms, None)
}

/// `aggregate` with an injected clock (tests pin the recent window) and an
/// optional display window: records older than `window_ms` are skipped from
/// EVERY aggregate (the AI Health range picker), not just the recent verdict.
fn aggregate_at(
    lines: impl Iterator<Item = String>,
    now_ms: u64,
    window_ms: Option<u64>,
) -> TurnPerfStats {
    use std::collections::BTreeMap;
    let recent_cutoff = now_ms.saturating_sub(RECENT_WINDOW_MS);
    let window_cutoff = window_ms.map(|w| now_ms.saturating_sub(w));

    let mut ttft_text: Vec<u64> = Vec::new();
    // Warm/cold split of first-reply (drives the warm-aware latency signal). A
    // record only joins one of these when it carries the post-WS6 `was_cold` tag;
    // untagged history stays in the all-turns `ttft_text` fallback.
    let mut ttft_text_warm: Vec<u64> = Vec::new();
    let mut ttft_text_cold: Vec<u64> = Vec::new();
    // Warm first-replies inside the recent window — the "right now" basis.
    let mut ttft_text_recent: Vec<u64> = Vec::new();
    let mut durations: Vec<u64> = Vec::new();
    // Non-API overhead per turn = duration_ms - cli_api_ms (Rift wall-clock minus
    // the model's own API time, as the CLI measures it). What Rift adds: IPC, tool
    // execution, stdin/stdout plumbing. Only collected when a turn carries both
    // numbers (post-attrib history); clamped at 0 (a turn whose tool work overlaps
    // API streaming can read slightly negative — that's not "negative overhead").
    let mut non_api_overhead: Vec<u64> = Vec::new();
    let mut cli_api_sum: u64 = 0;
    let mut cli_api_count: usize = 0;
    let mut cache_read_sum: u64 = 0;
    let mut input_sum: u64 = 0;
    let mut total_output: u64 = 0;
    let mut cost_by_day: BTreeMap<String, f64> = BTreeMap::new();
    let mut cost_by_model: BTreeMap<String, f64> = BTreeMap::new();
    // Per-day first-reply latencies (for the trend sparkline) and per-(model,
    // effort) latency+duration vecs (for the breakdown).
    let mut ttft_by_day: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    // Per group: (ttft vec, duration vec, cause→count tally for the modal cause).
    let mut by_group: GroupAcc = BTreeMap::new();
    let mut total = 0usize;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(rec) = serde_json::from_str::<TurnPerf>(trimmed) else { continue };
        // Display-window filter — a record outside the picked range contributes
        // to NOTHING (totals, days, groups), as if the log started at the cutoff.
        if let Some(cut) = window_cutoff {
            if rec.ts_start_ms < cut {
                continue;
            }
        }
        total += 1;
        // Per-model spend — powers "where the money goes" (cost share by model).
        if let (Some(model), Some(cost)) = (rec.model.as_deref(), rec.cost_usd) {
            *cost_by_model.entry(model.to_string()).or_default() += cost;
        }
        if let Some(v) = rec.ttft_text_ms {
            ttft_text.push(v);
            // Warm/cold split — only when the turn was tagged (post-WS6). An
            // untagged record contributes to the all-turns fallback only.
            match rec.was_cold {
                Some(true) => ttft_text_cold.push(v),
                Some(false) => {
                    ttft_text_warm.push(v);
                    if rec.ts_start_ms >= recent_cutoff {
                        ttft_text_recent.push(v);
                    }
                }
                None => {}
            }
        }
        if let Some(v) = rec.duration_ms {
            durations.push(v);
        }
        // Non-API overhead: only when the turn carried both its wall-clock and the
        // CLI's API time. saturating_sub clamps the rare overlap-negative to 0.
        if let (Some(dur), Some(api)) = (rec.duration_ms, rec.cli_api_ms) {
            non_api_overhead.push(dur.saturating_sub(api));
            cli_api_sum = cli_api_sum.saturating_add(api);
            cli_api_count += 1;
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
    ttft_text_recent.sort_unstable();
    let warm_turns_measured = ttft_text_warm.len();
    let cold_turns_measured = ttft_text_cold.len();
    durations.sort_unstable();
    non_api_overhead.sort_unstable();
    // Mean model-API time over turns that reported it — pairs with the overhead
    // p50 so the UI can say "model X ms vs Rift Y ms" at a glance.
    let avg_cli_api_ms = if cli_api_count > 0 {
        Some(cli_api_sum / cli_api_count as u64)
    } else {
        None
    };

    let cache_hit_rate = if cache_read_sum + input_sum > 0 {
        Some(cache_read_sum as f64 / (cache_read_sum + input_sum) as f64)
    } else {
        None
    };

    // Most-recent first, last 30 days.
    let cost_by_day: Vec<(String, f64)> = cost_by_day.into_iter().rev().take(30).collect();

    // Highest spender first; ties keep name order (BTreeMap iteration).
    let mut cost_by_model: Vec<(String, f64)> = cost_by_model.into_iter().collect();
    cost_by_model.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

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
    by_model.sort_by_key(|m| std::cmp::Reverse(m.turn_count));

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
        // Recent floor is 3 (not the lifetime 8): a 24h window on a normal dev
        // day holds a handful of turns; requiring 8 would leave the "right now"
        // verdict permanently dark for most users.
        p90_ttft_text_recent_ms: percentile(&ttft_text_recent, 0.90, 3),
        recent_turns_measured: ttft_text_recent.len(),
        cache_hit_rate,
        total_output_tokens: total_output,
        p50_non_api_overhead_ms: percentile(&non_api_overhead, 0.50, 1),
        avg_cli_api_ms,
        cost_by_day,
        cost_by_model,
        latency_p90_by_day,
        by_model,
        total_turns: total,
    }
}

/// Synchronous read+aggregate. Reads the rotated `.old` first (older history)
/// then the live file so day buckets accumulate across a rotation boundary.
/// `window_hours` narrows every aggregate to turns started inside the window
/// (AI Health's 24h/7d/30d range picker); None = full log, as ever.
pub fn query_turn_perf_sync(window_hours: Option<u32>) -> TurnPerfStats {
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
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    aggregate_at(all.into_iter(), now_ms, window_hours.map(|h| h as u64 * 3_600_000))
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
            send_effort: None,
            thinking_on: None,
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
            send_effort: None,
            thinking_on: None,
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
            send_effort: None,
            thinking_on: None,
            ttft_first_line_ms: None,
            pre_text_tool_ms: None,
            was_cold: Some(was_cold),
            dominant_cause: None,
            cli_ttft_ms: None,
            cli_api_ms: None,
        };
        serde_json::to_string(&r).unwrap()
    }

    // Like `rec_cold` but with an explicit start stamp so the recent-window
    // split can be pinned against the injected `aggregate_at` clock.
    fn rec_cold_at(ttft: u64, was_cold: bool, ts_start_ms: u64) -> String {
        let r = TurnPerf {
            ts_start_ms,
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
            send_effort: None,
            thinking_on: None,
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
    fn recent_p90_reads_only_the_last_24h_of_warm_turns() {
        let now = 1_700_000_000_000u64;
        let old = now - RECENT_WINDOW_MS - 60_000; // just outside the window
        let fresh = now - 60_000; // inside
        let lines = vec![
            // A slow stretch older than the window — must NOT drive "right now".
            rec_cold_at(30_000, false, old),
            rec_cold_at(28_000, false, old),
            rec_cold_at(27_000, false, old),
            // Today's pace: three fast warm turns + one cold (cold is excluded).
            rec_cold_at(1_000, false, fresh),
            rec_cold_at(1_200, false, fresh),
            rec_cold_at(1_400, false, fresh),
            rec_cold_at(9_000, true, fresh),
        ];
        let s = aggregate_at(lines.into_iter(), now, None);
        assert_eq!(s.recent_turns_measured, 3);
        assert_eq!(s.p90_ttft_text_recent_ms, Some(1_400));
        // Lifetime warm p90 keeps its own floor (8) — 6 warm turns → None.
        assert_eq!(s.p90_ttft_text_warm_ms, None);
    }

    #[test]
    fn display_window_excludes_old_records_from_every_aggregate() {
        let now = 1_700_000_000_000u64;
        let old = now - (8 * 24 * 3_600_000); // 8 days back
        let fresh = now - 3_600_000; // 1h back
        let lines = vec![
            rec_cold_at(20_000, false, old),
            rec_cold_at(1_000, false, fresh),
            rec_cold_at(1_200, false, fresh),
        ];
        // 7-day window: the 8-day-old record vanishes from totals AND groups.
        let s = aggregate_at(lines.into_iter(), now, Some(7 * 24 * 3_600_000));
        assert_eq!(s.total_turns, 2);
        assert_eq!(s.warm_turns_measured, 2);
        assert_eq!(s.by_model.len(), 1);
        assert_eq!(s.by_model[0].turn_count, 2);
        // Per-model spend counted only inside the window (2 × $0.01).
        assert_eq!(s.cost_by_model.len(), 1);
        assert!((s.cost_by_model[0].1 - 0.02).abs() < 1e-9);
    }

    #[test]
    fn recent_p90_floors_at_three_samples() {
        let now = 1_700_000_000_000u64;
        let lines = vec![
            rec_cold_at(1_000, false, now - 1_000),
            rec_cold_at(1_200, false, now - 2_000),
        ];
        let s = aggregate_at(lines.into_iter(), now, None);
        assert_eq!(s.recent_turns_measured, 2);
        assert_eq!(s.p90_ttft_text_recent_ms, None, "2 samples is below the recent floor");
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
    fn send_effort_and_thinking_round_trip_and_default_to_none() {
        // #69: a record stamped with the actual sent effort + thinking state must
        // serialize both, and a legacy line missing them must deserialize as None
        // (the serde(default) contract that keeps old turns.ndjson readable).
        let mut r: TurnPerf = serde_json::from_str(&rec(Some(100), Some(200), None, None)).unwrap();
        assert_eq!(r.send_effort, None, "base helper omits the new fields");
        assert_eq!(r.thinking_on, None);

        // A thinking-off "deep" turn really ran --effort low; stamp + round-trip it.
        r.effort = Some("high".into());
        r.send_effort = Some("low".into());
        r.thinking_on = Some(false);
        let back: TurnPerf = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back.effort.as_deref(), Some("high"), "keyed tier preserved");
        assert_eq!(back.send_effort.as_deref(), Some("low"), "floored flag preserved");
        assert_eq!(back.thinking_on, Some(false));
    }

    // Record carrying the CLI's server-side timing (duration + cli_api) so the
    // non-API overhead aggregation can be exercised.
    fn rec_attrib(duration_ms: u64, cli_api_ms: u64) -> String {
        let r = TurnPerf {
            ts_start_ms: 1_700_000_000_000,
            ttft_thinking_ms: None,
            ttft_text_ms: Some(2000),
            duration_ms: Some(duration_ms),
            input_tokens: Some(100),
            output_tokens: Some(100),
            cache_read_tokens: None,
            cache_create_tokens: None,
            cost_usd: Some(0.01),
            cache_hit_rate: None,
            session_id: "s".into(),
            result_subtype: Some("success".into()),
            model: Some("opus".into()),
            effort: Some("smart".into()),
            send_effort: None,
            thinking_on: None,
            ttft_first_line_ms: None,
            pre_text_tool_ms: None,
            was_cold: Some(false),
            dominant_cause: None,
            cli_ttft_ms: Some(2500),
            cli_api_ms: Some(cli_api_ms),
        };
        serde_json::to_string(&r).unwrap()
    }

    #[test]
    fn aggregate_non_api_overhead_split() {
        // Three turns: overhead = duration - cli_api → 1500, 1300, 1100.
        // p50 (median) = 1300; avg cli_api = (15000+18000+16000)/3 = 16333.
        let lines = vec![
            rec_attrib(16_500, 15_000),
            rec_attrib(19_300, 18_000),
            rec_attrib(17_100, 16_000),
            // A turn WITHOUT cli_api must not pollute the overhead stats.
            rec(Some(500), Some(9999), Some(900), Some(100)),
        ];
        let s = aggregate(lines.into_iter());
        assert_eq!(s.total_turns, 4);
        assert_eq!(s.p50_non_api_overhead_ms, Some(1300));
        assert_eq!(s.avg_cli_api_ms, Some((15_000 + 18_000 + 16_000) / 3));
    }

    #[test]
    fn aggregate_non_api_overhead_absent_without_cli_api() {
        // No turn carries cli_api → both attribution stats stay None (the UI then
        // hides the model-vs-Rift line rather than inventing a number).
        let lines = vec![
            rec(Some(500), Some(2000), Some(900), Some(100)),
            rec(Some(300), Some(1000), Some(800), Some(200)),
        ];
        let s = aggregate(lines.into_iter());
        assert_eq!(s.p50_non_api_overhead_ms, None);
        assert_eq!(s.avg_cli_api_ms, None);
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
