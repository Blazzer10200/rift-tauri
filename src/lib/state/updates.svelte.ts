// Update store — Velopack path (v0.4.47+, restored 2026-06-04).
//
// Backend (`update_service.rs` + `commands/update.rs`) wraps Velopack's
// UpdateManager over the native GithubSource. Flow:
//   checkOnLaunch → check_for_updates → (available) → download() →
//   download_update [streams `update-progress`] → apply_pending_update
//   [Velopack swaps files on exit + relaunches the new version].
//
// "One-click, then unattended": the user clicks Download once; we download
// with progress, then immediately apply — the app exits and Velopack's
// Update.exe installs silently and relaunches. No second prompt.
//
// State machine:
//   idle → checking → available → downloading → installing
//                  ↘ uptodate                ↘ (error → back to available)
//                  ↘ error
//
// The "an update is available" affordance is a dedicated, stable pill
// (`UpdatePill.svelte`, driven by `pillVisible`) — NOT a toast. A sticky toast
// in the shared stack was a moving target: it sat at the top of an
// upward-growing, bottom-anchored, FLIP-animated stack, so every other toast
// that appeared/expired slid it out from under the cursor → ~50/50 misclicks
// (the long-standing "update button won't click" bug). The pill is a singleton
// fixed element: it never reflows, so the click always lands. Toasts are still
// used for the transient install-FAILURE path, which can't move-target because
// it forces the dialog open at the same time (pill hidden).
//
// `dismissedVersion` is persisted in localStorage so a snoozed version doesn't
// pop the pill again next launch; a NEWER version supersedes.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "./toast.svelte";

/** Public release repo — used only to synthesize the human "View release on
 *  GitHub" link (Velopack's source doesn't return an html_url). */
const RELEASES_REPO_URL = "https://github.com/Blazzer10200/rift-releases";

/** What `check_for_updates` returns from the backend (UpdateInfoDto). */
type UpdateInfoDto = {
  version: string;
  releaseName: string;
  sizeBytes: number;
  notesMarkdown: string;
};

export type UpdateInfo = UpdateInfoDto & {
  /** Synthesized client-side from the version tag — the canonical release page. */
  releaseUrl: string;
};

