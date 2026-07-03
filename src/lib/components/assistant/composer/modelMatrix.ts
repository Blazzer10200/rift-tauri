// C7 (per docs/design/composer-split.md) — the model / effort / permission-mode
// option tables + their pure derivation helpers, lifted from Composer.svelte
// 2026-06-10 so the parent's onKey navigation and the SettingsMenu/PermMenu
// children derive from one source and can never disagree.
import { Hand, Code2, ClipboardList, Zap, Infinity as InfinityIcon, Gem, Feather, Sparkles, Rabbit } from "lucide-svelte";
import type { ModelSel, PermissionMode, ThinkingEffort } from "../../../state/assistant/types";
import { fableAvailable, haikuAvailable, MODEL_MAX_EFFORT, ctxWindowForModelId } from "../../../state/assistant/helpers";
import type { ModelSel as ModelSelType } from "../../../state/assistant/types";

export type EffortOpt = { id: ThinkingEffort; label: string; hint: string };
// Effort ladder mapped to the CLI's `--effort` flag (low/medium/high/xhigh) by
// effortToFlag (helpers.ts): none→low · smart→medium · deep→high · ultra→xhigh.
// "smart" is Rift's default and maps to `medium` — Anthropic's recommended
// responsive default for interactive use (the CLI default is `high`, but their
// guidance is to override it: high makes the model think for a long time before
// any visible text, which reads as a frozen UI). "deep" (high) is explicit
// heavy reasoning; "ultra" rides xhigh + the ultracode workflow key. Haiku
// rejects effort server-side, so the slider hides entirely there. Labels track
// the real CLI `--effort` flag each tier sends, per the user's "keep it
// vanilla" call — no marketing names. (The legacy "quick" tier — a second
// medium rung labeled "Medium" beside smart's "Medium+" — was retired
// 2026-07-03; stored "quick" folds into "smart" at loadEffort.)
export const EFFORT_OPTIONS: EffortOpt[] = [
  { id: "none",  label: "Low",    hint: "Low — minimal reasoning. Fastest answers for quick lookups and small edits." },
  { id: "smart", label: "Medium", hint: "Medium — balanced reasoning + fast responses. The recommended default for everyday work." },
  { id: "deep",  label: "High",   hint: "High — heavier reasoning and more thorough tool use, for complex tasks where quality matters more than speed." },
  { id: "ultra", label: "X-High", hint: "X-High — deepest reasoning + autonomous multi-agent workflows. Claude orchestrates fleets of subagents for the most exhaustive answer." },
];

