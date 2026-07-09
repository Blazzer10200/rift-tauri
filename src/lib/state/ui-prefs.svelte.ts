// (screen-tint comfort filter removed 2026-06-20 — overlapped all surfaces)
type Density = "compact" | "regular" | "comfy";
type CodePrefs = { fontSize: number; tabWidth: number; ligatures: boolean };

const STORAGE_KEY = "rift.ui.density.v1";
const RAIL_PINNED_KEY = "rift.ui.rail-pinned.v1";
const ACCENT_KEY = "rift.ui.accent.v1";
const CODE_KEY = "rift.ui.code.v1";
const STREAM_MODE_KEY = "rift.ui.stream-mode.v1";
const NARRATION_KEY = "rift.ui.narration.v1";
const COMMAND_OUTPUT_KEY = "rift.ui.command-output.v1";
const TOOL_DETAIL_KEY = "rift.ui.tool-detail.v1";
const DOTFIELD_KEY = "rift.ui.dotfield.v1";
const VIVIDNESS_KEY = "rift.ui.vividness.v1";

// How much of the model's between-tool narration to surface in the live stream.
//  - "focused": hide pure connective filler ("Now I'll build:") — the work rows
//    already name the target; keeps only substantive reasoning.
//  - "balanced" (default): keep every narration line but DEMOTE short connective
//    beats to a muted inline note hugging the work rows, so the turn reads as
//    work-with-commentary, not chat-between-tools.
//  - "chatty": every narration line as a full prose block (the original behavior).
export type Narration = "focused" | "balanced" | "chatty";
const NARRATION_IDS = new Set<string>(["focused", "balanced", "chatty"]);

// How much of a shell command's output (stdout/stderr + exit) to surface in the
// live stream. The output already rides the tool block; this only governs render.
//  - "minimal": command line only, no output body (the original behavior).
//  - "peek" (default): exit status + a few trailing lines, click to expand the
//    full output — calm stream, detail one click away.
//  - "full": stream the whole stdout/stderr in a terminal body as it runs
//    (VS Code-style in-and-out), exit code on finish.
export type CommandOutput = "minimal" | "peek" | "full";
const COMMAND_OUTPUT_IDS = new Set<string>(["minimal", "peek", "full"]);

// How much of each tool/file ACTION (Read/Grep/Edit/…) shows in the work stream.
// Orthogonal to `narration` (the model's prose) and `commandOutput` (shell body):
// this governs the WorkLine rows themselves.
//  - "minimal": rows collapse to a single named outcome line; a chevron still
//    lets you expand on demand (clean, but never trapped).
//  - "balanced" (default): named rows on the collapsed header ("Read a.ts ·
//    Searched \"foo\""), click to expand the per-tool list.
//  - "detailed": rows auto-expand with full file paths, and force full shell
//    output regardless of the separate commandOutput pref (the VS Code-ish look).
export type ToolDetail = "minimal" | "balanced" | "detailed";
const TOOL_DETAIL_IDS = new Set<string>(["minimal", "balanced", "detailed"]);
export const TOOL_DETAILS: { id: ToolDetail; label: string }[] = [
  { id: "minimal", label: "Minimal" },
  { id: "balanced", label: "Balanced" },
  { id: "detailed", label: "Detailed" },
];

// Named density presets set all three stream knobs at once (a starting point;
// each axis stays independently tweakable afterward). Preset is apply-only — we
// don't persist "which preset" because any later single-axis tweak would make a
// stored label lie; the settings UI DERIVES the active preset from the live triple.
export type DensityPreset = "calm" | "standard" | "verbose";
export const DENSITY_PRESETS: { id: DensityPreset; label: string }[] = [
  { id: "calm", label: "Calm" },
  { id: "standard", label: "Standard" },
  { id: "verbose", label: "Verbose" },
];
const PRESET_MAP: Record<DensityPreset, { toolDetail: ToolDetail; narration: Narration; commandOutput: CommandOutput }> = {
  calm:     { toolDetail: "minimal",  narration: "focused",  commandOutput: "minimal" },
  standard: { toolDetail: "balanced", narration: "balanced", commandOutput: "peek" },
  verbose:  { toolDetail: "detailed", narration: "chatty",   commandOutput: "full" },
};

// Background texture driving `.app[data-dots]` (variant CSS lives in AppShell).
// "dots" = the default base field (no override); "off" hides it entirely.
export type DotField =
  | "dots" | "dense" | "margins" | "grid" | "blueprint" | "lines" | "diagonal" | "crosshatch"
  | "rings" | "glow" | "grain" | "off";
export const DOT_FIELDS: { id: DotField; label: string }[] = [
  { id: "dots", label: "Dots" },
  { id: "dense", label: "Dense" },
  { id: "margins", label: "Margins" },
  { id: "grid", label: "Grid" },
  { id: "blueprint", label: "Blueprint" },
  { id: "lines", label: "Lines" },
  { id: "diagonal", label: "Diagonal" },
  { id: "crosshatch", label: "Crosshatch" },
  { id: "rings", label: "Rings" },
  { id: "glow", label: "Glow" },
  { id: "grain", label: "Grain" },
  { id: "off", label: "Off" },
];
const DOT_FIELD_IDS = new Set<string>(DOT_FIELDS.map((d) => d.id));

