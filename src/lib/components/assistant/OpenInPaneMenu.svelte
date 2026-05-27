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
    setTimeout(() => {
      document.addEventListener("mousedown", onDocClick);
      document.addEventListener("keydown", onKey);
    }, 0);
    return () => {
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
  class="menu"
  role="menu"
  style="left: {pos.x}px; top: {pos.y}px;"
>
  {#each panes as _p, i (i)}
    <button
      type="button"
      class="item"
      role="menuitem"
      onclick={() => pickPane(i)}
      disabled={panes[i].tabId === tabId}
      use:tooltip={panes[i].tabId === tabId ? "Already in this pane" : `Open in pane ${i + 1}`}
    >
      <Columns2 size={12} />
      <span>Open in pane {i + 1}</span>
    </button>
  {/each}
  {#if canAddPane}
    <button type="button" class="item" role="menuitem" onclick={newPane}>
      <Plus size={12} />
      <span>Open in new pane</span>
    </button>
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 180px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong, var(--border));
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32);
    padding: 4px;
    display: flex; flex-direction: column;
  }
  .item {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: var(--fg);
    font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
  }
  .item:hover:not(:disabled) { background: var(--surface-hover); }
  .item:disabled { color: var(--fg-subtle); cursor: not-allowed; }
  .item :global(svg) { color: var(--fg-muted); flex-shrink: 0; }
</style>
