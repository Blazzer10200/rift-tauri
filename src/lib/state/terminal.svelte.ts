// Global terminal panel state — lives at AppShell level so the embedded
// shell persists across Browser/Activity/Conflicts/Settings tabs. Holds
// open/closed, drawer height, the live tab list, and the user's default
// shell preference.

import { uiPrefs } from "./ui-prefs.svelte";
import { rightPane } from "./right-pane.svelte";

export type TermStatus = "starting" | "running" | "exited" | "error";

export type TermTab = {
  id: string;              // local UI id (uuid-ish)
  sessionId: string | null;
  shellId: string | null;  // null = let backend pick default
  shellLabel: string;      // resolved after spawn; "Terminal" until then
  customLabel: string;     // user-set rename; empty = use shellLabel
  autoLaunch: string;      // per-tab override; falls back to global pref
  status: TermStatus;
};

const OPEN_KEY = "rift.terminal.open";
const HEIGHT_KEY = "rift.terminal.height";
const DEFAULT_SHELL_KEY = "rift.terminal.defaultShell";
const FONT_SIZE_KEY = "rift.terminal.fontSize";
const FONT_FAMILY_KEY = "rift.terminal.fontFamily";
const FONT_FAMILY_CUSTOM_KEY = "rift.terminal.fontFamilyCustom";
const SCROLLBACK_KEY = "rift.terminal.scrollback";
const CURSOR_STYLE_KEY = "rift.terminal.cursorStyle";
const CURSOR_BLINK_KEY = "rift.terminal.cursorBlink";
const BELL_STYLE_KEY = "rift.terminal.bellStyle";
const COPY_ON_SELECT_KEY = "rift.terminal.copyOnSelect";
const RIGHT_CLICK_PASTE_KEY = "rift.terminal.rightClickPaste";
const THEME_PRESET_KEY = "rift.terminal.themePreset";
const AUTO_LAUNCH_KEY = "rift.terminal.autoLaunch";
const SAVED_TABS_KEY = "rift.terminal.savedTabs";
const ACTIVE_TAB_IDX_KEY = "rift.terminal.activeTabIdx";

type SavedTab = { shellId: string | null; shellLabel: string; autoLaunch?: string; customLabel?: string };

export type CursorStyle = "bar" | "block" | "underline";
export type BellStyle = "none" | "visual" | "sound";
export type FontFamilyPreset = "default" | "cascadia" | "consolas" | "menlo" | "custom";
export type ThemePresetId = "rift" | "dracula" | "solarized-dark" | "monokai" | "github-dark";

const HEIGHT_MIN = 120;
const HEIGHT_DEFAULT = 280;
const FONT_SIZE_MIN = 10;
const FONT_SIZE_MAX = 22;
const FONT_SIZE_DEFAULT = 13;
const SCROLLBACK_MIN = 1000;
const SCROLLBACK_MAX = 50000;
const SCROLLBACK_DEFAULT = 5000;

const FONT_FAMILY_STACKS: Record<Exclude<FontFamilyPreset, "custom">, string> = {
  default: 'JetBrains Mono Variable, "JetBrains Mono", Cascadia Code, Consolas, monospace',
  cascadia: '"Cascadia Code", "Cascadia Mono", Consolas, monospace',
  consolas: 'Consolas, "Courier New", monospace',
  menlo: 'Menlo, Monaco, "DejaVu Sans Mono", monospace',
};

