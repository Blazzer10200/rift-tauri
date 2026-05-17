<script lang="ts">
  import { ChevronRight, Maximize2, MoreHorizontal, X, Minimize2 } from "lucide-svelte";
  import type { PanelId } from "$lib/state/panel-types";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { PANELS } from "./panels";

  let {
    id,
    isMaximized = false,
    onDragStart,
    onDragOver,
    onDrop,
    onDragEnd,
  }: {
    id: PanelId;
    isMaximized?: boolean;
    onDragStart?: (id: PanelId, ev: DragEvent) => void;
    onDragOver?: (id: PanelId, ev: DragEvent) => void;
    onDrop?: (id: PanelId, ev: DragEvent) => void;
    onDragEnd?: (id: PanelId, ev: DragEvent) => void;
  } = $props();

  // Local name `ps` (panel state) — calling this `state` confuses svelte-check
  // into treating the var as a legacy auto-subscribe store, which fails type-
  // check on every downstream `.open`/`.collapsed` access.
  const def = $derived(PANELS[id]);
  const ps = $derived(uiPrefs.panels[id]);
  const showBody = $derived(!ps.collapsed);
  // Optional count pip from registry — reactive because getCount reads
  // $state-backed stores inside the lambda. Tone defaults to neutral grey.
  const count = $derived(def.getCount?.() ?? 0);
  const countTone = $derived(def.getTone ?? "neutral");

  // Lazy-mount: don't instantiate panel body until first open. Once mounted,
  // stays mounted so internal state (scroll, selection) survives toggle.
  let everOpened = $state(false);
  $effect(() => { if (ps.open && !everOpened) everOpened = true; });

  let menuOpen = $state(false);
  let menuEl = $state<HTMLDivElement | undefined>(undefined);

  function onWindowClick(ev: MouseEvent) {
    if (!menuOpen) return;
    if (menuEl && !menuEl.contains(ev.target as Node)) menuOpen = false;
  }
  function onWindowKey(ev: KeyboardEvent) {
    if (menuOpen && ev.key === "Escape") menuOpen = false;
  }

  // Resize handle drag (pinned-height mode). Pointer events for unified
  // mouse+touch+pen behavior; capture so release fires reliably.
  let resizing = $state(false);
  let startY = 0;
  let startH = 0;
  let bodyEl = $state<HTMLDivElement | undefined>(undefined);

  function onResizePointerDown(ev: PointerEvent) {
    if (!bodyEl) return;
    resizing = true;
    startY = ev.clientY;
    startH = ps.height ?? bodyEl.getBoundingClientRect().height;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    ev.preventDefault();
  }
  function onResizePointerMove(ev: PointerEvent) {
    if (!resizing) return;
    const next = Math.max(80, startH + (ev.clientY - startY));
    uiPrefs.setPanelHeight(id, next);
  }
  function onResizePointerUp(ev: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKey}/>

<section
  class="panel"
  data-panel-id={id}
  data-open={ps.open}
  data-collapsed={ps.collapsed}
  data-maximized={isMaximized}
  aria-label="{def.title} panel"
