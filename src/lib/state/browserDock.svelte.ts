// In-assistant web browser dock. A thin, isolated UI-state singleton (kept out
// of the large AssistantStore) shared by ChatTabsBar (the toggle) and
// AssistantPage (the dock layout). The actual browser lives in the Rust
// `browser` module, driven via the `browser_*` Tauri commands; this only tracks
// whether the dock is shown and how wide it is.

const OPEN_KEY = "rift.assistant.browserDock.open.v1";
const WIDTH_KEY = "rift.assistant.browserDock.width.v1";

const MIN_W = 360;
const MAX_W = 1200;
const DEFAULT_W = 560;

class BrowserDock {
  open = $state(false);
  width = $state(DEFAULT_W);
  // Bumped to request the address bar take focus + select-all (Ctrl+L). The
  // WebBrowserPage input watches this token rather than holding a DOM ref here.
  focusToken = $state(0);
  // URL queued for the dock to navigate to — set by the assistant
  // (`assistant://open-browser`) or a localhost link click in the chat.
  // WebBrowserPage consumes it once its stage element exists (the dock may
  // need a mount cycle first when this call also opens it).
  pendingUrl = $state<string | null>(null);

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

  // Open the dock if needed, then ask the address bar to focus.
  focusAddress() {
    if (!this.open) this.toggle();
    this.focusToken++;
  }

  // Queue a URL for the dock and make sure the dock is showing.
  openUrl(url: string) {
    this.pendingUrl = url;
    if (!this.open) this.toggle();
  }

  setWidth(w: number) {
    this.width = Math.max(MIN_W, Math.min(MAX_W, Math.round(w)));
    try { localStorage.setItem(WIDTH_KEY, String(this.width)); } catch { /* noop */ }
  }
}

export const browserDock = new BrowserDock();
