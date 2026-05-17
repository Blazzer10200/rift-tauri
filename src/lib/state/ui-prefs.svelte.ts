import { PANEL_IDS, PRESETS, type DockSlot, type LayoutPreset, type PanelId, type PanelState } from "./panel-types";

type Density = "compact" | "regular" | "comfy";

const STORAGE_KEY = "rift.ui.density.v1";
const RAIL_PINNED_KEY = "rift.ui.rail-pinned.v1";
const PANELS_KEY = "rift.ui.panels.v1";
const DOCK_WIDTH_KEY = "rift.ui.dock-w.v1";
const MAXIMIZED_KEY = "rift.ui.maximized.v1";
const PRESET_PICKED_KEY = "rift.ui.preset-picked.v1";
const V03_SHELL_KEY = "rift.ui.v03-shell.v1";
const DOCK_ACCORDION_KEY = "rift.ui.dock-accordion.v1";
const DOCK_SPLIT_KEY = "rift.ui.dock-split.v1";

const DOCK_WIDTH_MIN = 260;
// v0.4 — outer dock max grows to ~half viewport (capped at 900) so split-dock
// can show two slots comfortably. Min chat width reserved at 480px. Computed
// per resize via clampWidth() rather than baked in as a const.
const DOCK_WIDTH_DEFAULT = 320;
const DOCK_WIDTH_HARD_CAP = 900;
const CHAT_MIN_RESERVE = 480;

const DOCK_SPLIT_MIN = 20;
const DOCK_SPLIT_MAX = 80;
const DOCK_SPLIT_DEFAULT = 50;

function emptyPanelState(order: number): PanelState {
  return { open: false, collapsed: false, order, height: null, slot: "left" };
}

function defaultPanelMap(): Record<PanelId, PanelState> {
  const out = {} as Record<PanelId, PanelState>;
  PANEL_IDS.forEach((id, i) => { out[id] = emptyPanelState(i); });
  return out;
}

class UiPrefs {
  density = $state<Density>("compact");
  railPinned = $state(false);
  panels = $state<Record<PanelId, PanelState>>(defaultPanelMap());
  dockWidth = $state(DOCK_WIDTH_DEFAULT);
  /** v0.4 — left-slot percentage of dock width when right slot is occupied. */
  dockSplitPct = $state(DOCK_SPLIT_DEFAULT);
  maximized = $state<PanelId | null>(null);
  presetPicked = $state(false);
  useV03Shell = $state(false);
  dockAccordion = $state(true);

  init() {
    if (typeof window === "undefined") return;
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "compact" || raw === "regular" || raw === "comfy") {
      this.density = raw;
    }
    this.railPinned = localStorage.getItem(RAIL_PINNED_KEY) === "1";
    this.presetPicked = localStorage.getItem(PRESET_PICKED_KEY) === "1";
    this.useV03Shell = localStorage.getItem(V03_SHELL_KEY) === "1";
    // Accordion defaults to true. Missing key = true (first-launch); explicit "0" = off.
    this.dockAccordion = localStorage.getItem(DOCK_ACCORDION_KEY) !== "0";

    const dw = parseInt(localStorage.getItem(DOCK_WIDTH_KEY) ?? "", 10);
    if (Number.isFinite(dw)) this.dockWidth = clampWidth(dw);

    const sp = parseInt(localStorage.getItem(DOCK_SPLIT_KEY) ?? "", 10);
    if (Number.isFinite(sp)) this.dockSplitPct = clampSplit(sp);

    const mx = localStorage.getItem(MAXIMIZED_KEY);
    this.maximized = isPanelId(mx) ? mx : null;

