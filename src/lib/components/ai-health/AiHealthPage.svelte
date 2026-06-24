<script lang="ts">
  // AI Health — the observe layer (Stage 1). Surfaces what Rift already knows
  // about a user's plan + usage, and seats the "Analyze my usage" advisor
  // (Stage 2) that reasons over it via the user's own Claude. Coaches newcomers
  // in plain English; charts are supporting cast.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
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
  type TurnPerfStats = {
    p50_ttft_text_ms: number | null;
    p90_ttft_text_ms: number | null;
    p50_duration_ms: number | null;
    p90_duration_ms: number | null;
    cache_hit_rate: number | null;
    total_output_tokens: number;
    cost_by_day: [string, number][];
    total_turns: number;
  };
  let perfStats = $state<TurnPerfStats | null>(null);

  onMount(() => {
    void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
    void invoke<ConvoStat[]>("assistant_stats")
      .then((s) => { stats = s; })
      .catch((e) => { statsError = String(e); });
    void invoke<TurnPerfStats>("query_turn_perf")
      .then((p) => { perfStats = p; })
      .catch(() => {});
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
    if (!usage.analyzing) { stepIdx = 0; return; }
    // Advance every 3.2s, clamping at the last step (so a long cold-start lands
    // on "writing up…" rather than looping back to the start).
    const id = setInterval(() => {
      stepIdx = Math.min(stepIdx + 1, ANALYZE_STEPS.length - 1);
    }, 3200);
    return () => clearInterval(id);
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
      assistant.setModel(a.value as ModelSel);
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
    else if (a.kind === "model") assistant.setModel((st.prev as ModelSel) ?? "sonnet");
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
      {#if usage.rateLimits}
        <span class="ah-chip {zone(peakLimit)}">
          <span class="dot"></span>{peakLimit}% used
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
        <div class="ah-adv-ic"><Sparkles size={20} strokeWidth={1.75} /></div>
        <div class="ah-adv-tx">
          <div class="ah-adv-tt">Let Rift tune itself to how you work</div>
          <div class="ah-adv-sub">
            Rift reads your setup and recent usage, then asks your own Claude for
            a few plain-English changes that'll make your plan go further. Nothing
            changes until you say so.
          </div>
        </div>
        <button class="ah-adv-btn" type="button" onclick={analyze} disabled={usage.analyzing}>
          {#if usage.analyzing}
            <Loader2 size={15} strokeWidth={2} class="ah-spin" />Analyzing…
          {:else}
            {usage.advice ? "Re-analyze" : "Analyze my usage"}<ArrowRight size={15} strokeWidth={2} />
          {/if}
        </button>
      </div>

      {#if usage.analyzing}
        <div class="ah-analyzing" role="status" aria-live="polite">
          <div class="ah-an-orb">
            <Sparkles size={18} strokeWidth={1.9} class="ah-an-orb-ic" />
            <span class="ah-an-ring"></span>
          </div>
          <div class="ah-an-tx">
            <div class="ah-an-tt">Analyzing your usage{#each [0, 1, 2] as d (d)}<span class="ah-an-dot" style:--d="{d * 0.18}s">.</span>{/each}</div>
            {#key stepIdx}
              <div class="ah-an-step">{ANALYZE_STEPS[stepIdx]}</div>
            {/key}
            <div class="ah-an-hint">Your own Claude is reasoning over this privately — the first run can take up to a minute while it warms up. Nothing changes until you choose to.</div>
            <div class="ah-an-shimmer" aria-hidden="true"></div>
          </div>
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
        <p class="ah-cfg-note">These are the knobs Rift can tune for you. Advice above applies straight to them — one tap, undoable.</p>
      </section>

      <!-- ── Plan limits ── -->
      <section class="ah-card">
        <div class="ah-card-h"><Gauge size={15} strokeWidth={1.9} />Plan limits</div>
        {#if usage.rateLimitsError}
          <p class="ah-muted">{usage.rateLimitsError}</p>
        {:else if limitRows.length === 0}
          <p class="ah-muted">No subscription limits to show — sign in with a Claude plan to see them here.</p>
        {:else}
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
            <div class="ah-tile"><div class="ah-tile-v">{streak.current}🔥</div><div class="ah-tile-k">day streak</div></div>
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

      <!-- ── Performance (B2: persisted turn telemetry) ── -->
      {#if hasPerf && perfStats}
        <section class="ah-card">
          <div class="ah-card-h"><Gauge size={15} strokeWidth={1.9} />Performance</div>
          <div class="ah-tiles">
            <div class="ah-tile"><div class="ah-tile-v">{fmtMs(perfStats.p50_ttft_text_ms)}</div><div class="ah-tile-k">p50 first reply</div></div>
            {#if perfStats.p90_ttft_text_ms != null}
              <div class="ah-tile"><div class="ah-tile-v">{fmtMs(perfStats.p90_ttft_text_ms)}</div><div class="ah-tile-k">p90 first reply</div></div>
            {/if}
            <div class="ah-tile"><div class="ah-tile-v">{fmtMs(perfStats.p50_duration_ms)}</div><div class="ah-tile-k">p50 turn time</div></div>
            {#if perfStats.cache_hit_rate != null}
              <div class="ah-tile"><div class="ah-tile-v">{Math.round(perfStats.cache_hit_rate * 100)}%</div><div class="ah-tile-k">cache hit rate</div></div>
            {/if}
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(perfStats.total_output_tokens)}</div><div class="ah-tile-k">tokens out</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(perfStats.total_turns)}</div><div class="ah-tile-k">turns measured</div></div>
          </div>

          {#if perfStats.cost_by_day.length > 0}
            <div class="ah-trend">
              {#each perfStats.cost_by_day.slice(0, 7) as [day, cost] (day)}
                <div class="ah-trend-row">
                  <span class="ah-trend-k">{day}</span>
                  <div class="ah-track sm"><div class="ah-fill ok" style:width="{costBarPct(cost)}%"></div></div>
                  <span class="ah-trend-v">{fmtUsd(cost)}</span>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── This session ── -->
      <section class="ah-card">
        <div class="ah-card-h"><HeartPulse size={15} strokeWidth={1.9} />This session</div>
        {#if session.totalTurns === 0}
          <p class="ah-muted">No turns yet this session — start a conversation and live stats land here.</p>
        {:else}
          <div class="ah-tiles">
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(session.totalTurns)}</div><div class="ah-tile-k">turns</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtUsd(session.totalCostUsd)}</div><div class="ah-tile-k">spend</div></div>
            <div class="ah-tile"><div class="ah-tile-v">{fmtNum(session.toolCallTotal)}</div><div class="ah-tile-k">tool calls</div></div>
            {#if session.zeroToolTurns > 0}
              <div class="ah-tile"><div class="ah-tile-v">{fmtNum(session.zeroToolTurns)}</div><div class="ah-tile-k">chat-only turns</div></div>
            {/if}
            {#if session.avgTtfpMs != null}
              <div class="ah-tile"><div class="ah-tile-v">{(session.avgTtfpMs / 1000).toFixed(1)}s</div><div class="ah-tile-k">avg first reply</div></div>
            {/if}
          </div>
        {/if}
      </section>

    </div>
  </div>
</div>

<style>
  .sb-main { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .ah-scroll { flex: 1; min-height: 0; overflow-y: auto; }
  .ah-wrap { max-width: 820px; margin: 0 auto; padding: 18px 40px 28px; display: flex; flex-direction: column; gap: 14px; }

  .ah-chip { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; padding: 4px 10px; border-radius: 999px; background: var(--accent-soft); color: var(--fg-muted); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-chip .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--fg-subtle); }
  .ah-chip.ok .dot { background: var(--accent); }
  .ah-chip.warn .dot { background: #e0a82e; }
  .ah-chip.hot .dot { background: #e0573e; }

  .ah-advisor { display: flex; align-items: center; gap: 16px; padding: 18px 20px; border-radius: 14px; background: linear-gradient(135deg, color-mix(in oklab, var(--accent) 10%, transparent), color-mix(in oklab, var(--accent) 3%, transparent)); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-adv-ic { width: 40px; height: 40px; flex: none; border-radius: 11px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .ah-adv-tx { flex: 1; min-width: 0; }
  .ah-adv-tt { font-size: 15px; font-weight: 680; letter-spacing: -0.01em; }
  .ah-adv-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.45; }
  .ah-adv-btn { flex: none; display: inline-flex; align-items: center; gap: 6px; padding: 9px 16px; border-radius: 10px; border: none; font-size: 13px; font-weight: 640; cursor: pointer; background: var(--accent); color: var(--accent-fg, #fff); }
  .ah-adv-btn:disabled { opacity: 0.6; cursor: default; }
  .ah-adv-btn :global(.ah-spin) { animation: ah-spin 0.9s linear infinite; }
  @keyframes ah-spin { to { transform: rotate(360deg); } }

  /* ── animated "Analyzing…" card ── */
  .ah-analyzing { position: relative; display: flex; align-items: flex-start; gap: 15px; padding: 16px 18px; border-radius: 14px; overflow: hidden;
    background: linear-gradient(135deg, color-mix(in oklab, var(--accent) 9%, transparent), color-mix(in oklab, var(--accent) 3%, transparent));
    box-shadow: inset 0 0 0 1px var(--ghost-border); animation: ah-card-in 0.34s var(--ease-page) both; }
  @keyframes ah-card-in { from { opacity: 0; transform: translateY(-6px); } to { opacity: 1; transform: translateY(0); } }

  .ah-an-orb { position: relative; width: 40px; height: 40px; flex: none; border-radius: 50%; display: grid; place-items: center;
    background: var(--accent-soft); color: var(--accent); }
  .ah-an-orb :global(.ah-an-orb-ic) { position: relative; z-index: 1; animation: ah-orb-pulse 1.8s var(--ease-page) infinite; }
  /* full-circle spinner: a bordered ring with one bright quadrant — robust in every engine, reads in a single frame */
  .ah-an-ring { position: absolute; inset: -5px; border-radius: 50%; border: 3px solid color-mix(in oklab, var(--accent) 22%, transparent);
    border-top-color: var(--accent); border-right-color: var(--accent);
    animation: ah-spin 0.85s linear infinite; }
  @keyframes ah-orb-pulse { 0%, 100% { transform: scale(1); opacity: 0.85; } 50% { transform: scale(1.14); opacity: 1; } }

  .ah-an-tx { flex: 1; min-width: 0; }
  .ah-an-tt { font-size: 14px; font-weight: 680; letter-spacing: -0.01em; color: var(--fg); }
  .ah-an-dot { display: inline-block; animation: ah-dot 1.4s ease-in-out infinite; animation-delay: var(--d); opacity: 0.3; }
  @keyframes ah-dot { 0%, 60%, 100% { opacity: 0.25; } 30% { opacity: 1; } }
  .ah-an-step { font-size: var(--fs-sm); color: var(--accent); font-weight: 560; margin-top: 4px; animation: ah-step-in 0.4s var(--ease-page) both; }
  @keyframes ah-step-in { from { opacity: 0; transform: translateX(-5px); } to { opacity: 1; transform: translateX(0); } }
  .ah-an-hint { font-size: 11.5px; color: var(--fg-subtle); margin-top: 7px; line-height: 1.45; }

  .ah-an-shimmer { position: relative; margin-top: 11px; height: 4px; max-width: 260px; border-radius: 999px; overflow: hidden;
    background: color-mix(in oklab, var(--accent) 26%, var(--bg-inset)); }
  .ah-an-shimmer::after { content: ""; position: absolute; inset: 0; left: 0; width: 45%; border-radius: 999px;
    background: linear-gradient(90deg, transparent, var(--accent), transparent); animation: ah-shimmer 1.2s ease-in-out infinite; }
  @keyframes ah-shimmer { 0% { transform: translateX(-110%); } 100% { transform: translateX(320%); } }

  @media (prefers-reduced-motion: reduce) {
    .ah-analyzing, .ah-an-step { animation: none; }
    .ah-an-orb :global(.ah-an-orb-ic), .ah-an-ring, .ah-an-dot, .ah-an-shimmer::after { animation: none; }
    .ah-an-ring { border-color: color-mix(in oklab, var(--accent) 22%, transparent); border-top-color: var(--accent); border-right-color: var(--accent); }
  }
  .ah-advice-err { display: flex; align-items: center; gap: 8px; font-size: var(--fs-sm); color: #e0573e; padding: 10px 14px; border-radius: 10px; background: color-mix(in oklab, #e0573e 8%, transparent); }
  .ah-advice { display: flex; flex-direction: column; gap: 12px; }
  .ah-advice-sum { font-size: var(--fs-sm); color: var(--fg-muted); margin: 0 2px; line-height: 1.5; }
  .ah-cards { display: flex; flex-direction: column; gap: 10px; }
  .ah-rec { display: flex; gap: 13px; padding: 14px 16px; border-radius: 12px; background: var(--surface-1, var(--bg-2)); box-shadow: inset 0 0 0 1px var(--border); }
  .ah-rec-impact { flex: none; align-self: flex-start; font-size: 10px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; padding: 3px 8px; border-radius: 6px; }
  .ah-rec-impact.high { background: color-mix(in oklab, var(--accent) 18%, transparent); color: var(--accent); }
  .ah-rec-impact.medium { background: color-mix(in oklab, #e0a82e 18%, transparent); color: #e0a82e; }
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
  .ah-muted { font-size: var(--fs-sm); color: var(--fg-muted); margin: 0; line-height: 1.5; }

  .ah-bars { display: flex; flex-direction: column; gap: 14px; }
  .ah-bar-top { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 6px; }
  .ah-bar-k { font-size: var(--fs-sm); color: var(--fg-muted); }
  .ah-bar-v { font-size: var(--fs-sm); font-weight: 640; font-variant-numeric: tabular-nums; }
  .ah-bar-reset { font-size: 11px; font-weight: 400; color: var(--fg-subtle); margin-left: 8px; }
  .ah-track { height: 7px; border-radius: 999px; background: var(--ghost-border); overflow: hidden; }
  .ah-track.sm { height: 5px; flex: 1; }
  .ah-fill { height: 100%; border-radius: 999px; transition: width 0.3s ease; background: var(--accent); }
  .ah-fill.warn { background: #e0a82e; }
  .ah-fill.hot { background: #e0573e; }

  .ah-tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 12px; }
  .ah-tile { padding: 12px 14px; border-radius: 10px; background: var(--bg-2, var(--surface-2)); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .ah-tile-v { font-size: 20px; font-weight: 720; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .ah-tile-v.ah-tile-sm { font-size: 14px; font-weight: 640; }
  .ah-tile-k { font-size: 11px; color: var(--fg-subtle); margin-top: 2px; }

  .ah-models { display: flex; flex-direction: column; gap: 9px; margin-top: 16px; padding-top: 16px; border-top: 1px solid var(--border); }
  .ah-model-row { display: flex; align-items: center; gap: 12px; }
  .ah-model-k { font-size: var(--fs-sm); color: var(--fg-muted); width: 130px; flex: none; }
  .ah-model-v { font-size: var(--fs-sm); font-weight: 640; font-variant-numeric: tabular-nums; width: 38px; text-align: right; flex: none; }
  .ah-trend { display: flex; flex-direction: column; gap: 9px; margin-top: 16px; padding-top: 16px; border-top: 1px solid var(--border); }
  .ah-trend-row { display: flex; align-items: center; gap: 12px; }
  .ah-trend-k { font-size: var(--fs-sm); color: var(--fg-muted); width: 92px; flex: none; font-variant-numeric: tabular-nums; }
  .ah-trend-v { font-size: var(--fs-sm); font-weight: 640; font-variant-numeric: tabular-nums; width: 56px; text-align: right; flex: none; }
</style>
