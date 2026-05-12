// Global terminal panel state — lives at AppShell level so the embedded
// shell persists across Browser/Activity/Conflicts/Settings tabs. Holds
// open/closed, drawer height, the live tab list, and the user's default
// shell preference.

export type TermStatus = "starting" | "running" | "exited" | "error";

export type TermTab = {
  id: string;              // local UI id (uuid-ish)
  sessionId: string | null;
  shellId: string | null;  // null = let backend pick default
  shellLabel: string;      // resolved after spawn; "Terminal" until then
  autoLaunch: string;      // per-tab override; falls back to global pref
  status: TermStatus;
};

const OPEN_KEY = "rift.terminal.open";
const HEIGHT_KEY = "rift.terminal.height";
const DEFAULT_SHELL_KEY = "rift.terminal.defaultShell";
const FONT_SIZE_KEY = "rift.terminal.fontSize";
const AUTO_LAUNCH_KEY = "rift.terminal.autoLaunch";
const SAVED_TABS_KEY = "rift.terminal.savedTabs";
const ACTIVE_TAB_IDX_KEY = "rift.terminal.activeTabIdx";

type SavedTab = { shellId: string | null; shellLabel: string; autoLaunch?: string };

const HEIGHT_MIN = 120;
const HEIGHT_DEFAULT = 280;
const FONT_SIZE_MIN = 10;
const FONT_SIZE_MAX = 22;
const FONT_SIZE_DEFAULT = 13;

