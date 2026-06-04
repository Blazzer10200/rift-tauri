<script lang="ts">
  import { onMount } from "svelte";
  import { Columns2, Plus } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  let {
    tabId,
    x,
    y,
    onClose,
  }: { tabId: string; x: number; y: number; onClose: () => void } = $props();

  const panes = $derived(assistant.panes);
  const canAddPane = $derived(assistant.canAddPane);

  let menuEl = $state<HTMLDivElement | undefined>();
  // svelte-ignore state_referenced_locally
  let pos = $state({ x, y });

  onMount(() => {
    // Clamp into viewport on the next frame so we have real dimensions.
    requestAnimationFrame(() => {
      if (!menuEl) return;
      const r = menuEl.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      let nx = x, ny = y;
      if (nx + r.width + 4 > vw) nx = Math.max(4, vw - r.width - 4);
      if (ny + r.height + 4 > vh) ny = Math.max(4, vh - r.height - 4);
      pos = { x: nx, y: ny };
    });
    const onDocClick = (e: MouseEvent) => {
      if (!menuEl) return;
      if (!menuEl.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    let destroyed = false;
    const tid = setTimeout(() => {
      if (!destroyed) {
        document.addEventListener("mousedown", onDocClick);
        document.addEventListener("keydown", onKey);
      }
    }, 0);
    return () => {
      destroyed = true;
      clearTimeout(tid);
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  });

  function pickPane(i: number) {
    assistant.dropTabIntoPane(tabId, i);
    onClose();
  }
  function newPane() {
    assistant.dropTabIntoPane(tabId, panes.length);
    onClose();
  }
</script>

<div
  bind:this={menuEl}
  class="rift-menu menu"
  role="menu"
  style="left: {pos.x}px; top: {pos.y}px;"
>
  {#each panes as _p, i (i)}
    <button
      type="button"
      class="rift-menu-row"
      role="menuitem"
      onclick={() => pickPane(i)}
      disabled={panes[i].tabId === tabId}
      use:tooltip={panes[i].tabId === tabId ? "Already in this pane" : `Open in pane ${i + 1}`}
    >
      <Columns2 size={14} class="rift-menu-row-ic" />
      <span class="rift-menu-row-t">Open in pane {i + 1}</span>
    </button>
  {/each}
  {#if canAddPane}
    <button type="button" class="rift-menu-row" role="menuitem" onclick={newPane}>
      <Plus size={14} class="rift-menu-row-ic" />
      <span class="rift-menu-row-t">Open in new pane</span>
    </button>
  {/if}
</div>

<style>
  /* Inherits .rift-menu / .rift-menu-row chrome (app.css); only positioning here. */
  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 180px;
    display: flex; flex-direction: column; gap: 1px;
  }
  .menu :global(.rift-menu-row) { align-items: center; }
  .menu :global(.rift-menu-row:disabled) { color: var(--fg-subtle); cursor: not-allowed; }
  .menu :global(.rift-menu-row:disabled:hover) { background: transparent; }
</style>