>
  <header
    class="panel-head"
    role="toolbar"
    aria-label="{def.title} panel header"
    tabindex="-1"
    draggable={!isMaximized}
    ondragstart={(e) => onDragStart?.(id, e)}
    ondragover={(e) => onDragOver?.(id, e)}
    ondrop={(e) => onDrop?.(id, e)}
    ondragend={(e) => onDragEnd?.(id, e)}
  >
    <button
      class="caret"
      type="button"
      aria-label={ps.collapsed ? "Expand panel body" : "Collapse panel body"}
      aria-expanded={!ps.collapsed}
      onclick={(e) => { uiPrefs.togglePanelCollapsed(id); (e.currentTarget as HTMLButtonElement).blur(); }}
    >
      <span class="caret-icon" data-open={!ps.collapsed}><ChevronRight size={12}/></span>
    </button>
    <span class="head-icon"><def.icon size={13}/></span>
    <span class="head-title">{def.title}</span>
    {#if count > 0}
      <span class="head-count count-pip" data-tone={countTone}>{count}</span>
    {/if}
    <span class="head-spacer"></span>
    <button
      class="head-btn"
      type="button"
      title={isMaximized ? "Restore to dock" : "Maximize to center"}
      aria-label={isMaximized ? "Restore panel to dock" : "Maximize panel to center"}
      onclick={(e) => { uiPrefs.maximizePanel(isMaximized ? null : id); (e.currentTarget as HTMLButtonElement).blur(); }}
    >
      {#if isMaximized}<Minimize2 size={12}/>{:else}<Maximize2 size={12}/>{/if}
    </button>
    <div class="menu-wrap" bind:this={menuEl}>
      <button
        class="head-btn"
        type="button"
        title="Panel menu"
        aria-label="Panel menu"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onclick={(e) => { menuOpen = !menuOpen; e.stopPropagation(); }}
      >
        <MoreHorizontal size={12}/>
      </button>
      {#if menuOpen}
        <div class="menu" role="menu">
          <button
            class="menu-item"
            type="button"
            role="menuitem"
            onclick={() => { uiPrefs.setPanelOpen(id, false); menuOpen = false; }}
          >
            <X size={12}/>
            <span>Close panel</span>
          </button>
          <button
            class="menu-item dim"
            type="button"
            role="menuitem"
            disabled
            title="Per-panel settings — coming in Phase B"
          >
            <span>Settings…</span>
          </button>
          <button
            class="menu-item dim"
            type="button"
            role="menuitem"
            disabled
            title="Pop out to floating window — v0.4"
          >
            <span>Pop out</span>
          </button>
        </div>
      {/if}
    </div>
  </header>

  {#if showBody}
    <div
      class="panel-body"
      bind:this={bodyEl}
      style:height={ps.height ? `${ps.height}px` : null}
    >
      {#if everOpened}
        <def.component title={def.title} icon={def.icon}/>
      {/if}
    </div>
    {#if ps.height !== null}
      <div
        class="resize-handle"
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize panel height"
        onpointerdown={onResizePointerDown}
        onpointermove={onResizePointerMove}
        onpointerup={onResizePointerUp}
        onpointercancel={onResizePointerUp}
      ></div>
    {/if}
  {/if}
</section>

<style>
  .panel {
    display: flex; flex-direction: column;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    min-height: 0;
  }
  .panel:last-child { border-bottom: 0; }

  .panel-head {
    display: flex; align-items: center; gap: 6px;
    height: 30px;
    padding: 0 6px 0 2px;
    color: var(--fg-2);
    font-size: var(--fs-sm);
    user-select: none;
    cursor: grab;
    transition: background 120ms ease;
  }
  .panel-head:hover { background: var(--surface-hover); }
  .panel-head:active { cursor: grabbing; }

  .caret {
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 22px;
    background: transparent; border: 0; padding: 0;
    color: var(--fg-muted);
    cursor: pointer;
    border-radius: var(--radius-xs);
  }
  .caret:hover { color: var(--fg); background: var(--surface-active); }
  .caret-icon {
    display: inline-flex;
    transition: transform 140ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .caret-icon[data-open="true"] { transform: rotate(90deg); }

  .head-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 16px;
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .head-title {
    font-weight: 500;
    color: var(--fg);
    letter-spacing: -0.005em;
  }
  .head-spacer { flex: 1; }

  /* Count pip in panel header — uses the global .count-pip primitive from
     app.css plus per-tone overrides. Stays muted by default so it doesn't
     fight the title for attention. */
  .head-count { flex-shrink: 0; }
  .head-count[data-tone="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .head-count[data-tone="danger"] { background: var(--danger-soft); color: var(--danger); }
  .head-count[data-tone="info"]   { background: var(--info-soft);   color: var(--info); }

  .head-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px;
    background: transparent; border: 0; padding: 0;
    color: var(--fg-muted);
    border-radius: var(--radius-xs);
    cursor: pointer;
    opacity: 0;
    transition: background 120ms ease, color 120ms ease, opacity 120ms ease;
  }
  .panel-head:hover .head-btn,
  .panel-head:focus-within .head-btn { opacity: 1; }
  .head-btn:hover { background: var(--surface-active); color: var(--fg); }
  .head-btn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); opacity: 1; }

  .menu-wrap { position: relative; }
  .menu {
    position: absolute;
    top: 100%; right: 0;
    margin-top: 4px;
    min-width: 160px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    padding: 4px;
    display: flex; flex-direction: column; gap: 1px;
    z-index: 60;
    animation: menu-in 110ms ease-out both;
  }
  .menu-item {
    display: flex; align-items: center; gap: 8px;
    height: 26px; padding: 0 8px;
    background: transparent; border: 0;
    color: var(--fg);
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition: background 100ms ease;
  }
  .menu-item:hover:not(:disabled) { background: var(--surface-hover); }
  .menu-item:disabled { cursor: not-allowed; color: var(--fg-faint); }
  .menu-item.dim { color: var(--fg-faint); }

  .panel-body {
    overflow: auto;
    min-height: 0;
    background: var(--bg);
  }

  /* When panel is collapsed, body is gone — head sits alone, no extra
     bottom-border doubling (panel itself has the border). */

  .panel[data-collapsed="true"] .panel-head { border-bottom: 1px solid transparent; }

  .resize-handle {
    height: 4px;
    background: transparent;
    cursor: ns-resize;
    transition: background 120ms ease;
  }
  .resize-handle:hover,
  .resize-handle:active { background: var(--accent-soft); }

  @keyframes menu-in {
    from { opacity: 0; transform: translateY(-2px); }
    to   { opacity: 1; transform: none; }
  }
</style>
