<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { X, Loader2, Flame, Cpu, Wrench, DollarSign, Activity } from "lucide-svelte";
  import { fade, scale } from "svelte/transition";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import {
    type ConvoStat, type StatRange,
    filterRange, summarize, streaks, peakHour, perModel, topModel,
    dailySeries, dayLabel, summaryLine, funFact,
    fmtInt, fmtCompact, fmtCost,
  } from "./statsHelpers";

  let { onclose }: { onclose: () => void } = $props();

  let raw = $state<ConvoStat[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let range = $state<StatRange>("all");

  onMount(() => {
    invoke<ConvoStat[]>("assistant_stats")
      .then((s) => { raw = s; })
      .catch((e) => { error = String(e); })
      .finally(() => { loading = false; });
  });

  // Reactive so the day-boundary stays correct if the panel is left open past
  // midnight or the range changes (was a const frozen at mount → stale "today").
  let now = $state(Date.now());
  $effect(() => {
    void range;
    now = Date.now();
    const h = setInterval(() => { now = Date.now(); }, 60_000);
    return () => clearInterval(h);
  });
  const stats = $derived(filterRange(raw, range, now));
  const totals = $derived(summarize(stats));
  const strk = $derived(streaks(stats, now));
  const peak = $derived(peakHour(stats));
  const models = $derived(perModel(stats).filter((m) => m.messages > 0).slice(0, 5));
  const top = $derived(topModel(stats));
  // Range-aware window: 7d→14 bars, 30d→30, all→60. Reads dense, no dead corners.
  const windowDays = $derived(range === "7d" ? 14 : range === "30d" ? 30 : 60);
  const series = $derived(dailySeries(stats, windowDays, now));
  const summary = $derived(summaryLine(totals, peak));
  const fact = $derived(funFact(totals));
  const empty = $derived(!loading && !error && raw.length === 0);

  // Hue-keyed palette for stacked model segments — distinct, accent-family.
  const SEG_HUES = [163, 220, 285, 35, 130];
  const segs = $derived(
    models.map((m, i) => ({ ...m, hue: SEG_HUES[i % SEG_HUES.length] })),
  );

  function onKey(e: KeyboardEvent) { if (e.key === "Escape") onclose(); }
</script>

<svelte:window onkeydown={onKey} />

<div class="stats-backdrop" use:portal transition:fade={{ duration: 140 }}>
  <button class="sb-dismiss" type="button" aria-label="Close stats" onclick={onclose}></button>
  <div class="stats-panel" role="dialog" aria-label="Your activity" transition:scale={{ duration: 180, start: 0.97 }}>
    <header class="sp-head">
      <div class="sp-title"><Activity size={15} /><span>Activity</span></div>
      <div class="sp-range" role="group" aria-label="Time range">
        <button class:on={range === "7d"} type="button" onclick={() => (range = "7d")}>7d</button>
        <button class:on={range === "30d"} type="button" onclick={() => (range = "30d")}>30d</button>
        <button class:on={range === "all"} type="button" onclick={() => (range = "all")}>All</button>
      </div>
      <button class="sp-x" type="button" aria-label="Close" onclick={onclose}><X size={16} /></button>
    </header>

    {#if loading}
      <div class="sp-state"><Loader2 size={20} class="spin" /><span>Reading conversations…</span></div>
    {:else if error}
      <div class="sp-state err">Couldn't load stats: {error}</div>
    {:else if empty}
      <div class="sp-state">No conversations yet — your activity will show up here.</div>
    {:else}
      <div class="sp-body">
        <!-- Hero: the headline number + a human sentence, not a tile grid. -->
        <div class="hero">
          <div class="hero-num">
            <span class="hn-v">{fmtInt(totals.messages)}</span>
            <span class="hn-l">messages exchanged</span>
          </div>
          <p class="hero-sub">{summary}</p>
        </div>

        <!-- Dense daily bar chart — replaces the sparse GitHub heatmap. -->
        <section class="chart">
          <div class="chart-axis">
            <span class="ch-cap">Per day · last {windowDays}</span>
            {#if series.max > 0}<span class="ch-peak">peak {fmtInt(series.max)}</span>{/if}
          </div>
          <div class="chart-plot" style="--cols:{series.cells.length}">
            {#each series.cells as c (c.day)}
              <span
                class="ch-col"
                class:zero={c.messages === 0}
                style="--h:{series.max > 0 ? Math.max(c.messages > 0 ? 6 : 0, (c.messages / series.max) * 100) : 0}%"
                use:tooltip={`${dayLabel(c.ms)} · ${fmtInt(c.messages)} msg · ${c.sessions} session${c.sessions === 1 ? "" : "s"}`}
                aria-hidden="true"
              ></span>
            {/each}
          </div>
        </section>

        <!-- Supporting stat strip — secondary to the hero, tight inline row. -->
        <div class="strip">
          <div class="st"><Cpu size={13} /><b>{fmtInt(totals.sessions)}</b><span>sessions</span></div>
          <div class="st"><Wrench size={13} /><b>{fmtCompact(totals.toolCalls)}</b><span>tool calls</span></div>
          <div class="st"><DollarSign size={13} /><b>{fmtCost(totals.cost)}</b><span>spent</span></div>
          <div class="st"><Flame size={13} /><b>{strk.current}d</b><span>streak · best {strk.longest}d</span></div>
        </div>

        <!-- Model mix — one stacked proportion bar + a legend, not 5 tracks. -->
        {#if segs.length}
          <section class="mix">
            <div class="mix-h">Model mix{#if top}<span class="mix-sub">· mostly {top}</span>{/if}</div>
            <div class="mix-bar" role="img" aria-label="Model usage share">
              {#each segs as m (m.model)}
                <span class="mseg"
                  style="flex:{Math.max(0.04, m.share)}; --mh:{m.hue}"
                  use:tooltip={`${m.label} · ${fmtInt(m.messages)} msg · ${Math.round(m.share * 100)}%`}
                ></span>
              {/each}
            </div>
            <div class="mix-legend">
              {#each segs as m (m.model)}
                <span class="lg"><i style="--mh:{m.hue}"></i>{m.label}<small>{Math.round(m.share * 100)}%</small></span>
              {/each}
            </div>
          </section>
        {/if}

        {#if fact}
          <div class="sig">{fact}</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  :global(.stats-backdrop) { position: fixed; inset: 0; z-index: 60; display: grid; place-items: center;
    background: oklch(0 0 0 / 0.5); -webkit-backdrop-filter: blur(3px); backdrop-filter: blur(3px); }
  .sb-dismiss { position: absolute; inset: 0; background: none; cursor: default; }
  :global(.stats-panel) { position: relative; width: min(560px, calc(100vw - 40px)); max-height: min(82vh, 720px);
    display: flex; flex-direction: column; overflow: hidden;
    border-radius: var(--radius-lg, 16px); border: 1px solid var(--border-strong);
    background: color-mix(in oklab, var(--bg-elev-2) 96%, var(--bg));
    box-shadow: 0 40px 90px -36px oklch(0 0 0 / 0.7), var(--shadow-lg); }

  .sp-head { display: flex; align-items: center; gap: 12px; padding: 14px 16px; flex: none; border-bottom: 1px solid var(--border); }
  .sp-title { display: inline-flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 650; color: var(--fg); }
  .sp-title :global(svg) { color: var(--accent); }
  .sp-range { display: flex; gap: 2px; margin-left: auto; padding: 2px; border-radius: 8px; background: var(--bg-inset); border: 1px solid var(--border); }
  .sp-range button { height: 22px; padding: 0 10px; border-radius: 6px; font-size: 11px; font-weight: 600; color: var(--fg-subtle); transition: background var(--dur-fast), color var(--dur-fast); }
  .sp-range button:hover { color: var(--fg-2); }
  .sp-range button.on { background: var(--surface-active); color: var(--fg); }
  .sp-x { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; color: var(--fg-faint); }
  .sp-x:hover { background: var(--surface-hover); color: var(--fg); }

  .sp-state { padding: 48px 24px; display: flex; flex-direction: column; align-items: center; gap: 10px; color: var(--fg-subtle); font-size: 12.5px; text-align: center; }
  .sp-state.err { color: var(--danger); }
  :global(.stats-panel .spin) { animation: spStatsSpin 0.9s linear infinite; }
  @keyframes spStatsSpin { to { transform: rotate(360deg); } }

  .sp-body { padding: 20px; overflow-y: auto; display: flex; flex-direction: column; gap: 22px;
    scrollbar-width: thin; scrollbar-color: var(--border-strong) transparent; }

  /* ── Hero — the headline number tells the story ─────────────────────────── */
  .hero { display: flex; flex-direction: column; gap: 5px; }
  .hero-num { display: flex; align-items: baseline; gap: 10px; }
  .hn-v { font-size: 46px; font-weight: 760; line-height: 1; letter-spacing: -0.025em; color: var(--fg); font-variant-numeric: tabular-nums;
    background: linear-gradient(180deg, var(--fg), color-mix(in oklab, var(--accent) 30%, var(--fg)));
    -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }
  .hn-l { font-size: 13px; font-weight: 550; color: var(--fg-subtle); }
  .hero-sub { margin: 0; font-size: 12.5px; color: var(--fg-muted); }

  /* ── Daily bar chart — dense, honest, no dead corners ───────────────────── */
  .chart-axis { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 8px; }
  .ch-cap { font-size: 10.5px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--fg-faint); }
  .ch-peak { font-size: 10.5px; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .chart-plot { display: grid; grid-template-columns: repeat(var(--cols), 1fr); align-items: end; gap: 2px; height: 92px;
    padding: 6px 8px; border-radius: 12px; background: var(--bg-inset); border: 1px solid var(--border); }
  .ch-col { height: var(--h); min-height: 0; border-radius: 2px 2px 1px 1px; align-self: end;
    background: linear-gradient(180deg, oklch(0.82 0.15 var(--accent-h)), oklch(0.66 0.13 var(--accent-h)));
    transition: filter var(--dur-fast), transform var(--dur-fast); transform-origin: bottom; }
  .ch-col:hover { filter: brightness(1.18); transform: scaleY(1.03); }
  .ch-col.zero { height: 2px; background: color-mix(in oklab, var(--fg) 8%, transparent); border-radius: 2px; }

  /* ── Supporting stat strip ──────────────────────────────────────────────── */
  .strip { display: grid; grid-template-columns: repeat(4, 1fr); gap: 1px; padding: 1px; border-radius: 12px;
    background: var(--border); overflow: hidden; }
  .st { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 11px 13px; background: var(--bg-inset); }
  .st :global(svg) { color: var(--accent); opacity: 0.8; margin-bottom: 2px; }
  .st b { font-size: 17px; font-weight: 700; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1; }
  .st span { font-size: 10px; color: var(--fg-subtle); }

  /* ── Model mix — one stacked proportion bar + legend ────────────────────── */
  .mix-h { font-size: 10.5px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--fg-faint); margin-bottom: 9px; }
  .mix-sub { margin-left: 7px; font-weight: 600; letter-spacing: 0; text-transform: none; color: var(--fg-subtle); }
  .mix-bar { display: flex; gap: 2px; height: 14px; border-radius: 7px; overflow: hidden; }
  .mseg { min-width: 3px; border-radius: 2px; background: linear-gradient(180deg, oklch(0.78 0.15 var(--mh)), oklch(0.62 0.13 var(--mh)));
    transition: filter var(--dur-fast); }
  .mseg:hover { filter: brightness(1.15); }
  .mix-legend { display: flex; flex-wrap: wrap; gap: 6px 14px; margin-top: 10px; }
  .lg { display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--fg-2); }
  .lg i { width: 9px; height: 9px; border-radius: 3px; flex: none; background: oklch(0.72 0.14 var(--mh)); }
  .lg small { color: var(--fg-subtle); font-variant-numeric: tabular-nums; }

  /* ── Signature footer ───────────────────────────────────────────────────── */
  .sig { font-size: 12px; color: var(--fg-muted); padding-top: 16px; border-top: 1px solid var(--border);
    text-align: center; font-style: italic; }

  @media (prefers-reduced-motion: reduce) {
    :global(.stats-panel .spin) { animation: none; }
  }
</style>
