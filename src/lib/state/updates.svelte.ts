// Update store — v0.4.34+ GH-release-API path.
//
// Replaced `tauri-plugin-updater` (2026-05-27 → 2026-05-27 brief lifetime;
// signing-key loss bricks all clients permanently, see commands/update.rs).
// Backend `check_for_updates` polls the latest GitHub release; on user
// confirm we open the Setup.exe asset URL via `tauri-plugin-opener`. NSIS
// handles install over the running binary (its template prompts to close
// Rift if needed, then relaunches).
//
// State machine:
//   idle → checking → available → launched
//                  ↘ uptodate
//                  ↘ error
//
// `launched` = user clicked Download, browser opened the asset URL. They run
// Setup.exe externally; next launch is the new version.
//
// `dismissedVersion` is persisted in localStorage so a snoozed version
// doesn't pop the toast again next launch; a NEWER version supersedes.

import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

export type UpdateInfo = {
  version: string;
  releaseName: string;
  sizeBytes: number;
  notesMarkdown: string;
  releaseUrl: string;
  downloadUrl: string;
  publishedAt: string;
};

export type UpdateState =
  | "idle"
  | "checking"
  | "available"
  | "launched"
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
  /** Retained for back-compat w/ UI bindings; always 0 in the GH-release path. */
  progress = $state(0);
  dialogOpen = $state(false);
  toastVisible = $state(false);
  dismissedVersion = $state<string | null>(loadDismissed());

  /** True when there's an unsnoozed update waiting for user action. */
  get pillVisible(): boolean {
    return (
      this.state === "available" &&
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

  /** Open the Setup.exe URL in the user's default browser. NSIS wizard
   *  prompts to close Rift if needed, installs, then relaunches. */
  async download() {
    if (this.state !== "available" || !this.info?.downloadUrl) return;
    this.error = "";
    try {
      await openUrl(this.info.downloadUrl);
      this.state = "launched";
    } catch (e) {
      this.error = String(e);
      this.state = "error";
    }
  }

  /** Back-compat alias — UpdateDialog's "Install" button calls this. */
  async applyNow() { await this.download(); }

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

  /** Called once on app launch from AppShell.onMount. Pops the toast if a
   *  newer release exists and the user hasn't snoozed that exact version. */
  async checkOnLaunch() {
    await this.refresh();
    if (
      this.state === "available" &&
      this.info &&
      this.info.version !== this.dismissedVersion
    ) {
      this.toastVisible = true;
    }
  }

  open()  { this.dialogOpen = true; this.toastVisible = false; }
  close() { this.dialogOpen = false; }

  /** No-op in the GH-release path — kept so HMR teardown callers don't error. */
  dispose() {}
}

export const updates = new UpdateStore();

// #173: HMR teardown so a hot-reload doesn't leave stale handlers wired.
if (typeof import.meta !== "undefined" && (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot) {
  (import.meta as { hot: { dispose: (cb: () => void) => void } }).hot.dispose(() => updates.dispose());
}
