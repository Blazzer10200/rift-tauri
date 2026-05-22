<script lang="ts">
  // Visual container for a "step" detected in assistant prose. Headed by a
  // text block matching /^(\*\*)?Step \d+/ (or `## Step \d+` markdown);
  // children are every block that follows until the next step header.
  //
  // Renders: numbered marker in the gutter, header prose at top, children
  // nested under a soft accent rail. Gives the chat hierarchy + breathing
  // room between steps instead of a flat sequence of prose + chips.

  import { ChevronRight } from "lucide-svelte";
  import Markdown from "./Markdown.svelte";

  let {
    stepNum,
    headerText,
    status = "neutral",
    collapsible = false,
    childSummary = null,
    children,
  }: {
    stepNum: number | null;
    headerText: string;
    status?: "neutral" | "pending" | "done" | "error";
    /** If true, children render in a collapsible region (auto-collapsed by default). */
    collapsible?: boolean;
    /** Tiny right-aligned hint shown next to the header when collapsed (e.g. "2 tools"). */
    childSummary?: string | null;
    children: import("svelte").Snippet;
  } = $props();

  // User-driven override — once expanded, stays expanded for the session.
  let userExpanded = $state(false);
  const expanded = $derived(!collapsible || userExpanded);

  function toggle() {
    if (!collapsible) return;
    userExpanded = !userExpanded;
  }
  function onKey(e: KeyboardEvent) {
    if (!collapsible) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      toggle();
    }
  }
</script>

<section
  class="step"
  data-has-num={stepNum != null}
  data-status={status}
  data-collapsible={collapsible}
  data-expanded={expanded}
