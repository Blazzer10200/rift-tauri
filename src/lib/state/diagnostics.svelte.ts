/**
 * Diagnostics store — frontend consumer of the backend `diag://event` bus.
 *
 * The backend (`diagnostics/mod.rs`) already broadcasts every `log::` record +
 * structured `emit_with_fields` event over `diag://event`, rate-limited 200/s
 * (Log) / 50/s (System). Until now nothing on the frontend listened — this
 * store closes that gap: it subscribes, keeps a bounded ring of recent events,
 * and exposes derived filters for the live console (Phase 1).
 *
 * Scrubbing already happens at the bus boundary (`publish`), so events arriving
 * here are username/home/key-safe. The ONLY frontend-added paths are the
 * `window.onerror`/`unhandledrejection` captures below, which run through
 * `scrubUser` before entering the ring.
 */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { scrubUser } from "$lib/utils/redact";
import { rollUpHealth, overallHealth } from "./diagnosticsHealth";

export type DiagStage = "log" | "system";
export type DiagLevel = "trace" | "debug" | "info" | "warn" | "error";

/** Wire shape — mirrors `diagnostics::DiagEvent` (serde snake_case/lowercase). */
export type DiagEvent = {
  at: string;
  seq: number;
  stage: DiagStage;
  level: DiagLevel;
  resource: string | null;
  file: string | null;
  message: string;
  fields: unknown;
};

const RING_CAP = 2000;
const LEVEL_RANK: Record<DiagLevel, number> = { trace: 0, debug: 1, info: 2, warn: 3, error: 4 };

class DiagnosticsStore {
  /** Bounded ring of received events (oldest dropped past RING_CAP). */
  events = $state<DiagEvent[]>([]);
  /** When paused, incoming events buffer off-screen and are flushed on resume. */
  paused = $state(false);
  /** Filter: minimum level to show. */
  minLevel = $state<DiagLevel>("debug");
  /** Filter: resource exact-match, or "" for all. */
  resourceFilter = $state<string>("");
  /** Filter: free-text search across message + resource + file + fields. */
  search = $state<string>("");
  /** True once the `diag://event` listener is attached. */
  live = $state(false);
  /** Count of events dropped off the tail since last clear (ring overflow). */
  dropped = $state(0);

  #unlisten: UnlistenFn | null = null;
  /** In-flight init(), so concurrent callers (AppShell boot + DiagnosticsPage
   *  mount land in the same tick when the app opens ON the diagnostics page)
   *  can't both pass the #unlisten guard and attach TWO bus listeners — double
   *  delivery duplicated every seq and blew up the keyed each (each_key_duplicate). */
  #initing: Promise<void> | null = null;
  /** Events received while paused, flushed into the ring on resume. */
  #pausedBuf: DiagEvent[] = [];
  /** Synthetic seq for frontend-originated events (negative to avoid clashes). */
  #feSeq = -1;
  #errHooked = false;
  #onError: ((e: ErrorEvent) => void) | null = null;
  #onRejection: ((e: PromiseRejectionEvent) => void) | null = null;

  /** Distinct resources seen, for the filter-chip list. */
  resources = $derived.by(() => {
    const set = new Set<string>();
    for (const e of this.events) if (e.resource) set.add(e.resource);
    return [...set].sort();
  });

  /** Phase 3: per-subsystem green/amber/red roll-up over the live ring. */
  health = $derived(rollUpHealth(this.events));
  /** Worst subsystem level — one dot for the console header. */
  overall = $derived(overallHealth(this.health));

