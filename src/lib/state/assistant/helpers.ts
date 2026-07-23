// M1 (per docs/design/assistant-svelte-split.md) — pure helper fns lifted out
// of `src/lib/state/assistant.svelte.ts`. Zero state, zero IPC; only
// localStorage prefs + pure transforms. Safe to import anywhere.

import type { ModelFamily, ModelSel, PermissionMode, RiftPlan, ThinkingEffort } from "./types";

const MODEL_SELS: readonly ModelSel[] = [
  "sonnet", "opus", "claude-opus-4-7", "haiku", "claude-fable-5",
  "claude-opus-4-6", "claude-opus-4-5", "claude-sonnet-4-6", "claude-sonnet-4-5",
] as const;

// Claude Fable 5 — owner call 2026-07-01: keep it ALWAYS VISIBLE (flag `false`)
// even while the upstream Fable/Mythos access gate is up, so the picker row is
// live and rolls the instant access returns. Tradeoff accepted: while the gate
// holds, a Fable turn returns "Claude Fable 5 is currently unavailable" (the
// Fable/Mythos access gate), which Rift renders gracefully — EXPECTED, not a bug
// to re-gate. Sunset stays far out; only the kill-switch below hides the row.
export const FABLE_SUNSET_MS = Date.UTC(2099, 0, 1);
// Manual kill-switch — true pulls Fable: hides the picker row, coerces any
// stored/selected Fable pref to the default, and the backend swaps a pinned
// Fable session → opus before it can hit the API. Mirror in config.rs.
export const FABLE_DISABLED = false;
export function fableAvailable(): boolean {
  return !FABLE_DISABLED && Date.now() < FABLE_SUNSET_MS;
}

// Haiku 4.5 — pulled by Anthropic 2026-06-26, REINSTATED upstream (probe-
// confirmed serving 2026-07-22, "max models" owner call). Kill-switch mirror
// of Fable: hides the picker row + coerces any stored/pinned Haiku pref → sonnet
// (the fast-tier fallback). Label/effort/history fallbacks stay intact so old
// Haiku conversations still render. Flip false to restore if it ever returns;
// mirror HAIKU_DISABLED in config.rs (backend coerces a pinned Haiku session).
export const HAIKU_DISABLED = false;
export function haikuAvailable(): boolean {
  return !HAIKU_DISABLED;
}

// The bare `sonnet` alias resolution lives BACKEND-side only (config.rs
// SONNET_MODEL / canonical_model_alias, applied in turn.rs before `--model`).
// A frontend mirror (SONNET_MODEL + canonicalModelAlias) existed here but had
// zero callers — removed 2026-07-01; the send path sends the bare selection.

const MODEL_KEY = "rift.assistant.model";
const EFFORT_KEY = "rift.assistant.thinkingEffort";
const THINKING_KEY = "rift.assistant.thinkingEnabled";
const PERMISSION_KEY = "rift.assistant.permissionMode";
const PLAN_KEY = "rift.assistant.plan";
const FAST_MODE_KEY = "rift.assistant.fastMode";
const AUTOCORRECT_KEY = "rift.assistant.autocorrect";

// Per-workspace override keys for model + effort. A `base::<root>` key holds a
// workspace's pinned choice; the bare global key is the baseline default for
// workspaces that have never been pinned. A per-workspace save writes ONLY the
// `base::<root>` key — it must NOT touch the global, or pinning a heavy project
// (e.g. a WPF repo with rebuild-on-chat loops) to Sonnet would drag the baseline
// to Sonnet and bleed into every unpinned project — the exact "one global
// setting dragging every project to the same cost" this feature exists to stop.
// The global baseline is updated only on a no-workspace save (ws null).
function wsKey(base: string, ws: string | null | undefined): string | null {
  return ws ? `${base}::${ws}` : null;
}

