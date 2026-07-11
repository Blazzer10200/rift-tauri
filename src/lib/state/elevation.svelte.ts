// Administrator-elevation state (Windows). Mirrors `crate::elevation` +
// `commands/elevation.rs`. Backs the Settings "Administrator access" card and
// the status-bar Admin badge.
//
// Elevation is inherited by child processes, so once Rift itself runs elevated
// the whole tree — Rift → the Claude CLI → its Bash/PowerShell tools — runs
// elevated with no per-action UAC prompt (the "launch VS Code as admin"
// experience). Two layers: a one-prompt-per-session relaunch, and an opt-in
// "always run elevated" that launches prompt-free via a per-user scheduled task.

import { invoke } from "@tauri-apps/api/core";

type Status = {
  supported: boolean;
  elevated: boolean;
  always_elevated: boolean;
  pref_on: boolean;
};

type ApplyResult = { always_elevated: boolean; relaunching: boolean };

class ElevationStore {
  /** Windows only — controls are hidden elsewhere. */
  supported = $state(false);
  /** This process currently holds an elevated (admin) token. */
  elevated = $state(false);
  /** The prompt-free launcher is fully set up (pref on AND the task exists). */
  alwaysElevated = $state(false);
  /** In-flight command — disables the controls. Stays true through a relaunch. */
  busy = $state(false);
  /** Last error (declined UAC, task failure) — surfaced under the card. */
  error = $state<string | null>(null);
  loaded = $state(false);

  async refresh(): Promise<void> {
    try {
      const s = await invoke<Status>("elevation_status");
      this.supported = s.supported;
      this.elevated = s.elevated;
      this.alwaysElevated = s.always_elevated;
      this.error = null;
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loaded = true;
    }
  }

  /** Relaunch elevated for this session (one UAC prompt). On success the app
   *  exits ~250ms later and only the elevated instance remains. */
  async relaunchAsAdmin(): Promise<void> {
    this.busy = true;
    this.error = null;
    try {
      await invoke("elevation_relaunch_as_admin");
      // Success → the app is tearing down; keep `busy` so controls stay disabled.
    } catch (e) {
      this.error = String(e);
      this.busy = false;
    }
  }

  /** Toggle "always run as administrator". Enabling while non-elevated triggers
   *  a one-time UAC relaunch (then the app exits and comes back elevated). */
  async setAlwaysElevated(enabled: boolean): Promise<void> {
    this.busy = true;
    this.error = null;
    try {
      const r = await invoke<ApplyResult>("elevation_set_always", { enabled });
      this.alwaysElevated = r.always_elevated;
      if (r.relaunching) {
        // App exits shortly to relaunch elevated — leave controls disabled.
        return;
      }
      this.busy = false;
      await this.refresh();
    } catch (e) {
      this.error = String(e);
      this.busy = false;
      await this.refresh();
    }
  }
}

export const elevation = new ElevationStore();
