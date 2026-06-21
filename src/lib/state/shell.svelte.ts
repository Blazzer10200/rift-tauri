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

  readonly minWidth = MIN_W;
  readonly maxWidth = MAX_W;

  init() {
    if (typeof window === "undefined") return;
    this.collapsed = localStorage.getItem(COLLAPSE_KEY) === "1";
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
    if (typeof window !== "undefined") {
      localStorage.setItem(COLLAPSE_KEY, this.collapsed ? "1" : "0");
    }
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
