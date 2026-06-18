// C7 (per docs/design/composer-split.md) — the model / effort / permission-mode
// option tables + their pure derivation helpers, lifted from Composer.svelte
// 2026-06-10 so the parent's onKey navigation and the SettingsMenu/PermMenu
// children derive from one source and can never disagree.
import { Hand, Code2, ClipboardList, Zap, Infinity as InfinityIcon } from "lucide-svelte";
import type { ModelSel, PermissionMode, ThinkingEffort } from "../../../state/assistant/types";
import { fableAvailable, MODEL_MAX_EFFORT } from "../../../state/assistant/helpers";

export type EffortOpt = { id: ThinkingEffort; label: string; hint: string; level: 1 | 2 | 3 | 4 | 5 };
// Effort ladder — 1:1 with the CLI's `--effort` ladder (low/medium/high/xhigh)
// so the slider never lies about what gets sent. "smart" (high) is the API
// default and Rift's default; "deep" (xhigh) is Claude Code's own default for
// agentic coding; "ultra" rides xhigh + the ultracode workflow key. Haiku
// rejects effort server-side, so the slider hides entirely there.
export const EFFORT_OPTIONS: EffortOpt[] = [
  { id: "none",  label: "Instant", level: 1, hint: "Instant — minimal reasoning. Fastest answers for quick lookups and small edits." },
  { id: "quick", label: "Quick",   level: 2, hint: "Quick — light reasoning with leaner tool use. Good for routine, well-defined tasks." },
  { id: "smart", label: "Smart",   level: 3, hint: "Smart — standard reasoning depth, the recommended default for everyday work." },
  { id: "deep",  label: "Deep",    level: 4, hint: "Deep — extra reasoning depth and more thorough tool use. Claude Code's tier for hard agentic coding." },
  { id: "ultra", label: "Ultracode", level: 5, hint: "Ultracode — deep reasoning + autonomous multi-agent workflows. Claude orchestrates fleets of subagents for the most exhaustive answer." },
];

export type ModelOpt = {
  id: ModelSel;
  label: string;
  version: string;
  tagline: string;
  blurb: string;     // short second-line description in the model menu row
  ctx: string;
  suffix: string;    // muted inline tag beside the name (e.g. "1M context")
  legacy: boolean;   // previous-generation — grouped under a "Legacy" subhead
  limited?: boolean; // limited-run — accent name + "until" badge (Fable)
  // ── Capability matrix (source of truth for what each model can actually do).
  // Drives every affordance gate so the panel never offers a mode the model
  // ignores server-side. Grounded in the model capability docs:
  //   • effort     — accepts the CLI `--effort` flag at all. The API rejects
  //                  effort on Haiku 4.5 wholesale, so this is false for Haiku.
  //   • maxEffort  — highest effort tier the model honors. Opus + Fable reach
  //                  "ultra" (xhigh + ultracode); Sonnet 4.6 tops out at
  //                  "smart" (high) — xhigh/ultracode are Opus-tier only.
  //   • fastMode   — Opus-only faster-output mode (CC's `/fast`).
  effort: boolean;
  maxEffort: ThinkingEffort;
  fastMode: boolean;
};
// Flat single-column list (Claude-Code-Desktop layout): current models first,
// legacy generations grouped below. `opus` is the alias → newest Opus (4.8,
// 1M-ctx beta); `claude-opus-4-7` pins the prior generation. The CLI takes
// the alias / pinned id; name + suffix are display-only.
// Fable 5 is a limited run — row exists only while fableAvailable() (through
// Jun 22 2026); after sunset the list collapses back to the standard four.
export const MODEL_OPTIONS: ModelOpt[] = [
  ...(fableAvailable() ? [{ id: "claude-fable-5" as ModelSel, label: "Fable", version: "5", tagline: "Anthropic's most capable model — limited run, retired after Jun 22", blurb: "Most capable — limited run", ctx: "1M ctx", suffix: "1M context", legacy: false, limited: true, effort: true, maxEffort: MODEL_MAX_EFFORT["claude-fable-5"], fastMode: false }] : []),
  { id: "opus",            label: "Opus",   version: "4.8", tagline: "Newest + most capable — complex reasoning & agentic coding", blurb: "Deep reasoning & agentic coding", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.opus, fastMode: true  },
  { id: "sonnet",          label: "Sonnet", version: "4.6", tagline: "Best speed + intelligence balance — the default",            blurb: "Everyday default — speed + smarts", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.sonnet, fastMode: false },
  { id: "haiku",           label: "Haiku",  version: "4.5", tagline: "Fastest, near-frontier — quick edits & lookups",             blurb: "Fastest — quick edits & lookups", ctx: "200K ctx", suffix: "200K context", legacy: false, effort: false, maxEffort: MODEL_MAX_EFFORT.haiku,  fastMode: false },
  { id: "claude-opus-4-7", label: "Opus",   version: "4.7", tagline: "Previous-generation Opus — proven for complex reasoning",    blurb: "Previous-generation Opus", ctx: "1M ctx",   suffix: "1M context",   legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-opus-4-7"], fastMode: true  },
];
// 1-based number shortcut → model id (digit keys pick directly in the menu).
export const modelShortcut = (id: ModelSel) => MODEL_OPTIONS.findIndex((m) => m.id === id) + 1;

// Fast mode is Opus-only (CC's `/fast`). It is also not yet plumbed to the CLI
// spawn (ui-prefs TODO), so the row stays hidden behind FAST_MODE_WIRED until
// the backend honors it — showing a dead toggle would be the exact false
// signal we're removing. Flip to true once wired; it then appears Opus-only.
export const FAST_MODE_WIRED = false;

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
export type ModeOpt = { id: PermissionMode; label: string; icon: typeof Hand; hint: string };
export const MODE_OPTIONS: ModeOpt[] = [
  { id: "default",           label: "Ask before edits", icon: Hand,          hint: "Ask before edits — approve each change before it's made" },
  { id: "acceptEdits",       label: "Edit automatically", icon: Code2,       hint: "Edit automatically — apply file edits without asking" },
  { id: "plan",              label: "Plan mode",        icon: ClipboardList, hint: "Plan mode — explore and present a plan before editing" },
  { id: "auto",              label: "Auto mode",        icon: Zap,           hint: "Auto mode — pick the best permission mode per task" },
  { id: "bypassPermissions", label: "Bypass permissions", icon: InfinityIcon, hint: "Bypass permissions — never ask before running anything" },
];
