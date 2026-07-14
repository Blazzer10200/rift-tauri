<script lang="ts">
  // Pinned agent HUD — the periscope for running sub-agents / workflow / skill
  // spawns. The inline StreamAgent cards are the source of truth but scroll
  // away with the stream; while spawns are LIVE this bar keeps the fleet
  // glanceable at the top of the pane. Collapsed: count + the most recent
  // spawn's live "now-doing" line. Expanded: one row per spawn; clicking a row
  // jumps the transcript to its inline card.
  //
  // NOT a dock revival (cont.252 guard stands): no sub-transcripts render
  // here, nothing persists — rows only point back into the transcript.
  // Owner-approved 2026-07-08 ("agents get left behind" ask).
  //
  // Lifecycle mirrors PlanHud: visible only while a spawn is live during a
  // streaming turn (streaming = the honest liveness signal; the turn-end sweep
  // closes every spawn), then a brief linger showing the settled fleet.
  import { Bot, Check, ChevronDown, AlertCircle, Brain, Loader2 } from "lucide-svelte";
  import { fade } from "svelte/transition";
  import { agentNowLine } from "../toolCaption";
  import { fmtDur } from "./streamModel";
  import type { Block, TabState } from "$lib/state/assistant.svelte";

  // Svelte transitions don't respect prefers-reduced-motion on their own.
  const reduceMotion =
    typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  let { tab = null, streaming = false }:
    { tab?: TabState | null; streaming?: boolean } = $props();

  type Spawn = TabState["agentSpawns"][number];
  // Per-TURN fleet: agentSpawns accumulates across a conversation's turns
  // (capSpawns caps, nothing clears at beginTurn), so an unfiltered list read
  // "3 agents finished" when the current turn ran one (caught live 2026-07-08).
  // Settled spawns count only when they started within the current turn window
  // — but a RUNNING spawn is never stale (the turn-end sweep closes everything
  // at each terminal), so it's always in: a bg-agent continuation resets
  // turnStartedAt (maybeBeginContinuation) and would otherwise hide a spawn
  // that's still genuinely alive.
  const spawns = $derived.by((): Spawn[] => {
    const all = (tab?.agentSpawns ?? []) as Spawn[];
    const turnStart = tab?.activity.turnStartedAt ?? 0;
    return all.filter((s) => s.completedAt == null || s.startedAt >= turnStart);
  });
  const total = $derived(spawns.length);
  const running = $derived(spawns.filter((s) => s.completedAt == null));
  const done = $derived(total - running.length);
  const anyError = $derived(spawns.some((s) => s.isError));
  const allDone = $derived(total > 0 && running.length === 0);

  // Headline = the newest running spawn's live activity ("recon · Searching
  // for auth…"). Newest, not first: the most recently spawned agent is the one
  // the model just narrated, so it's the freshest context for the reader.
  const headline = $derived.by(() => {
    if (running.length === 0) return null;
    const s = running[running.length - 1];
    const now = agentNowLine((s.blocks ?? []) as Block[]);
    return `${s.subagentType} · ${now.label}`;
  });

  let open = $state(false);
  // Set by the IntersectionObserver effect below (declared here, above the
  // deriveds that read it — Svelte 5 TDZ, see the pinned $effect.pre gotcha).
  let cardsInView = $state(false);

  // sawLive/linger latch — same shape (and same reasons) as PlanHud: a
  // reopened convo must not flash a stale settled fleet, and the linger must
  // fire exactly once per completion.
  let sawLive = $state(false);
  let linger = $state(false);
  let lingerTimer: ReturnType<typeof setTimeout> | null = null;
  let lingered = false;

  let seenTab: TabState | null | undefined;
  $effect.pre(() => {
    if (tab === seenTab) return;
    seenTab = tab;
    sawLive = false;
    linger = false;
    lingered = false;
    open = false;
    if (lingerTimer) { clearTimeout(lingerTimer); lingerTimer = null; }
  });

  $effect.pre(() => {
    if (streaming && running.length > 0) {
      sawLive = true;
      linger = false;
      lingered = false;
      if (lingerTimer) { clearTimeout(lingerTimer); lingerTimer = null; }
    } else if (allDone && sawLive && !lingerTimer && !lingered) {
      linger = true;
      lingered = true;
      lingerTimer = setTimeout(() => { linger = false; lingerTimer = null; }, 4000);
    }
  });
  $effect(() => () => { if (lingerTimer) clearTimeout(lingerTimer); });

  const hudLive = $derived((streaming && running.length > 0) || linger);
  const visible = $derived(hudLive && !cardsInView);
  $effect(() => { if (!visible) open = false; });

  // 1s ticker for the per-row elapsed clocks while anything runs.
  let now = $state(0);
  $effect(() => {
    if (!visible || running.length === 0) return;
    now = Date.now();
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  function spawnSecs(s: Spawn): number | null {
    if (s.completedAt != null) return (s.completedAt - s.startedAt) / 1000;
    return now > 0 ? Math.max(0, (now - s.startedAt) / 1000) : null;
  }

  // Row click → jump the transcript to that spawn's inline card. Scoped to
  // this pane (multi-pane: ids repeat across panes when a convo is mirrored).
  let hudEl = $state<HTMLElement | null>(null);
  // Always-mounted zero-size handle for pane scoping — hudEl only exists while
  // the bar renders, and the visibility decision below must run BEFORE that.
  let sentinelEl = $state<HTMLElement | null>(null);
  function jumpTo(id: string) {
    const scope = hudEl?.closest(".csurf-col") ?? document;
    scope.querySelector(`#sacard-${CSS.escape(id)}`)
      ?.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  // Duplication guard (owner ask 2026-07-08): the HUD is an OVERFLOW
  // affordance, not a second copy — it only shows while the inline cards it
  // mirrors are OFF-screen. An IntersectionObserver on the fleet's card
  // anchors (running cards while live; the whole fleet during the linger)
  // flips cardsInView (declared with the state above); any watched card ≥15%
  // visible in this pane's scroll region suppresses the bar. Cards scroll
  // away → bar fades in; scroll back to them → bar yields.
  $effect(() => {
    const fleet = running.length > 0 ? running : spawns;
    const ids = fleet.map((s) => s.id);
    const scope = sentinelEl?.closest(".csurf-col");
    const rootEl = scope?.querySelector(".stream");
    if (!scope || !rootEl || ids.length === 0) {
      cardsInView = false;
      return;
    }
    const els = ids
      .map((id) => scope.querySelector(`#sacard-${CSS.escape(id)}`))
      .filter((el): el is Element => !!el);
    if (els.length === 0) {
      cardsInView = false;
      return;
    }
    const seen = new Set<Element>();
    // Boundary polish: yield IMMEDIATELY when a card comes into view, but
    // debounce the APPEAR — auto-scroll hovers cards right at the threshold,
    // and an undebounced bar flickered in/out at that edge. Pessimistic start
    // (true until the first IO callback) kills the one-frame flash on re-arm.
    let showTimer: ReturnType<typeof setTimeout> | null = null;
    cardsInView = true;
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) seen.add(e.target);
          else seen.delete(e.target);
        }
        if (seen.size > 0) {
          if (showTimer) { clearTimeout(showTimer); showTimer = null; }
          cardsInView = true;
        } else if (!showTimer) {
          showTimer = setTimeout(() => { cardsInView = false; showTimer = null; }, 350);
        }
      },
      { root: rootEl, threshold: 0.15 },
    );
    for (const el of els) io.observe(el);
    return () => {
      io.disconnect();
      if (showTimer) clearTimeout(showTimer);
    };
  });

  // Focus rescue on non-user-initiated hide — same contract as PlanHud.
  function rescueFocus() {
    const el = hudEl;
    if (!el || !el.contains(document.activeElement)) return;
    el.closest(".pane-shell")?.querySelector<HTMLElement>("textarea")?.focus();
  }
  $effect.pre(() => { if (!visible) rescueFocus(); });
  $effect(() => () => rescueFocus());
