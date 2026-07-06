import { describe, it, expect } from "vitest";
import { buildSaveRecord } from "./persistence";

// buildSaveRecord's workspaceRoot resolution is the regression surface: a tab's
// saved folder must come from the tab's OWN root, else the GLOBAL workspace
// default — NEVER the focused pane's root (`activeRoot`), which would misfile a
// background/unfiled tab under an unrelated project. These build a minimal host
// + tab shaped like the structural PersistenceHost / SaveableTab subsets the
// function actually reads.

function host(over: Record<string, unknown> = {}) {
  return {
    // Fields buildSaveRecord touches; activeRoot is deliberately DIFFERENT from
    // workspaceCurrent so a leak is visible.
    model: "sonnet",
    activeRoot: "C:/proj/FOCUSED",
    workspaceCurrent: "C:/proj/GLOBAL",
    ...over,
  } as unknown as Parameters<typeof buildSaveRecord>[0];
}

function tab(over: Record<string, unknown> = {}) {
  return {
    messages: [{ role: "user", blocks: [{ type: "text", text: "hi" }] }],
    convoTitle: "t",
    convoCreatedAt: 1,
    lastActivityAt: 2,
    cliSessionId: "sid",
    modelOverride: null,
    lastTurnUsage: null,
    workspaceRoot: null,
    ...over,
  } as unknown as Parameters<typeof buildSaveRecord>[2];
}

describe("buildSaveRecord — workspaceRoot resolution", () => {
  it("uses the tab's own root when set", () => {
    const rec = buildSaveRecord(host(), "c1", tab({ workspaceRoot: "C:/proj/OWN" }));
    expect(rec.workspaceRoot).toBe("C:/proj/OWN");
  });

  it("falls back to the GLOBAL default, not the focused pane's root", () => {
    const rec = buildSaveRecord(host(), "c1", tab({ workspaceRoot: null }));
    expect(rec.workspaceRoot).toBe("C:/proj/GLOBAL");
    expect(rec.workspaceRoot).not.toBe("C:/proj/FOCUSED");
  });

  it("stays unfiled (null) when the tab has no root and no global default", () => {
    const rec = buildSaveRecord(host({ workspaceCurrent: null }), "c1", tab({ workspaceRoot: null }));
    expect(rec.workspaceRoot).toBeNull();
  });
});
