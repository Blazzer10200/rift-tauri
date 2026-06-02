<script lang="ts">
  import { onDestroy } from "svelte";
  import { WORKSPACES } from "../workspaces";
  import { workspace, type WorkspaceId } from "$lib/state/workspace.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  // Pointer-event drag-to-reorder. The previous HTML5 DnD path didn't fire
  // reliably from <button> inside WebView2 (no ghost image, dragstart
  // suppressed by the button's click activation). Pointer events bypass that
  // entirely and let us paint our own drag affordance.
  //
  // Settings sits in its own bottom group, pinned + non-draggable. Disabled
  // stubs (agents / attachments) also opt out — they stay in flow but can't
  // be picked up.
  let topRowEl = $state<HTMLDivElement | null>(null);
  let dragSrc = $state<number | null>(null);   // index into workspace.order
  let pointerStartY = 0;
  let dragDeltaY = $state(0);                  // clamped px source has shifted from start
  let dragActive = $state(false);              // past threshold → committed to drag
  let suppressClickKey = $state<string | null>(null);
  // dropSlot = insertion slot in topOrder (0..topOrder.length). The source
  // ends up at this slot when released. Non-source rows slide to open a gap
  // here — no separate drop-line needed.
  let dropSlot = $state<number | null>(null);
  const ROW_PITCH = 42;          // 40px button + 2px gap (matches .ab-group gap)
  const DRAG_THRESHOLD_PX = 4;

  const topOrder = $derived(workspace.order.filter((id) => id !== "settings"));
  const settingsIdx = $derived(workspace.order.indexOf("settings"));
  // Source's visual position inside topOrder (-1 if not dragging).
  const srcVisualIdx = $derived(
    dragSrc === null
      ? -1
      : (topOrder as WorkspaceId[]).indexOf(workspace.order[dragSrc])
  );

  function onPointerDown(idx: number, ev: PointerEvent) {
    if (ev.button !== 0) return;       // left click only
    if (dragSrc !== null) return;       // another drag already in flight (multi-touch)
    if (idx < 0 || idx >= workspace.order.length) return; // stale idx guard
    dragSrc = idx;
    dragDeltaY = 0;
    dropSlot = null;
    dragActive = false;
    pointerStartY = ev.clientY;
    try {
      (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    } catch { /* capture unsupported / element detached — drag still works via bubbling */ }
  }

  function onPointerMove(ev: PointerEvent) {
    if (dragSrc === null) return;
    const rawDelta = ev.clientY - pointerStartY;
    if (!dragActive) {
      if (Math.abs(rawDelta) < DRAG_THRESHOLD_PX) return;
      dragActive = true;
    }
    // Clamp source travel so the lifted icon can't visually escape the top
    // group (no flying into the divider / Settings zone).
    const visIdx = srcVisualIdx;
    if (visIdx < 0 || topOrder.length === 0) return;
    const minDelta = -visIdx * ROW_PITCH;
    const maxDelta = (topOrder.length - 1 - visIdx) * ROW_PITCH;
    dragDeltaY = Math.max(minDelta, Math.min(maxDelta, rawDelta));

    // Slot k's center sits at `k * ROW_PITCH + ROW_CENTER` from the top of
    // `.ab-top` (20 = half the 40px button). Pick the nearest slot center
    // to where the source icon is currently centered.
    const ROW_CENTER = 20;
    const sourceCenterY = visIdx * ROW_PITCH + ROW_CENTER + dragDeltaY;
    let slot = Math.round((sourceCenterY - ROW_CENTER) / ROW_PITCH);
    if (slot < 0) slot = 0;
    if (slot > topOrder.length - 1) slot = topOrder.length - 1;
    dropSlot = slot;
  }

  function onPointerUp(ev: PointerEvent) {
    if (dragSrc === null) return;
    try {
      (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
    } catch { /* capture lost */ }
    const src = dragSrc;
    const visIdx = srcVisualIdx;
    const slot = dropSlot;
    const wasDrag = dragActive;
    dragSrc = null;
    dragDeltaY = 0;
    dropSlot = null;
    dragActive = false;
    if (wasDrag && slot !== null && visIdx >= 0 && slot !== visIdx) {
      // Map topOrder slot → workspace.order insertion index after source
      // removal. Settings (and any other non-top items) stays put.
      const orderWithoutSrc = workspace.order.filter((_, i) => i !== src);
      const topWithoutSrc = orderWithoutSrc.filter((id) => id !== "settings");
      let to: number;
      if (slot >= topWithoutSrc.length) {
        const sIdx = orderWithoutSrc.indexOf("settings");
        to = sIdx >= 0 ? sIdx : orderWithoutSrc.length;
      } else {
        to = orderWithoutSrc.indexOf(topWithoutSrc[slot]);
      }
      workspace.reorder(src, to);
      suppressClickKey = workspace.order[to] ?? null;
      setTimeout(() => { suppressClickKey = null; }, 0);
    } else if (wasDrag) {
      suppressClickKey = workspace.order[src] ?? null;
      setTimeout(() => { suppressClickKey = null; }, 0);
    }
  }

  function onPointerCancel() {
    dragSrc = null;
    dragDeltaY = 0;
    dropSlot = null;
    dragActive = false;
  }

  // HMR / parent-driven unmount mid-drag — reset so the next mount doesn't
  // inherit a phantom dragSrc. Pointer capture is released automatically when
  // the captured element leaves the DOM.
  onDestroy(() => {
    dragSrc = null;
    dragDeltaY = 0;
    dropSlot = null;
    dragActive = false;
    suppressClickKey = null;
  });

  // shiftY for a non-source row at visualIdx `v` — opens a gap at `dropSlot`
  // by pretending the source is "out of the list", then re-inserting it at
  // the slot. Animated via CSS transition on .ab-btn.
  function rowShift(v: number): number {
    if (!dragActive || srcVisualIdx < 0 || dropSlot === null) return 0;
    if (v === srcVisualIdx) return 0;
    // targetSlot in the source-removed coord space
    const targetSlot = dropSlot > srcVisualIdx ? dropSlot - 1 : dropSlot;
    if (v > srcVisualIdx && v - 1 < targetSlot) return -ROW_PITCH;
    if (v < srcVisualIdx && v >= targetSlot) return ROW_PITCH;
    return 0;
  }
</script>

<nav class="activitybar" aria-label="Workspaces">
  <div class="ab-group ab-top" bind:this={topRowEl}>
    {#each topOrder as id, v (id)}
      {@const idx = workspace.order.indexOf(id)}
      {@const def = WORKSPACES[id as WorkspaceId]}
      {@const isActive = workspace.activeId === id}
      {@const count = def.disabled ? 0 : (def.getCount?.() ?? 0)}
      {@const tone = def.getTone ?? "neutral"}
      {@const isSrc = dragSrc === idx && dragActive}
      {@const shift = isSrc ? 0 : rowShift(v)}
      <button
        class="ab-btn"
        type="button"
        data-row-idx={idx}
        data-active={isActive}
        data-disabled={def.disabled ? "true" : "false"}
        data-drag-src={isSrc}
        disabled={def.disabled}
        use:tooltip={def.disabled ? `${def.title} — Coming soon` : `${def.title} · Ctrl+${def.kbd} · drag to reorder`}
        aria-label={isActive ? `${def.title} (active)` : def.title}
        aria-pressed={isActive}
        onpointerdown={(e) => { if (!def.disabled) onPointerDown(idx, e); }}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerCancel}
        onclick={() => {
          if (def.disabled) return;
          if (suppressClickKey === id) return;
          workspace.setActive(id as WorkspaceId);
        }}
        style={isSrc
          ? `transform: translateY(${dragDeltaY}px) scale(1.08); z-index: 5;`
          : `transform: translateY(${shift}px);`}
      >
        <span class="ab-hit">
          <span class="ab-icon"><def.icon size={17}/></span>
          {#if !def.disabled && count > 0}
            <span class="ab-count count-pip" data-tone={tone}>{count > 99 ? "99+" : count}</span>
          {/if}
        </span>
      </button>
    {/each}
  </div>

  <div class="ab-group ab-bottom">
    {#if settingsIdx >= 0}
      {@const def = WORKSPACES.settings}
      {@const isActive = workspace.activeId === "settings"}
      <button
        class="ab-btn"
        type="button"
        data-active={isActive}
        data-disabled="false"
        use:tooltip={`${def.title} · Ctrl+${def.kbd} · Ctrl+,`}
        aria-label={isActive ? `${def.title} (active)` : def.title}
        aria-pressed={isActive}
        onclick={() => workspace.setActive("settings")}
      >
        <span class="ab-hit">
          <span class="ab-icon"><def.icon size={17}/></span>
        </span>
      </button>
    {/if}
  </div>
</nav>

<style>
  .activitybar {
    width: 44px;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    /* Blend into the shell — no harsh divider. A faint inset on the inner
       edge gives just enough separation against the main pane. */
    background: linear-gradient(
      to right,
      color-mix(in oklch, var(--surface) 96%, transparent),
      color-mix(in oklch, var(--surface) 70%, transparent)
    );
    box-shadow: inset -1px 0 0 color-mix(in oklch, var(--border) 55%, transparent);
    overflow: hidden;
    user-select: none;
    padding: 6px 0;
  }
  .ab-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .ab-top    { flex: 0 0 auto; position: relative; }
  .ab-bottom {
    margin-top: auto;
    padding-top: 8px;
    position: relative;
  }
  .ab-bottom::before {
    content: "";
    position: absolute;
    left: 8px; right: 8px; top: 0;
    height: 1px;
    background: color-mix(in oklch, var(--border) 70%, transparent);
  }

  .ab-btn {
    position: relative;
    display: flex; align-items: center; justify-content: center;
    width: 44px; height: 40px;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--fg-muted);
    cursor: pointer;
    flex-shrink: 0;
    /* Smooth slide when neighbors shift to open a drop gap. Source row
       overrides w/ `transition: none` to track the cursor 1:1. */
    transition: transform 180ms cubic-bezier(.2,.7,.2,1);
    /* will-change removed (perma-compositor caused icon-blur at 1.25x DPR).
       Drag/drop reorders are rare; the transition's transform alone GPU-
       composites for the brief animation. */
  }
  /* Inset pill — the actual hover/active target. Gives a modern rounded
     affordance instead of a full-width slab. */
  .ab-hit {
    position: relative;
    display: inline-flex; align-items: center; justify-content: center;
    width: 32px; height: 32px;
    border-radius: 8px;
    transition: background 140ms ease, color 140ms ease, transform 140ms ease;
  }
  .ab-btn:hover .ab-hit {
    background: color-mix(in oklch, var(--fg) 8%, transparent);
    color: var(--fg);
  }
  .ab-btn:active .ab-hit { transform: scale(0.94); }
  .ab-btn:focus-visible { outline: none; }
  .ab-btn:focus-visible .ab-hit {
    box-shadow: 0 0 0 2px var(--ring);
  }

  .ab-btn[data-active="true"] { color: var(--accent); }
  .ab-btn[data-active="true"] .ab-hit {
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: var(--accent);
  }
  /* Thin static accent left-bar — the redesign's "active = accent + thin left
     bar, calm not blocky". No infinite decorative loops on resting content. */
  .ab-btn[data-active="true"]::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    height: 18px;
    width: 3px;
    background: var(--accent);
    border-radius: 0 3px 3px 0;
    box-shadow: 0 0 8px color-mix(in oklch, var(--accent) 40%, transparent);
  }
  /* Hover halo on the hit-target — subtle accent ring on hover so non-active
     workspaces feel reactive, not just bg-tinted. */
  .ab-btn:not([data-active="true"]):not([data-disabled="true"]):hover .ab-hit {
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--accent) 20%, transparent);
  }

  /* Dragging — the picked-up row visibly lifts: stronger accent fill,
     prominent shadow, scale + cursor-tracking translateY (inline style).
     Transition off so it tracks the pointer 1:1. */
  .ab-btn[data-drag-src="true"] {
    cursor: grabbing;
    transition: none;
  }
  .ab-btn[data-drag-src="true"] .ab-hit {
    background: color-mix(in oklch, var(--accent) 28%, transparent);
    color: var(--accent);
    box-shadow:
      0 0 0 1.5px var(--accent),
      0 12px 24px -8px rgba(0,0,0,0.55),
      0 4px 10px -2px rgba(0,0,0,0.4);
    transition: none;
  }
  .ab-btn[data-drag-src="true"]::before { display: none; }

  .ab-btn:not([data-disabled="true"]) { cursor: grab; }
  .ab-btn:not([data-disabled="true"]):active { cursor: grabbing; }

  .ab-btn[data-disabled="true"] {
    cursor: not-allowed;
  }
  .ab-btn[data-disabled="true"] .ab-hit {
    opacity: 0.32;
  }
  .ab-btn[data-disabled="true"]:hover .ab-hit {
    background: transparent;
    color: var(--fg-muted);
  }

  .ab-icon {
    display: inline-flex; align-items: center; justify-content: center;
  }

  .ab-count {
    position: absolute;
    top: -3px; right: -4px;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 999px;
    font-size: 9px;
    font-weight: 600;
    line-height: 15px;
    text-align: center;
    color: var(--fg);
    background: var(--bg-elev-2);
    box-shadow: 0 0 0 2px var(--surface);
    pointer-events: none;
    font-variant-numeric: tabular-nums;
  }
  .ab-count[data-tone="warn"]   { background: var(--warn);   color: #fff; }
  .ab-count[data-tone="danger"] { background: var(--danger); color: #fff; }
  .ab-count[data-tone="info"]   {
    background: color-mix(in oklch, var(--info) 85%, transparent);
    color: #fff;
  }
</style>
