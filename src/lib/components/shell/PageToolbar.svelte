<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    left,
    right,
    children,
  }: {
    left?: Snippet;
    right?: Snippet;
    children?: Snippet;
  } = $props();
</script>

<div class="page-toolbar">
  {#if children}
    {@render children()}
  {:else}
    <div class="tb-left">{#if left}{@render left()}{/if}</div>
    <div class="tb-right">{#if right}{@render right()}{/if}</div>
  {/if}
</div>

<style>
  /* Toolbar sits one elevation above the window bg, below the surface header. */
  .page-toolbar {
    display: flex; align-items: center; justify-content: space-between;
    gap: 8px;
    padding: 5px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev-1);
    flex-shrink: 0;
    min-height: 34px;
  }

  .tb-left, .tb-right {
    display: flex; align-items: center; gap: 6px;
    min-width: 0;
  }
  .tb-left { flex: 1; min-width: 0; }

  /* Eyebrow/section labels inside toolbar: 10-11px, 600, uppercase, ls 0.07em.
     Consumers can apply .tb-label or rely on the global .section-label class. */
  .page-toolbar :global(.tb-label) {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--fg-subtle);
  }

  /* Ghost button treatment for toolbar actions — no solid accent in toolbar. */
  .page-toolbar :global(button),
  .page-toolbar :global(.btn-ghost) {
    transition: background var(--dur-page-out) var(--ease-soft),
                color     var(--dur-page-out) var(--ease-soft);
  }
</style>
