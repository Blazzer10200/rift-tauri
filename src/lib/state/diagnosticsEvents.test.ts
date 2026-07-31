import { describe, expect, it } from "vitest";
import type { DiagEvent } from "./diagnostics.svelte.js";
import { mergeDiagEvents } from "./diagnosticsEvents.js";

function event(seq: number): DiagEvent {
  return {
    at: "2026-07-31T00:00:00.000Z",
    seq,
    stage: "log",
    level: "info",
    resource: null,
    file: null,
    message: `event ${seq}`,
    fields: {},
  };
}

describe("mergeDiagEvents", () => {
  it("deduplicates backlog events that also arrive through the live queue", () => {
    const backlog = mergeDiagEvents([], [event(1), event(2), event(3)], 10);
    const live = mergeDiagEvents(backlog.events, [event(3), event(4)], 10);

    expect(live.events.map((entry) => entry.seq)).toEqual([1, 2, 3, 4]);
    expect(live.dropped).toBe(0);
  });

  it("counts only unique events dropped from the bounded ring", () => {
    const result = mergeDiagEvents([event(1), event(2)], [event(2), event(3), event(4)], 3);

    expect(result.events.map((entry) => entry.seq)).toEqual([2, 3, 4]);
    expect(result.dropped).toBe(1);
  });
});
