<script lang="ts">
  // AI Health — the observe layer (Stage 1). Surfaces what Rift already knows
  // about a user's plan + usage, and seats the "Analyze my usage" advisor
  // (Stage 2) that reasons over it via the user's own Claude. Coaches newcomers
  // in plain English; charts are supporting cast.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { HeartPulse, Gauge, Sparkles, ArrowRight, Wrench, Loader2, AlertTriangle, Check, Undo2, SlidersHorizontal } from "lucide-svelte";
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
  };
  type TurnPerfStats = {
    p50_ttft_text_ms: number | null;
    p90_ttft_text_ms: number | null;
    p50_duration_ms: number | null;
    p90_duration_ms: number | null;
    cache_hit_rate: number | null;
    total_output_tokens: number;
    cost_by_day: [string, number][];
    latency_p90_by_day: [string, number | null][];
    by_model: ModelPerfStats[];
    total_turns: number;
  };
  let perfStats = $state<TurnPerfStats | null>(null);

  // Ticks every 30s so the "fetched Xs ago" label stays live without a render
  // on every frame. Set once on mount, cleared on destroy.
  let nowTick = $state(Date.now());

  // WS3: real, frame-driven analyze stage from the backend's progress events
  // ("spawned" | "thinking" | "writing"). Floors the cosmetic step ticker so the
  // visible step jumps forward the moment the CLI actually reaches that phase,
  // instead of guessing on a timer. "" between runs.
  let analyzeStage = $state<"" | "spawned" | "thinking" | "writing">("");
  // Map a stage to the earliest ANALYZE_STEPS index it justifies.
  const STAGE_FLOOR: Record<string, number> = { spawned: 0, thinking: 3, writing: 4 };

  onMount(() => {
    const poll = () => void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
    poll();
    void invoke<ConvoStat[]>("assistant_stats")
      .then((s) => { stats = s; })
      .catch((e) => { statsError = String(e); });
    void invoke<TurnPerfStats>("query_turn_perf")
      .then((p) => { perfStats = p; })
      .catch(() => {});
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
  // Cost-trend bars normalise to the costliest day in the shown window.
  const costPeak = $derived.by(() => {
    let max = 0;
    for (const [, c] of perfStats?.cost_by_day.slice(0, 7) ?? []) max = Math.max(max, c);
    return max;
  });
  const costBarPct = (cost: number) => (costPeak > 0 ? Math.max(4, Math.round((cost / costPeak) * 100)) : 0);

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

  // ── This-session snapshot (live, in-memory) ── pure rollup, no Date.now /
  // full-turns bundling that snapshot() would re-do every render.
  const session = $derived(summarizeSession(assistant.telemetry.turns, assistant.telemetry.events));

  // ── All-time rollup (persisted) ──
  const totals = $derived(stats ? summarize(stats) : null);
  const models = $derived(stats ? perModel(stats) : []);
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
  const latencySignal = $derived.by(() => {
    const p90 = perfStats?.p90_ttft_text_ms;
    if (p90 == null) return null;
    return p90 < 4000 ? "ok" : p90 < 9000 ? "slow" : "degraded";
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
    if (latencyTint) dims.push({ k: "Latency", tint: latencyTint, v: fmtMs(perfStats?.p90_ttft_text_ms ?? null) });
    if (cacheTint && perfStats?.cache_hit_rate != null) dims.push({ k: "Cache", tint: cacheTint, v: `${Math.round(perfStats.cache_hit_rate * 100)}%` });
    if (usage.rateLimits) dims.push({ k: "Plan", tint: rateLimitRisk, v: `${peakLimit}%` });
    if (dims.length === 0) return null;
    const worst = dims.reduce((a, b) => (RANK[b.tint] > RANK[a.tint] ? b : a));
    const tint = worst.tint;
    const label = tint === "ok" ? "Healthy" : tint === "warn" ? "Needs a look" : "Action needed";
    // Green note names only the dimensions actually checked (some are absent
    // below their sample floor), so it never claims a clean bill on data it
    // didn't have. Oxford-join the lowercased dimension names.
    const okNames = dims.map((d) => d.k.toLowerCase());
    const okList = okNames.length === 1 ? okNames[0]
      : okNames.length === 2 ? `${okNames[0]} and ${okNames[1]}`
      : `${okNames.slice(0, -1).join(", ")}, and ${okNames[okNames.length - 1]}`;
    const note = tint === "ok"
      ? `${okList.charAt(0).toUpperCase()}${okList.slice(1)} ${okNames.length === 1 ? "looks" : "look"} good.`
      : `${worst.k} ${tint === "hot" ? "needs attention" : "is worth a look"}.`;
    // Problem dimensions get a labeled value pill on the right of the strip; when
    // all-clear, the three dimension dots stand in (nothing to flag).
    const flagged = dims.filter((d) => d.tint !== "ok");
    return { tint, label, note, dims, flagged };
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
            })),
          }
        : null,
      costTrend: perfStats?.cost_by_day.slice(0, 7) ?? [],
      // Pre-digested verdicts: the advisor leads with these, then cites the raw
      // number from perf/planLimits that justifies the call.
      signals: { latency: latencySignal, cache: cacheSignal, rateLimitRisk },
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
  const cards = $derived(
    usage.advice ? [...usage.advice.cards].sort((a, b) => (impactRank[a.impact] ?? 3) - (impactRank[b.impact] ?? 3)) : [],
  );

  // ── Current harness config ── the live values an apply action would change.
  // Pretty labels so newcomers see plain words, not "xhigh"/"sonnet".
  const EFFORT_LABEL: Record<string, string> = {
    none: "Off", quick: "Quick", smart: "Smart", deep: "Deep", ultra: "Ultra",
  };
  const MODEL_LABEL: Record<string, string> = {
    opus: "Opus", sonnet: "Sonnet", haiku: "Haiku",
    "claude-opus-4-7": "Opus", "claude-fable-5": "Fable",
  };
  const modelKey = (m: string) => (m === "opus" || m === "claude-opus-4-7" ? "opus" : m === "haiku" ? "haiku" : m === "claude-fable-5" ? "fable" : "sonnet");
  // Inverse of modelKey: the advisor emits short keys ("fable"), but setModel
  // wants a ModelSel — and "fable" is NOT a valid ModelSel ("claude-fable-5" is).
  const applyKeyToModel = (k: string): ModelSel => (k === "fable" ? "claude-fable-5" : (k as ModelSel));
  const budgetLabel = (n: number | null) => (n == null ? "No cap" : `$${n.toFixed(2)}/turn`);

  // Per-turn dollar budget is only a real knob in API-key mode (pay-per-token).
  // For a subscription session it's inert (usage-limit windows govern spend), so
  // it's dropped from the "knobs Rift can tune" list rather than shown as a lie.
  const configRows = $derived([
    { k: "Default effort", v: EFFORT_LABEL[assistant.thinkingEffort] ?? assistant.thinkingEffort },
    { k: "Default model", v: MODEL_LABEL[assistant.model] ?? assistant.model },
    ...(assistant.hasApiKey ? [{ k: "Per-turn budget", v: budgetLabel(assistant.maxBudgetUsd) }] : []),
  ]);

  // Read the live value a given apply action would replace — for current→new.
  function currentValueFor(a: AdviceApply): string {
    if (a.kind === "effort") return EFFORT_LABEL[assistant.thinkingEffort] ?? assistant.thinkingEffort;
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
    if (a.kind === "effort") return assistant.thinkingEffort === a.value;
    if (a.kind === "model") return modelKey(assistant.model) === a.value;
    return assistant.maxBudgetUsd === a.value;
  }

  // Per-card apply bookkeeping, keyed by card title (stable within one advice
  // set). Holds the prior value for one-tap undo + a transient "applied" flag.
  type ApplyState = { prev: string | number | null; appliedAt: number };
  let applied = $state<Record<string, ApplyState>>({});

  async function applyAction(title: string, a: AdviceApply) {
    // Snapshot the prior value so undo can restore it exactly.
    let prev: string | number | null;
    if (a.kind === "effort") {
      prev = assistant.thinkingEffort;
      assistant.setThinkingEffort(a.value as never);
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
    if (a.kind === "effort") assistant.setThinkingEffort(st.prev as never);
    else if (a.kind === "model") assistant.setModel(applyKeyToModel(String(st.prev ?? "sonnet")));
    else await assistant.setMaxBudgetUsd(st.prev as number | null);
    const next = { ...applied };
    delete next[title];
    applied = next;
  }
</script>

<div class="sb-main">
  <PageHero
    eyebrow="Experimental"
    title="AI Health"
    desc="See how you're using Claude through Rift — your plan limits, where your usage goes, and one-tap advice on how to stretch your plan further."
  >
    {#snippet icon()}<HeartPulse size={22} strokeWidth={1.75} />{/snippet}
    {#snippet chip()}
      {#if healthScore}
        <span class="ah-chip {healthScore.tint}" aria-label="Overall health: {healthScore.label}. {healthScore.note}">
          <span class="dot"></span>{healthScore.label}
        </span>
      {:else}
        <span class="ah-chip">
          <span class="dot"></span>Loading
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
          {#if healthScore.flagged.length > 0}
            <span class="ah-vs-flags">
              {#each healthScore.flagged as d (d.k)}
                <span class="ah-vs-flag {d.tint}" aria-label="{d.k}: {d.tint === 'warn' ? 'elevated' : 'high'}, {d.v}">
                  <span class="ah-vs-flag-k">{d.k}</span>
                  <span class="ah-vs-flag-v">{d.v}</span>
                </span>
              {/each}
            </span>
          {:else}
            <span class="ah-vs-dims" aria-hidden="true">
              {#each healthScore.dims as d (d.k)}
                <span class="ah-vs-dim {d.tint}" title="{d.k}: good"></span>
              {/each}
            </span>
          {/if}
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
                          <button class="ah-apply-btn" type="button" onclick={() => void applyAction(c.title, a)}>
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

      <!-- ── Current setup ── the live harness knobs an apply action tunes -->
      <section class="ah-card">
        <div class="ah-card-h"><SlidersHorizontal size={15} strokeWidth={1.9} />Your current setup</div>
        <div class="ah-cfg">
          {#each configRows as row (row.k)}
            <div class="ah-cfg-row"><span class="ah-cfg-k">{row.k}</span><span class="ah-cfg-v">{row.v}</span></div>
          {/each}
        </div>
        {#if !usage.advice}
          <p class="ah-cfg-note">These are the knobs Rift can tune for you. Advice above applies straight to them — one tap, undoable.</p>
        {/if}
      </section>

      <!-- ── Plan limits ── -->
      <section class="ah-card">
        <div class="ah-card-h"><Gauge size={15} strokeWidth={1.9} />Plan limits{#if fetchedAgo && limitRows.length > 0}<span class="ah-asof">{fetchedAgo}</span>{/if}</div>
        {#if usage.rateLimitsError}
          <p class="ah-muted">{usage.rateLimitsError}</p>
        {:else if limitRows.length === 0}
          <p class="ah-muted">No subscription limits to show — sign in with a Claude plan to see them here.</p>
        {:else}
          <p class="ah-card-sub">How much of each usage window you've used. When a bar fills up, Claude pauses until it resets at the time shown.</p>
          <div class="ah-bars">
            {#each limitRows as row (row.k)}
              {@const u = Math.round(row.w.utilization)}
              <div class="ah-bar-row">
                <div class="ah-bar-top">
                  <span class="ah-bar-k">{row.k}</span>
                  <span class="ah-bar-v">{u}%<span class="ah-bar-reset">{fmtReset(row.w.resetsAt)}</span></span>
                </div>
                <div class="ah-track"><div class="ah-fill {zone(u)}" style:width="{Math.min(100, u)}%"></div></div>
              </div>
            {/each}
          </div>
        {/if}
        {#if usage.rateLimits?.extraUsage?.isEnabled}
          {@const x = usage.rateLimits.extraUsage}
          {@const xu = x.utilization != null ? Math.round(x.utilization) : null}
          <div class="ah-bars ah-extra">
            <div class="ah-bar-row">
              <div class="ah-bar-top">
                <span class="ah-bar-k">Add-on credits</span>
                <span class="ah-bar-v">
                  {#if x.usedCredits != null && x.monthlyLimit != null}
                    {fmtUsd(x.usedCredits)} / {fmtUsd(x.monthlyLimit)}
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
      <section class="ah-card">
        <div class="ah-card-h"><Wrench size={15} strokeWidth={1.9} />Where your usage goes</div>
        {#if statsError}
          <p class="ah-muted">Couldn't load your history: {statsError}</p>
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

          {#if models.length > 0}
            <div class="ah-models">
              {#each models as m (m.model)}
                <div class="ah-model-row">
                  <span class="ah-model-k">{m.label}</span>
                  <div class="ah-track sm"><div class="ah-fill ok" style:width="{Math.round(m.share * 100)}%"></div></div>
                  <span class="ah-model-v">{Math.round(m.share * 100)}%</span>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </section>

      <!-- ── Speed & efficiency (B2: persisted turn telemetry) ── plain-English
           labels: "first reply" = the wait before text starts; "typical" = the
           middle of your replies (median), "slower" = your slow tail (≈1 in 10,
           p90). Tooltips are banned app-wide (2026-06-15), so the meaning lives
           in the labels + the footnote, not on hover. -->
      {#if hasPerf && perfStats}
        <section class="ah-card">
          <div class="ah-card-h"><Gauge size={15} strokeWidth={1.9} />Speed &amp; efficiency</div>
          <p class="ah-card-sub">How fast Claude responds and how efficiently it reuses your conversation — measured across your last {fmtNum(perfStats.total_turns)} replies.</p>
          <div class="ah-tiles">
            <div class="ah-tile">
              <div class="ah-tile-v">{fmtMs(perfStats.p50_ttft_text_ms)}</div>
              <div class="ah-tile-k">typical wait to first reply</div>
            </div>
            {#if perfStats.p90_ttft_text_ms != null}
              <div class="ah-tile {latencyTint}">
                <div class="ah-tile-v">{fmtMs(perfStats.p90_ttft_text_ms)}</div>
                <div class="ah-tile-k">on a slow reply{#if latencyVerdict}<span class="ah-verdict {latencyTint}">{latencyVerdict}</span>{/if}</div>
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

          {#if latencySpark}
            <div class="ah-spark-row">
              <span class="ah-spark-k">slow-reply wait · last {perfStats.latency_p90_by_day.filter((d) => d[1] != null).length} days</span>
              <svg class="ah-spark" viewBox="0 0 {SPARK_W} {SPARK_H}" preserveAspectRatio="none" aria-hidden="true">
                <polyline class="ah-spark-line {latencySpark.tint}" points={latencySpark.line} fill="none" />
              </svg>
              <span class="ah-spark-v">{fmtMs(latencySpark.first)} → {fmtMs(latencySpark.last)}</span>
            </div>
          {/if}

          {#if perfStats.cost_by_day.length > 1}
            <div class="ah-trend">
              <span class="ah-trend-h">Spend per day</span>
              {#each perfStats.cost_by_day.slice(0, 7) as [day, cost] (day)}
                <div class="ah-trend-row">
                  <span class="ah-trend-k">{day}</span>
                  <div class="ah-track sm"><div class="ah-fill ok" style:width="{costBarPct(cost)}%"></div></div>
                  <span class="ah-trend-v">{fmtUsd(cost)}</span>
                </div>
              {/each}
            </div>
          {/if}

          <p class="ah-glossary">
            <strong>Typical</strong> is the middle of your replies — half are faster, half slower.
            <strong>Slow reply</strong> is one of your slower ones (about 1 in 10).
            <strong>Conversation reused</strong> is how much of the chat Claude remembers without re-reading it — higher means faster, cheaper replies.
          </p>
        </section>
      {/if}

      <!-- ── This session ── only shown once this session has recorded turns -->
      {#if session.totalTurns > 0}
      <section class="ah-card">
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
  .ah-wrap { max-width: 820px; margin: 0 auto; padding: 18px 24px 28px; display: flex; flex-direction: column; gap: 14px; }

  .ah-chip { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; padding: 4px 10px; border-radius: 999px; background: var(--accent-soft); color: var(--fg-muted); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-chip .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--fg-subtle); }
  .ah-chip.ok .dot { background: var(--accent); }
  .ah-chip.warn .dot { background: var(--warn); }
  .ah-chip.hot .dot { background: var(--danger); }

  .ah-advisor { display: flex; align-items: center; gap: 16px; padding: 20px; border-radius: 14px; background: linear-gradient(135deg, color-mix(in oklab, var(--accent) 13%, transparent), color-mix(in oklab, var(--accent) 4%, transparent)); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 22%, var(--ghost-border)); }
  .ah-adv-ic { width: 36px; height: 36px; flex: none; border-radius: 10px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .ah-adv-tx { flex: 1; min-width: 0; }
  .ah-adv-tt { font-size: 16px; font-weight: 700; letter-spacing: -0.015em; }

  /* Overall health verdict strip — the at-a-glance "am I OK?" line and the ONE
     place a flagged metric is stated (folds in the old separate signals row).
     Neutral card surface with a colored left-rule + dot per the stream design
     language (accent = meaningful signal only). Right side: labeled value pills
     for any flagged dimension, or three all-clear dots when nothing's wrong. */
  .ah-verdict-strip { display: flex; align-items: center; gap: 11px; padding: 13px 20px; border-radius: 14px; background: var(--surface-1, var(--bg-2)); box-shadow: inset 0 0 0 1px var(--border); border-left: 3px solid var(--fg-subtle); animation: ah-vs-in 0.42s var(--ease-page) both; }
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
  .ah-vs-note { font-size: var(--fs-sm); color: var(--fg-muted); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* All-clear: three dimension dots stand in for "nothing to flag". */
  .ah-vs-dims { display: inline-flex; gap: 5px; margin-left: auto; flex: none; }
  .ah-vs-dim { width: 7px; height: 7px; border-radius: 999px; background: var(--fg-faint); }
  .ah-vs-dim.ok { background: var(--accent); }
  .ah-vs-dim.warn { background: var(--warn); }
  .ah-vs-dim.hot { background: var(--danger); }
  /* Problem dimensions: labeled value pills inline on the right of the strip —
     the one place the flagged metric is stated (no separate chip row). */
  .ah-vs-flags { display: inline-flex; gap: 7px; margin-left: auto; flex: none; flex-wrap: wrap; justify-content: flex-end; }
  .ah-vs-flag { display: inline-flex; align-items: baseline; gap: 6px; padding: 3px 10px; border-radius: 999px; font-size: var(--fs-sm); background: var(--bg-2, var(--surface-2)); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-vs-flag.warn { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--warn) 36%, transparent); }
  .ah-vs-flag.hot { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--danger) 38%, transparent); }
  .ah-vs-flag-k { color: var(--fg-muted); }
  .ah-vs-flag.warn .ah-vs-flag-k { color: var(--warn); }
  .ah-vs-flag.hot .ah-vs-flag-k { color: var(--danger); }
  .ah-vs-flag-v { font-weight: 680; font-variant-numeric: tabular-nums; }

  .ah-adv-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.45; }
  .ah-adv-btn { flex: none; display: inline-flex; align-items: center; gap: 6px; padding: 9px 16px; border-radius: 10px; border: none; font-size: 13px; font-weight: 640; cursor: pointer; background: var(--accent); color: var(--accent-fg, #fff); }
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
  .ah-rec { display: flex; gap: 13px; padding: 15px 20px; border-radius: 14px; background: var(--surface-1, var(--bg-2)); box-shadow: inset 0 0 0 1px var(--border); }
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

  .ah-card { border-radius: 14px; padding: 15px 20px; background: var(--surface-1, var(--bg-2)); box-shadow: inset 0 0 0 1px var(--border); }
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

  .ah-tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 12px; }
  .ah-tile { padding: 12px 14px; border-radius: 10px; background: var(--bg-2, var(--surface-2)); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-tiles.sm .ah-tile { padding: 10px 12px; }
  .ah-tiles.sm .ah-tile-v { font-size: 18px; }
  .ah-tile-v { font-size: 20px; font-weight: 720; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .ah-tile-v.ah-tile-sm { font-size: 14px; font-weight: 640; }
  /* Label wraps as normal text; the verdict badge flows inline after the last
     word (was display:flex + align-items:center — which floated the badge
     vertically against multi-line labels and collided with the wrapped text). */
  .ah-tile-k { font-size: 11.5px; color: var(--fg-subtle); margin-top: 2px; line-height: 1.45; }
  /* WS1: signal tints — a faint wash + colored inset ring on the tile whose
     verdict matters, reusing the shared accent/warn/danger tokens. */
  .ah-tile.ok { background: color-mix(in srgb, var(--accent) 8%, var(--bg-2, var(--surface-2))); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 38%, transparent); }
  .ah-tile.warn { background: color-mix(in srgb, var(--warn) 10%, var(--bg-2, var(--surface-2))); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--warn) 42%, transparent); }
  .ah-tile.hot { background: color-mix(in srgb, var(--danger) 10%, var(--bg-2, var(--surface-2))); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--danger) 45%, transparent); }
  .ah-verdict { display: inline-block; margin-left: 5px; font-size: 9.5px; font-weight: 700; letter-spacing: 0.03em; text-transform: uppercase; padding: 1px 5px; border-radius: 999px; line-height: 1.5; white-space: nowrap; vertical-align: baseline; }
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
