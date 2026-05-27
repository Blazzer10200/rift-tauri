<script lang="ts">
  import { onMount } from "svelte";
  import { assistant } from "../../state/assistant.svelte";
  import AssistantPane from "./AssistantPane.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  onMount(() => {
    void assistant.init().then(() => {
      if (assistant.openTabs.length === 0) void assistant.newTab();
    });
  });

  // Phase D + split-pane v2.1: auto-compact iterates EVERY pane so a
  // background-pane tab can't sail past threshold silently. Guards per tab:
  //   - threshold non-null (feature opt-in, global)
  //   - tab exists + not streaming + not already compacting
  //   - ≥5min since last successful compaction (cooldown vs runaway on failure)
  //   - that tab's ctxPct has crossed threshold
  // Page-scoped so the effect lives only while the chat workspace is mounted;
  // navigating to Sync/Settings pauses auto-trigger naturally.
  $effect(() => {
    const threshold = assistant.autoCompactThreshold;
    if (!threshold) return;
    const threshPct = threshold * 100;
    const now = Date.now();
    for (const p of assistant.panes) {
      if (!p.tabId) continue;
      const tab = assistant.tabFor(p.tabId);
      if (!tab) continue;
      if (tab.streaming || tab.compactingNow) continue;
      if (now - tab.lastCompactionAt < 5 * 60_000) continue;
      if (assistant.ctxPctFor(tab) < threshPct) continue;
      void assistant.compactConversation(undefined, p.tabId);
    }
  });

  // ── Resizable dividers ────────────────────────────────────────────────────
  // Per pane-count storage. fracs[i] is pane i's share; sum ≈ 1. Drag on the
  // divider btw pane i and i+1 redistributes between those two only.
  const SPLIT_MIN = 0.15;
  const STORE_KEY = (n: number) => `rift.assistant.splitFracs.${n}`;
  let fracs = $state<number[]>([1]);
  let splitEl = $state<HTMLDivElement | undefined>();
  let dragging = $state(false);
  let dragIdx = $state<number | null>(null);

  function loadFracs(n: number): number[] {
    try {
      const saved = localStorage.getItem(STORE_KEY(n));
      if (saved) {
        const arr = JSON.parse(saved);
        if (Array.isArray(arr) && arr.length === n
            && arr.every((v) => typeof v === "number" && v >= SPLIT_MIN)) {
          const sum = arr.reduce((a: number, b: number) => a + b, 0);
          if (sum > 0) return arr.map((v: number) => v / sum);
        }
      }
    } catch { /* noop */ }
    return Array(n).fill(1 / n);
  }
  function persistFracs() {
    try { localStorage.setItem(STORE_KEY(fracs.length), JSON.stringify(fracs)); } catch { /* noop */ }
  }

  $effect(() => {
    const n = assistant.panes.length || 1;
    if (fracs.length !== n) fracs = loadFracs(n);
  });

  const gridTemplate = $derived.by(() => {
    const parts: string[] = [];
    for (let i = 0; i < fracs.length; i++) {
      parts.push(`${fracs[i]}fr`);
      if (i < fracs.length - 1) parts.push("6px");
    }
    return parts.join(" ");
  });

  function onDividerPointerDown(e: PointerEvent, i: number) {
    e.preventDefault();
    dragging = true;
    dragIdx = i;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function onDividerPointerMove(e: PointerEvent, i: number) {
    if (!dragging || dragIdx !== i || !splitEl) return;
    const rect = splitEl.getBoundingClientRect();
    if (rect.width <= 0) return;
    // Compute the cursor position as a fraction of the split. Map it onto the
    // boundary between pane i and i+1 — everything left of cursor is the new
    // cumulative share through pane i; everything right is the rest. We only
    // redistribute the (fracs[i] + fracs[i+1]) bucket so other panes stay put.
    let leftSum = 0;
    for (let k = 0; k < i; k++) leftSum += fracs[k];
    let rightSum = 0;
    for (let k = i + 2; k < fracs.length; k++) rightSum += fracs[k];
    const bucket = 1 - leftSum - rightSum;
    if (bucket <= 0) return;
    const cursorFrac = (e.clientX - rect.left) / rect.width;
    let newLeft = cursorFrac - leftSum;
    const minA = SPLIT_MIN;
    const minB = SPLIT_MIN;
    if (newLeft < minA) newLeft = minA;
    if (newLeft > bucket - minB) newLeft = bucket - minB;
    const next = fracs.slice();
    next[i] = newLeft;
    next[i + 1] = bucket - newLeft;
    fracs = next;
  }
  function onDividerPointerUp(e: PointerEvent, i: number) {
    if (!dragging || dragIdx !== i) return;
    dragging = false;
    dragIdx = null;
    try { (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId); } catch { /* noop */ }
    persistFracs();
  }
  function resetSplit() {
    const n = fracs.length;
    fracs = Array(n).fill(1 / n);
    try { localStorage.removeItem(STORE_KEY(n)); } catch { /* noop */ }
  }
  function onDividerKey(e: KeyboardEvent, i: number) {
    const step = e.shiftKey ? 0.05 : 0.02;
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      const dir = e.key === "ArrowLeft" ? -1 : 1;
      const bucket = fracs[i] + fracs[i + 1];
      let newLeft = fracs[i] + dir * step;
      if (newLeft < SPLIT_MIN) newLeft = SPLIT_MIN;
      if (newLeft > bucket - SPLIT_MIN) newLeft = bucket - SPLIT_MIN;
      const next = fracs.slice();
      next[i] = newLeft;
      next[i + 1] = bucket - newLeft;
      fracs = next;
      persistFracs();
    } else if (e.key === "Home" || e.key === "Enter") {
      e.preventDefault();
      resetSplit();
    }
  }
