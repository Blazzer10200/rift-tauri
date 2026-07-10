import { describe, expect, it } from "vitest";
import { mcpHint, mergeMcpRows, statusMeta } from "./mcpStatus";
import type { HarnessMcpRow, MergedMcpRow } from "./mcpStatus";

const h = (name: string, status: string): HarnessMcpRow => ({
  name,
  target: "http://x",
  transport: null,
  status,
  detail: null,
});

const m = (name: string, status: string, live = false): MergedMcpRow => ({
  name,
  status,
  live,
  target: null,
  transport: null,
  detail: null,
});

describe("mergeMcpRows", () => {
  it("keeps harness rows with their health-checked status and display fields", () => {
    const rows = mergeMcpRows([h("gmail", "connected"), h("search", "needs-approval")], null);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({ name: "gmail", status: "connected", live: false, target: "http://x" });
    expect(rows[1].status).toBe("needs-approval");
  });

  it("session status wins over the harness row with the same name", () => {
    // Terminal says connected, but inside Rift's headless CLI the connector
    // sits needs-auth — the chat's own view is the honest one.
    const rows = mergeMcpRows([h("claude.ai Gmail", "connected")], [
      { name: "claude.ai Gmail", status: "needs-auth" },
    ]);
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({ status: "needs-auth", live: true, target: "http://x" });
  });

  it("session-only rift row is added at the front, live, with a detail blurb", () => {
    const rows = mergeMcpRows([h("gmail", "connected")], [{ name: "rift", status: "connected" }]);
    expect(rows[0]).toMatchObject({ name: "rift", live: true, target: null });
    expect(rows[0].detail).toContain("workspace tools");
    expect(rows).toHaveLength(2);
  });
});

describe("statusMeta", () => {
  it("maps both layers' vocabularies to label + tint", () => {
    expect(statusMeta("connected")).toEqual({ label: "Connected", tint: "ok" });
    expect(statusMeta("needs-auth").tint).toBe("warn");
    expect(statusMeta("needs-approval").tint).toBe("warn");
    expect(statusMeta("failed").tint).toBe("danger");
    expect(statusMeta("pending").tint).toBe("muted");
    expect(statusMeta("disabled").tint).toBe("muted");
  });

  it("unknown statuses render as Configured instead of being dropped", () => {
    expect(statusMeta("half-open")).toEqual({ label: "Configured", tint: "muted" });
  });
});

describe("mcpHint", () => {
  it("failed outranks approval outranks sign-in", () => {
    expect(
      mcpHint([m("a", "failed"), m("b", "needs-approval"), m("c", "needs-auth")]),
    ).toContain("health check");
    expect(mcpHint([m("b", "needs-approval"), m("c", "needs-auth")])).toContain("one-time approval");
    expect(mcpHint([m("c", "needs-auth")])).toContain("Sign-in");
  });

  it("healthy roster yields no hint", () => {
    expect(mcpHint([m("a", "connected")])).toBeNull();
    expect(mcpHint([])).toBeNull();
  });
});
