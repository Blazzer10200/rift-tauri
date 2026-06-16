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
  import { Loader2, Check, AlertTriangle, ChevronDown, ChevronRight, Bot, Sparkles, X } from "lucide-svelte";

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
</script>

<div class="subagent-dock" role="complementary" aria-label="Sub-agent activity" onpointerenter={() => activityDock.notePointerEnter()}>
  <header class="head">
    <span class="head-icon" class:live={runningCount > 0}><Bot size={14} /></span>
    <span class="title">Sub-agents</span>
    {#if spawns.length > 0}<span class="count">{spawns.length}</span>{/if}
    {#if runningCount > 0}
      <span class="head-live" use:tooltip={`${runningCount} running`}><span class="live-dot"></span>{runningCount} live</span>
    {/if}
    <button class="close" onclick={() => activityDock.toggle()} use:tooltip={"Close panel"} aria-label="Close sub-agent panel">
      <X size={14} />
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
            <div class="agent-summary">
              {#if toolCount(a.blocks) > 0}{toolCount(a.blocks)} step{toolCount(a.blocks) === 1 ? "" : "s"}{:else}no steps{/if}
            </div>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .subagent-dock {
    flex: 1 1 auto;
    min-width: 0; min-height: 0;
    display: flex; flex-direction: column;
    background: var(--bg);
    border-left: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    overflow: hidden;
  }

  /* ── Header ── */
  .head {
    flex: 0 0 auto;
    display: flex; align-items: center; gap: 7px;
    padding: 0 8px 0 11px;
    height: var(--titlebar-h);
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
  }
  .head-icon { display: grid; place-items: center; color: var(--fg-muted); transition: color var(--dur-base) var(--ease-soft); }
  .head-icon.live { color: var(--accent); }
  .head .title { font-size: var(--fs-sm); font-weight: 600; letter-spacing: -0.01em; }
  .head .count {
    font-size: var(--fs-xs); font-weight: 600; line-height: 1;
    padding: 2px 6px; border-radius: 999px;
    background: var(--accent-soft); color: var(--accent);
  }
  .head-live {
    display: inline-flex; align-items: center; gap: 5px; margin-left: 2px;
    font-size: 10px; font-weight: 600; letter-spacing: 0.02em; text-transform: uppercase;
    color: var(--accent);
  }
  .live-dot {
    width: 6px; height: 6px; border-radius: 999px; background: var(--accent);
    box-shadow: 0 0 0 0 var(--accent-soft); animation: live-pulse 1.8s var(--ease-soft) infinite;
  }
  .close {
    margin-left: auto; display: grid; place-items: center;
    width: 24px; height: 24px; border-radius: var(--radius-sm);
    color: var(--fg-subtle); background: transparent; border: 0; cursor: pointer;
    transition: background var(--dur-fast) ease-out, color var(--dur-fast) ease-out;
  }
  .close:hover { background: var(--surface-hover); color: var(--fg); }

  /* ── Empty state ── */
  .empty {
    flex: 1; min-height: 0;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 7px; padding: 28px 24px; text-align: center;
    animation: enter var(--dur-base) var(--ease-page);
  }
  .empty-icon {
    display: grid; place-items: center; width: 48px; height: 48px; border-radius: var(--radius-lg);
    background: var(--bg-inset); color: var(--fg-faint); margin-bottom: 2px;
  }
  .empty p { margin: 0; font-size: var(--fs-md); font-weight: 600; color: var(--fg-2); }
  .empty-sub { font-size: var(--fs-xs); line-height: 1.55; max-width: 230px; color: var(--fg-subtle); }

  /* ── Agent accordion ── */
  .agents {
    flex: 1 1 auto; min-height: 0; overflow-y: auto;
    display: flex; flex-direction: column;
  }
  .agent {
    border-bottom: 1px solid var(--border);
    animation: enter var(--dur-base) var(--ease-page);
  }
  .agent-head {
    width: 100%; position: relative;
    display: flex; align-items: flex-start; gap: 8px;
    padding: 9px 10px 9px 11px; text-align: left;
    background: transparent; border: 0; cursor: pointer; color: var(--fg);
    transition: background var(--dur-fast) ease-out;
  }
  .agent-head::before {
    content: ""; position: absolute; left: 0; top: 0; bottom: 0; width: 2px;
    background: transparent; transition: background var(--dur-fast) ease-out;
  }
  .agent-head:hover { background: var(--surface-hover); }
  .agent[data-status="running"] .agent-head::before { background: var(--accent); }
  .agent[data-status="running"].open > .agent-head { background: var(--accent-soft); }
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
  .desc { font-size: var(--fs-xs); line-height: 1.45; color: var(--fg-muted); }

  .agent-summary {
    padding: 0 11px 9px 33px;
    font-size: 10px; color: var(--fg-subtle);
  }

  /* ── Transcript body ── */
  .agent-body {
    display: flex; flex-direction: column; gap: 8px;
    padding: 2px 12px 13px 13px;
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

  .tool {
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    border-left-width: 2px; border-radius: var(--radius-sm);
    background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent);
    overflow: hidden;
    transition: border-color var(--dur-fast) ease-out, opacity var(--dur-fast) ease-out;
  }
  .tool[data-status="done"] { opacity: 0.9; }
  .tool[data-status="done"]:hover { opacity: 1; }
  .tool[data-status="pending"] { border-left-color: var(--accent); animation: tool-pulse 1.9s var(--ease-soft) infinite; }
  .tool[data-status="error"] { border-left-color: var(--danger); }
  .tool-head {
    width: 100%; display: flex; align-items: center; gap: 8px;
    padding: 6px 9px; text-align: left;
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
    margin: 0; padding: 8px 10px;
    border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
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
