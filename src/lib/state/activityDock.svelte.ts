// Sub-agent live-activity dock. A thin UI-state singleton (kept out of the large
// AssistantStore) shared by SubAgentDock (the toggle) and AssistantPage (the dock
// layout + activity sync). It tracks whether the dock is shown and how wide it
// is; the data lives on the active tab's `agentSpawns`. The dock is SMART: it
// auto-reveals while sub-agents run and auto-dismisses a few seconds after they
// all finish — unless the user has manually toggled or moused into it this cycle.

const OPEN_KEY = "rift.assistant.activityDock.open.v1";
const WIDTH_KEY = "rift.assistant.activityDock.width.v1";
const ENABLED_KEY = "rift.assistant.activityDock.enabled.v1";

const MIN_W = 320;
const MAX_W = 900;
const DEFAULT_W = 440;
// Grace window after the last sub-agent finishes before an auto-revealed dock
// slides away — long enough to glance at the result, short enough to feel tidy.
const DISMISS_MS = 6000;

class ActivityDock {
  // Master switch (Settings → Chat). When off, the dock never auto-reveals and
  // its toggle/render are suppressed entirely. Default on. Closing the dock is a
  // per-cycle dismissal (`open`); disabling it is the durable opt-out.
  enabled = $state(true);
  open = $state(false);
  width = $state(DEFAULT_W);

  // Auto-visibility bookkeeping (plain fields — they gate behavior, not render).
  private autoShown = false;   // dock is currently shown BY activity, not the user
  private userPinned = false;  // user toggled/hovered this cycle → never auto-dismiss
  private dismissTimer: ReturnType<typeof setTimeout> | null = null;

  init() {
    if (typeof window === "undefined") return;
    // Default on — only an explicit "0" disables (absent key = first run = on).
    this.enabled = localStorage.getItem(ENABLED_KEY) !== "0";
    this.open = localStorage.getItem(OPEN_KEY) === "1";
    const w = Number(localStorage.getItem(WIDTH_KEY));
    if (Number.isFinite(w) && w >= MIN_W && w <= MAX_W) this.width = w;
  }

  /** Master enable/disable (Settings). Disabling tears the dock down immediately
   *  and resets the auto-controller so re-enabling starts clean. */
  setEnabled(on: boolean) {
    this.enabled = on;
    try { localStorage.setItem(ENABLED_KEY, on ? "1" : "0"); } catch { /* noop */ }
    if (!on) {
      this.open = false;
      this.autoShown = false;
      this.userPinned = false;
      this.clearDismiss();
    }
  }

  /** User action — pins the dock open/closed for this activity cycle so the auto
   *  controller never fights the user, and persists the preference. */
  toggle() {
    if (!this.enabled) return;
    this.userPinned = true;
    this.autoShown = false;
    this.clearDismiss();
    this.open = !this.open;
    try { localStorage.setItem(OPEN_KEY, this.open ? "1" : "0"); } catch { /* noop */ }
  }

  /** The user moused into the dock — they're reading it, so cancel any pending
   *  auto-dismiss and keep it up until they close it themselves. */
  notePointerEnter() {
    if (this.autoShown && !this.userPinned) {
      this.userPinned = true;
      this.clearDismiss();
    }
  }

  /** Called reactively with the active tab's sub-agent counts. Drives the
   *  auto-reveal / auto-dismiss. Auto open/close is NOT persisted — only an
   *  explicit user toggle changes the remembered preference. */
  syncActivity(running: number, total: number) {
    // Master switch off → never auto-reveal; the dock stays torn down.
    if (!this.enabled) return;
    if (total === 0) {
      // Fresh turn with no sub-agents — reset the controller so the next batch
      // can auto-manage again. Leave a user-opened empty dock alone.
      this.userPinned = false;
      this.autoShown = false;
      this.clearDismiss();
      return;
    }
    if (running > 0) {
      this.clearDismiss();
      if (!this.open && !this.userPinned) {
        this.open = true;
        this.autoShown = true;
      }
      return;
    }
    // All sub-agents finished — schedule the slide-away if WE revealed it.
    if (this.autoShown && !this.userPinned && this.dismissTimer === null) {
      this.dismissTimer = setTimeout(() => {
        this.dismissTimer = null;
        if (this.autoShown && !this.userPinned) {
          this.open = false;
          this.autoShown = false;
        }
      }, DISMISS_MS);
    }
  }

  private clearDismiss() {
    if (this.dismissTimer !== null) {
      clearTimeout(this.dismissTimer);
      this.dismissTimer = null;
    }
  }

  setWidth(w: number) {
    this.width = Math.max(MIN_W, Math.min(MAX_W, Math.round(w)));
    try { localStorage.setItem(WIDTH_KEY, String(this.width)); } catch { /* noop */ }
  }
}

export const activityDock = new ActivityDock();
