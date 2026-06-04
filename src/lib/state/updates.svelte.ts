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
import { openUrl, openPath } from "@tauri-apps/plugin-opener";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Sparkles } from "lucide-svelte";
import { toast } from "./toast.svelte";

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
  | "downloading"
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
  /** Error from the most recent `download()` attempt. Distinct from
   *  `error` (which is feed/check failures only) — keeps `state` on
   *  "available" so the user can retry the Download button. */
  downloadError = $state("");
  /** Active toast id when an update notification is on screen. */
  private toastId: number | null = null;

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

  /** Download the installer in-app (with progress), then launch it via the
   *  opener plugin — NSIS closes Rift, installs, and relaunches. Falls back to
   *  opening the URL in the browser (the prior v0.4.36 behavior) on any
   *  download/launch failure, so this never regresses. */
  async download() {
    if (this.state !== "available" || !this.info?.downloadUrl) return;
    const url = this.info.downloadUrl;
    this.downloadError = "";
    this.progress = 0;
    this.state = "downloading";
    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<{ downloaded: number; total: number }>(
        "update://download-progress",
        (e) => {
          const { downloaded, total } = e.payload;
          this.progress =
            total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
        },
      );
      const path = await invoke<string>("download_update", { url });
      await openPath(path);
      this.state = "launched";
    } catch (e) {
      // In-app path failed — fall back to the proven browser handoff.
      try {
        await openUrl(url);
        this.state = "launched";
      } catch (e2) {
        this.state = "available";
        this.downloadError = String(e2 ?? e);
      }
    } finally {
      if (unlisten) unlisten();
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
    this.clearToast();
    this.dialogOpen = false;
  }

  dismissToast() { this.clearToast(); }

  private clearToast() {
    if (this.toastId != null) {
      const id = this.toastId;
      this.toastId = null;
      toast.dismiss(id, /* callHandler */ false);
    }
    this.toastVisible = false;
  }

  private showToast() {
    if (!this.info) return;
    if (this.toastId != null) return;
    this.toastVisible = true;
    const target = this.info.version;
    const cur = this.currentVersion;
    const size = this.sizeLabel ? ` · ${this.sizeLabel}` : "";
    this.toastId = toast.push({
      severity: "info",
      icon: Sparkles,
      title: "Update available",
      detail: `v${cur} → v${target}${size}`,
      mono: true,
      sticky: true,
      action: { label: "View", onClick: () => this.open() },
      onDismiss: () => {
        // Close button → snooze this version (matches prior UpdateToast × behavior).
        this.toastId = null;
        this.toastVisible = false;
        this.snooze();
      },
    });
  }

  /** Called once on app launch from AppShell.onMount. Pops the toast if a
   *  newer release exists and the user hasn't snoozed that exact version. */
  async checkOnLaunch() {
    await this.refresh();
    if (
      this.state === "available" &&
      this.info &&
      this.info.version !== this.dismissedVersion
    ) {
      this.showToast();
    }
  }

  open()  { this.dialogOpen = true; this.clearToast(); }
  close() { this.dialogOpen = false; }

  /** No-op in the GH-release path — kept so HMR teardown callers don't error. */
  dispose() {}
}

export const updates = new UpdateStore();

// #173: HMR teardown so a hot-reload doesn't leave stale handlers wired.
const _hmrHot = (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot;
if (_hmrHot) _hmrHot.dispose(() => updates.dispose());
