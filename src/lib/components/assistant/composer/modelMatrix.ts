// C7 (per docs/design/composer-split.md) — the model / effort / permission-mode
// option tables + their pure derivation helpers, lifted from Composer.svelte
// 2026-06-10 so the parent's onKey navigation and the SettingsMenu/PermMenu
// children derive from one source and can never disagree.
import { Hand, Code2, ClipboardList, Zap, Infinity as InfinityIcon, Gem, Feather, Sparkles, Rabbit } from "lucide-svelte";
import type { ModelSel, PermissionMode, ThinkingEffort } from "../../../state/assistant/types";
import { fableAvailable, MODEL_MAX_EFFORT } from "../../../state/assistant/helpers";

export type EffortOpt = { id: ThinkingEffort; label: string; hint: string };
// Effort ladder mapped to the CLI's `--effort` flag (low/medium/high/xhigh) by
// effortToFlag (helpers.ts): none→low · quick→medium · smart→medium · deep→high
// · ultra→xhigh. "smart" is Rift's default and maps to `medium` — Anthropic's
// recommended responsive default for interactive use (the CLI default is `high`,
// but their guidance is to override it: high makes the model think for a long
// time before any visible text, which reads as a frozen UI). "deep" (high) is
// explicit heavy reasoning; "ultra" rides xhigh + the ultracode workflow key.
// Haiku rejects effort server-side, so the slider hides entirely there.
// Labels track the real CLI `--effort` flag each tier sends (low/medium/high/
// xhigh), per the user's "keep it vanilla" call — no marketing names. Two tiers
// ("quick"/"smart") both send `medium`; "Medium" vs "Medium+" keeps them
// distinguishable on the slider without inventing a flag the CLI doesn't have.
export const EFFORT_OPTIONS: EffortOpt[] = [
  { id: "none",  label: "Low",     hint: "Low — minimal reasoning. Fastest answers for quick lookups and small edits." },
  { id: "quick", label: "Medium",  hint: "Medium — light reasoning with leaner tool use. Good for routine, well-defined tasks." },
  { id: "smart", label: "Medium+", hint: "Medium+ — balanced reasoning + fast responses. The recommended default for everyday work." },
  { id: "deep",  label: "High",    hint: "High — heavier reasoning and more thorough tool use, for complex tasks where quality matters more than speed." },
  { id: "ultra", label: "X-High",  hint: "X-High — deepest reasoning + autonomous multi-agent workflows. Claude orchestrates fleets of subagents for the most exhaustive answer." },
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
  //   • maxEffort  — highest effort tier the model honors. Opus + Fable reach
  //                  "ultra" (xhigh + ultracode); Sonnet 4.6 reaches "deep"
  //                  (high) — it accepts high but not xhigh, so ultracode is
  //                  Opus-tier only.
  effort: boolean;
  maxEffort: ThinkingEffort;
};
// Flat single-column list (Claude-Code-Desktop layout): current models first,
// legacy generations grouped below. `opus` is the alias → newest Opus (4.8,
// 1M-ctx beta); `claude-opus-4-7` pins the prior generation. The CLI takes
// the alias / pinned id; name + suffix are display-only.
// Fable 5 is a limited run — row exists only while fableAvailable() (gated by
// FABLE_DISABLED + the sunset date in helpers.ts); when pulled the list
// collapses back to the standard four.
export const MODEL_OPTIONS: ModelOpt[] = [
  ...(fableAvailable() ? [{ id: "claude-fable-5" as ModelSel, label: "Fable", version: "5", tagline: "Anthropic's most capable model — limited run", blurb: "Most capable — limited run", ctx: "1M ctx", suffix: "1M context", legacy: false, limited: true, effort: true, maxEffort: MODEL_MAX_EFFORT["claude-fable-5"], icon: Sparkles }] : []),
  { id: "opus",            label: "Opus",   version: "4.8", tagline: "Newest + most capable — complex reasoning & agentic coding", blurb: "Deep reasoning & agentic coding", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.opus, icon: Gem },
  { id: "sonnet",          label: "Sonnet", version: "4.6", tagline: "Best speed + intelligence balance — the default",            blurb: "Everyday default — speed + smarts", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.sonnet, icon: Feather },
  { id: "haiku",           label: "Haiku",  version: "4.5", tagline: "Fastest, near-frontier — quick edits & lookups",             blurb: "Fastest — quick edits & lookups", ctx: "200K ctx", suffix: "200K context", legacy: false, effort: false, maxEffort: MODEL_MAX_EFFORT.haiku, icon: Rabbit },
  { id: "claude-opus-4-7", label: "Opus",   version: "4.7", tagline: "Previous-generation Opus — proven for complex reasoning",    blurb: "Previous-generation Opus", ctx: "1M ctx",   suffix: "1M context",   legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-opus-4-7"], icon: Gem },
];
// 1-based number shortcut → model id (digit keys pick directly in the menu).
export const modelShortcut = (id: ModelSel) => MODEL_OPTIONS.findIndex((m) => m.id === id) + 1;

// Current-generation models surface in the main list; previous generations
// (`legacy`) fold into the "More models" flyout — matching the desktop picker.
// Both derive from the one MODEL_OPTIONS table so the number hotkeys (which
// index into MODEL_OPTIONS) and the row split can never disagree.
export const currentModels = MODEL_OPTIONS.filter((m) => !m.legacy);
export const legacyModels = MODEL_OPTIONS.filter((m) => m.legacy);

/** The slider's allowed tiers — EFFORT_OPTIONS truncated at the model's
 *  ceiling. A prefix slice, so an index into it equals the absolute tier index. */
export function effortStopsFor(m: ModelOpt | undefined): EffortOpt[] {
  if (!m?.effort) return [];
  const cap = EFFORT_OPTIONS.findIndex((e) => e.id === m.maxEffort);
  return EFFORT_OPTIONS.slice(0, cap >= 0 ? cap + 1 : EFFORT_OPTIONS.length);
}

/** Clamp an index into `stops` (pure half of setEffortByIdx). */
export function clampEffortIdx(stops: EffortOpt[], i: number): number {
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
