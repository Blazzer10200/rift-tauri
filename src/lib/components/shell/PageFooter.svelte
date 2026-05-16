<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    primary,
    secondary,
    active = false,
  }: {
    primary?: Snippet;
    secondary?: Snippet;
    active?: boolean;
  } = $props();
</script>

<footer class="page-foot" class:active>
  <div class="foot-primary">{#if primary}{@render primary()}{/if}</div>
  <div class="foot-spacer"></div>
  <div class="foot-secondary">{#if secondary}{@render secondary()}{/if}</div>
</footer>

<style>
  .page-foot {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
    min-height: 44px;
    transition: background 140ms ease;
  }
  .page-foot.active {
    background: color-mix(in oklch, var(--accent) 6%, var(--bg));
  }
  .foot-spacer { flex: 1; }
  .foot-primary, .foot-secondary {
    display: flex; align-items: center; gap: 8px;
  }
</style>
