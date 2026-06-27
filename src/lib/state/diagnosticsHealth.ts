/**
 * Per-subsystem health roll-up (Phase 3) — derived from the live diag event
 * ring, not a second backend stream. The Phase-2 structured events already
 * carry typed `fields` (dur_ms / ok / outcome / reason / certs_loaded …), so a
 * green/amber/red verdict per subsystem is a pure fold over the events the
 * console already holds. Kept out of the .svelte.ts store so it can be unit
 * tested in isolation (matches the project's pure-helper convention).
 */
import type { DiagEvent } from "./diagnostics.svelte";

export type HealthLevel = "ok" | "warn" | "bad" | "idle";

export type SubsystemHealth = {
  /** Resource key as it appears on events ("warm_pool", "update", …). */
  key: string;
  /** Display label. */
  label: string;
  level: HealthLevel;
  /** One-line human summary of the current state. */
  detail: string;
  /** Events seen for this subsystem in the ring (sample size for the verdict). */
  count: number;
};

/** The subsystems we roll up, in display order, with their event resource key. */
const SUBSYSTEMS: { key: string; label: string }[] = [
  { key: "warm_pool", label: "Warm pool" },
  { key: "update", label: "Updates" },
  { key: "mcp", label: "MCP tools" },
  { key: "bridge", label: "Bridge" },
  { key: "stt", label: "Speech" },
  { key: "usage", label: "Usage" },
  { key: "certs", label: "Certs" },
  { key: "frontend", label: "Frontend" },
];

const LEVEL_RANK: Record<HealthLevel, number> = { idle: 0, ok: 1, warn: 2, bad: 3 };

function fieldNum(e: DiagEvent, k: string): number | undefined {
  const f = e.fields as Record<string, unknown> | null;
  const v = f?.[k];
  return typeof v === "number" ? v : undefined;
}
function fieldStr(e: DiagEvent, k: string): string | undefined {
  const f = e.fields as Record<string, unknown> | null;
  const v = f?.[k];
  return typeof v === "string" ? v : undefined;
}
function fieldBool(e: DiagEvent, k: string): boolean | undefined {
  const f = e.fields as Record<string, unknown> | null;
  const v = f?.[k];
  return typeof v === "boolean" ? v : undefined;
}

function median(xs: number[]): number | undefined {
  if (!xs.length) return undefined;
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor((s.length - 1) / 2)];
}

/**
 * Fold the event ring into one verdict per subsystem. Pure: same events in →
 * same verdicts out. A subsystem with no events is "idle" (no signal yet, not a
 * problem). Error-level events and tool/bridge failures drive amber/red; the
 * `detail` names the concrete reason so the strip is self-explaining.
 */
export function rollUpHealth(events: DiagEvent[]): SubsystemHealth[] {
  // Bucket events by resource once.
  const byKey = new Map<string, DiagEvent[]>();
  for (const e of events) {
    if (!e.resource) continue;
    const arr = byKey.get(e.resource);
    if (arr) arr.push(e);
    else byKey.set(e.resource, [e]);
  }

  return SUBSYSTEMS.map(({ key, label }) => {
    const evs = byKey.get(key) ?? [];
    if (evs.length === 0) {
      return { key, label, level: "idle" as HealthLevel, detail: "no events yet", count: 0 };
    }
    const errors = evs.filter((e) => e.level === "error").length;
    const warns = evs.filter((e) => e.level === "warn").length;

    // Start from the warn baseline, then let errors escalate: a single error is
    // amber (one transient failure), ≥2 is red (a pattern). Warn-level events
    // alone are amber; all-info is green.
    let level: HealthLevel = warns > 0 ? "warn" : "ok";
    if (errors >= 2) level = "bad";
    else if (errors === 1) level = "warn";

    return { key, label, level, detail: describe(key, evs), count: evs.length };
  });
}

/** Subsystem-specific summary line from its events (most recent wins). */
function describe(key: string, evs: DiagEvent[]): string {
  const last = evs[evs.length - 1];
  switch (key) {
    case "warm_pool": {
      const dispatches = evs.filter((e) => fieldStr(e, "outcome"));
      const hits = dispatches.filter((e) => fieldStr(e, "outcome") === "hit").length;
      if (!dispatches.length) return `${evs.length} events`;
      const rate = Math.round((hits / dispatches.length) * 100);
      return `${rate}% warm-hit (${dispatches.length} turns)`;
    }
    case "mcp":
    case "bridge": {
      const durs = evs.map((e) => fieldNum(e, "dur_ms")).filter((n): n is number => n != null);
      const fails = evs.filter((e) => fieldBool(e, "ok") === false).length;
      const p50 = median(durs);
      const dur = p50 != null ? `p50 ${p50}ms` : "";
      return fails > 0 ? `${fails} failed · ${dur}` : dur || `${evs.length} calls`;
    }
    case "stt": {
      const load = evs.find((e) => fieldStr(e, "event") === "model_load");
      if (load) {
        const ms = fieldNum(load, "load_ms");
        const be = fieldStr(load, "backend");
        return `model ready${ms != null ? ` (${ms}ms, ${be})` : ""}`;
      }
      return last.message;
    }
    case "usage": {
      const reason = fieldStr(last, "reason");
      return reason ? `state: ${reason}` : last.message;
    }
    case "certs": {
      const loaded = fieldNum(last, "certs_loaded");
      return loaded != null ? `${loaded} corporate root(s)` : last.message;
    }
    case "update": {
      const stage = fieldStr(last, "stage");
      return stage ? `last: ${stage}` : last.message;
    }
    case "frontend": {
      const errs = evs.filter((e) => e.level === "error").length;
      return errs > 0 ? `${errs} JS error(s)` : `${evs.length} events`;
    }
    default:
      return `${evs.length} events`;
  }
}

/** Worst level across all subsystems — drives a single console-header dot. */
export function overallHealth(rolled: SubsystemHealth[]): HealthLevel {
  let worst: HealthLevel = "idle";
  for (const s of rolled) {
    if (LEVEL_RANK[s.level] > LEVEL_RANK[worst]) worst = s.level;
  }
  return worst;
}
