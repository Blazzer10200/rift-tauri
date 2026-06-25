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
    icon?: Snippet;
    chip?: Snippet;
    children?: Snippet;
  }
  const { eyebrow, title, desc, padBottom = true, maxWidth = 820, icon, chip, children }: Props = $props();
</script>

<div class="sb-topbar" style="--hero-w: {maxWidth}px">
  <div class="sb-hero" class:pb={padBottom}>
    <div class="sb-hero-l">
      {#if icon}
        <div class="sb-hero-ic">{@render icon()}</div>
      {/if}
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
  .sb-topbar { flex: none; padding: 26px 40px 0; background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 5%, transparent), transparent 140px); border-bottom: 1px solid var(--border); }
  .sb-hero { display: flex; align-items: flex-start; justify-content: space-between; gap: 24px; max-width: var(--hero-w, 820px); margin: 0 auto; }
  .sb-hero.pb { padding-bottom: 26px; }
  /* tab bar / extra content aligns to the same centered column as the hero */
  .sb-hero-extra { max-width: var(--hero-w, 820px); margin: 0 auto; }
  .sb-hero-l { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .sb-hero-ic { width: 44px; height: 44px; border-radius: 12px; flex: none; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .sb-hero-ic :global(svg) { color: var(--accent); }
  .sb-eyebrow { font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-subtle); }
  .sb-hero-tt { font-size: 24px; font-weight: 760; letter-spacing: -0.025em; line-height: 1.1; margin-top: 1px; }
  .sb-hero-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.45; max-width: 64ch; }
  .sb-hero-r { display: flex; align-items: center; gap: 8px; flex: none; }
</style>
