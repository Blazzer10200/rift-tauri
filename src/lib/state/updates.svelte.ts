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
// The "an update is available" affordance is a dedicated, stable surface
// (`shell/UpdateBanner.svelte`, gated on `hasUpdate`/`snoozeActive`/`dialogOpen`)
// — NOT a toast. A sticky toast in the shared stack was a moving target: it sat
// at the top of an upward-growing, bottom-anchored, FLIP-animated stack, so every
// other toast that appeared/expired slid it out from under the cursor → ~50/50
// misclicks (the long-standing "update button won't click" bug). The banner is a
// fixed top strip: it never reflows, so the click always lands. Toasts are still
// used for the transient install-FAILURE path, which can't move-target because
// it forces the dialog open at the same time (banner hidden).
//
// Snooze is TIME-BASED (24h), persisted as {version, until} in localStorage.
// It was version-permanent until 2026-06-09: one stray click on the pill's ×
// silenced that version forever with no visible recovery — the user sat on
// v0.8.10 while v0.8.11 was "available" on every launch. Now a snooze expires
// after a day, and a newer version always supersedes it immediately.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import { toast } from "./toast.svelte";
import { humanizeError } from "../utils/humanizeError";
import { assistant } from "./assistant.svelte";

/** Public release repo — used only to synthesize the human "View release on
 *  GitHub" link (Velopack's source doesn't return an html_url). */
const RELEASES_REPO_URL = "https://github.com/Blazzer10200/rift";

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
const SNOOZE_MS = 24 * 60 * 60 * 1000; // 24h — snooze is a delay, never a kill switch

// Bounded backoff for a transient check failure. Without it, an offline-at-launch
// (or a momentary R2 feed blip) lands in "error" and gets no re-check until the
// 6h auto-tick — a user could sit a whole work session unaware a release shipped.
// Mirrors the CLI-update store's retry (v0.20.5).
const REFRESH_RETRY_MS = [30_000, 120_000, 300_000];

type Snooze = { version: string; until: number };

