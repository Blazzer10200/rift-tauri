<script lang="ts">
  import { slide } from "svelte/transition";
  import { ChevronRight } from "lucide-svelte";
  import { VERB_PAST, VERB_ING, type StreamTool } from "./streamModel";
  import EditDiff from "../EditDiff.svelte";
  import AnimatedCount from "./AnimatedCount.svelte";

  let { tools }: { tools: StreamTool[] } = $props();

  const reduced =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  // Each row can expand to a compact code-diff preview (the real lines that
  // changed), not just a "made an edit" stub. Small edits auto-open; large ones
  // stay folded behind the +N/−M summary. Keyed by tool id so a user's manual
  // toggle survives re-renders.
  let openMap = $state<Record<string, boolean>>({});
  const SMALL = 14;

  function hasDiff(t: StreamTool): boolean {
    return !!t.input && (t.add != null || t.del != null);
  }
  function autoOpen(t: StreamTool): boolean {
    return (t.add ?? 0) + (t.del ?? 0) <= SMALL;
  }
  function isOpen(t: StreamTool): boolean {
    return openMap[t.id] ?? autoOpen(t);
  }
  function toggle(t: StreamTool) {
    openMap = { ...openMap, [t.id]: !isOpen(t) };
  }
</script>

<div class="wbatch">
  {#each tools as t (t.id)}
    {@const live = t.status === "pending"}
    {@const open = hasDiff(t) && isOpen(t)}
    <div class="wbrow {live ? 'active' : ''}">
      {#if hasDiff(t)}
        <button class="wb-chev" class:open type="button" onclick={() => toggle(t)} aria-label={open ? "Hide diff" : "Show diff"}>
          <ChevronRight size={12} strokeWidth={2.2} />
        </button>
      {/if}
      <span class="wb-verb">{live ? VERB_ING[t.kind] : VERB_PAST[t.kind]}</span>
      <span class="wb-label" title={t.path ?? t.cap}>
        {#if t.dir}<span class="wb-dir">{t.dir}</span>{/if}<span class="wb-name">{t.cap}</span>
      </span>
      {#if t.add != null || t.del != null}
        <span class="wb-diff">
          {#if t.add != null}
            <span class="wb-add">+<AnimatedCount value={t.add} live={live && !reduced} /></span>
          {/if}
          {#if t.del != null}
            <span class="wb-del {t.del > 0 ? 'real' : ''}">−<AnimatedCount value={t.del} live={live && !reduced} /></span>
          {/if}
        </span>
      {/if}
      <span class="wb-dot {live ? 'live' : ''}"></span>
    </div>
    {#if open && t.input}
      <div class="wb-diffwrap" transition:slide={{ duration: reduced ? 0 : 200 }}>
        <EditDiff input={t.input} compact defaultExpanded hideHead />
      </div>
    {/if}
  {/each}
</div>

<style>
  .wb-chev {
    display: inline-flex; padding: 0; margin-right: -3px;
    border: 0; background: transparent;
    color: var(--fg-faint);
    flex: none; cursor: pointer;
    transition: transform 140ms ease, color 140ms ease;
  }
  .wb-chev:hover { color: var(--fg-2); }
  .wb-chev.open { transform: rotate(90deg); }
  .wb-chev:focus-visible {
    outline: 2px solid color-mix(in oklab, var(--accent) 60%, transparent);
    outline-offset: 2px; border-radius: 4px;
  }
  .wb-label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .wb-dir { color: var(--fg-faint); opacity: 0.6; font-family: var(--font-mono); font-size: 11px; }
  .wb-diffwrap {
    margin: 2px 0 8px 2px;
    border-left: 2px solid color-mix(in oklch, var(--border) 80%, transparent);
    padding-left: 10px;
  }
  @media (prefers-reduced-motion: reduce) { .wb-chev { transition: color 140ms ease; } }
</style>
