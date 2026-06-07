// Cost-cockpit store (idea-phase-plan §1e). Thin reactive layer over the
// Rust `usage_*` commands — the heavy aggregation runs in SQLite, this just
// holds the render-ready DTOs and a refresh() that fans the queries out. The
// CostPage (Harness sub-tab) reads off this; no folding happens here.

import { invoke } from "@tauri-apps/api/core";

export type DailyRow = {
  date: string; cost: number; turns: number;
  input: number; output: number; cacheRead: number; cacheWrite: number;
};
export type MonthlyRow = {
  month: string; cost: number; turns: number;
  input: number; output: number; cacheRead: number; cacheWrite: number;
};
export type ModelRow = {
  modelId: string; provider: string | null; cost: number; turns: number;
  input: number; output: number; cacheRead: number; cacheWrite: number; priced: boolean;
};
export type WorkspaceRow = {
  workspace: string | null; cost: number; turns: number; input: number; output: number;
};
export type BlockRow = {
  start: number; end: number; cost: number; turns: number;
  input: number; output: number; cacheRead: number; cacheWrite: number; active: boolean;
};
export type BudgetPlan = "pro" | "max5x" | "max20x" | "custom";
export type BudgetCadence = "monthly" | "weekly" | "daily";
export type BudgetConfig = { plan: BudgetPlan; customLimitUsd: number | null; cadence: BudgetCadence };
export type BudgetStatus = {
  plan: BudgetPlan; cadence: BudgetCadence; limit: number; spent: number;
  pctRemaining: number; windowStart: number; windowEnd: number; burnPerDay: number;
  projectedExhaustionDate: number | null; daysRemaining: number | null;
};
// 2b insight layer — observational "Rift noticed…" patterns from the corpus.
export type Insight = {
  id: string; kind: string; title: string; detail: string;
  severity: "good" | "info" | "warn";
};

class UsageStore {
  daily = $state<DailyRow[]>([]);
  monthly = $state<MonthlyRow[]>([]);
  byModel = $state<ModelRow[]>([]);
  byWorkspace = $state<WorkspaceRow[]>([]);
  blocks = $state<BlockRow[]>([]);
  insights = $state<Insight[]>([]);
  budget = $state<BudgetStatus | null>(null);
  config = $state<BudgetConfig>({ plan: "max5x", customLimitUsd: null, cadence: "monthly" });

  loading = $state(false);
  loaded = $state(false);
  error = $state<string | null>(null);

  /** Total cost across all monthly buckets — the all-time spend headline. */
  allTimeCost = $derived(this.monthly.reduce((s, m) => s + m.cost, 0));
  allTimeTurns = $derived(this.monthly.reduce((s, m) => s + m.turns, 0));

  /** Fan out every read in parallel; the queries are independent. */
  async refresh(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const [daily, monthly, byModel, byWorkspace, blocks, insights, budget, config] = await Promise.all([
        invoke<DailyRow[]>("usage_daily", { days: 30 }),
        invoke<MonthlyRow[]>("usage_monthly"),
        invoke<ModelRow[]>("usage_by_model"),
        invoke<WorkspaceRow[]>("usage_by_workspace"),
        invoke<BlockRow[]>("usage_blocks", { window: 5 }),
        invoke<Insight[]>("usage_insights"),
        invoke<BudgetStatus>("usage_budget_status"),
        invoke<BudgetConfig>("usage_get_budget"),
      ]);
      this.daily = daily;
      this.monthly = monthly;
      this.byModel = byModel;
      this.byWorkspace = byWorkspace;
      this.blocks = blocks;
      this.insights = insights;
      this.budget = budget;
      this.config = config;
      this.loaded = true;
    } catch (e) {
      this.error = String(e);
      console.warn("usage refresh failed", e);
    } finally {
      this.loading = false;
    }
  }

  /** Persist a new budget config, then re-pull the derived status. */
  async setBudget(next: BudgetConfig): Promise<void> {
    try {
      await invoke("usage_set_budget", { config: next });
      this.config = next;
      this.budget = await invoke<BudgetStatus>("usage_budget_status");
    } catch (e) {
      this.error = String(e);
      console.warn("usage_set_budget failed", e);
    }
  }
}

export const usage = new UsageStore();
