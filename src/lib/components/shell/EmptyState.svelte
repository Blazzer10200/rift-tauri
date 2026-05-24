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
  .empty > * {
    animation: enter 280ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .empty > :nth-child(1) { animation-delay: 0ms; }
  .empty > :nth-child(2) { animation-delay: 50ms; }
  .empty > :nth-child(3) { animation-delay: 100ms; }
  .empty > :nth-child(4) { animation-delay: 150ms; }
  @media (prefers-reduced-motion: reduce) {
    .empty > * { animation: none; }
  }

  .empty-glyph {
    position: relative;
    width: 56px; height: 56px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    display: inline-flex; align-items: center; justify-content: center;
    margin-bottom: 4px;
    isolation: isolate;
    box-shadow:
      0 0 0 1px color-mix(in oklch, currentColor 25%, transparent),
      0 14px 36px -8px color-mix(in oklch, currentColor 25%, transparent);
  }
  /* Soft conic halo behind the glyph — matches the assistant-landing glyph
     treatment so empty states across the app share a vocabulary. */
  .empty-glyph::before {
    content: "";
    position: absolute;
    inset: -8px;
    border-radius: 50%;
    background: conic-gradient(
      from 0deg,
      color-mix(in oklch, currentColor 65%, transparent),
      transparent 30%,
      color-mix(in oklch, currentColor 45%, transparent) 55%,
      transparent 80%,
      color-mix(in oklch, currentColor 65%, transparent)
    );
    filter: blur(8px);
    opacity: 0.45;
    z-index: -1;
  }
  @media (prefers-reduced-motion: no-preference) {
    .empty-glyph::before { animation: empty-halo-spin 10s linear infinite; }
  }
  @keyframes empty-halo-spin { to { transform: rotate(360deg); } }
  .empty-glyph[data-tone="info"]    { background: var(--info-soft);   color: var(--info); }
  .empty-glyph[data-tone="warn"]    { background: var(--warn-soft);   color: var(--warn); }
  .empty-glyph[data-tone="danger"]  { background: var(--danger-soft); color: var(--danger); }
  .empty-glyph[data-tone="ok"]      { background: var(--ok-soft);     color: var(--ok); }
  .empty-glyph[data-tone="neutral"] {
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    box-shadow: 0 0 0 1px var(--border);
  }
  .empty-glyph[data-tone="neutral"]::before { display: none; }
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
    display: flex; flex-direction: row; flex-wrap: wrap;
    gap: 10px;
    align-items: center; justify-content: center;
  }
</style>
