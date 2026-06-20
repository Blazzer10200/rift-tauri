type Density = "compact" | "regular" | "comfy";
type CodePrefs = { fontSize: number; tabWidth: number; ligatures: boolean };

const STORAGE_KEY = "rift.ui.density.v1";
const RAIL_PINNED_KEY = "rift.ui.rail-pinned.v1";
const ACCENT_KEY = "rift.ui.accent.v1";
const CODE_KEY = "rift.ui.code.v1";
const FAST_MODE_KEY = "rift.ui.fast-mode.v1";
const DOTFIELD_KEY = "rift.ui.dotfield.v1";
const VIVIDNESS_KEY = "rift.ui.vividness.v1";

// Background texture driving `.app[data-dots]` (variant CSS lives in AppShell).
// "dots" = the default base field (no override); "off" hides it entirely.
export type DotField =
  | "dots" | "dense" | "margins" | "grid" | "lines" | "diagonal" | "crosshatch" | "glow" | "off";
export const DOT_FIELDS: { id: DotField; label: string }[] = [
  { id: "dots", label: "Dots" },
  { id: "dense", label: "Dense" },
  { id: "margins", label: "Margins" },
  { id: "grid", label: "Grid" },
  { id: "lines", label: "Lines" },
  { id: "diagonal", label: "Diagonal" },
  { id: "crosshatch", label: "Crosshatch" },
  { id: "glow", label: "Glow" },
  { id: "off", label: "Off" },
];
const DOT_FIELD_IDS = new Set<string>(DOT_FIELDS.map((d) => d.id));

// Accent chroma range for the vividness dial (drives --accent-c). Default 0.15
// matches app.css; floor stays > 0 so the accent never goes fully grey.
export const VIVIDNESS_MIN = 0.05;
export const VIVIDNESS_MAX = 0.26;
const DEFAULT_VIVIDNESS = 0.15;

// Screen tint — a full-viewport comfort-filter overlay (`.app-tint` in AppShell)
// painted oklch(0.70 0.16 --tint-h) at --tint-a. Strength 0 = filter off.
const TINT_HUE_KEY = "rift.ui.tint-hue.v1";
const TINT_STRENGTH_KEY = "rift.ui.tint-strength.v1";
export const TINT_MAX = 0.12;
const DEFAULT_TINT_STRENGTH = 0.05;
export const TINT_OPTS: { id: string; label: string; hue: number | null }[] = [
  { id: "none", label: "None", hue: null },
  { id: "amber", label: "Amber", hue: 70 },
  { id: "sepia", label: "Sepia", hue: 45 },
  { id: "rose", label: "Rose", hue: 18 },
  { id: "sky", label: "Sky", hue: 232 },
  { id: "mint", label: "Mint", hue: 158 },
];

// 8 curated accent hues — one hue drives the whole accent ramp via --accent-h.
export type AccentSwatch = { id: string; label: string; hue: number };
export const ACCENTS: AccentSwatch[] = [
  { id: "emerald", label: "Emerald", hue: 163 },
  { id: "teal", label: "Teal", hue: 195 },
  { id: "sky", label: "Sky", hue: 230 },
  { id: "violet", label: "Violet", hue: 275 },
  { id: "magenta", label: "Magenta", hue: 328 },
  { id: "rose", label: "Rose", hue: 12 },
  { id: "amber", label: "Amber", hue: 70 },
  { id: "lime", label: "Lime", hue: 130 },
];

const DEFAULT_CODE: CodePrefs = { fontSize: 12, tabWidth: 2, ligatures: false };

class UiPrefs {
  density = $state<Density>("compact");
  railPinned = $state(false);
  accentHue = $state(163);
  vividness = $state(DEFAULT_VIVIDNESS);
  dotField = $state<DotField>("dots");
  tintHue = $state(70);
  tintStrength = $state(0);
  code = $state<CodePrefs>({ ...DEFAULT_CODE });
  // Fast mode = Opus with faster output (CC's `/fast`). TODO: not yet plumbed
  // to the CLI spawn in assistant.svelte.ts — this only persists the intent.
  fastMode = $state(false);

  init() {
    if (typeof window === "undefined") return;
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "compact" || raw === "regular" || raw === "comfy") {
      this.density = raw;
    }
    this.railPinned = localStorage.getItem(RAIL_PINNED_KEY) === "1";

    const accentRaw = localStorage.getItem(ACCENT_KEY);
    if (accentRaw !== null) {
      const hue = Number(accentRaw);
      if (Number.isFinite(hue) && hue >= 0 && hue <= 360) this.accentHue = hue;
    }

