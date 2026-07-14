<script lang="ts">
  import { Sparkles, ChevronDown } from "lucide-svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import Markdown from "../Markdown.svelte";
  import { formatBoundaryAt } from "./helpers";
  import type { Block } from "../../../state/assistant.svelte";

  let { boundaryBlock }: { boundaryBlock: Extract<Block, { type: "boundary" }> } = $props();
  let boundaryExpanded = $state(false);
</script>

{#if boundaryBlock.source === "cli"}
  {@const hasPct =
    typeof boundaryBlock.ctxPctBefore === "number" &&
    typeof boundaryBlock.ctxPctEstAfter === "number"}
  <div class="boundary boundary-cli" data-role="system">
    <span class="boundary-line" aria-hidden="true"></span>
    <span
      class="boundary-pill"
      use:tooltip={"Claude Code automatically summarized older messages to free up the context window. The conversation continues normally — nothing on screen was deleted."}
    >
      <Sparkles size={11} />
      <span>Conversation compacted{boundaryBlock.trigger === "manual" ? " · manual" : ""}</span>
      {#if hasPct}
        <span class="boundary-meta mono">
          Ctx {Math.round(boundaryBlock.ctxPctBefore ?? 0)}% → {Math.round(boundaryBlock.ctxPctEstAfter ?? 0)}%
        </span>
      {/if}
    </span>
    <span class="boundary-line" aria-hidden="true"></span>
  </div>
{:else}
  {@const isCompacting = boundaryBlock.streaming === true}
  {@const showBody = isCompacting || boundaryExpanded}
  <div class="boundary" data-role="system" class:streaming={isCompacting}>
    <button
      type="button"
      class="boundary-head"
      onclick={() => (boundaryExpanded = !boundaryExpanded)}
      aria-expanded={boundaryExpanded}
      disabled={isCompacting}
      use:tooltip={isCompacting ? "Summarizing…" : `Click to ${boundaryExpanded ? "hide" : "show"} the compaction summary`}
    >
      <span class="boundary-line" aria-hidden="true"></span>
      <span class="boundary-pill">
        <Sparkles size={11} />
        {#if isCompacting}
          <span class="live-dot" aria-label="Compacting" use:tooltip={"Summarizing in progress"}></span>
          <span>Compacting · {boundaryBlock.archivedCount} message{boundaryBlock.archivedCount === 1 ? "" : "s"} · {boundaryBlock.summary.length.toLocaleString()} chars</span>
        {:else}
          <span>Conversation compacted · {boundaryBlock.archivedCount} message{boundaryBlock.archivedCount === 1 ? "" : "s"} archived</span>
          {#if typeof boundaryBlock.ctxPctBefore === "number" && typeof boundaryBlock.ctxPctEstAfter === "number"}
            <span class="boundary-meta mono" use:tooltip={"Context window utilization — pre-compact → estimated post-compact"}>
              Ctx {Math.round(boundaryBlock.ctxPctBefore)}% → est {Math.round(boundaryBlock.ctxPctEstAfter)}%
            </span>
          {/if}
          <span class="boundary-meta mono">
            ${boundaryBlock.costUsd.toFixed(4)} · {boundaryBlock.summaryModel} · {formatBoundaryAt(boundaryBlock.at)}
          </span>
          <ChevronDown size={11} class="chev" />
        {/if}
      </span>
      <span class="boundary-line" aria-hidden="true"></span>
    </button>
    {#if showBody && boundaryBlock.summary.length > 0}
      <div class="boundary-body"><Markdown text={boundaryBlock.summary} /></div>
    {/if}
  </div>
{/if}

<style>
  .boundary {
    width: 100%;
    padding: 8px 4px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .boundary-cli {
    flex-direction: row;
    align-items: center;
    gap: 10px;
  }
  .boundary-head {
    display: flex;
    align-items: center;
    gap: 10px;
    background: none;
    border: 0;
    padding: 0;
    color: inherit;
    cursor: pointer;
    width: 100%;
  }
  .boundary-line {
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      transparent,
      color-mix(in oklch, var(--border) 80%, transparent),
      color-mix(in oklab, var(--accent) 30%, transparent) 50%,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
  }
  .boundary-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-1) 86%, transparent);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid color-mix(in oklab, var(--accent) 25%, var(--border));
    box-shadow: 0 4px 14px -4px color-mix(in oklab, var(--accent) 25%, transparent);
    font-size: 11px;
    color: var(--fg-muted);
    white-space: nowrap;
    transition: background var(--dur-fast) ease, border-color var(--dur-fast) ease, transform var(--dur-fast) ease;
  }
  .boundary-head:not(:disabled):hover .boundary-pill {
    background: color-mix(in oklch, var(--bg-elev-1) 95%, transparent);
    border-color: color-mix(in oklab, var(--accent) 45%, var(--border));
    transform: translateY(-1px);
  }
  .boundary-pill :global(svg) { color: var(--accent); }
  .boundary-pill :global(.chev) {
    transition: transform var(--dur-fast) ease;
    opacity: 0.6;
  }
  .boundary-head[aria-expanded="true"] :global(.chev) {
    transform: rotate(180deg);
  }
  .boundary-meta {
    opacity: 0.55;
    font-size: 10px;
  }
  .boundary-body {
    margin: 4px 24px 0;
    padding: 10px 14px;
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    border-radius: 10px;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--fg-2);
  }
</style>
