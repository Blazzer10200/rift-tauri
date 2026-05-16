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
  .page-toolbar {
    display: flex; align-items: center; justify-content: space-between;
    gap: 8px;
    padding: 6px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
    min-height: 36px;
  }
  .tb-left, .tb-right {
    display: flex; align-items: center; gap: 8px;
    min-width: 0;
  }
  .tb-left { flex: 1; min-width: 0; }
</style>
