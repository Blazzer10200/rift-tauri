// Unified settings-panel row builder — the pure glue both Composer (keyboard
// nav) and SettingsMenu (render) derive from. These lock the row ORDER
// contract; a drift here desyncs cursor from pixels.
import { describe, it, expect } from "vitest";
import {
  settingsRowsFor,
  MODEL_OPTIONS,
} from "./modelMatrix";

describe("settingsRowsFor", () => {
  it("model rows, then effort row when the dial applies", () => {
    const rows = settingsRowsFor({ dialApplies: true });
    expect(rows.slice(0, MODEL_OPTIONS.length).every((r) => r.kind === "model")).toBe(true);
    expect(rows[MODEL_OPTIONS.length]).toEqual({ kind: "effort" });
    expect(rows.length).toBe(MODEL_OPTIONS.length + 1);
  });

  it("no effort row when the dial doesn't apply", () => {
    const rows = settingsRowsFor({ dialApplies: false });
    expect(rows).toEqual(MODEL_OPTIONS.map((m) => ({ kind: "model", model: m })));
  });
});