    this.panels = readPanelsFromStorage();
    this.apply();
  }

  setDensity(d: Density) {
    this.density = d;
    localStorage.setItem(STORAGE_KEY, d);
    this.apply();
  }

  toggleRailPinned() {
    this.railPinned = !this.railPinned;
    localStorage.setItem(RAIL_PINNED_KEY, this.railPinned ? "1" : "0");
    this.applyRail();
  }

  togglePanel(id: PanelId, opts?: { allowMulti?: boolean }) {
    const cur = this.panels[id];
    const opening = !cur.open;
    this.applyOpenState(id, !cur.open, opening && this.accordionActive(opts));
  }

  setPanelOpen(id: PanelId, open: boolean, opts?: { allowMulti?: boolean }) {
    if (this.panels[id].open === open) return;
    this.applyOpenState(id, open, open && this.accordionActive(opts));
  }

  // Accordion gating: only under v0.3 + accordion pref enabled + shift-bypass not set.
  // Phase C Part 2 — keeps v0.2 callers and shift-click users free to stack panels.
  private accordionActive(opts?: { allowMulti?: boolean }): boolean {
    return this.useV03Shell && this.dockAccordion && !opts?.allowMulti;
  }

  private applyOpenState(id: PanelId, open: boolean, closeOthers: boolean) {
    const next = { ...this.panels };
    if (closeOthers) {
      // v0.4 — accordion sweep restricts to the opening panel's slot. Opening
      // a left-slot panel doesn't collapse right-slot panels and vice versa.
      const slot = next[id].slot;
      for (const pid of PANEL_IDS) {
        if (pid !== id && next[pid].open && next[pid].slot === slot) {
          next[pid] = { ...next[pid], open: false };
        }
      }
    }
    next[id] = { ...next[id], open };
    this.panels = next;
    this.persistPanels();
    // If the maximized panel just got closed (directly OR via accordion sweep),
    // restore chat — leaving it maximized while it has no dock header would
    // strand the user (no ⛶ to click to restore besides Esc).
    if (this.maximized && !next[this.maximized].open) this.maximizePanel(null);
  }

  /** Move a panel between left/right slots. Inserts at end of target slot's
   *  ordered stack. No-op if already in target slot. */
  setPanelSlot(id: PanelId, slot: DockSlot) {
    const cur = this.panels[id];
    if (cur.slot === slot) return;
    // Renumber: keep other panels' relative order; place moved one at end of
    // target slot.
    const others = (Object.entries(this.panels) as [PanelId, PanelState][])
      .filter(([pid]) => pid !== id)
      .sort((a, b) => a[1].order - b[1].order);
    const next = {} as Record<PanelId, PanelState>;
    let cursor = 0;
    for (const [pid, st] of others) next[pid] = { ...st, order: cursor++ };
    next[id] = { ...cur, slot, order: cursor };
    this.panels = next;
    this.persistPanels();
  }

  togglePanelCollapsed(id: PanelId) {
    const cur = this.panels[id];
    this.panels = { ...this.panels, [id]: { ...cur, collapsed: !cur.collapsed } };
    this.persistPanels();
  }

  reorderPanel(id: PanelId, newOrder: number) {
    // Pull `id` out; renumber remaining by current order; reinsert at newOrder.
    const others = (Object.entries(this.panels) as [PanelId, PanelState][])
      .filter(([pid]) => pid !== id)
      .sort((a, b) => a[1].order - b[1].order);
    const clamped = Math.max(0, Math.min(newOrder, others.length));
    const reordered = {} as Record<PanelId, PanelState>;
    let cursor = 0;
    for (let i = 0; i <= others.length; i++) {
      if (i === clamped) {
        reordered[id] = { ...this.panels[id], order: cursor++ };
      }
      if (i < others.length) {
        const [pid, st] = others[i];
        reordered[pid] = { ...st, order: cursor++ };
      }
    }
    this.panels = reordered;
    this.persistPanels();
  }

  setPanelHeight(id: PanelId, h: number | null) {
    this.panels = { ...this.panels, [id]: { ...this.panels[id], height: h } };
    this.persistPanels();
  }

  setDockWidth(w: number) {
    this.dockWidth = clampWidth(w);
    localStorage.setItem(DOCK_WIDTH_KEY, String(this.dockWidth));
    this.applyDockWidth();
  }

  // Drag-time path — state + CSS only, no localStorage write. Pair w/
  // persistDockWidth() on pointerup. Synchronous localStorage on every
  // pointermove was the jitter source (~100 writes/sec during a drag).
  setDockWidthLive(w: number) {
    this.dockWidth = clampWidth(w);
    this.applyDockWidth();
  }

  persistDockWidth() {
    localStorage.setItem(DOCK_WIDTH_KEY, String(this.dockWidth));
  }

  /** v0.4 — internal split (left-slot percentage 0-100). Live setter follows
   *  the same drag-vs-release pattern as the outer width handle: no
   *  localStorage during pointermove, one write on pointerup. */
  setDockSplitLive(pct: number) {
    this.dockSplitPct = clampSplit(pct);
    this.applyDockSplit();
  }

  persistDockSplit() {
    localStorage.setItem(DOCK_SPLIT_KEY, String(this.dockSplitPct));
  }

  setDockSplitPct(pct: number) {
    this.setDockSplitLive(pct);
    this.persistDockSplit();
  }

  maximizePanel(id: PanelId | null) {
    this.maximized = id;
    if (id) localStorage.setItem(MAXIMIZED_KEY, id);
    else localStorage.removeItem(MAXIMIZED_KEY);
  }

  applyPreset(p: LayoutPreset) {
    const ids = PRESETS[p];
    const next = {} as Record<PanelId, PanelState>;
    let cursor = 0;
    for (const id of ids) {
      next[id] = { open: true, collapsed: false, order: cursor++, height: null, slot: "left" };
    }
    // Closed panels keep a stable order after the open ones.
    for (const id of PANEL_IDS) {
      if (!next[id]) next[id] = { open: false, collapsed: false, order: cursor++, height: null, slot: "left" };
    }
    this.panels = next;
    this.markPresetPicked();
    this.persistPanels();
  }

  markPresetPicked() {
    this.presetPicked = true;
    localStorage.setItem(PRESET_PICKED_KEY, "1");
  }

  setUseV03Shell(on: boolean) {
    this.useV03Shell = on;
    localStorage.setItem(V03_SHELL_KEY, on ? "1" : "0");
  }

  setDockAccordion(on: boolean) {
    this.dockAccordion = on;
    localStorage.setItem(DOCK_ACCORDION_KEY, on ? "1" : "0");
  }

  private persistPanels() {
    localStorage.setItem(PANELS_KEY, JSON.stringify(this.panels));
  }

  private apply() {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.density = this.density;
    this.applyRail();
    this.applyDockWidth();
    this.applyDockSplit();
  }

  private applyRail() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--rail-w", this.railPinned ? "220px" : "48px");
    }
  }

  private applyDockWidth() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--dock-w", `${this.dockWidth}px`);
    }
  }

  private applyDockSplit() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--dock-split-pct", `${this.dockSplitPct}%`);
    }
  }
}

