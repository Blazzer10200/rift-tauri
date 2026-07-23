// Pure aggregation for the Home stats dashboard. The backend (`assistant_stats`)
// hands us one lightweight row per saved conversation; everything time-bucketed
// (active days, streaks, peak hour, heatmap) is computed HERE so it lands in the
// user's LOCAL timezone. Zero state, zero IPC — unit-tested in statsHelpers.test.ts.

export type ConvoStat = {
  updatedAt: number;
  createdAt: number;
  model: string;
  messages: number;
  userMessages: number;
  toolCalls: number;
  words: number;
  costUsd: number;
};

export type StatRange = "all" | "30d" | "7d";

export type Totals = {
  sessions: number;
  messages: number;
  userMessages: number;
  toolCalls: number;
  words: number;
  cost: number;
  activeDays: number;
};

export type ModelSlice = {
  model: string;
  label: string;
  messages: number;
  toolCalls: number;
  cost: number;
  share: number; // 0..1 by message share
};

export type DayCell = {
  day: number; // local epoch-day index
  ms: number; // local midnight, for labels
  messages: number;
  toolCalls: number;
  cost: number;
  sessions: number;
};

const DAY_MS = 86_400_000;

/** Local-midnight epoch-day index — stable integer per calendar day in the
 *  user's timezone, so consecutive days differ by exactly 1 (streak math). */
export function localDayIndex(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return Math.round(d.getTime() / DAY_MS);
}

/** ms threshold for a range window (inclusive lower bound). "all" → -Infinity. */
function rangeStart(range: StatRange, now: number): number {
  if (range === "7d") return now - 7 * DAY_MS;
  if (range === "30d") return now - 30 * DAY_MS;
  return -Infinity;
}

export function filterRange(stats: ConvoStat[], range: StatRange, now: number): ConvoStat[] {
  if (range === "all") return stats;
  const from = rangeStart(range, now);
  return stats.filter((s) => s.updatedAt >= from);
}

export function summarize(stats: ConvoStat[]): Totals {
  const days = new Set<number>();
  const t: Totals = { sessions: stats.length, messages: 0, userMessages: 0, toolCalls: 0, words: 0, cost: 0, activeDays: 0 };
  for (const s of stats) {
    t.messages += s.messages;
    t.userMessages += s.userMessages;
    t.toolCalls += s.toolCalls;
    t.words += s.words;
    t.cost += s.costUsd;
    days.add(localDayIndex(s.updatedAt));
  }
  t.activeDays = days.size;
  return t;
}

/** Distinct active local days, as a sorted ascending array of day indices. */
function activeDaySet(stats: ConvoStat[]): number[] {
  const set = new Set<number>();
  for (const s of stats) set.add(localDayIndex(s.updatedAt));
  return [...set].sort((a, b) => a - b);
}

/** Current streak counts consecutive active days ending today OR yesterday
 *  (so a day you haven't opened Rift yet doesn't break a live streak); longest
 *  is the max consecutive run anywhere in history. */
export function streaks(stats: ConvoStat[], now: number): { current: number; longest: number } {
  const days = activeDaySet(stats);
  if (days.length === 0) return { current: 0, longest: 0 };
  const set = new Set(days);

  const today = localDayIndex(now);
  let current = 0;
  let anchor = set.has(today) ? today : set.has(today - 1) ? today - 1 : null;
  if (anchor != null) {
    let d = anchor;
    while (set.has(d)) {
      current++;
      d--;
    }
  }

  let longest = 1;
  let run = 1;
  for (let i = 1; i < days.length; i++) {
    run = days[i] === days[i - 1] + 1 ? run + 1 : 1;
    if (run > longest) longest = run;
  }
  return { current, longest };
}

/** Hour-of-day (0–23, local) with the most message activity, weighted by
 *  message count. null when there's nothing to rank. */
export function peakHour(stats: ConvoStat[]): number | null {
  const buckets = new Array(24).fill(0);
  let any = false;
  for (const s of stats) {
    if (s.messages <= 0) continue;
    buckets[new Date(s.updatedAt).getHours()] += s.messages;
    any = true;
  }
  if (!any) return null;
  let best = 0;
  for (let h = 1; h < 24; h++) if (buckets[h] > buckets[best]) best = h;
  return best;
}

export function perModel(stats: ConvoStat[]): ModelSlice[] {
  const map = new Map<string, ModelSlice>();
  let totalMsgs = 0;
  for (const s of stats) {
    const key = s.model || "unknown";
    const slice = map.get(key) ?? { model: key, label: modelLabel(key), messages: 0, toolCalls: 0, cost: 0, share: 0 };
    slice.messages += s.messages;
    slice.toolCalls += s.toolCalls;
    slice.cost += s.costUsd;
    map.set(key, slice);
    totalMsgs += s.messages;
  }
  const out = [...map.values()];
  for (const s of out) s.share = totalMsgs > 0 ? s.messages / totalMsgs : 0;
  out.sort((a, b) => b.messages - a.messages || b.cost - a.cost);
  return out;
}

