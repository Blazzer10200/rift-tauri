// Global update store. One instance per session: triggers a launch-time
// check, caches the result, drives the popup dialog + StatusBar pill +
// corner toast, and pumps download progress from the backend.
//
// State machine:
//   idle → checking → available → downloading → ready → applying
//                  ↘ uptodate
//                  ↘ error (recoverable, retryable)
//
// `dismissedVersion` is persisted in localStorage so a snoozed update for
// version X doesn't pop the toast again every relaunch. A NEWER version
// supersedes — the toast will fire for the new tag even if the prior was
// snoozed.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type UpdateInfo = {
  version: string;
  releaseName: string;
  sizeBytes: number;
  notesMarkdown: string;
  releaseUrl: string;
  publishedAt: string;
};

export type UpdateState =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "applying"
  | "uptodate"
  | "error";

const DISMISSED_KEY = "rift.updates.dismissed-version";

function loadDismissed(): string | null {
  try { return localStorage.getItem(DISMISSED_KEY); } catch { return null; }
}
function saveDismissed(v: string | null) {
  try {
    if (v) localStorage.setItem(DISMISSED_KEY, v);
    else   localStorage.removeItem(DISMISSED_KEY);
  } catch { /* private mode etc — non-fatal */ }
}

class UpdateStore {
  state = $state<UpdateState>("idle");
  info = $state<UpdateInfo | null>(null);
  error = $state("");
  currentVersion = $state("?");
  progress = $state(0);
  dialogOpen = $state(false);
  toastVisible = $state(false);
  dismissedVersion = $state<string | null>(loadDismissed());

  private progressUnlisten: UnlistenFn | null = null;
  private downloadedUnlisten: UnlistenFn | null = null;
  private sizeUnlisten: UnlistenFn | null = null;

  /** True when there's an unsnoozed update waiting for user action. */
  get pillVisible(): boolean {
    return (
      (this.state === "available" || this.state === "ready") &&
      !this.toastVisible &&
      !this.dialogOpen
    );
  }

  /** Human-readable "12.4 MB" style. */
  get sizeLabel(): string {
    const b = this.info?.sizeBytes ?? 0;
    if (b <= 0) return "";
    const mb = b / (1024 * 1024);
    if (mb >= 1) return `${mb.toFixed(1)} MB`;
    return `${(b / 1024).toFixed(0)} KB`;
  }

  /** ISO date → "May 19, 2026". Empty string if backend didn't fill it. */
  get publishedLabel(): string {
    const raw = this.info?.publishedAt ?? "";
    if (!raw) return "";
    try {
      return new Date(raw).toLocaleDateString([], { year: "numeric", month: "short", day: "numeric" });
    } catch { return raw; }
  }

  async refresh() {
    this.state = "checking";
    this.error = "";
    try {
      this.currentVersion = await invoke<string>("app_version");
    } catch (e) {
      console.warn("app_version invoke failed", e);
    }
    try {
      const res = await invoke<UpdateInfo | null>("check_for_updates");
      if (res) {
        this.info = res;
        this.state = "available";
      } else {
        this.info = null;
        this.state = "uptodate";
      }
    } catch (e) {
      this.error = String(e);
      this.state = "error";
    }
  }

  /** Download the pending package. Tauri streams `update-progress` events. */
  async download() {
    if (this.state !== "available") return;
    this.state = "downloading";
    this.progress = 0;
    this.error = "";
    try {
      await this.ensureListeners();
      await invoke<void>("download_update");
      // `update-downloaded` event flips us to `ready`; defensive set here in
      // case the event fires before this await resolves.
      if ((this.state as UpdateState) !== "ready") this.state = "ready";
    } catch (e) {
      this.error = String(e);
      this.state = "error";
    }
  }

  /** Apply the staged download + restart. Control never returns on success. */
  async applyNow() {
    if (this.state !== "ready") return;
    this.state = "applying";
    this.error = "";
    try {
      await invoke<void>("apply_pending_update");
    } catch (e) {
      this.error = String(e);
      this.state = "error";
    }
  }

  /** Snooze the current available version — toast + pill stay quiet until a
   *  newer version ships. Closes the dialog if open. */
  snooze() {
    if (this.info?.version) {
      this.dismissedVersion = this.info.version;
      saveDismissed(this.info.version);
    }
    this.toastVisible = false;
    this.dialogOpen = false;
  }

  dismissToast() { this.toastVisible = false; }

  /** Called once on app launch from AppShell.onMount.
   *
   *  When an unsnoozed update is detected, surface the toast AND start the
   *  background download. By the time the user opens the dialog → Install,
   *  the bytes are already on disk so the only wait is the NSIS apply
   *  (sub-30s on the Tauri-updater path). Idempotent — `download_update`
   *  on the backend short-circuits when the pending Update is already
   *  resolved. */
  async checkOnLaunch() {
    await this.refresh();
    if (
      this.state === "available" &&
      this.info &&
      this.info.version !== this.dismissedVersion
    ) {
      this.toastVisible = true;
      void this.download();
    }
  }

  open()  { this.dialogOpen = true; this.toastVisible = false; }
  close() { this.dialogOpen = false; }

  private async ensureListeners() {
    if (!this.progressUnlisten) {
      this.progressUnlisten = await listen<number>("update-progress", (e) => {
        const pct = typeof e.payload === "number" ? e.payload : Number(e.payload);
        if (!Number.isFinite(pct)) return;
        this.progress = Math.max(0, Math.min(100, Math.round(pct)));
      });
    }
    if (!this.downloadedUnlisten) {
      this.downloadedUnlisten = await listen<unknown>("update-downloaded", () => {
        this.progress = 100;
        this.state = "ready";
      });
    }
    if (!this.sizeUnlisten) {
      // tauri-plugin-updater doesn't expose Content-Length before the byte
      // stream begins, so the backend emits `update-size` once on first
      // chunk. Patch info.sizeBytes so the dialog's "X of Y MB" label resolves.
      this.sizeUnlisten = await listen<number>("update-size", (e) => {
        const bytes = typeof e.payload === "number" ? e.payload : Number(e.payload);
        if (!Number.isFinite(bytes) || bytes <= 0) return;
        if (this.info) this.info.sizeBytes = bytes;
      });
    }
  }

  /** #173: tear down Tauri event listeners. Wired via `import.meta.hot.dispose`
   *  so HMR doesn't stack duplicates; AppShell may also call this on unmount
   *  (separate milestone). Safe to call when listeners were never installed. */
  dispose() {
    if (this.progressUnlisten) {
      try { this.progressUnlisten(); } catch (e) { console.warn("update-progress unlisten threw", e); }
      this.progressUnlisten = null;
    }
    if (this.downloadedUnlisten) {
      try { this.downloadedUnlisten(); } catch (e) { console.warn("update-downloaded unlisten threw", e); }
      this.downloadedUnlisten = null;
    }
    if (this.sizeUnlisten) {
      try { this.sizeUnlisten(); } catch (e) { console.warn("update-size unlisten threw", e); }
      this.sizeUnlisten = null;
    }
  }
}

export const updates = new UpdateStore();

// #173: HMR teardown so a hot-reload doesn't leave the old listeners firing
// into a stale store instance.
if (typeof import.meta !== "undefined" && (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot) {
  (import.meta as { hot: { dispose: (cb: () => void) => void } }).hot.dispose(() => updates.dispose());
}