// One-time sweep of stale per-workspace `thinkingEnabled::<root>` pins. Before
// v0.65.0 thinking was a separate toggle that defaulted ON and persisted a
// per-folder `on` pin on first use; the off-by-default flip (cont.205/212) only
// changed the GLOBAL baseline, so any folder that already carried an `on` pin
// kept silently extend-thinking every turn. On Opus that thinking block emits
// no visible text, so the symptom is "this folder is randomly slow / hangs"
// with nothing in the UI explaining it. Clearing the per-folder pins makes every
// such folder fall back to the fast off-by-default baseline; the user can still
// raise the dial per-folder (a fresh, intentional pin) afterward. Model + effort
// pins are deliberately left alone — those are visible (picker / dial rung) and
// the per-folder design is intended (helpers.ts wsKey note). Idempotent + version
// -guarded so it runs exactly once per install.
const THINKING_PIN_SWEEP_KEY = "rift.assistant.thinkingPinSweep.v1";
export function migrateThinkingPins() {
  try {
    if (typeof localStorage === "undefined") return;
    if (localStorage.getItem(THINKING_PIN_SWEEP_KEY)) return; // already swept
    const prefix = `${THINKING_KEY}::`;
    const stale: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (k && k.startsWith(prefix)) stale.push(k);
    }
    for (const k of stale) localStorage.removeItem(k);
    localStorage.setItem(THINKING_PIN_SWEEP_KEY, "done");
  } catch {
    /* storage disabled — nothing to migrate */
  }
}

const PERMISSION_MODES: readonly PermissionMode[] = [
  "default", "acceptEdits", "plan", "auto", "bypassPermissions",
] as const;

/** Validate an untrusted string (e.g. a saved convo's model) into a ModelSel.
 *  Returns null for unknown values and for Fable past its sunset. */
export function asModelSel(v: unknown): ModelSel | null {
  if (typeof v !== "string" || !(MODEL_SELS as readonly string[]).includes(v)) return null;
  if (v === "claude-fable-5" && !fableAvailable()) return null;
  if (v === "haiku" && !haikuAvailable()) return null;
  return v as ModelSel;
}

export function loadModel(ws?: string | null): ModelSel {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(MODEL_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(MODEL_KEY);
      if (v && (MODEL_SELS as readonly string[]).includes(v)) {
        if (v === "claude-fable-5" && !fableAvailable()) return "opus"; // matches backend FABLE_FALLBACK_MODEL (turn.rs)
        if (v === "haiku" && !haikuAvailable()) return "sonnet"; // Haiku pulled → fast-tier fallback (mirror config.rs)
        return v as ModelSel;
      }
    }
  } catch {
    /* SSR or storage disabled */
  }
  return "sonnet";
}

export function saveModel(v: ModelSel, ws?: string | null) {
  try {
    if (typeof localStorage === "undefined") return;
    const k = wsKey(MODEL_KEY, ws);
    if (k) localStorage.setItem(k, v); // per-workspace pin — never touches the global baseline
    else localStorage.setItem(MODEL_KEY, v); // no workspace → set the baseline default
  } catch {
    /* storage disabled */
  }
}

/** Map a selected model to its visual family for the aurora hue. */
export function modelFamily(model: ModelSel): ModelFamily {
  if (model === "haiku") return "haiku";
  if (model === "opus" || model.includes("opus") || model === "claude-fable-5") return "opus";
  return "sonnet";
}

export function loadEffort(ws?: string | null): ThinkingEffort {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(EFFORT_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(EFFORT_KEY);
      // Legacy "quick" (retired — also sent the medium flag) folds into "smart"
      // read-side so old pins keep their wire behavior. Mirrors the backend's
      // normalize_effort_tier (config.rs).
      if (v === "quick") return "smart";
      if (v === "none" || v === "smart" || v === "deep" || v === "ultra") return v;
    }
  } catch {
    /* SSR or storage disabled */
  }
  return "smart";
}

export function saveEffort(v: ThinkingEffort, ws?: string | null) {
  try {
    if (typeof localStorage === "undefined") return;
    const k = wsKey(EFFORT_KEY, ws);
    if (k) localStorage.setItem(k, v); // per-workspace pin — never touches the global baseline
    else localStorage.setItem(EFFORT_KEY, v); // no workspace → set the baseline default
  } catch {
    /* storage disabled */
  }
}

/** Extended-thinking master switch. Default on (current behavior). Per-workspace
 *  like effort — a `base::<root>` pin overrides the global baseline. */
