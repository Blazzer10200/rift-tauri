import { describe, expect, it } from "vitest";
import { projectHue } from "./projectHue";

describe("projectHue", () => {
  it("is stable for the same name", () => {
    expect(projectHue("rift-tauri")).toBe(projectHue("rift-tauri"));
  });

  it("stays in [0, 360)", () => {
    for (const n of ["rgb-orchestrator", "rift-tauri", "exfil-v2", "Discord-Bot", "", "·"]) {
      const h = projectHue(n);
      expect(h).toBeGreaterThanOrEqual(0);
      expect(h).toBeLessThan(360);
    }
  });

  it("separates the current project set", () => {
    const hues = ["rgb-orchestrator", "rift-tauri", "exfil-v2", "Discord-Bot"].map(projectHue);
    expect(new Set(hues).size).toBe(hues.length);
  });
});
