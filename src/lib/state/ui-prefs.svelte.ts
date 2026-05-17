type Density = "compact" | "regular" | "comfy";

const STORAGE_KEY = "rift.ui.density.v1";
const RAIL_PINNED_KEY = "rift.ui.rail-pinned.v1";
const V03_SHELL_KEY = "rift.ui.v03-shell.v1";

class UiPrefs {
  density = $state<Density>("compact");
  railPinned = $state(false);
  /** Storage key kept as `v03-shell` for upgrade compat — v0.4.1 reuses the
   *  same toggle to switch between the v0.2 page-tab shell and the new
   *  chat-first / right-pane shell. Rename of the storage slot would
   *  silently flip existing users back to v0.2 on first launch. */
  useV03Shell = $state(false);

  init() {
    if (typeof window === "undefined") return;
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "compact" || raw === "regular" || raw === "comfy") {
      this.density = raw;
    }
    this.railPinned = localStorage.getItem(RAIL_PINNED_KEY) === "1";
    this.useV03Shell = localStorage.getItem(V03_SHELL_KEY) === "1";
    this.apply();
  }

  setDensity(d: Density) {
    this.density = d;
    localStorage.setItem(STORAGE_KEY, d);
    this.apply();
  }

  toggleRailPinned() {
    this.railPinned = !this.railPinned;
    localStorage.setItem(RAIL_PINNED_KEY, this.railPinned ? "1" : "0");
    this.applyRail();
  }

  setUseV03Shell(on: boolean) {
    this.useV03Shell = on;
    localStorage.setItem(V03_SHELL_KEY, on ? "1" : "0");
  }

  private apply() {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.density = this.density;
    this.applyRail();
  }

  private applyRail() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--rail-w", this.railPinned ? "220px" : "48px");
    }
  }
}

export const uiPrefs = new UiPrefs();
