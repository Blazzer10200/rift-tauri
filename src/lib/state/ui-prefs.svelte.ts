type Density = "compact" | "regular" | "comfy";

const STORAGE_KEY = "rift.ui.density.v1";

class UiPrefs {
  density = $state<Density>("compact");

  init() {
    if (typeof window === "undefined") return;
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "compact" || raw === "regular" || raw === "comfy") {
      this.density = raw;
    }
    this.apply();
  }

  setDensity(d: Density) {
    this.density = d;
    localStorage.setItem(STORAGE_KEY, d);
    this.apply();
  }

  private apply() {
    if (typeof document !== "undefined") {
      document.documentElement.dataset.density = this.density;
    }
  }
}

export const uiPrefs = new UiPrefs();
