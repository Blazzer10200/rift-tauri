<script lang="ts">
  // Shared block-header contract for tool blocks in BOTH trees (live stream +
  // history ToolChip): [lead] [title………] [meta: pill · duration · copy · chev].
  // Consumers provide lead/title as snippets and keep their own container
  // chrome; the META CLUSTER (pill tones, duration residue, copy affordance,
  // chevron) is styled once here so every block reads as one family.
  import { Check, ChevronRight, Copy, Loader2, X } from "@lucide/svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import type { Snippet } from "svelte";

  let {
    lead = null,
    title,
    meta = null,
    pill = null,
    durationLabel = null,
    copyText = null,
    expandable = false,
    expanded = false,
    onToggle = null,
    spin = false,
  }: {
    lead?: Snippet | null;
    title: Snippet;
    meta?: Snippet | null;
    pill?: { text: string; tone: "running" | "ok" | "bad" | "neutral" } | null;
    durationLabel?: string | null;
    copyText?: (() => string) | null;
    expandable?: boolean;
    expanded?: boolean;
    onToggle?: (() => void) | null;
    spin?: boolean;
  } = $props();

  // Pill tones map onto the global `.tile-pill` status classes (app.css).
  const PILL_TONE = { running: "is-live", ok: "is-ok", bad: "is-bad", neutral: "" } as const;

  let copied = $state(false);
  let copyFailed = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => () => { if (copyTimer) clearTimeout(copyTimer); });
  async function copy(e: Event) {
    e.stopPropagation();
    if (!copyText) return;
    if (copyTimer) clearTimeout(copyTimer);
    try {
      await navigator.clipboard.writeText(copyText());
      copied = true;
      copyFailed = false;
    } catch (err) {
      console.warn("block copy failed", err);
      copied = false;
      copyFailed = true;
    }
    copyTimer = setTimeout(() => { copied = false; copyFailed = false; copyTimer = null; }, 1200);
  }
</script>

<button
  class="bh"
  type="button"
  disabled={!expandable}
  onclick={() => onToggle?.()}
  aria-expanded={expandable ? expanded : undefined}
>
  {#if lead}{@render lead()}{/if}
  <span class="bh-title">{@render title()}</span>
  <span class="bh-meta">
    {#if meta}{@render meta()}{/if}
    {#if pill}<span class="tile-pill {PILL_TONE[pill.tone]}">{pill.text}</span>{/if}
    {#if spin}<span class="bh-spin" aria-hidden="true"><Loader2 size={11} /></span>{/if}
    {#if durationLabel}<span class="tile-dur" use:tooltip={"Wall-clock duration"}>{durationLabel}</span>{/if}
    {#if copyText}
      <span
        class="bh-copy"
        class:copied
        class:failed={copyFailed}
        role="button"
        tabindex="0"
        aria-label={copyFailed ? "Copy failed" : "Copy"}
        use:tooltip={copyFailed ? "Copy failed" : "Copy"}
        onclick={copy}
        onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); void copy(e); } }}
      >{#if copied}<Check size={11} strokeWidth={2.5} />{:else if copyFailed}<X size={11} strokeWidth={2.5} />{:else}<Copy size={11} />{/if}</span>
    {/if}
    {#if expandable}<span class="tile-chev" class:open={expanded}><ChevronRight size={13} /></span>{/if}
  </span>
</button>

<style>
  .bh {
    display: flex; align-items: center; gap: 8px;
    width: 100%; min-width: 0;
    padding: 0; border: 0; background: transparent;
    font: inherit; color: var(--fg-2);
    text-align: left; cursor: pointer;
  }
  .bh:disabled { cursor: default; }
  .bh-title { flex: 1; min-width: 0; display: inline-flex; align-items: center; gap: 6px;
    overflow: hidden; white-space: nowrap; }

  /* ── meta cluster: pill · duration · copy · chevron. Pill, duration and
     chevron are the global `.tile-*` classes (app.css) so every block reads
     as one family; only the copy affordance + spinner are local. ── */
  .bh-meta { flex: none; display: inline-flex; align-items: center; gap: 7px; }
  .bh-spin { display: inline-flex; color: var(--accent); }
  .bh-spin :global(svg) { animation: bh-spin 1s linear infinite; }
  @keyframes bh-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
  .bh-copy { flex: none; display: inline-grid; place-items: center; width: 20px; height: 20px;
    border-radius: var(--radius-xs); color: var(--fg-faint); opacity: 0;
    transition: opacity var(--dur-fast), color var(--dur-fast), background var(--dur-fast); }
  .bh:hover .bh-copy, .bh-copy:focus-visible { opacity: 1; }
  .bh-copy:hover { color: var(--fg-2); background: var(--surface-hover); }
  .bh-copy.copied { color: var(--ok); opacity: 1; }
  .bh-copy.failed { color: var(--danger); opacity: 1; }
  .bh:hover:not(:disabled) .tile-chev { color: var(--fg-muted); }
  @media (prefers-reduced-motion: reduce) { .bh-spin :global(svg) { animation: none; } }
</style>
