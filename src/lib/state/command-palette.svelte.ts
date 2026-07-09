// Global command palette state. Ctrl+P / Ctrl+K opens; Esc / outside-click
// closes. `targetSettingsSection` is the deep-link channel — Settings page
// watches it and pulls to the requested section, then clears.

import type { WorkspaceId } from "./workspace.svelte";

// "chat" = conversation display prefs; "claude" = session/plan/API-key tab.
type Section = "appearance" | "chat" | "claude" | "speech" | "about";

class CommandPaletteState {
  open = $state(false);
  targetSettingsSection = $state<Section | null>(null);
  /** Bumped whenever the palette opens — components watch this if they need
   *  to refresh their item list (e.g. recent conversations). */
  openTick = $state(0);

  show() {
    this.open = true;
    this.openTick += 1;
  }
  hide() {
    this.open = false;
  }
  toggle() {
    if (this.open) this.hide();
    else this.show();
  }
  requestSettingsSection(s: Section) {
    this.targetSettingsSection = s;
  }
  clearSettingsSection() {
    this.targetSettingsSection = null;
  }
}

export const commandPalette = new CommandPaletteState();
export type { Section as SettingsSection };
