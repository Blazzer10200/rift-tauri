<script lang="ts">
  import { onMount } from "svelte";
  import { BarChart3, Flame, MessageSquare, Wrench, CalendarDays, Clock, Sparkles, Cpu, Coins } from "lucide-svelte";
  import { homeStats } from "$lib/state/homeStats.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import {
    type StatRange,
    filterRange, summarize, streaks, peakHour, topModel, perModel, heatmap, dailySeries,
    intensity, funFact, fmtInt, fmtCompact, fmtCost, hourLabel,
  } from "./statsHelpers";

  let tab = $state<"overview" | "models">("overview");
  let range = $state<StatRange>("all");
  const now = Date.now();

  onMount(() => void homeStats.load());

  const all = $derived(homeStats.stats ?? []);
  const empty = $derived(all.length === 0);
  const ranged = $derived(filterRange(all, range, now));

  const totals = $derived(summarize(ranged));
  const strk = $derived(streaks(ranged, now));
  const peak = $derived(peakHour(ranged));
  const fav = $derived(topModel(ranged));
  const fact = $derived(funFact(totals));

  // Activity calendar is a fixed trailing window (independent of the range
  // chips, which scope the KPI numbers + model split) — 18 weeks reads as a
  // proper GitHub-style calendar where a 7-day grid wouldn't.
  const HEAT_DAYS = 18 * 7;
  const heat = $derived(heatmap(all, HEAT_DAYS, now));

  const models = $derived(perModel(ranged));
  const barDays = $derived(range === "7d" ? 7 : 30);
  const bars = $derived(dailySeries(all, barDays, now));
  const barMax = $derived(Math.max(1, ...bars.map((b) => b.messages)));

  const MODEL_COLORS = [
    "oklch(0.74 0.15 163)", // emerald — house accent
    "oklch(0.70 0.13 232)", // blue
    "oklch(0.72 0.13 285)", // violet
    "oklch(0.80 0.13 85)", // amber
    "oklch(0.70 0.16 18)", // rose
    "oklch(0.75 0.10 200)", // teal
  ];
  const modelColor = (i: number) => MODEL_COLORS[i % MODEL_COLORS.length];

  const RANGES: { id: StatRange; label: string }[] = [
    { id: "all", label: "All" },
    { id: "30d", label: "30d" },
    { id: "7d", label: "7d" },
  ];
  const dayLabel = (ms: number) => new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });
</script>

