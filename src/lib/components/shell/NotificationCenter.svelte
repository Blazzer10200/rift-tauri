<script lang="ts">
  // The "one organized location" for every notification — a bell in the Topbar
  // w/ an unread badge that opens a dropdown listing the full, persisted history
  // grouped by recency. Live toasts still pop bottom-right (ToastHost) and
  // archive in here. Severity → icon/tone mapping mirrors ToastHost so a record
  // looks identical whether it's a live toast or a history row.
  import { Bell, X, CheckCheck, Trash2, BellOff } from "lucide-svelte";
  import { fly, fade } from "svelte/transition";
  import { toast, type NotifyRecord } from "$lib/state/toast.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { portal } from "$lib/actions/portal";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let rootEl = $state<HTMLDivElement | null>(null);
  let panelEl = $state<HTMLDivElement | null>(null);

  // The bell lives in the sidebar footer now, so the panel opens UPWARD from
  // the trigger. It portals to <body> (fixed coords measured at open) because
  // the sidebar island clips overflow and its backdrop-filter would otherwise
  // become the containing block and dislocate the panel.
  let panelPos = $state({ left: 0, bottom: 0, maxH: 560 });
  function toggleCenter() {
    if (toast.centerOpen) {
      toast.closeCenter();
      return;
    }
    if (rootEl) {
      const r = rootEl.getBoundingClientRect();
      panelPos = {
        left: Math.max(8, Math.min(r.left, window.innerWidth - 360 - 8)),
        bottom: Math.max(8, window.innerHeight - r.top + 8),
        // Growing upward from the bell — never taller than the space above it
        // (the mini-peek sidebar parks the bell mid-screen, not at the bottom).
        maxH: Math.max(180, r.top - 16),
      };
    }
    toast.openCenter();
  }

  // Recency buckets — relative to "now" each render. Records are already
  // newest-first in the store, so within a bucket they stay ordered.
  type Group = { key: string; label: string; items: NotifyRecord[] };
  const groups = $derived.by<Group[]>(() => {
    const now = Date.now();
    const buckets: Record<string, NotifyRecord[]> = {
      now: [], earlier: [], today: [], older: [],
    };
    for (const r of toast.history) {
      const age = now - r.ts;
      if (age < 2 * 60_000) buckets.now.push(r);
      else if (age < 60 * 60_000) buckets.earlier.push(r);
      else if (age < 24 * 60 * 60_000) buckets.today.push(r);
      else buckets.older.push(r);
    }
    return [
      { key: "now", label: "Now", items: buckets.now },
      { key: "earlier", label: "Past hour", items: buckets.earlier },
      { key: "today", label: "Today", items: buckets.today },
      { key: "older", label: "Older", items: buckets.older },
    ].filter((g) => g.items.length > 0);
  });

  function relTime(ts: number): string {
    const s = Math.max(0, Math.round((Date.now() - ts) / 1000));
    if (s < 45) return "just now";
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    if (h < 24) return `${h}h ago`;
    const d = Math.round(h / 24);
    return `${d}d ago`;
  }

  function fireAction(r: NotifyRecord) {
    r.action?.onClick();
    toast.closeCenter();
  }

  function onDocMousedown(ev: MouseEvent) {
    if (!toast.centerOpen) return;
    if (!(ev.target instanceof Node)) return;
    // Panel is portaled to <body> — it's no longer inside rootEl, so check both.
    if (rootEl?.contains(ev.target) || panelEl?.contains(ev.target)) return;
    toast.closeCenter();
  }
  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape" && toast.centerOpen) toast.closeCenter();
  }
  $effect(() => {
    window.addEventListener("mousedown", onDocMousedown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDocMousedown);
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="nc" bind:this={rootEl}>
  <button
    class="nc-bell"
    class:active={toast.centerOpen}
    type="button"
    onclick={toggleCenter}
    use:tooltip={"Notifications"}
    aria-label="Notifications"
    aria-haspopup="dialog"
    aria-expanded={toast.centerOpen}
  >
    <Bell size={17} />
    {#if toast.unreadCount > 0}
      <span class="nc-badge" transition:fade={{ duration: reducedMotion ? 0 : 120 }}>
        {toast.unreadCount > 9 ? "9+" : toast.unreadCount}
      </span>
    {/if}
  </button>

  {#if toast.centerOpen}
    <div
      class="panel"
      use:portal
      bind:this={panelEl}
      style="left: {panelPos.left}px; bottom: {panelPos.bottom}px; max-height: min(560px, {panelPos.maxH}px);"
      transition:fly={{ y: reducedMotion ? 0 : 6, duration: reducedMotion ? 0 : 160 }}
      role="dialog"
      aria-label="Notifications"
    >
      <header class="panel-head">
        <span class="ph-title">Notifications</span>
        <div class="ph-actions">
          <button
            class="ph-btn"
            type="button"
            onclick={() => toast.markAllRead()}
            disabled={toast.history.every((r) => r.read)}
            use:tooltip={"Mark all read"}
            aria-label="Mark all read"
          >
            <CheckCheck size={13} />
          </button>
          <button
            class="ph-btn"
            type="button"
            onclick={() => toast.clearHistory()}
            disabled={toast.history.length === 0}
            use:tooltip={"Clear all"}
            aria-label="Clear all notifications"
          >
            <Trash2 size={13} />
          </button>
        </div>
      </header>

      <div class="panel-body">
        {#if toast.history.length === 0}
          <div class="empty">
            <BellOff size={22} />
            <span class="empty-title">You're all caught up</span>
            <span class="empty-sub">Notifications you get will collect here.</span>
          </div>
        {:else}
          {#each groups as group (group.key)}
            <div class="group-label">{group.label}</div>
            {#each group.items as r (r.id)}
              <div
                class="row"
                class:unread={!r.read}
                data-severity={r.severity}
                transition:fade={{ duration: reducedMotion ? 0 : 120 }}
              >
                <span class="row-dot" data-severity={r.severity}></span>
                <div class="row-text">
                  <span class="row-top">
                    <span class="row-title">
                      {r.title}{#if (r.count ?? 1) > 1}<span class="row-count">×{r.count}</span>{/if}
                    </span>
                    <span class="row-time">{relTime(r.ts)}</span>
                  </span>
                  {#if r.detail}
                    <span class="row-detail" class:mono={r.mono}>{r.detail}</span>
                  {/if}
                  {#if r.action}
                    <button class="row-action" type="button" onclick={() => fireAction(r)}>
                      {r.action.label}
                    </button>
                  {/if}
                </div>
                <button
                  class="row-close"
                  type="button"
                  onclick={() => toast.removeFromHistory(r.id)}
                  aria-label="Remove"
                >
                  <X size={12} />
                </button>
              </div>
            {/each}
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  /* Self-contained: bell trigger + history panel. (The TopbarMenu dropdown that
     used to proxy this was dissolved — search was a sidebar/Ctrl+K duplicate and
     notifications deserve one-click access with a visible badge.) */
  .nc { position: relative; display: flex; }

  /* Sized to the sidebar footer-nav items (40×34) so the bell reads as a
     sibling of the workspace icons, not a stray control. */
  .nc-bell {
    position: relative;
    width: 36px; height: 30px;
    display: grid; place-items: center;
    border-radius: 8px;
    color: var(--fg-muted);
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .nc-bell:hover, .nc-bell.active { background: var(--surface-hover); color: var(--fg-2); }

  .nc-badge {
    position: absolute;
    top: 3px; right: 7px;
    min-width: 14px; height: 14px;
    padding: 0 3px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--bg);
    font-size: 9px;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
    box-shadow: 0 0 0 2px var(--bg);
    pointer-events: none;
  }

  .panel {
    /* Portaled to <body>, fixed coords measured at open — opens upward from
       the sidebar-footer bell, escaping the sidebar island's overflow clip
       and backdrop-filter containing block. Floating-island tier: island
       radius + hairline, opaque fill (backdrop-filter is banned on fixed
       overlays — WebView2 mis-composites it). */
    position: fixed;
    width: 340px;
    max-width: calc(100vw - 24px);
    display: flex;
    flex-direction: column;
    background: color-mix(in oklab, var(--fg) 2.2%, var(--bg-elev-1));
    border: 1px solid var(--border-strong);
    border-radius: var(--island-radius);
    box-shadow: var(--shadow-float);
    overflow: hidden;
    z-index: 2100;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 10px 11px 14px;
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .ph-title { font-size: var(--fs-md); font-weight: 600; color: var(--fg); letter-spacing: -0.01em; }
  .ph-actions { display: flex; gap: 4px; }
  .ph-btn {
    width: 26px; height: 26px;
    display: grid; place-items: center;
    border-radius: 7px;
    color: var(--fg-subtle);
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .ph-btn:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .ph-btn:disabled { opacity: 0.3; pointer-events: none; }

  .panel-body { overflow-y: auto; padding: 4px 0 6px; flex: 1; min-height: 0; }

  .group-label {
    padding: 9px 14px 4px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-faint);
  }

  /* Rows are dot-led — severity is a small colored dot on the first text
     line (same language as status pills + toasts), not a boxed icon tile.
     Unread = full-strength dot + bright title; read rows recede. */
  .row {
    --tone: var(--info);
    position: relative;
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: start;
    gap: 10px;
    padding: 8px 10px 8px 12px;
    margin: 0 6px;
    border-radius: 9px;
    transition: background var(--dur-fast);
  }
  .row:hover { background: var(--surface-hover); }
  .row[data-severity="ok"]     { --tone: var(--ok); }
  .row[data-severity="warn"]   { --tone: var(--warn); }
  .row[data-severity="danger"] { --tone: var(--danger); }
  .row[data-severity="info"]   { --tone: var(--info); }
  .row[data-severity="muted"]  { --tone: var(--fg-faint); }

  .row-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: var(--tone);
    opacity: 0.4;
    margin-top: 6px;
    flex-shrink: 0;
    transition: opacity var(--dur-fast), box-shadow var(--dur-fast);
  }
  .row.unread .row-dot {
    opacity: 1;
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--tone) 18%, transparent);
  }

  .row-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .row-top {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .row-title {
    flex: 1;
    min-width: 0;
    font-size: var(--fs-sm);
    color: var(--fg-2);
    line-height: 1.35;
  }
  .row.unread .row-title { color: var(--fg); font-weight: 500; }
  .row-detail {
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    line-height: 1.35;
    word-break: break-word;
  }
  .row-detail.mono { font-family: var(--font-mono); }
  .row-count {
    margin-left: 6px;
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--fg-subtle);
    background: var(--bg-elev-3);
    border-radius: 5px;
    padding: 1px 5px;
    vertical-align: 1px;
  }
  .row-time {
    font-size: 10.5px;
    color: var(--fg-faint);
    flex-shrink: 0;
    transition: opacity var(--dur-fast);
  }
  .row:hover .row-time { opacity: 0; }

  .row-action {
    align-self: flex-start;
    margin-top: 4px;
    font-size: var(--fs-xs);
    color: var(--accent);
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 7px;
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 24%, transparent);
    transition: background var(--dur-fast);
  }
  .row-action:hover { background: color-mix(in oklab, var(--accent) 22%, transparent); }

  /* Close swaps in over the timestamp on hover — one quiet corner, no
     dedicated column reserving space in every row. */
  .row-close {
    position: absolute;
    top: 7px; right: 8px;
    width: 22px; height: 22px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: 6px;
    color: var(--fg-faint);
    opacity: 0;
    transition: background var(--dur-fast), color var(--dur-fast), opacity var(--dur-fast);
  }
  .row:hover .row-close { opacity: 1; }
  .row-close:hover { background: var(--surface-hover); color: var(--fg); }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 38px 20px 42px;
    color: var(--fg-faint);
    text-align: center;
  }
  .empty-title { font-size: var(--fs-sm); color: var(--fg-subtle); font-weight: 500; }
  .empty-sub { font-size: var(--fs-xs); color: var(--fg-faint); }

  @media (prefers-reduced-motion: reduce) {
    .row, .ph-btn, .row-action, .row-close { transition: none; }
  }
</style>