export type UpdateState =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "installing"
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
  /** Download progress 0..100, streamed from Velopack via `update-progress`. */
  progress = $state(0);
  dialogOpen = $state(false);
  dismissedVersion = $state<string | null>(loadDismissed());
  /** Error from the most recent `download()`/apply attempt. Distinct from
   *  `error` (feed/check failures) — keeps `state` on "available" so the user
   *  can retry the Download button or grab it manually from GitHub. */
  downloadError = $state("");
  /** Periodic re-check timer — Rift can stay open for days, so launch-only
   *  checking would never surface a release shipped mid-session. */
  private autoTimer: ReturnType<typeof setInterval> | null = null;
  private readonly AUTO_MS = 6 * 60 * 60 * 1000; // every 6h

  /** Static "latest release" page — usable even when a check failed (so we have
   *  no `info.releaseUrl`), e.g. a corrupted install that needs a manual Setup.exe. */
  readonly latestReleaseUrl = `${RELEASES_REPO_URL}/releases/latest`;

  /** The current `error` is a "not properly installed" failure (Velopack can't
   *  locate its manifest). Distinct from a transient feed/network error — the
   *  only fix is a clean reinstall, so the UI must say so instead of "try again". */
  get installBroken(): boolean {
    return /properly installed|reinstall/i.test(this.error);
  }

  /** True when there's an unsnoozed update waiting for user action — drives the
   *  stable update pill. Hidden once the dialog is open (the dialog IS the
   *  detail view) or the user has snoozed this exact version. */
  get pillVisible(): boolean {
    return (
      this.state === "available" &&
      !!this.info &&
      this.info.version !== this.dismissedVersion &&
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

  /** Velopack's feed carries no publish date — kept for UI back-compat (the
   *  dialog guards with `{#if}`), always empty. */
  get publishedLabel(): string {
    return "";
  }

  async refresh() {
    this.state = "checking";
    this.error = "";
    try {
      this.currentVersion = await invoke<string>("app_version");
    } catch (e) {
      this.error = String(e);
      this.state = "error";
      return;
    }
    try {
      const res = await invoke<UpdateInfoDto | null>("check_for_updates");
      if (res) {
        this.info = {
          ...res,
          releaseUrl: `${RELEASES_REPO_URL}/releases/tag/v${res.version}`,
        };
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

  /** Download the update (with progress) then apply it. Velopack stages the
   *  package, and on apply it schedules Update.exe, exits Rift, installs
   *  silently, and relaunches the new version. On any failure we drop back to
   *  "available" with a `downloadError` so the user can retry or grab it
   *  manually from the GitHub release link. */
  async download() {
    if (this.state !== "available") return;
    this.downloadError = "";
    this.progress = 0;
    this.state = "downloading";
    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<number>("update-progress", (e) => {
        this.progress = Math.min(100, Math.max(0, e.payload));
      });
      await invoke("download_update");
      // Download complete — apply. This exits the app, so the invoke below
      // never resolves on success; Velopack relaunches the new version.
      this.state = "installing";
      this.progress = 100;
      await invoke("apply_pending_update");
    } catch (e) {
      this.state = "available";
      this.downloadError = String(e);
      // Never let an update failure look like "nothing happened": force the
      // dialog open so the error card is visible, AND raise a sticky toast with
      // the always-available manual fallback (grab it from GitHub directly).
      this.dialogOpen = true;
      toast.push({
        severity: "danger",
        title: "Update couldn't install",
        detail: String(e),
        sticky: true,
        action: { label: "Get it on GitHub", onClick: () => void this.openReleasePage() },
      });
    } finally {
      if (unlisten) unlisten();
    }
  }

  /** Back-compat alias — any "Install" caller routes through the same flow. */
  async applyNow() { await this.download(); }

  /** Open the GitHub release page in the OS browser — the always-available
   *  manual fallback when the in-app Velopack path fails on a given machine.
   *  Only ever hands a real https URL to the opener (F47). */
  async openReleasePage() {
    const url = this.info?.releaseUrl;
    if (!url || !/^https:\/\//i.test(url)) return;
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch (e) {
      toast.push({ severity: "danger", title: "Couldn't open the release page", detail: String(e) });
    }
  }

  /** Open the GitHub "latest release" page — the manual recovery path when the
   *  in-app updater is dead (corrupted install). Always a real https URL. */
  async openLatestRelease() {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(this.latestReleaseUrl);
    } catch (e) {
      toast.push({ severity: "danger", title: "Couldn't open the releases page", detail: String(e) });
    }
  }

  /** Snooze the current available version — the pill stays quiet until a newer
   *  version ships. Closes the dialog if open. */
  snooze() {
    if (this.info?.version) {
      this.dismissedVersion = this.info.version;
      saveDismissed(this.info.version);
    }
    this.dialogOpen = false;
  }

  /** Called once on app launch from AppShell.onMount. The pill surfaces itself
   *  reactively via `pillVisible` once `refresh()` resolves — no imperative
   *  notification needed. */
  async checkOnLaunch() {
    await this.refresh();
    this.startAutoCheck();
  }

  private startAutoCheck() {
    if (this.autoTimer != null) return;
    this.autoTimer = setInterval(() => void this.autoTick(), this.AUTO_MS);
  }

  /** Background re-check — never disrupts an in-flight download or open dialog. */
  private async autoTick() {
    if (this.state === "downloading" || this.state === "installing" || this.dialogOpen) return;
    await this.refresh();
  }

  open()  { this.dialogOpen = true; }
  close() { this.dialogOpen = false; }

  /** Clear the periodic re-check timer (HMR teardown / app teardown). */
  dispose() {
    if (this.autoTimer != null) {
      clearInterval(this.autoTimer);
      this.autoTimer = null;
    }
  }
}

export const updates = new UpdateStore();

// #173: HMR teardown so a hot-reload doesn't leave stale handlers wired.
const _hmrHot = (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot;
if (_hmrHot) _hmrHot.dispose(() => updates.dispose());