export function resolveFontFamily(preset: FontFamilyPreset, custom: string): string {
  if (preset === "custom") return custom.trim() || FONT_FAMILY_STACKS.default;
  return FONT_FAMILY_STACKS[preset];
}

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
  fontFamilyPreset = $state<FontFamilyPreset>("default");
  fontFamilyCustom = $state("");
  scrollback = $state(SCROLLBACK_DEFAULT);
  cursorStyle = $state<CursorStyle>("bar");
  cursorBlink = $state(true);
  bellStyle = $state<BellStyle>("none");
  copyOnSelect = $state(false);
  rightClickPaste = $state(true);
  themePreset = $state<ThemePresetId>("rift");
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
      const ff = localStorage.getItem(FONT_FAMILY_KEY) as FontFamilyPreset | null;
      if (ff && ["default", "cascadia", "consolas", "menlo", "custom"].includes(ff)) {
        this.fontFamilyPreset = ff;
      }
      const ffc = localStorage.getItem(FONT_FAMILY_CUSTOM_KEY);
      if (ffc) this.fontFamilyCustom = ffc;
      const sb = parseInt(localStorage.getItem(SCROLLBACK_KEY) ?? "", 10);
      if (!isNaN(sb) && sb >= SCROLLBACK_MIN && sb <= SCROLLBACK_MAX) this.scrollback = sb;
      const cs = localStorage.getItem(CURSOR_STYLE_KEY) as CursorStyle | null;
      if (cs === "bar" || cs === "block" || cs === "underline") this.cursorStyle = cs;
      const cb = localStorage.getItem(CURSOR_BLINK_KEY);
      if (cb === "0" || cb === "1") this.cursorBlink = cb === "1";
      const bs = localStorage.getItem(BELL_STYLE_KEY) as BellStyle | null;
      if (bs === "none" || bs === "visual" || bs === "sound") this.bellStyle = bs;
      const cos = localStorage.getItem(COPY_ON_SELECT_KEY);
      if (cos === "0" || cos === "1") this.copyOnSelect = cos === "1";
      const rcp = localStorage.getItem(RIGHT_CLICK_PASTE_KEY);
      if (rcp === "0" || rcp === "1") this.rightClickPaste = rcp === "1";
      const tp = localStorage.getItem(THEME_PRESET_KEY) as ThemePresetId | null;
      if (tp && ["rift", "dracula", "solarized-dark", "monokai", "github-dark"].includes(tp)) {
        this.themePreset = tp;
      }
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
        customLabel: t.customLabel || undefined,
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
    // HMR guard: TerminalPanel.onMount fires on every hot reload, but the
    // store singleton survives. If tabs are already populated, this is an
    // HMR rerun — skip the restore so we don't accumulate (which then
    // persistTabs writes back to localStorage permanently).
    if (this.tabs.length > 0) return;
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
    for (const t of capped) this.addTab(t.shellId, t.shellLabel, t.autoLaunch ?? "", t.customLabel ?? "");
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

  toggle() {
    // Under v0.4.1 the embedded overlay is gone — the terminal lives in the
    // right pane. Route through rightPane.toggle so Ctrl+` and every existing
    // terminal.toggle() call site stays valid in both shells.
    if (uiPrefs.useV03Shell) {
      rightPane.toggle("terminal");
      return;
    }
    this.setOpen(!this.open);
  }

  setHeight(h: number) {
    this.height = h;
    try { localStorage.setItem(HEIGHT_KEY, String(Math.round(h))); } catch { /* noop */ }
  }
  resetHeight() { this.setHeight(HEIGHT_DEFAULT); }

  addTab(shellId: string | null, shellLabel: string = "Terminal", autoLaunch: string = "", customLabel: string = ""): string {
    const id = uid();
    const tab: TermTab = {
      id,
      sessionId: null,
      shellId,
      shellLabel,
      customLabel,
      autoLaunch,
      status: "starting",
    };
    this.tabs = [...this.tabs, tab];
    this.activeTabId = id;
    this.persistTabs();
    return id;
  }

  renameTab(id: string, label: string) {
    const trimmed = label.trim();
    this.tabs = this.tabs.map((t) => (t.id === id ? { ...t, customLabel: trimmed } : t));
    this.persistTabs();
  }

  cycleTab(direction: 1 | -1) {
    if (this.tabs.length < 2) return;
    const idx = this.tabs.findIndex((t) => t.id === this.activeTabId);
    if (idx < 0) return;
    const next = (idx + direction + this.tabs.length) % this.tabs.length;
    this.activeTabId = this.tabs[next].id;
    this.persistTabs();
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

  setFontFamilyPreset(p: FontFamilyPreset) {
    this.fontFamilyPreset = p;
    try { localStorage.setItem(FONT_FAMILY_KEY, p); } catch { /* noop */ }
  }
  setFontFamilyCustom(s: string) {
    this.fontFamilyCustom = s;
    try {
      if (s) localStorage.setItem(FONT_FAMILY_CUSTOM_KEY, s);
      else localStorage.removeItem(FONT_FAMILY_CUSTOM_KEY);
    } catch { /* noop */ }
  }
  setScrollback(n: number) {
    const clamped = Math.max(SCROLLBACK_MIN, Math.min(SCROLLBACK_MAX, Math.round(n)));
    this.scrollback = clamped;
    try { localStorage.setItem(SCROLLBACK_KEY, String(clamped)); } catch { /* noop */ }
  }
  setCursorStyle(c: CursorStyle) {
    this.cursorStyle = c;
    try { localStorage.setItem(CURSOR_STYLE_KEY, c); } catch { /* noop */ }
  }
  setCursorBlink(v: boolean) {
    this.cursorBlink = v;
    try { localStorage.setItem(CURSOR_BLINK_KEY, v ? "1" : "0"); } catch { /* noop */ }
  }
  setBellStyle(b: BellStyle) {
    this.bellStyle = b;
    try { localStorage.setItem(BELL_STYLE_KEY, b); } catch { /* noop */ }
  }
  setCopyOnSelect(v: boolean) {
    this.copyOnSelect = v;
    try { localStorage.setItem(COPY_ON_SELECT_KEY, v ? "1" : "0"); } catch { /* noop */ }
  }
  setRightClickPaste(v: boolean) {
    this.rightClickPaste = v;
    try { localStorage.setItem(RIGHT_CLICK_PASTE_KEY, v ? "1" : "0"); } catch { /* noop */ }
  }
  setThemePreset(t: ThemePresetId) {
    this.themePreset = t;
    try { localStorage.setItem(THEME_PRESET_KEY, t); } catch { /* noop */ }
  }

  resetAppearance() {
    this.setFontSize(FONT_SIZE_DEFAULT);
    this.setFontFamilyPreset("default");
    this.setFontFamilyCustom("");
    this.setScrollback(SCROLLBACK_DEFAULT);
    this.setCursorStyle("bar");
    this.setCursorBlink(true);
    this.setBellStyle("none");
    this.setCopyOnSelect(false);
    this.setRightClickPaste(true);
    this.setThemePreset("rift");
  }
}

export const terminal = new TerminalStore();

export const TERM_HEIGHT_MIN = HEIGHT_MIN;
export const TERM_FONT_SIZE_MIN = FONT_SIZE_MIN;
export const TERM_FONT_SIZE_MAX = FONT_SIZE_MAX;
export const TERM_SCROLLBACK_MIN = SCROLLBACK_MIN;
export const TERM_SCROLLBACK_MAX = SCROLLBACK_MAX;
