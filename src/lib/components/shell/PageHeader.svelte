<script lang="ts">
  import type { Snippet } from "svelte";
  import type { Icon as LucideIcon } from "lucide-svelte";

  type Tone = "accent" | "info" | "warn" | "danger" | "ok" | "neutral";

  let {
    icon,
    title,
    subtitle,
    badge,
    tone = "accent",
    actions,
    extras,
  }: {
    icon?: typeof LucideIcon;
    title: string;
    subtitle?: string;
    badge?: string;
    tone?: Tone;
    actions?: Snippet;
    extras?: Snippet;
  } = $props();
</script>

<header class="page-head" data-tone={tone}>
  <div class="left">
    {#if icon}
      {@const Icon = icon}
      <span class="head-icon" data-tone={tone}><Icon size={14} /></span>
    {/if}
    <span class="head-title">{title}</span>
    {#if badge}
      <span class="head-badge" data-tone={tone}>{badge}</span>
    {/if}
    {#if subtitle}
      <span class="head-sub">{subtitle}</span>
    {/if}
    {#if extras}{@render extras()}{/if}
  </div>
  {#if actions}
    <div class="right">{@render actions()}</div>
  {/if}
</header>

<style>
  .page-head {
    position: relative;
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex-shrink: 0;
    min-height: 44px;
  }
  /* Tone accent stripe — 2px along the bottom edge, ties each tab to its tone. */
  .page-head::after {
    content: "";
    position: absolute;
    left: 0; right: 0; bottom: -1px;
    height: 2px;
    background: var(--tone-color, var(--accent));
    opacity: 0.55;
    pointer-events: none;
    transition: background var(--dur-page-out) var(--ease-soft),
                opacity   var(--dur-page-out) var(--ease-soft);
  }
  .page-head[data-tone="info"]    { --tone-color: var(--info); }
  .page-head[data-tone="warn"]    { --tone-color: var(--warn); }
  .page-head[data-tone="danger"]  { --tone-color: var(--danger); }
  .page-head[data-tone="ok"]      { --tone-color: var(--ok); }
  .page-head[data-tone="neutral"] { --tone-color: var(--fg-muted); }
  .page-head[data-tone="neutral"]::after { opacity: 0.2; }

  .left, .right {
    display: flex; align-items: center; gap: 8px;
    min-width: 0;
  }
  .left { flex: 1; min-width: 0; }

  .head-icon {
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--accent);
    transition: color var(--dur-page-out) var(--ease-soft);
    flex-shrink: 0;
  }
  .head-icon[data-tone="info"]    { color: var(--info); }
  .head-icon[data-tone="warn"]    { color: var(--warn); }
  .head-icon[data-tone="danger"]  { color: var(--danger); }
  .head-icon[data-tone="ok"]      { color: var(--ok); }
  .head-icon[data-tone="neutral"] { color: var(--fg-muted); }

  .head-title {
    font-size: var(--fs-lg);
    font-weight: 650;
    color: var(--fg);
    white-space: nowrap;
    letter-spacing: -0.012em;
  }

  /* Badge: eyebrow/label spec — 11px, 600, uppercase, ls 0.07em */
  .head-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 7px;
    background: var(--warn-soft);
    color: var(--warn);
    border-radius: var(--radius-xs);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    line-height: 1.4;
  }
  .head-badge[data-tone="accent"] { background: var(--accent-soft); color: var(--accent); }
  .head-badge[data-tone="info"]   { background: var(--info-soft);   color: var(--info); }
  .head-badge[data-tone="danger"] { background: var(--danger-soft); color: var(--danger); }
  .head-badge[data-tone="ok"]     { background: var(--ok-soft);     color: var(--ok); }

  .head-sub {
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    letter-spacing: 0.01em;
  }

  /* Right-slot: action buttons land here — ghost treatment by default.
     Consumers pass <button class="btn"> or the .btn global class handles it. */
  .right {
    flex-shrink: 0;
  }
</style>
