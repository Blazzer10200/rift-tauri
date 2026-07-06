//! Reusable metrics primitive (Phase 4) — so new instrumentation is a one-liner
//! instead of a hand-rolled `emit_with_fields` block like the Phase-2 sites.
//!
//! Two shapes:
//! * `metric!("warm_pool.hit")` — bump a named counter by 1 (or
//!   `metric!("mcp.bytes", n)` to add `n`).
//! * `timed!("mcp.tool", { … })` — time a block; on scope exit it records the
//!   elapsed ms into a histogram AND emits a `diag://event` so it shows live in
//!   the console.
//!
//! Both feed a process-global registry (`query_metrics` reads it for a future
//! health panel) and the bus, so a metric is visible three ways at once:
//! live event, counter total, latency histogram. The registry is a plain
//! `Mutex<HashMap>` — metrics are low-frequency control-plane events, not a
//! per-sample hot path (the rate cap + the call-site's own discretion keep them
//! sparse; see the `timed!` note on sampling).

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use serde::Serialize;

/// One histogram's running summary — count + sum + min/max, enough for a mean
/// and a range without storing every sample (the console keeps the raw events).
#[derive(Debug, Clone, Default, Serialize)]
pub struct HistoSummary {
    pub count: u64,
    pub sum_ms: u64,
    pub min_ms: u64,
    pub max_ms: u64,
}

#[derive(Debug, Default)]
struct Registry {
    counters: HashMap<String, u64>,
    histos: HashMap<String, HistoSummary>,
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

/// Add `n` to a named counter. Used by `metric!`. Cheap + lock-guarded; mutex
/// poison recovers in place (a metric must never panic a real code path).
pub fn incr(name: &str, n: u64) {
    let mut g = match registry().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *g.counters.entry(name.to_string()).or_insert(0) += n;
}

/// Record one timing sample (ms) into a named histogram. Used by `timed!`.
pub fn record_ms(name: &str, ms: u64) {
    let mut g = match registry().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let h = g.histos.entry(name.to_string()).or_insert_with(|| HistoSummary {
        min_ms: u64::MAX,
        ..Default::default()
    });
    h.count += 1;
    h.sum_ms = h.sum_ms.saturating_add(ms);
    h.min_ms = h.min_ms.min(ms);
    h.max_ms = h.max_ms.max(ms);
}

/// Snapshot of all metrics — counters + histogram summaries — for a query
/// command / health panel. Sorted for stable rendering.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub counters: Vec<(String, u64)>,
    pub histos: Vec<(String, HistoSummary)>,
}

pub fn snapshot() -> MetricsSnapshot {
    let g = match registry().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let mut counters: Vec<(String, u64)> = g.counters.iter().map(|(k, v)| (k.clone(), *v)).collect();
    counters.sort_by(|a, b| a.0.cmp(&b.0));
    let mut histos: Vec<(String, HistoSummary)> =
        g.histos.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    histos.sort_by(|a, b| a.0.cmp(&b.0));
    MetricsSnapshot { counters, histos }
}

/// Backing fn for `timed!` — records the histogram sample AND emits a live
/// event so the span shows in the console. Separate from `record_ms` so the
/// macro stays a thin wrapper and tests can hit the registry without the bus.
/// `site` is the caller's `file!()` — passed in from the macro expansion, NOT
/// `file!()` here, which would always resolve to metrics.rs and misattribute
/// every span to this file regardless of where it was timed.
pub fn record_span(name: &str, ms: u64, site: &str) {
    record_ms(name, ms);
    super::emit_with_fields(
        super::DiagStage::Log,
        super::DiagLevel::Debug,
        Some("metric"),
        Some(site),
        name,
        serde_json::json!({ "metric": name, "dur_ms": ms }),
    );
}

/// Bump a named counter. `metric!("name")` adds 1; `metric!("name", n)` adds n.
/// Fire-and-forget; never fails. The counter total is read via `snapshot()`;
/// the bump itself does NOT emit an event (counters are read on demand, not
/// streamed — a per-increment event would flood the bus).
#[macro_export]
macro_rules! metric {
    ($name:expr) => {
        $crate::diagnostics::metrics::incr($name, 1)
    };
    ($name:expr, $n:expr) => {
        $crate::diagnostics::metrics::incr($name, $n)
    };
}

/// Time a block and record its elapsed ms into a histogram + emit a live event.
/// `let v = timed!("mcp.tool", { do_work() });` — the block's value is returned,
/// so it drops in transparently around an existing expression.
#[macro_export]
macro_rules! timed {
    ($name:expr, $body:block) => {{
        let __timed_t0 = ::std::time::Instant::now();
        let __timed_out = $body;
        let __timed_ms = __timed_t0.elapsed().as_millis() as u64;
        // `file!()` here expands at the CALL SITE, so the span attributes to the
        // instrumented code, not metrics.rs.
        $crate::diagnostics::metrics::record_span($name, __timed_ms, file!());
        __timed_out
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incr_accumulates() {
        // Use a unique name so the global registry can't be polluted by another
        // test running in the same process.
        incr("test.incr.a", 1);
        incr("test.incr.a", 1);
        incr("test.incr.a", 3);
        let snap = snapshot();
        let v = snap.counters.iter().find(|(k, _)| k == "test.incr.a").map(|(_, v)| *v);
        assert_eq!(v, Some(5));
    }

    #[test]
    fn record_ms_tracks_count_sum_min_max() {
        record_ms("test.histo.b", 10);
        record_ms("test.histo.b", 30);
        record_ms("test.histo.b", 20);
        let snap = snapshot();
        let h = snap.histos.iter().find(|(k, _)| k == "test.histo.b").map(|(_, v)| v.clone()).unwrap();
        assert_eq!(h.count, 3);
        assert_eq!(h.sum_ms, 60);
        assert_eq!(h.min_ms, 10);
        assert_eq!(h.max_ms, 30);
    }

    #[test]
    fn timed_macro_returns_block_value_and_records() {
        let out = timed!("test.timed.c", { 2 + 2 });
        assert_eq!(out, 4);
        let snap = snapshot();
        let h = snap.histos.iter().find(|(k, _)| k == "test.timed.c").map(|(_, v)| v.clone());
        assert!(h.is_some(), "timed! should record a histogram sample");
        assert_eq!(h.unwrap().count, 1);
    }

    #[test]
    fn metric_macro_bumps_counter() {
        metric!("test.metric.d");
        metric!("test.metric.d", 4);
        let snap = snapshot();
        let v = snap.counters.iter().find(|(k, _)| k == "test.metric.d").map(|(_, v)| *v);
        assert_eq!(v, Some(5));
    }
}
