// Plan-limit gauges — thin reactive layer over `usage_rate_limits` (the one
// surviving usage command). Composer UsagePanel + Home t-limits tile read this.

import { invoke } from "@tauri-apps/api/core";

// Live plan limits (same data as Claude Code's /usage) — utilization is 0–100.
export type LimitWindow = { utilization: number; resetsAt: string | null };
export type ExtraUsage = {
  isEnabled: boolean; monthlyLimit: number | null; usedCredits: number | null;
  utilization: number | null; currency: string | null;
};
export type RateLimits = {
  fiveHour: LimitWindow | null; sevenDay: LimitWindow | null;
  sevenDayOpus: LimitWindow | null; sevenDaySonnet: LimitWindow | null;
  extraUsage: ExtraUsage | null; fetchedAt: number;
};

class UsageStore {
  rateLimits = $state<RateLimits | null>(null);
  rateLimitsError = $state<string | null>(null);

  /** Live plan-limit gauge — best-effort: an OAuth hiccup (no login, throttle)
   *  only sets the error string. */
  async refreshRateLimits(cliVersion: string | null): Promise<void> {
    try {
      this.rateLimits = await invoke<RateLimits>("usage_rate_limits", { cliVersion });
      this.rateLimitsError = null;
    } catch (e) {
      this.rateLimitsError = String(e);
    }
  }
}

export const usage = new UsageStore();