function loadSnooze(): Snooze | null {
  try {
    const raw = localStorage.getItem(DISMISSED_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Snooze;
    // Legacy bare-version strings fail JSON.parse → catch discards them (that
    // format meant "silenced forever", which is exactly the bug being removed).
    if (typeof parsed?.version !== "string" || typeof parsed?.until !== "number") return null;
    if (Date.now() >= parsed.until) { localStorage.removeItem(DISMISSED_KEY); return null; }
    return parsed;
  } catch { return null; }
}
function saveSnooze(v: Snooze | null) {
  try {
    if (v) localStorage.setItem(DISMISSED_KEY, JSON.stringify(v));
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
  /** True while a repair() drives the shared download() chain — lets the
   *  failure toast distinguish "Repair failed" from a normal update failure. */
  private repairing = false;
  snoozed = $state<Snooze | null>(loadSnooze());
  /** Wakes the pill when an active snooze expires mid-session. */
  private snoozeTimer: ReturnType<typeof setTimeout> | null = null;
  /** Error from the most recent `download()`/apply attempt. Distinct from
   *  `error` (feed/check failures) — keeps `state` on "available" so the user
   *  can retry the Download button or grab it manually from GitHub. */
  downloadError = $state("");
  /** Periodic re-check timer — Rift can stay open for days, so launch-only
   *  checking would never surface a release shipped mid-session. */
  private autoTimer: ReturnType<typeof setInterval> | null = null;
  private readonly AUTO_MS = 6 * 60 * 60 * 1000; // every 6h
  /** Backoff state for auto-retrying a failed check before the 6h auto-tick. */
  private refreshRetryTimer: ReturnType<typeof setTimeout> | null = null;
  private refreshRetries = 0;

  /** Static "latest release" page — usable even when a check failed (so we have
   *  no `info.releaseUrl`), e.g. a corrupted install that needs a manual Setup.exe. */
  readonly latestReleaseUrl = `${RELEASES_REPO_URL}/releases/latest`;

  /** The current `error` is a "not properly installed" failure (Velopack can't
   *  locate its manifest). Distinct from a transient feed/network error — the
   *  only fix is a clean reinstall, so the UI must say so instead of "try again". */
  get installBroken(): boolean {
    return /properly installed|reinstall/i.test(this.error);
  }

  /** An update exists and is waiting on user action — true even while snoozed.
   *  Drives the always-visible titlebar dot so a snoozed update is never
   *  invisible. */
  get hasUpdate(): boolean {
    return this.state === "available" && !!this.info;
  }

  /** The current available version is under an unexpired snooze. */
  get snoozeActive(): boolean {
    return (
      !!this.snoozed &&
      !!this.info &&
      this.snoozed.version === this.info.version &&
      Date.now() < this.snoozed.until
    );
  }

  /** UI-drift fix: the ONE derived status summary every passive update
   *  surface renders from (Settings chip, future Home card, …), so a chip
   *  can't claim "up to date" while the pill says an update is available.
   *  `label` is empty while idle — surfaces show just the version instead of
   *  asserting a freshness we haven't checked yet. */
  get summary(): { kind: "ok" | "warn" | "busy" | "danger"; label: string } {
    switch (this.state) {
      case "available":
      case "downloading":
      case "installing":
        // Download/install states still mean "an update exists" to a passive
        // chip — the dialog owns the in-flight progress detail.
        return { kind: "warn", label: `v${this.info?.version ?? "?"} available` };
      case "checking":
        return { kind: "busy", label: "checking…" };
      case "error":
        // Dev binary is never Velopack-installed — "reinstall needed" there is
        // noise, not a broken install. Real packaged installs keep the alarm.
        if (this.installBroken && import.meta.env.DEV) return { kind: "busy", label: "dev build" };
        return { kind: "danger", label: this.installBroken ? "reinstall needed" : "update check failed" };
      case "uptodate":
        return { kind: "ok", label: "up to date" };
      case "idle":
      default:
        return { kind: "busy", label: "" };
    }
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
      this.scheduleRefreshRetry();
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
      this.resetRefreshRetry();
    } catch (e) {
      this.error = String(e);
      this.state = "error";
      this.scheduleRefreshRetry();
    }
  }

  /** Arm the next bounded-backoff re-check after a failed `refresh()`. No-op
   *  once the backoff list is exhausted (the 6h auto-tick takes over) or while a
   *  retry is already pending. */
  private scheduleRefreshRetry() {
    const delay = REFRESH_RETRY_MS[this.refreshRetries];
    if (delay == null || this.refreshRetryTimer != null) return;
    this.refreshRetries++;
    this.refreshRetryTimer = setTimeout(() => {
      this.refreshRetryTimer = null;
      // Don't stomp an in-flight download or an open dialog — mirror autoTick.
      if (this.state === "downloading" || this.state === "installing" || this.dialogOpen) return;
      void this.refresh();
    }, delay);
  }

  /** A check succeeded — clear the backoff so the next failure starts fresh. */
  private resetRefreshRetry() {
    this.refreshRetries = 0;
    if (this.refreshRetryTimer != null) { clearTimeout(this.refreshRetryTimer); this.refreshRetryTimer = null; }
  }

  /** Download the update (with progress) then apply it. Velopack stages the
   *  package, and on apply it schedules Update.exe, exits Rift, installs
   *  silently, and relaunches the new version. On any failure we drop back to
   *  "available" with a `downloadError` so the user can retry or grab it
   *  manually from the GitHub release link. */
  async download() {
    if (this.state !== "available") return;
    // Applying kills every in-flight Claude turn (backend drains + kills session
    // children before relaunch) — warn before committing rather than silently
    // tearing down a live conversation.
    if (assistant.liveTabs.length > 0) {
      const ok = await confirm(
        "An update is ready to install, but you have an active conversation running. Installing will end it now. Update anyway?",
        { title: "Update Rift", kind: "warning", okLabel: "Update anyway", cancelLabel: "Not now" },
      );
      if (!ok) return;
    }
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
        title: this.repairing ? "Repair couldn't finish" : "Update couldn't install",
        detail: humanizeError(e),
        sticky: true,
        action: { label: "Get it on GitHub", onClick: () => void this.openReleasePage() },
      });
    } finally {
      if (unlisten) unlisten();
    }
  }

  /** Back-compat alias — any "Install" caller routes through the same flow. */
  async applyNow() { await this.download(); }

  /** Repair installation — force a fresh full re-download + re-apply of the
   *  LATEST release (even when already on it), overwriting corrupted/half-written
   *  binaries. Arms the pending plan to the latest full release on the backend,
   *  then reuses the normal download → apply chain (app exits + relaunches).
   *  Drives the same dialog progress UI as a normal update. */
  async repair() {
    // RR8: include 'checking' — a concurrent check in flight + repair would
    // race two arm operations on the backend pending plan. Latent today (all
    // callers gate via the dialog) but cheap to make correct.
    if (this.state === "downloading" || this.state === "installing" || this.state === "checking") return;
    this.downloadError = "";
    this.error = "";
    try {
      const res = await invoke<UpdateInfoDto>("repair_install");
      // arm_repair set the pending plan; mirror an "available" state so the
      // shared download() guard + dialog progress render correctly.
      this.info = { ...res, releaseUrl: `${RELEASES_REPO_URL}/releases/tag/v${res.version}` };
      this.state = "available";
      this.dialogOpen = true;
      this.repairing = true;
      try {
        await this.download();
      } finally {
        this.repairing = false;
      }
    } catch (e) {
      this.state = "error";
      this.downloadError = String(e);
      this.dialogOpen = true;
      toast.push({
        severity: "danger",
        title: "Repair couldn't start",
        detail: humanizeError(e),
        sticky: true,
        action: { label: "Get it on GitHub", onClick: () => void this.openLatestRelease() },
      });
    }
  }

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
      toast.push({ severity: "danger", title: "Couldn't open the release page", detail: humanizeError(e) });
    }
  }

  /** Open the GitHub "latest release" page — the manual recovery path when the
   *  in-app updater is dead (corrupted install). Always a real https URL. */
  async openLatestRelease() {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(this.latestReleaseUrl);
    } catch (e) {
      toast.push({ severity: "danger", title: "Couldn't open the releases page", detail: humanizeError(e) });
    }
  }

  /** Snooze the current available version for 24h — the pill goes quiet, the
   *  titlebar dot stays, and the pill returns on expiry (or sooner if a newer
   *  version ships). NEVER permanent. Closes the dialog if open. */
  snooze() {
    if (this.info?.version) {
      const s: Snooze = { version: this.info.version, until: Date.now() + SNOOZE_MS };
      this.snoozed = s;
      saveSnooze(s);
      this.armSnoozeTimer(SNOOZE_MS);
    }
    this.dialogOpen = false;
  }

  /** Clear any snooze immediately (titlebar dot click → show everything now). */
  unsnooze() {
    this.snoozed = null;
    saveSnooze(null);
    if (this.snoozeTimer != null) { clearTimeout(this.snoozeTimer); this.snoozeTimer = null; }
  }

  private armSnoozeTimer(ms: number) {
    if (this.snoozeTimer != null) clearTimeout(this.snoozeTimer);
    this.snoozeTimer = setTimeout(() => {
      this.snoozeTimer = null;
      this.snoozed = null; // state write → banner gate recomputes → banner returns
      saveSnooze(null);
    }, ms);
  }

  /** Called once on app launch from AppShell.onMount. The banner surfaces itself
   *  reactively via the `hasUpdate` gate once `refresh()` resolves — no imperative
   *  notification needed. */
  async checkOnLaunch() {
    // A snooze restored from a previous launch still expires on time.
    if (this.snoozed) this.armSnoozeTimer(Math.max(0, this.snoozed.until - Date.now()));
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

  /** Clear timers (HMR teardown / app teardown). */
  dispose() {
    if (this.autoTimer != null) {
      clearInterval(this.autoTimer);
      this.autoTimer = null;
    }
    if (this.snoozeTimer != null) {
      clearTimeout(this.snoozeTimer);
      this.snoozeTimer = null;
    }
    if (this.refreshRetryTimer != null) {
      clearTimeout(this.refreshRetryTimer);
      this.refreshRetryTimer = null;
    }
  }
}

export const updates = new UpdateStore();

// #173: HMR teardown so a hot-reload doesn't leave stale handlers wired.
const _hmrHot = (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot;
if (_hmrHot) _hmrHot.dispose(() => updates.dispose());