// Accent chroma range for the vividness dial (drives --accent-c). Default 0.15
// matches app.css; floor stays > 0 so the accent never goes fully grey.
export const VIVIDNESS_MIN = 0.05;
export const VIVIDNESS_MAX = 0.26;
const DEFAULT_VIVIDNESS = 0.15;

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
  // Transient hover-preview from the settings texture picker — overrides the
  // rendered field while non-null, never persisted.
  previewField = $state<DotField | null>(null);
  code = $state<CodePrefs>({ ...DEFAULT_CODE });
  // Stream mode = the redesigned boxless turn render (the spec's default tool
  // display, redesign-port.md §"Net-new"). Default ON; only an explicit opt-out
  // ("0") falls back to the legacy MessageBubble path.
  streamMode = $state(true);
  // Live-stream narration density (see Narration type). Default "balanced".
  narration = $state<Narration>("balanced");
  // How much shell command output to render in the live stream (see
  // CommandOutput type). Default "peek".
  commandOutput = $state<CommandOutput>("peek");
  // How much per-tool action detail to render (see ToolDetail). Default "balanced".
  toolDetail = $state<ToolDetail>("balanced");

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

    this.streamMode = localStorage.getItem(STREAM_MODE_KEY) !== "0";

    const narrRaw = localStorage.getItem(NARRATION_KEY);
    if (narrRaw !== null && NARRATION_IDS.has(narrRaw)) this.narration = narrRaw as Narration;

    const cmdOutRaw = localStorage.getItem(COMMAND_OUTPUT_KEY);
    if (cmdOutRaw !== null && COMMAND_OUTPUT_IDS.has(cmdOutRaw)) this.commandOutput = cmdOutRaw as CommandOutput;

    const toolDetailRaw = localStorage.getItem(TOOL_DETAIL_KEY);
    if (toolDetailRaw !== null && TOOL_DETAIL_IDS.has(toolDetailRaw)) this.toolDetail = toolDetailRaw as ToolDetail;

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

  /** Back to the stock emerald accent (hue + vividness only — texture/density untouched). */
  resetAccent() {
    this.setAccentHue(163);
    this.setVividness(DEFAULT_VIVIDNESS);
  }

  /** Appearance tab → factory defaults: accent, texture, density, code rendering. */
  resetAppearance() {
    this.resetAccent();
    this.setDotField("dots");
    this.setDensity("compact");
    this.setCode({ ...DEFAULT_CODE });
  }

  /** Chat-rendering knobs → factory defaults: stream view on, Standard dial triple. */
  resetChatRendering() {
    if (!this.streamMode) this.toggleStreamMode();
    this.setToolDetail("balanced");
    this.setNarration("balanced");
    this.setCommandOutput("peek");
  }

  // dotField drives `.app[data-dots]` via a template binding in AppShell — no
  // DOM write needed here beyond persisting the choice.
  setDotField(d: DotField) {
    this.dotField = d;
    localStorage.setItem(DOTFIELD_KEY, d);
  }

  /** Live try-before-you-buy from the picker; null = show the committed field. */
  setPreviewField(d: DotField | null) {
    this.previewField = d;
  }

  setCode(patch: Partial<CodePrefs>) {
    this.code = { ...this.code, ...patch };
    localStorage.setItem(CODE_KEY, JSON.stringify(this.code));
    this.applyCode();
  }

  toggleStreamMode() {
    this.streamMode = !this.streamMode;
    localStorage.setItem(STREAM_MODE_KEY, this.streamMode ? "1" : "0");
  }

  setNarration(n: Narration) {
    this.narration = n;
    localStorage.setItem(NARRATION_KEY, n);
  }

  setCommandOutput(c: CommandOutput) {
    this.commandOutput = c;
    localStorage.setItem(COMMAND_OUTPUT_KEY, c);
  }

  setToolDetail(t: ToolDetail) {
    this.toolDetail = t;
    localStorage.setItem(TOOL_DETAIL_KEY, t);
  }

  // Apply a named preset = set all three stream knobs together. One-shot: we
  // don't remember "which preset" — `activePreset` re-derives it from the live
  // triple, so a later single-axis tweak simply un-highlights the preset.
  applyPreset(p: DensityPreset) {
    const { toolDetail, narration, commandOutput } = PRESET_MAP[p];
    this.setToolDetail(toolDetail);
    this.setNarration(narration);
    this.setCommandOutput(commandOutput);
  }

  // Which preset (if any) the current axis triple exactly matches — for the
  // settings UI to highlight. null once the user drifts off a preset.
  get activePreset(): DensityPreset | null {
    for (const { id } of DENSITY_PRESETS) {
      const m = PRESET_MAP[id];
      if (m.toolDetail === this.toolDetail && m.narration === this.narration && m.commandOutput === this.commandOutput) {
        return id;
      }
    }
    return null;
  }


  private apply() {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.density = this.density;
    this.applyRail();
    this.applyAccent();
    this.applyCode();
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
