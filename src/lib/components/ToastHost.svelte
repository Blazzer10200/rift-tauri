<script lang="ts">
  // Single-stack toast renderer. Mounted once in AppShell. Reads `toast.items`
  // and animates entry/exit/reflow. Replaces ActivityToast + UpdateToast.
  import { X, CheckCircle2, AlertTriangle, AlertCircle, Info, Bell } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { toast, type ToastSeverity } from "../state/toast.svelte";

  // Per-severity fallback icon. Callers may omit `icon` (e.g. info-severity
  // toasts pushed from `.ts` state modules that shouldn't import UI components);
  // the renderer owns the icon mapping so those imports stay out of state.
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  const SEVERITY_ICON: Record<ToastSeverity, typeof Info> = {
    ok: CheckCircle2,
    warn: AlertTriangle,
    danger: AlertCircle,
    info: Info,
    muted: Bell,
  };

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
    {@const Icon = item.icon ?? SEVERITY_ICON[item.severity]}
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
      in:fly={{ y: reducedMotion ? 0 : 10, duration: reducedMotion ? 0 : 200 }}
      out:fly={{ y: reducedMotion ? 0 : 6, duration: reducedMotion ? 0 : 140 }}
    >
      <span class="kind" data-severity={item.severity}>
        <Icon size={14}/>
      </span>
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
    position: fixed;
    bottom: 28px;
    right: 16px;
    /* Toasts must outrank every transient overlay or they render but can't be
       clicked (e.g. the browser overflow scrim at z-999, Select menus at z-1200
       blanket the old z-60 toast invisibly — the "update toast is just for show"
       bug). Sit above interactive layers; tooltips/splash (9999, pointer-events
       none) stay clear. */
    z-index: 2000;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
    pointer-events: none;
    max-width: calc(100vw - 32px);
  }
  .toast {
    pointer-events: auto;
    --tone: var(--info);
    /* Opaque, NO backdrop-filter — WebView2 mis-composites it on bottom-anchored
       fixed elements (garbage rects + collapsed box; same measured bug class
       fixed on the update pill in v0.8.10). The severity wash is a flat
       color-mix layered into the bg (no gradient image) so it stays opaque. */
    background:
      linear-gradient(100deg, color-mix(in oklab, var(--tone) 9%, transparent), transparent 46%),
      var(--bg-elev-1);
    border: 1px solid color-mix(in oklch, var(--tone) 22%, var(--border-strong));
    border-radius: 11px;
    padding: 9px 10px 9px 11px;
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    gap: 11px;
    align-items: center;
    color: var(--fg);
    box-shadow:
      0 14px 34px -14px oklch(0 0 0 / 0.6),
      0 1px 3px -1px oklch(0 0 0 / 0.4),
      inset 0 1px 0 oklch(1 0 0 / 0.03);
    width: 348px;
    max-width: 100%;
    font: inherit;
    text-align: left;
    transition: border-color var(--dur-fast) ease-out, transform var(--dur-fast) var(--ease-soft), box-shadow var(--dur-fast) ease-out;
  }
  .toast.clickable { cursor: pointer; }
  .toast.clickable:hover {
    border-color: color-mix(in oklch, var(--tone) 40%, var(--border-strong));
    transform: translateY(-1px);
    box-shadow:
      0 18px 40px -14px oklch(0 0 0 / 0.65),
      0 1px 3px -1px oklch(0 0 0 / 0.4),
      inset 0 1px 0 oklch(1 0 0 / 0.04);
  }

  .toast[data-severity="ok"]     { --tone: var(--ok); }
  .toast[data-severity="warn"]   { --tone: var(--warn); }
  .toast[data-severity="danger"] { --tone: var(--danger); }
  .toast[data-severity="info"]   { --tone: var(--info); }
  .toast[data-severity="muted"]  { --tone: var(--border-strong); }

  .kind {
    width: 28px; height: 28px;
    border-radius: 8px;
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-3);
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .kind[data-severity="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kind[data-severity="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .kind[data-severity="danger"] { background: var(--danger-soft); color: var(--danger); }
  .kind[data-severity="info"]   { background: var(--info-soft);   color: var(--info); }

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
    transition: background var(--dur-fast) ease, color var(--dur-fast) ease;
    flex-shrink: 0;
  }
  .close:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }

  @media (prefers-reduced-motion: reduce) {
    .toast { transition: none; }
  }
</style>
