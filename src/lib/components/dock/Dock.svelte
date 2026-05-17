<script lang="ts">
  import type { PanelId } from "$lib/state/panel-types";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import PanelShell from "./PanelShell.svelte";

  // All 8 panels render their headers at all times under v0.3. `open` now
  // means "body expanded" rather than "in dock at all." Sort by `order` so
  // user-arranged drag-reorder still works.
  const panelStack = $derived(
    (Object.entries(uiPrefs.panels) as [PanelId, { order: number }][])
      .sort((a, b) => a[1].order - b[1].order)
      .map(([id]) => id),
  );

  // Maximized state is mirrored in main column. Dock still renders the head
  // (collapsed body) so the user can swap focus by clicking another ⛶.
  const maximizedId = $derived(uiPrefs.maximized);

  // ── Drag-to-reorder ────────────────────────────────────────────────
  // HTML5 native drag. Source = panel header (draggable=true on PanelShell).
  // Target = panel we drop onto, OR the bottom drop zone (append-to-end).
  let draggingId = $state<PanelId | null>(null);
  let dragOverId = $state<PanelId | null>(null);

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
  }
  function onPanelDrop(id: PanelId, ev: DragEvent) {
    ev.preventDefault();
    if (!draggingId || draggingId === id) { resetDrag(); return; }
    const targetOrder = panelStack.indexOf(id);
    uiPrefs.reorderPanel(draggingId, targetOrder);
    resetDrag();
  }
  function onPanelDragEnd() { resetDrag(); }
  function onTailDragOver(ev: DragEvent) {
    if (!draggingId) return;
    ev.preventDefault();
    if (ev.dataTransfer) ev.dataTransfer.dropEffect = "move";
    dragOverId = null; // tail marker handled separately
  }
  function onTailDrop(ev: DragEvent) {
    ev.preventDefault();
    if (!draggingId) return;
    uiPrefs.reorderPanel(draggingId, panelStack.length);
    resetDrag();
  }
  function resetDrag() { draggingId = null; dragOverId = null; }

  // ── Width-resize handle (left edge) ────────────────────────────────
  // Pointer events for unified mouse/touch/pen. setPointerCapture so the
  // pointerup still fires even if the cursor leaves the strip.
  // RAF-throttled move + persist-on-release to avoid synchronous localStorage
  // writes during the drag (those were the lag source).
  let resizing = $state(false);
  let startX = 0;
  let startW = 0;
  let pendingW = 0;
  let rafId = 0;

  function flushDrag() {
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
    // Block text/element selection during the drag; user-select on body is
    // the lightest-weight way to do it (vs gating mousemove on every node).
    document.body.style.userSelect = "none";
    ev.preventDefault();
  }
  function onWidthPointerMove(ev: PointerEvent) {
    if (!resizing) return;
    // Dock is on the right edge: dragging the left-side handle LEFT widens.
    pendingW = startW + (startX - ev.clientX);
    if (rafId === 0) rafId = requestAnimationFrame(flushDrag);
  }
  function onWidthPointerUp(ev: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    if (rafId !== 0) { cancelAnimationFrame(rafId); rafId = 0; }
    // Final commit + persist (one localStorage write per drag, not 100/sec).
    uiPrefs.setDockWidthLive(pendingW);
    uiPrefs.persistDockWidth();
    (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }

</script>

<aside class="dock" aria-label="Dock panels">
  <div
    class="dock-resize"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize dock width"
    aria-valuenow={uiPrefs.dockWidth}
    aria-valuemin={280}
    aria-valuemax={560}
    data-active={resizing}
    onpointerdown={onWidthPointerDown}
    onpointermove={onWidthPointerMove}
    onpointerup={onWidthPointerUp}
    onpointercancel={onWidthPointerUp}
  ></div>

  <div class="dock-body">
    {#each panelStack as id (id)}
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

    {#if draggingId}
      <div
        class="tail-drop"
        ondragover={onTailDragOver}
        ondrop={onTailDrop}
        aria-hidden="true"
      ></div>
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

  /* Hit target is 10px wide (5px each side of the border seam) for easy
     grab; visual feedback is a 2px accent line on the inside edge that
     shows on hover/active. Strip is shifted -5px so it straddles the
     dock's left border instead of starting inside the dock content. */
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
    overflow-y: auto;
    overflow-x: hidden;
    display: flex; flex-direction: column;
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

  .tail-drop {
    height: 28px;
    margin: 2px 0;
    border: 1px dashed color-mix(in oklch, var(--accent) 40%, transparent);
    border-radius: var(--radius-sm);
    background: color-mix(in oklch, var(--accent) 6%, transparent);
  }
</style>
