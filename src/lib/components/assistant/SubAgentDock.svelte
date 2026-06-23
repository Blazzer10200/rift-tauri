<script lang="ts">
  // Live sub-agent activity dock. Single-column accordion: each Task/Agent the
  // current turn spawned is a collapsible section that streams its own transcript
  // (thinking / text / tool steps) as its parent-tagged frames arrive — see
  // applySubAgentFrame in streaming.ts.
  import { slide } from "svelte/transition";
  import { assistant } from "../../state/assistant.svelte";
  import { activityDock } from "../../state/activityDock.svelte";
  import { captionForTool } from "./toolCaption";
  import { tooltip } from "$lib/actions/tooltip";
  import Markdown from "./Markdown.svelte";
  import { scale } from "svelte/transition";
  import { Loader2, Check, AlertTriangle, ChevronDown, ChevronRight, Bot, Sparkles, Minus } from "lucide-svelte";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  const spawns = $derived(assistant.activeTab?.agentSpawns ?? []);
  const runningCount = $derived(spawns.filter((a) => a.completedAt == null).length);

  type Status = "running" | "done" | "error";
  function statusOf(a: { completedAt: number | null; isError: boolean }): Status {
    if (a.completedAt == null) return "running";
    return a.isError ? "error" : "done";
  }

  // Section open-state. Default: running agents + a lone agent are open; once
  // there are several, completed ones collapse so the panel stays scannable.
  // A user click records an override that wins over the default.
  let overrides = $state<Map<string, boolean>>(new Map());
  function isOpen(a: { id: string; completedAt: number | null }): boolean {
    const o = overrides.get(a.id);
    if (o !== undefined) return o;
    if (a.completedAt == null) return true;
    return spawns.length === 1;
  }
  function toggleAgent(id: string, cur: boolean) {
    const next = new Map(overrides);
    next.set(id, !cur);
    overrides = next;
  }

  // Per-tool-step result expansion.
  let openTools = $state<Set<string>>(new Set());
  function toggleTool(id: string) {
    const next = new Set(openTools);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    openTools = next;
  }

  // Live ticker — only ticks while something is in flight.
  let now = $state(Date.now());
  $effect(() => {
    if (runningCount === 0) return;
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  function fmtDur(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const s = ms / 1000;
    if (s < 60) return `${s.toFixed(s < 10 ? 1 : 0)}s`;
    const m = Math.floor(s / 60);
    return `${m}m ${Math.round(s % 60)}s`;
  }
  const elapsed = (a: { startedAt: number; completedAt: number | null }) =>
    fmtDur((a.completedAt ?? now) - a.startedAt);

  // Count the tool steps in a transcript for the collapsed-section summary.
  const toolCount = (blocks: { type: string }[]) => blocks.filter((b) => b.type === "tool").length;
  // Of those steps, how many have settled (done/error) — drives the progress bar.
  const doneToolCount = (blocks: { type: string; status?: string }[]) =>
    blocks.filter((b) => b.type === "tool" && b.status !== "pending").length;
</script>

{#if !activityDock.open}
  <!-- Collapsed: a compact live pill anchored top-right. Click to expand the
       card. Hidden entirely when there's nothing to show (idle + no spawns) so
       a fresh chat stays clean. -->
  {#if spawns.length > 0}
    <button
      class="subagent-pill"
      class:live={runningCount > 0}
      transition:scale={{ duration: reducedMotion ? 0 : 160, start: 0.85 }}
      onclick={() => activityDock.toggle()}
      onpointerenter={() => activityDock.notePointerEnter()}
      use:tooltip={runningCount > 0 ? `${runningCount} sub-agent${runningCount === 1 ? "" : "s"} running — click to expand` : "Sub-agent activity — click to expand"}
      aria-label="Expand sub-agent activity"
    >
      <span class="pill-badge">
        {#if runningCount > 0}<Loader2 size={13} class="spin" />{:else}<Bot size={13} />{/if}
      </span>
      {#if runningCount > 0}
        <span class="pill-live"><span class="live-dot"></span>{runningCount}</span>
      {:else}
        <span class="pill-count">{spawns.length}</span>
      {/if}
    </button>
  {/if}
{:else}
<div class="subagent-dock" role="complementary" aria-label="Sub-agent activity"
     transition:scale={{ duration: reducedMotion ? 0 : 180, start: 0.96 }}
     onpointerenter={() => activityDock.notePointerEnter()}>
  <header class="head">
    <span class="head-badge" class:live={runningCount > 0}><Bot size={15} /></span>
    <span class="head-text">
      <span class="title">Sub-agents</span>
      {#if runningCount > 0}
        <span class="head-live" use:tooltip={`${runningCount} running`}><span class="live-dot"></span>{runningCount} live</span>
      {:else if spawns.length > 0}
        <span class="head-rest">{spawns.length} done</span>
      {/if}
    </span>
    {#if spawns.length > 0}<span class="count">{spawns.length}</span>{/if}
    <button class="close" onclick={() => activityDock.toggle()} use:tooltip={"Minimize to pill"} aria-label="Minimize sub-agent panel">
      <Minus size={15} />
    </button>
  </header>

  {#if spawns.length === 0}
    <div class="empty">
      <span class="empty-icon"><Bot size={24} /></span>
      <p>No sub-agents yet</p>
      <span class="empty-sub">When Claude delegates to a Task or Agent, its live work streams here.</span>
    </div>
  {:else}
    <div class="agents">
      {#each spawns as a (a.id)}
        {@const st = statusOf(a)}
        {@const open = isOpen(a)}
        <section class="agent" data-status={st} class:open>
          <button class="agent-head" onclick={() => toggleAgent(a.id, open)} aria-expanded={open}>
            <span class="chev">{#if open}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}</span>
            <span class="stat" data-status={st}>
              {#if st === "running"}<Loader2 size={13} class="spin" />
              {:else if st === "error"}<AlertTriangle size={13} />
              {:else}<Check size={13} />{/if}
            </span>
            <span class="meta">
              <span class="meta-top">
                <span class="kind-icon" data-kind={a.kind === "skill" ? "skill" : "agent"}>
                  {#if a.kind === "skill"}<Sparkles size={12} />{:else}<Bot size={12} />{/if}
                </span>
                <span class="type">{a.subagentType}</span>
                <span class="elapsed mono">{elapsed(a)}</span>
              </span>
              <span class="desc">{a.description}</span>
            </span>
          </button>

          {#if open}
            <div class="agent-body" transition:slide={{ duration: reducedMotion ? 0 : 200 }}>
              {#if a.blocks.length === 0}
                <div class="thinking-line"><Loader2 size={13} class="spin" /><span class="think-text">Starting up…</span></div>
              {:else}
                {#each a.blocks as b, i (i)}
                  {#if b.type === "thinking"}
                    <div class="block thinking-line">
                      <span class="think-badge">reasoning</span>
                      {#if b.text}<span class="think-text">{b.text}</span>{/if}
                    </div>
                  {:else if b.type === "text"}
                    <div class="block text-block"><Markdown text={b.text} /></div>
                  {:else if b.type === "tool"}
                    <div class="block tool" data-status={b.status}>
                      <button class="tool-head" onclick={() => toggleTool(b.id)} aria-expanded={openTools.has(b.id)}>
                        <span class="tool-stat">
                          {#if b.status === "pending"}<Loader2 size={12} class="spin" />
                          {:else if b.status === "error"}<AlertTriangle size={12} />
                          {:else}<Check size={12} />{/if}
                        </span>
                        <span class="tool-label">{captionForTool(b.name, b.input)}</span>
                        {#if b.status !== "pending" && typeof b.durationMs === "number" && b.durationMs >= 1000}
                          <span class="tool-dur mono">{fmtDur(b.durationMs)}</span>
                        {/if}
                        {#if b.result}<span class="tool-chev">{#if openTools.has(b.id)}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}</span>{/if}
                      </button>
                      {#if b.result && openTools.has(b.id)}
                        <pre class="tool-result mono" transition:slide={{ duration: reducedMotion ? 0 : 160 }}>{b.result}</pre>
                      {/if}
                    </div>
                  {/if}
                {/each}
              {/if}
            </div>
          {:else}
            {@const total = toolCount(a.blocks)}
            {@const settled = doneToolCount(a.blocks)}
            <div class="agent-summary">
              {#if total > 0}
                <span class="prog-track" aria-hidden="true">
                  <span class="prog-fill" data-status={st} style="width:{Math.round((settled / total) * 100)}%"></span>
                </span>
                <span class="prog-label">{settled}/{total} step{total === 1 ? "" : "s"}</span>
              {:else}
                <span class="prog-label muted">no steps yet</span>
              {/if}
            </div>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>
{/if}

<style>
  /* ── Floating card ── a fixed-width top-right overlay (anchored by .subagent-float
     in AssistantPage), not a full-height column. Rounded, raised, self-scrolling;
     it floats above the chat instead of reserving a side column. */
  .subagent-dock {
    width: 360px; max-width: calc(100vw - 32px);
    max-height: min(62vh, 560px);
    display: flex; flex-direction: column;
    background: var(--bg-elev-1);
    border: 1px solid color-mix(in oklch, var(--border) 85%, transparent);
    border-radius: var(--radius-lg);
    box-shadow: 0 12px 40px -12px color-mix(in oklch, var(--shadow, #000) 55%, transparent),
                0 2px 8px -4px color-mix(in oklch, var(--shadow, #000) 40%, transparent);
    overflow: hidden;
    backdrop-filter: blur(2px);
  }

  /* ── Collapsed pill ── the idle/minimized affordance. A small capsule with the
     bot/spinner badge + live or done count; clicking expands the card. */
  .subagent-pill {
    display: inline-flex; align-items: center; gap: 6px;
    height: 30px; padding: 0 11px 0 9px;
    border-radius: 999px;
    border: 1px solid color-mix(in oklch, var(--border) 85%, transparent);
    background: color-mix(in oklch, var(--bg-elev-1) 92%, transparent);
    color: var(--fg-2); cursor: pointer;
    box-shadow: 0 6px 20px -8px color-mix(in oklch, var(--shadow, #000) 50%, transparent);
    backdrop-filter: blur(4px);
    transition: border-color var(--dur-base) var(--ease-soft), box-shadow var(--dur-base) var(--ease-soft),
                background var(--dur-fast) ease-out, transform var(--dur-fast) var(--ease-soft);
  }
  .subagent-pill:hover { background: var(--bg-elev-1); transform: translateY(-1px); }
  .subagent-pill.live {
    border-color: color-mix(in oklch, var(--accent) 42%, transparent);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 16%, transparent),
                0 6px 22px -8px color-mix(in oklch, var(--accent) 45%, transparent);
  }
  .pill-badge { display: grid; place-items: center; color: var(--fg-muted); }
  .subagent-pill.live .pill-badge { color: var(--accent); }
  .pill-live {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: var(--fs-xs); font-weight: 650; color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  .pill-count {
    font-size: var(--fs-xs); font-weight: 650; color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }

  /* ── Header ── lighter than a bordered toolbar: a soft gradient wash + an
     accent-badged title, no hard bottom rule (a faint shadow separates it). */
  .head {
    flex: 0 0 auto;
    display: flex; align-items: center; gap: 9px;
    padding: 0 8px 0 12px;
    height: var(--titlebar-h);
    background: linear-gradient(180deg, color-mix(in oklch, var(--bg-elev-1) 60%, var(--bg)), var(--bg));
    box-shadow: 0 1px 0 color-mix(in oklch, var(--border) 60%, transparent);
    color: var(--fg);
  }
  .head-badge {
    flex: 0 0 auto; display: grid; place-items: center;
    width: 26px; height: 26px; border-radius: var(--radius-sm);
    background: var(--bg-elev-2); color: var(--fg-muted);
    transition: color var(--dur-base) var(--ease-soft), background var(--dur-base) var(--ease-soft);
  }
  .head-badge.live {
    color: var(--accent);
    background: var(--accent-soft);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 30%, transparent);
  }
  .head-text { display: flex; flex-direction: column; gap: 1px; line-height: 1.1; min-width: 0; }
  .head .title { font-size: var(--fs-sm); font-weight: 650; letter-spacing: -0.01em; }
  .head-rest { font-size: 10px; color: var(--fg-subtle); }
  .head .count {
    margin-left: auto;
    font-size: var(--fs-xs); font-weight: 650; line-height: 1;
    padding: 3px 7px; border-radius: 999px;
    background: var(--field); color: var(--fg-muted);
  }
  .head-live {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: 10px; font-weight: 600; letter-spacing: 0.02em; text-transform: uppercase;
    color: var(--accent);
  }
  .live-dot {
    width: 6px; height: 6px; border-radius: 999px; background: var(--accent);
    box-shadow: 0 0 0 0 var(--accent-soft); animation: live-pulse 1.8s var(--ease-soft) infinite;
  }
  .close {
    flex: 0 0 auto; display: grid; place-items: center;
    width: 26px; height: 26px; border-radius: var(--radius-sm);
    color: var(--fg-subtle); background: transparent; border: 0; cursor: pointer;
    transition: background var(--dur-fast) ease-out, color var(--dur-fast) ease-out;
  }
  .close:hover { background: var(--surface-hover); color: var(--fg); }

  /* ── Empty state ── card shrink-wraps content, so give it a stable height
     rather than flex-filling a (now absent) full-height column. */
  .empty {
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 7px; padding: 30px 24px; text-align: center;
    animation: enter var(--dur-base) var(--ease-page);
  }
  .empty-icon {
    display: grid; place-items: center; width: 48px; height: 48px; border-radius: var(--radius-lg);
    background: var(--bg-inset); color: var(--fg-faint); margin-bottom: 2px;
  }
  .empty p { margin: 0; font-size: var(--fs-md); font-weight: 600; color: var(--fg-2); }
  .empty-sub { font-size: var(--fs-xs); line-height: 1.55; max-width: 230px; color: var(--fg-subtle); }

  /* ── Agent cards ── each spawn is a distinct raised surface w/ breathing room,
     not a hard-bordered list row. Running cards get an accent ring + soft glow. */
  .agents {
    flex: 1 1 auto; min-height: 0; overflow-y: auto;
    display: flex; flex-direction: column; gap: 8px;
    padding: 9px;
  }
  .agent {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-elev-1);
    overflow: hidden;
    transition: border-color var(--dur-base) var(--ease-soft), box-shadow var(--dur-base) var(--ease-soft);
    animation: enter var(--dur-base) var(--ease-page);
  }
  .agent[data-status="running"] {
    border-color: color-mix(in oklch, var(--accent) 38%, transparent);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 14%, transparent),
                0 4px 18px -10px color-mix(in oklch, var(--accent) 50%, transparent);
  }
  .agent[data-status="error"] { border-color: color-mix(in oklch, var(--danger) 40%, transparent); }
  .agent-head {
    width: 100%; position: relative;
    display: flex; align-items: flex-start; gap: 8px;
    padding: 10px 11px 10px 12px; text-align: left;
    background: transparent; border: 0; cursor: pointer; color: var(--fg);
    transition: background var(--dur-fast) ease-out;
  }
  .agent-head::before {
    content: ""; position: absolute; left: 0; top: 0; bottom: 0; width: 2px;
    background: transparent; transition: background var(--dur-fast) ease-out;
  }
  .agent-head:hover { background: var(--surface-hover); }
  .agent[data-status="running"] > .agent-head::before { background: var(--accent); }
  .agent[data-status="running"].open > .agent-head {
    background: color-mix(in oklch, var(--accent-soft) 50%, transparent);
  }
  .chev { flex: 0 0 auto; margin-top: 1px; display: grid; place-items: center; color: var(--fg-subtle); }
  .stat { flex: 0 0 auto; margin-top: 1px; display: grid; place-items: center; }
  .stat[data-status="running"] { color: var(--accent); }
  .stat[data-status="done"] { color: var(--ok); }
  .stat[data-status="error"] { color: var(--danger); }
  .meta { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 auto; }
  .meta-top { display: flex; align-items: center; gap: 6px; }
  .kind-icon { flex: 0 0 auto; display: grid; place-items: center; color: var(--fg-subtle); }
  .kind-icon[data-kind="skill"] { color: var(--accent); }
  .type {
    flex: 1 1 auto; min-width: 0;
    font-size: var(--fs-sm); font-weight: 600; color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .elapsed { flex: 0 0 auto; font-size: 10px; color: var(--fg-subtle); }
  /* The spawn `description` is the full prompt — for a skill spawn (/plan) it's a
     wall of text. Clamp to 2 lines so the head stays a scannable label; the full
     prompt lives in the main transcript's Task tool-call. */
  .desc {
    font-size: var(--fs-xs); line-height: 1.45; color: var(--fg-muted);
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical;
    overflow: hidden; text-overflow: ellipsis;
  }

  .agent-summary {
    display: flex; align-items: center; gap: 8px;
    padding: 0 12px 10px 33px;
  }
  .prog-track {
    flex: 1 1 auto; height: 4px; border-radius: 999px; overflow: hidden;
    background: var(--bg-inset);
  }
  .prog-fill {
    display: block; height: 100%; border-radius: 999px;
    background: var(--ok);
    transition: width var(--dur-slow) var(--ease-page), background var(--dur-base);
  }
  .prog-fill[data-status="running"] { background: var(--accent); }
  .prog-fill[data-status="error"] { background: var(--danger); }
  .prog-label { flex: 0 0 auto; font-size: 10px; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .prog-label.muted { color: var(--fg-faint); }

  /* ── Transcript body ── */
  .agent-body {
    display: flex; flex-direction: column; gap: 5px;
    padding: 4px 12px 13px 13px;
    margin-left: 18px;
    border-left: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    transition: border-color var(--dur-base) var(--ease-soft);
  }
  .agent[data-status="running"] .agent-body { border-left-color: color-mix(in oklch, var(--accent) 35%, transparent); }
  .block { animation: enter var(--dur-base) var(--ease-page); }

  .thinking-line { display: flex; align-items: center; gap: 7px; font-size: var(--fs-xs); color: var(--fg-muted); }
  .think-badge {
    flex: 0 0 auto;
    font-size: 9.5px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;
    padding: 2px 6px; border-radius: var(--radius-xs);
    background: var(--field); color: var(--fg-subtle);
  }
  .think-text { font-style: italic; line-height: 1.5; }

  .text-block { font-size: var(--fs-md); line-height: 1.55; }

  /* Lighter than a stack of bordered cards: each step is a borderless checklist
     row (status icon + label) so a long tool list reads as one clean column.
     Only an in-flight step gets a subtle accent wash; results stay expandable. */
  .tool {
    border-radius: var(--radius-sm);
    overflow: hidden;
    transition: background var(--dur-fast) ease-out;
  }
  .tool[data-status="pending"] {
    background: color-mix(in oklch, var(--accent-soft) 16%, transparent);
    animation: tool-pulse 1.9s var(--ease-soft) infinite;
  }
  .tool[data-status="error"] { background: color-mix(in oklch, var(--danger) 9%, transparent); }
  .tool-head {
    width: 100%; display: flex; align-items: center; gap: 7px;
    padding: 3px 7px; text-align: left;
    background: transparent; border: 0; cursor: pointer; color: var(--fg-2);
    font-size: var(--fs-sm); transition: background var(--dur-fast) ease-out;
  }
  .tool-head:hover { background: color-mix(in oklch, var(--surface-hover) 60%, transparent); }
  .tool-stat { flex: 0 0 auto; display: grid; place-items: center; color: var(--fg-muted); }
  .tool[data-status="pending"] .tool-stat { color: var(--accent); }
  .tool[data-status="done"] .tool-stat { color: var(--ok); }
  .tool[data-status="error"] .tool-stat { color: var(--danger); }
  .tool-label { flex: 1 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tool-dur { flex: 0 0 auto; font-size: 10px; color: var(--fg-subtle); }
  .tool-chev { flex: 0 0 auto; display: grid; place-items: center; color: var(--fg-subtle); }
  .tool-result {
    margin: 4px 0 2px; padding: 8px 10px;
    border-left: 2px solid color-mix(in oklch, var(--border) 70%, transparent);
    border-radius: var(--radius-xs);
    background: var(--bg-inset);
    font-size: var(--fs-xs); line-height: 1.5;
    white-space: pre-wrap; word-break: break-word;
    max-height: 240px; overflow: auto; color: var(--fg-muted);
  }

  /* Slimmer, quieter scrollbars scoped to the dock — the app-wide 10px bar reads
     too heavy in this narrow panel. */
  .agents, .tool-result { scrollbar-width: thin; }
  .agents::-webkit-scrollbar, .tool-result::-webkit-scrollbar { width: 6px; height: 6px; }
  .agents::-webkit-scrollbar-thumb, .tool-result::-webkit-scrollbar-thumb {
    background: color-mix(in oklch, var(--border-strong) 80%, transparent);
    border: 0; border-radius: 999px;
  }
  .agents::-webkit-scrollbar-thumb:hover, .tool-result::-webkit-scrollbar-thumb:hover {
    background: var(--border-strong);
  }

  :global(.subagent-dock .spin) { animation: subagent-spin 0.9s linear infinite; }

  @keyframes subagent-spin { to { transform: rotate(360deg); } }
  @keyframes enter { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: translateY(0); } }
  @keyframes live-pulse { 0% { box-shadow: 0 0 0 0 var(--accent-soft); } 70% { box-shadow: 0 0 0 5px transparent; } 100% { box-shadow: 0 0 0 0 transparent; } }
  @keyframes tool-pulse {
    0%, 100% { background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent); }
    50%      { background: color-mix(in oklch, var(--accent-soft) 22%, var(--bg-elev-1)); }
  }

  @media (prefers-reduced-motion: reduce) {
    .empty, .block, .agent { animation: none; }
    .live-dot, .tool[data-status="pending"] { animation: none; }
  }
</style>
