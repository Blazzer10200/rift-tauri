import { PANEL_IDS, PRESETS, type LayoutPreset, type PanelId, type PanelState } from "./panel-types";

type Density = "compact" | "regular" | "comfy";

const STORAGE_KEY = "rift.ui.density.v1";
const RAIL_PINNED_KEY = "rift.ui.rail-pinned.v1";
const PANELS_KEY = "rift.ui.panels.v1";
const DOCK_WIDTH_KEY = "rift.ui.dock-w.v1";
const MAXIMIZED_KEY = "rift.ui.maximized.v1";
const PRESET_PICKED_KEY = "rift.ui.preset-picked.v1";
const V03_SHELL_KEY = "rift.ui.v03-shell.v1";

const DOCK_WIDTH_MIN = 280;
const DOCK_WIDTH_MAX = 560;
const DOCK_WIDTH_DEFAULT = 320;

function emptyPanelState(order: number): PanelState {
  return { open: false, collapsed: false, order, height: null };
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
  maximized = $state<PanelId | null>(null);
  presetPicked = $state(false);
  useV03Shell = $state(false);

  init() {
    if (typeof window === "undefined") return;
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "compact" || raw === "regular" || raw === "comfy") {
      this.density = raw;
    }
    this.railPinned = localStorage.getItem(RAIL_PINNED_KEY) === "1";
    this.presetPicked = localStorage.getItem(PRESET_PICKED_KEY) === "1";
    this.useV03Shell = localStorage.getItem(V03_SHELL_KEY) === "1";

    const dw = parseInt(localStorage.getItem(DOCK_WIDTH_KEY) ?? "", 10);
    if (Number.isFinite(dw)) this.dockWidth = clampWidth(dw);

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

  togglePanel(id: PanelId) {
    const cur = this.panels[id];
    const next: PanelState = { ...cur, open: !cur.open };
    this.panels = { ...this.panels, [id]: next };
    this.persistPanels();
  }

  setPanelOpen(id: PanelId, open: boolean) {
    if (this.panels[id].open === open) return;
    this.panels = { ...this.panels, [id]: { ...this.panels[id], open } };
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
      next[id] = { open: true, collapsed: false, order: cursor++, height: null };
    }
    // Closed panels keep a stable order after the open ones.
    for (const id of PANEL_IDS) {
      if (!next[id]) next[id] = { open: false, collapsed: false, order: cursor++, height: null };
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

  private persistPanels() {
    localStorage.setItem(PANELS_KEY, JSON.stringify(this.panels));
  }

  private apply() {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.density = this.density;
    this.applyRail();
    this.applyDockWidth();
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
}

function clampWidth(w: number): number {
  if (!Number.isFinite(w)) return DOCK_WIDTH_DEFAULT;
  return Math.max(DOCK_WIDTH_MIN, Math.min(DOCK_WIDTH_MAX, w));
}

function isPanelId(v: string | null): v is PanelId {
  return v !== null && (PANEL_IDS as readonly string[]).includes(v);
}

function readPanelsFromStorage(): Record<PanelId, PanelState> {
  const raw = localStorage.getItem(PANELS_KEY);
  if (!raw) return defaultPanelMap();
  try {
    const parsed = JSON.parse(raw) as Partial<Record<PanelId, PanelState>>;
    const out = defaultPanelMap();
    for (const id of PANEL_IDS) {
      const st = parsed[id];
      if (st && typeof st.open === "boolean" && typeof st.order === "number") {
        out[id] = {
          open: !!st.open,
          collapsed: !!st.collapsed,
          order: Number(st.order),
          height: st.height === null ? null : (Number.isFinite(st.height) ? Number(st.height) : null),
        };
      }
    }
    return out;
  } catch {
    return defaultPanelMap();
  }
}

export const uiPrefs = new UiPrefs();
