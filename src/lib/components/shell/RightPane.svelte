<script lang="ts">
  import { PANELS } from "../dock/panels";
  import { rightPane, type ActivityBarId } from "$lib/state/right-pane.svelte";
  import { PANEL_IDS } from "$lib/state/panel-types";

  // Lazy-mount latch — render each page once its id has been active at least
  // once, then keep mounted so scroll/selection/terminal-session state
  // survives toggling. Same pattern PanelShell used (everOpened set on the
  // RightPane state — module-scope so it persists across remounts too).

  // ── Width-resize handle (left edge) — same drag/persist split as Dock had.
  let resizing = $state(false);
  let startX = 0;
  let startW = 0;
  let pendingW = 0;
  let rafId = 0;

  function flushWidth() {
    rafId = 0;
    rightPane.setWidthLive(pendingW);
  }
  function onPointerDown(ev: PointerEvent) {
    resizing = true;
    startX = ev.clientX;
    startW = rightPane.width;
    pendingW = startW;
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    document.body.style.cursor = "ew-resize";
    document.body.style.userSelect = "none";
    ev.preventDefault();
  }
  function onPointerMove(ev: PointerEvent) {
    if (!resizing) return;
    pendingW = startW + (startX - ev.clientX);
    if (rafId === 0) rafId = requestAnimationFrame(flushWidth);
  }
  function onPointerUp(ev: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    if (rafId !== 0) { cancelAnimationFrame(rafId); rafId = 0; }
    rightPane.setWidthLive(pendingW);
    rightPane.persistWidth();
    (ev.currentTarget as HTMLElement).releasePointerCapture(ev.pointerId);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }
  function onDblClick() {
    rightPane.snapHalfViewport();
  }
</script>

{#if rightPane.activeId}
  <aside class="right-pane" aria-label="Right pane">
    <div
      class="rp-resize"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize right pane"
      aria-valuenow={rightPane.width}
      aria-valuemin={320}
      aria-valuemax={1200}
      data-active={resizing}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      ondblclick={onDblClick}
    ></div>

    <div class="rp-body">
      {#each PANEL_IDS as id (id)}
        {#if rightPane.everOpened.has(id as ActivityBarId)}
          {@const def = PANELS[id]}
          <div class="rp-page" hidden={rightPane.activeId !== id}>
            <def.component title={def.title} icon={def.icon}/>
          </div>
        {/if}
      {/each}
    </div>
  </aside>
{/if}

<style>
  .right-pane {
    position: relative;
    display: flex; flex-direction: column;
    height: 100%;
    background: var(--bg-elev-1);
    border-left: 1px solid var(--border);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .rp-resize {
    position: absolute;
    left: -5px; top: 0; bottom: 0;
    width: 10px;
    background: transparent;
    cursor: ew-resize;
    z-index: 20;
    touch-action: none;
  }
  .rp-resize::after {
    content: "";
    position: absolute;
    left: 5px; top: 0; bottom: 0;
    width: 2px;
    background: transparent;
    transition: background 120ms ease;
    pointer-events: none;
  }
  .rp-resize:hover::after,
  .rp-resize[data-active="true"]::after { background: var(--accent); }

  .rp-body {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }
  .rp-page {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .rp-page[hidden] {
    display: none;
  }
</style>
