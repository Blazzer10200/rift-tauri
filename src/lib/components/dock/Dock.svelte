<script lang="ts">
  import type { PanelId } from "$lib/state/panel-types";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import PanelShell from "./PanelShell.svelte";
  import AddPanelMenu from "./AddPanelMenu.svelte";

  // Sorted list of panel ids that are currently open in the dock, in their
  // user-arranged order. Closed panels disappear from the stack entirely —
  // toggled back via rail icon or AddPanelMenu.
  const openPanels = $derived(
    (Object.entries(uiPrefs.panels) as [PanelId, { open: boolean; order: number }][])
      .filter(([, st]) => st.open)
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
    const targetOrder = openPanels.indexOf(id);
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
    uiPrefs.reorderPanel(draggingId, openPanels.length);
    resetDrag();
  }
  function resetDrag() { draggingId = null; dragOverId = null; }

  // ── Width-resize handle (left edge) ────────────────────────────────
  // Pointer events for unified mouse/touch/pen. setPointerCapture so the
  // pointerup still fires even if the cursor leaves the 4px strip.
  let resizing = $state(false);
  let startX = 0;
  let startW = 0;

  function onWidthPointerDown(ev: PointerEvent) {
    resizing = true;
    startX = ev.clientX;
    startW = uiPrefs.dockWidth;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    document.body.style.cursor = "ew-resize";
    ev.preventDefault();
  }
  function onWidthPointerMove(ev: PointerEvent) {
    if (!resizing) return;
    // Dock is on the right edge: dragging the left-side handle LEFT widens.
    const delta = startX - ev.clientX;
    uiPrefs.setDockWidth(startW + delta);
  }
  function onWidthPointerUp(ev: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
    document.body.style.cursor = "";
  }

  let addOpen = $state(false);
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
    {#if openPanels.length === 0}
      <div class="dock-empty">
        <p>No panels open</p>
        <p class="dim">Use the rail or <kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">1…8</kbd> to open a panel.</p>
      </div>
    {/if}

    {#each openPanels as id (id)}
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

    {#if draggingId && openPanels.length > 0}
      <div
        class="tail-drop"
        ondragover={onTailDragOver}
        ondrop={onTailDrop}
        aria-hidden="true"
      ></div>
    {/if}
  </div>

  <div class="dock-foot">
    <div class="add-wrap">
      <button
        class="dock-add"
        type="button"
        onclick={() => addOpen = !addOpen}
        aria-haspopup="menu"
        aria-expanded={addOpen}
        title="Add or remove panels"
      >
        <span class="plus">+</span>
        <span>Add panel</span>
      </button>
      {#if addOpen}
        <AddPanelMenu onClose={() => addOpen = false}/>
      {/if}
    </div>
  </div>
</aside>

<style>
  .dock {
    position: relative;
    display: flex; flex-direction: column;
    height: 100%;
    background: var(--bg);
    border-left: 1px solid var(--border);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .dock-resize {
    position: absolute;
    left: 0; top: 0; bottom: 0;
    width: 4px;
    background: transparent;
    cursor: ew-resize;
    z-index: 10;
    transition: background 120ms ease;
  }
  .dock-resize:hover,
  .dock-resize[data-active="true"] { background: var(--accent-soft); }

  .dock-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex; flex-direction: column;
  }
  .dock-empty {
    padding: 24px 16px;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
    display: flex; flex-direction: column; gap: 6px;
  }
  .dock-empty p { margin: 0; }
  .dock-empty .dim { color: var(--fg-faint); font-size: var(--fs-xs); }
  .dock-empty .kbd { margin: 0 2px; }

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

  .dock-foot {
    border-top: 1px solid var(--border);
    padding: 6px;
    flex-shrink: 0;
  }
  .add-wrap { position: relative; }
  .dock-add {
    display: flex; align-items: center; gap: 8px;
    width: 100%; height: 28px;
    padding: 0 10px;
    background: transparent;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-sm);
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .dock-add:hover {
    background: var(--surface-hover);
    color: var(--fg);
    border-color: color-mix(in oklch, var(--accent) 35%, var(--border-strong));
  }
  .dock-add .plus {
    font-weight: 600; font-size: 14px; line-height: 1;
    color: var(--fg-muted);
  }
  .dock-add:hover .plus { color: var(--accent); }
</style>