    const vivRaw = localStorage.getItem(VIVIDNESS_KEY);
    if (vivRaw !== null) {
      const c = Number(vivRaw);
      if (Number.isFinite(c)) this.vividness = Math.min(VIVIDNESS_MAX, Math.max(VIVIDNESS_MIN, c));
    }

    const dotRaw = localStorage.getItem(DOTFIELD_KEY);
    if (dotRaw !== null && DOT_FIELD_IDS.has(dotRaw)) this.dotField = dotRaw as DotField;

    const tintHueRaw = localStorage.getItem(TINT_HUE_KEY);
    if (tintHueRaw !== null) {
      const h = Number(tintHueRaw);
      if (Number.isFinite(h) && h >= 0 && h <= 360) this.tintHue = h;
    }
    const tintStrRaw = localStorage.getItem(TINT_STRENGTH_KEY);
    if (tintStrRaw !== null) {
      const a = Number(tintStrRaw);
      if (Number.isFinite(a)) this.tintStrength = Math.min(TINT_MAX, Math.max(0, a));
    }

    try {
      const c = JSON.parse(localStorage.getItem(CODE_KEY) ?? "null");
      if (c && typeof c === "object") {
        const rawFs = typeof c.fontSize === "number" && Number.isFinite(c.fontSize) ? c.fontSize : DEFAULT_CODE.fontSize;
        const rawTw = typeof c.tabWidth === "number" && Number.isFinite(c.tabWidth) ? c.tabWidth : DEFAULT_CODE.tabWidth;
        this.code = {
          fontSize:  Math.min(32, Math.max(6, rawFs)),
          tabWidth:  Math.min(8,  Math.max(1, rawTw)),
          ligatures: typeof c.ligatures === "boolean" ? c.ligatures : DEFAULT_CODE.ligatures,
        };
      }
    } catch {
      /* malformed code prefs — fall back to defaults */
    }

    this.fastMode = localStorage.getItem(FAST_MODE_KEY) === "1";

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

  setAccentHue(h: number) {
    this.accentHue = h;
    localStorage.setItem(ACCENT_KEY, String(h));
    this.applyAccent();
  }

  setVividness(c: number) {
    this.vividness = Math.min(VIVIDNESS_MAX, Math.max(VIVIDNESS_MIN, c));
    localStorage.setItem(VIVIDNESS_KEY, String(this.vividness));
    this.applyAccent();
  }

  // dotField drives `.app[data-dots]` via a template binding in AppShell — no
  // DOM write needed here beyond persisting the choice.
  setDotField(d: DotField) {
    this.dotField = d;
    localStorage.setItem(DOTFIELD_KEY, d);
  }

  // Selecting a tint hue (or null = None). Picking a hue turns the filter on at
  // a gentle default if it was off; None forces strength to 0 but keeps the hue.
  setTintHue(h: number | null) {
    if (h === null) { this.setTintStrength(0); return; }
    this.tintHue = h;
    localStorage.setItem(TINT_HUE_KEY, String(h));
    if (this.tintStrength === 0) this.setTintStrength(DEFAULT_TINT_STRENGTH);
    else this.applyTint();
  }

  setTintStrength(a: number) {
    this.tintStrength = Math.min(TINT_MAX, Math.max(0, a));
    localStorage.setItem(TINT_STRENGTH_KEY, String(this.tintStrength));
    this.applyTint();
  }

  setCode(patch: Partial<CodePrefs>) {
    this.code = { ...this.code, ...patch };
    localStorage.setItem(CODE_KEY, JSON.stringify(this.code));
    this.applyCode();
  }

  toggleFastMode() {
    this.fastMode = !this.fastMode;
    localStorage.setItem(FAST_MODE_KEY, this.fastMode ? "1" : "0");
  }


  private apply() {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.density = this.density;
    this.applyRail();
    this.applyAccent();
    this.applyCode();
    this.applyTint();
  }

  private applyTint() {
    if (typeof document === "undefined") return;
    const r = document.documentElement;
    r.style.setProperty("--tint-h", String(this.tintHue));
    r.style.setProperty("--tint-a", String(this.tintStrength));
  }

  private applyRail() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--rail-w", this.railPinned ? "220px" : "48px");
    }
  }

  private applyAccent() {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--accent-h", String(this.accentHue));
      document.documentElement.style.setProperty("--accent-c", String(this.vividness));
    }
  }

  private applyCode() {
    if (typeof document === "undefined") return;
    const r = document.documentElement;
    r.style.setProperty("--code-fs", `${this.code.fontSize}px`);
    r.style.setProperty("--code-tab", String(this.code.tabWidth));
    r.style.setProperty("--code-liga", this.code.ligatures ? "normal" : "none");
  }
}

export const uiPrefs = new UiPrefs();
