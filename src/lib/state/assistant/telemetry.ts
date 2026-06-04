import type { TurnRecord } from "./types";

/** Session-wide telemetry singleton. */
export class SessionTelemetry {
  startedAt = Date.now();
  turns: TurnRecord[] = [];
  /** Non-turn lifecycle events: tab open/close/new/switch, slash commands,
   *  workspace changes, session-lost recoveries, etc. Cheap to capture. */
  events: { ts: number; kind: string; detail?: unknown }[] = [];

  event(kind: string, detail?: unknown) {
    this.events.push({ ts: Date.now(), kind, detail });
  }

  /** JSON snapshot for /diag clipboard export. */
  snapshot() {
    const now = Date.now();
    return {
      startedAt: this.startedAt,
      capturedAt: now,
      durationMs: now - this.startedAt,
      turnCount: this.turns.length,
      summary: this.summarize(),
      turns: this.turns,
      events: this.events,
    };
  }

  /** Per-session rollup. Self-summarizing JSON so a `/diag` reader doesn't
   *  have to fold over `turns[]` to see the basics. */
  private summarize() {
    const byModel: Record<string, {
      turns: number;
      costUsd: number;
      inputTokens: number;
      outputTokens: number;
      cacheReadTokens: number;
      cacheCreateTokens: number;
      thinkingTurns: number;
      blankTurns: number;
      envelopeFallbacks: number;
      avgTtfpMs: number | null;
      avgDoneMs: number | null;
    }> = {};
    let totalCost = 0;
    let blank = 0;
    let envFallback = 0;
    let thinkingTurns = 0;
    let toolCallTotal = 0;
    let toolErrorTotal = 0;
    let slowestTool: { name: string; durationMs: number; turnIdx: number } | null = null;
    const toolNameCounts: Record<string, number> = {};
    let slowestTurn: { idx: number; durationMs: number } | null = null;
    let costliestTurn: { idx: number; costUsd: number } | null = null;
    let firstTurnCostUsd: number | null = null;
    let coldStartCacheCreate: number | null = null;
    let totalOutputTokens = 0;
    let totalStreamMs = 0;
    let mostParallelTurn: { idx: number; maxConcurrentTools: number } | null = null;
    let staleCacheTurns = 0;
    const ttfps: number[] = [];
    const doneTimes: number[] = [];
    // #204: per-model timing accumulators, filled in the single pass below so
    // the per-model averages don't re-filter this.turns once per model key.
    const byModelTimings: Record<string, { ttfp: number[]; done: number[] }> = {};
    for (let i = 0; i < this.turns.length; i++) {
      const t = this.turns[i];
      // Cold-start surfacing: the first turn typically pays the SessionStart
      // 40-50K cache_creation tax; we record turn[0]'s cost+cacheCreate to
      // make that tax legible without folding turns[]. Must run before the
      // modelId guard so a failed turn[0] still records the cold-start metrics.
      if (i === 0) {
        firstTurnCostUsd = t.costUsd ?? null;
        const u0 = t.resultUsage || t.envelopeUsage;
        coldStartCacheCreate = u0?.cacheCreate ?? null;
      }
      // Skip user-stop / error turns w/ no resolved modelId from the byModel
      // rollup — they otherwise create a phantom "opus"/"sonnet"/"haiku"
      // bucket alongside the real "claude-opus-4-7" etc.
      if (t.modelId == null && t.endKind !== "success") continue;
      const key = t.modelId || t.model;
      const bucket = byModel[key] ||= {
        turns: 0, costUsd: 0, inputTokens: 0, outputTokens: 0,
        cacheReadTokens: 0, cacheCreateTokens: 0,
        thinkingTurns: 0, blankTurns: 0, envelopeFallbacks: 0,
        avgTtfpMs: null, avgDoneMs: null,
      };
      const tm = (byModelTimings[key] ||= { ttfp: [], done: [] });
      bucket.turns += 1;
      bucket.costUsd += t.costUsd ?? 0;
      const u = t.resultUsage || t.envelopeUsage;
      if (u) {
        bucket.inputTokens += u.input;
        bucket.outputTokens += u.output;
        bucket.cacheReadTokens += u.cacheRead;
        bucket.cacheCreateTokens += u.cacheCreate;
      }
      if (t.thinkingCount > 0) { bucket.thinkingTurns += 1; thinkingTurns += 1; }
      if (t.blankTurn) { bucket.blankTurns += 1; blank += 1; }
      if (t.envelopeFallback) { bucket.envelopeFallbacks += 1; envFallback += 1; }
      totalCost += t.costUsd ?? 0;
      if (t.firstPaintAt != null) { const v = t.firstPaintAt - t.ts; ttfps.push(v); tm.ttfp.push(v); }
      if (t.doneAt != null) {
        const dur = t.doneAt - t.ts;
        doneTimes.push(dur);
        tm.done.push(dur);
        if (!slowestTurn || dur > slowestTurn.durationMs) slowestTurn = { idx: i, durationMs: dur };
      }
      if (t.costUsd != null && (!costliestTurn || t.costUsd > costliestTurn.costUsd)) {
        costliestTurn = { idx: i, costUsd: t.costUsd };
      }
      // Tool rollup + parallelism detection via sweep-line over intervals.
      const intervals: { ts: number; delta: 1 | -1 }[] = [];
      for (const tu of t.toolUses) {
        toolCallTotal += 1;
        toolNameCounts[tu.name] = (toolNameCounts[tu.name] ?? 0) + 1;
        if (tu.isError === true) toolErrorTotal += 1;
        if (tu.durationMs != null && (!slowestTool || tu.durationMs > slowestTool.durationMs)) {
          slowestTool = { name: tu.name, durationMs: tu.durationMs, turnIdx: i };
        }
        if (tu.completedAt != null) {
          intervals.push({ ts: tu.startedAt, delta: 1 });
          intervals.push({ ts: tu.completedAt, delta: -1 });
        }
      }
      if (intervals.length > 0) {
        intervals.sort((a, b) => a.ts - b.ts || b.delta - a.delta);
        let active = 0;
        let peak = 0;
        for (const iv of intervals) {
          active += iv.delta;
          if (active > peak) peak = active;
        }
        if (!mostParallelTurn || peak > mostParallelTurn.maxConcurrentTools) {
          mostParallelTurn = { idx: i, maxConcurrentTools: peak };
        }
      }

      // Stale-cache flag: a continuation turn that paid full cache_create but
      // got zero cache_read = the API isn't reusing our prefix. Flagged what
      // surfaced the sonnet cache anomaly during effort A/B.
      if (!t.isFirstTurn && t.endKind === "success") {
        const uForCache = t.resultUsage || t.envelopeUsage;
        if (uForCache && uForCache.cacheRead === 0 && uForCache.cacheCreate > 0) {
          staleCacheTurns += 1;
        }
      }
      // Streaming velocity accumulator.
      if (t.firstPaintAt != null && t.doneAt != null && t.doneAt > t.firstPaintAt) {
        if (u) {
          totalOutputTokens += u.output;
          totalStreamMs += t.doneAt - t.firstPaintAt;
        }
      }
    }
    // Per-model timing averages — single O(N) pass already accumulated the
    // arrays in #204; here we only reduce the pre-bucketed values.
    for (const key of Object.keys(byModel)) {
      const bucket = byModel[key];
      const tm = byModelTimings[key] ?? { ttfp: [], done: [] };
      bucket.avgTtfpMs = tm.ttfp.length ? Math.round(tm.ttfp.reduce((a, b) => a + b, 0) / tm.ttfp.length) : null;
      bucket.avgDoneMs = tm.done.length ? Math.round(tm.done.reduce((a, b) => a + b, 0) / tm.done.length) : null;
    }
    return {
      totalTurns: this.turns.length,
      totalCostUsd: Math.round(totalCost * 10000) / 10000,
      blankTurns: blank,
      envelopeFallbacks: envFallback,
      thinkingTurns,
      avgTtfpMs: ttfps.length ? Math.round(ttfps.reduce((a, b) => a + b, 0) / ttfps.length) : null,
      avgDoneMs: doneTimes.length ? Math.round(doneTimes.reduce((a, b) => a + b, 0) / doneTimes.length) : null,
      toolCallTotal,
      toolErrorTotal,
      toolNameCounts,
      slowestTool,
      slowestTurn,
      costliestTurn,
      firstTurnCostUsd,
      coldStartCacheCreate,
      mostParallelTurn,
      staleCacheTurns,
      outputTokensPerSec: totalStreamMs > 0
        ? Math.round((totalOutputTokens / totalStreamMs) * 1000)
        : null,
      byModel,
      eventCounts: this.events.reduce<Record<string, number>>((acc, e) => {
        acc[e.kind] = (acc[e.kind] ?? 0) + 1;
        return acc;
      }, {}),
    };
  }

  reset() {
    this.startedAt = Date.now();
    this.turns = [];
    this.events = [];
  }
}
