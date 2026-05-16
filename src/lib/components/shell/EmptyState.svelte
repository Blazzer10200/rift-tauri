<script lang="ts">
  import type { Snippet } from "svelte";
  import type { Icon as LucideIcon } from "lucide-svelte";

  type Tone = "accent" | "info" | "warn" | "danger" | "ok" | "neutral";

  let {
    icon,
    title,
    hint,
    tone = "accent",
    children,
  }: {
    icon?: typeof LucideIcon;
    title: string;
    hint?: string;
    tone?: Tone;
    children?: Snippet;
  } = $props();
</script>

<div class="empty">
  {#if icon}
    {@const Icon = icon}
    <div class="empty-glyph" data-tone={tone}><Icon size={22} /></div>
  {/if}
  <h2 class="empty-title">{title}</h2>
  {#if hint}<p class="empty-hint">{hint}</p>{/if}
  {#if children}
    <div class="empty-body">{@render children()}</div>
  {/if}
</div>

<style>
  .empty {
    flex: 1;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 12px;
    padding: 48px 24px;
    text-align: center;
  }
  .empty-glyph {
    width: 52px; height: 52px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    display: inline-flex; align-items: center; justify-content: center;
    margin-bottom: 4px;
  }
  .empty-glyph[data-tone="info"]    { background: var(--info-soft);   color: var(--info); }
  .empty-glyph[data-tone="warn"]    { background: var(--warn-soft);   color: var(--warn); }
  .empty-glyph[data-tone="danger"]  { background: var(--danger-soft); color: var(--danger); }
  .empty-glyph[data-tone="ok"]      { background: var(--ok-soft);     color: var(--ok); }
  .empty-glyph[data-tone="neutral"] { background: var(--bg-elev-2);   color: var(--fg-muted); }
  .empty-title {
    font-size: var(--fs-xl);
    font-weight: 600;
    color: var(--fg);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .empty-hint {
    font-size: var(--fs-sm);
    color: var(--fg-muted);
    margin: 0;
    max-width: 440px;
    line-height: 1.5;
  }
  .empty-body {
    margin-top: 8px;
    max-width: 480px;
    width: 100%;
    display: flex; flex-direction: column; gap: 10px;
    align-items: stretch;
  }
</style>
