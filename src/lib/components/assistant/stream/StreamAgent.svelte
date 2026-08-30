<script lang="ts">
  import { Bot, Loader2, CheckCircle2, AlertCircle, ChevronDown, ArrowRight, Brain,
    FileSearch, FilePen, FilePlus, Search, FolderTree, Terminal, Globe, AppWindow, GitBranch, ListChecks, Wrench } from "@lucide/svelte";
  import { fmtDur, type StreamTool } from "./streamModel";
  import { captionForTool, agentNowLine } from "../toolCaption";
  import { fmtTokens } from "$lib/state/assistant/helpers";
  import Markdown from "../Markdown.svelte";
  import type { Block, TabState } from "$lib/state/assistant.svelte";

  // Live delegated sub-agent, rendered inline in the transcript (CC-UI ref §5) —
  // the retired floating dock's per-agent card, moved into the conversation flow.
  // `tool` is the parent Task/Agent tool block; `spawn` is this agent's live
  // sub-transcript (agentSpawns entry, its `blocks` accumulate as parent-tagged
  // frames arrive — see applySubAgentFrame in streaming.ts). Falls back to `tool`
  // alone when the spawn was pruned (MAX_SPAWNS) or predates tracking.
  type Spawn = TabState["agentSpawns"][number];
  let { tool, spawn = undefined, childSpawns = [], workspaceRoot = null }:
    { tool: StreamTool; spawn?: Spawn; childSpawns?: Spawn[]; workspaceRoot?: string | null } = $props();

  // Tool-kind → glyph (ported from the retired SubAgentDock) so each step reads
  // as a distinct action, not a flat bullet.
  function toolIcon(name: string): typeof Bot {
    const n = name.replace(/^mcp__rift__/, "");
    if (n === "Read" || n === "read_file") return FileSearch;
    if (n === "Edit" || n === "MultiEdit" || n === "NotebookEdit") return FilePen;
    if (n === "Write") return FilePlus;
    if (n === "Grep" || n === "grep" || n === "Glob") return Search;
    if (n === "list_dir") return FolderTree;
    if (n === "Bash" || n === "remote_bash" || n === "BashOutput") return Terminal;
    if (n === "WebFetch" || n === "WebSearch") return Globe;
    if (n === "open_browser") return AppWindow;
    if (n.startsWith("git_")) return GitBranch;
    if (n === "TaskCreate" || n === "TaskUpdate" || n === "TodoWrite") return ListChecks;
    return Wrench;
  }

  const status = $derived<"running" | "done" | "error">(
    spawn
      ? spawn.completedAt == null ? "running" : spawn.isError ? "error" : "done"
      : tool.status === "pending" ? "running" : tool.status,
  );
  const agentType = $derived(spawn?.subagentType ?? (tool.task ?? tool.cap ?? "task").split(" · ")[0]);
  const desc = $derived(
    spawn?.description ??
      (tool.cap.includes(" · ") ? tool.cap.split(" · ").slice(1).join(" · ") : ""),
  );
  const blocks = $derived((spawn?.blocks ?? []) as Block[]);
  const toolSteps = $derived(blocks.filter((b) => b.type === "tool"));

  // One-line preview for the agent's own prose/thinking in the expanded
  // timeline — flattened + capped so a chatty agent stays a peek, not a wall.
  function snip(s: string, n = 220): string {
    const t = s.trim().replace(/\s+/g, " ");
    return t.length > n ? t.slice(0, n) + "…" : t;
  }

  // Live "now-doing" headline while running — shared with ActivityHud via
  // agentNowLine (toolCaption.ts) so the card and the pinned periscope can't
  // drift. This is what made the retired dock read as alive, not a spinner.
  const nowLine = $derived(status === "running" ? agentNowLine(blocks) : null);

  // Ticking duration while running; final wall-clock when done. Interval is
  // cleared on completion/unmount ($effect cleanup) — this now mounts per-card.
  let now = $state(0);
  $effect(() => {
    if (status !== "running") return;
    now = Date.now();
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  const durSecs = $derived.by(() => {
    if (status === "running") {
      const started = spawn?.startedAt;
      return started != null && now > 0 ? Math.max(0, (now - started) / 1000) : null;
    }
    if (spawn?.completedAt != null && spawn.startedAt != null)
      return (spawn.completedAt - spawn.startedAt) / 1000;
    return tool.durSecs > 0 ? tool.durSecs : null;
  });

  const result = $derived(tool.result);
  const expandable = $derived(blocks.length > 0 || !!result);
  let open = $state(false);

  // Terminal-style tail-follow: while running + open, the timeline sticks to
  // the newest row as frames land — unless the user scrolled up to read back.
  let bodyEl = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
  function onBodyScroll() {
    if (!bodyEl) return;
    follow = bodyEl.scrollHeight - bodyEl.scrollTop - bodyEl.clientHeight < 40;
  }
  $effect(() => {
    void blocks.length;
    if (open && status === "running" && follow && bodyEl) bodyEl.scrollTop = bodyEl.scrollHeight;
  });
</script>

<div class="sacard" data-status={status} id={"sacard-" + tool.id}>
  <button class="sa-head" class:sa-clickable={expandable} type="button"
    onclick={() => expandable && (open = !open)} aria-expanded={open}>
    <span class="sa-bot" aria-hidden="true"><Bot size={14} strokeWidth={2} /></span>
    <span class="sa-pill">{agentType}</span>
    {#if desc}<span class="sa-desc">{desc}</span>{/if}
    {#if toolSteps.length > 0 && !open}<span class="sa-dur">{toolSteps.length} step{toolSteps.length === 1 ? "" : "s"}</span>{/if}
    {#if spawn?.tokens}<span class="sa-dur">{fmtTokens(spawn.tokens)} tok</span>{/if}
    {#if durSecs != null}<span class="sa-dur">{fmtDur(durSecs)}</span>{/if}
    <span class="sa-stat" aria-label={status}>
      {#if status === "running"}<Loader2 size={13} class="sa-spin" />
      {:else if status === "error"}<AlertCircle size={13} />
      {:else}<CheckCircle2 size={13} />{/if}
    </span>
    {#if expandable}<span class="sa-chev" class:open><ChevronDown size={13} strokeWidth={2} /></span>{/if}
  </button>

  {#if nowLine}
    <div class="sa-now">
      <span class="sa-now-ic" aria-hidden="true">
        {#if nowLine.thinking}<Brain size={12} />{:else}<Loader2 size={12} class="sa-spin" />{/if}
      </span>
      <span class="sa-now-label" class:has-snip={!!nowLine.snip}>{nowLine.label}{#if nowLine.thinking && !nowLine.snip}<span class="sa-dots"><span></span><span></span><span></span></span>{/if}</span>
      {#if nowLine.snip}<span class="sa-now-snip">{nowLine.snip}</span>{/if}
    </div>
  {/if}

  {#snippet timeline(bs: Block[], kids: Spawn[])}
    {#each bs as b, i (i)}
      {#if b.type === "tool"}
        {@const Ic = toolIcon(b.name)}
        <div class="sa-step" data-status={b.status}>
          <span class="sa-step-stat" aria-hidden="true">
            {#if b.status === "pending"}<Loader2 size={11} class="sa-spin" />
            {:else if b.status === "error"}<AlertCircle size={11} />
            {:else}<CheckCircle2 size={11} />{/if}
          </span>
          <span class="sa-step-ic" aria-hidden="true"><Ic size={12} /></span>
          <span class="sa-step-label">{captionForTool(b.name, b.input ?? {})}</span>
          {#if b.status === "pending" && typeof b.lastProgressAt === "number" && now > 0 && now - b.lastProgressAt < 6000}
            <span class="sa-beat" title="CLI heartbeat — tool confirmed alive" aria-hidden="true"></span>
          {/if}
          {#if b.status === "pending" && typeof b.startedAt === "number" && now > 0 && now - b.startedAt >= 3000}
            <span class="sa-step-dur">{fmtDur((now - b.startedAt) / 1000)}</span>
          {:else if b.status !== "pending" && typeof b.durationMs === "number" && b.durationMs >= 1000}
            <span class="sa-step-dur">{fmtDur(b.durationMs / 1000)}</span>
          {/if}
        </div>
        {#if b.status === "error" && b.result}
          <div class="sa-step-err">{snip(b.result, 160)}</div>
        {/if}
        {@const kid = kids.find((c) => c.id === b.id)}
        {#if kid}
          <!-- Depth-2 child agent — nested tint-group under the step that
               spawned it (hairline rail, not a second full card — island
               nesting ceiling, DESIGN §8). -->
          <div class="sa-kid" data-status={kid.completedAt == null ? "running" : kid.isError ? "error" : "done"}>
            <div class="sa-kid-head">
              <span class="sa-kid-stat" aria-hidden="true">
                {#if kid.completedAt == null}<Loader2 size={10} class="sa-spin" />
                {:else if kid.isError}<AlertCircle size={10} />
                {:else}<CheckCircle2 size={10} />{/if}
              </span>
              <span class="sa-kid-pill">{kid.subagentType}</span>
              {#if kid.description}<span class="sa-kid-desc">{kid.description}</span>{/if}
              {#if kid.tokens}<span class="sa-step-dur">{fmtTokens(kid.tokens)} tok</span>{/if}
            </div>
            {@render timeline(kid.blocks as Block[], [])}
          </div>
        {/if}
      {:else if b.type === "thinking" && b.text.trim()}
        <div class="sa-think">
          <span class="sa-step-ic" aria-hidden="true"><Brain size={12} /></span>
          <span>{snip(b.text)}</span>
        </div>
      {:else if b.type === "text" && b.text.trim()}
        <div class="sa-prose">{snip(b.text)}</div>
      {/if}
    {/each}
  {/snippet}

  {#if open && expandable}
    <div class="sa-body" bind:this={bodyEl} onscroll={onBodyScroll}>
      {@render timeline(blocks, childSpawns)}
      {#if result}
        <div class="sa-result"><ArrowRight size={13} strokeWidth={2} /><div class="sa-result-md"><Markdown text={result} {workspaceRoot} /></div></div>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* Bordered card — first-class, distinct from the boxless tool rows around it
     (CC-UI ref §5). Translucent surface so it blends into the transcript rather
     than reading as a pasted panel; running state warms the hairline. */
  /* Same tile family as every block; running warms the hairline with the
     shell "running" accent tint. The pill + bot glyph carry delegation —
     no wash, no seam (atmosphere doctrine). */
  .sacard {
    margin: var(--stream-gap, 13px) 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: color-mix(in oklab, var(--fg) 2.8%, transparent);
    overflow: hidden;
    transition: border-color 240ms var(--ease-soft);
    animation: blockIn var(--dur-base) var(--ease-page) both;
  }
  .sacard:hover { border-color: var(--border-strong); }
  .sacard[data-status="running"] {
    border-color: color-mix(in oklab, var(--accent) 28%, var(--border));
  }

  .sa-head {
    display: flex; align-items: center; gap: 9px; width: 100%;
    padding: 8px 11px; border: 0; text-align: left;
    color: var(--fg-2); font: inherit;
    background: transparent;
    transition: background var(--dur-fast);
  }
  .sa-clickable { cursor: pointer; }
  .sa-clickable:hover { background: color-mix(in oklab, var(--fg) 4%, transparent); }
  .sa-bot { display: inline-flex; color: var(--accent); flex: none; }
  .sa-pill {
    display: inline-flex; align-items: center; padding: 2px 9px; border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent); font-size: 10.5px; font-weight: 600; letter-spacing: 0.02em;
    font-family: var(--font-mono); flex: none;
  }
  .sa-desc {
    flex: 1; min-width: 0; color: var(--fg-2); font-size: 12.5px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .sa-dur {
    flex: none; font-size: 10px; padding: 1px 6px; border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    color: var(--fg-muted); border: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    font-family: var(--font-mono); font-variant-numeric: tabular-nums; font-weight: 600;
  }
  /* Status lives in the glyph, never a row-background tint (CC-UI ref §4/§9):
     activity green while running, outcome tokens once settled. */
  .sa-stat { display: inline-flex; flex: none; }
  .sacard[data-status="running"] .sa-stat { color: var(--status-busy); }
  .sacard[data-status="done"] .sa-stat { color: var(--ok); }
  .sacard[data-status="error"] .sa-stat { color: var(--danger); }
  .sa-chev { display: inline-flex; color: var(--fg-faint); flex: none; transition: transform var(--dur-fast); }
  .sa-chev.open { transform: rotate(180deg); }
  :global(.sa-spin) { animation: ringspin 1s linear infinite; }

  /* Live "now-doing" line — the momentum signal while the agent is in flight. */
  .sa-now {
    display: flex; align-items: center; gap: 8px;
    padding: 7px 12px 9px;
    border-top: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    font-size: 12px;
  }
  .sa-now-ic { display: inline-flex; flex: none; color: var(--status-busy); }
  .sa-now-label { color: var(--fg-muted); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sa-now-label.has-snip { flex: none; }
  /* The agent's own newest words — the live feed the forwarded frames carry. */
  .sa-now-snip {
    flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: var(--fg-faint); font-size: 11.5px; font-style: italic;
  }
  .sa-dots { display: inline-flex; gap: 3px; margin-left: 6px; vertical-align: middle; }
  .sa-dots span { width: 3px; height: 3px; border-radius: 50%; background: var(--status-busy); animation: sa-dot 1.1s ease-in-out infinite; }
  .sa-dots span:nth-child(2) { animation-delay: 0.15s; }
  .sa-dots span:nth-child(3) { animation-delay: 0.3s; }
  @keyframes sa-dot { 0%, 60%, 100% { opacity: 0.3; } 30% { opacity: 1; } }

  /* Expanded timeline — the sub-agent's own thinking/prose/tool steps in
     arrival order. Scroll-clamped so a chatty agent stays a card, not a wall
     (bell-portal maxH precedent). */
  .sa-body {
    display: flex; flex-direction: column; gap: 5px;
    padding: 7px 12px 10px;
    border-top: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    background: var(--bg-inset);
    animation: workOpen 0.3s var(--ease-page) both;
    max-height: 300px; overflow-y: auto;
  }
  /* Agent thinking — dim italic whisper; prose — quiet narration. Both stay
     visually below the tool rows (fg-muted) so steps keep primacy. */
  .sa-think {
    display: flex; align-items: flex-start; gap: 8px;
    font-size: 11px; color: var(--fg-faint); font-style: italic; line-height: 1.45;
    padding-left: 19px; /* align under step labels (stat glyph width + gap) */
  }
  .sa-think > span:last-child { min-width: 0; }
  .sa-prose {
    font-size: 11.5px; color: var(--fg-muted); line-height: 1.45;
    padding-left: 19px;
  }
  .sa-step-err {
    font-size: 10.5px; color: var(--danger); font-family: var(--font-mono);
    line-height: 1.4; padding-left: 39px; /* indent under the step label */
    overflow-wrap: anywhere;
  }
  /* New rows rise in as frames land — orientation beat, not theater. */
  .sa-step, .sa-think, .sa-prose { animation: sa-row-in 0.22s var(--ease-page) both; }
  @keyframes sa-row-in {
    from { opacity: 0; transform: translateY(3px); }
    to { opacity: 1; transform: none; }
  }
  .sa-step { display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: var(--fg-muted); }
  .sa-step-stat { display: inline-flex; flex: none; color: var(--fg-faint); }
  .sa-step[data-status="pending"] .sa-step-stat { color: var(--status-busy); }
  .sa-step[data-status="done"] .sa-step-stat { color: color-mix(in oklch, var(--ok) 78%, var(--fg-faint)); }
  .sa-step[data-status="error"] .sa-step-stat { color: var(--danger); }
  .sa-step-ic { display: inline-flex; flex: none; color: var(--fg-subtle); }
  .sa-step-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sa-step-dur { flex: none; font-size: 10px; color: var(--fg-faint); font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  /* Heartbeat — the CLI's "still alive" ping for a long-silent call; breathing
     live dot (sanctioned stationary liveness signal, DESIGN §8). */
  .sa-beat {
    flex: none; width: 5px; height: 5px; border-radius: 50%;
    background: var(--status-busy);
    animation: sa-beat var(--pulse-live) ease-in-out infinite;
  }
  @keyframes sa-beat { 0%, 100% { opacity: 0.35; } 50% { opacity: 1; } }
  /* Depth-2 child agent — indented tint-group with a hairline rail. */
  .sa-kid {
    display: flex; flex-direction: column; gap: 4px;
    margin: 1px 0 3px 19px; padding: 5px 8px 6px;
    border-left: 2px solid color-mix(in oklab, var(--accent) 25%, var(--border));
    border-radius: 0 6px 6px 0;
    background: color-mix(in oklab, var(--fg) 2.5%, transparent);
  }
  .sa-kid-head { display: flex; align-items: center; gap: 7px; min-width: 0; }
  .sa-kid-stat { display: inline-flex; flex: none; color: var(--fg-faint); }
  .sa-kid[data-status="running"] .sa-kid-stat { color: var(--status-busy); }
  .sa-kid[data-status="done"] .sa-kid-stat { color: color-mix(in oklch, var(--ok) 78%, var(--fg-faint)); }
  .sa-kid[data-status="error"] .sa-kid-stat { color: var(--danger); }
  .sa-kid-pill {
    display: inline-flex; align-items: center; padding: 1px 7px; border-radius: 999px;
    background: var(--accent-soft); color: var(--accent);
    font-size: 9.5px; font-weight: 600; font-family: var(--font-mono); flex: none;
  }
  .sa-kid-desc {
    flex: 1; min-width: 0; font-size: 11px; color: var(--fg-faint);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .sa-result {
    display: flex; align-items: flex-start; gap: 7px; margin-top: 3px; padding-top: 8px;
    border-top: 1px dashed color-mix(in oklch, var(--border) 60%, transparent);
    font-size: 12px; color: var(--fg-2); line-height: 1.5;
  }
  .sa-result > :global(svg) { color: var(--fg-subtle); flex: none; margin-top: 2px; }
  .sa-result-md { flex: 1; min-width: 0; }
  /* Agent results are compact digests — pull the markdown's outer margins in. */
  .sa-result-md :global(> :first-child) { margin-top: 0; }
  .sa-result-md :global(> :last-child) { margin-bottom: 0; }

  @media (prefers-reduced-motion: reduce) {
    .sacard { animation: none; }
    .sa-body { animation: none; }
    .sa-step, .sa-think, .sa-prose { animation: none; }
    .sa-beat { animation: none; opacity: 0.8; }
    :global(.sa-spin) { animation: none; }
    .sa-dots span { animation: none; opacity: 0.7; }
  }
</style>
