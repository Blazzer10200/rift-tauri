import { describe, it, expect } from "vitest";

import { rollUpHealth, overallHealth } from "./diagnosticsHealth.js";
import type { DiagEvent } from "./diagnostics.svelte.js";

// Minimal event factory — only the fields the roll-up reads.
function ev(
  resource: string,
  level: DiagEvent["level"],
  fields: Record<string, unknown> = {},
  message = "",
): DiagEvent {
  return {
    at: "2026-06-27T00:00:00.000Z",
    seq: 0,
    stage: "log",
    level,
    resource,
    file: null,
    message,
    fields,
  };
}

describe("rollUpHealth", () => {
  it("marks a subsystem with no events as idle", () => {
    const h = rollUpHealth([]);
    const wp = h.find((s) => s.key === "warm_pool")!;
    expect(wp.level).toBe("idle");
    expect(wp.count).toBe(0);
  });

  it("is ok when all events are info-level", () => {
    const h = rollUpHealth([
      ev("warm_pool", "info", { outcome: "hit" }),
      ev("warm_pool", "info", { outcome: "hit" }),
    ]);
    const wp = h.find((s) => s.key === "warm_pool")!;
    expect(wp.level).toBe("ok");
    expect(wp.detail).toContain("100% warm-hit");
  });

  it("computes warm-hit rate across mixed outcomes", () => {
    const h = rollUpHealth([
      ev("warm_pool", "info", { outcome: "hit" }),
      ev("warm_pool", "info", { outcome: "hit" }),
      ev("warm_pool", "info", { outcome: "cold" }),
      ev("warm_pool", "info", { outcome: "signature_drain" }),
    ]);
    const wp = h.find((s) => s.key === "warm_pool")!;
    expect(wp.detail).toContain("50% warm-hit");
  });

  it("escalates a single error to warn, multiple errors to bad", () => {
    const oneErr = rollUpHealth([ev("update", "error", { stage: "init" })]);
    expect(oneErr.find((s) => s.key === "update")!.level).toBe("warn");

    const twoErr = rollUpHealth([
      ev("update", "error", { stage: "init" }),
      ev("update", "error", { stage: "apply_sweep", ok: false }),
    ]);
    expect(twoErr.find((s) => s.key === "update")!.level).toBe("bad");
  });

  it("reports MCP p50 duration and failure count", () => {
    const h = rollUpHealth([
      ev("mcp", "info", { tool: "read_file", dur_ms: 10, ok: true }),
      ev("mcp", "info", { tool: "grep", dur_ms: 30, ok: true }),
      ev("mcp", "warn", { tool: "list_dir", dur_ms: 50, ok: false }),
    ]);
    const mcp = h.find((s) => s.key === "mcp")!;
    expect(mcp.detail).toContain("1 failed");
    expect(mcp.detail).toContain("p50 30ms");
    expect(mcp.level).toBe("warn"); // a warn-level fail, no error
  });

  it("surfaces STT model-load backend + duration", () => {
    const h = rollUpHealth([
      ev("stt", "info", { event: "model_load", load_ms: 1200, backend: "cuda", ok: true }),
    ]);
    expect(h.find((s) => s.key === "stt")!.detail).toContain("1200ms, cuda");
  });

  it("surfaces cert count and usage reason from the latest event", () => {
    const h = rollUpHealth([
      ev("certs", "info", { certs_loaded: 5, pem_written: true }),
      ev("usage", "debug", { reason: "expired" }),
    ]);
    expect(h.find((s) => s.key === "certs")!.detail).toContain("5 corporate root");
    expect(h.find((s) => s.key === "usage")!.detail).toContain("expired");
  });
});

describe("overallHealth", () => {
  it("is the worst level across subsystems", () => {
    const rolled = rollUpHealth([
      ev("warm_pool", "info", { outcome: "hit" }),
      ev("update", "error", { stage: "init" }),
      ev("update", "error", { stage: "apply_sweep", ok: false }),
    ]);
    expect(overallHealth(rolled)).toBe("bad");
  });

  it("is idle when nothing has been seen", () => {
    expect(overallHealth(rollUpHealth([]))).toBe("idle");
  });
});
