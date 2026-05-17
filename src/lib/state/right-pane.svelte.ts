import type { PanelId } from "./panel-types";
import { PANEL_IDS } from "./panel-types";

// v0.4.1 — RightPane state. Replaces the v0.3/v0.4 multi-panel dock w/ a
// single-page right-side surface picked by the ActivityBar. ActivityBarId
// is an alias for the post-tasks PanelId; Phase 3 renames the underlying
// type + file so this alias becomes the canonical name.
export type ActivityBarId = PanelId;

const ACTIVE_KEY = "rift.ui.right-pane.v1";
const WIDTH_KEY = "rift.ui.right-pane-w.v1";
const ORDER_KEY = "rift.ui.activitybar-order.v1";

// Legacy keys deleted on first v0.4.1 launch — they belonged to the dock
// model that v0.4.1 replaces. dock-w.v1 is read once to seed WIDTH_KEY
// (clamped into the new range), then deleted.
const LEGACY_PANELS_KEY = "rift.ui.panels.v1";
const LEGACY_DOCK_WIDTH_KEY = "rift.ui.dock-w.v1";
const LEGACY_DOCK_SPLIT_KEY = "rift.ui.dock-split.v1";
const LEGACY_MAXIMIZED_KEY = "rift.ui.maximized.v1";
const LEGACY_PRESET_PICKED_KEY = "rift.ui.preset-picked.v1";
const LEGACY_DOCK_ACCORDION_KEY = "rift.ui.dock-accordion.v1";

const WIDTH_MIN = 320;
const WIDTH_MAX = 1200;
const WIDTH_DEFAULT = 560;

// Spec default order for the activity bar (Files · Sync · Activity · Terminal
// · Agents · Attachments · History). PANEL_IDS is the canonical id set, but
// its order is the registry-declaration order; the bar uses its own default.
const DEFAULT_ORDER: readonly ActivityBarId[] = [
  "files",
  "sync",
  "activity",
  "terminal",
  "agents",
  "attachments",
  "history",
] as const;

function clampWidth(w: number): number {
  if (!Number.isFinite(w)) return WIDTH_DEFAULT;
  return Math.max(WIDTH_MIN, Math.min(WIDTH_MAX, w));
}

function isActivityBarId(v: unknown): v is ActivityBarId {
  return typeof v === "string" && (PANEL_IDS as readonly string[]).includes(v);
}

class RightPane {
  activeId = $state<ActivityBarId | null>(null);
  width = $state<number>(WIDTH_DEFAULT);
  order = $state<ActivityBarId[]>([...DEFAULT_ORDER]);
  /** Latch — first time a page becomes active, it stays mounted thereafter
   *  so internal state (scroll, expanded rows, terminal session) survives
   *  toggles. Same pattern v0.3 PanelShell uses. */
  everOpened = $state<Set<ActivityBarId>>(new Set());

  init() {
    if (typeof window === "undefined") return;
    this.migrateLegacy();

    const aid = localStorage.getItem(ACTIVE_KEY);
    this.activeId = isActivityBarId(aid) ? aid : null;
    if (this.activeId) this.everOpened = new Set([this.activeId]);

    const w = parseInt(localStorage.getItem(WIDTH_KEY) ?? "", 10);
    if (Number.isFinite(w)) this.width = clampWidth(w);

    try {
      const raw = localStorage.getItem(ORDER_KEY);
      if (raw) {
        const arr = JSON.parse(raw) as unknown;
        if (Array.isArray(arr)) {
          const valid = arr.filter(isActivityBarId);
          const seen = new Set(valid);
          // Backfill any ids missing from stored order (new entries, dropped
          // legacy entries that filtered out). Append to end so user-chosen
          // ordering wins for what they've already arranged.
          for (const id of DEFAULT_ORDER) if (!seen.has(id)) valid.push(id);
          this.order = valid;
        }
      }
    } catch { /* fall through to default order */ }

    this.applyWidth();
  }

