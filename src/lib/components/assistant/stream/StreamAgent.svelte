<script lang="ts">
  import { Bot, Loader2, CheckCircle2, AlertCircle, ChevronDown, ArrowRight, Brain,
    FileSearch, FilePen, FilePlus, Search, FolderTree, Terminal, Globe, AppWindow, GitBranch, ListChecks, Wrench } from "lucide-svelte";
  import { fmtDur, type StreamTool } from "./streamModel";
  import { captionForTool, agentNowLine } from "../toolCaption";
  import type { Block, TabState } from "$lib/state/assistant.svelte";

  // Live delegated sub-agent, rendered inline in the transcript (CC-UI ref §5) —
  // the retired floating dock's per-agent card, moved into the conversation flow.
  // `tool` is the parent Task/Agent tool block; `spawn` is this agent's live
  // sub-transcript (agentSpawns entry, its `blocks` accumulate as parent-tagged
  // frames arrive — see applySubAgentFrame in streaming.ts). Falls back to `tool`
  // alone when the spawn was pruned (MAX_SPAWNS) or predates tracking.
  type Spawn = TabState["agentSpawns"][number];
  let { tool, spawn = undefined }: { tool: StreamTool; spawn?: Spawn } = $props();

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

  // Live "now-doing" headline while running — shared with AgentHud via
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
  const expandable = $derived(toolSteps.length > 0 || !!result);
  let open = $state(false);
</script>

<div class="sacard" data-status={status} id={"sacard-" + tool.id}>
  <button class="sa-head" class:sa-clickable={expandable} type="button"
    onclick={() => expandable && (open = !open)} aria-expanded={open}>
    <span class="sa-bot" aria-hidden="true"><Bot size={14} strokeWidth={2} /></span>
    <span class="sa-pill">{agentType}</span>
    {#if desc}<span class="sa-desc">{desc}</span>{/if}
    {#if toolSteps.length > 0 && !open}<span class="sa-dur">{toolSteps.length} step{toolSteps.length === 1 ? "" : "s"}</span>{/if}
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
      <span class="sa-now-label">{nowLine.label}{#if nowLine.thinking}<span class="sa-dots"><span></span><span></span><span></span></span>{/if}</span>
    </div>
  {/if}

  {#if open && expandable}
    <div class="sa-body">
      {#each toolSteps as b, i (i)}
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
            {#if b.status !== "pending" && typeof b.durationMs === "number" && b.durationMs >= 1000}
              <span class="sa-step-dur">{fmtDur(b.durationMs / 1000)}</span>
            {/if}
          </div>
        {/if}
      {/each}
      {#if result}
        <div class="sa-result"><ArrowRight size={13} strokeWidth={2} /><span>{result}</span></div>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* Bordered card — first-class, distinct from the boxless tool rows around it
     (CC-UI ref §5). Translucent surface so it blends into the transcript rather
     than reading as a pasted panel; running state warms the hairline. */
  .sacard {
    margin: 13px 0;
    border: 1px solid color-mix(in oklch, var(--border) 88%, transparent);
    border-radius: var(--radius-lg);
    background: color-mix(in oklch, var(--bg-elev-1) 82%, transparent);
    overflow: hidden;
    animation: blockIn var(--dur-base) var(--ease-page) both;
  }
  .sacard[data-status="running"] {
    border-color: color-mix(in oklch, var(--status-busy) 28%, var(--border));
  }

  .sa-head {
    display: flex; align-items: center; gap: 9px; width: 100%;
    padding: 8px 11px; background: none; border: 0; text-align: left;
    color: var(--fg-2); font: inherit;
  }
  .sa-clickable { cursor: pointer; }
  .sa-clickable:hover { background: color-mix(in oklch, var(--fg) 3%, transparent); }
  .sa-bot { display: inline-flex; color: var(--accent-hover); flex: none; }
  .sa-pill {
    display: inline-flex; align-items: center; padding: 2px 9px; border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 36%, var(--border));
    color: var(--accent-hover); font-size: 10.5px; font-weight: 600; letter-spacing: 0.02em;
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
  .sa-dots { display: inline-flex; gap: 3px; margin-left: 6px; vertical-align: middle; }
  .sa-dots span { width: 3px; height: 3px; border-radius: 50%; background: var(--status-busy); animation: sa-dot 1.1s ease-in-out infinite; }
  .sa-dots span:nth-child(2) { animation-delay: 0.15s; }
  .sa-dots span:nth-child(3) { animation-delay: 0.3s; }
  @keyframes sa-dot { 0%, 60%, 100% { opacity: 0.3; } 30% { opacity: 1; } }

  /* Expanded step trail — the sub-agent's own tool steps, captioned + iconed. */
  .sa-body {
    display: flex; flex-direction: column; gap: 5px;
    padding: 7px 12px 10px;
    border-top: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    animation: workOpen 0.3s var(--ease-page) both;
  }
  .sa-step { display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: var(--fg-muted); }
  .sa-step-stat { display: inline-flex; flex: none; color: var(--fg-faint); }
  .sa-step[data-status="pending"] .sa-step-stat { color: var(--status-busy); }
  .sa-step[data-status="done"] .sa-step-stat { color: color-mix(in oklch, var(--ok) 78%, var(--fg-faint)); }
  .sa-step[data-status="error"] .sa-step-stat { color: var(--danger); }
  .sa-step-ic { display: inline-flex; flex: none; color: var(--fg-subtle); }
  .sa-step-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sa-step-dur { flex: none; font-size: 10px; color: var(--fg-faint); font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  .sa-result {
    display: flex; align-items: flex-start; gap: 7px; margin-top: 3px; padding-top: 8px;
    border-top: 1px dashed color-mix(in oklch, var(--border) 60%, transparent);
    font-size: 12px; color: var(--fg-2); line-height: 1.5;
  }
  .sa-result > :global(svg) { color: var(--fg-subtle); flex: none; margin-top: 2px; }

  @media (prefers-reduced-motion: reduce) {
    .sacard { animation: none; }
    .sa-body { animation: none; }
    :global(.sa-spin) { animation: none; }
    .sa-dots span { animation: none; opacity: 0.7; }
  }
</style>
