<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    eyebrow: string;
    title: string;
    desc: string;
    /** Add bottom padding on the hero row (false when a tab bar follows). */
    padBottom?: boolean;
    /** Inner column width — hero + tab bar align to this so the body below can match. */
    maxWidth?: number;
    chip?: Snippet;
    children?: Snippet;
  }
  const { eyebrow, title, desc, padBottom = true, maxWidth = 820, chip, children }: Props = $props();
</script>

<div class="sb-topbar" style="--hero-w: {maxWidth}px">
  <div class="sb-hero" class:pb={padBottom}>
    <div class="sb-hero-l">
      <div class="sb-hero-tx">
        <div class="sb-eyebrow">{eyebrow}</div>
        <div class="sb-hero-tt">{title}</div>
        <div class="sb-hero-sub">{desc}</div>
      </div>
    </div>
    {#if chip}
      <div class="sb-hero-r">{@render chip()}</div>
    {/if}
  </div>
  {#if children}<div class="sb-hero-extra">{@render children()}</div>{/if}
</div>

<style>
  /* Transparent chrome — the hero sits directly on the page texture instead of
     its own tinted band, so the top of every page reads as one surface. */
  .sb-topbar { flex: none; padding: 30px 40px 0; }
  .sb-hero { display: flex; align-items: flex-start; justify-content: space-between; gap: 24px; max-width: var(--hero-w, 820px); margin: 0 auto; }
  .sb-hero.pb { padding-bottom: 26px; }
  /* tab bar / extra content aligns to the same centered column as the hero */
  .sb-hero-extra { max-width: var(--hero-w, 820px); margin: 0 auto; }
  .sb-hero-l { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .sb-eyebrow { display: flex; align-items: center; gap: 8px; font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: color-mix(in oklab, var(--accent) 80%, var(--fg-muted)); }
  .sb-eyebrow::before { content: ""; width: 16px; height: 2px; border-radius: 999px; background: var(--accent); }
  .sb-hero-tt { font-size: 24px; font-weight: 760; letter-spacing: -0.025em; line-height: 1.1; margin-top: 1px; }
  .sb-hero-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.45; max-width: 64ch; }
  .sb-hero-r { display: flex; align-items: center; gap: 8px; flex: none; }
</style>
