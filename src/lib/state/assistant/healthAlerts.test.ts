import { describe, it, expect } from "vitest";
import { ctxAlertTransition } from "./healthAlerts";

describe("ctxAlertTransition", () => {
  it("stays quiet below the warn threshold", () => {
    expect(ctxAlertTransition(0, false)).toEqual({ fire: false, latched: false });
    expect(ctxAlertTransition(84.9, false)).toEqual({ fire: false, latched: false });
  });

  it("fires once at ≥85% and latches", () => {
    expect(ctxAlertTransition(85, false)).toEqual({ fire: true, latched: true });
    expect(ctxAlertTransition(99, false)).toEqual({ fire: true, latched: true });
  });

  it("never re-fires while latched above the re-arm floor", () => {
    expect(ctxAlertTransition(92, true)).toEqual({ fire: false, latched: true });
    expect(ctxAlertTransition(70, true)).toEqual({ fire: false, latched: true });
  });

  it("re-arms after compaction drops usage under 70%", () => {
    expect(ctxAlertTransition(31, true)).toEqual({ fire: false, latched: false });
    // next climb past 85 fires again
    expect(ctxAlertTransition(86, false)).toEqual({ fire: true, latched: true });
  });
});
