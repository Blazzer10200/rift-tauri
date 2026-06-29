// Sub-agent live-activity master switch. A thin UI-state singleton (kept out of
// the large AssistantStore) holding the Settings → Chat enable/disable toggle for
// the sub-agent dock. Per-pane expand/collapse + auto-reveal now live INSIDE each
// SubAgentDock instance (one per pane, scoped to its tab) — so this singleton is
// just the durable master opt-out. When disabled, no pane renders its dock.

const ENABLED_KEY = "rift.assistant.activityDock.enabled.v1";

class ActivityDock {
  // Master switch (Settings → Chat). When off, no pane's dock auto-reveals and
  // its render is suppressed entirely. Default on.
  enabled = $state(true);

  init() {
    if (typeof window === "undefined") return;
    // Default on — only an explicit "0" disables (absent key = first run = on).
    this.enabled = localStorage.getItem(ENABLED_KEY) !== "0";
  }

  /** Master enable/disable (Settings). */
  setEnabled(on: boolean) {
    this.enabled = on;
    try { localStorage.setItem(ENABLED_KEY, on ? "1" : "0"); } catch { /* noop */ }
  }
}

export const activityDock = new ActivityDock();