/** Recompute every call so a window resize that shrinks the viewport pulls
 *  the dock back below the new effective max. */
function currentDockMax(): number {
  if (typeof window === "undefined") return DOCK_WIDTH_HARD_CAP;
  const viewport = window.innerWidth || DOCK_WIDTH_HARD_CAP;
  return Math.max(DOCK_WIDTH_MIN, Math.min(DOCK_WIDTH_HARD_CAP, viewport - CHAT_MIN_RESERVE));
}

function clampWidth(w: number): number {
  if (!Number.isFinite(w)) return DOCK_WIDTH_DEFAULT;
  return Math.max(DOCK_WIDTH_MIN, Math.min(currentDockMax(), w));
}

function clampSplit(p: number): number {
  if (!Number.isFinite(p)) return DOCK_SPLIT_DEFAULT;
  return Math.max(DOCK_SPLIT_MIN, Math.min(DOCK_SPLIT_MAX, p));
}

function isPanelId(v: string | null): v is PanelId {
  return v !== null && (PANEL_IDS as readonly string[]).includes(v);
}

function readPanelsFromStorage(): Record<PanelId, PanelState> {
  const raw = localStorage.getItem(PANELS_KEY);
  if (!raw) return defaultPanelMap();
  try {
    const parsed = JSON.parse(raw) as Partial<Record<PanelId, Partial<PanelState>>>;
    const out = defaultPanelMap();
    for (const id of PANEL_IDS) {
      const st = parsed[id];
      if (st && typeof st.open === "boolean" && typeof st.order === "number") {
        out[id] = {
          open: !!st.open,
          collapsed: !!st.collapsed,
          order: Number(st.order),
          height: st.height === null ? null : (Number.isFinite(st.height) ? Number(st.height) : null),
          // v0.4 migration — pre-slot records default to "left" so existing
          // users see no change. Validate string to guard against junk.
          slot: st.slot === "right" ? "right" : "left",
        };
      }
    }
    return out;
  } catch {
    return defaultPanelMap();
  }
}

export const uiPrefs = new UiPrefs();