</script>

<div class="assistant">
  <div class="layout">
    {#if assistant.splitActive}
      <div
        class="split"
        bind:this={splitEl}
        data-dragging={dragging}
        style="grid-template-columns: {gridTemplate};"
      >
        {#each assistant.panes as p, i (i)}
          <AssistantPane
            tabId={p.tabId}
            focused={assistant.focusedPaneIdx === i}
            paneIdx={i}
          />
          {#if i < assistant.panes.length - 1}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <div
              class="divider"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize panes (drag, or use arrow keys; double-click or Home to reset)"
              aria-valuenow={Math.round(fracs[i] * 100)}
              aria-valuemin={Math.round(SPLIT_MIN * 100)}
              aria-valuemax={Math.round((1 - SPLIT_MIN) * 100)}
              tabindex="0"
              use:tooltip={"Drag to resize · double-click to reset"}
              onpointerdown={(e) => onDividerPointerDown(e, i)}
              onpointermove={(e) => onDividerPointerMove(e, i)}
              onpointerup={(e) => onDividerPointerUp(e, i)}
              onpointercancel={(e) => onDividerPointerUp(e, i)}
              ondblclick={resetSplit}
              onkeydown={(e) => onDividerKey(e, i)}
            ><span class="divider-grip" aria-hidden="true"></span></div>
          {/if}
        {/each}
      </div>
    {:else}
      <AssistantPane
        tabId={assistant.currentConvoId}
        focused={true}
        paneIdx={0}
      />
    {/if}
  </div>
</div>

<style>
  .assistant {
    flex: 1;
    display: flex; flex-direction: column;
    min-height: 0;
    background: var(--bg);
    color: var(--fg);
  }
  .layout {
    flex: 1; min-height: 0;
    display: flex;
    overflow: hidden;
    position: relative;
  }
  .split {
    flex: 1; min-width: 0; min-height: 0;
    display: grid;
    overflow: hidden;
  }
  .divider {
    position: relative;
    cursor: col-resize;
    background: var(--border);
    align-self: stretch;
    user-select: none;
    transition: background 120ms ease-out;
  }
  .divider:hover,
  .split[data-dragging="true"] > .divider {
    background: color-mix(in oklch, var(--accent) 50%, var(--border));
  }
  .divider:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .divider-grip {
    position: absolute;
    top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    width: 2px; height: 24px;
    background: var(--fg-faint);
    border-radius: 2px;
    opacity: 0;
    transition: opacity 120ms ease-out;
  }
  .divider:hover .divider-grip,
  .split[data-dragging="true"] .divider-grip { opacity: 0.7; }
</style>
