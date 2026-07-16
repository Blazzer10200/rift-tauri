<script lang="ts">
  // Single-stack toast renderer. Mounted once in AppShell. Reads `toast.items`
  // and animates entry/exit/reflow. Toasts drop in top-center as compact
  // island pills (bottom-right stack read as clutter next to the composer +
  // status bar); severity is a colored dot, not an icon tile — same language
  // as status pills and the notification center rows.
  import { X } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { toast } from "../state/toast.svelte";

  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // DEV-ONLY: CDP hook for driving toasts from scripts/cdp (same pattern as
  // window.__riftDiag). Tree-shaken out of production builds.
  if (import.meta.env.DEV) {
    (window as unknown as Record<string, unknown>).__riftToast = toast;
  }

  // Details clamp to 2 lines; clicking the detail un-clamps that toast (long
  // warnings were unreadable behind a 1-line ellipsis). Set is per-mount —
  // toast ids are session-monotonic so no stale-id leak matters.
  let expandedIds = $state(new Set<number>());
  function toggleDetail(id: number, e: Event) {
    e.stopPropagation(); // never trigger the toast's action from an expand
    const next = new Set(expandedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    expandedIds = next;
  }
</script>

<div class="host" aria-live="polite">
  {#each toast.items as item (item.id)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="toast"
      data-severity={item.severity}
      class:clickable={!!item.action}
      role={item.action ? "button" : "status"}
      tabindex={item.action ? 0 : -1}
      onclick={() => item.action?.onClick()}
      onkeydown={(e) => {
        if (item.action && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          item.action.onClick();
        }
      }}
      onmouseenter={() => toast.pause(item.id)}
      onmouseleave={() => toast.resume(item.id)}
      animate:flip={{ duration: 180 }}
      in:fly={{ y: reducedMotion ? 0 : -10, duration: reducedMotion ? 0 : 220 }}
      out:fly={{ y: reducedMotion ? 0 : -6, duration: reducedMotion ? 0 : 140 }}
    >
      <span class="dot" data-severity={item.severity}></span>
      <span class="text">
        <span class="title">{item.title}</span>
        {#if item.detail}
          <button
            type="button"
            class="detail"
            class:mono={item.mono}
            class:expanded={expandedIds.has(item.id)}
            onclick={(e) => toggleDetail(item.id, e)}
            title={expandedIds.has(item.id) ? undefined : item.detail}
          >{item.detail}</button>
        {/if}
      </span>
      {#if item.action}
        <span class="action">{item.action.label}</span>
      {/if}
      <button
        class="close"
        type="button"
        onclick={(e) => { e.stopPropagation(); toast.dismiss(item.id); }}
        aria-label="Dismiss"
      >
        <X size={11}/>
      </button>
    </div>
  {/each}
</div>

<style>
  .host {
    /* Top-center of the MAIN ISLAND, not the window — the rail offsets the
       island right, so window-center reads visibly off-axis. --rail-w tweens
       on <html> during collapse, so the stack glides with the island.
       column-reverse keeps the newest toast nearest the top edge. */
    position: fixed;
    top: 54px;
    left: calc((100vw + var(--rail-w, 0px)) / 2);
    transform: translateX(-50%);
    /* Toasts must outrank every transient overlay or they render but can't be
       clicked (browser overflow scrim z-999, Select menus z-1200 — the old
       "update toast is just for show" bug). Tooltips/splash (9999) stay clear. */
    z-index: 2000;
    display: flex;
    flex-direction: column-reverse;
    align-items: center;
    gap: 6px;
    pointer-events: none;
    max-width: min(560px, calc(100vw - var(--rail-w, 0px) - 32px));
  }
  .toast {
    pointer-events: auto;
    --tone: var(--info);
    /* Opaque, NO backdrop-filter — WebView2 mis-composites it on fixed overlay
       elements (garbage rects; same measured bug class fixed on the update
       pill in v0.8.10). The severity wash is a flat color-mix layered into the
       bg (no gradient image) so it stays opaque. */
    background:
      linear-gradient(100deg, color-mix(in oklab, var(--tone) 7%, transparent), transparent 50%),
      var(--bg-elev-2);
    border: 1px solid color-mix(in oklch, var(--tone) 20%, var(--border-strong));
    border-radius: 12px;
    padding: 8px 10px 8px 13px;
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    gap: 10px;
    align-items: center;
    color: var(--fg);
    box-shadow: var(--shadow-float);
    width: max-content;
    min-width: 240px;
    max-width: 100%;
    font: inherit;
    text-align: left;
    transition: border-color var(--dur-fast) ease-out, transform var(--dur-fast) var(--ease-soft), box-shadow var(--dur-fast) ease-out;
  }
  .toast.clickable { cursor: pointer; }
  .toast.clickable:hover {
    border-color: color-mix(in oklch, var(--tone) 38%, var(--border-strong));
    transform: translateY(1px);
  }

  .toast[data-severity="ok"]     { --tone: var(--ok); }
  .toast[data-severity="warn"]   { --tone: var(--warn); }
  .toast[data-severity="danger"] { --tone: var(--danger); }
  .toast[data-severity="info"]   { --tone: var(--info); }
  .toast[data-severity="muted"]  { --tone: var(--border-strong); }

  .dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: var(--tone);
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--tone) 18%, transparent);
    flex-shrink: 0;
  }
  .toast[data-severity="muted"] .dot { background: var(--fg-faint); box-shadow: none; }

  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .title {
    color: var(--fg);
    font-size: var(--fs-sm);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .detail {
    /* Button reset — reads as text, clicks to un-clamp. */
    background: none; border: 0; padding: 0; margin: 0;
    font: inherit; text-align: left; cursor: pointer;
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .detail.expanded { -webkit-line-clamp: unset; line-clamp: unset; display: block; }
  .detail.mono { font-family: var(--font-mono); }

  .action {
    font-size: var(--fs-xs);
    color: var(--accent);
    font-weight: 600;
    padding: 3px 7px;
    border-radius: var(--radius-xs);
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 24%, transparent);
    flex-shrink: 0;
  }
  .toast.clickable:hover .action {
    background: color-mix(in oklab, var(--accent) 20%, transparent);
  }

  .close {
    width: 20px; height: 20px;
    display: inline-flex; align-items: center; justify-content: center;
    background: transparent;
    border: 0;
    border-radius: var(--radius-xs);
    color: var(--fg-faint);
    cursor: pointer;
    opacity: 0;
    transition: background var(--dur-fast) ease, color var(--dur-fast) ease, opacity var(--dur-fast) ease;
    flex-shrink: 0;
  }
  .toast:hover .close, .close:focus-visible { opacity: 1; }
  .close:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }

  @media (prefers-reduced-motion: reduce) {
    .toast { transition: none; }
  }
</style>
