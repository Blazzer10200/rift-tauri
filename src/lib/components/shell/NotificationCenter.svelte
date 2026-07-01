<script lang="ts">
  // The "one organized location" for every notification — a bell in the Topbar
  // w/ an unread badge that opens a dropdown listing the full, persisted history
  // grouped by recency. Live toasts still pop bottom-right (ToastHost) and
  // archive in here. Severity → icon/tone mapping mirrors ToastHost so a record
  // looks identical whether it's a live toast or a history row.
  import {
    Bell, X, CheckCircle2, AlertTriangle, AlertCircle, Info,
    CheckCheck, Trash2, BellOff,
  } from "lucide-svelte";
  import { fly, fade } from "svelte/transition";
  import { toast, type ToastSeverity, type NotifyRecord } from "$lib/state/toast.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const SEVERITY_ICON: Record<ToastSeverity, typeof Info> = {
    ok: CheckCircle2,
    warn: AlertTriangle,
    danger: AlertCircle,
    info: Info,
    muted: Bell,
  };

  let rootEl = $state<HTMLDivElement | null>(null);

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
    if (rootEl && ev.target instanceof Node && rootEl.contains(ev.target)) return;
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
    onclick={() => (toast.centerOpen ? toast.closeCenter() : toast.openCenter())}
    use:tooltip={"Notifications"}
    aria-label="Notifications"
    aria-haspopup="dialog"
    aria-expanded={toast.centerOpen}
  >
    <Bell size={15} />
    {#if toast.unreadCount > 0}
      <span class="nc-badge" transition:fade={{ duration: reducedMotion ? 0 : 120 }}>
        {toast.unreadCount > 9 ? "9+" : toast.unreadCount}
      </span>
    {/if}
  </button>

  {#if toast.centerOpen}
    <div
      class="panel"
      transition:fly={{ y: reducedMotion ? 0 : -6, duration: reducedMotion ? 0 : 160 }}
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
              {@const Icon = SEVERITY_ICON[r.severity]}
              <div
                class="row"
                class:unread={!r.read}
                data-severity={r.severity}
                transition:fade={{ duration: reducedMotion ? 0 : 120 }}
              >
                <span class="row-kind" data-severity={r.severity}>
                  <Icon size={13} />
                </span>
                <div class="row-text">
                  <span class="row-title">
                    {r.title}{#if (r.count ?? 1) > 1}<span class="row-count">×{r.count}</span>{/if}
                  </span>
                  {#if r.detail}
                    <span class="row-detail" class:mono={r.mono}>{r.detail}</span>
                  {/if}
                  <span class="row-time">{relTime(r.ts)}</span>
                </div>
                {#if r.action}
                  <button class="row-action" type="button" onclick={() => fireAction(r)}>
                    {r.action.label}
                  </button>
                {/if}
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

  .nc-bell {
    position: relative;
    width: 30px; height: 30px;
    display: grid; place-items: center;
    border-radius: 8px;
    color: var(--fg-subtle);
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .nc-bell:hover, .nc-bell.active { background: var(--surface-hover); color: var(--fg-2); }

  .nc-badge {
    position: absolute;
    top: 2px; right: 2px;
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
    /* Anchored below the bell (was fixed top:46px — that overlapped the bell
       itself once the update banner pushed the topbar down, so clicking the
       bell to close actually hit the panel header). */
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 360px;
    max-width: calc(100vw - 24px);
    max-height: min(560px, calc(100vh - 140px));
    display: flex;
    flex-direction: column;
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow:
      0 18px 48px -16px oklch(0 0 0 / 0.6),
      0 2px 8px -4px oklch(0 0 0 / 0.4);
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

  .row {
    --tone: var(--info);
    position: relative;
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: start;
    gap: 10px;
    padding: 9px 10px 9px 14px;
    margin: 0 6px;
    border-radius: 9px;
    transition: background var(--dur-fast);
  }
  .row:hover { background: var(--surface-hover); }
  .row[data-severity="ok"]     { --tone: var(--ok); }
  .row[data-severity="warn"]   { --tone: var(--warn); }
  .row[data-severity="danger"] { --tone: var(--danger); }
  .row[data-severity="info"]   { --tone: var(--info); }
  .row[data-severity="muted"]  { --tone: var(--border-strong); }

  /* Unread accent rail on the left edge. */
  .row.unread::before {
    content: "";
    position: absolute;
    left: 5px; top: 11px; bottom: 11px;
    width: 2px;
    border-radius: 2px;
    background: var(--tone);
  }

  .row-kind {
    width: 26px; height: 26px;
    border-radius: 7px;
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-3);
    color: var(--fg-muted);
    flex-shrink: 0;
    margin-top: 1px;
  }
  .row-kind[data-severity="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .row-kind[data-severity="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .row-kind[data-severity="danger"] { background: var(--danger-soft); color: var(--danger); }
  .row-kind[data-severity="info"]   { background: var(--info-soft);   color: var(--info); }

  .row-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .row-title {
    font-size: var(--fs-sm);
    color: var(--fg);
    line-height: 1.35;
  }
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
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--fg-subtle);
    background: var(--bg-elev-3);
    border-radius: 5px;
    padding: 1px 5px;
    vertical-align: 1px;
  }
  .row-time { font-size: 10.5px; color: var(--fg-faint); margin-top: 1px; }

  .row-action {
    align-self: center;
    font-size: var(--fs-xs);
    color: var(--accent);
    font-weight: 600;
    padding: 4px 9px;
    border-radius: 7px;
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 24%, transparent);
    flex-shrink: 0;
    transition: background var(--dur-fast);
  }
  .row-action:hover { background: color-mix(in oklab, var(--accent) 22%, transparent); }

  .row-close {
    align-self: center;
    width: 22px; height: 22px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: 6px;
    color: var(--fg-faint);
    opacity: 0;
    transition: background var(--dur-fast), color var(--dur-fast), opacity var(--dur-fast);
    flex-shrink: 0;
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