export function loadThinkingEnabled(ws?: string | null): boolean {
  try {
    if (typeof localStorage !== "undefined") {
      const k = wsKey(THINKING_KEY, ws);
      const v = (k ? localStorage.getItem(k) : null) ?? localStorage.getItem(THINKING_KEY);
      if (v === "off") return false;
      if (v === "on") return true;
    }
  } catch {
    /* SSR or storage disabled */
  }
  // Thinking OFF by default — mirrors Claude Code's own behavior (extended
  // thinking is opt-in, not every-turn), so a casual "hello" answers in ~1-2s
  // instead of burning ~3s on a hidden thinking block first. Any user who set a
  // preference (on/off stored above) is respected; only the fresh default flips.
  // Turn it on per-send (or raise effort) when a turn needs real reasoning.
  return false;
}

export function saveThinkingEnabled(v: boolean, ws?: string | null) {
  try {
    if (typeof localStorage === "undefined") return;
    const k = wsKey(THINKING_KEY, ws);
    if (k) localStorage.setItem(k, v ? "on" : "off"); // per-workspace pin — never touches the global baseline
    else localStorage.setItem(THINKING_KEY, v ? "on" : "off"); // no workspace → set the baseline default
  } catch {
    /* storage disabled */
  }
}

export function loadPermissionMode(): PermissionMode {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(PERMISSION_KEY) : null;
    if (v && (PERMISSION_MODES as readonly string[]).includes(v)) return v as PermissionMode;
  } catch {
    /* SSR or storage disabled */
  }
  // Fresh installs (no stored key) get bypassPermissions — Rift is an embedded
  // coding partner whose system prompt explicitly says "don't ask for permission
  // on routine work"; the user expects tools to just run and reverts via git.
  // The prompting modes (default/acceptEdits/plan) remain available per-tab via
  // the composer for anyone who wants the Allow/Deny gate. (Reverted from the
  // cont.202 "safe default" experiment: in prompting mode every Bash/Edit/Write
  // parked on a can_use_tool round-trip that wedged the turn for up to 30 min.)
  return "bypassPermissions";
}

export function savePermissionMode(v: PermissionMode) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(PERMISSION_KEY, v);
  } catch {
    /* storage disabled */
  }
}

/** Models the CLI's fast-output mode applies to — the Opus family only (the
 *  bare `opus` alias + pinned `claude-opus-4-x` ids). Fable shares Opus's
 *  VISUAL family (modelFamily) but is not fast-eligible upstream, so don't
 *  derive this from modelFamily. Mirrors model_fast_eligible in config.rs. */
export function fastEligible(model: string): boolean {
  return model === "opus" || model.startsWith("claude-opus-4-");
}

/** Fast mode (Opus fast output) master switch. GLOBAL like permissionMode —
 *  a speed preference, not a per-project pin. Default off; the toggle only
 *  surfaces on fast-eligible (Opus) rows and the backend re-gates by model +
 *  CLI version, so a stale `on` can never reach an ineligible spawn. */
export function loadFastMode(): boolean {
  try {
    if (typeof localStorage !== "undefined") return localStorage.getItem(FAST_MODE_KEY) === "on";
  } catch {
    /* SSR or storage disabled */
  }
  return false;
}

export function loadAutocorrect(): boolean {
  try {
    if (typeof localStorage !== "undefined") return localStorage.getItem(AUTOCORRECT_KEY) === "on";
  } catch {
    /* SSR or storage disabled */
  }
  return false; // opt-in — owner call (composer autocorrect arc)
}

export function saveAutocorrect(v: boolean) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(AUTOCORRECT_KEY, v ? "on" : "off");
  } catch {
    /* storage disabled */
  }
}

export function saveFastMode(v: boolean) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(FAST_MODE_KEY, v ? "on" : "off");
  } catch {
    /* storage disabled */
  }
}

const RIFT_PLANS: readonly RiftPlan[] = ["free", "pro", "max"] as const;

/** The user's subscription plan (USER-SET — Anthropic exposes no plan signal for
 *  OAuth users; see the RiftPlan type doc). Fresh installs default to `max` (1M)
 *  because that's correct for Max/Team/Enterprise and credit-enabled Pro — the
 *  majority of paying users. Free / uncredited-Pro users set their tier once in
 *  Settings, capping the gauge honestly at 200K. */