>
  {#if stepNum != null}
    <div class="step-marker" aria-hidden="true">
      <span class="step-num mono">{stepNum}</span>
    </div>
  {/if}
  <div class="step-body">
    <!-- Header row. When collapsible, the whole row is a toggle target. -->
    {#if collapsible}
      <div
        class="step-head-row"
        role="button"
        tabindex="0"
        aria-expanded={expanded}
        onclick={toggle}
        onkeydown={onKey}
      >
        <div class="step-head">
          <Markdown text={headerText} />
        </div>
        {#if !expanded && childSummary}
          <span class="child-summary">{childSummary}</span>
        {/if}
        <span class="step-chev" class:open={expanded} aria-hidden="true">
          <ChevronRight size={12} />
        </span>
      </div>
    {:else}
      <div class="step-head-row">
        <div class="step-head">
          <Markdown text={headerText} />
        </div>
      </div>
    {/if}
    <!-- Collapsible children. Uses the `grid-template-rows 1fr ↔ 0fr` trick
         for a content-aware height animation w/o JS measurement. -->
    <div class="children-wrap">
      <div class="children-inner">
        <div class="step-children">
          {@render children()}
        </div>
      </div>
    </div>
  </div>
</section>

<style>
  .step {
    display: grid;
    grid-template-columns: 22px 1fr;
    column-gap: 9px;
    padding: 2px 0 4px;
    /* Parent turn-rail now spans the whole turn — local rail dropped to avoid
       double-rail. Hover/status visual moves to the step-num marker. */
    padding-left: 0;
    margin: 1px 0;
    animation: enter 320ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  .step[data-has-num="false"] {
    grid-template-columns: 1fr;
  }
  .step[data-has-num="false"] .step-marker { display: none; }

  .step-marker {
    display: flex; align-items: flex-start; justify-content: center;
    padding-top: 2px;
  }
  .step-num {
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 18px;
    border-radius: 50%;
    background: color-mix(in oklch, var(--accent-soft) 75%, transparent);
    color: var(--accent);
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    opacity: 0.85;
    box-shadow: 0 0 0 1.5px transparent;
    transition: box-shadow 220ms ease-out, background 220ms ease-out, color 220ms ease-out;
  }
  .step:hover .step-num { opacity: 1; }
  /* Status rings — colored halo around the step number. Mirrors the
     left rail color so the eye reads "this whole step succeeded/failed"
     at a glance, even when the chip status icon is off-screen. */
  .step[data-status="done"] .step-num {
    background: color-mix(in oklch, var(--ok, oklch(0.74 0.15 145)) 18%, var(--bg-elev-1));
    color: var(--ok, oklch(0.74 0.15 145));
    box-shadow: 0 0 0 1.5px color-mix(in oklch, var(--ok, oklch(0.74 0.15 145)) 45%, transparent);
  }
  .step[data-status="error"] .step-num {
    background: color-mix(in oklch, var(--danger) 22%, var(--bg-elev-1));
    color: oklch(0.85 0.13 22);
    box-shadow: 0 0 0 1.5px color-mix(in oklch, var(--danger) 55%, transparent);
  }
  .step[data-status="pending"] .step-num {
    background: color-mix(in oklch, var(--accent-soft) 85%, transparent);
    color: var(--accent);
    box-shadow: 0 0 0 1.5px color-mix(in oklch, var(--accent) 50%, transparent);
    animation: num-pulse 1.8s ease-in-out infinite;
  }
  @keyframes num-pulse {
    0%, 100% { box-shadow: 0 0 0 1.5px color-mix(in oklch, var(--accent) 35%, transparent); }
    50%      { box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 12%, transparent),
                            0 0 0 1.5px color-mix(in oklch, var(--accent) 70%, transparent); }
  }

  .step-body {
    min-width: 0;
    display: flex; flex-direction: column;
    gap: 3px;
  }
  .step-head-row {
    display: flex; align-items: center; gap: 8px;
    min-width: 0;
    border-radius: 4px;
    padding: 1px 0;
  }
  .step[data-collapsible="true"] .step-head-row {
    cursor: pointer;
    margin: -1px -4px;
    padding: 1px 4px;
    transition: background 140ms ease-out;
  }
  .step[data-collapsible="true"] .step-head-row:hover {
    background: color-mix(in oklch, var(--surface-hover) 70%, transparent);
  }
  .step[data-collapsible="true"] .step-head-row:focus-visible {
    outline: 2px solid color-mix(in oklch, var(--accent) 45%, transparent);
    outline-offset: 1px;
  }
  .step-head {
    flex: 1; min-width: 0;
    font-size: var(--fs-sm);
    font-weight: 500;
    color: var(--fg-2);
    line-height: 1.4;
    letter-spacing: 0.005em;
  }
  .step[data-collapsible="true"][data-expanded="false"] .step-head {
    color: var(--fg-muted);
  }
  .child-summary {
    font-size: 10px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    padding: 1px 7px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 50%, transparent);
    flex-shrink: 0;
    letter-spacing: 0.02em;
    animation: badge-in 200ms ease-out;
  }
  @keyframes badge-in {
    from { opacity: 0; transform: scale(0.92); }
    to   { opacity: 1; transform: scale(1); }
  }
  .step-chev {
    display: inline-flex;
    color: var(--fg-faint);
    flex-shrink: 0;
    transition: transform 200ms cubic-bezier(0.22, 1, 0.36, 1), color 140ms ease-out;
  }
  .step-chev.open { transform: rotate(90deg); color: var(--fg-muted); }
  .step[data-collapsible="true"] .step-head-row:hover .step-chev { color: var(--fg-2); }

  /* Collapsible children — `grid-template-rows: 1fr ↔ 0fr` gives a smooth,
     content-aware height animation. Overflow hidden on the inner so chips
     don't escape the collapsed envelope mid-transition. */
  .children-wrap {
    display: grid;
    grid-template-rows: 1fr;
    transition: grid-template-rows 280ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .step[data-expanded="false"] .children-wrap {
    grid-template-rows: 0fr;
  }
  .children-inner {
    min-height: 0;
    overflow: hidden;
    transition: opacity 220ms ease-out, padding-top 280ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .step[data-expanded="false"] .children-inner {
    opacity: 0;
    padding-top: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .step, .step[data-status="pending"] { animation: none; }
    .children-wrap, .children-inner, .step-chev { transition: none; }
    .child-summary { animation: none; }
  }
  /* Tighten Markdown's <p> spacing inside the step head — the prose is
     short, full <p> margins would push the children too far down. */
  .step-head :global(p) { margin: 0; }
  .step-head :global(strong) { font-weight: 700; }

  .step-children {
    display: flex; flex-direction: column;
    gap: 3px;
    padding-top: 2px;
  }
  /* Tighten any prose Markdown inside the children — keep it visually
     subordinate to the step heading. */
  .step-children :global(p) {
    margin: 4px 0;
    color: var(--fg-2);
    font-size: var(--fs-sm);
    line-height: 1.5;
  }
  .step-children :global(p:first-child) { margin-top: 0; }
  .step-children :global(p:last-child) { margin-bottom: 0; }

  .mono { font-family: var(--font-mono, monospace); }
</style>