export type ModelOpt = {
  id: ModelSel;
  label: string;
  version: string;
  tagline: string;
  blurb: string;     // short second-line description in the model menu row
  ctx: string;
  suffix: string;    // muted inline tag beside the name (e.g. "1M context")
  legacy: boolean;   // previous-generation — moved into the "More models" flyout
  limited?: boolean; // limited-run — accent name + "until" badge (Fable)
  icon: typeof Gem;  // per-model identity glyph in the row's leading tile
  // ── Capability matrix (source of truth for what each model can actually do).
  // Drives every affordance gate so the panel never offers a mode the model
  // ignores server-side. Grounded in the model capability docs:
  //   • effort     — accepts the CLI `--effort` flag at all. The API rejects
  //                  effort on Haiku 4.5 wholesale, so this is false for Haiku.
  //   • maxEffort  — highest effort tier the model honors. Opus, Fable, and
  //                  Sonnet 5 reach "ultra" (xhigh + ultracode); Sonnet 5
  //                  honors xhigh + max server-side, where Sonnet 4.6 rejected
  //                  xhigh and stopped at deep. Haiku rejects effort entirely.
  effort: boolean;
  maxEffort: ThinkingEffort;
};
// Flat single-column list (Claude-Code-Desktop layout): current models first,
// legacy generations grouped below. `opus` is the alias → newest Opus (4.8,
// 1M-ctx beta); `claude-opus-4-7` pins the prior generation. The CLI takes
// the alias / pinned id; name + suffix are display-only.
// Fable 5 row exists while fableAvailable() (gated by FABLE_DISABLED + the
// sunset date in helpers.ts). Owner call 2026-07-01: kept always-visible even
// while the upstream access gate holds — hard-pull only (set FABLE_DISABLED).
export const MODEL_OPTIONS: ModelOpt[] = [
  ...(fableAvailable() ? [{ id: "claude-fable-5" as ModelSel, label: "Fable", version: "5", tagline: "Anthropic's most capable model — limited run", blurb: "Most capable — limited run", ctx: "1M ctx", suffix: "1M context", legacy: false, limited: true, effort: true, maxEffort: MODEL_MAX_EFFORT["claude-fable-5"], icon: Sparkles }] : []),
  { id: "opus",            label: "Opus",   version: "4.8", tagline: "Newest + most capable — complex reasoning & agentic coding", blurb: "Deep reasoning & agentic coding", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.opus, icon: Gem },
  { id: "sonnet",          label: "Sonnet", version: "5",   tagline: "Best speed + intelligence balance — the default",            blurb: "Everyday default — speed + smarts", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.sonnet, icon: Feather },
  ...(haikuAvailable() ? [{ id: "haiku" as ModelSel, label: "Haiku", version: "4.5", tagline: "Fastest, near-frontier — quick edits & lookups", blurb: "Fastest — quick edits & lookups", ctx: "200K ctx", suffix: "200K context", legacy: false, effort: false, maxEffort: MODEL_MAX_EFFORT.haiku, icon: Rabbit }] : []),
  { id: "claude-opus-4-7", label: "Opus",   version: "4.7", tagline: "Previous-generation Opus — proven for complex reasoning",    blurb: "Previous-generation Opus", ctx: "1M ctx",   suffix: "1M context",   legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-opus-4-7"], icon: Gem },
];
// 1-based number shortcut → model id (digit keys pick directly in the menu).
export const modelShortcut = (id: ModelSel) => MODEL_OPTIONS.findIndex((m) => m.id === id) + 1;

/** The context-window tag shown beside a model in the picker, HONEST under the
 *  user's plan: a 1M-native model capped to 200K by a Free plan must read "200K
 *  context", not the static "1M context" baked into MODEL_OPTIONS. Derives from
 *  the SAME `ctxWindowForModelId` that drives the gauge + compaction pill, so all
 *  three agree. `planCap` is the value from `assistant.planCap`. */
export function modelWindowSuffix(id: ModelSelType, planCap: number): string {
  const w = ctxWindowForModelId(id, planCap);
  return w >= 1_000_000 ? "1M context" : `${Math.round(w / 1000)}K context`;
}

// Current-generation models surface in the main list; previous generations
// (`legacy`) fold into the "More models" flyout — matching the desktop picker.
// Both derive from the one MODEL_OPTIONS table so the number hotkeys (which
// index into MODEL_OPTIONS) and the row split can never disagree.
export const currentModels = MODEL_OPTIONS.filter((m) => !m.legacy);
export const legacyModels = MODEL_OPTIONS.filter((m) => m.legacy);

// ── Reasoning ladder ─────────────────────────────────────────────────────────
// ONE control for reasoning depth. The old separate `Thinking` toggle + effort
// slider rendered two knobs over ONE wire lever: the CLI has no thinking-disable
// flag (`--effort` is the only control), so toggle-OFF sent `--effort low`
// regardless of the slider (the slider was decorative) and toggle-ON sent the
// slider's tier — i.e. the toggle WAS an effort jump, which read as "thinking is
// 5× slower for the same answer". The ladder exposes the real states, one rung
// per distinct CLI `--effort` flag. Rung 0 (`effort: null`) writes
// thinkingEnabled=false — the exact wire state of the old toggle-off (sends
// `--effort low`, rides the no-think shim on the API-key path) — so the fast
// default and the effortToFlag 3-file lockstep are untouched. Rungs 1+ write
// thinkingEnabled=true + their tier atomically (assistant.setThinkingDial).
// (The legacy `quick` tier is coerced to `smart` at loadEffort — a rung here
// never sees it.)
export type DialId = "low" | "medium" | "high" | "xhigh";
export type DialStop = {
  id: DialId;
  label: string;
  hint: string;
  /** null = fastest rung (extended thinking off → wire `--effort low`).
   *  Otherwise the ThinkingEffort tier this rung selects (thinking on). */
  effort: ThinkingEffort | null;
};
// Labels track the real CLI `--effort` flag each rung sends (vanilla flag
// names, no marketing labels — standing owner call).
export const DIAL_STOPS: DialStop[] = [
  { id: "low",    label: "Low",    effort: null,    hint: "Low — replies immediately with minimal reasoning. Fastest for chat, quick lookups, and small edits." },
  { id: "medium", label: "Medium", effort: "smart", hint: "Medium — balanced reasoning at a responsive pace. The recommended default for everyday work." },
  { id: "high",   label: "High",   effort: "deep",  hint: "High — reasons before replying and works more thoroughly, for complex tasks where quality matters more than speed." },
  { id: "xhigh",  label: "X-High", effort: "ultra", hint: "X-High — deepest reasoning + autonomous multi-agent workflows. Claude orchestrates fleets of subagents for the most exhaustive answer." },
];

/** The ladder rungs a model supports. Haiku (no effort capability) gets an
 *  empty list — the panel hides the ladder entirely. Otherwise rungs truncate
 *  at the model's effort ceiling; the Low rung is always present. */
export function dialStopsFor(m: ModelOpt | undefined): DialStop[] {
  if (!m?.effort) return [];
  const capIdx = EFFORT_OPTIONS.findIndex((e) => e.id === m.maxEffort);
  return DIAL_STOPS.filter(
    (s) => s.effort === null
      || (capIdx >= 0 && EFFORT_OPTIONS.findIndex((e) => e.id === s.effort) <= capIdx),
  );
}

/** Project the store pair (thinkingEnabled, thinkingEffort) onto a rung index.
 *  Off → the Low rung whatever tier is parked in storage — that IS the wire
 *  truth (thinking-off sends `--effort low`). On → the tier's rung; a stale
 *  out-of-range tier falls back to Medium. */
export function dialIdxFor(stops: DialStop[], thinkingOn: boolean, effort: ThinkingEffort): number {
  if (!thinkingOn || effort === "none") return 0;
  const i = stops.findIndex((s) => s.effort === effort);
  if (i >= 0) return i;
  const med = stops.findIndex((s) => s.effort === "smart");
  return med >= 0 ? med : 0;
}

/** Clamp an index into a stops array (pure half of setEffortByIdx). Only the
 *  length matters, so it accepts any stop list. */
export function clampEffortIdx(stops: readonly unknown[], i: number): number {
  const max = Math.max(0, stops.length - 1);
  return Math.min(max, Math.max(0, i));
}

// Permission-mode picker options — order matches the VS Code Claude Code menu:
// ask → auto-edit → plan → auto → bypass. Icons echo that menu.
// `label` = full descriptive label (menu rows). `short` = the compact bar-pill
// label (design-system `PERM_SHORT`) so the composer bar stays terse — e.g.
// "Bypass" not "Bypass permissions". Keep both in sync per mode.
export type ModeOpt = { id: PermissionMode; label: string; short: string; icon: typeof Hand; hint: string };
export const MODE_OPTIONS: ModeOpt[] = [
  { id: "default",           label: "Ask before edits",   short: "Ask first", icon: Hand,          hint: "Ask before edits — approve each change before it's made" },
  { id: "acceptEdits",       label: "Edit automatically", short: "Auto-edit", icon: Code2,         hint: "Edit automatically — apply file edits without asking" },
  { id: "plan",              label: "Plan mode",          short: "Plan",      icon: ClipboardList, hint: "Plan mode — explore and present a plan before editing" },
  { id: "auto",              label: "Auto mode",          short: "Auto",      icon: Zap,           hint: "Auto mode — pick the best permission mode per task" },
  { id: "bypassPermissions", label: "Bypass permissions", short: "Bypass",    icon: InfinityIcon,  hint: "Bypass permissions — never ask before running anything" },
];

// Per-mode semantic tone for the spec `.pop-item.tone-*` tinting + the composer
// bar's perm-button tone (ok = safe automation, warn = guardrails off, info =
// non-editing). Single source so the menu, the bar pill, and Composer agree.
export type PermTone = "ok" | "warn" | "info" | "";
export function permToneFor(id: PermissionMode): PermTone {
  if (id === "acceptEdits" || id === "auto") return "ok";
  if (id === "bypassPermissions") return "warn";
  if (id === "plan") return "info";
  return "";
}
