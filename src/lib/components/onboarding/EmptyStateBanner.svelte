<script lang="ts">
  import { Info, ArrowRight } from "lucide-svelte";

  type Props = {
    title: string;
    body: string;
    actionLabel?: string;
    onAction?: () => void;
  };
  const { title, body, actionLabel, onAction }: Props = $props();
</script>

<div class="empty-banner">
  <div class="icon-cell"><Info size={18} /></div>
  <div class="body-cell">
    <div class="title">{title}</div>
    <div class="body">{body}</div>
  </div>
  {#if actionLabel}
    <button class="btn primary sm" type="button" onclick={() => onAction?.()}>
      {actionLabel}
      <ArrowRight size={12} />
    </button>
  {/if}
</div>

<style>
  .empty-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    background: color-mix(in oklch, var(--accent) 8%, var(--bg-elev-1));
    border: 1px solid color-mix(in oklch, var(--accent) 26%, var(--border));
    border-radius: var(--radius-md);
    color: var(--fg);
  }
  .icon-cell {
    flex-shrink: 0;
    width: 28px; height: 28px;
    display: flex; align-items: center; justify-content: center;
    border-radius: var(--radius-sm);
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    color: var(--accent);
  }
  .body-cell { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .title { font-size: 13px; font-weight: 600; color: var(--fg); }
  .body { font-size: 12.5px; color: var(--fg-muted); line-height: 1.45; }
</style>
