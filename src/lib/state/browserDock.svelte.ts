// In-assistant web browser dock. A thin, isolated UI-state singleton (kept out
// of the large AssistantStore) shared by the topbar/Shift+B toggle and
// AssistantPage (the dock layout). The actual browser lives in the Rust
// `browser` module, driven via the `browser_*` Tauri commands; this only tracks
// whether the dock is shown and how wide it is.

const OPEN_KEY = "rift.assistant.browserDock.open.v1";
const WIDTH_KEY = "rift.assistant.browserDock.width.v1";
const LAST_URL_KEY = "rift.assistant.browserDock.lastUrl.v1";

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
  // Last successfully loaded page — closing the dock destroys the native
  // webview, so this powers the empty-state "Reopen <host>" affordance.
  lastUrl = $state<string | null>(null);
  // "Read by assistant" indicator — set from the tool stream (streaming.ts)
  // when a read_browser_* tool fires. The token re-triggers the chip on
  // back-to-back reads of the same kind.
  assistantRead = $state<{ kind: "page" | "console"; token: number } | null>(null);

  init() {
    if (typeof window === "undefined") return;
    this.open = localStorage.getItem(OPEN_KEY) === "1";
    const w = Number(localStorage.getItem(WIDTH_KEY));
    if (Number.isFinite(w) && w >= MIN_W && w <= MAX_W) this.width = w;
    const last = localStorage.getItem(LAST_URL_KEY);
    if (last && /^https?:\/\//i.test(last)) this.lastUrl = last;
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

  setLastUrl(url: string) {
    if (!/^https?:\/\//i.test(url)) return; // never persist about:blank etc.
    this.lastUrl = url;
    try { localStorage.setItem(LAST_URL_KEY, url); } catch { /* noop */ }
  }

  noteAssistantRead(kind: "page" | "console") {
    this.assistantRead = { kind, token: (this.assistantRead?.token ?? 0) + 1 };
  }

  setWidth(w: number) {
    this.width = Math.max(MIN_W, Math.min(MAX_W, Math.round(w)));
    try { localStorage.setItem(WIDTH_KEY, String(this.width)); } catch { /* noop */ }
  }
}

export const browserDock = new BrowserDock();