export function topModel(stats: ConvoStat[]): string | null {
  const m = perModel(stats);
  return m.length > 0 && m[0].messages > 0 ? m[0].label : null;
}

/** Dense per-day series for a column bar chart — last `days` calendar days
 *  ending today, gap-filled with zeros, oldest→newest. A flat run (no weekday
 *  padding) meant to be drawn as side-by-side bars. */
export function dailySeries(stats: ConvoStat[], days: number, now: number): { cells: DayCell[]; max: number } {
  const today = localDayIndex(now);
  const first = today - (days - 1);
  const byDay = new Map<number, DayCell>();
  for (let d = first; d <= today; d++) {
    byDay.set(d, { day: d, ms: d * DAY_MS, messages: 0, toolCalls: 0, cost: 0, sessions: 0 });
  }
  for (const s of stats) {
    const cell = byDay.get(localDayIndex(s.updatedAt));
    if (!cell) continue;
    cell.messages += s.messages;
    cell.toolCalls += s.toolCalls;
    cell.cost += s.costUsd;
    cell.sessions += 1;
  }
  const cells = [...byDay.values()];
  let max = 0;
  for (const c of cells) if (c.messages > max) max = c.messages;
  return { cells, max };
}

/** Short local date label for a day cell ("Jun 3", "Mon"). `weekday` form is
 *  used when the series is short enough that weekday is more legible than date. */
export function dayLabel(ms: number, weekday = false): string {
  const d = new Date(ms);
  return weekday
    ? d.toLocaleDateString("en-US", { weekday: "short" })
    : d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

/** 5-step intensity (0 empty … 4 hot) for a heatmap cell, log-ish so a few
 *  monster days don't wash out everything else. */
export function intensity(value: number, max: number): 0 | 1 | 2 | 3 | 4 {
  if (value <= 0 || max <= 0) return 0;
  const r = Math.log1p(value) / Math.log1p(max);
  if (r > 0.75) return 4;
  if (r > 0.5) return 3;
  if (r > 0.25) return 2;
  return 1;
}

/** One honest highlight line. Leads with the real words-exchanged figure (we
 *  count words for real) + a plain per-session average, falls back to tool
 *  activity, else null. */
export function funFact(t: Totals): string | null {
  if (t.words >= 2000) {
    const perSession = t.sessions > 0 ? Math.round(t.words / t.sessions) : 0;
    const avg = perSession > 0 ? ` · ~${fmtCompact(perSession)} per session` : "";
    return `≈ ${fmtCompact(t.words)} words exchanged${avg}.`;
  }
  if (t.toolCalls > 0) {
    return `${fmtInt(t.toolCalls)} tool ${t.toolCalls === 1 ? "call" : "calls"} run across ${fmtInt(t.sessions)} ${t.sessions === 1 ? "session" : "sessions"}.`;
  }
  return null;
}

/** One human sentence summarizing the range — sits under the hero number so the
 *  panel reads as a story, not a spreadsheet. Drops empty clauses gracefully. */
export function summaryLine(t: Totals, peak: number | null): string {
  const parts: string[] = [];
  parts.push(`${fmtInt(t.sessions)} session${t.sessions === 1 ? "" : "s"}`);
  if (t.activeDays > 0) parts.push(`across ${fmtInt(t.activeDays)} active day${t.activeDays === 1 ? "" : "s"}`);
  if (peak != null) parts.push(`busiest around ${hourLabel(peak)}`);
  return parts.join(" · ");
}

// ── Formatters ──────────────────────────────────────────────────────────────

export function fmtInt(n: number): string {
  return Math.round(n).toLocaleString("en-US");
}

/** Compact magnitude: 9_600_000 → "9.6M", 12_271 → "12.3k", 980 → "980". */
export function fmtCompact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 0 : 1)}M`;
  if (n >= 10_000) return `${(n / 1_000).toFixed(0)}k`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return fmtInt(n);
}

export function fmtCost(n: number): string {
  if (n <= 0) return "$0";
  if (n < 0.01) return "<$0.01";
  if (n >= 1000) return `$${(n / 1000).toFixed(1)}k`;
  return `$${n.toFixed(2)}`;
}

export function hourLabel(h: number | null): string {
  if (h == null) return "—";
  if (h === 0) return "12 AM";
  if (h === 12) return "12 PM";
  return h < 12 ? `${h} AM` : `${h - 12} PM`;
}

const KNOWN_MODELS: Record<string, string> = {
  sonnet: "Sonnet 5",
  haiku: "Haiku 4.5",
  opus: "Opus 4.8",
  "claude-opus-4-7": "Opus 4.7",
  "claude-opus-4-6": "Opus 4.6",
  "claude-opus-4-5": "Opus 4.5",
  "claude-sonnet-4-6": "Sonnet 4.6",
  "claude-sonnet-4-5": "Sonnet 4.5",
  "claude-fable-5": "Fable 5",
};

export function modelLabel(id: string): string {
  if (KNOWN_MODELS[id]) return KNOWN_MODELS[id];
  if (!id || id === "unknown") return "Unknown";
  // Local/passthrough ids (e.g. "qwen3-coder:30b") render as-is.
  return id;
}
