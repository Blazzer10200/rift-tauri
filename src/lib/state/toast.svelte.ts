// Unified toast queue. Single stack rendered by ToastHost. Replaces
// ActivityToast + UpdateToast singletons that overlapped in the same corner
// and had no shared lifecycle. Severity drives timeout; sticky entries
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

const MAX_VISIBLE = 3;

function defaultTimeout(sev: ToastSeverity): number {
  switch (sev) {
    case "danger": return 8000;
    case "warn":   return 6000;
    default:       return 4000;
  }
}

class ToastStore {
  items = $state<ToastItem[]>([]);
  private nextId = 1;
  private timers = new Map<number, ReturnType<typeof setTimeout>>();
  // #44/#224: absolute expiry per running timer + remaining-on-pause, so
  // resume() continues the countdown instead of restarting the full duration.
  private deadlines = new Map<number, number>();
  private remaining = new Map<number, number>();

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
    this.items = [];
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
