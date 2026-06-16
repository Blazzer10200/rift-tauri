// Environment / Source-Control dock — UI-state singleton for the right-side
// panel that surfaces the git working tree. Mirrors browserDock/activityDock
// (open + width, localStorage-persisted) and is deliberately a FRESH store, not
// a revival of the removed activity dock's `assistant.ui.dockOpen/dockWidth`
// (CLAUDE.md guardrail) — distinct keys, distinct purpose.

const OPEN_KEY = "rift.environment.panel.open.v1";
const WIDTH_KEY = "rift.environment.panel.width.v1";

const MIN_W = 320;
const MAX_W = 820;
const DEFAULT_W = 480;

class EnvironmentDock {
  open = $state(false);
  width = $state(DEFAULT_W);

  init() {
    if (typeof window === "undefined") return;
    this.open = localStorage.getItem(OPEN_KEY) === "1";
    const w = Number(localStorage.getItem(WIDTH_KEY));
    if (Number.isFinite(w) && w >= MIN_W && w <= MAX_W) this.width = w;
  }

  toggle() {
    this.open = !this.open;
    try { localStorage.setItem(OPEN_KEY, this.open ? "1" : "0"); } catch { /* noop */ }
  }

  setWidth(w: number) {
    this.width = Math.max(MIN_W, Math.min(MAX_W, Math.round(w)));
    try { localStorage.setItem(WIDTH_KEY, String(this.width)); } catch { /* noop */ }
  }
}

export const environmentDock = new EnvironmentDock();