</script>

<span class="ahud-sentinel" bind:this={sentinelEl} aria-hidden="true"></span>
{#if visible}
  <div class="ahud" class:complete={allDone} class:open bind:this={hudEl}
    out:fade={{ duration: reduceMotion ? 0 : 160 }}>
    <button
      class="ahud-bar"
      type="button"
      aria-expanded={open}
      aria-label={open ? "Collapse agents" : "Expand agents"}
      onclick={() => (open = !open)}
    >
      <span class="ahud-ic" aria-hidden="true">
        {#if allDone}
          {#if anyError}<AlertCircle size={13} />{:else}<Check size={13} strokeWidth={2.5} />{/if}
        {:else}
          <Bot size={13} strokeWidth={2} />
        {/if}
      </span>
      <span class="ahud-text">
        {#if allDone}
          {total} agent{total === 1 ? "" : "s"} finished
        {:else}
          {running.length} agent{running.length === 1 ? "" : "s"}{headline ? ` · ${headline}` : ""}
        {/if}
      </span>
      <span class="ahud-count">{done}/{total}</span>
      <span class="ahud-chev" aria-hidden="true"><ChevronDown size={12} /></span>
    </button>
    {#if open}
      <ul class="ahud-list">
        {#each spawns as s (s.id)}
          {@const secs = spawnSecs(s)}
          {@const live = s.completedAt == null}
          {@const nowL = live ? agentNowLine((s.blocks ?? []) as Block[]) : null}
          <li>
            <button class="ahud-row" type="button" onclick={() => jumpTo(s.id)}
              title="Jump to this agent in the conversation">
              <span class="ahud-mark" aria-hidden="true">
                {#if live}<Loader2 size={12} class="ahud-spin" />
                {:else if s.isError}<AlertCircle size={12} />
                {:else}<Check size={12} strokeWidth={2.5} />{/if}
              </span>
              <span class="ahud-pill">{s.subagentType}</span>
              <span class="ahud-desc" class:settled={!live}>{s.description}</span>
              {#if nowL}
                <span class="ahud-now">
                  {#if nowL.thinking}<Brain size={11} />{/if}
                  {nowL.label}
                </span>
              {/if}
              {#if secs != null}<span class="ahud-dur">{fmtDur(secs)}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .ahud-sentinel { display: none; }

  /* Same glassy chrome family as .phud — this renders inside the shared
     .hud-stack (AssistantPane), which owns centering/width/stacking. */
  .ahud {
    width: 100%;
    border-radius: 12px;
    background: color-mix(in oklch, var(--surface) 84%, transparent);
    backdrop-filter: blur(16px) saturate(140%);
    -webkit-backdrop-filter: blur(16px) saturate(140%);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-float);
    overflow: hidden;
    animation: ahud-in 260ms cubic-bezier(0.22, 1, 0.36, 1) both;
    transition: border-color 200ms ease-out;
    pointer-events: auto;
  }
  @keyframes ahud-in {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: none; }
  }
  .ahud.complete { border-color: color-mix(in oklab, var(--ok) 45%, var(--border-strong)); }

  .ahud-bar {
    display: flex; align-items: center; gap: 9px;
    width: 100%; height: 32px; padding: 0 12px 0 11px;
    background: none; border: 0; cursor: pointer;
    font-size: 12px; color: var(--fg-2); text-align: left;
  }
  .ahud-bar:hover { background: color-mix(in oklab, var(--fg) 4%, transparent); }
  .ahud-ic {
    display: grid; place-items: center;
    width: 16px; height: 16px; flex: none;
    color: var(--accent-hover);
  }
  .ahud.complete .ahud-ic { color: var(--ok); }
  .ahud-text {
    flex: 1; min-width: 0;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    font-weight: 500; letter-spacing: -0.004em;
  }
  .ahud.complete .ahud-text { color: var(--ok); font-weight: 600; }
  .ahud-count {
    flex: none;
    font-family: var(--font-mono, monospace); font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-subtle);
  }
  .ahud-chev { display: grid; place-items: center; flex: none; color: var(--fg-faint);
    transition: transform var(--dur-fast); }
  .ahud.open .ahud-chev { transform: rotate(180deg); }

  .ahud-list {
    margin: 0; padding: 5px 6px 7px;
    list-style: none;
    display: flex; flex-direction: column; gap: 1px;
    border-top: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    max-height: 38vh; overflow-y: auto;
    scrollbar-width: none;
    animation: ahud-open 200ms var(--ease-page) both;
  }
  .ahud-list::-webkit-scrollbar { width: 0; height: 0; display: none; }
  @keyframes ahud-open { from { opacity: 0; transform: translateY(-3px); } to { opacity: 1; transform: none; } }

  .ahud-row {
    display: flex; align-items: center; gap: 8px;
    width: 100%; padding: 4px 6px;
    background: none; border: 0; border-radius: 7px; cursor: pointer;
    font-size: 12px; color: var(--fg-muted); text-align: left;
    transition: background var(--dur-fast);
  }
  .ahud-row:hover { background: color-mix(in oklab, var(--fg) 5%, transparent); }
  .ahud-mark { width: 14px; height: 14px; display: grid; place-items: center; flex: none; color: var(--status-busy); }
  .ahud-row .ahud-mark :global(.ahud-spin) { animation: ringspin 1s linear infinite; }
  li:has(.ahud-desc.settled) .ahud-mark { color: var(--ok); }
  .ahud-pill {
    display: inline-flex; align-items: center; padding: 1px 7px; border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 16%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 32%, var(--border));
    color: var(--accent-hover); font-size: 10px; font-weight: 600; letter-spacing: 0.02em;
    font-family: var(--font-mono); flex: none;
  }
  .ahud-desc {
    flex: none; min-width: 0; max-width: 34%;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: var(--fg-2);
  }
  .ahud-desc.settled { color: var(--fg-faint); }
  .ahud-now {
    flex: 1; min-width: 0;
    display: inline-flex; align-items: center; gap: 5px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: var(--fg-subtle); font-size: 11.5px;
  }
  .ahud-now :global(svg) { flex: none; color: var(--status-busy); }
  .ahud-dur {
    flex: none; margin-left: auto;
    font-family: var(--font-mono); font-size: 10px;
    font-variant-numeric: tabular-nums; color: var(--fg-faint);
  }

  @media (prefers-reduced-motion: reduce) {
    .ahud, .ahud-list { animation: none; }
    .ahud-row .ahud-mark :global(.ahud-spin) { animation: none; }
  }
</style>
