// Global update store. One instance per session: triggers a launch-time check,
// caches the result, drives the sidebar pill + the popup dialog, and lets
// Settings/About + TabRail/Sidebar both reach into the same state.

import { invoke } from "@tauri-apps/api/core";

export type UpdateInfo = { version: string; releaseName: string };
export type UpdateState = "idle" | "checking" | "available" | "uptodate" | "error" | "applying";

class UpdateStore {
  state = $state<UpdateState>("idle");
  info = $state<UpdateInfo | null>(null);
  error = $state("");
  currentVersion = $state("?");
  dialogOpen = $state(false);
  /** Set the first time the launch-popup auto-opens, so re-checks don't re-pop. */
  launchPopupShown = $state(false);

  async refresh() {
    this.state = "checking";
    this.error = "";
    try { this.currentVersion = await invoke<string>("app_version"); } catch {}
    try {
      const res = await invoke<UpdateInfo | null>("check_for_updates");
      if (res) { this.info = res; this.state = "available"; }
      else     { this.info = null; this.state = "uptodate"; }
    } catch (e) {
      this.error = String(e);
      this.state = "error";
    }
  }

  /**
   * Apply the pending update. Stops autosync, downloads, then exits the
   * process to swap binaries — control never returns on success. On error
   * the dialog stays open so the user can retry or close.
   */
  async apply() {
    if (this.state !== "available") return;
    this.state = "applying";
    this.error = "";
    try {
      await invoke<void>("apply_updates");
    } catch (e) {
      this.error = String(e);
      this.state = "error";
    }
  }

  /** Called once on app launch from AppShell.onMount. */
  async checkOnLaunch() {
    await this.refresh();
    if (this.state === "available" && !this.launchPopupShown) {
      this.launchPopupShown = true;
      this.dialogOpen = true;
    }
  }

  open()  { this.dialogOpen = true; }
  close() { this.dialogOpen = false; }
}

export const updates = new UpdateStore();
