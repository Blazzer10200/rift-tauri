<script lang="ts">
  // Dedicated, STABLE "update available" affordance. Replaces the sticky toast
  // that lived in the shared toast stack — that toast sat at the top of an
  // upward-growing, bottom-anchored, FLIP-animated column, so every other toast
  // that appeared/expired slid it out from under the cursor and ~half the
  // clicks missed (the long-standing "update button won't click" bug).
  //
  // This is a singleton fixed element: it never reflows, animates only its own
  // enter/leave, and owns its corner — so the click always lands. Click the
  // body → open the dialog; click × → snooze this version.
  import { Sparkles, ArrowRight, X } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { updates } from "../state/updates.svelte";

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
</script>

{#if updates.pillVisible && updates.info}
  <div
    class="pill-host"
    transition:fly={{ y: reducedMotion ? 0 : 10, duration: reducedMotion ? 0 : 200 }}
  >
    <button
      class="upd-pill"
      type="button"
      onclick={() => updates.open()}
      aria-label="Update available — open updater"
    >
      <span class="icon"><Sparkles size={13} /></span>
      <span class="text">
        <span class="title">Update available</span>
        <span class="diff mono">
          v{updates.currentVersion}<ArrowRight size={10} class="arr" />v{updates.info.version}{#if updates.sizeLabel}<span class="size"> · {updates.sizeLabel}</span>{/if}
        </span>
      </span>
      <span class="cta">View</span>
    </button>
    <button
      class="snooze"
      type="button"
      onclick={(e) => { e.stopPropagation(); updates.snooze(); }}
      aria-label="Remind me later"
    >
      <X size={12} />
    </button>
  </div>
{/if}

<style>
  .pill-host {
    position: fixed;
    bottom: 28px;
    right: 16px;
    /* Above interactive layers, matching the toast host. A transient error
       toast (z 2000) sits above it — but that only ever shows with the dialog
       open, which hides this pill, so they never collide. */
    z-index: 1990;
    display: flex;
    align-items: stretch;
    gap: 0;
    max-width: calc(100vw - 32px);
  }

  .upd-pill {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 11px;
    /* Solid opaque bg — NO backdrop-filter. WebView2 mis-composites
       backdrop-filter on a bottom-anchored fixed element as garbage green/black
       rects AND collapses its layout box to ~20px (measured). Same bug the
       Harness hero avoids by staying opaque. */
    background: var(--bg-elev-1);
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border-strong));
    border-left: 2px solid var(--accent);
    border-radius: 10px 0 0 10px;
    color: var(--fg);
    font: inherit;
    text-align: left;
    cursor: pointer;
    box-shadow:
      0 8px 22px -10px oklch(0 0 0 / 0.5),
      0 0 18px color-mix(in oklab, var(--accent) 18%, transparent);
    transition: border-color 140ms ease-out, box-shadow 140ms ease-out;
  }
  .upd-pill:hover {
    border-color: color-mix(in oklab, var(--accent) 48%, var(--border-strong));
    box-shadow:
      0 10px 26px -10px oklch(0 0 0 / 0.55),
      0 0 26px color-mix(in oklab, var(--accent) 30%, transparent);
  }

  .icon {
    width: 24px; height: 24px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: color-mix(in oklab, var(--accent) 22%, var(--bg-elev-3));
    color: var(--accent);
    flex-shrink: 0;
  }

  .text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .title {
    font-size: var(--fs-sm);
    color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .diff {
    display: inline-flex; align-items: center; gap: 2px;
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    white-space: nowrap;
  }
  .diff :global(.arr) { color: var(--fg-faint); margin: 0 1px; }
  .size { color: var(--fg-faint); }

  .cta {
    font-size: var(--fs-xs);
    color: var(--accent);
    font-weight: 600;
    padding: 3px 8px;
    border-radius: var(--radius-xs);
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 28%, transparent);
    flex-shrink: 0;
  }
  .upd-pill:hover .cta {
    background: color-mix(in oklab, var(--accent) 22%, transparent);
  }

  .snooze {
    display: inline-flex; align-items: center; justify-content: center;
    width: 26px;
    background: var(--bg-elev-1);
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border-strong));
    border-left: 0;
    border-radius: 0 10px 10px 0;
    color: var(--fg-faint);
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .snooze:hover { background: var(--surface-hover); color: var(--fg); }

  @media (prefers-reduced-motion: reduce) {
    .upd-pill, .snooze { transition: none; }
  }
</style>
