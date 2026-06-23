<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { X, Loader2, Flame, Clock, Cpu, MessageSquare, Wrench, DollarSign, CalendarDays } from "lucide-svelte";
  import { fade, scale } from "svelte/transition";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import {
    type ConvoStat, type StatRange,
    filterRange, summarize, streaks, peakHour, perModel, topModel,
    heatmap, intensity, funFact,
    fmtInt, fmtCompact, fmtCost, hourLabel,
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
  const hm = $derived(heatmap(stats, 84, now)); // 12 weeks
  const fact = $derived(funFact(totals));
  const empty = $derived(!loading && !error && raw.length === 0);

  function onKey(e: KeyboardEvent) { if (e.key === "Escape") onclose(); }
</script>

<svelte:window onkeydown={onKey} />

<div class="stats-backdrop" use:portal transition:fade={{ duration: 140 }}>
  <button class="sb-dismiss" type="button" aria-label="Close stats" onclick={onclose}></button>
  <div class="stats-panel" role="dialog" aria-label="Your activity" transition:scale={{ duration: 180, start: 0.97 }}>
    <header class="sp-head">
      <div class="sp-title"><CalendarDays size={16} /><span>Your activity</span></div>
      <div class="sp-range" role="group" aria-label="Time range">
        <button class:on={range === "all"} type="button" onclick={() => (range = "all")}>All</button>
        <button class:on={range === "30d"} type="button" onclick={() => (range = "30d")}>30d</button>
        <button class:on={range === "7d"} type="button" onclick={() => (range = "7d")}>7d</button>
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
        <!-- headline stat cards -->
        <div class="cards">
          <div class="card"><span class="c-ic"><MessageSquare size={14} /></span><span class="c-v">{fmtInt(totals.messages)}</span><span class="c-l">messages</span></div>
          <div class="card"><span class="c-ic"><Cpu size={14} /></span><span class="c-v">{fmtInt(totals.sessions)}</span><span class="c-l">sessions</span></div>
          <div class="card"><span class="c-ic"><Wrench size={14} /></span><span class="c-v">{fmtCompact(totals.toolCalls)}</span><span class="c-l">tool calls</span></div>
          <div class="card"><span class="c-ic"><DollarSign size={14} /></span><span class="c-v">{fmtCost(totals.cost)}</span><span class="c-l">spent</span></div>
          <div class="card"><span class="c-ic"><Flame size={14} /></span><span class="c-v">{strk.current}<span class="c-sub">d</span></span><span class="c-l">streak · best {strk.longest}d</span></div>
          <div class="card"><span class="c-ic"><Clock size={14} /></span><span class="c-v">{hourLabel(peak)}</span><span class="c-l">peak hour</span></div>
        </div>

        <!-- activity heatmap -->
        <section class="block">
          <div class="b-h">Activity · last 12 weeks</div>
          <div class="heat">
            {#each hm.cells as c, i (c.day)}
              <span
                class="hc"
                data-lvl={intensity(c.messages, hm.max)}
                style={i === 0 ? `grid-row:${hm.leadPad + 1}` : ""}
                use:tooltip={`${fmtInt(c.messages)} msg · ${c.sessions} session${c.sessions === 1 ? "" : "s"}`}
                aria-hidden="true"
              ></span>
            {/each}
          </div>
        </section>

        <!-- per-model split -->
        {#if models.length}
          <section class="block">
            <div class="b-h">By model{#if top}<span class="b-sub">· mostly {top}</span>{/if}</div>
            <div class="bars">
              {#each models as m (m.model)}
                <div class="bar-row">
                  <span class="bar-name" title={m.label}>{m.label}</span>
                  <span class="bar-track"><i style="width:{Math.max(2, m.share * 100)}%"></i></span>
                  <span class="bar-val">{fmtInt(m.messages)}</span>
                </div>
              {/each}
            </div>
          </section>
        {/if}

        {#if fact}
          <div class="fact">{fact}</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  :global(.stats-backdrop) { position: fixed; inset: 0; z-index: 60; display: grid; place-items: center;
    background: oklch(0 0 0 / 0.5); -webkit-backdrop-filter: blur(3px); backdrop-filter: blur(3px); }
  .sb-dismiss { position: absolute; inset: 0; background: none; cursor: default; }
  :global(.stats-panel) { position: relative; width: min(680px, calc(100vw - 40px)); max-height: min(82vh, 720px);
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

  .sp-body { padding: 16px; overflow-y: auto; display: flex; flex-direction: column; gap: 18px;
    scrollbar-width: thin; scrollbar-color: var(--border-strong) transparent; }

  .cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
  .card { display: flex; flex-direction: column; gap: 3px; padding: 12px 13px; border-radius: 12px;
    background: var(--bg-inset); border: 1px solid var(--border); }
  .c-ic { display: inline-flex; color: var(--accent); opacity: 0.85; }
  .c-v { font-size: 19px; font-weight: 700; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1.1; }
  .c-sub { font-size: 12px; font-weight: 600; color: var(--fg-subtle); margin-left: 1px; }
  .c-l { font-size: 10.5px; color: var(--fg-subtle); }

  .block .b-h { font-size: 10.5px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--fg-faint); margin-bottom: 9px; }
  .b-sub { margin-left: 7px; font-weight: 600; letter-spacing: 0; text-transform: none; color: var(--fg-subtle); }

  /* heatmap — 7 rows (weekdays), columns flow left→right by week */
  .heat { display: grid; grid-auto-flow: column; grid-template-rows: repeat(7, 1fr); gap: 3px; }
  .hc { width: 100%; aspect-ratio: 1; min-width: 9px; border-radius: 3px; background: var(--bg-inset); }
  .hc[data-lvl="1"] { background: color-mix(in oklab, var(--accent) 26%, var(--bg-inset)); }
  .hc[data-lvl="2"] { background: color-mix(in oklab, var(--accent) 48%, var(--bg-inset)); }
  .hc[data-lvl="3"] { background: color-mix(in oklab, var(--accent) 70%, var(--bg-inset)); }
  .hc[data-lvl="4"] { background: var(--accent); }

  .bars { display: flex; flex-direction: column; gap: 8px; }
  .bar-row { display: grid; grid-template-columns: 92px 1fr 48px; align-items: center; gap: 10px; }
  .bar-name { font-size: 11.5px; color: var(--fg-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .bar-track { height: 8px; border-radius: 999px; background: var(--bg-inset); overflow: hidden; }
  .bar-track > i { display: block; height: 100%; border-radius: 999px;
    background: linear-gradient(90deg, oklch(0.62 0.15 var(--accent-h)), oklch(0.80 0.16 var(--accent-h))); }
  .bar-val { font-size: 11px; color: var(--fg-subtle); text-align: right; font-variant-numeric: tabular-nums; }

  .fact { font-size: 12px; color: var(--fg-muted); padding: 11px 13px; border-radius: 11px;
    background: color-mix(in oklab, var(--accent) 7%, var(--bg-inset)); border: 1px solid var(--border); }

  @media (prefers-reduced-motion: reduce) {
    :global(.stats-panel .spin) { animation: none; }
  }
</style>
