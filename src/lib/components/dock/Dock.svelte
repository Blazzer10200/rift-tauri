<script lang="ts">
  import type { DockSlot, PanelId } from "$lib/state/panel-types";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import PanelShell from "./PanelShell.svelte";

  // All 8 panels render their headers at all times under v0.3. `open` now
  // means "body expanded" rather than "in dock at all." Sort by `order` so
  // user-arranged drag-reorder still works, then split by slot for v0.4.
  const orderedPanels = $derived(
    (Object.entries(uiPrefs.panels) as [PanelId, { order: number; slot: DockSlot }][])
      .sort((a, b) => a[1].order - b[1].order),
  );
  const leftStack = $derived(orderedPanels.filter(([, st]) => st.slot === "left").map(([id]) => id));
  const rightStack = $derived(orderedPanels.filter(([, st]) => st.slot === "right").map(([id]) => id));
  const rightOccupied = $derived(rightStack.length > 0);

  // Maximized state is mirrored in main column. Dock still renders the head
  // (collapsed body) so the user can swap focus by clicking another ⛶.
  const maximizedId = $derived(uiPrefs.maximized);

  // ── Drag-to-reorder / cross-slot ──────────────────────────────────
  let draggingId = $state<PanelId | null>(null);
  let dragOverId = $state<PanelId | null>(null);
  let dragOverSlot = $state<DockSlot | null>(null);

  function onPanelDragStart(id: PanelId, ev: DragEvent) {
    draggingId = id;
    if (ev.dataTransfer) {
      ev.dataTransfer.effectAllowed = "move";
      ev.dataTransfer.setData("text/x-rift-panel", id);
    }
  }
  function onPanelDragOver(id: PanelId, ev: DragEvent) {
    if (!draggingId || draggingId === id) return;
    ev.preventDefault();
    if (ev.dataTransfer) ev.dataTransfer.dropEffect = "move";
    dragOverId = id;
    dragOverSlot = uiPrefs.panels[id].slot;
  }
  function onPanelDrop(id: PanelId, ev: DragEvent) {
    ev.preventDefault();
    if (!draggingId || draggingId === id) { resetDrag(); return; }
    const targetSlot = uiPrefs.panels[id].slot;
    const sourceSlot = uiPrefs.panels[draggingId].slot;
    if (sourceSlot !== targetSlot) {
      // Cross-slot drop: move into target slot, append-to-end semantics handled
      // by setPanelSlot; ignore intra-slot ordering of target since slot move
      // already places it after existing target-slot panels.
      uiPrefs.setPanelSlot(draggingId, targetSlot);
    } else {
      // Same-slot reorder: target's current position.
      const stack = targetSlot === "left" ? leftStack : rightStack;
      const order = uiPrefs.panels[stack[stack.indexOf(id)]].order;
      uiPrefs.reorderPanel(draggingId, order);
    }
    resetDrag();
  }
  function onPanelDragEnd() { resetDrag(); }
  function onSlotDragOver(slot: DockSlot, ev: DragEvent) {
    if (!draggingId) return;
    ev.preventDefault();
    if (ev.dataTransfer) ev.dataTransfer.dropEffect = "move";
    dragOverSlot = slot;
    dragOverId = null;
  }
  function onSlotDrop(slot: DockSlot, ev: DragEvent) {
    ev.preventDefault();
    if (!draggingId) return;
    if (uiPrefs.panels[draggingId].slot !== slot) {
      uiPrefs.setPanelSlot(draggingId, slot);
    } else {
      // Append to end of same slot.
      uiPrefs.reorderPanel(draggingId, orderedPanels.length);
    }
    resetDrag();
  }
  function resetDrag() { draggingId = null; dragOverId = null; dragOverSlot = null; }

  // ── Width-resize handle (outer left edge) ──────────────────────────
  let resizing = $state(false);
  let startX = 0;
  let startW = 0;
  let pendingW = 0;
  let rafId = 0;

  function flushWidth() {
    rafId = 0;
    uiPrefs.setDockWidthLive(pendingW);
  }

  function onWidthPointerDown(ev: PointerEvent) {
    resizing = true;
    startX = ev.clientX;
    startW = uiPrefs.dockWidth;
    pendingW = startW;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    document.body.style.cursor = "ew-resize";
    document.body.style.userSelect = "none";
    ev.preventDefault();
  }
  function onWidthPointerMove(ev: PointerEvent) {
    if (!resizing) return;
    pendingW = startW + (startX - ev.clientX);
    if (rafId === 0) rafId = requestAnimationFrame(flushWidth);
  }
  function onWidthPointerUp(ev: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    if (rafId !== 0) { cancelAnimationFrame(rafId); rafId = 0; }
    uiPrefs.setDockWidthLive(pendingW);
    uiPrefs.persistDockWidth();
    (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }
  function onWidthDblClick() {
    // v0.4 — snap to ~half viewport, clamped by clampWidth.
    if (typeof window === "undefined") return;
    uiPrefs.setDockWidth(window.innerWidth / 2);
  }

  // ── Internal split-resize handle ───────────────────────────────────
  let splitResizing = $state(false);
  let splitStartX = 0;
  let splitStartPct = 0;
  let splitDockEl = $state<HTMLDivElement | undefined>();
  let splitPending = 0;
  let splitRaf = 0;

  function flushSplit() {
    splitRaf = 0;
    uiPrefs.setDockSplitLive(splitPending);
  }
  function onSplitPointerDown(ev: PointerEvent) {
    if (!splitDockEl) return;
    splitResizing = true;
    splitStartX = ev.clientX;
    splitStartPct = uiPrefs.dockSplitPct;
    splitPending = splitStartPct;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    document.body.style.cursor = "ew-resize";
    document.body.style.userSelect = "none";
    ev.preventDefault();
  }
  function onSplitPointerMove(ev: PointerEvent) {
    if (!splitResizing || !splitDockEl) return;
    const rect = splitDockEl.getBoundingClientRect();
    if (rect.width <= 0) return;
    const deltaPct = ((ev.clientX - splitStartX) / rect.width) * 100;
    splitPending = splitStartPct + deltaPct;
    if (splitRaf === 0) splitRaf = requestAnimationFrame(flushSplit);
  }
  function onSplitPointerUp(ev: PointerEvent) {
    if (!splitResizing) return;
    splitResizing = false;
    if (splitRaf !== 0) { cancelAnimationFrame(splitRaf); splitRaf = 0; }
    uiPrefs.setDockSplitLive(splitPending);
    uiPrefs.persistDockSplit();
    (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }
  function onSplitDblClick() {
    uiPrefs.setDockSplitPct(50);
  }
</script>

<aside class="dock" aria-label="Dock panels">
  <div
    class="dock-resize"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize dock width"
    aria-valuenow={uiPrefs.dockWidth}
    data-active={resizing}
    onpointerdown={onWidthPointerDown}
    onpointermove={onWidthPointerMove}
    onpointerup={onWidthPointerUp}
    onpointercancel={onWidthPointerUp}
    ondblclick={onWidthDblClick}
  ></div>

  <div
    class="dock-body"
    bind:this={splitDockEl}
    data-split={rightOccupied}
  >
    <!-- LEFT slot -->
    <div
      class="slot slot-left"
      data-slot="left"
      data-drag-over={dragOverSlot === "left" && draggingId !== null && uiPrefs.panels[draggingId].slot !== "left"}
      role="region"
      aria-label="Left dock slot"
      ondragover={(e) => onSlotDragOver("left", e)}
      ondrop={(e) => onSlotDrop("left", e)}
    >
      {#each leftStack as id (id)}
        <div
          class="panel-slot"
          data-dragging={draggingId === id}
          data-drag-over={dragOverId === id}
        >
          <PanelShell
            {id}
            isMaximized={maximizedId === id}
            onDragStart={onPanelDragStart}
            onDragOver={onPanelDragOver}
            onDrop={onPanelDrop}
            onDragEnd={onPanelDragEnd}
          />
        </div>
      {/each}
      {#if draggingId && uiPrefs.panels[draggingId].slot !== "left"}
        <div class="slot-drop-hint">Drop here → Left slot</div>
      {/if}
    </div>

    {#if rightOccupied}
      <!-- Internal split-resize handle -->
      <div
        class="split-handle"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize dock split"
        aria-valuenow={uiPrefs.dockSplitPct}
        aria-valuemin={20}
        aria-valuemax={80}
        data-active={splitResizing}
        onpointerdown={onSplitPointerDown}
        onpointermove={onSplitPointerMove}
        onpointerup={onSplitPointerUp}
        onpointercancel={onSplitPointerUp}
        ondblclick={onSplitDblClick}
      ></div>

      <!-- RIGHT slot -->
      <div
        class="slot slot-right"
        data-slot="right"
        data-drag-over={dragOverSlot === "right" && draggingId !== null && uiPrefs.panels[draggingId].slot !== "right"}
        role="region"
        aria-label="Right dock slot"
        ondragover={(e) => onSlotDragOver("right", e)}
        ondrop={(e) => onSlotDrop("right", e)}
      >
        {#each rightStack as id (id)}
          <div
            class="panel-slot"
            data-dragging={draggingId === id}
            data-drag-over={dragOverId === id}
          >
            <PanelShell
              {id}
              isMaximized={maximizedId === id}
              onDragStart={onPanelDragStart}
              onDragOver={onPanelDragOver}
              onDrop={onPanelDrop}
              onDragEnd={onPanelDragEnd}
            />
          </div>
        {/each}
      </div>
    {:else if draggingId && uiPrefs.panels[draggingId].slot === "left"}
      <!-- Right slot collapsed but visible during drag so user can drop into it -->
      <div
        class="slot slot-right slot-empty-target"
        data-slot="right"
        data-drag-over={dragOverSlot === "right"}
        role="region"
        aria-label="Right dock slot (empty drop target)"
        ondragover={(e) => onSlotDragOver("right", e)}
        ondrop={(e) => onSlotDrop("right", e)}
      >
        <div class="slot-drop-hint">Drop here → New right slot</div>
      </div>
    {/if}
  </div>
</aside>

<style>
  .dock {
    position: relative;
    display: flex; flex-direction: column;
    height: 100%;
    background: var(--bg-elev-1);
    border-left: 1px solid var(--border);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .dock-resize {
    position: absolute;
    left: -5px; top: 0; bottom: 0;
    width: 10px;
    background: transparent;
    cursor: ew-resize;
    z-index: 20;
    touch-action: none;
  }
  .dock-resize::after {
    content: "";
    position: absolute;
    left: 5px; top: 0; bottom: 0;
    width: 2px;
    background: transparent;
    transition: background 120ms ease;
    pointer-events: none;
  }
  .dock-resize:hover::after,
  .dock-resize[data-active="true"]::after { background: var(--accent); }

  .dock-body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 1fr;
    overflow: hidden;
  }
  .dock-body[data-split="true"] {
    grid-template-columns: var(--dock-split-pct, 50%) 4px minmax(0, 1fr);
  }

  .slot {
    min-height: 0;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex; flex-direction: column;
    position: relative;
  }
  .slot-left { grid-column: 1; }
  .slot-right { grid-column: 3; }
  /* Cross-slot drag highlight — soft tinted background + accent ring. */
  .slot[data-drag-over="true"] {
    background: color-mix(in oklch, var(--accent) 6%, transparent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .slot-empty-target {
    grid-column: 3;
    background: color-mix(in oklch, var(--accent) 4%, transparent);
    border-left: 1px dashed color-mix(in oklch, var(--accent) 40%, var(--border));
    display: flex; align-items: center; justify-content: center;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
  }
  .slot-drop-hint {
    margin: 8px;
    padding: 6px 10px;
    text-align: center;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
    border: 1px dashed color-mix(in oklch, var(--accent) 40%, var(--border));
    border-radius: var(--radius-sm);
    background: color-mix(in oklch, var(--accent) 6%, transparent);
    flex-shrink: 0;
  }

  .panel-slot {
    display: flex; flex-direction: column;
    min-height: 0;
    transition: opacity 120ms ease, box-shadow 120ms ease;
  }
  .panel-slot[data-dragging="true"] {
    opacity: 0.4;
  }
  .panel-slot[data-drag-over="true"] {
    box-shadow: inset 0 2px 0 0 var(--accent);
  }

  .split-handle {
    grid-column: 2;
    position: relative;
    background: var(--border);
    cursor: ew-resize;
    z-index: 10;
    touch-action: none;
  }
  .split-handle::before {
    content: "";
    position: absolute;
    inset: 0 -3px;
    background: transparent;
  }
  .split-handle::after {
    content: "";
    position: absolute;
    left: 1px; top: 0; bottom: 0;
    width: 2px;
    background: transparent;
    transition: background 120ms ease;
    pointer-events: none;
  }
  .split-handle:hover::after,
  .split-handle[data-active="true"]::after { background: var(--accent); }
</style>
