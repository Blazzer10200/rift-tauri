// Redesign shell UI state — sidebar collapse + width, shared between Sidebar
// (rail) and Topbar (the show-side toggle). Persisted to localStorage so the
// rail's open/collapsed state and width survive reloads.

import { browserDock } from "./browserDock.svelte";

const COLLAPSE_KEY = "rift.ui.sidebar-collapsed.v1";
const WIDTH_KEY = "rift.ui.sidebar-width.v1";
const PINNED_KEY = "rift.ui.pinned-convos.v1";
const ALL_PROJECTS_KEY = "rift.ui.conv-all-projects.v1";
const PROJECTS_OPEN_KEY = "rift.ui.projects-expanded.v1";
const HISTORY_OPEN_KEY = "rift.ui.history-expanded.v1";
const MIN_W = 208;
const MAX_W = 380;
const DEFAULT_W = 248;

/** Conversation-list context. `all` is an application view, never a project
 *  id/path and never a nullable workspace alias. */
export type ConversationScope =
  | { kind: "focused-workspace" }
  | { kind: "all" };

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
  /** Transient hover-peek: while collapsed, the island floats over the content
   *  as long as the cursor is on the topbar trigger or the island itself.
   *  Never persisted — a pin (toggleCollapsed) always clears it. */
  peek = $state(false);
  private peekTimer: ReturnType<typeof setTimeout> | null = null;
  /** Live during a drag on the rail's resize handle — suppresses transitions. */
  resizing = $state(false);
  /** Pinned conversation ids — frontend-only (the backend meta has no pin). */
  pinned = $state<Set<string>>(new Set<string>());
  /** Explicit sidebar view. It filters history only and cannot mutate pane or
   *  workspace identity. */
  conversationScope = $state<ConversationScope>({ kind: "focused-workspace" });
  get allProjects(): boolean { return this.conversationScope.kind === "all"; }
  /** Projects list expanded (show all) vs collapsed (active project only).
   *  Progressive disclosure so a 20-project rail stays 2 rows tall by default. */
  projectsExpanded = $state(false);
  /** Conversation history shows Today only by default; expanded reveals all the
   *  older date groups. One "Show earlier" toggle instead of per-group chevrons. */
  historyExpanded = $state(false);

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
    const scope = localStorage.getItem(ALL_PROJECTS_KEY);
    this.conversationScope = scope === "1" || scope === "all"
      ? { kind: "all" }
      : { kind: "focused-workspace" };
    this.projectsExpanded = localStorage.getItem(PROJECTS_OPEN_KEY) === "1";
    this.historyExpanded = localStorage.getItem(HISTORY_OPEN_KEY) === "1";
  }

  toggleProjectsExpanded() {
    this.projectsExpanded = !this.projectsExpanded;
    if (typeof window !== "undefined") localStorage.setItem(PROJECTS_OPEN_KEY, this.projectsExpanded ? "1" : "0");
  }

  toggleHistoryExpanded() {
    this.historyExpanded = !this.historyExpanded;
    if (typeof window !== "undefined") localStorage.setItem(HISTORY_OPEN_KEY, this.historyExpanded ? "1" : "0");
  }

  isPinned(id: string): boolean { return this.pinned.has(id); }

  toggleAllProjects() {
    this.setAllProjects(!this.allProjects);
  }

  setAllProjects(v: boolean) {
    this.conversationScope = v ? { kind: "all" } : { kind: "focused-workspace" };
    if (typeof window !== "undefined") {
      localStorage.setItem(ALL_PROJECTS_KEY, v ? "all" : "workspace");
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
    this.endPeek();
    // A manual toggle while narrow becomes the user's new intent: clear the
    // auto-flag so a later widen doesn't override what they just chose.
    this.autoCollapsed = false;
    this.userCollapsed = this.collapsed;
    if (typeof window !== "undefined") {
      localStorage.setItem(COLLAPSE_KEY, this.collapsed ? "1" : "0");
    }
  }

  /** Open the hover-peek island (collapsed only). Cancels a pending close so
   *  moving trigger → island doesn't flicker. */
  beginPeek() {
    if (!this.collapsed) return;
    this.cancelPeekClose();
    this.peek = true;
  }

  /** Schedule the peek to retract — the grace delay covers the cursor's hop
   *  from the topbar trigger down onto the island. */
  schedulePeekClose(delayMs = 260) {
    this.cancelPeekClose();
    this.peekTimer = setTimeout(() => {
      this.peek = false;
      this.peekTimer = null;
    }, delayMs);
  }

  cancelPeekClose() {
    if (this.peekTimer) {
      clearTimeout(this.peekTimer);
      this.peekTimer = null;
    }
  }

  private endPeek() {
    this.cancelPeekClose();
    this.peek = false;
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
      this.endPeek();
    } else if (winWidth >= NARROW_REOPEN_W && this.autoCollapsed) {
      // Widening past the reopen mark: restore the rail we auto-hid.
      this.autoCollapsed = false;
      this.collapsed = this.userCollapsed;
      this.endPeek();
    }
  }

  /** How many chat panes the current viewport can hold without slivers.
   *  Accounts for the live sidebar footprint AND the browser dock when open —
   *  both eat real content width, so ignoring either hands out sliver panes.
   *  Always ≥1. */
  maxPanesForWidth(): number {
    if (typeof window === "undefined") return 1;
    const sidebar = this.collapsed ? 0 : this.width;
    const dock = browserDock.open ? browserDock.width : 0;
    const content = Math.max(0, window.innerWidth - sidebar - dock);
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
