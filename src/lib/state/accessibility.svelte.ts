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
    this.dyslexiaMode = localStorage.getItem(DYSLEXIA_KEY) === "1";
    const f = localStorage.getItem(FONT_KEY);
    if (f === "lexend" || f === "system") this.font = f;
    this.lineHeightBoost = localStorage.getItem(LINE_HEIGHT_KEY) === "1";
    this.warmTint = localStorage.getItem(WARM_TINT_KEY) === "1";
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
      localStorage.setItem(FONT_KEY, "lexend");
      localStorage.setItem(LINE_HEIGHT_KEY, "1");
    }
    localStorage.setItem(DYSLEXIA_KEY, on ? "1" : "0");
    this.apply();
  }

  setFont(f: DyslexicFont) {
    this.font = f;
    localStorage.setItem(FONT_KEY, f);
    this.apply();
  }

  setLineHeightBoost(on: boolean) {
    this.lineHeightBoost = on;
    localStorage.setItem(LINE_HEIGHT_KEY, on ? "1" : "0");
    this.apply();
  }

  setWarmTint(on: boolean) {
    this.warmTint = on;
    localStorage.setItem(WARM_TINT_KEY, on ? "1" : "0");
    this.apply();
  }

  private apply() {
    if (typeof document === "undefined") return;
    const ds = document.documentElement.dataset;
    ds.a11yDyslexia = this.dyslexiaMode ? "on" : "off";
    ds.a11yFont = this.font;
    ds.a11yLineHeight = this.lineHeightBoost ? "on" : "off";
    ds.a11yWarmTint = this.warmTint ? "on" : "off";
  }
}

export const accessibility = new Accessibility();
