// Unified toast queue + persistent notification history. The live stack is
// rendered bottom-right by ToastHost (transient "just happened" feedback); every
// pushed toast is ALSO archived into `history`, which the NotificationCenter
// (bell + dropdown in the Topbar) renders as the one organized location for
// everything that's happened. Severity drives timeout; sticky entries
// (e.g. update available) never auto-dismiss.

export type ToastSeverity = "ok" | "warn" | "danger" | "info" | "muted";

export type ToastItem = {
  id: number;
  severity: ToastSeverity;
  /** Lucide-svelte icon constructor (legacy Svelte 4 component shape;
   *  typing it precisely fights the lucide-svelte type defs, so leave it
   *  permissive — the ToastHost just renders `<Icon size={12}/>`).
   *  Optional: when omitted, ToastHost falls back to a per-severity default
   *  icon. Keeps icon imports in the renderer rather than `.ts` state modules. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  icon?: any;
  title: string;
  detail?: string;
  /** Render detail in monospace (file paths, version numbers). */
  mono?: boolean;
  /** When true, never auto-dismiss. User must click the action or close. */
  sticky?: boolean;
  /** Override the severity-derived timeout. Ignored if sticky. */
  timeoutMs?: number;
  /** Optional CTA; clicking the toast body (excluding close) fires this. */
  action?: { label: string; onClick: () => void };
  /** Called when the user dismisses (close button or auto-timeout). */
  onDismiss?: () => void;
};

export type ToastPushOptions = Omit<ToastItem, "id">;

/** A persisted, serializable record of a notification. The live `action`
 *  closure can't survive a reload, so history keeps only what's re-renderable;
 *  the `action.label` is retained as a static tag, and the live action is
 *  attached only while the toast is still in the active stack. */
export type NotifyRecord = {
  id: number;
  severity: ToastSeverity;
  title: string;
  detail?: string;
  mono?: boolean;
  /** epoch ms — drives the Now / Past hour / Today / older grouping. */
  ts: number;
  read: boolean;
  /** How many consecutive identical pushes this record absorbed (≥2 renders "×N"). */
  count?: number;
  /** Static label of the action this notification carried (e.g. "View"),
   *  shown as a muted tag in history even after the live closure is gone. */
  actionLabel?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  action?: { label: string; onClick: () => void };
};

const MAX_VISIBLE = 3;
const HISTORY_CAP = 50;
// Age-out: notifications older than this are pruned on init + on every push, so
// the center never shows weeks-old noise even if the 50-cap isn't reached.
const HISTORY_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000; // 7 days
const HISTORY_KEY = "rift.notify.history.v1";

function defaultTimeout(sev: ToastSeverity): number {
  switch (sev) {
    case "danger": return 8000;
    case "warn":   return 6000;
    default:       return 4000;
  }
}

class ToastStore {
  items = $state<ToastItem[]>([]);
  /** Newest-first archive of every notification, persisted to localStorage. */
  history = $state<NotifyRecord[]>([]);
  /** True while the NotificationCenter dropdown is open (drives the badge +
   *  closes-on-outside-click in the component). */
  centerOpen = $state(false);

  private nextId = 1;
  private timers = new Map<number, ReturnType<typeof setTimeout>>();
  // #44/#224: absolute expiry per running timer + remaining-on-pause, so
  // resume() continues the countdown instead of restarting the full duration.
  private deadlines = new Map<number, number>();
  private remaining = new Map<number, number>();

  /** Unread = history records the user hasn't acknowledged (panel not opened
   *  on them). Derived so the badge updates reactively. */
  unreadCount = $derived(this.history.filter((r) => !r.read).length);

