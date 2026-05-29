<script lang="ts">
  // Activity dashboard — the live "what is Claude doing" surface for a tab.
  // Deliberately NOT a re-list of tool rows (the transcript already shows
  // those). It surfaces what the transcript can't: live running items,
  // session aggregates, a tool-mix histogram, and perf insights. Everything
  // here is derived from reactive per-tab state (messages[].blocks +
  // agentSpawns + tab fields); tok/s is the one session-global metric, pulled
  // from the telemetry engine and refreshed by the 1s ticker.
  import { onMount, onDestroy } from "svelte";
  import { Loader2, Terminal, Bot, AlertCircle, Gauge, Wrench, CircleDollarSign, Activity } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import type { Block, ChatMessage } from "../../state/assistant.svelte";
  import { liveActivity } from "../../state/assistant/helpers";
  import { tooltip } from "$lib/actions/tooltip";

  let { tabId = null }: { tabId?: string | null } = $props();

  const tab = $derived(tabId == null ? assistant.activeTab : assistant.tabFor(tabId));
  const messages = $derived<ChatMessage[]>(tab?.messages ?? []);

  // 1s ticker — drives live elapsed readouts + tok/s refresh. Mounts only
  // while the panel is open, so it costs nothing when hidden.
  let now = $state(Date.now());
  let ticker: ReturnType<typeof setInterval> | null = null;
  onMount(() => { ticker = setInterval(() => { now = Date.now(); }, 1000); });
  onDestroy(() => { if (ticker) clearInterval(ticker); });

  // CR4: a constant mount-time stamp for liveActivity's `fallbackTs`. It only
  // stands in for legacy shell blocks missing a `startedAt`, so it never needs
  // to track the ticker — feeding `now` here re-ran liveActivity (a re-sort +
  // fresh array alloc) every second even with nothing in flight. Pinning it to
  // mount decouples `running` from the ticker; it now recomputes only when
  // messages / agentSpawns actually change.
  const mountTs = Date.now();

  // ── Running: live shells (pending Bash) + live agents (no completedAt) ──
  // Shared with the composer live pills via `liveActivity` so the two surfaces
  // can never disagree on what's in flight.
  const running = $derived(liveActivity(messages, tab?.agentSpawns ?? [], mountTs));

  // ── Tool rollup — counts, errors, slowest — all per-tab, reactive ──────
  const toolStats = $derived.by(() => {
    const counts: Record<string, number> = {};
    let total = 0, errors = 0, cancelled = 0;
    let slowest: { name: string; ms: number; id: string } | null = null;
    let lastFail: string | null = null;
    let firstTs = 0, lastTs = 0;
    const stamps: number[] = [];
    for (const m of messages) {
      for (const b of m.blocks as Block[]) {
        if (b.type !== "tool") continue;
        counts[b.name] = (counts[b.name] ?? 0) + 1;
        total += 1;
        // Parallel tool calls aborted by an earlier failure come back as
        // `<tool_use_error>Cancelled: …` — those aren't real failures, so
        // bucket them separately rather than inflating the error count.
        const isCancelled = b.result != null && b.result.includes("Cancelled:");
        if (isCancelled) cancelled += 1;
        else if (b.isError || b.status === "error") { errors += 1; lastFail = b.name; }
        if (b.durationMs != null && (!slowest || b.durationMs > slowest.ms)) slowest = { name: b.name, ms: b.durationMs, id: b.id };
        if (b.startedAt != null) {
          stamps.push(b.startedAt);
          if (firstTs === 0 || b.startedAt < firstTs) firstTs = b.startedAt;
          if (b.startedAt > lastTs) lastTs = b.startedAt;
        }
      }
    }
    const histo = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    const max = histo.length ? histo[0][1] : 1;
    return { counts, total, errors, cancelled, slowest, lastFail, histo, max, stamps, firstTs, lastTs };
  });

  // Scroll the transcript to a tool block + briefly flash it. Anchored on the
  // `actnode-<id>` ids MessageBubble sets per tool node. No-op if the node is
  // off-screen in a different tab / already unmounted.
  function jumpTo(blockId: string) {
    const el = document.getElementById(`actnode-${blockId}`);
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    el.classList.add("act-flash");
    setTimeout(() => el.classList.remove("act-flash"), 1100);
  }

  // 12-bucket activity sparkline over the tool-call timespan.
  const spark = $derived.by(() => {
    const { stamps, firstTs, lastTs } = toolStats;
    if (stamps.length < 2 || lastTs <= firstTs) return [] as number[];
    const N = 12;
    const span = lastTs - firstTs;
    const bins = new Array(N).fill(0);
    for (const s of stamps) {
      const idx = Math.min(N - 1, Math.floor(((s - firstTs) / span) * N));
      bins[idx] += 1;
    }
    const peak = Math.max(...bins, 1);
    return bins.map((b) => Math.round((b / peak) * 100));
  });

  const cost = $derived(tab?.totalCostUsd ?? null);
  // tok/s is session-global (needs stream timing the per-tab blocks don't
  // carry). Recompute each tick via the ticker by referencing `now`.
  const tokPerSec = $derived.by(() => {
    void now;
    return assistant.telemetry.snapshot().summary.outputTokensPerSec;
  });

  const isEmpty = $derived(running.length === 0 && toolStats.total === 0);

  function fmtElapsed(ms: number): string {
    const s = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }
  function fmtDur(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const s = ms / 1000;
    return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m ${String(Math.floor(s % 60)).padStart(2, "0")}s`;
  }
</script>

<div class="activity">
  {#if isEmpty}
    <div class="empty-note">
      <div class="empty-title">Activity</div>
      Live shells, agents, tool mix, and performance for this conversation will
      appear here once Claude gets to work.
    </div>
  {:else}
    <!-- Running ──────────────────────────────────────────────────────── -->
    {#if running.length > 0}
      <section class="sect">
        <header class="sect-head">
          <Activity size={12} />
          <span class="sect-title">Running</span>
          <span class="badge live"><span class="live-dot"></span>{running.length}</span>
        </header>
        <ul class="rows">
          {#each running as r (r.id)}
            <li>
              <button
                type="button"
                class="run"
                onclick={() => jumpTo(r.id)}
                use:tooltip={"Jump to this call in the transcript"}
              >
                <span class="run-ic"><Loader2 size={13} class="mon-spin" /></span>
                <span class="run-label" class:mono={r.kind === "shell"}>
                  {#if r.sub}<span class="agtype">{r.sub}</span>{/if}<span class="run-t">{r.label}</span>
                </span>
                <span class="run-el mono">{fmtElapsed(now - r.startedAt)}</span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- This session — stat cards + sparkline ──────────────────────────── -->
    <section class="sect">
      <header class="sect-head"><Gauge size={12} /><span class="sect-title">This session</span></header>
      <div class="stats">
        <div class="stat"><div class="v acc">{tokPerSec ?? "—"}</div><div class="k">tok / sec</div></div>
        <div class="stat"><div class="v">{toolStats.total}</div><div class="k">tools</div></div>
        <div class="stat"><div class="v ok">{cost != null ? `$${cost.toFixed(2)}` : "—"}</div><div class="k">cost</div></div>
      </div>
      {#if spark.length > 0}
        <div class="spark" aria-hidden="true">
          {#each spark as h, i (i)}
            <i class:hot={h >= 80} style="height: {Math.max(6, h)}%"></i>
          {/each}
        </div>
      {/if}
    </section>

    <!-- Tool mix histogram ─────────────────────────────────────────────── -->
    {#if toolStats.histo.length > 0}
      <section class="sect">
        <header class="sect-head"><Wrench size={12} /><span class="sect-title">Tool mix</span></header>
        <div class="histo">
          {#each toolStats.histo.slice(0, 6) as [name, count] (name)}
            <div class="hrow" use:tooltip={name}>
              <span class="hname">{name}</span>
              <span class="hbar"><i style="width: {(count / toolStats.max) * 100}%"></i></span>
              <span class="hn mono">{count}</span>
            </div>
          {/each}
          {#if toolStats.histo.length > 6}
            <div class="hmore">+{toolStats.histo.length - 6} more tool{toolStats.histo.length - 6 === 1 ? "" : "s"}</div>
          {/if}
        </div>
      </section>
    {/if}

    <!-- Insights ───────────────────────────────────────────────────────── -->
    {#if toolStats.slowest || toolStats.errors > 0 || toolStats.cancelled > 0}
      <section class="sect insights">
        {#if toolStats.slowest}
          <button
            type="button"
            class="insight warn jump"
            onclick={() => { const s = toolStats.slowest; if (s) jumpTo(s.id); }}
            use:tooltip={"Jump to this call in the transcript"}
          >
            <span class="ic"><AlertCircle size={13} /></span>
            Slowest tool · <b>{toolStats.slowest.name}</b> <span class="mono">{fmtDur(toolStats.slowest.ms)}</span>
          </button>
        {/if}
        {#if toolStats.errors > 0}
          <div class="insight err">
            <span class="ic"><AlertCircle size={13} /></span>
            {toolStats.errors} failed call{toolStats.errors === 1 ? "" : "s"}{#if toolStats.lastFail} · <span class="mono">{toolStats.lastFail}</span>{/if}
          </div>
        {/if}
        {#if toolStats.cancelled > 0}
          <div class="insight">
            <span class="ic"><AlertCircle size={13} /></span>
            {toolStats.cancelled} cancelled <span class="dim">· aborted parallel calls</span>
          </div>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .activity {
    width: 100%; flex: 1;
    display: flex; flex-direction: column;
    min-height: 0; overflow-x: hidden; overflow-y: auto;
    box-sizing: border-box;
  }
  .activity::-webkit-scrollbar { width: 8px; height: 0; }
  .activity::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 4px; }
  .activity::-webkit-scrollbar-thumb:hover { background: var(--fg-faint); }

  .sect { display: flex; flex-direction: column; border-bottom: 1px solid var(--border); }
  .sect:last-of-type { border-bottom: none; }
  .sect-head {
    display: flex; align-items: center; gap: 8px;
    padding: 11px 14px 8px;
    color: var(--fg); font-size: var(--fs-sm); font-weight: 600; flex-shrink: 0;
  }
  .sect-head :global(svg) { color: var(--accent); flex-shrink: 0; }
  .sect-title { color: var(--fg); }
  .badge {
    margin-left: auto; font-size: 10px; padding: 2px 7px;
    background: var(--accent-soft); color: var(--accent);
    border-radius: 999px; font-variant-numeric: tabular-nums; font-weight: 650;
  }
  .badge.live { display: inline-flex; align-items: center; gap: 5px; }

  /* Running rows */
  .rows { list-style: none; margin: 0; padding: 4px 8px 12px; display: flex; flex-direction: column; gap: 2px; }
  .run {
    display: flex; align-items: center; gap: 9px;
    width: 100%; padding: 7px 8px; border-radius: 6px;
    background: none; border: 0; text-align: left; font: inherit; cursor: pointer;
    font-size: var(--fs-sm); color: var(--fg-2);
    transition: background 120ms ease;
  }
  .run:hover { background: var(--bg-elev-2); }
  .run:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .run-ic { display: flex; align-items: center; color: var(--accent); flex-shrink: 0; }
  .run-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .run-label.mono .run-t { font-family: var(--font-mono); font-size: 12px; color: var(--fg); }
  .agtype { color: var(--accent); font-family: var(--font-mono); font-size: var(--fs-xs); margin-right: 6px; }
  .run-el { flex-shrink: 0; font-size: 10px; color: var(--fg-faint); font-variant-numeric: tabular-nums; }

  /* Stat cards */
  .stats { display: flex; gap: 8px; padding: 8px 14px 4px; }
  .stat { flex: 1; background: var(--bg-elev-2); border: 1px solid var(--border); border-radius: var(--radius); padding: 8px 10px; }
  .stat .v { font-size: var(--fs-lg); font-weight: 650; font-variant-numeric: tabular-nums; color: var(--fg); }
  .stat .v.acc { color: var(--accent); }
  .stat .v.ok { color: var(--ok); }
  .stat .k { font-size: 10px; color: var(--fg-subtle); text-transform: uppercase; letter-spacing: 0.04em; margin-top: 2px; }

  /* Sparkline */
  .spark { display: flex; align-items: flex-end; gap: 3px; height: 34px; padding: 6px 14px 14px; }
  .spark i { flex: 1; background: var(--accent-soft); border-radius: 2px 2px 0 0; min-height: 2px; transition: height 280ms cubic-bezier(0.22,1,0.36,1); }
  .spark i.hot { background: var(--accent); }

  /* Histogram */
  .histo { padding: 4px 14px 14px; display: flex; flex-direction: column; gap: 7px; }
  .hrow { display: flex; align-items: center; gap: 9px; font-size: var(--fs-sm); }
  .hname { width: 62px; flex-shrink: 0; color: var(--fg-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hmore { padding-top: 2px; font-size: var(--fs-xs); color: var(--fg-subtle); }
  .hbar { flex: 1; height: 7px; background: var(--bg-elev-2); border-radius: 4px; overflow: hidden; }
  .hbar i { display: block; height: 100%; background: var(--accent); border-radius: 4px; transition: width 280ms cubic-bezier(0.22,1,0.36,1); }
  .hn { width: 18px; text-align: right; color: var(--fg-muted); font-variant-numeric: tabular-nums; }

  /* Insights */
  .insights { padding-bottom: 6px; }
  .insight { display: flex; align-items: center; gap: 8px; padding: 9px 14px; font-size: var(--fs-sm); color: var(--fg-muted); }
  .insight .ic { display: flex; align-items: center; flex-shrink: 0; }
  .insight b { color: var(--fg-2); font-weight: 600; }
  .insight.warn .ic { color: var(--warn); }
  .insight.err .ic { color: var(--danger); }
  .insight .dim { color: var(--fg-subtle); }
  /* Slowest-tool row doubles as a jump button. */
  .insight.jump {
    width: 100%; background: none; border: 0; font: inherit; text-align: left;
    cursor: pointer; transition: background 120ms ease;
  }
  .insight.jump:hover { background: var(--bg-elev-2); }
  .insight.jump:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  /* Transcript flash when a row jumps to its call. Global — the target node
     (.tl-node) lives in MessageBubble's scope, not here. */
  :global(.tl-node.act-flash) {
    animation: act-flash 1.1s cubic-bezier(0.22, 1, 0.36, 1) both;
    border-radius: 8px;
  }
  @keyframes act-flash {
    0%   { box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 70%, transparent); background: color-mix(in oklch, var(--accent) 14%, transparent); }
    100% { box-shadow: 0 0 0 2px transparent; background: transparent; }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.tl-node.act-flash) { animation: none; }
  }

  .empty-note { color: var(--fg-subtle); font-size: var(--fs-xs); line-height: 1.55; padding: 14px 16px; }
  .empty-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); margin-bottom: 4px; }

  .mono { font-family: var(--font-mono, monospace); }
  .live-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); animation: mon-live-pulse 1.4s ease-in-out infinite; }
  .activity :global(.mon-spin) { animation: mon-spin 0.9s linear infinite; }
  @keyframes mon-spin { to { transform: rotate(360deg); } }
  @keyframes mon-live-pulse { 0%,100% { opacity: 0.4; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) {
    .live-dot, .activity :global(.mon-spin) { animation: none; }
  }
</style>
