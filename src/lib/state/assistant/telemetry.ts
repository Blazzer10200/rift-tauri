import type { TurnRecord } from "./types";

/** UUID for a fresh session. `crypto.randomUUID` is available in the webview;
 *  the fallback keeps SSR/test contexts from throwing. */
function newSessionId(): string {
  const c = (globalThis as { crypto?: Crypto }).crypto;
  if (c?.randomUUID) return c.randomUUID();
  return "sess-" + Date.now().toString(36) + "-" + Math.floor(Math.random() * 1e9).toString(36);
}

/** Session-wide telemetry singleton. */
export class SessionTelemetry {
  /** Stable per-launch id. Doubles as the on-disk session-log filename so a
   *  persisted snapshot keeps the same identity across autosaves. */
  id = newSessionId();
  startedAt = Date.now();
  turns: TurnRecord[] = [];
  /** Non-turn lifecycle events: tab open/close/new/switch, slash commands,
   *  workspace changes, session-lost recoveries, etc. Cheap to capture. */
  events: { ts: number; kind: string; detail?: unknown }[] = [];

  /** Ring caps — a long-lived session (days open) would otherwise grow
   *  turns[]/events[] unbounded until /diag-clear. Drop oldest on overflow. */
  private static readonly MAX_TURNS = 500;
  private static readonly MAX_EVENTS = 2000;

  pushTurn(t: TurnRecord) {
    this.turns.push(t);
    if (this.turns.length > SessionTelemetry.MAX_TURNS) {
      this.turns.splice(0, this.turns.length - SessionTelemetry.MAX_TURNS);
    }
  }

  event(kind: string, detail?: unknown) {
    this.events.push({ ts: Date.now(), kind, detail });
    if (this.events.length > SessionTelemetry.MAX_EVENTS) {
      this.events.splice(0, this.events.length - SessionTelemetry.MAX_EVENTS);
    }
  }

  /** JSON snapshot for /diag clipboard export. */
  snapshot() {
    const now = Date.now();
    return {
      id: this.id,
      startedAt: this.startedAt,
      capturedAt: now,
      durationMs: now - this.startedAt,
      turnCount: this.turns.length,
      summary: this.summarize(),
      turns: this.turns,
      events: this.events,
    };
  }

  /** Per-session rollup. Delegates to the pure {@link summarizeSession} so a
   *  snapshot loaded from disk can re-derive fields added after it was written
   *  (#33: avgDeadWaitMs et al. showed "—" on pre-field logs). */
  private summarize() {
    return summarizeSession(this.turns, this.events);
  }

  reset() {
    this.id = newSessionId();
    this.startedAt = Date.now();
    this.turns = [];
    this.events = [];
  }
}

/** Pure per-session rollup over raw turns/events. Self-summarizing JSON so a
 *  `/diag` reader doesn't have to fold over `turns[]` — and so a loaded log can
 *  backfill derived fields that postdate its on-disk summary. */
export function summarizeSession(
  turns: TurnRecord[],
  events: { ts: number; kind: string; detail?: unknown }[],
) {
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
    // Zero-tool turns = pure-conversation turns. Tracked with their spend b/c
    // they're the cheapest thing to route to a smaller model (40-session audit
    // 2026-06-11: 28% of turns used no tools, mostly on the priciest model).
    let zeroToolTurns = 0;
    let zeroToolCostUsd = 0;
    const ttfps: number[] = [];
    const doneTimes: number[] = [];
    // Latency attribution: ttfp is what the user *feels* as the silent wait, but
    // it lumps together two very different costs. `deadWait` = the slice of
    // time-to-first-paint NOT explained by thinking (CLI subprocess spawn,
    // prefill on a cache miss, queue) — the genuinely silent stall. Surfaced
    // because a zero-tool turn was observed at 52s ttfp / 5.5s thinking = ~46s
    // of unattributed silence the dashboard couldn't previously see.
    const deadWaits: number[] = [];
    let worstDeadWaitTurn: { idx: number; deadWaitMs: number } | null = null;
    // Done-time attribution: split a turn's wall-clock into tool-active time
    // (union of tool intervals — they parallelize, so sum would over-count) vs
    // model time (generation/decode). Tells whether a slow turn was tool-bound
    // or generation-bound.
    let totalToolActiveMs = 0;
    let totalModelMs = 0;
    let worstToolBoundTurn: { idx: number; toolActiveMs: number; doneMs: number } | null = null;
    // #204: per-model timing accumulators, filled in the single pass below so
    // the per-model averages don't re-filter turns once per model key.
    const byModelTimings: Record<string, { ttfp: number[]; done: number[] }> = {};
    for (let i = 0; i < turns.length; i++) {
      const t = turns[i];
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
      if (t.toolUses.length === 0 && t.endKind === "success") {
        zeroToolTurns += 1;
        zeroToolCostUsd += t.costUsd ?? 0;
      }
      if (t.firstPaintAt != null) {
        const v = t.firstPaintAt - t.ts; ttfps.push(v); tm.ttfp.push(v);
        const dead = Math.max(0, v - t.thinkingTotalMs);
        deadWaits.push(dead);
        if (!worstDeadWaitTurn || dead > worstDeadWaitTurn.deadWaitMs) {
          worstDeadWaitTurn = { idx: i, deadWaitMs: dead };
        }
      }
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
      let toolActiveMs = 0;
      if (intervals.length > 0) {
        intervals.sort((a, b) => a.ts - b.ts || b.delta - a.delta);
        let active = 0;
        let peak = 0;
        let spanStart = 0;
        for (const iv of intervals) {
          const prev = active;
          active += iv.delta;
          // Union span: a busy region opens when concurrency leaves 0 and
          // closes when it returns to 0; the gap is wall-clock with ≥1 tool live.
          if (prev === 0 && active > 0) spanStart = iv.ts;
          else if (prev > 0 && active === 0) toolActiveMs += iv.ts - spanStart;
          if (active > peak) peak = active;
        }
        if (!mostParallelTurn || peak > mostParallelTurn.maxConcurrentTools) {
          mostParallelTurn = { idx: i, maxConcurrentTools: peak };
        }
      }
      totalToolActiveMs += toolActiveMs;
      if (t.doneAt != null) {
        // Model time = wall-clock minus thinking minus tool-active. Floored at 0
        // since the three are independently measured and can slightly overlap.
        totalModelMs += Math.max(0, (t.doneAt - t.ts) - t.thinkingTotalMs - toolActiveMs);
        if (toolActiveMs > 0 && (!worstToolBoundTurn || toolActiveMs > worstToolBoundTurn.toolActiveMs)) {
          worstToolBoundTurn = { idx: i, toolActiveMs, doneMs: t.doneAt - t.ts };
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
      totalTurns: turns.length,
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
      zeroToolTurns,
      zeroToolCostUsd: Math.round(zeroToolCostUsd * 10000) / 10000,
      avgDeadWaitMs: deadWaits.length ? Math.round(deadWaits.reduce((a, b) => a + b, 0) / deadWaits.length) : null,
      worstDeadWaitTurn,
      totalToolActiveMs,
      totalModelMs,
      worstToolBoundTurn,
      outputTokensPerSec: totalStreamMs > 0
        ? Math.round((totalOutputTokens / totalStreamMs) * 1000)
        : null,
      byModel,
      eventCounts: events.reduce<Record<string, number>>((acc, e) => {
        acc[e.kind] = (acc[e.kind] ?? 0) + 1;
        return acc;
      }, {}),
    };
  }
