// Plan-limit gauges — thin reactive layer over `usage_rate_limits` (the one
// surviving usage command). Composer UsagePanel + Home t-limits tile read this.

import { invoke } from "@tauri-apps/api/core";

// Live plan limits (same data as Claude Code's /usage) — utilization is 0–100.
export type LimitWindow = { utilization: number; resetsAt: string | null };
type ExtraUsage = {
  isEnabled: boolean; monthlyLimit: number | null; usedCredits: number | null;
  utilization: number | null; currency: string | null;
  // Minor-unit exponent: monthlyLimit/usedCredits are scaled integers (8000 @ dp=2 = $80).
  decimalPlaces: number;
};
// Newer generic window list — model-scoped weeklies (e.g. Fable) only exist
// here; the legacy per-model buckets above come back null now.
export type ScopedLimit = {
  kind: string | null; group: string | null; percent: number;
  severity: string | null; resetsAt: string | null;
  scope: { model: { displayName: string | null } | null } | null;
  isActive: boolean;
};
type RateLimits = {
  fiveHour: LimitWindow | null; sevenDay: LimitWindow | null;
  sevenDayOpus: LimitWindow | null; sevenDaySonnet: LimitWindow | null;
  extraUsage: ExtraUsage | null; limits: ScopedLimit[]; fetchedAt: number;
};

// AI Health advisor — one recommendation card from the analyzer. Shape mirrors
// the JSON contract in oneshot.rs ANALYZE_META_PROMPT; the model is instructed
// to emit exactly this. Parse is guarded, so a malformed reply degrades to an
// error string rather than throwing.
type AdviceImpact = "high" | "medium" | "low";
// A concrete one-tap action a card can carry. The model emits a machine value;
// the frontend re-validates it (see normalizeApply) before ever applying — the
// model's value is a suggestion, not a trusted command.
type AdviceApplyKind = "effort" | "model" | "budget";
export type AdviceApply = { kind: AdviceApplyKind; value: string | number; label: string };
type AdviceCard = {
  title: string; detail: string; impact: AdviceImpact; apply: AdviceApply | null;
};
type UsageAdvice = { summary: string; cards: AdviceCard[] };

/** Shared warn/hot zoning for limit bars (status bar + usage panel). Percent
 *  thresholds, overridden by the endpoint's own `severity` when it says things
 *  are worse — it knows the plan better than our cutoffs do. */
export function limitZone(u: number, severity: string | null = null): string {
  if (severity === "exceeded" || severity === "critical") return "hot";
  if (severity === "warning" || severity === "elevated") return "warn";
  return u < 60 ? "ok" : u < 85 ? "warn" : "hot";
}

const EFFORT_VALUES = ["none", "smart", "deep", "ultra"] as const;
// The advisor intentionally recommends only the main default-model tiers.
// Haiku can remain picker-eligible without becoming an advisor apply target.
// Keep this list and ANALYZE_META_PROMPT's model enum (oneshot.rs) in lockstep.
const MODEL_VALUES = ["opus", "sonnet", "fable"] as const;

/** Re-validate a model-emitted apply action into a known-safe shape, or drop it
 *  (→ null) if it's malformed or out of range. Never trust the model's value
 *  directly — this is the gate between "the model said" and "Rift will do".
 *  `allowBudget` is false for subscription sessions: a per-turn dollar cap is
 *  inert there (usage-limit windows govern spend, not dollars), so a stray
 *  budget card from an older/confused reply is dropped rather than applied. */
function normalizeApply(raw: unknown, allowBudget: boolean): AdviceApply | null {
  if (!raw || typeof raw !== "object") return null;
  const a = raw as Record<string, unknown>;
  const label = typeof a.label === "string" ? a.label : "";
  // Legacy "quick" tier (retired — sent the same medium flag as smart) folds
  // into "smart" so an older/stale advisor reply still applies cleanly.
  const effortValue = a.value === "quick" ? "smart" : a.value;
  if (a.kind === "effort" && EFFORT_VALUES.includes(effortValue as never)) {
    return { kind: "effort", value: effortValue as string, label };
  }
  if (a.kind === "model" && MODEL_VALUES.includes(a.value as never)) {
    return { kind: "model", value: a.value as string, label };
  }
  if (a.kind === "budget" && allowBudget) {
    const n = typeof a.value === "number" ? a.value : Number(a.value);
    if (Number.isFinite(n) && n > 0 && n <= 100) {
      return { kind: "budget", value: Math.round(n * 100) / 100, label };
    }
  }
  return null;
}

class UsageStore {
  rateLimits = $state<RateLimits | null>(null);
  rateLimitsError = $state<string | null>(null);

  // Advisor state — the "Analyze my usage" flow.
  analyzing = $state(false);
  advice = $state<UsageAdvice | null>(null);
  adviceError = $state<string | null>(null);

  /** Live plan-limit gauge — best-effort: an OAuth hiccup (no login, throttle)
   *  only sets the error string. `force` busts the backend's 60s cache (the
   *  panel's manual refresh button). */
  async refreshRateLimits(cliVersion: string | null, force = false): Promise<void> {
    try {
      this.rateLimits = await invoke<RateLimits>("usage_rate_limits", { cliVersion, force });
      this.rateLimitsError = null;
    } catch (e) {
      this.rateLimitsError = String(e);
    }
  }

  /** Spawn the user's own Claude to analyze a usage snapshot and produce advice
   *  cards. `snapshotJson` is the frontend-assembled limits + telemetry + setup
   *  blob; the backend enriches it with server-only config before the call.
   *  `allowBudget` (API-key sessions only) lets a per-turn dollar-cap apply card
   *  through; subscription sessions drop it — a $ cap is inert under plan limits. */
  async analyzeUsage(snapshotJson: string, allowBudget: boolean): Promise<void> {
    this.analyzing = true;
    this.adviceError = null;
    try {
      const raw = await invoke<string>("assistant_analyze_usage", { snapshotJson });
      const parsed = JSON.parse(raw) as UsageAdvice;
      if (!parsed || !Array.isArray(parsed.cards)) {
        throw new Error("analysis returned an unexpected shape");
      }
      // Re-validate each card's apply action — the model's value is a hint, not
      // a trusted command (normalizeApply drops anything out of range).
      this.advice = {
        summary: typeof parsed.summary === "string" ? parsed.summary : "",
        cards: parsed.cards.map((c) => ({
          title: String(c?.title ?? ""),
          detail: String(c?.detail ?? ""),
          impact: (["high", "medium", "low"] as const).includes(c?.impact as never)
            ? c.impact : "medium",
          apply: normalizeApply((c as Record<string, unknown>)?.apply, allowBudget),
        })),
      };
    } catch (e) {
      this.adviceError = e instanceof SyntaxError
        ? "Couldn't read the advice — the analysis came back malformed. Try again."
        : String(e);
    } finally {
      this.analyzing = false;
    }
  }
}

export const usage = new UsageStore();
