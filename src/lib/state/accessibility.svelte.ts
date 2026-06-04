// Accessibility prefs — dyslexia-friendly font / line-height / warm tint, plus
// a system-prompt hint that tells the embedded Claude to interpret typos and
// voice-to-text artifacts charitably. Master toggle flips sensible defaults
// the first time it's enabled; individual dials below let users fine-tune.
//
// All prefs are local (localStorage); the dyslexia-mode flag is forwarded
// per-turn to `assistant_send` so the Rust side can append the addendum.

export type DyslexicFont = "system" | "lexend";

const DYSLEXIA_KEY = "rift.a11y.dyslexia.v1";
const FONT_KEY = "rift.a11y.font.v1";
const LINE_HEIGHT_KEY = "rift.a11y.line-height.v1";
const WARM_TINT_KEY = "rift.a11y.warm-tint.v1";

class Accessibility {
  /** Master switch — also drives the addendum sent with each assistant turn. */
  dyslexiaMode = $state(false);
  font = $state<DyslexicFont>("system");
  /** Increased line-height + letter-spacing inside reading surfaces. */
  lineHeightBoost = $state(false);
  /** Warm sepia tint on message bubbles + composer for less stark contrast. */
  warmTint = $state(false);

  init() {
    if (typeof window === "undefined") return;
    try {
      this.dyslexiaMode = localStorage.getItem(DYSLEXIA_KEY) === "1";
      const f = localStorage.getItem(FONT_KEY);
      if (f === "lexend" || f === "system") this.font = f;
      this.lineHeightBoost = localStorage.getItem(LINE_HEIGHT_KEY) === "1";
      this.warmTint = localStorage.getItem(WARM_TINT_KEY) === "1";
    } catch {
      // localStorage unavailable (private-browsing restriction etc.) — keep defaults
    }
    this.apply();
  }

  /** Master toggle. First-time-on seeds the recommended bundle (font + line
   *  height); subsequent toggles preserve whatever fine-grained dials the
   *  user has tweaked. */
  setDyslexiaMode(on: boolean) {
    const firstTimeOn =
      on && !this.dyslexiaMode && this.font === "system" && !this.lineHeightBoost;
    this.dyslexiaMode = on;
    if (firstTimeOn) {
      this.font = "lexend";
      this.lineHeightBoost = true;
      try { localStorage.setItem(FONT_KEY, "lexend"); } catch { /* ignore */ }
      try { localStorage.setItem(LINE_HEIGHT_KEY, "1"); } catch { /* ignore */ }
    }
    try { localStorage.setItem(DYSLEXIA_KEY, on ? "1" : "0"); } catch { /* ignore */ }
    this.apply();
  }

  setFont(f: DyslexicFont) {
    this.font = f;
    try { localStorage.setItem(FONT_KEY, f); } catch { /* ignore */ }
    this.apply();
  }

  setLineHeightBoost(on: boolean) {
    this.lineHeightBoost = on;
    try { localStorage.setItem(LINE_HEIGHT_KEY, on ? "1" : "0"); } catch { /* ignore */ }
    this.apply();
  }

  setWarmTint(on: boolean) {
    this.warmTint = on;
    try { localStorage.setItem(WARM_TINT_KEY, on ? "1" : "0"); } catch { /* ignore */ }
    this.apply();
  }

  private apply() {
    if (typeof document === "undefined") return;
    const ds = document.documentElement.dataset;
    ds.a11yDyslexia = this.dyslexiaMode ? "on" : "off";
    // Font + line-height dials are GATED by the master toggle so flipping
    // dyslexia mode off snaps the visual back to defaults. Their localStorage
    // values still persist independently so re-enabling the master restores
    // whatever the user last picked.
    ds.a11yFont = this.dyslexiaMode ? this.font : "system";
    ds.a11yLineHeight = this.dyslexiaMode && this.lineHeightBoost ? "on" : "off";
    // Warm tint is independent — some users want it without the dyslexia
    // bundle (just glare reduction), so it ignores the master switch.
    ds.a11yWarmTint = this.warmTint ? "on" : "off";
  }
}

export const accessibility = new Accessibility();
