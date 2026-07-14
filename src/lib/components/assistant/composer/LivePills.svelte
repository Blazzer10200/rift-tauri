<script lang="ts">
  // C4 (per docs/design/composer-split.md) — the toolbar's middle slot, lifted
  // verbatim from Composer.svelte 2026-06-10: live-activity pills while a turn
  // runs (elapsed · tok/s · agents · shells · tools · queued) and the ↵/⇧↵
  // keyboard hint while idle-focused. Talks to the assistant store directly
  // (telemetry snapshot + Activity dock toggle — the cluster already reads it
  // pervasively, per the brief's QueueRail precedent).
  import { ListPlus } from "lucide-svelte";
  import { tooltip } from "$lib/actions/tooltip";

  let {
    queue,
  }: {
    queue: { id: string; text: string }[];
  } = $props();

  // The in-flight live-status (agents · shells · tools · elapsed · tokens) now
  // lives in StreamTurn's inline footer, under the turn's "Working…" head —
  // matching the DS reference (app/stream.jsx StreamFooter). The old toolbar
  // count pills duplicated that and popped opaque badges near the composer on
  // every bash/PowerShell run, so they were dropped (2026-06-21). This slot now
  // keeps only the genuinely-distinct "N queued" signal + the idle keyboard hint.
  const showLivePills = $derived(queue.length > 0);
</script>

{#if showLivePills}
  <div class="live-pills" role="group" aria-label="Live turn activity">
    {#if queue.length > 0}
      <span class="live-pill queued" use:tooltip={`${queue.length} message${queue.length === 1 ? "" : "s"} queued to send after this turn.`}>
        <ListPlus size={12} />
        <span class="mono">{queue.length}</span>
      </span>
    {/if}
  </div>
{/if}
<!-- The ↵/⇧↵ keyboard hint retired 2026-07-02 — a learned-once fact shouldn't
     be permanent chrome; it lives in the send button's tooltip now. -->

<style>
  /* One neutral capsule (same surface as the settings pill) for the lone
     "N queued" readout — the agents/shells/tools counts moved to StreamTurn's
     inline footer, so this slot stays quiet and blends into the toolbar. */
  .live-pills {
    display: inline-flex; align-items: center;
    height: 26px;
    padding: 0 2px;
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 65%, transparent);
    border-radius: 999px;
    min-width: 0;
    animation: enter 180ms ease-out;
  }
  .live-pill {
    display: inline-flex; align-items: center; gap: 5px;
    height: 100%; padding: 0 9px;
    font: inherit; font-size: 11px; font-weight: 600; line-height: 1;
    color: var(--fg-muted);
    background: transparent;
    border: 0;
    border-radius: 999px;
    flex-shrink: 0;
    transition: color var(--dur-fast) ease-out;
  }
  .live-pill:hover { color: var(--fg); }
  .live-pill :global(svg) { color: var(--fg-faint); transition: color var(--dur-fast) ease-out; }
  .live-pill:hover :global(svg) { color: var(--fg-muted); }
  .live-pill .mono { font-variant-numeric: tabular-nums; color: var(--fg-2); }
</style>