<section class="stats">
  <header class="st-head">
    <div class="seg" role="tablist" aria-label="Stats view">
      <button role="tab" aria-selected={tab === "overview"} class:on={tab === "overview"} onclick={() => (tab = "overview")}>
        <Sparkles size={12} />Overview
      </button>
      <button role="tab" aria-selected={tab === "models"} class:on={tab === "models"} onclick={() => (tab = "models")}>
        <BarChart3 size={12} />Models
      </button>
    </div>
    <div class="ranges" role="group" aria-label="Time range">
      {#each RANGES as r (r.id)}
        <button class:on={range === r.id} onclick={() => (range = r.id)}>{r.label}</button>
      {/each}
    </div>
  </header>

  {#if homeStats.loading && homeStats.stats === null}
    <div class="st-skeleton">
      {#each Array(8) as _, i (i)}<div class="sk-cell"></div>{/each}
    </div>
  {:else if empty}
    <div class="st-empty">
      <span class="se-ic"><BarChart3 size={20} /></span>
      <div class="se-t">No stats yet</div>
      <div class="se-s">Your activity, models, and streaks show up here once you start chatting.</div>
    </div>
  {:else if tab === "overview"}
    <div class="kpis">
      <div class="kpi"><span class="k-ic"><MessageSquare size={13} /></span><span class="k-l">Sessions</span><span class="k-v">{fmtInt(totals.sessions)}</span></div>
      <div class="kpi"><span class="k-ic"><MessageSquare size={13} /></span><span class="k-l">Messages</span><span class="k-v">{fmtInt(totals.messages)}</span></div>
      <div class="kpi"><span class="k-ic"><Wrench size={13} /></span><span class="k-l">Tool calls</span><span class="k-v">{fmtInt(totals.toolCalls)}</span></div>
      <div class="kpi"><span class="k-ic"><Coins size={13} /></span><span class="k-l">Spend</span><span class="k-v">{fmtCost(totals.cost)}</span></div>

      <div class="kpi"><span class="k-ic"><CalendarDays size={13} /></span><span class="k-l">Active days</span><span class="k-v">{fmtInt(totals.activeDays)}</span></div>
      <div class="kpi">
        <span class="k-ic"><Flame size={13} /></span><span class="k-l">Streak</span>
        <span class="k-v">{strk.current}<span class="k-u">d</span>{#if strk.longest > strk.current}<span class="k-sub">best {strk.longest}d</span>{/if}</span>
      </div>
      <div class="kpi"><span class="k-ic"><Clock size={13} /></span><span class="k-l">Peak hour</span><span class="k-v">{hourLabel(peak)}</span></div>
      <div class="kpi"><span class="k-ic"><Cpu size={13} /></span><span class="k-l">Top model</span><span class="k-v sm">{fav ?? "—"}</span></div>
    </div>

    <div class="heat-wrap">
      <div class="heat" style="grid-template-rows: repeat(7, 1fr);">
        {#each Array(heat.leadPad) as _, i (`pad${i}`)}<span class="cell pad"></span>{/each}
        {#each heat.cells as c (c.day)}
          <span
            class="cell"
            data-l={intensity(c.messages, heat.max)}
            use:tooltip={`${dayLabel(c.ms)} · ${fmtInt(c.messages)} msg${c.messages === 1 ? "" : "s"}${c.sessions ? ` · ${c.sessions} session${c.sessions === 1 ? "" : "s"}` : ""}`}
          ></span>
        {/each}
      </div>
      <div class="heat-foot">
        <span class="hf-cap">last 18 weeks · messages / day</span>
        <span class="legend">Less {#each [0, 1, 2, 3, 4] as l (l)}<span class="cell sm" data-l={l}></span>{/each} More</span>
      </div>
    </div>

    {#if fact}
      <div class="fact"><Sparkles size={12} />{fact}</div>
    {/if}
  {:else}
    <!-- Models tab -->
    <div class="bars" style="--bars: {bars.length};">
      {#each bars as b (b.day)}
        <span class="bar-col" use:tooltip={`${dayLabel(b.ms)} · ${fmtInt(b.messages)} msgs`}>
          <span class="bar" style="height: {Math.max(b.messages > 0 ? 6 : 0, (b.messages / barMax) * 100)}%;"></span>
        </span>
      {/each}
    </div>
    <div class="bars-axis">
      <span>{bars.length ? dayLabel(bars[0].ms) : ""}</span>
      <span class="ba-mid">messages / day</span>
      <span>{bars.length ? dayLabel(bars[bars.length - 1].ms) : ""}</span>
    </div>

    <div class="mdl-list">
      {#if models.length === 0}
        <div class="st-empty mini">No model activity in this range.</div>
      {:else}
        {#each models.slice(0, 6) as m, i (m.model)}
          <div class="mdl-row">
            <span class="mdl-dot" style="background: {modelColor(i)};"></span>
            <span class="mdl-name">{m.label}</span>
            <span class="mdl-bar"><span class="mdl-fill" style="width: {Math.max(2, m.share * 100)}%; background: {modelColor(i)};"></span></span>
            <span class="mdl-meta mono">{fmtCompact(m.messages)} msg · {fmtCost(m.cost)}</span>
            <span class="mdl-pct mono">{(m.share * 100).toFixed(m.share >= 0.1 ? 0 : 1)}%</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</section>

<style>
  .stats {
    flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; gap: 12px;
    padding: 16px 18px 14px; border-radius: 16px;
    background: var(--surface); border: 1px solid var(--border);
    box-shadow: inset 0 1px 0 color-mix(in oklch, white 2.5%, transparent);
  }

  /* Header: tabs left, range chips right */
  .st-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex: none; }
  .seg, .ranges { display: inline-flex; align-items: center; gap: 2px; padding: 3px; border-radius: 10px; background: var(--bg-inset); border: 1px solid var(--border); }
  .seg button, .ranges button {
    display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border: 0; border-radius: 7px;
    background: transparent; color: var(--fg-muted); font: inherit; font-size: var(--fs-xs); font-weight: 600; cursor: pointer;
    transition: background 120ms, color 120ms;
  }
  .ranges button { padding: 0 10px; font-family: var(--font-mono); }
  .seg button:hover, .ranges button:hover { color: var(--fg); }
  .seg button.on { background: var(--surface); color: var(--fg); box-shadow: 0 1px 2px color-mix(in oklab, black 25%, transparent); }
  .ranges button.on { background: var(--accent-soft); color: var(--accent); }

  /* KPI grid — 4 × 2 */
  .kpis { flex: none; display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
  .kpi {
    display: grid; grid-template-columns: auto 1fr; grid-template-rows: auto auto; gap: 0 8px;
    align-items: center; padding: 10px 12px; border-radius: 11px;
    background: var(--bg-inset); border: 1px solid var(--border);
  }
  .kpi .k-ic { grid-row: 1 / 3; width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); }
  .k-l { font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--fg-faint); align-self: end; }
  .k-v { font-size: 19px; font-weight: 720; letter-spacing: -0.02em; color: var(--fg); line-height: 1.05; font-variant-numeric: tabular-nums; align-self: start; display: flex; align-items: baseline; gap: 5px; min-width: 0; }
  .k-v.sm { font-size: 14px; font-weight: 680; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: block; }
  .k-u { font-size: 11px; font-weight: 600; color: var(--fg-subtle); }
  .k-sub { font-size: 9.5px; font-weight: 600; color: var(--fg-faint); letter-spacing: 0; text-transform: none; }

  /* Heatmap */
  .heat-wrap { flex: none; display: flex; flex-direction: column; gap: 7px; }
  .heat { display: grid; grid-auto-flow: column; grid-auto-columns: 1fr; gap: 3px; }
  .cell { aspect-ratio: 1; width: 100%; border-radius: 3px; background: color-mix(in oklab, var(--fg) 7%, transparent); }
  .cell.pad { background: transparent; }
  .cell.sm { width: 10px; height: 10px; aspect-ratio: auto; border-radius: 2px; }
  .cell[data-l="0"] { background: color-mix(in oklab, var(--fg) 7%, transparent); }
  .cell[data-l="1"] { background: oklch(0.34 0.06 var(--accent-h)); }
  .cell[data-l="2"] { background: oklch(0.50 0.11 var(--accent-h)); }
  .cell[data-l="3"] { background: oklch(0.64 0.14 var(--accent-h)); }
  .cell[data-l="4"] { background: oklch(0.80 0.16 var(--accent-h)); }
  .heat-foot { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .hf-cap { font-size: 10px; color: var(--fg-faint); }
  .legend { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; color: var(--fg-faint); }

  .fact {
    flex: none; display: inline-flex; align-items: center; gap: 7px; margin-top: auto;
    padding: 8px 11px; border-radius: 10px; font-size: var(--fs-xs); color: var(--fg-muted);
    background: var(--bg-inset); border: 1px solid var(--border);
  }
  .fact :global(svg) { color: var(--accent); flex-shrink: 0; }

  /* Models tab — daily bars */
  .bars { flex: none; display: grid; grid-template-columns: repeat(var(--bars), 1fr); align-items: end; gap: 3px; height: 96px; padding: 4px 0; }
  .bar-col { display: flex; align-items: flex-end; justify-content: center; height: 100%; }
  .bar { width: 100%; max-width: 16px; border-radius: 3px 3px 1px 1px; min-height: 0;
    background: linear-gradient(180deg, oklch(0.78 0.15 var(--accent-h)), oklch(0.6 0.14 var(--accent-h))); transition: height 200ms var(--ease-page, ease); }
  .bars-axis { display: flex; align-items: center; justify-content: space-between; font-family: var(--font-mono); font-size: 9.5px; color: var(--fg-faint); margin-top: -2px; }
  .ba-mid { color: var(--fg-subtle); }

  /* Models tab — per-model breakdown */
  .mdl-list { display: flex; flex-direction: column; gap: 7px; margin-top: 4px; }
  .mdl-row { display: grid; grid-template-columns: 10px minmax(64px, auto) 1fr auto auto; align-items: center; gap: 10px; }
  .mdl-dot { width: 9px; height: 9px; border-radius: 3px; }
  .mdl-name { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .mdl-bar { height: 6px; border-radius: 999px; background: var(--bg-inset); overflow: hidden; }
  .mdl-fill { display: block; height: 100%; border-radius: 999px; opacity: 0.85; transition: width 220ms var(--ease-page, ease); }
  .mdl-meta { font-size: 10.5px; color: var(--fg-subtle); white-space: nowrap; }
  .mdl-pct { font-size: 11px; font-weight: 680; color: var(--fg-2); min-width: 34px; text-align: right; }
  .mono { font-family: var(--font-mono); }

  /* Empty + skeleton + loading */
  .st-empty { flex: 1; min-height: 120px; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 6px; padding: 18px; }
  .st-empty.mini { min-height: 60px; }
  .se-ic { width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); margin-bottom: 4px; }
  .se-t { font-size: var(--fs-md); font-weight: 650; color: var(--fg); }
  .se-s { font-size: var(--fs-xs); color: var(--fg-subtle); max-width: 280px; }
  .st-skeleton { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
  .sk-cell { height: 54px; border-radius: 11px; background: linear-gradient(90deg, var(--bg-inset), var(--surface-hover), var(--bg-inset)); background-size: 200% 100%; animation: sk 1.3s ease-in-out infinite; }
  @keyframes sk { to { background-position: -200% 0; } }
  @media (prefers-reduced-motion: reduce) { .sk-cell { animation: none; } }

  /* Narrow: KPIs collapse to 2 cols */
  @media (max-width: 1240px) {
    .kpis, .st-skeleton { grid-template-columns: repeat(2, 1fr); }
  }
</style>
