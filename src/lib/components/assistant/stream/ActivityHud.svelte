<script lang="ts">
  // Pinned activity HUD (né AgentHud) — the periscope for running sub-agents /
  // workflow / skill spawns AND the live shell processes under this session's
  // CLI child. The inline StreamAgent cards are the source of truth for agents
  // but scroll away with the stream; while anything is LIVE this bar keeps the
  // fleet glanceable at the top of the pane. Collapsed: counts + the most
  // recent spawn's live "now-doing" line. Expanded: one row per spawn (click
  // jumps to its inline card) + one row per shell (hover reveals a per-PID
  // kill — the ONE thing that CAN die individually; agents can't, so the only
  // agent-scoped control is the honest whole-turn Stop in the bar).
  //
  // NOT a dock revival (cont.252 guard stands): no sub-transcripts render
  // here, nothing persists — rows only point back into the transcript.
  // Owner-approved 2026-07-08 ("agents get left behind" ask).
  //
  // Lifecycle mirrors PlanHud: visible only while a spawn/shell is live during
  // a streaming turn (streaming = the honest liveness signal; the turn-end
  // sweep closes every spawn + clears shellRows), then a brief linger showing
  // the settled fleet.
  import { Bot, Check, ChevronDown, AlertCircle, Brain, Loader2, Square, Terminal, X } from "lucide-svelte";
  import { fade } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { agentNowLine } from "../toolCaption";
  import { fmtDur, shellLabel, trimCmd } from "./streamModel";
  import { assistant, type Block, type TabState } from "$lib/state/assistant.svelte";
  import type { ShellRow } from "$lib/state/assistant/listeners";

  // Svelte transitions don't respect prefers-reduced-motion on their own.
  const reduceMotion =
    typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  let { tab = null, tabId = null, streaming = false }:
    { tab?: TabState | null; tabId?: string | null; streaming?: boolean } = $props();

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

  // Live shell processes under the CLI child (backend poller, cleared on every
  // turn terminal). No linger of their own: rows vanish when the process does.
  const shells = $derived(tab?.shellRows ?? []);
  const anyLive = $derived(running.length > 0 || shells.length > 0);

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

  const hudLive = $derived((streaming && anyLive) || linger);
  const visible = $derived(hudLive && !cardsInView);
  $effect(() => { if (!visible) open = false; });

  // 1s ticker for the per-row elapsed clocks while anything runs.
  let now = $state(0);
  $effect(() => {
    if (!visible || !anyLive) return;
    now = Date.now();
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  function spawnSecs(s: Spawn): number | null {
    if (s.completedAt != null) return (s.completedAt - s.startedAt) / 1000;
    return now > 0 ? Math.max(0, (now - s.startedAt) / 1000) : null;
  }
  // sysinfo start_time is epoch SECONDS (coarser than the spawn clocks above).
  function shellSecs(sh: ShellRow): number | null {
    return now > 0 ? Math.max(0, now / 1000 - sh.started_at) : null;
  }

  // Per-PID kill — backend re-verifies the PID is a live descendant of this
  // session's CLI child before killing (a dead/foreign PID is a safe no-op).
  const killing = new Set<number>();
  async function killShell(pid: number) {
    const sid = tab?.cliSessionId;
    if (!sid || killing.has(pid)) return;
    killing.add(pid);
    try {
      await invoke("assistant_kill_shell", { sessionId: sid, pid });
    } catch (e) {
      console.warn("assistant_kill_shell failed", e);
    } finally {
      killing.delete(pid);
    }
  }

  // Whole-turn stop — the only honest agent-scoped control (individual
  // sub-agents live inside the CLI process and can't be killed one by one).
  function stopTurn(e: MouseEvent) {
    e.stopPropagation();
    void assistant.stop(tabId ?? undefined);
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
  <div class="ahud" class:complete={allDone && shells.length === 0} class:open bind:this={hudEl}
    out:fade={{ duration: reduceMotion ? 0 : 160 }}>
    <div class="ahud-bar">
      <button
        class="ahud-toggle"
        type="button"
        aria-expanded={open}
        aria-label={open ? "Collapse activity" : "Expand activity"}
        onclick={() => (open = !open)}
      >
        <span class="ahud-ic" aria-hidden="true">
          {#if !anyLive && allDone}
            {#if anyError}<AlertCircle size={13} />{:else}<Check size={13} strokeWidth={2.5} />{/if}
          {:else if running.length > 0}
            <Bot size={13} strokeWidth={2} />
          {:else}
            <Terminal size={13} strokeWidth={2} />
          {/if}
        </span>
        <span class="ahud-text">
          {#if running.length > 0}
            {running.length} agent{running.length === 1 ? "" : "s"}{headline ? ` · ${headline}` : ""}{shells.length > 0 ? ` · ${shells.length} shell${shells.length === 1 ? "" : "s"}` : ""}
          {:else if shells.length > 0}
            {shells.length} shell{shells.length === 1 ? "" : "s"} running · {trimCmd(shellLabel(shells[shells.length - 1].cmd), 48)}
          {:else}
            {total} agent{total === 1 ? "" : "s"} finished
          {/if}
        </span>
        {#if total > 0}<span class="ahud-count">{done}/{total}</span>{/if}
        <span class="ahud-chev" aria-hidden="true"><ChevronDown size={12} /></span>
      </button>
      {#if streaming && anyLive}
        <button class="ahud-stop" type="button" onclick={stopTurn}
          title="Stop this turn — ends every agent and shell it started">
          <Square size={9} strokeWidth={2.5} />
          Stop
        </button>
      {/if}
    </div>
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
        {#each shells as sh (sh.pid)}
          {@const secs = shellSecs(sh)}
          <li>
            <div class="ahud-row ahud-shell">
              <span class="ahud-mark" aria-hidden="true"><Terminal size={12} /></span>
              <span class="ahud-pill ahud-pid">PID {sh.pid}</span>
              <span class="ahud-cmd" title={sh.cmd}>{trimCmd(shellLabel(sh.cmd), 90)}</span>
              {#if secs != null}<span class="ahud-dur">{fmtDur(secs)}</span>{/if}
              <button class="ahud-kill" type="button" onclick={() => killShell(sh.pid)}
                title="Kill this process" aria-label={`Kill PID ${sh.pid}`}>
                <X size={11} strokeWidth={2.5} />
              </button>
            </div>
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
    position: relative;
    width: 100%;
    border-radius: 12px;
    background: color-mix(in oklch, var(--surface) 94%, transparent);
    backdrop-filter: blur(20px) saturate(150%);
    -webkit-backdrop-filter: blur(20px) saturate(150%);
    border: 1px solid var(--border-strong);
    /* Layered depth + top catch-light — matches .phud (shared chrome family). */
    box-shadow:
      inset 0 1px 0 color-mix(in oklab, var(--fg) 6%, transparent),
      0 12px 32px -10px rgb(0 0 0 / 0.55),
      var(--shadow-float);
    overflow: hidden;
    animation: ahud-in var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1) both;
    transition: border-color var(--dur-base) ease-out;
    pointer-events: auto;
  }
  .ahud::before {
    content: ""; position: absolute; top: 0; left: 14%; right: 14%; height: 1px;
    background: linear-gradient(90deg, transparent,
      color-mix(in oklab, var(--accent) 45%, transparent), transparent);
    pointer-events: none;
  }
  .ahud.complete::before {
    background: linear-gradient(90deg, transparent,
      color-mix(in oklab, var(--ok) 50%, transparent), transparent);
  }
  @keyframes ahud-in {
    from { opacity: 0; transform: translateY(-10px) scale(0.96); }
    to { opacity: 1; transform: none; }
  }
  .ahud.complete { border-color: color-mix(in oklab, var(--ok) 45%, var(--border-strong)); }

  .ahud-bar { display: flex; align-items: stretch; width: 100%; height: 32px; }
  .ahud-toggle {
    flex: 1; min-width: 0;
    display: flex; align-items: center; gap: 9px;
    padding: 0 12px 0 11px;
    background: none; border: 0; cursor: pointer;
    font-size: 12px; color: var(--fg-2); text-align: left;
  }
  .ahud-toggle:hover { background: color-mix(in oklab, var(--fg) 4%, transparent); }
  /* Whole-turn stop — quiet until hovered, then honest danger. */
  .ahud-stop {
    flex: none;
    display: inline-flex; align-items: center; gap: 5px;
    padding: 0 11px;
    background: none; border: 0; cursor: pointer;
    border-left: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    font-size: 11px; font-weight: 600; color: var(--fg-subtle);
    transition: color var(--dur-fast), background var(--dur-fast);
  }
  .ahud-stop:hover {
    color: var(--danger);
    background: color-mix(in oklab, var(--danger) 8%, transparent);
  }
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
    animation: ahud-open var(--dur-base) var(--ease-page) both;
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

  /* Shell rows — same rhythm as agent rows; div (not button: the kill nests). */
  .ahud-shell { cursor: default; }
  .ahud-shell .ahud-mark { color: var(--fg-subtle); }
  .ahud-pid {
    background: color-mix(in oklab, var(--fg) 7%, transparent);
    border-color: var(--border);
    color: var(--fg-muted);
  }
  .ahud-cmd {
    flex: 1; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-family: var(--font-mono); font-size: 11px; color: var(--fg-2);
  }
  .ahud-kill {
    flex: none;
    display: grid; place-items: center;
    width: 18px; height: 18px;
    background: none; border: 0; border-radius: 5px; cursor: pointer;
    color: var(--fg-faint); opacity: 0;
    transition: opacity var(--dur-fast), color var(--dur-fast), background var(--dur-fast);
  }
  .ahud-shell:hover .ahud-kill, .ahud-kill:focus-visible { opacity: 1; }
  .ahud-kill:hover {
    color: var(--danger);
    background: color-mix(in oklab, var(--danger) 10%, transparent);
  }

  @media (prefers-reduced-motion: reduce) {
    .ahud, .ahud-list { animation: none; }
    .ahud-row .ahud-mark :global(.ahud-spin) { animation: none; }
  }
</style>