  /** The events actually shown, after level/resource/text filters. */
  filtered = $derived.by(() => {
    const min = LEVEL_RANK[this.minLevel];
    const res = this.resourceFilter;
    const q = this.search.trim().toLowerCase();
    return this.events.filter((e) => {
      if (LEVEL_RANK[e.level] < min) return false;
      if (res && e.resource !== res) return false;
      if (q) {
        const hay = `${e.message} ${e.resource ?? ""} ${e.file ?? ""} ${stringifyFields(e.fields)}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
  });

  /** Idempotent + single-flight: attach the bus listener + frontend error hooks. */
  async init() {
    if (this.#unlisten) return;
    if (this.#initing) return this.#initing;
    this.#initing = (async () => {
      this.#unlisten = await listen<DiagEvent>("diag://event", (e) => this.#push(e.payload));
      this.live = true;
      this.#hookErrors();
      this.#exposeDevHook();
      // Backfill: the pump only forwards LIVE events, so attaching mid-session
      // used to start blank. The bus keeps a bounded backlog — merge it in front
      // of anything already received (dedupe by seq).
      try {
        const backlog = await invoke<DiagEvent[]>("diag_backlog");
        const have = new Set(this.events.map((ev) => ev.seq));
        const fresh = backlog.filter((ev) => !have.has(ev.seq));
        if (fresh.length) this.events = [...fresh, ...this.events].slice(-RING_CAP);
      } catch (e) {
        console.warn("diag_backlog failed", e);
      }
    })().finally(() => { this.#initing = null; });
    return this.#initing;
  }

  /**
   * DEV-ONLY: expose a read-only snapshot on `window.__riftDiag` so the live
   * event stream + health roll-up can be pulled programmatically (CDP / the
   * `c.sh diag` helper) WITHOUT navigating to the console UI or screenshotting.
   * Returns plain data, never the reactive store. No-op in production builds.
   */
  #exposeDevHook() {
    if (!import.meta.env.DEV || typeof window === "undefined") return;
    (window as unknown as { __riftDiag?: unknown }).__riftDiag = {
      /** Last `n` events (default 40), newest last, as plain rows. */
      recent: (n = 40) =>
        this.events.slice(-n).map((e) => ({
          at: e.at,
          level: e.level,
          resource: e.resource,
          message: e.message,
          fields: e.fields,
        })),
      /** Per-subsystem health verdicts (same roll-up the strip renders). */
      health: () => this.health.map((h) => ({ key: h.key, level: h.level, detail: h.detail, count: h.count })),
      /** Counts: total in ring, dropped off tail, whether the listener is live. */
      stats: () => ({ total: this.events.length, dropped: this.dropped, live: this.live, overall: this.overall }),
    };
  }

  dispose() {
    if (this.#unlisten) { this.#unlisten(); this.#unlisten = null; }
    this.#unhookErrors();
    this.live = false;
  }

  pause() { this.paused = true; }
  resume() {
    this.paused = false;
    if (this.#pausedBuf.length) {
      for (const ev of this.#pausedBuf) this.#append(ev);
      this.#pausedBuf = [];
    }
  }
  togglePause() { this.paused ? this.resume() : this.pause(); }

  clear() {
    this.events = [];
    this.#pausedBuf = [];
    this.dropped = 0;
  }

  /** Plain-text export of the filtered view (already scrubbed at the bus). */
  exportText(): string {
    return this.filtered
      .map((e) => {
        const t = e.at;
        const r = e.resource ? ` [${e.resource}]` : "";
        const f = hasFields(e.fields) ? ` ${stringifyFields(e.fields)}` : "";
        return `${t} ${e.level.toUpperCase().padEnd(5)}${r} ${e.message}${f}`;
      })
      .join("\n");
  }

  #push(ev: DiagEvent) {
    if (this.paused) {
      this.#pausedBuf.push(ev);
      // Bound the off-screen buffer too, so a long pause can't grow unbounded.
      if (this.#pausedBuf.length > RING_CAP) this.#pausedBuf.shift();
      return;
    }
    this.#append(ev);
  }

  #append(ev: DiagEvent) {
    // Reassign (not mutate) so Svelte 5 tracks the change.
    const next = this.events.length >= RING_CAP
      ? (this.dropped++, this.events.slice(1))   // drop oldest (index 0), not a tail-splice
      : this.events.slice();
    next.push(ev);
    this.events = next;
  }

  #hookErrors() {
    if (this.#errHooked) return;
    this.#errHooked = true;
    this.#onError = (e: ErrorEvent) => {
      this.#push(this.#feEvent("error", e.message || "Uncaught error", {
        source: scrubUser(e.filename),
        line: e.lineno,
        col: e.colno,
        stack: e.error?.stack ? scrubUser(String(e.error.stack)) : undefined,
      }));
    };
    this.#onRejection = (e: PromiseRejectionEvent) => {
      const reason = e.reason;
      const msg = reason instanceof Error ? reason.message : String(reason);
      this.#push(this.#feEvent("error", `Unhandled rejection: ${msg}`, {
        stack: reason instanceof Error && reason.stack ? scrubUser(reason.stack) : undefined,
      }));
    };
    window.addEventListener("error", this.#onError);
    window.addEventListener("unhandledrejection", this.#onRejection);
  }

  #unhookErrors() {
    if (this.#onError) window.removeEventListener("error", this.#onError);
    if (this.#onRejection) window.removeEventListener("unhandledrejection", this.#onRejection);
    this.#onError = this.#onRejection = null;
    this.#errHooked = false;
  }

  #feEvent(level: DiagLevel, message: string, fields: Record<string, unknown>): DiagEvent {
    return {
      at: new Date().toISOString(),
      seq: this.#feSeq--,
      stage: "system",
      level,
      resource: "frontend",
      file: null,
      message,
      fields,
    };
  }
}

function hasFields(fields: unknown): boolean {
  if (fields == null) return false;
  if (typeof fields === "object") return Object.keys(fields as object).length > 0;
  return true;
}

function stringifyFields(fields: unknown): string {
  if (!hasFields(fields)) return "";
  try { return JSON.stringify(fields); } catch { return String(fields); }
}

export const diagnostics = new DiagnosticsStore();
