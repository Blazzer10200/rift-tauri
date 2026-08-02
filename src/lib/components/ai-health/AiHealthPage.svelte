<script lang="ts">
  // AI Health — the observe layer (Stage 1). Surfaces what Rift already knows
  // about a user's plan + usage, and seats the "Analyze my usage" advisor
  // (Stage 2) that reasons over it via the user's own Claude. Coaches newcomers
  // in plain English; charts are supporting cast.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { HeartPulse, Gauge, Sparkles, ArrowRight, Wrench, Loader2, AlertTriangle, Check, Undo2, Wifi, Snowflake, Plug } from "lucide-svelte";
  import PageHero from "../shared/PageHero.svelte";
  import { usage, type LimitWindow, type AdviceApply } from "../../state/usage.svelte";
  import { assistant, type ModelSel } from "../../state/assistant.svelte";
  import { summarizeSession } from "../../state/assistant/telemetry";
  import {
    summarize, perModel, streaks, topModel, type ConvoStat,
  } from "../home/statsHelpers";

  // All-time usage from the persisted convo store (same source the Home stats
  // panel reads) — gives day-one advice something to chew on even before this
  // session has many turns.
  let stats = $state<ConvoStat[] | null>(null);
  let statsError = $state<string | null>(null);

  // B2 — cross-session turn-perf aggregate from the persisted turns.ndjson:
  // p50/p90 latency, cache-hit rate, cost-by-day. Absent until the first turn
  // is recorded; a parse failure just leaves the panel hidden.
  type ModelPerfStats = {
    model: string;
    effort: string | null;
    p50_ttft_text_ms: number | null;
    p90_ttft_text_ms: number | null;
    p50_duration_ms: number | null;
    turn_count: number;
    dominant_cause: string | null;
    dominant_cause_turns: number;
  };
  type TurnPerfStats = {
    p50_ttft_text_ms: number | null;
    p90_ttft_text_ms: number | null;
    p50_duration_ms: number | null;
    p90_duration_ms: number | null;
    // Warm-only first-reply (excludes the one-time cold-start warm-up tax) —
    // the health verdict reads these so a spawn cost doesn't flash red. Null
    // below the 8-warm-turn floor. Cold split is informational context.
    p90_ttft_text_warm_ms: number | null;
    p50_ttft_text_warm_ms: number | null;
    warm_turns_measured: number;
    cold_turns_measured: number;
    p90_ttft_text_cold_ms: number | null;
    cache_hit_rate: number | null;
    total_output_tokens: number;
    // Latency attribution: model API time (avg_cli_api_ms) vs what Rift adds on
    // top (p50_non_api_overhead_ms = duration - cli_api). Null until turns carry
    // the CLI's server-side timing. Answers "is the wait Rift or the model".
    p50_non_api_overhead_ms: number | null;
    avg_cli_api_ms: number | null;
    p90_ttft_text_recent_ms: number | null;
    recent_turns_measured: number;
    cost_by_day: [string, number][];
    cost_by_model: [string, number][];
    latency_p90_by_day: [string, number | null][];
    by_model: ModelPerfStats[];
    total_turns: number;
  };
  let perfStats = $state<TurnPerfStats | null>(null);

  // ── Time-range picker (cont.300) ── narrows the Speed/Spend aggregates to a
  // window BE-side (query_turn_perf window_hours). The 24h live-verdict fields
  // are computed inside whatever window is picked (every option ⊇ 24h), so the
  // verdict strip stays correct across ranges. Persisted per user.
  type PerfWindow = "24h" | "7d" | "30d" | "all";
  const PERF_WINDOW_HOURS: Record<PerfWindow, number | null> = { "24h": 24, "7d": 168, "30d": 720, all: null };
  const PERF_WINDOW_LABEL: Record<PerfWindow, string> = {
    "24h": "the last 24 hours", "7d": "the last 7 days", "30d": "the last 30 days", all: "all time",
  };
  const PW_KEY = "rift.aihealth.perfWindow.v1";
  let perfWindow = $state<PerfWindow>("7d");
  try {
    const s = localStorage.getItem(PW_KEY);
    if (s === "24h" || s === "7d" || s === "30d" || s === "all") perfWindow = s;
  } catch { /* noop */ }
  async function refreshPerf() {
    try {
      perfStats = await invoke<TurnPerfStats>("query_turn_perf", { windowHours: PERF_WINDOW_HOURS[perfWindow] });
    } catch { /* absent panel is the degraded state */ }
  }
  function setPerfWindow(w: PerfWindow) {
    perfWindow = w;
    try { localStorage.setItem(PW_KEY, w); } catch { /* noop */ }
    void refreshPerf();
  }

  // Ticks every 30s so the "fetched Xs ago" label stays live without a render
  // on every frame. Set once on mount, cleared on destroy.
  let nowTick = $state(Date.now());

  // WS3: real, frame-driven analyze stage from the backend's progress events
  // ("spawned" | "thinking" | "writing"). Floors the cosmetic step ticker so the
  // visible step jumps forward the moment the CLI actually reaches that phase,
  // instead of guessing on a timer. "" between runs.
  let analyzeStage = $state<"" | "spawned" | "thinking" | "writing">("");
  // Map a stage to the earliest ANALYZE_STEPS index it justifies.
  const STAGE_FLOOR: Record<string, number> = { thinking: 3, writing: 4 };

  onMount(() => {
    const poll = () => void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
    poll();
    void invoke<ConvoStat[]>("assistant_stats")
      .then((s) => { stats = s; })
      .catch((e) => { statsError = String(e); });
    void refreshPerf();
    // WS3 listener — backend emits a stage per real stream frame.
    const unlistenP = listen<{ stage: "spawned" | "thinking" | "writing" }>(
      "usage-analyze-progress",
      (e) => { analyzeStage = e.payload.stage; },
    );
    // Re-poll limits every 3min (BE caches 60s, so this is network-cheap) and
    // tick the "ago" clock every 30s.
    const repoll = setInterval(poll, 3 * 60_000);
    const clock = setInterval(() => { nowTick = Date.now(); }, 30_000);
    return () => {
      clearInterval(repoll); clearInterval(clock);
      void unlistenP.then((un) => un());
    };
  });

  // "fetched Xs ago" — recomputes when nowTick advances or limits refresh.
  const fetchedAgo = $derived.by(() => {
    const at = usage.rateLimits?.fetchedAt;
    if (!at) return "";
    const secs = Math.max(0, Math.round((nowTick - at) / 1000));
    if (secs < 45) return "just now";
    const mins = Math.round(secs / 60);
    return mins < 60 ? `${mins}m ago` : `${Math.round(mins / 60)}h ago`;
  });

  const fmtMs = (ms: number | null) => (ms == null ? "—" : ms < 1000 ? `${Math.round(ms)}ms` : `${(ms / 1000).toFixed(1)}s`);
  // Has enough recorded turns to be worth showing (avoids a one-sample panel).
  const hasPerf = $derived(!!perfStats && perfStats.total_turns >= 3);

  // ── Latency sparkline (WS4) ── per-day p90 first-reply, charted as a tiny
  // inline SVG polyline (no library). Drops null days, needs ≥2 points to draw a
  // line. Y is inverted (SVG origin top-left) and normalised to the window's own
  // min/max so the shape reads as "trending up/down", not absolute ms.
  const SPARK_W = 100;
  const SPARK_H = 28;
  const latencySpark = $derived.by(() => {
    const days = (perfStats?.latency_p90_by_day ?? []).filter((d): d is [string, number] => d[1] != null);
    if (days.length < 2) return null;
    const vals = days.map((d) => d[1]);
    const min = Math.min(...vals), max = Math.max(...vals);
    const span = max - min || 1;
    const stepX = SPARK_W / (days.length - 1);
    const pts = vals.map((v, i) => {
      const x = i * stepX;
      const y = SPARK_H - 2 - ((v - min) / span) * (SPARK_H - 4);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    // Trend tint: compare last point to first — rising latency is bad (hot),
    // falling is good (ok), flat-ish is neutral.
    const delta = vals[vals.length - 1] - vals[0];
    const tint = Math.abs(delta) < span * 0.15 ? "" : delta > 0 ? "hot" : "ok";
    return { line: pts.join(" "), tint, first: vals[0], last: vals[vals.length - 1] };
  });

  // ── Plan-limit rail ──
  const limitRows = $derived.by(() => {
    const rl = usage.rateLimits;
    if (!rl) return [] as { k: string; w: LimitWindow }[];
    const out: { k: string; w: LimitWindow }[] = [];
    if (rl.fiveHour) out.push({ k: "5-hour window", w: rl.fiveHour });
    if (rl.sevenDay) out.push({ k: "Weekly · all models", w: rl.sevenDay });
    if (rl.sevenDayOpus) out.push({ k: "Weekly · Opus", w: rl.sevenDayOpus });
    if (rl.sevenDaySonnet) out.push({ k: "Weekly · Sonnet", w: rl.sevenDaySonnet });
    return out;
  });
  function zone(u: number): string {
    return u < 60 ? "ok" : u < 85 ? "warn" : "hot";
  }
  function fmtReset(iso: string | null): string {
    if (!iso) return "";
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "";
    const mins = Math.max(0, Math.round((d.getTime() - Date.now()) / 60000));
    if (mins < 60) return `resets in ${mins}m`;
    const h = Math.floor(mins / 60);
    if (h < 48) return `resets in ${h}h ${mins % 60}m`;
    return `resets ${d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })}`;
  }

  // ── Plan pace (cont.300) ── linear projection from elapsed window time:
  // "at this pace you land at ~X% by reset". Claimed only when ≥15% of the
  // window has elapsed (early extrapolation lies) and the projection is high
  // enough to matter (<25% headroom is obvious, the line would be noise).
  const WINDOW_LEN_MS: Record<string, number> = { "5-hour window": 5 * 3_600_000 };
  function paceFor(k: string, w: LimitWindow): { pct: number; hot: boolean } | null {
    if (!w.resetsAt || w.utilization <= 0) return null;
    const len = WINDOW_LEN_MS[k] ?? 7 * 24 * 3_600_000; // weekly rows default 7d
    const reset = Date.parse(w.resetsAt);
    if (!Number.isFinite(reset)) return null;
    const remaining = reset - nowTick;
    if (remaining <= 0 || remaining >= len) return null;
    const elapsed = len - remaining;
    if (elapsed < len * 0.15) return null;
    const projected = Math.round(w.utilization * (len / elapsed));
    if (projected < 25) return null;
    return { pct: Math.min(projected, 999), hot: projected >= 100 };
  }

  // ── Spend bars (cont.300) ── vertical per-day chart, oldest→newest. Value
  // labels ride the peak + the newest bar; the rest stay clean.
  const spendBars = $derived.by(() => {
    const days = (perfStats?.cost_by_day ?? []).slice(0, 14).reverse();
    if (days.length < 2) return null;
    const max = Math.max(...days.map((d) => d[1]));
    if (max <= 0) return null;
    const total = days.reduce((a, d) => a + d[1], 0);
    let maxSeen = false;
    return {
      total,
      bars: days.map(([day, cost], i) => {
        const isMax = !maxSeen && cost === max && (maxSeen = true);
        return {
          day, cost,
          h: Math.max(4, Math.round((cost / max) * 100)),
          dow: "SMTWTFS"[new Date(`${day}T00:00:00`).getDay()] ?? "",
          labeled: isMax || i === days.length - 1,
        };
      }),
    };
  });

  // ── Per-model rollups (cont.300) ── the advisor always saw these; now the
  // user does too. Spend share (≠ message share) + latency by model·effort.
  // The log holds mixed tag eras ("opus" send keys AND full "claude-fable-5"
  // ids) — normalize to the family label and MERGE, or the same model shows
  // as two rows.
  const modelLabel = (m: string) => {
    const t = m.toLowerCase();
    if (t.includes("fable")) return "Fable";
    if (t.includes("opus")) return "Opus";
    if (t.includes("sonnet")) return "Sonnet";
    if (t.includes("haiku")) return "Haiku";
    // Claude-only product now — any non-Claude legacy id left in the historical
    // log folds into one "Other" bucket rather than being featured by name.
    return "Other";
  };
  const modelSpend = $derived.by(() => {
    const merged = new Map<string, number>();
    for (const [m, c] of perfStats?.cost_by_model ?? []) {
      const label = modelLabel(m);
      merged.set(label, (merged.get(label) ?? 0) + c);
    }
    const rows = [...merged.entries()].sort((a, b) => b[1] - a[1]);
    const total = rows.reduce((a, r) => a + r[1], 0);
    if (total <= 0) return [];
    return rows.slice(0, 4).map(([label, c]) => ({ label, share: c / total, usd: c }));
  });
  const modelLat = $derived.by(() =>
    (perfStats?.by_model ?? []).filter((g) => g.turn_count >= 3).slice(0, 4));

  // ── This-session snapshot (live, in-memory) ── pure rollup, no Date.now /
  // full-turns bundling that snapshot() would re-do every render.
  const session = $derived(summarizeSession(assistant.telemetry.turns, assistant.telemetry.events));

  // ── All-time rollup (persisted) ──
  const totals = $derived(stats ? summarize(stats) : null);
  const models = $derived(stats ? perModel(stats) : []);
  // The "where your usage goes" breakdown lists only models that round to ≥1% —
  // a "0%" row (a few messages on a since-retired model) is dead clutter. Keep at
  // least the top row so the list never empties on a tiny sample.
  const modelsShown = $derived(models.filter((m, i) => i === 0 || Math.round(m.share * 100) >= 1));
  const streak = $derived(stats ? streaks(stats, Date.now()) : { current: 0, longest: 0 });
  const top = $derived(stats ? topModel(stats) : null);
  const hasHistory = $derived(!!totals && totals.sessions > 0);

  const fmtUsd = (n: number) => (n < 0.01 && n > 0 ? "<$0.01" : `$${n.toFixed(2)}`);
  const fmtNum = (n: number) => n.toLocaleString();

  // Hero chip = closest-to-limit window, the "am I OK?" glance.
  const peakLimit = $derived.by(() => {
    let max = 0;
    for (const r of limitRows) max = Math.max(max, r.w.utilization);
    return Math.round(max);
  });

  // ── Advisor signals ── pre-digested verdicts so the model reasons over a
  // labelled signal ("slow") rather than re-thresholding raw ms every call.
  // p90 first-reply: <4s ok · <9s slow · ≥9s degraded (matches the felt-latency
  // bands in the latency doctrine). cache-hit <40% = thrash when there's history.
  //
  // G1 — the latency verdict reads the WARM-ONLY p90 (cold-start warm-up excluded),
  // so a one-time spawn cost the user can't act on never flashes the score red.
  // The cold turn of each session pays model warm-up + a 40-50K cache-create tax;
  // folding it into the verdict made the score cry wolf. Fallback to the all-turns
  // p90 only while no turn carries the (post-WS6) was_cold tag yet — older history.
  // G2 — below the sample floor the warm p90 is null (backend min_samples=8), so
  // `latencySignal` is null → the UI shows a "still learning" state, not a red
  // verdict over a handful of turns.
  // G6 (2026-07-10) — recent-first: the live verdict must describe TODAY, not
  // the lifetime log. The all-history warm p90 kept "API is slow right now" on
  // screen for days after one slow afternoon (and even at boot, before any turn
  // this session). Lifetime numbers still DISPLAY (recent:false), but only a
  // last-24h basis may tint/alarm — see the `latencySignal` gate below.
  const latencyP90Source = $derived.by((): { ms: number | null; warm: boolean; recent: boolean } => {
    const recent = perfStats?.p90_ttft_text_recent_ms;
    if (recent != null) return { ms: recent, warm: true, recent: true };
    const warm = perfStats?.p90_ttft_text_warm_ms;
    if (warm != null) return { ms: warm, warm: true, recent: false };
    // Warm p90 is null. TWO very different cases:
    //  (a) tagged data exists but is below the warm floor (warm+cold turns seen)
    //      → DON'T fall back to the all-turns p90: it's cold-poisoned, the exact
    //      number we're excluding. Return null so the UI shows "still learning".
    //  (b) no tagged data at all (pure pre-WS6 legacy history) → the all-turns
    //      p90 is all we have; use it (already floored at 10 by the backend).
    const tagged = (perfStats?.warm_turns_measured ?? 0) + (perfStats?.cold_turns_measured ?? 0);
    if (tagged > 0) return { ms: null, warm: false, recent: false };
    return { ms: perfStats?.p90_ttft_text_ms ?? null, warm: false, recent: false };
  });
  const latencySignal = $derived.by(() => {
    // Only a recent (last-24h) basis may claim a live verdict — lifetime
    // numbers answer "how has it been", never "how is it right now".
    if (!latencyP90Source.recent) return null;
    const p90 = latencyP90Source.ms;
    if (p90 == null) return null;
    return p90 < 4000 ? "ok" : p90 < 9000 ? "slow" : "degraded";
  });
  // G3 — "still learning" when we have perf data but not enough WARM turns to
  // trust a latency verdict. Distinct from "no data at all" (perfStats null).
  const latencyLearning = $derived.by(() => {
    if (!perfStats) return false;
    // We have records but nothing displayable yet (below every floor). A
    // merely-STALE lifetime value is not "learning" — it renders with a
    // "lifetime" chip instead (see the Speed tile).
    return latencyP90Source.ms == null && perfStats.total_turns > 0;
  });
  // ── MCP servers (#93-4) ── the active tab's latest init-frame server list.
  // Same source /mcp reads; null until a turn has run this session.
  const mcpRows = $derived(assistant.activeTab?.mcpServers ?? null);
  const mcpTint = (status: string): string =>
    status === "connected" ? "ok"
    : status === "needs-auth" ? "warn"
    : status === "failed" || status === "disconnected" ? "hot"
    : "";

  // G5 — is the slow first-reply the API or Rift? A large COLD p90 alongside a
  // snappy warm p90 means the wait is one-time spawn/warm-up, not steady-state.
  // A slow WARM p90 with a healthy cache points upstream (Anthropic API/queue),
  // not at the user's setup — the most reassuring thing we can tell them.
  const latencyAttribution = $derived.by((): string | null => {
    if (latencySignal == null || latencySignal === "ok") return null;
    const cache = perfStats?.cache_hit_rate;
    // Warm + slow + healthy cache = upstream. (A cache miss would mean context
    // re-upload, which the advisor handles separately.)
    if (latencyP90Source.warm && cache != null && cache >= 0.7) {
      return "This looks like the Anthropic API being slow, not your Rift setup — it usually passes.";
    }
    return null;
  });
  // Model-vs-Rift split (cont.219): the single most reassuring fact when a turn
  // "drags" — most of the wait is the model thinking, not Rift. Reads the CLI's
  // own API time (avg_cli_api_ms) against the overhead Rift adds on top
  // (p50_non_api_overhead_ms). Only shown once both exist AND the model clearly
  // dominates (≥2× overhead), so the claim is always true for this user's data.
  const splitAttribution = $derived.by((): { rift: string; pct: number } | null => {
    const api = perfStats?.avg_cli_api_ms;
    const overhead = perfStats?.p50_non_api_overhead_ms;
    if (api == null || overhead == null || api <= 0) return null;
    if (api < overhead * 2) return null;
    const pct = Math.round((api / (api + overhead)) * 100);
    return { rift: fmtMs(overhead), pct };
  });
  // Show the cold-start aside only when warm is genuinely faster than cold
  // (≥2s gap) so "keeping a chat going stays fast" is a true claim, not hollow
  // (if warm is also slow, that's a real signal the verdict already owns).
  const showColdNote = $derived.by(() => {
    if (!latencyP90Source.warm) return false;
    const cold = perfStats?.p90_ttft_text_cold_ms;
    const warm = perfStats?.p90_ttft_text_warm_ms;
    if (cold == null || warm == null || (perfStats?.cold_turns_measured ?? 0) === 0) return false;
    return cold - warm >= 2000;
  });
  const cacheSignal = $derived.by(() => {
    const c = perfStats?.cache_hit_rate;
    if (c == null || (perfStats?.total_turns ?? 0) < 5) return null;
    return c < 0.4 ? "thrash" : c < 0.7 ? "fair" : "good";
  });
  const rateLimitRisk = $derived(peakLimit >= 85 ? "hot" : peakLimit >= 60 ? "warn" : "ok");

  // Inline tile verdicts (WS1): map each signal onto the shared ok|warn|hot color
  // tokens (same ones `.ah-fill` uses) plus a one-word badge. "" tint = no tint
  // (signal absent / too few turns) so the tile renders neutral.
  const latencyTint = $derived(latencySignal === "ok" ? "ok" : latencySignal === "slow" ? "warn" : latencySignal === "degraded" ? "hot" : "");
  const latencyVerdict = $derived(latencySignal === "ok" ? "snappy" : latencySignal === "slow" ? "slow" : latencySignal === "degraded" ? "laggy" : "");
  const cacheTint = $derived(cacheSignal === "good" ? "ok" : cacheSignal === "fair" ? "warn" : cacheSignal === "thrash" ? "hot" : "");
  const cacheVerdict = $derived(cacheSignal === "good" ? "efficient" : cacheSignal === "fair" ? "fair" : cacheSignal === "thrash" ? "low" : "");

  // ── Overall health score ── the honest "am I OK?" glance. Composes the three
  // independent dimensions (latency / cache / plan-usage) into ONE verdict where
  // the WORST dimension wins — a health summary should surface the live problem,
  // never average it into a green that hides it. Dimensions with no signal yet
  // (too few turns, limits not loaded) don't drag the score; the verdict is
  // computed only over dimensions we can actually read. Null until at least one
  // dimension reports, so the hero chip can fall back to "Loading".
  const RANK: Record<string, number> = { ok: 0, warn: 1, hot: 2 };
  const healthScore = $derived.by(() => {
    // Each dimension carries its live value so the verdict strip can show the
    // metric inline (one health line) instead of a separate, redundant chip row.
    const dims: { k: string; tint: string; v: string }[] = [];
    // Latency dim reads the WARM p90 (the value the verdict is actually based on),
    // so the metric shown next to the verdict matches the verdict — not the
    // cold-poisoned all-turns number.
    if (latencyTint) dims.push({ k: "Latency", tint: latencyTint, v: fmtMs(latencyP90Source.ms) });
    if (cacheTint && perfStats?.cache_hit_rate != null) dims.push({ k: "Cache", tint: cacheTint, v: `${Math.round(perfStats.cache_hit_rate * 100)}%` });
    // Only when a real usage window reported — a non-null rateLimits with all
    // windows null (Pro plan, or pre-data) must NOT contribute a false "ok".
    if (limitRows.length > 0) dims.push({ k: "Plan", tint: rateLimitRisk, v: `${peakLimit}%` });
    if (dims.length === 0) return null;
    const worst = dims.reduce((a, b) => (RANK[b.tint] > RANK[a.tint] ? b : a));
    // Upstream-latency reframe: when the ONLY problem dimension is Latency and
    // we've attributed it to the Anthropic API (not the user's setup), the top
    // strip must not blare "Action needed" — there's nothing the user can do, and
    // the Speed section already says it "usually passes". Otherwise the verdict
    // contradicts its own body. Demote the headline to an informational "API is
    // slow right now" rather than a user-actionable alarm.
    const flaggedDims = dims.filter((d) => d.tint !== "ok");
    // Latency flagged but attributed upstream (Anthropic API, not the user's
    // setup) must never be what a red "Action needed" headline points at — there's
    // nothing the user can do about it. Two cases:
    //  • latency is the SOLE flag → demote the whole headline to a calm info line.
    //  • latency is worst but something ELSE is also flagged → lead the headline
    //    with that OTHER (actionable) dim, and add a short "slow replies are
    //    upstream" reassurance tail so the strip never blames the user's setup.
    const latencyUpstream = !!latencyAttribution;
    const nonLatencyFlagged = flaggedDims.filter((d) => d.k !== "Latency");
    const latencyIsSoleUpstream = latencyUpstream
      && flaggedDims.length === 1 && flaggedDims[0].k === "Latency";
    const headlineDim = latencyUpstream && nonLatencyFlagged.length > 0
      ? nonLatencyFlagged.reduce((a, b) => (RANK[b.tint] > RANK[a.tint] ? b : a))
      : worst;
    const tint = headlineDim.tint;
    const label = latencyIsSoleUpstream ? "API is slow right now"
      : tint === "ok" ? "Healthy" : tint === "warn" ? "Needs a look" : "Action needed";
    // Green note names only the dimensions actually checked (some are absent
    // below their sample floor), so it never claims a clean bill on data it
    // didn't have. Oxford-join the lowercased dimension names.
    const okNames = dims.map((d) => d.k.toLowerCase());
    const okList = okNames.length === 1 ? okNames[0]
      : okNames.length === 2 ? `${okNames[0]} and ${okNames[1]}`
      : `${okNames.slice(0, -1).join(", ")}, and ${okNames[okNames.length - 1]}`;
    // When the headline speaks to a real actionable dim AND latency is also
    // flagged upstream, tack on the reassurance so the strip isn't read as
    // blaming the user's setup for the slow replies.
    const upstreamTail = latencyUpstream && headlineDim.k !== "Latency"
      ? " Slow replies are on Anthropic's side, not your setup." : "";
    const note = latencyIsSoleUpstream ? "The wait is on Anthropic's side, not your setup — it usually clears on its own."
      : tint === "ok"
      ? `${okList.charAt(0).toUpperCase()}${okList.slice(1)} ${okNames.length === 1 ? "looks" : "look"} good.`
      : `${headlineDim.k} ${tint === "hot" ? "needs attention" : "is worth a look"}.${upstreamTail}`;
    // Problem dimensions get a labeled value pill on the right of the strip; when
    // all-clear, the dimension dots stand in (nothing to flag).
    const flagged = flaggedDims;
    // Display tint softens the headline to amber (warn) only for the sole-upstream
    // case — a red "hot" strip reads as user-actionable breakage, but there's
    // nothing to fix. The per-dimension flag pill keeps its true (hot) tint, so the
    // real latency number is still honestly shown as high.
    const displayTint = latencyIsSoleUpstream ? "warn" : tint;
    return { tint: displayTint, label, note, dims, flagged };
  });

  // ── Advisor ── assemble the snapshot the backend reasons over. Limits +
  // this-session rollup + all-time totals/per-model + the model lineup. Kept as
  // a plain object so cross-session history is additive later (append more).
  function buildSnapshot(): string {
    return JSON.stringify({
      // The live harness knobs the advisor can recommend changing. Authoritative
      // here (effort + model live in localStorage post-F48, not config.json), so
      // the model sees the real current values to avoid no-op suggestions.
      // authMode tells the advisor whether per-turn DOLLARS are a coherent lever:
      // an API key bills pay-per-token (a $ cap stops spend), a subscription is
      // governed by usage-limit windows (a $ cap does nothing). maxBudgetUsd is
      // only meaningful — and only sent — in api-key mode.
      currentSetup: {
        thinkingEnabled: assistant.thinkingEnabled,
        effortDefault: assistant.thinkingEffort,
        model: modelKey(assistant.model),
        authMode: assistant.hasApiKey ? "api-key" : "subscription",
        ...(assistant.hasApiKey ? { maxBudgetUsd: assistant.maxBudgetUsd } : {}),
      },
      planLimits: usage.rateLimits,
      thisSession: session,
      allTime: totals
        ? { ...totals, perModel: models.map((m) => ({ model: m.label, share: Math.round(m.share * 100), cost: m.cost })) }
        : null,
      streak,
      // ── Latency / throughput telemetry ── the data behind the latency-doctor
      // role: real measured per-turn perf so the advisor can DIAGNOSE slowness
      // (cold start, effort mismatch, context bloat, cache thrash), not just
      // restate cost. perfStats is the persisted p50/p90 aggregate; costTrend is
      // the 7-day cost-by-day series for spike detection.
      perf: perfStats
        ? {
            p50FirstReplyMs: perfStats.p50_ttft_text_ms,
            p90FirstReplyMs: perfStats.p90_ttft_text_ms,
            // Warm-only first-reply — the STEADY-STATE latency the advisor should
            // judge. p90FirstReplyMs (all turns) is poisoned by the one-time
            // cold-start tax; lead with warm and treat cold as the separate
            // warm-up cost it is. Null until 8 warm turns accrue.
            p90FirstReplyWarmMs: perfStats.p90_ttft_text_warm_ms,
            p50FirstReplyWarmMs: perfStats.p50_ttft_text_warm_ms,
            warmTurnsMeasured: perfStats.warm_turns_measured,
            coldTurnsMeasured: perfStats.cold_turns_measured,
            p90FirstReplyColdMs: perfStats.p90_ttft_text_cold_ms,
            p50TurnMs: perfStats.p50_duration_ms,
            p90TurnMs: perfStats.p90_duration_ms,
            cacheHitRate: perfStats.cache_hit_rate,
            totalOutputTokens: perfStats.total_output_tokens,
            turnsMeasured: perfStats.total_turns,
            // Per (model, effort) latency so the advisor can pin slowness to a
            // specific lever ("Opus/deep p50 first-reply 22s vs Sonnet/smart 4s")
            // instead of blaming an aggregate. Top 4 groups by turn count.
            byModel: perfStats.by_model.slice(0, 4).map((m) => ({
              model: m.model,
              effort: m.effort,
              p50FirstReplyMs: m.p50_ttft_text_ms,
              p90FirstReplyMs: m.p90_ttft_text_ms,
              p50TurnMs: m.p50_duration_ms,
              turns: m.turn_count,
              // WS6 measured root cause: the modal reason this group's slow turns
              // were slow + how many voted. Lets the advisor name the lever from
              // fact ("9 of 12 slow Opus turns were thinking") not inference.
              dominantCause: m.dominant_cause,
              dominantCauseTurns: m.dominant_cause_turns,
            })),
          }
        : null,
      costTrend: perfStats?.cost_by_day.slice(0, 7) ?? [],
      // Pre-digested verdicts: the advisor leads with these, then cites the raw
      // number from perf/planLimits that justifies the call. `latency` is the
      // WARM-aware verdict (cold excluded); `latencyBasis` tells the advisor
      // whether the verdict rests on warm-tagged data or the all-turns fallback
      // (so it doesn't over-trust a cold-poisoned number on older history).
      signals: {
        latency: latencySignal,
        latencyBasis: latencyP90Source.recent ? "recent-24h"
          : latencyP90Source.warm ? "warm-lifetime"
          : perfStats ? "all-turns-fallback" : null,
        cache: cacheSignal,
        rateLimitRisk,
      },
    });
  }
  async function analyze() {
    await usage.analyzeUsage(buildSnapshot(), assistant.hasApiKey);
  }

  // ── "Analyzing…" card ── a rotating set of plain-English steps so the wait
  // (the first run warms the warm pool, ~a minute) feels alive instead of hung.
  // Cosmetic only — the steps are illustrative, not a real progress signal.
  const ANALYZE_STEPS = [
    "Reading your plan limits…",
    "Looking at where your usage goes…",
    "Checking your effort, model, and budget settings…",
    "Asking your own Claude for ideas…",
    "Writing up plain-English suggestions…",
  ];
  let stepIdx = $state(0);
  $effect(() => {
    if (!usage.analyzing) { stepIdx = 0; analyzeStage = ""; return; }
    // Timer is the fallback floor — advance every 3.2s, clamping at the last
    // step. Real stage events (WS3) jump it forward via the effect below; the
    // timer just fills the gaps so the early "reading/looking" steps still move.
    const id = setInterval(() => {
      stepIdx = Math.min(stepIdx + 1, ANALYZE_STEPS.length - 1);
    }, 3200);
    return () => clearInterval(id);
  });
  // WS3: a real stage frame floors the visible step — never walks it backward.
  $effect(() => {
    const floor = STAGE_FLOOR[analyzeStage] ?? -1;
    if (floor > stepIdx) stepIdx = floor;
  });

  const impactRank: Record<string, number> = { high: 0, medium: 1, low: 2 };
  // Sort by impact, then dedupe by title — title is the only stable id the model
  // emits, and it keys the apply/undo bookkeeping + the keyed each. Two cards
  // sharing a title would collide; keep the first (highest-impact) of each.
  const cards = $derived.by(() => {
    if (!usage.advice) return [];
    const sorted = [...usage.advice.cards].sort((a, b) => (impactRank[a.impact] ?? 3) - (impactRank[b.impact] ?? 3));
    const seen = new Set<string>();
    return sorted.filter((c) => (seen.has(c.title) ? false : (seen.add(c.title), true)));
  });

  // ── Current harness config ── the live values an apply action would change.
  // Pretty labels so newcomers see plain words, not "xhigh"/"sonnet".
  // Display labels match the composer's unified thinking dial (Off·Low·Medium·
  // High·Max). The advisor still operates on the underlying effort tier ids
  // (none/smart/deep/ultra — the JSON-contract allow-list; legacy "quick" is
  // coerced to "smart" by normalizeApply); this is presentation only. Labels
  // track the composer's reasoning ladder (rung names = the CLI flag each tier
  // sends).
  const EFFORT_LABEL: Record<string, string> = {
    none: "Low", smart: "Medium", deep: "High", ultra: "X-High",
  };
  const MODEL_LABEL: Record<string, string> = {
    opus: "Opus", sonnet: "Sonnet", haiku: "Haiku", fable: "Fable",
    "claude-opus-4-7": "Opus", "claude-opus-4-6": "Opus", "claude-opus-4-5": "Opus",
    "claude-sonnet-4-6": "Sonnet", "claude-sonnet-4-5": "Sonnet", "claude-fable-5": "Fable",
  };
  const modelKey = (m: string) => (m === "opus" || m.startsWith("claude-opus") ? "opus" : m === "haiku" ? "haiku" : m === "claude-fable-5" ? "fable" : "sonnet");
  // Inverse of modelKey: the advisor emits short keys ("fable"), but setModel
  // wants a ModelSel — and "fable" is NOT a valid ModelSel ("claude-fable-5" is).
  const applyKeyToModel = (k: string): ModelSel => (k === "fable" ? "claude-fable-5" : (k as ModelSel));
  const budgetLabel = (n: number | null) => (n == null ? "No cap" : `$${n.toFixed(2)}/turn`);

  // Per-turn dollar budget is only a real knob in API-key mode (pay-per-token).
  // For a subscription session it's inert (usage-limit windows govern spend), so
  // it's dropped from the "knobs Rift can tune" list rather than shown as a lie.
  // Read the live value a given apply action would replace — for current→new.
  function currentValueFor(a: AdviceApply): string {
    if (a.kind === "effort") return assistant.thinkingEnabled ? (EFFORT_LABEL[assistant.thinkingEffort] ?? assistant.thinkingEffort) : "Low";
    if (a.kind === "model") return MODEL_LABEL[assistant.model] ?? assistant.model;
    return budgetLabel(assistant.maxBudgetUsd);
  }
  // The proposed value, formatted the same way as the current one.
  function newValueFor(a: AdviceApply): string {
    if (a.kind === "effort") return EFFORT_LABEL[String(a.value)] ?? String(a.value);
    if (a.kind === "model") return MODEL_LABEL[String(a.value)] ?? String(a.value);
    return `$${Number(a.value).toFixed(2)}/turn`;
  }
  // True when the action wouldn't actually change anything (model already
  // re-validated; this catches "current == proposed" the analyzer missed).
  function isNoop(a: AdviceApply): boolean {
    // Thinking disabled is never a no-op — the master switch always needs
    // flipping on, regardless of whether the stored effort tier matches.
    if (a.kind === "effort") return assistant.thinkingEnabled && assistant.thinkingEffort === a.value;
    if (a.kind === "model") return modelKey(assistant.model) === a.value;
    return assistant.maxBudgetUsd === a.value;
  }

  // Per-card apply bookkeeping, keyed by card title (stable within one advice
  // set). Holds the prior value for one-tap undo + a transient "applied" flag.
  type ApplyState = { prev: string | number | null; appliedAt: number };
  let applied = $state<Record<string, ApplyState>>({});
  // #86: `applied` deliberately SURVIVES a Re-analyze — wiping it on every new
  // advice set orphaned a live apply (the Undo button + its pre-apply snapshot
  // vanished, and isNoop could then hide the card entirely once the config
  // matched). The render gate below keeps an applied card visible via
  // `applied[c.title]` even when isNoop would hide it. Page unmount resets it.

  async function applyAction(title: string, a: AdviceApply) {
    // Snapshot the prior value so undo can restore it exactly.
    let prev: string | number | null;
    if (a.kind === "effort") {
      // setThinkingEffort alone is a silent no-op when the master switch is
      // off (the default) — go through setThinkingDial to flip it on
      // atomically, same as onboarding. Snapshot enabled+effort together so
      // undo can restore both.
      prev = `${assistant.thinkingEnabled}:${assistant.thinkingEffort}`;
      assistant.setThinkingDial(true, a.value as never);
    } else if (a.kind === "model") {
      prev = modelKey(assistant.model);
      assistant.setModel(applyKeyToModel(String(a.value)));
    } else {
      prev = assistant.maxBudgetUsd;
      await assistant.setMaxBudgetUsd(Number(a.value));
    }
    applied = { ...applied, [title]: { prev, appliedAt: Date.now() } };
  }

  async function undoAction(title: string, a: AdviceApply) {
    const st = applied[title];
    if (!st) return;
    if (a.kind === "effort") {
      const [prevEnabled, prevEffort] = String(st.prev).split(":");
      assistant.setThinkingDial(prevEnabled === "true", prevEffort as never);
    } else if (a.kind === "model") assistant.setModel(applyKeyToModel(String(st.prev ?? "sonnet")));
    else await assistant.setMaxBudgetUsd(st.prev as number | null);
    const next = { ...applied };
    delete next[title];
    applied = next;
  }
</script>

<div class="sb-main">
  <PageHero
    eyebrow="Experimental"
    title="Claude Usage & Health"
    desc="See how you're using Claude through Rift — your plan limits, where your usage goes, and one-tap advice on how to stretch your plan further."
  >
    {#snippet chip()}
      <!-- Verdict lives in the verdict strip below (label + note + value pills);
           a chip repeating the same label 150px above it was a duplicate readout.
           The chip only covers the pre-data gap. -->
      {#if !healthScore}
        <span class="ah-chip" aria-label="No health data yet — send a few Claude turns to start measuring.">
          <span class="dot"></span>Getting started
        </span>
      {/if}
    {/snippet}
  </PageHero>

  <div class="ah-scroll">
    <div class="ah-wrap">

      <!-- ── Advisor ── -->
      <div class="ah-advisor">
        <div class="ah-adv-ic"><Sparkles size={17} strokeWidth={1.75} /></div>
        <div class="ah-adv-tx">
          <div class="ah-adv-tt">Let Rift tune itself to how you work</div>
          {#if usage.analyzing}
            {#key stepIdx}
              <div class="ah-adv-step">{ANALYZE_STEPS[stepIdx]}</div>
            {/key}
            {#if stepIdx >= 3}
              <div class="ah-adv-hint">Your own Claude is reasoning over this privately — the first run can take up to a minute while it warms up. Nothing changes until you choose to.</div>
            {/if}
          {:else if !usage.advice}
            <div class="ah-adv-sub">
              Rift reads your setup and recent usage, then asks your own Claude for
              a few plain-English changes that'll make your plan go further. Nothing
              changes until you say so.
            </div>
          {/if}
        </div>
        <button class="ah-adv-btn" type="button" onclick={analyze} disabled={usage.analyzing}>
          {#if usage.analyzing}
            <Loader2 size={15} strokeWidth={2} class="ah-spin" />Analyzing…
          {:else}
            {usage.advice ? "Re-analyze" : "Analyze my usage"}<ArrowRight size={15} strokeWidth={2} />
          {/if}
        </button>
      </div>

      {#if healthScore}
        <div class="ah-verdict-strip {healthScore.tint}" role="status">
          <span class="ah-vs-dot"></span>
          <span class="ah-vs-label">{healthScore.label}</span>
          <span class="ah-vs-note">{healthScore.note}</span>
          <!-- cont.300: every measured dimension shows as a labeled value pill
               (was: anonymous dots when all-clear) — the strip reads as a real
               dashboard headline instead of a mystery traffic light. -->
          <span class="ah-vs-flags">
            {#each healthScore.dims as d (d.k)}
              <span class="ah-vs-flag {d.tint}" aria-label="{d.k}: {d.tint === 'ok' ? 'good' : d.tint === 'warn' ? 'elevated' : 'high'}, {d.v}">
                <span class="ah-vs-flag-k">{d.k}</span>
                <span class="ah-vs-flag-v">{d.v}</span>
              </span>
            {/each}
          </span>
        </div>
      {/if}

      {#if usage.adviceError}
        <div class="ah-advice-err"><AlertTriangle size={15} strokeWidth={1.9} />{usage.adviceError}</div>
      {/if}

      {#if usage.advice}
        <section class="ah-advice">
          {#if usage.advice.summary}
            <p class="ah-advice-sum">{usage.advice.summary}</p>
          {/if}
          {#if cards.length === 0}
            <p class="ah-muted">Your setup already looks well-tuned — nothing to change right now.</p>
          {:else}
            <div class="ah-cards">
              {#each cards as c (c.title)}
                <div class="ah-rec">
                  <span class="ah-rec-impact {c.impact}">{c.impact}</span>
                  <div class="ah-rec-tx">
                    <div class="ah-rec-tt">{c.title}</div>
                    <div class="ah-rec-detail">{c.detail}</div>
                    {#if c.apply && (applied[c.title] || !isNoop(c.apply))}
                      {@const a = c.apply}
                      <div class="ah-apply">
                        {#if applied[c.title]}
                          <span class="ah-applied"><Check size={13} strokeWidth={2.4} />{a.label || "Applied"}</span>
                          <button class="ah-undo" type="button" onclick={() => void undoAction(c.title, a)}>
                            <Undo2 size={12} strokeWidth={2} />Undo
                          </button>
                        {:else}
                          <span class="ah-apply-delta">
                            <span class="ah-apply-from">{currentValueFor(a)}</span>
                            <ArrowRight size={12} strokeWidth={2} />
                            <span class="ah-apply-to">{newValueFor(a)}</span>
                          </span>
                          <button class="ah-apply-btn" type="button" disabled={usage.analyzing} onclick={() => void applyAction(c.title, a)}>
                            {a.label || "Apply"}
                          </button>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── Speed & efficiency (B2: persisted turn telemetry) ── plain-English
           labels: "first reply" = the wait before text starts; "typical" = the
           middle of your replies (median), "slower" = your slow tail (≈1 in 10,
           p90). Tooltips are banned app-wide (2026-06-15), so the meaning lives
           in the labels + the footnote, not on hover. Sits high (right under the
           advisor) because the verdict strip's Latency/Cache reads come straight
           from here. -->
      {#if perfStats}
        <section class="ah-card">
          <div class="ah-card-h"><Gauge size={15} strokeWidth={1.9} />Speed &amp; efficiency
            <span class="ah-range" role="group" aria-label="Time range">
              {#each ["24h", "7d", "30d", "all"] as const as w (w)}
                <button type="button" class="ah-range-b" class:active={perfWindow === w} aria-pressed={perfWindow === w} onclick={() => setPerfWindow(w)}>
                  {w === "all" ? "All" : w}
                </button>
              {/each}
            </span>
          </div>
          {#if !hasPerf}
          <p class="ah-muted">{perfWindow === "all"
            ? "Not enough replies yet to measure — send a few Claude turns and check back."
            : `Not enough replies in ${PERF_WINDOW_LABEL[perfWindow]} to measure — try a wider range.`}</p>
          {:else}
          <p class="ah-card-sub">How fast Claude responds and how efficiently it reuses your conversation — {fmtNum(perfStats.total_turns)} replies over {PERF_WINDOW_LABEL[perfWindow]}{latencyP90Source.recent ? " · live verdict from the last 24 hours" : ""}.</p>
          <div class="ah-tiles">
            <div class="ah-tile">
              <div class="ah-tile-v">{fmtMs(perfStats.p50_ttft_text_ms)}</div>
              <div class="ah-tile-k">typical wait to first reply</div>
            </div>
            {#if latencyP90Source.ms != null}
              <!-- Warm-only slow-reply (cold-start excluded) so the tinted verdict
                   reflects steady-state speed, not a one-time spawn cost. -->
              <div class="ah-tile {latencyTint}">
                <div class="ah-tile-v">{fmtMs(latencyP90Source.ms)}</div>
                <div class="ah-tile-k">on a slow reply{#if latencyVerdict}<span class="ah-verdict {latencyTint}">{latencyVerdict}</span>{:else if !latencyP90Source.recent}<span class="ah-verdict">lifetime</span>{/if}</div>
              </div>
            {:else if latencyLearning}
              <!-- G3: perf data exists but warm sample is below the floor. -->
              <div class="ah-tile">
                <div class="ah-tile-v">—</div>
                <div class="ah-tile-k">slow-reply speed<span class="ah-verdict">learning</span></div>
              </div>
            {/if}
            <div class="ah-tile">
              <div class="ah-tile-v">{fmtMs(perfStats.p50_duration_ms)}</div>
              <div class="ah-tile-k">typical full reply</div>
            </div>
            {#if perfStats.cache_hit_rate != null}
              <div class="ah-tile {cacheTint}">
                <div class="ah-tile-v">{Math.round(perfStats.cache_hit_rate * 100)}%</div>
                <div class="ah-tile-k">conversation reused{#if cacheVerdict}<span class="ah-verdict {cacheTint}">{cacheVerdict}</span>{/if}</div>
              </div>
            {/if}
            <div class="ah-tile">
              <div class="ah-tile-v">{fmtNum(perfStats.total_output_tokens)}</div>
              <div class="ah-tile-k">words written (tokens)</div>
            </div>
          </div>

          <!-- G5: tell the user when slowness is upstream (the API), not their
               setup — the single most reassuring thing to surface. -->
          {#if latencyAttribution}
            <p class="ah-attrib"><Wifi size={13} strokeWidth={1.9} />{latencyAttribution}</p>
          {/if}
          <!-- cont.219: model-vs-Rift split — proves the wait is the model, not
               Rift's plumbing, on the user's own turns. -->
          {#if splitAttribution}
            <p class="ah-attrib subtle"><Gauge size={13} strokeWidth={1.9} />About {splitAttribution.pct}% of any wait is Claude itself working; Rift's own overhead adds just {splitAttribution.rift} per reply. The wait is the model, not the app.</p>
          {/if}
          <!-- G1: cold-start shown as the one-time warm-up it is, never as a
               problem with the user's setup. Only when warm is meaningfully
               faster than cold (≥2s gap) — else "keeping a chat going stays fast"
               would be a hollow claim (warm is slow too → that's a real signal,
               not a warm-up artifact, and the verdict/advisor already own it). -->
          {#if showColdNote}
            <p class="ah-attrib subtle"><Snowflake size={13} strokeWidth={1.9} />The first reply of a session takes ~{fmtMs(perfStats.p90_ttft_text_cold_ms)} while Claude warms up — a one-time cost, not counted against your speed above. Keeping a chat going stays fast.</p>
          {/if}

          {#if latencySpark}
            <div class="ah-spark-row">
              <span class="ah-spark-k">slow-reply wait · {PERF_WINDOW_LABEL[perfWindow]}</span>
              <svg class="ah-spark" viewBox="0 0 {SPARK_W} {SPARK_H}" preserveAspectRatio="none" aria-hidden="true">
                <polyline class="ah-spark-line {latencySpark.tint}" points={latencySpark.line} fill="none" />
              </svg>
              <span class="ah-spark-v">{fmtMs(latencySpark.first)} → {fmtMs(latencySpark.last)}</span>
            </div>
          {/if}

          {#if modelLat.length > 0}
            <!-- cont.300: by_model always existed in the aggregate (the advisor
                 reads it) — now the user sees it too. -->
            <div class="ah-mlat">
              <span class="ah-mlat-h">By model</span>
              {#each modelLat as g (`${g.model}:${g.effort ?? ""}`)}
                <div class="ah-mlat-row">
                  <span class="ah-mlat-k">{modelLabel(g.model)}{g.effort ? ` · ${g.effort}` : ""}</span>
                  <span class="ah-mlat-v">typical {fmtMs(g.p50_ttft_text_ms)} · slow {fmtMs(g.p90_ttft_text_ms)} · {fmtNum(g.turn_count)} replies{g.dominant_cause ? ` · mostly ${g.dominant_cause}` : ""}</span>
                </div>
              {/each}
            </div>
          {/if}

          <p class="ah-glossary">
            <strong>Typical</strong> is the middle of your replies — half are faster, half slower.
            <strong>Slow reply</strong> is one of your slower ones (about 1 in 10).
            <strong>Conversation reused</strong> is how much of the chat Claude remembers without re-reading it — higher means faster, cheaper replies.
          </p>
          {/if}
        </section>
      {/if}

      <!-- ── MCP servers (#93-4) ── per-session tool-server health from the
           latest init frame — the same data /mcp prints, as a persistent
           surface instead of a one-shot notice. -->
      <section class="ah-card half">
        <div class="ah-card-h"><Plug size={15} strokeWidth={1.9} />MCP servers</div>
        {#if !mcpRows}
          <p class="ah-muted">No status yet this session — the CLI reports server health at the start of each turn. Send a message and check back, or run <code>/mcp</code> in the chat.</p>
        {:else if mcpRows.length === 0}
          <p class="ah-muted">No MCP servers configured for this session.</p>
        {:else}
          <p class="ah-card-sub">Tool servers your Claude CLI reported at the start of the latest turn.</p>
          <div class="ah-cfg">
            {#each mcpRows as s (s.name)}
              <div class="ah-cfg-row">
                <span class="ah-cfg-k"><span class="ah-mcp-dot {mcpTint(s.status)}" aria-hidden="true"></span>{s.name}</span>
                <span class="ah-cfg-v">{s.status}</span>
              </div>
            {/each}
          </div>
          {#if mcpRows.some((s) => s.status === "needs-auth")}
            <p class="ah-cfg-note">claude.ai connectors can't complete sign-in from inside Rift, so needs-auth is their normal state here — those tools simply stay off this session.</p>
          {/if}
        {/if}
      </section>

      <!-- ── Plan limits ── -->
      <section class="ah-card half">
        <div class="ah-card-h"><Gauge size={15} strokeWidth={1.9} />Plan limits{#if fetchedAgo && limitRows.length > 0}<span class="ah-asof">{fetchedAgo}</span>{/if}</div>
        {#if assistant.hasApiKey}
          <p class="ah-muted">Plan limits apply to Claude subscription accounts. You're on an API key — billed per token — so there are no usage windows to track here. Your speed &amp; efficiency below still apply.</p>
        {:else if usage.rateLimitsError}
          <p class="ah-muted">Couldn't load your plan limits right now. {usage.rateLimitsError} They'll appear once you've run a Claude turn to refresh your login.</p>
        {:else if usage.rateLimits === null}
          <p class="ah-muted">Loading your plan limits…</p>
        {:else if limitRows.length === 0}
          <p class="ah-muted">No subscription limits to show yet — sign in with a Claude plan, or run a turn to refresh.</p>
        {:else}
          <p class="ah-card-sub">How much of each usage window you've used. When a bar fills up, Claude pauses until it resets at the time shown.</p>
          <div class="ah-bars">
            {#each limitRows as row (row.k)}
              {@const u = Math.round(row.w.utilization)}
              {@const pace = paceFor(row.k, row.w)}
              <div class="ah-bar-row">
                <div class="ah-bar-top">
                  <span class="ah-bar-k">{row.k}</span>
                  <span class="ah-bar-v">{u}%<span class="ah-bar-reset">{fmtReset(row.w.resetsAt)}</span></span>
                </div>
                <div class="ah-track"><div class="ah-fill {zone(u)}" style:width="{Math.min(100, u)}%"></div></div>
                {#if pace}
                  <div class="ah-pace" class:hot={pace.hot}>
                    {pace.hot ? "at this pace you'll hit the cap before it resets" : `on pace for ~${pace.pct}% by reset`}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
        {#if usage.rateLimits?.extraUsage?.isEnabled}
          {@const x = usage.rateLimits.extraUsage}
          {@const xu = x.utilization != null ? Math.round(x.utilization) : null}
          {@const scale = 10 ** (x.decimalPlaces ?? 2)}
          <div class="ah-bars ah-extra">
            <div class="ah-bar-row">
              <div class="ah-bar-top">
                <span class="ah-bar-k">Usage credits</span>
                <span class="ah-bar-v">
                  {#if x.usedCredits != null && x.monthlyLimit != null}
                    {fmtUsd(x.usedCredits / scale)} / {fmtUsd(x.monthlyLimit / scale)}
                  {:else if xu != null}{xu}%{/if}
                </span>
              </div>
              {#if xu != null}
                <div class="ah-track"><div class="ah-fill {zone(xu)}" style:width="{Math.min(100, xu)}%"></div></div>
              {/if}
            </div>
          </div>
        {/if}
      </section>

      <!-- ── All-time usage ── -->
      <section class="ah-card half">
        <div class="ah-card-h"><Wrench size={15} strokeWidth={1.9} />Where your usage goes</div>
        {#if statsError}
          <p class="ah-muted">Couldn't load your history: {statsError}</p>
        {:else if stats === null}
          <!-- Still fetching — don't flash the "no conversations yet" copy at a
               user with hundreds of sessions while the async load races in.
               Skeleton tiles keep the grid cell from reading as a dead zone. -->
          <p class="ah-muted">Reading your history…</p>
          <div class="ah-tiles" aria-hidden="true">
            {#each { length: 4 } as _, i (i)}
              <div class="ah-tile ah-tile-skel"><div class="ah-skel ah-skel-v"></div><div class="ah-skel ah-skel-k"></div></div>
            {/each}
          </div>
        {:else if !hasHistory}
          <p class="ah-muted">Once you've had a few conversations, your usage breakdown shows up here.</p>
        {:else if totals}
          <div class="ah-tiles">
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(totals.sessions)}</div><div class="ah-tile-k">conversations</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(totals.messages)}</div><div class="ah-tile-k">messages</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(totals.toolCalls)}</div><div class="ah-tile-k">tool calls</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtUsd(totals.cost)}</div><div class="ah-tile-k">est. spend</div></div>
            {#if top}<div class="ah-tile"><div class="ah-tile-v ah-tile-sm">{top}</div><div class="ah-tile-k">most used</div></div>{/if}
          </div>

          {#if modelsShown.length > 0}
            <div class="ah-models">
              <span class="ah-models-h">Messages by model</span>
              {#each modelsShown as m (m.model)}
                <div class="ah-model-row">
                  <span class="ah-model-k">{m.label}</span>
                  <div class="ah-track sm"><div class="ah-fill ok" style:width="{Math.round(m.share * 100)}%"></div></div>
                  <span class="ah-model-v">{Math.round(m.share * 100)}%</span>
                </div>
              {/each}
            </div>
          {/if}
          {#if modelSpend.length > 0}
            <!-- cont.300: dollar share ≠ message share — Opus can be 30% of
                 messages and 90% of spend. The actionable split. -->
            <div class="ah-models">
              <span class="ah-models-h">Spend by model · {PERF_WINDOW_LABEL[perfWindow]}</span>
              {#each modelSpend as m (m.label)}
                <div class="ah-model-row">
                  <span class="ah-model-k">{m.label}</span>
                  <div class="ah-track sm"><div class="ah-fill spend" style:width="{Math.round(m.share * 100)}%"></div></div>
                  <span class="ah-model-v">{fmtUsd(m.usd)}</span>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </section>

      <!-- ── Spend per day (cont.300) ── extracted from the Speed card into a
           real bar chart: per-day columns, peak + newest labeled, range-driven. -->
      {#if spendBars}
        <section class="ah-card half">
          <div class="ah-card-h"><Wrench size={15} strokeWidth={1.9} />Spend per day</div>
          <p class="ah-card-sub">{fmtUsd(spendBars.total)} over {PERF_WINDOW_LABEL[perfWindow]}.</p>
          <div class="ah-spendchart" role="img" aria-label="Daily spend, {spendBars.bars.length} days, total {fmtUsd(spendBars.total)}">
            {#each spendBars.bars as b (b.day)}
              <div class="ah-sc-col">
                <!-- Label rides ABOVE the bar absolutely — in-flow it would be
                     part of the column's flex math and shrink the tallest bar
                     below its true height (the exact bug it once caused). -->
                <div class="ah-sc-barwrap" style:height="{b.h}%">
                  {#if b.labeled}<span class="ah-sc-v">{fmtUsd(b.cost)}</span>{/if}
                  <div class="ah-sc-bar"></div>
                </div>
                <span class="ah-sc-d">{b.dow}</span>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- ── This session ── only shown once this session has recorded turns -->
      {#if session.totalTurns > 0}
      <section class="ah-card half">
        <div class="ah-card-h"><HeartPulse size={15} strokeWidth={1.9} />This session</div>
          <div class="ah-tiles sm">
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(session.totalTurns)}</div><div class="ah-tile-k">replies</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtUsd(session.totalCostUsd)}</div><div class="ah-tile-k">spend</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(session.toolCallTotal)}</div><div class="ah-tile-k">tool calls</div></div>
            {#if session.zeroToolTurns > 0}
              <div class="ah-tile"><div class="ah-tile-v">{fmtNum(session.zeroToolTurns)}</div><div class="ah-tile-k">chat-only replies</div></div>
            {/if}
            {#if session.avgTtfpMs != null}
              <div class="ah-tile"><div class="ah-tile-v">{(session.avgTtfpMs / 1000).toFixed(1)}s</div><div class="ah-tile-k">avg wait to first reply</div></div>
            {/if}
          </div>
      </section>
      {/if}

    </div>
  </div>
</div>

<style>
  .sb-main { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .ah-scroll { flex: 1; min-height: 0; overflow-y: auto; }
  /* cont.300: dashboard grid. Narrow = the classic single column; ≥1160px
     viewport = two-column card grid (`.half` cards pair up, everything else
     spans). Desktop app — use the desktop's width. */
  .ah-wrap {
    max-width: 820px; margin: 0 auto; padding: 18px 40px 28px;
    display: grid; grid-template-columns: minmax(0, 1fr); gap: 14px;
    /* start (not stretch): paired .half cards size to their own content instead
       of stretching to the taller sibling — kills the dead space under a short
       card (Plan limits next to the much taller usage-history card). */
    align-items: start;
  }
  .ah-wrap > :global(*) { grid-column: 1 / -1; min-width: 0; }
  @media (min-width: 1160px) {
    .ah-wrap { max-width: 1180px; grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .ah-wrap > :global(.half) { grid-column: span 1; }
  }

  /* Staggered rise-in for the dashboard cards — reuses the shared `enter` motion
     (app.css) so the page assembles top-down instead of flashing in all at once.
     nth-child cascade avoids threading `--idx` through every section's markup. */
  @media (prefers-reduced-motion: no-preference) {
    .ah-wrap > :global(*) { animation: enter 360ms var(--ease-page) both; }
    .ah-wrap > :global(*:nth-child(1)) { animation-delay: 20ms; }
    .ah-wrap > :global(*:nth-child(2)) { animation-delay: 70ms; }
    .ah-wrap > :global(*:nth-child(3)) { animation-delay: 120ms; }
    .ah-wrap > :global(*:nth-child(4)) { animation-delay: 170ms; }
    .ah-wrap > :global(*:nth-child(5)) { animation-delay: 220ms; }
    .ah-wrap > :global(*:nth-child(6)) { animation-delay: 270ms; }
    .ah-wrap > :global(*:nth-child(n+7)) { animation-delay: 320ms; }
  }

  .ah-chip { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; padding: 4px 10px; border-radius: 999px; background: var(--accent-soft); color: var(--fg-muted); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-chip .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--fg-subtle); }
  .ah-chip.ok .dot { background: var(--accent); }
  .ah-chip.warn .dot { background: var(--warn); }
  .ah-chip.hot .dot { background: var(--danger); }

  .ah-advisor { display: flex; align-items: center; gap: 16px; padding: 20px; border-radius: var(--radius-xl); background: linear-gradient(135deg, color-mix(in oklab, var(--accent) 13%, transparent), color-mix(in oklab, var(--accent) 4%, transparent)); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 22%, var(--ghost-border)); }
  .ah-adv-ic { width: 36px; height: 36px; flex: none; border-radius: 10px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .ah-adv-tx { flex: 1; min-width: 0; }
  .ah-adv-tt { font-size: 16px; font-weight: 700; letter-spacing: -0.015em; }

  /* Overall health verdict strip — the at-a-glance "am I OK?" line and the ONE
     place a flagged metric is stated (folds in the old separate signals row).
     Neutral card surface with a colored left-rule + dot per the stream design
     language (accent = meaningful signal only). Right side: labeled value pills
     for any flagged dimension, or three all-clear dots when nothing's wrong. */
  .ah-verdict-strip { display: flex; align-items: center; gap: 11px; padding: 13px 20px; border-radius: var(--radius-xl); background: var(--surface); box-shadow: inset 0 0 0 1px var(--border); border-left: 3px solid var(--fg-subtle); animation: ah-vs-in 0.42s var(--ease-page) both; }
  .ah-verdict-strip.ok { border-left-color: var(--accent); }
  .ah-verdict-strip.warn { border-left-color: var(--warn); }
  .ah-verdict-strip.hot { border-left-color: var(--danger); }
  @keyframes ah-vs-in { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: translateY(0); } }
  @media (prefers-reduced-motion: reduce) { .ah-verdict-strip { animation: none; } }
  .ah-vs-dot { width: 8px; height: 8px; border-radius: 999px; flex: none; background: var(--fg-subtle); }
  .ah-verdict-strip.ok .ah-vs-dot { background: var(--accent); }
  .ah-verdict-strip.warn .ah-vs-dot { background: var(--warn); }
  /* hot = the one state worth a heartbeat — a slow, calm pulse draws the eye to
     "Action needed" without alarm. Meaningful-signal motion only (never on ok). */
  .ah-verdict-strip.hot .ah-vs-dot { background: var(--danger); animation: ah-vs-pulse 2.4s var(--ease-page) infinite; }
  @keyframes ah-vs-pulse { 0%, 100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--danger) 45%, transparent); } 50% { box-shadow: 0 0 0 4px color-mix(in srgb, var(--danger) 0%, transparent); } }
  @media (prefers-reduced-motion: reduce) { .ah-verdict-strip.hot .ah-vs-dot { animation: none; } }
  .ah-vs-label { font-size: 13px; font-weight: 680; letter-spacing: -0.01em; flex: none; }
  .ah-vs-note { font-size: var(--fs-sm); color: var(--fg-muted); min-width: 0; overflow: hidden; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; line-height: 1.35; }
  /* All-clear: three dimension dots stand in for "nothing to flag". */
  /* Every measured dimension as a labeled value pill (cont.300 — the old
     anonymous all-clear dots told the user nothing). ok pills stay quiet;
     warn/hot pills carry their tint. */
  .ah-vs-flags { display: inline-flex; gap: 7px; margin-left: auto; flex: none; flex-wrap: wrap; justify-content: flex-end; }
  .ah-vs-flag { display: inline-flex; align-items: baseline; gap: 6px; padding: 3px 10px; border-radius: 999px; font-size: var(--fs-sm); background: var(--bg-inset); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-vs-flag.ok { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 26%, transparent); }
  .ah-vs-flag.warn { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--warn) 36%, transparent); }
  .ah-vs-flag.hot { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--danger) 38%, transparent); }
  .ah-vs-flag-k { color: var(--fg-muted); }
  .ah-vs-flag.ok .ah-vs-flag-k { color: var(--accent); }
  .ah-vs-flag.warn .ah-vs-flag-k { color: var(--warn); }
  .ah-vs-flag.hot .ah-vs-flag-k { color: var(--danger); }
  .ah-vs-flag-v { font-weight: 680; font-variant-numeric: tabular-nums; }

  .ah-adv-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.45; }
  .ah-adv-btn { flex: none; display: inline-flex; align-items: center; gap: 6px; padding: 9px 16px; border-radius: 10px; border: none; font-size: 13px; font-weight: 640; cursor: pointer; background: var(--accent); color: var(--accent-fg); }
  .ah-adv-btn:disabled { opacity: 0.6; cursor: default; }
  .ah-adv-btn :global(.ah-spin) { animation: ah-spin 0.9s linear infinite; }
  @keyframes ah-spin { to { transform: rotate(360deg); } }

  /* ── inline analyzing progress (folded into the advisor card) ── */
  .ah-adv-step { font-size: var(--fs-sm); color: var(--accent); font-weight: 560; margin-top: 4px; animation: ah-step-in 0.4s var(--ease-page) both; }
  @keyframes ah-step-in { from { opacity: 0; transform: translateX(-5px); } to { opacity: 1; transform: translateX(0); } }
  .ah-adv-hint { font-size: 11.5px; color: var(--fg-subtle); margin-top: 6px; line-height: 1.45; }
  @media (prefers-reduced-motion: reduce) { .ah-adv-step { animation: none; } }
  .ah-advice-err { display: flex; align-items: center; gap: 8px; font-size: var(--fs-sm); color: var(--danger); padding: 10px 14px; border-radius: 10px; background: color-mix(in oklab, var(--danger) 8%, transparent); }
  .ah-advice { display: flex; flex-direction: column; gap: 12px; }
  .ah-advice-sum { font-size: var(--fs-sm); color: var(--fg-muted); margin: 0 2px; line-height: 1.5; }
  .ah-cards { display: flex; flex-direction: column; gap: 10px; }
  .ah-rec { display: flex; gap: 13px; padding: 15px 20px; border-radius: var(--radius-xl); background: var(--surface); box-shadow: inset 0 0 0 1px var(--border); }
  .ah-rec-impact { flex: none; align-self: flex-start; font-size: 10px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; padding: 3px 8px; border-radius: 6px; }
  .ah-rec-impact.high { background: color-mix(in oklab, var(--accent) 18%, transparent); color: var(--accent); }
  .ah-rec-impact.medium { background: color-mix(in oklab, var(--warn) 18%, transparent); color: var(--warn); }
  .ah-rec-impact.low { background: var(--ghost-border); color: var(--fg-subtle); }
  .ah-rec-tx { min-width: 0; }
  .ah-rec-tt { font-size: 14px; font-weight: 660; letter-spacing: -0.01em; }
  .ah-rec-detail { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; }

  .ah-apply { display: flex; align-items: center; gap: 12px; margin-top: 11px; flex-wrap: wrap; }
  .ah-apply-delta { display: inline-flex; align-items: center; gap: 7px; font-size: 12px; font-variant-numeric: tabular-nums; }
  .ah-apply-delta :global(svg) { color: var(--fg-faint); }
  .ah-apply-from { color: var(--fg-subtle); text-decoration: line-through; }
  .ah-apply-to { color: var(--fg); font-weight: 640; }
  .ah-apply-btn { flex: none; display: inline-flex; align-items: center; padding: 6px 13px; border-radius: 8px; border: none; cursor: pointer;
    font-size: 12px; font-weight: 620; background: var(--accent-soft); color: var(--accent); box-shadow: inset 0 0 0 1px var(--ghost-border);
    transition: background var(--dur-fast); }
  .ah-apply-btn:hover { background: color-mix(in oklab, var(--accent) 18%, transparent); }
  .ah-applied { display: inline-flex; align-items: center; gap: 5px; font-size: 12px; font-weight: 620; color: var(--accent); }
  .ah-undo { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; font-weight: 540; color: var(--fg-subtle); cursor: pointer;
    background: none; border: none; padding: 2px 4px; transition: color var(--dur-fast); }
  .ah-undo:hover { color: var(--fg-2); }

  .ah-cfg { display: flex; flex-direction: column; gap: 1px; }
  .ah-cfg-row { display: flex; justify-content: space-between; align-items: baseline; padding: 7px 0; border-bottom: 1px solid var(--ghost-border); }
  .ah-cfg-row:last-child { border-bottom: none; }
  .ah-cfg-k { font-size: var(--fs-sm); color: var(--fg-muted); }
  .ah-cfg-v { font-size: var(--fs-sm); font-weight: 640; color: var(--fg); font-variant-numeric: tabular-nums; }
  .ah-cfg-note { font-size: 11.5px; color: var(--fg-subtle); margin: 11px 0 0; line-height: 1.45; }

  /* ── cont.300 additions ─────────────────────────────────────────────── */
  /* Range picker — compact segmented control in the card header. */
  .ah-range {
    margin-left: auto; display: inline-flex; gap: 2px; padding: 2px;
    border-radius: 8px; background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--ghost-border);
  }
  .ah-range-b {
    border: 0; background: transparent; color: var(--fg-subtle);
    font: inherit; font-size: 11px; font-weight: 620; letter-spacing: 0.01em;
    padding: 3px 9px; border-radius: 6px; cursor: pointer;
    transition: background var(--dur-fast) ease-out, color var(--dur-fast) ease-out;
  }
  .ah-range-b:hover { color: var(--fg); }
  .ah-range-b.active { background: var(--surface); color: var(--fg); box-shadow: inset 0 0 0 1px var(--border); }

  /* Plan pace projection — quiet forecast line under a limit bar. */
  .ah-pace { font-size: 11px; color: var(--fg-subtle); margin-top: 4px; }
  .ah-pace.hot { color: var(--danger); font-weight: 600; }

  /* Spend bar chart — per-day columns, baseline-aligned. */
  .ah-spendchart {
    display: flex; align-items: flex-end; gap: 5px;
    height: 96px; margin-top: 12px; padding-top: 14px;
  }
  .ah-sc-col {
    flex: 1; min-width: 0; height: 100%;
    display: flex; flex-direction: column; align-items: center; justify-content: flex-end;
    position: relative;
    /* Reserve the day-letter strip OUT of the bar's %-height math. */
    padding-bottom: 14px;
  }
  .ah-sc-barwrap {
    position: relative;
    width: 100%; max-width: 26px;
    display: flex; align-items: stretch;
    transition: height var(--dur-base) var(--ease-page);
  }
  .ah-sc-bar {
    width: 100%; border-radius: 4px 4px 2px 2px;
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 78%, transparent), color-mix(in oklab, var(--accent) 34%, transparent));
    min-height: 3px;
  }
  .ah-sc-v {
    position: absolute; bottom: 100%; left: 50%; transform: translateX(-50%);
    margin-bottom: 3px;
    font-size: 10px; font-weight: 640; color: var(--fg-muted);
    font-variant-numeric: tabular-nums; white-space: nowrap;
  }
  .ah-sc-d {
    position: absolute; bottom: 0; left: 50%; transform: translateX(-50%);
    font-size: 9.5px; color: var(--fg-faint); line-height: 1;
  }

  /* By-model latency rows (Speed card). */
  .ah-mlat { margin-top: 13px; display: flex; flex-direction: column; gap: 1px; }
  .ah-mlat-h, .ah-models-h { font-size: 10.5px; font-weight: 680; letter-spacing: 0.05em; text-transform: uppercase; color: var(--fg-subtle); margin-bottom: 4px; display: block; }
  .ah-mlat-row { display: flex; justify-content: space-between; gap: 12px; padding: 5px 0; border-bottom: 1px solid var(--ghost-border); }
  .ah-mlat-row:last-child { border-bottom: none; }
  .ah-mlat-k { font-size: var(--fs-sm); color: var(--fg-muted); flex: none; }
  .ah-mlat-v { font-size: var(--fs-sm); color: var(--fg); font-variant-numeric: tabular-nums; text-align: right; min-width: 0; }
  .ah-models + .ah-models { margin-top: 12px; }

  /* MCP server status dot — same ok/warn/hot vocabulary as the verdict strip. */
  .ah-mcp-dot {
    display: inline-block; width: 7px; height: 7px; border-radius: 50%;
    margin-right: 7px; vertical-align: middle;
    background: var(--fg-faint);
  }
  .ah-mcp-dot.ok { background: var(--accent); }
  .ah-mcp-dot.warn { background: var(--warn); }
  .ah-mcp-dot.hot { background: var(--danger); }

  .ah-card { border-radius: var(--radius-xl); padding: 15px 20px; background: var(--surface); box-shadow: inset 0 0 0 1px var(--border); }
  .ah-card-h { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 660; letter-spacing: -0.01em; margin-bottom: 12px; color: var(--fg); }
  .ah-card-h :global(svg) { color: var(--accent); }
  /* intro line under a card header — plain-English context before the numbers */
  .ah-card-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin: -6px 0 13px; line-height: 1.5; }
  .ah-asof { margin-left: auto; font-size: 11.5px; font-weight: 500; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .ah-muted { font-size: var(--fs-sm); color: var(--fg-muted); margin: 0; line-height: 1.5; }
  /* glossary footnote — defines the plain-English terms inline (tooltips are
     banned app-wide), de-emphasized so it never competes with the numbers */
  .ah-glossary { font-size: 11.5px; color: var(--fg-subtle); margin: 13px 0 0; padding-top: 12px; border-top: 1px solid var(--ghost-border); line-height: 1.6; }
  .ah-glossary strong { color: var(--fg-muted); font-weight: 620; }

  /* G5/G1 — inline attribution notes (API-not-Rift, cold-start warm-up). Calm,
     boxless, left-iconed; subtle variant for the one-time cold-start aside. */
  .ah-attrib { display: flex; align-items: flex-start; gap: 8px; font-size: 12px; color: var(--fg-muted); line-height: 1.55; margin: 12px 0 0; }
  .ah-attrib :global(svg) { flex: 0 0 auto; margin-top: 1px; color: color-mix(in oklab, var(--accent) 70%, var(--fg-subtle)); }
  .ah-attrib.subtle { color: var(--fg-subtle); }
  .ah-attrib.subtle :global(svg) { color: var(--fg-subtle); }

  .ah-bars { display: flex; flex-direction: column; gap: 14px; }
  .ah-extra { margin-top: 14px; padding-top: 12px; border-top: 1px solid var(--ghost-border); }
  .ah-bar-top { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 6px; }
  .ah-bar-k { font-size: var(--fs-sm); color: var(--fg-muted); }
  .ah-bar-v { font-size: var(--fs-sm); font-weight: 640; font-variant-numeric: tabular-nums; }
  .ah-bar-reset { font-size: 11.5px; font-weight: 400; color: var(--fg-subtle); margin-left: 8px; }
  .ah-track { height: 7px; border-radius: 999px; background: var(--ghost-border); overflow: hidden; }
  .ah-track.sm { height: 5px; flex: 1; }
  .ah-fill { height: 100%; border-radius: 999px; transition: width 0.3s ease; background: var(--accent); }
  .ah-fill.warn { background: var(--warn); }
  .ah-fill.hot { background: var(--danger); }
  /* spend bars: neutral grey — a spend breakdown is data, not a warning. Distinct
     from the emerald message-share bars so the two lists don't read as the same. */
  .ah-fill.spend { background: color-mix(in oklab, var(--fg) 40%, transparent); }

  .ah-tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 12px; }
  .ah-tile { padding: 12px 14px; border-radius: 10px; background: var(--bg-inset); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-tiles.sm .ah-tile { padding: 10px 12px; }
  .ah-tiles.sm .ah-tile-v { font-size: 18px; }
  .ah-tile-v { font-size: 20px; font-weight: 720; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .ah-tile-v.ah-tile-sm { font-size: 14px; font-weight: 640; }
  /* Label wraps as normal text; the verdict badge flows inline after the last
     word (was display:flex + align-items:center — which floated the badge
     vertically against multi-line labels and collided with the wrapped text). */
  .ah-tile-k { font-size: 11.5px; color: var(--fg-subtle); margin-top: 2px; line-height: 1.45; }
  /* Loading skeleton — quiet pulsing blocks in the tile grid so the card holds
     its shape while history stats load, instead of a near-empty cell. */
  .ah-tile-skel { pointer-events: none; }
  .ah-skel { border-radius: 5px; background: color-mix(in oklab, var(--fg) 8%, transparent); animation: ah-skel-pulse 1.4s var(--ease-soft) infinite; }
  .ah-skel-v { width: 58px; height: 20px; }
  .ah-skel-k { width: 86px; height: 10px; margin-top: 8px; }
  @keyframes ah-skel-pulse { 0%, 100% { opacity: 0.5; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .ah-skel { animation: none; } }
  /* WS1: signal tints — a faint wash + colored inset ring on the tile whose
     verdict matters, reusing the shared accent/warn/danger tokens. */
  .ah-tile.ok { background: color-mix(in srgb, var(--accent) 8%, var(--bg-inset)); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 38%, transparent); }
  .ah-tile.warn { background: color-mix(in srgb, var(--warn) 10%, var(--bg-inset)); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--warn) 42%, transparent); }
  .ah-tile.hot { background: color-mix(in srgb, var(--danger) 10%, var(--bg-inset)); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--danger) 45%, transparent); }
  .ah-verdict { display: inline-block; margin-left: 5px; font-size: 9.5px; font-weight: 700; letter-spacing: 0.03em; text-transform: uppercase; padding: 1px 5px; border-radius: 999px; line-height: 1.5; white-space: nowrap; vertical-align: baseline; color: var(--fg-subtle); background: color-mix(in srgb, var(--fg-subtle) 13%, transparent); }
  .ah-verdict.ok { color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, transparent); }
  .ah-verdict.warn { color: var(--warn); background: color-mix(in srgb, var(--warn) 18%, transparent); }
  .ah-verdict.hot { color: var(--danger); background: color-mix(in srgb, var(--danger) 18%, transparent); }

  .ah-models { display: flex; flex-direction: column; gap: 9px; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border); }
  .ah-model-row { display: flex; align-items: center; gap: 12px; }
  .ah-model-k { font-size: var(--fs-sm); color: var(--fg-muted); width: 130px; flex: none; }
  .ah-model-v { font-size: var(--fs-sm); font-weight: 640; font-variant-numeric: tabular-nums; width: 38px; text-align: right; flex: none; }
  .ah-trend { display: flex; flex-direction: column; gap: 9px; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border); }
  .ah-trend-h { font-size: 11.5px; font-weight: 620; color: var(--fg-muted); letter-spacing: -0.005em; margin-bottom: 1px; }
  .ah-trend-row { display: flex; align-items: center; gap: 12px; }
  .ah-trend-k { font-size: var(--fs-sm); color: var(--fg-muted); width: 92px; flex: none; font-variant-numeric: tabular-nums; }
  .ah-trend-v { font-size: var(--fs-sm); font-weight: 640; font-variant-numeric: tabular-nums; width: 56px; text-align: right; flex: none; }

  /* WS4: latency sparkline — tiny inline polyline, no axes/labels. */
  .ah-spark-row { display: flex; align-items: center; gap: 12px; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border); }
  .ah-spark-k { font-size: var(--fs-sm); color: var(--fg-muted); flex: none; }
  .ah-spark { width: 100px; height: 28px; flex: none; overflow: visible; }
  .ah-spark-line { stroke: var(--accent); stroke-width: 1.6; vector-effect: non-scaling-stroke; stroke-linecap: round; stroke-linejoin: round; }
  .ah-spark-line.warn { stroke: var(--warn); }
  .ah-spark-line.hot { stroke: var(--danger); }
  .ah-spark-v { font-size: var(--fs-sm); font-weight: 640; font-variant-numeric: tabular-nums; margin-left: auto; color: var(--fg-muted); }
</style>
