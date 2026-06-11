<script lang="ts">
  import { contextMenu } from "$lib/state/contextMenu.svelte";
  import { portal } from "$lib/actions/portal";

  let menuEl = $state<HTMLDivElement | undefined>();
  let pos = $state({ x: 0, y: 0 });

  $effect(() => {
    const cur = contextMenu.current;
    if (!cur) return;
    pos = { x: cur.x, y: cur.y };
    // Clamp into viewport on the next frame so we have real dimensions.
    const raf = requestAnimationFrame(() => {
      if (!menuEl) return;
      const r = menuEl.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      let nx = cur.x, ny = cur.y;
      if (nx + r.width + 4 > vw) nx = Math.max(4, vw - r.width - 4);
      if (ny + r.height + 4 > vh) ny = Math.max(4, vh - r.height - 4);
      pos = { x: nx, y: ny };
    });
    const onDown = (e: MouseEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) contextMenu.close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") contextMenu.close();
    };
    const onAway = () => contextMenu.close();
    document.addEventListener("mousedown", onDown, true);
    document.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", onAway);
    window.addEventListener("resize", onAway);
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("mousedown", onDown, true);
      document.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", onAway);
      window.removeEventListener("resize", onAway);
    };
  });

  function run(action: () => void | Promise<void>) {
    contextMenu.close();
    void action();
  }
</script>

{#if contextMenu.current}
  <div
    bind:this={menuEl}
    class="rift-menu menu"
    role="menu"
    tabindex="-1"
    use:portal
    style="left: {pos.x}px; top: {pos.y}px;"
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#each contextMenu.current.items as it, i (i)}
      {#if it.kind === "divider"}
        <div class="rift-menu-divider"></div>
      {:else}
        <button
          type="button"
          class="rift-menu-row"
          role="menuitem"
          disabled={it.disabled}
          onclick={() => run(it.action)}
        >
          {#if it.icon}
            {@const Ic = it.icon}
            <Ic size={14} class="rift-menu-row-ic" />
          {/if}
          <span class="rift-menu-row-t">{it.label}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

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