  /** One-time migration on first v0.4.1 launch: seed activeId from the
   *  legacy panel state (exactly-one-open → that id, else null), rename
   *  the dock-width key, and delete the obsolete dock-era keys. */
  private migrateLegacy() {
    const panelsRaw = localStorage.getItem(LEGACY_PANELS_KEY);
    if (panelsRaw && !localStorage.getItem(ACTIVE_KEY)) {
      try {
        const parsed = JSON.parse(panelsRaw) as Record<string, { open?: boolean }>;
        const openIds = Object.entries(parsed)
          .filter(([id, st]) => id !== "tasks" && st?.open === true && isActivityBarId(id))
          .map(([id]) => id as ActivityBarId);
        if (openIds.length === 1) {
          localStorage.setItem(ACTIVE_KEY, openIds[0]);
        }
      } catch { /* unparseable — skip seeding */ }
    }
    if (panelsRaw) localStorage.removeItem(LEGACY_PANELS_KEY);

    const legacyW = localStorage.getItem(LEGACY_DOCK_WIDTH_KEY);
    if (legacyW && !localStorage.getItem(WIDTH_KEY)) {
      const n = parseInt(legacyW, 10);
      if (Number.isFinite(n)) {
        localStorage.setItem(WIDTH_KEY, String(clampWidth(n)));
      }
    }
    if (legacyW) localStorage.removeItem(LEGACY_DOCK_WIDTH_KEY);

    for (const k of [
      LEGACY_DOCK_SPLIT_KEY,
      LEGACY_MAXIMIZED_KEY,
      LEGACY_PRESET_PICKED_KEY,
      LEGACY_DOCK_ACCORDION_KEY,
    ]) {
      localStorage.removeItem(k);
    }
  }

  setActive(id: ActivityBarId) {
    this.activeId = id;
    if (!this.everOpened.has(id)) {
      this.everOpened = new Set([...this.everOpened, id]);
    }
    localStorage.setItem(ACTIVE_KEY, id);
    this.applyWidth();
  }

  toggle(id: ActivityBarId) {
    if (this.activeId === id) this.close();
    else this.setActive(id);
  }

  close() {
    this.activeId = null;
    localStorage.removeItem(ACTIVE_KEY);
    this.applyWidth();
  }

  setWidth(w: number) {
    this.width = clampWidth(w);
    localStorage.setItem(WIDTH_KEY, String(this.width));
    this.applyWidth();
  }

  /** Drag-time path — state + CSS var only, no localStorage write per
   *  pointermove. Pair w/ persistWidth() on pointerup. Same drag/persist
   *  split the dock used; ~100 writes/sec during a drag was the jitter
   *  source the old internal handle had. */
  setWidthLive(w: number) {
    this.width = clampWidth(w);
    this.applyWidth();
  }

  persistWidth() {
    localStorage.setItem(WIDTH_KEY, String(this.width));
  }

  /** Dblclick handle → 50% of viewport, clamped. */
  snapHalfViewport() {
    if (typeof window === "undefined") return;
    this.setWidth(Math.floor(window.innerWidth / 2));
  }

  reorder(from: number, to: number) {
    if (from === to || from < 0 || from >= this.order.length) return;
    const next = [...this.order];
    const [moved] = next.splice(from, 1);
    next.splice(Math.max(0, Math.min(next.length, to)), 0, moved);
    this.order = next;
    localStorage.setItem(ORDER_KEY, JSON.stringify(next));
  }

  resetOrder() {
    this.order = [...DEFAULT_ORDER];
    localStorage.removeItem(ORDER_KEY);
  }

  reset() {
    this.close();
    this.setWidth(WIDTH_DEFAULT);
    this.resetOrder();
  }

  private applyWidth() {
    if (typeof document === "undefined") return;
    // 0 when closed so the column collapses entirely; active px otherwise.
    const px = this.activeId === null ? 0 : this.width;
    document.documentElement.style.setProperty("--right-pane-w", `${px}px`);
  }
}

export const rightPane = new RightPane();
