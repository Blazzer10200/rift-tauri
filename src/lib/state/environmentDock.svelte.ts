// Environment / Source-Control floating box — UI-state singleton for the
// top-right floating widget over the chat. Collapsed = a branch/changes pill;
// expanded = the full source-control panel. Auto-shows once a chat has messages
// (unless the user dismissed it) and re-opens from the View menu. Deliberately a
// FRESH store, not a revival of the removed activity dock's
// `assistant.ui.dockOpen/dockWidth` (CLAUDE.md guardrail) — distinct keys.

const EXPANDED_KEY = "rift.environment.expanded.v1";

class EnvironmentDock {
  // Visible at all. Auto-managed (chat-start) so it is NOT persisted.
  open = $state(false);
  // Pill vs full panel — a sticky user preference, persisted.
  expanded = $state(false);
  // Set when the user dismisses the box; suppresses auto-show until they
  // re-open it from the View menu. Session-only.
  userClosed = $state(false);

  init() {
    if (typeof window === "undefined") return;
    this.expanded = localStorage.getItem(EXPANDED_KEY) === "1";
  }

  /** Auto-show entry point — opens unless the user explicitly dismissed it. */
  autoShow() {
    if (!this.userClosed) this.open = true;
  }

  show() {
    this.open = true;
    this.userClosed = false;
  }

  close() {
    this.open = false;
    this.userClosed = true;
  }

  toggle() {
    if (this.open) this.close();
    else this.show();
  }

  setExpanded(v: boolean) {
    this.expanded = v;
    try { localStorage.setItem(EXPANDED_KEY, v ? "1" : "0"); } catch { /* noop */ }
  }

  toggleExpanded() { this.setExpanded(!this.expanded); }
}

export const environmentDock = new EnvironmentDock();