function uid(): string {
  return "t" + Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

class TerminalStore {
  open = $state(false);
  height = $state(HEIGHT_DEFAULT);
  tabs = $state<TermTab[]>([]);
  activeTabId = $state<string | null>(null);
  defaultShellId = $state<string | null>(null);
  fontSize = $state(FONT_SIZE_DEFAULT);
  autoLaunchCommand = $state("");
  // Carried across init() → TerminalPanel.onMount so it can rehydrate tabs
  // once the shell list is known (we need labels for the saved shell IDs).
  pendingRestore: { tabs: SavedTab[]; activeIdx: number } | null = null;

  init() {
    if (typeof window === "undefined") return;
    try {
      this.open = localStorage.getItem(OPEN_KEY) === "1";
      const h = parseFloat(localStorage.getItem(HEIGHT_KEY) ?? "");
      if (!isNaN(h) && h >= HEIGHT_MIN) this.height = h;
      const ds = localStorage.getItem(DEFAULT_SHELL_KEY);
      if (ds) this.defaultShellId = ds;
      const fs = parseFloat(localStorage.getItem(FONT_SIZE_KEY) ?? "");
      if (!isNaN(fs) && fs >= FONT_SIZE_MIN && fs <= FONT_SIZE_MAX) this.fontSize = fs;
      const auto = localStorage.getItem(AUTO_LAUNCH_KEY);
      if (auto) this.autoLaunchCommand = auto;
      const rawTabs = localStorage.getItem(SAVED_TABS_KEY);
      if (rawTabs) {
        const parsed = JSON.parse(rawTabs) as SavedTab[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          const idx = parseInt(localStorage.getItem(ACTIVE_TAB_IDX_KEY) ?? "0", 10);
          this.pendingRestore = { tabs: parsed, activeIdx: isNaN(idx) ? 0 : idx };
        }
      }
    } catch { /* localStorage unavailable */ }
  }

  private persistTabs() {
    try {
      const snap: SavedTab[] = this.tabs.map((t) => ({
        shellId: t.shellId,
        shellLabel: t.shellLabel,
        autoLaunch: t.autoLaunch || undefined,
      }));
      localStorage.setItem(SAVED_TABS_KEY, JSON.stringify(snap));
      const activeIdx = this.tabs.findIndex((t) => t.id === this.activeTabId);
      localStorage.setItem(ACTIVE_TAB_IDX_KEY, String(Math.max(0, activeIdx)));
    } catch { /* noop */ }
  }

  setOpen(v: boolean) {
    this.open = v;
    try { localStorage.setItem(OPEN_KEY, v ? "1" : "0"); } catch { /* noop */ }
    if (v && this.tabs.length === 0) this.addTab(this.defaultShellId);
  }

  /// Recreate tabs from a saved snapshot. Called from TerminalPanel once the
  /// shell list has loaded so labels resolve correctly. Sessions don't spawn
  /// until the user activates the tab — see Terminal.svelte `visible` gate.
  // Hard ceiling on auto-restore. Without this, an enthusiastic testing
  // session that opened 14 tabs comes back as 14 tabs on relaunch and the
  // strip explodes.
  static readonly MAX_RESTORE_TABS = 4;

  consumePendingRestore() {
    const r = this.pendingRestore;
    this.pendingRestore = null;
    if (!r) return;
    // Dedupe by (shellId, autoLaunch). 14 identical git-bash tabs from a
    // testing burst collapse to 1; distinct presets (Claude Code + Codex +
    // plain bash) stay separate b/c their composite keys differ.
    const seen = new Set<string>();
    const unique: SavedTab[] = [];
    for (const t of r.tabs) {
      const key = `${t.shellId ?? ""}|${t.autoLaunch ?? ""}`;
      if (seen.has(key)) continue;
      seen.add(key);
      unique.push(t);
    }
    const capped = unique.slice(0, TerminalStore.MAX_RESTORE_TABS);
    for (const t of capped) this.addTab(t.shellId, t.shellLabel, t.autoLaunch ?? "");
    // Best-effort active focus — saved index may not map after dedupe; fall
    // back to first tab.
    const target = this.tabs[0];
    if (target) this.activeTabId = target.id;
    this.persistTabs();
  }

  closeAllTabs() {
    this.tabs = [];
    this.activeTabId = null;
    this.persistTabs();
    this.setOpen(false);
  }

  toggle() { this.setOpen(!this.open); }

  setHeight(h: number) {
    this.height = h;
    try { localStorage.setItem(HEIGHT_KEY, String(Math.round(h))); } catch { /* noop */ }
  }
  resetHeight() { this.setHeight(HEIGHT_DEFAULT); }

  addTab(shellId: string | null, shellLabel: string = "Terminal", autoLaunch: string = ""): string {
    const id = uid();
    const tab: TermTab = {
      id,
      sessionId: null,
      shellId,
      shellLabel,
      autoLaunch,
      status: "starting",
    };
    this.tabs = [...this.tabs, tab];
    this.activeTabId = id;
    this.persistTabs();
    return id;
  }

  setActive(id: string) {
    this.activeTabId = id;
    this.persistTabs();
  }

  closeTab(id: string) {
    const idx = this.tabs.findIndex((t) => t.id === id);
    if (idx < 0) return;
    const remaining = this.tabs.filter((t) => t.id !== id);
    this.tabs = remaining;
    if (this.activeTabId === id) {
      const next = remaining[idx] ?? remaining[idx - 1] ?? null;
      this.activeTabId = next?.id ?? null;
    }
    if (remaining.length === 0) {
      this.setOpen(false);
    }
    this.persistTabs();
  }

  patchTab(id: string, patch: Partial<TermTab>) {
    this.tabs = this.tabs.map((t) => (t.id === id ? { ...t, ...patch } : t));
    if (patch.shellLabel || patch.shellId !== undefined) this.persistTabs();
  }

  setDefaultShell(id: string | null) {
    this.defaultShellId = id;
    try {
      if (id) localStorage.setItem(DEFAULT_SHELL_KEY, id);
      else localStorage.removeItem(DEFAULT_SHELL_KEY);
    } catch { /* noop */ }
  }

  setFontSize(n: number) {
    const clamped = Math.max(FONT_SIZE_MIN, Math.min(FONT_SIZE_MAX, Math.round(n)));
    this.fontSize = clamped;
    try { localStorage.setItem(FONT_SIZE_KEY, String(clamped)); } catch { /* noop */ }
  }

  setAutoLaunchCommand(cmd: string) {
    this.autoLaunchCommand = cmd;
    try {
      if (cmd) localStorage.setItem(AUTO_LAUNCH_KEY, cmd);
      else localStorage.removeItem(AUTO_LAUNCH_KEY);
    } catch { /* noop */ }
  }
}

export const terminal = new TerminalStore();

export const TERM_HEIGHT_MIN = HEIGHT_MIN;
export const TERM_FONT_SIZE_MIN = FONT_SIZE_MIN;
export const TERM_FONT_SIZE_MAX = FONT_SIZE_MAX;
