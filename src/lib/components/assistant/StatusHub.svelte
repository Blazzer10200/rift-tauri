<script lang="ts">
  // Live-status hub — surfaces `assistant.activity.currentLabel` + elapsed
  // turn time as a thin row attached to the Composer's top edge. Stop is
  // handled by the Composer's Send→Stop button morph (no duplicate affordance).
  // Renders only while the active tab is streaming.

  import { Loader2 } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";

  // Optional tabId — when omitted, falls back to the active tab (single-pane
  // mode compat). Split-pane mode passes the pane's tabId so each pane shows
  // its own stream status independently.
  let { tabId = null }: { tabId?: string | null } = $props();
  const tab = $derived(tabId !== null ? assistant.tabFor(tabId) : null);
  const streaming = $derived(tab ? tab.streaming : assistant.streaming);
  const turnStartedAt = $derived(tab ? tab.activity.turnStartedAt : assistant.activity.turnStartedAt);
  const currentLabel = $derived(tab ? tab.activity.currentLabel : assistant.activity.currentLabel);

  let tickNow = $state(Date.now());
  let tickHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if (streaming) {
      if (!tickHandle) tickHandle = setInterval(() => (tickNow = Date.now()), 500);
    } else if (tickHandle) {
      clearInterval(tickHandle);
      tickHandle = null;
    }
    return () => {
      if (tickHandle) { clearInterval(tickHandle); tickHandle = null; }
    };
  });

  const WHIM_WORDS = [
    "Thinking", "Sussing", "Spelunking", "Pondering", "Brewing",
    "Reckoning", "Mulling", "Cogitating", "Hatching", "Conjuring",
    "Noodling", "Untangling",
  ];
  let whimTick = $state(0);
  $effect(() => {
    if (!streaming) return;
    const id = setInterval(() => (whimTick = (whimTick + 1) % WHIM_WORDS.length), 2400);
    return () => clearInterval(id);
  });

  const elapsed = $derived.by<string | null>(() => {
    if (!streaming) return null;
    const start = turnStartedAt;
    if (!start) return null;
    void tickNow;
    const s = Math.floor((Date.now() - start) / 1000);
    return s < 60 ? `${s}s` : `${Math.floor(s / 60)}m ${s % 60}s`;
  });

  // Truncate verbose paths from the LEFT so the useful tail (filename + parent
   // dir) stays visible. Mirrors EditDiff's `shortPath` — long FS paths in a
   // status row used to cut off mid-bracket and lose the meaningful end. Path
   // detection is a light heuristic: drive letter / leading slash / `://`.
  function shortenPaths(s: string): string {
    return s.replace(/([A-Za-z]:[\\/][^\s]+|\/[^\s]+|[a-z]+:\/\/[^\s]+)/g, (m) => {
      const parts = m.replace(/\\/g, "/").split("/").filter(Boolean);
      if (parts.length <= 3) return m;
      return ".../" + parts.slice(-3).join("/");
    });
  }

  const label = $derived.by<string | null>(() => {
    if (!streaming) return null;
    const raw = currentLabel ?? "Thinking…";
    if (/^thinking/i.test(raw)) return `${WHIM_WORDS[whimTick]}…`;
    return shortenPaths(raw);
  });

  const isShell = $derived(label != null && /^\$\s/.test(label));
</script>

{#if streaming && label}
  <div class="hub" role="status" aria-live="polite">
    <Loader2 size={11} class="spin" />
    {#key label}
      <span class="hub-label" class:mono={isShell}>{label}</span>
    {/key}
    {#if elapsed}
      <span class="hub-elapsed" title="Elapsed since turn started">{elapsed}</span>
    {/if}
  </div>
{/if}

<style>
  /* Renders inside the Composer's .composer-shell directly above .composer.
     Visually attached: same horizontal padding as the composer box, only
     top corners rounded, no bottom border (so the seam between hub and
     composer reads as one element). Composer-side complement (`:has(.hub)`)
     squares the composer's top corners. */
  .hub {
    flex-shrink: 0;
    display: flex; align-items: center; gap: 8px;
    padding: 5px 12px;
    background: color-mix(in oklch, var(--accent) 6%, var(--surface));
    border: 1px solid var(--border);
    border-bottom: 0;
    border-radius: 13px 13px 0 0;
    /* Inset 1px so the side borders align flush with the composer box below. */
    margin: 0 1px -1px;
    font-size: var(--fs-xs);
    color: var(--fg-2);
    animation: hub-in 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .hub :global(.spin) {
    color: var(--accent);
    animation: hub-spin 0.9s linear infinite;
    flex-shrink: 0;
  }
  @keyframes hub-spin { to { transform: rotate(360deg); } }
  .hub-label {
    flex: 1; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: var(--fg-2);
    animation: hub-fade 260ms ease-out;
  }
  .hub-label.mono {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
  }
  @keyframes hub-fade {
    from { opacity: 0; transform: translateY(-1px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .hub-elapsed {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  @keyframes hub-in {
    from { opacity: 0; transform: translateY(2px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .hub, .hub-label, .hub :global(.spin) { animation: none; }
  }
</style>