export function loadPlan(): RiftPlan {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(PLAN_KEY) : null;
    if (v && (RIFT_PLANS as readonly string[]).includes(v)) return v as RiftPlan;
  } catch {
    /* SSR or storage disabled */
  }
  return "max";
}

export function savePlan(v: RiftPlan) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(PLAN_KEY, v);
  } catch {
    /* storage disabled */
  }
}

/** Context-window ceiling the user's plan grants, regardless of the model's
 *  native window. Free is hard-capped at 200K even on 1M-native models; Pro
 *  (with usage credits) and Max get the full 1M. `ctxWindowForModelId` clamps
 *  the model-native window down to this. */
export function planContextCap(plan: RiftPlan): number {
  return plan === "free" ? 200_000 : 1_000_000;
}

export function flattenToolResult(content: unknown): string {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .map((c) => {
        if (typeof c !== "object" || !c) return "";
        if ("text" in c) return String((c as { text: unknown }).text ?? "");
        // Image parts (Read on a PNG, screenshot-bearing results) have no text
        // form — an honest placeholder beats silently rendering an empty result.
        if ((c as { type?: unknown }).type === "image") return "[image]";
        return "";
      })
      .join("");
  }
  return "";
}

/** First-priority field to preview for each known tool. Returns first ~120
 *  chars of that field's string value, or null. */
export function previewToolInput(_name: string, input: Record<string, unknown> | undefined): string | null {
  if (!input) return null;
  const fields = ["command", "file_path", "pattern", "path", "url", "query"] as const;
  for (const f of fields) {
    const v = input[f];
    if (typeof v === "string" && v.length > 0) {
      return v.length > 120 ? v.slice(0, 120) + "…" : v;
    }
  }
  return null;
}

/** Compact token count for the live turn readout (Claude-Code style: "1.2k").
 *  Guard against the 999_500–999_999 band rounding to "1000k": Math.round(n/1000)
 *  there yields 1000, so those values cross into the M branch instead. */
export function fmtTokens(n: number): string {
  if (n >= 999_500) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 10_000) return `${Math.round(n / 1000)}k`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return String(Math.round(n));
}

/** The model's NATIVE context window (tokens), ignoring plan entitlement. Haiku
 *  is 200K; current-gen Opus 4.8 / Sonnet 5 / Fable 5 are 1M. Matches the FULL
 *  pinned ids (claude-opus-4-8 …) AND the bare aliases Rift sends to the CLI
 *  (`opus`/`sonnet`/`fable` — see modelMatrix.ts MODEL_OPTIONS). Without the alias
 *  arm these fell through to 200K, so the ctx gauge read ~5× too full (16% at real
 *  3% usage) and the compaction pill's pre/post % used the wrong window. */
export function modelNativeWindow(model: string | null): number {
  if (!model) return 200_000;
  if (/\[1m\]/i.test(model)) return 1_000_000;
  const id = model.toLowerCase();
  if (id.includes("haiku")) return 200_000;
  if (/^(opus|sonnet|fable)$/.test(id)) return 1_000_000;
  // Sonnet 4.6 / 5 are 1M (the backend appends `[1m]` to the CLI arg for them —
  // caught by the `[1m]` arm above for live turns; this arm covers the bare echo).
  // Sonnet 4.5 is deliberately NOT here: the CLI gates it at 200K and the backend
  // never sends `[1m]` for it (config.rs SONNET_1M_GATED excludes it), so claiming
  // 1M here would over-report the window 5× for any resumed pre-rename session.
  if (/sonnet-(4-6|5)/.test(id) || /opus-4-[678]/.test(id) || /fable-5/.test(id)) return 1_000_000;
  return 200_000;
}

/** Effective context window for a model under the user's plan: the model's native
 *  window clamped to the plan ceiling. A Free user on Opus → min(1M, 200K) = 200K;
 *  a Max user → 1M. Single source of truth for the ctx% gauge (assistant.svelte.ts),
 *  the compaction pill (streaming.ts), and the picker labels (modelMatrix.ts) — so
 *  all three report the SAME number. `planCap` is optional for call sites that
 *  legitimately want the raw model window; omit it = native window (no clamp). */
export function ctxWindowForModelId(model: string | null, planCap?: number): number {
  const native = modelNativeWindow(model);
  return planCap !== undefined ? Math.min(native, planCap) : native;
}

