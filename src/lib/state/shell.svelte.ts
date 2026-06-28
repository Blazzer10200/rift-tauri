// Redesign shell UI state — sidebar collapse + width, shared between Sidebar
// (rail) and Topbar (the show-side toggle). Persisted to localStorage so the
// rail's open/collapsed state and width survive reloads.

const COLLAPSE_KEY = "rift.ui.sidebar-collapsed.v1";
const WIDTH_KEY = "rift.ui.sidebar-width.v1";
const PINNED_KEY = "rift.ui.pinned-convos.v1";
const ALL_PROJECTS_KEY = "rift.ui.conv-all-projects.v1";
const MIN_W = 208;
const MAX_W = 380;
const DEFAULT_W = 248;

// Below this window width the sidebar auto-collapses to give the main content
// room (1366px laptops, 150%-scaled 1080p). Hysteresis: re-open only past a
// wider mark so a window parked near the edge doesn't flicker open/closed.
const NARROW_COLLAPSE_W = 1100;
const NARROW_REOPEN_W = 1180;

// Min usable content width per chat pane. addPane() refuses to split below this
// so a 4-way split never produces unusable slivers on a small screen.
const MIN_PANE_W = 360;

function clampW(w: number): number {
  return Math.max(MIN_W, Math.min(MAX_W, Math.round(w)));
}

class ShellState {
  collapsed = $state(false);
  width = $state(DEFAULT_W);
  /** Live during a drag on the rail's resize handle — suppresses transitions. */
  resizing = $state(false);
  /** Conversation-list search query (sidebar). */
  convQuery = $state("");
  /** Pinned conversation ids — frontend-only (the backend meta has no pin). */
  pinned = $state<Set<string>>(new Set<string>());
  /** Sidebar scope: false (default) = show only the open project's chats;
   *  true = show every project's chats (with a per-row project label). */
  allProjects = $state(false);

  /** True while the window is narrow enough that we auto-collapsed the rail.
   *  Tracked separately from `collapsed` so widening restores the user's own
   *  open/closed choice rather than force-opening a rail they'd closed. */
  autoCollapsed = $state(false);
  /** The user's explicit collapse choice, snapshotted when an auto-collapse
   *  kicks in so we can restore it on widen. */
  private userCollapsed = false;

  readonly minWidth = MIN_W;
  readonly maxWidth = MAX_W;

  init() {
    if (typeof window === "undefined") return;
    this.collapsed = localStorage.getItem(COLLAPSE_KEY) === "1";
    this.userCollapsed = this.collapsed;
    // Honor a narrow boot window immediately (auto-collapse before first paint).
    this.syncToViewport(window.innerWidth);
    const raw = localStorage.getItem(WIDTH_KEY);
    const n = raw ? Number(raw) : NaN;
    if (Number.isFinite(n)) this.width = clampW(n);
    try {
      const ids = JSON.parse(localStorage.getItem(PINNED_KEY) ?? "[]") as unknown;
      if (Array.isArray(ids)) this.pinned = new Set(ids.filter((s): s is string => typeof s === "string"));
    } catch (e) { console.warn("[shell] pinned-convos parse failed:", e); }
    this.allProjects = localStorage.getItem(ALL_PROJECTS_KEY) === "1";
  }

  isPinned(id: string): boolean { return this.pinned.has(id); }

  toggleAllProjects() {
    this.allProjects = !this.allProjects;
    if (typeof window !== "undefined") {
      localStorage.setItem(ALL_PROJECTS_KEY, this.allProjects ? "1" : "0");
    }
  }

  togglePin(id: string) {
    const next = new Set(this.pinned);
    if (next.has(id)) next.delete(id); else next.add(id);
    this.pinned = next;
    if (typeof window !== "undefined") {
      localStorage.setItem(PINNED_KEY, JSON.stringify([...next]));
    }
  }

  toggleCollapsed() {
    this.collapsed = !this.collapsed;
    // A manual toggle while narrow becomes the user's new intent: clear the
    // auto-flag so a later widen doesn't override what they just chose.
    this.autoCollapsed = false;
    this.userCollapsed = this.collapsed;
    if (typeof window !== "undefined") {
      localStorage.setItem(COLLAPSE_KEY, this.collapsed ? "1" : "0");
    }
  }

  /** Drive auto-collapse from the live window width. Call on resize + once at
   *  boot. Only touches `collapsed` when crossing a threshold, and never
   *  overwrites the persisted choice in localStorage (auto-state is transient). */
  syncToViewport(winWidth: number) {
    if (winWidth <= NARROW_COLLAPSE_W && !this.collapsed) {
      // Narrowing: remember the user's open state, collapse for room.
      this.userCollapsed = false;
      this.autoCollapsed = true;
      this.collapsed = true;
    } else if (winWidth >= NARROW_REOPEN_W && this.autoCollapsed) {
      // Widening past the reopen mark: restore the rail we auto-hid.
      this.autoCollapsed = false;
      this.collapsed = this.userCollapsed;
    }
  }

  /** How many chat panes the current viewport can hold without slivers.
   *  Accounts for the live sidebar footprint when it's open. Always ≥1. */
  maxPanesForWidth(): number {
    if (typeof window === "undefined") return 1;
    const sidebar = this.collapsed ? 0 : this.width;
    const content = Math.max(0, window.innerWidth - sidebar);
    return Math.max(1, Math.floor(content / MIN_PANE_W));
  }

  setWidth(w: number) {
    this.width = clampW(w);
  }

  commitWidth() {
    if (typeof window !== "undefined") {
      localStorage.setItem(WIDTH_KEY, String(this.width));
    }
  }
}

export const shell = new ShellState();
