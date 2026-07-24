import { describe, expect, it } from "vitest";
import {
  type ConvoStat,
  filterRange, summarize, streaks, peakHour, perModel, topModel,
  intensity, funFact, fmtCompact, fmtCost, fmtInt, hourLabel, modelLabel, localDayIndex,
} from "./statsHelpers";

const DAY = 86_400_000;
// A fixed "now" at a known local wall-clock (mid-afternoon) so day/hour math is
// deterministic regardless of the machine's offset — stats are built relative to it.
const NOW = new Date(2026, 5, 14, 15, 0, 0).getTime(); // Sun Jun 14 2026, 3 PM local

function stat(p: Partial<ConvoStat> & { updatedAt: number }): ConvoStat {
  return {
    createdAt: p.updatedAt, model: "opus",
    messages: 0, userMessages: 0, toolCalls: 0, words: 0, costUsd: 0,
    ...p,
  };
}
/** A convo whose updatedAt is N local days before NOW, at NOW's hour. */
function daysAgo(n: number, extra: Partial<ConvoStat> = {}): ConvoStat {
  return stat({ updatedAt: NOW - n * DAY, ...extra });
}

describe("localDayIndex", () => {
  it("collapses same-day timestamps to one index, adjacent days differ by 1", () => {
    const morning = new Date(2026, 5, 14, 8).getTime();
    const night = new Date(2026, 5, 14, 23).getTime();
    const next = new Date(2026, 5, 15, 1).getTime();
    expect(localDayIndex(morning)).toBe(localDayIndex(night));
    expect(localDayIndex(next)).toBe(localDayIndex(morning) + 1);
  });
});

describe("summarize", () => {
  it("sums counts and tallies distinct active local days", () => {
    const stats = [
      daysAgo(0, { messages: 4, userMessages: 2, toolCalls: 3, words: 100, costUsd: 0.5 }),
      daysAgo(0, { messages: 2, userMessages: 1, toolCalls: 1, words: 50, costUsd: 0.25 }), // same day
      daysAgo(2, { messages: 6, userMessages: 3, toolCalls: 5, words: 200, costUsd: 1 }),
    ];
    const t = summarize(stats);
    expect(t.sessions).toBe(3);
    expect(t.messages).toBe(12);
    expect(t.toolCalls).toBe(9);
    expect(t.words).toBe(350);
    expect(t.cost).toBeCloseTo(1.75);
    expect(t.activeDays).toBe(2); // two distinct days despite three sessions
  });
});

describe("filterRange", () => {
  const stats = [daysAgo(0), daysAgo(5), daysAgo(20), daysAgo(100)];
  it("7d keeps only the last week, 30d the last month, all keeps everything", () => {
    expect(filterRange(stats, "7d", NOW).length).toBe(2);
    expect(filterRange(stats, "30d", NOW).length).toBe(3);
    expect(filterRange(stats, "all", NOW).length).toBe(4);
  });
});

describe("streaks", () => {
  it("counts a consecutive run ending today", () => {
    const s = streaks([daysAgo(0), daysAgo(1), daysAgo(2)], NOW);
    expect(s.current).toBe(3);
    expect(s.longest).toBe(3);
  });
  it("still counts a streak that ends yesterday (today not opened yet)", () => {
    expect(streaks([daysAgo(1), daysAgo(2)], NOW).current).toBe(2);
  });
  it("breaks the current streak when there's a gap, but longest survives", () => {
    const s = streaks([daysAgo(0), daysAgo(3), daysAgo(4), daysAgo(5)], NOW);
    expect(s.current).toBe(1); // only today
    expect(s.longest).toBe(3); // the 3-5 run
  });
  it("is zero for no activity", () => {
    expect(streaks([], NOW)).toEqual({ current: 0, longest: 0 });
  });
});

describe("peakHour", () => {
  it("returns the local hour with the most messages, weighted", () => {
    const at = (h: number, messages: number) => stat({ updatedAt: new Date(2026, 5, 10, h).getTime(), messages });
    expect(peakHour([at(9, 2), at(14, 9), at(14, 1), at(2, 3)])).toBe(14);
  });
  it("is null with no messages", () => {
    expect(peakHour([stat({ updatedAt: NOW, messages: 0 })])).toBeNull();
  });
});

describe("perModel / topModel", () => {
  const stats = [
    daysAgo(0, { model: "opus", messages: 8, costUsd: 6 }),
    daysAgo(1, { model: "sonnet", messages: 2, costUsd: 0.1 }),
    daysAgo(1, { model: "opus", messages: 2, costUsd: 1 }),
  ];
  it("groups by model, computes message share, sorts by messages desc", () => {
    const m = perModel(stats);
    expect(m[0].model).toBe("opus");
    expect(m[0].messages).toBe(10);
    expect(m[0].share).toBeCloseTo(10 / 12);
    expect(m[0].label).toBe("Opus 5");
  });
  it("topModel returns the busiest model's label", () => {
    expect(topModel(stats)).toBe("Opus 5");
    expect(topModel([])).toBeNull();
  });
});

describe("intensity", () => {
  it("is 0 for empty, 4 for the max, monotonic in between", () => {
    expect(intensity(0, 100)).toBe(0);
    expect(intensity(100, 100)).toBe(4);
    expect(intensity(1, 100)).toBeLessThan(intensity(50, 100));
  });
});

describe("funFact", () => {
  it("prefers the words-exchanged line when there's enough text", () => {
    const f = funFact({ sessions: 5, messages: 100, userMessages: 50, toolCalls: 10, words: 500_000, cost: 5, activeDays: 4 });
    expect(f).toContain("words exchanged");
    expect(f).toContain("per session");
  });
  it("falls back to tool activity when words are sparse", () => {
    const f = funFact({ sessions: 3, messages: 20, userMessages: 10, toolCalls: 42, words: 10, cost: 1, activeDays: 2 });
    expect(f).toContain("42 tool calls");
  });
  it("is null when there's nothing worth bragging about", () => {
    expect(funFact({ sessions: 0, messages: 0, userMessages: 0, toolCalls: 0, words: 0, cost: 0, activeDays: 0 })).toBeNull();
  });
});

describe("formatters", () => {
  it("fmtCompact scales magnitudes", () => {
    expect(fmtCompact(9_600_000)).toBe("9.6M");
    expect(fmtCompact(12_271)).toBe("12k");
    expect(fmtCompact(980)).toBe("980");
  });
  it("fmtInt groups thousands", () => {
    expect(fmtInt(12271)).toBe("12,271");
  });
  it("fmtCost handles zero, sub-cent, and normal", () => {
    expect(fmtCost(0)).toBe("$0");
    expect(fmtCost(0.004)).toBe("<$0.01");
    expect(fmtCost(12.5)).toBe("$12.50");
  });
  it("hourLabel renders 12-hour clock", () => {
    expect(hourLabel(0)).toBe("12 AM");
    expect(hourLabel(2)).toBe("2 AM");
    expect(hourLabel(14)).toBe("2 PM");
    expect(hourLabel(null)).toBe("—");
  });
  it("modelLabel maps known ids and passes through local ones", () => {
    expect(modelLabel("opus")).toBe("Opus 5");
    expect(modelLabel("qwen3-coder:30b")).toBe("qwen3-coder:30b");
  });
});
