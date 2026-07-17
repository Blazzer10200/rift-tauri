// Unified settings-panel row builder + provider effort caps — the pure glue
// both Composer (keyboard nav) and SettingsMenu (render) derive from. These
// lock the row ORDER contract; a drift here desyncs cursor from pixels.
import { describe, it, expect } from "vitest";
import {
  settingsRowsFor,
  providerEffortCaps,
  dialStopsFor,
  MODEL_OPTIONS,
  DIAL_STOPS,
} from "./modelMatrix";

describe("providerEffortCaps", () => {
  it("effort-capable provider gets the full ladder", () => {
    const caps = providerEffortCaps(true);
    expect(caps).toEqual({ effort: true, maxEffort: "ultra" });
    expect(dialStopsFor(caps)).toEqual(DIAL_STOPS);
  });
  it("non-capable provider hides the ladder", () => {
    expect(providerEffortCaps(false)).toBeUndefined();
    expect(dialStopsFor(undefined)).toEqual([]);
  });
});

describe("settingsRowsFor", () => {
  it("Claude mode: model rows, then effort, then provider switch rows", () => {
    const rows = settingsRowsFor({
      providerMode: false,
      providerModels: [],
      providerIds: ["kimi", "ollama"],
      dialApplies: true,
    });
    expect(rows.slice(0, MODEL_OPTIONS.length).every((r) => r.kind === "model")).toBe(true);
    expect(rows[MODEL_OPTIONS.length]).toEqual({ kind: "effort" });
    expect(rows.slice(MODEL_OPTIONS.length + 1)).toEqual([
      { kind: "provider", id: "kimi" },
      { kind: "provider", id: "ollama" },
    ]);
  });

  it("provider mode: pmodel rows for the active provider only", () => {
    const rows = settingsRowsFor({
      providerMode: true,
      providerModels: ["kimi-k3", "kimi-k2.7-code"],
      providerIds: ["kimi", "ollama"],
      dialApplies: true,
    });
    expect(rows).toEqual([
      { kind: "pmodel", id: "kimi-k3" },
      { kind: "pmodel", id: "kimi-k2.7-code" },
      { kind: "effort" },
    ]);
  });

  it("no effort row when the dial doesn't apply", () => {
    const rows = settingsRowsFor({
      providerMode: true,
      providerModels: ["llama3"],
      providerIds: ["ollama"],
      dialApplies: false,
    });
    expect(rows).toEqual([{ kind: "pmodel", id: "llama3" }]);
  });

  it("Claude mode without saved providers has no provider section", () => {
    const rows = settingsRowsFor({
      providerMode: false,
      providerModels: [],
      providerIds: [],
      dialApplies: false,
    });
    expect(rows).toEqual(MODEL_OPTIONS.map((m) => ({ kind: "model", model: m })));
  });
});