/** Effort tiers low→high — canonical order for clamping + ladder UIs. */
export const EFFORT_ORDER: readonly ThinkingEffort[] = [
  "none", "smart", "deep", "ultra",
] as const;

/** Highest effort tier each model honors server-side — the single source of
 *  truth for the capability ceiling. `MODEL_OPTIONS.maxEffort` (the picker's
 *  slider) and `clampEffort` (the value actually sent) both derive from this so
 *  they can't disagree. Opus/Fable/Sonnet reach `ultra` (xhigh + ultracode) —
 *  Sonnet 5 honors xhigh + max server-side (unlike Sonnet 4.6, which rejected
 *  xhigh and capped at deep); Haiku rejects effort wholesale (`none`). Mirror
 *  the Sonnet ceiling in src-tauri/src/assistant/turn.rs (model_max_effort). */
export const MODEL_MAX_EFFORT: Record<ModelSel, ThinkingEffort> = {
  opus: "ultra",
  "claude-opus-4-7": "ultra",
  "claude-opus-4-6": "ultra",
  "claude-opus-4-5": "ultra",
  "claude-fable-5": "ultra",
  sonnet: "ultra",
  // Legacy Sonnets honor xhigh too — the old "4.6 rejects xhigh" ruling is
  // stale (live probe 2026-07-22: 4-5/4-6 both accept `--effort xhigh`).
  "claude-sonnet-4-6": "ultra",
  "claude-sonnet-4-5": "ultra",
  haiku: "none",
};

/** Clamp an effort tier to a model's ceiling. Pure. Fixes the slider hiding
 *  out-of-range stops while a stored pref still carried (e.g. an Opus workspace
 *  pinned to `ultra`, switched to Sonnet, kept sending xhigh). */
export function clampEffort(effort: ThinkingEffort, model: ModelSel): ThinkingEffort {
  const cap = MODEL_MAX_EFFORT[model] ?? "ultra";
  return EFFORT_ORDER.indexOf(effort) > EFFORT_ORDER.indexOf(cap) ? cap : effort;
}

/** Effort → CLI flag mapping. Must mirror src-tauri/src/assistant/turn.rs.
 *  Ladder: none→low · smart→medium · deep→high · ultra→xhigh;
 *  ultra's autonomous-workflow behavior rides the separate `ultracode` settings
 *  key, set in turn.rs. The effort is clamped to the model's ceiling first, so an
 *  out-of-range tier can never emit a flag the model rejects.
 *
 *  Why "smart" → medium (not high): the CLI default is `high`, but Anthropic's
 *  own API guidance says to OVERRIDE that for interactive use — Sonnet 4.6 at
 *  `high` "almost always thinks" and the postmortem names the exact symptom we
 *  hit (the UI appears frozen while it does a long hidden pre-pass before the
 *  first token; their words). `medium` is the documented "best balance of speed,
 *  cost, and performance for most applications." Heavy reasoning stays one click
 *  away as the explicit "Deep" (high) / "Ultracode" (xhigh) tiers. Model-
 *  agnostic — every present + future model gets a responsive default tier. */
export function effortToFlag(
  effort: ThinkingEffort,
  model: ModelSel,
): "low" | "medium" | "high" | "xhigh" | null {
  if (model === "haiku") return null;
  const e = clampEffort(effort, model);
  if (e === "none") return "low";
  if (e === "smart") return "medium";
  if (e === "ultra") return "xhigh";
  return "high"; // "deep"
}

/** #80: stale-event discrimination. The backend stamps every stream/done/error
 *  event with the `turn_epoch` the frontend minted for that turn (TabState.
 *  turnEpoch, bumped per send). A positive payload epoch that differs from the
 *  tab's current one belongs to a stopped/superseded turn — the listener drops
 *  it instead of letting it finalize (or paint into) the next turn. A missing/
 *  zero epoch on EITHER side means "no epoch info" (prewarm, legacy event, dev
 *  hot-reload) → accept, preserving pre-epoch behavior. Pure. */
export function isStaleTurnEpoch(tabEpoch: number, payloadEpoch: unknown): boolean {
  return typeof payloadEpoch === "number" && payloadEpoch > 0
    && tabEpoch > 0 && payloadEpoch !== tabEpoch;
}