  init(): void {
    if (typeof window === "undefined") return;
    try {
      const raw = JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "null");
      if (Array.isArray(raw)) {
        this.history = raw
          .filter((r) => r && typeof r.title === "string" && typeof r.ts === "number")
          .slice(0, HISTORY_CAP)
          .map((r) => ({
            id: r.id ?? 0,
            severity: r.severity ?? "info",
            title: r.title,
            detail: r.detail,
            mono: !!r.mono,
            ts: r.ts,
            read: !!r.read,
            count: r.count,
            actionLabel: r.actionLabel,
          }));
        // Continue ids past the largest persisted one so live + history don't collide.
        this.nextId = Math.max(this.nextId, ...this.history.map((r) => r.id + 1));
        if (this.pruneOld()) this.persist();
      }
    } catch {
      /* malformed history — start clean */
    }
  }

  /** Drop history entries past the age cutoff. Returns true if anything was
   *  removed (caller persists). Called on init + every push. */
  private pruneOld(): boolean {
    const cutoff = Date.now() - HISTORY_MAX_AGE_MS;
    const kept = this.history.filter((r) => r.ts >= cutoff);
    if (kept.length === this.history.length) return false;
    this.history = kept;
    return true;
  }

  private persist(): void {
    if (typeof window === "undefined") return;
    // Strip the live action closure before serializing.
    const slim = this.history.map(({ action: _a, ...rest }) => rest);
    try {
      localStorage.setItem(HISTORY_KEY, JSON.stringify(slim));
    } catch {
      /* quota/serialization failure — history is best-effort, never throw */
    }
  }

  private arm(id: number, ms: number): void {
    this.deadlines.set(id, Date.now() + ms);
    this.timers.set(id, setTimeout(() => this.dismiss(id), ms));
  }

  push(opts: ToastPushOptions): number {
    const id = this.nextId++;
    const item: ToastItem = { ...opts, id };

    const next = [...this.items, item];
    // Cap visible count by dropping oldest non-sticky. Sticky entries
    // (update toast) stay until explicitly dismissed.
    while (next.length > MAX_VISIBLE) {
      const dropIdx = next.findIndex((t) => !t.sticky);
      if (dropIdx < 0) break;
      this.clearTimer(next[dropIdx].id);
      next.splice(dropIdx, 1);
    }
    this.items = next;

    // Archive into history (newest-first), capped. The badge counts it unread
    // until the center is opened. A push identical to the newest record
    // coalesces into it (count++ / fresh ts) instead of stacking dupes.
    this.pruneOld();
    const head = this.history[0];
    if (
      head &&
      head.title === opts.title &&
      head.detail === opts.detail &&
      head.severity === opts.severity
    ) {
      const merged: NotifyRecord = {
        ...head,
        ts: Date.now(),
        read: false,
        count: (head.count ?? 1) + 1,
        actionLabel: opts.action?.label,
        action: opts.action,
      };
      this.history = [merged, ...this.history.slice(1)];
    } else {
      const rec: NotifyRecord = {
        id,
        severity: opts.severity,
        title: opts.title,
        detail: opts.detail,
        mono: opts.mono,
        ts: Date.now(),
        read: false,
        actionLabel: opts.action?.label,
        action: opts.action,
      };
      this.history = [rec, ...this.history].slice(0, HISTORY_CAP);
    }
    this.persist();

    if (!opts.sticky) {
      this.arm(id, opts.timeoutMs ?? defaultTimeout(opts.severity));
    }
    return id;
  }

  dismiss(id: number, callHandler = true): void {
    const item = this.items.find((i) => i.id === id);
    this.clearTimer(id);
    this.remaining.delete(id);
    if (!item) return;
    this.items = this.items.filter((i) => i.id !== id);
    if (callHandler && item.onDismiss) item.onDismiss();
  }

  pause(id: number): void {
    const dl = this.deadlines.get(id);
    if (dl != null) this.remaining.set(id, Math.max(0, dl - Date.now()));
    this.clearTimer(id);
  }

  resume(id: number): void {
    if (this.timers.has(id)) return;
    const item = this.items.find((i) => i.id === id);
    if (!item || item.sticky) return;
    const ms = this.remaining.get(id) ?? item.timeoutMs ?? defaultTimeout(item.severity);
    this.remaining.delete(id);
    this.arm(id, ms);
  }

  clear(): void {
    for (const id of [...this.timers.keys()]) this.clearTimer(id);
    this.remaining.clear();
    this.items = [];
  }

  // ── notification center ────────────────────────────────────────────
  openCenter(): void {
    this.centerOpen = true;
    this.markAllRead();
  }

  closeCenter(): void {
    this.centerOpen = false;
  }

  toggleCenter(): void {
    if (this.centerOpen) this.closeCenter();
    else this.openCenter();
  }

  markAllRead(): void {
    if (!this.history.some((r) => !r.read)) return;
    this.history = this.history.map((r) => (r.read ? r : { ...r, read: true }));
    this.persist();
  }

  /** Remove one record from history (the live toast, if still showing, stays). */
  removeFromHistory(id: number): void {
    this.history = this.history.filter((r) => r.id !== id);
    this.persist();
  }

  clearHistory(): void {
    this.history = [];
    this.persist();
  }

  private clearTimer(id: number): void {
    const t = this.timers.get(id);
    if (t) {
      clearTimeout(t);
      this.timers.delete(id);
    }
    this.deadlines.delete(id);
  }
}

export const toast = new ToastStore();

/** Terse helpers for the common one-line case — keeps the ~severity at every
 *  call site obvious and the push options optional. `notify.ok("Copied")`,
 *  `notify.warn("No folder open", { detail })`, etc. Returns the toast id. */
type NotifyOpts = Partial<Omit<ToastPushOptions, "severity" | "title">>;
export const notify = {
  ok: (title: string, opts?: NotifyOpts) => toast.push({ severity: "ok", title, ...opts }),
  info: (title: string, opts?: NotifyOpts) => toast.push({ severity: "info", title, ...opts }),
  warn: (title: string, opts?: NotifyOpts) => toast.push({ severity: "warn", title, ...opts }),
  danger: (title: string, opts?: NotifyOpts) => toast.push({ severity: "danger", title, ...opts }),
};
