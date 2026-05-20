<script lang="ts">
  // Live-status hub — surfaces `assistant.activity.currentLabel` + elapsed
  // turn time + a Stop button as a single strip above the Composer. Replaces
  // the per-bubble stream-status: that row was easy to miss when the user was
  // scrolled near the input. Renders only while the active tab is streaming.

  import { Loader2, Square } from "lucide-svelte";
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

  const label = $derived.by<string | null>(() => {
    if (!streaming) return null;
    const raw = currentLabel ?? "Thinking…";
    if (/^thinking/i.test(raw)) return `${WHIM_WORDS[whimTick]}…`;
    return raw;
  });

  const isShell = $derived(label != null && /^\$\s/.test(label));

  function onStop() {
    // In split-pane mode, stop targets the pane's tab via focus. Single-pane
    // falls through to the active-tab stop.
    if (tabId && assistant.splitActive) {
      // Focus the pane that owns this tab so stop() hits the right session.
      const idx = assistant.panes.findIndex((p) => p.tabId === tabId);
      if (idx !== -1) assistant.setFocusedPane(idx);
    }
    void assistant.stop();
  }
</script>

{#if streaming && label}
  <div class="hub" role="status" aria-live="polite">
    <Loader2 size={12} class="spin" />
    {#key label}
      <span class="hub-label" class:mono={isShell}>{label}</span>
    {/key}
    {#if elapsed}
      <span class="hub-elapsed" title="Elapsed since turn started">{elapsed}</span>
    {/if}
    <button class="hub-stop" type="button" onclick={onStop} title="Stop this turn">
      <Square size={10} />
      <span>Stop</span>
    </button>
  </div>
{/if}

<style>
  .hub {
    flex-shrink: 0;
    display: flex; align-items: center; gap: 10px;
    padding: 7px 12px;
    margin: 6px auto 0;
    max-width: min(960px, 88ch);
    width: calc(100% - 36px);
    background: color-mix(in oklch, var(--accent) 8%, var(--surface));
    border: 1px solid color-mix(in oklch, var(--accent) 30%, var(--border));
    border-radius: 8px;
    font-size: var(--fs-sm);
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
    color: var(--fg);
    animation: hub-fade 260ms ease-out;
  }
  .hub-label.mono {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
  }
  @keyframes hub-fade {
    from { opacity: 0; transform: translateY(-2px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .hub-elapsed {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .hub-stop {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 4px 10px;
    background: transparent;
    border: 1px solid color-mix(in oklch, var(--danger) 30%, var(--border));
    border-radius: 5px;
    color: var(--fg-2);
    font: inherit;
    font-size: var(--fs-xs);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .hub-stop:hover {
    background: color-mix(in oklch, var(--danger) 18%, transparent);
    color: var(--danger);
    border-color: color-mix(in oklch, var(--danger) 55%, var(--border));
  }
  @keyframes hub-in {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .hub, .hub-label, .hub :global(.spin) { animation: none; }
  }
</style>
