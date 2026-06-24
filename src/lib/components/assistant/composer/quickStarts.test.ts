import { describe, expect, it } from "vitest";
import { detectStack, quickStartsFor } from "./quickStarts";

describe("detectStack", () => {
  it("falls back to generic when no marker is present", () => {
    expect(detectStack(["README.md", "src/main.css"])).toBe("generic");
    expect(detectStack([])).toBe("generic");
  });

  it("detects each single-language marker at the workspace root", () => {
    expect(detectStack(["fxmanifest.lua"])).toBe("fivem");
    expect(detectStack(["package.json"])).toBe("node");
    expect(detectStack(["Cargo.toml"])).toBe("rust");
    expect(detectStack(["pyproject.toml"])).toBe("python");
    expect(detectStack(["go.mod"])).toBe("go");
  });

  it("detects python via any of its three markers", () => {
    expect(detectStack(["requirements.txt"])).toBe("python");
    expect(detectStack(["setup.py"])).toBe("python");
  });

  it("is case-insensitive and matches markers one level deep", () => {
    expect(detectStack(["FXManifest.lua"])).toBe("fivem");
    expect(detectStack(["server/resources/foo/fxmanifest.lua"])).toBe("fivem");
    expect(detectStack(["frontend/package.json"])).toBe("node");
  });

  it("does NOT match a marker name as a path substring", () => {
    // 'mypackage.json' or 'cargo.toml.bak' must not register.
    expect(detectStack(["src/mypackage.json"])).toBe("generic");
    expect(detectStack(["Cargo.toml.bak"])).toBe("generic");
  });

  it("prefers FiveM over the Lua/Node overlap", () => {
    expect(detectStack(["fxmanifest.lua", "package.json"])).toBe("fivem");
  });

  it("prefers Node over Rust for a Tauri/Electron-style dual-marker project", () => {
    expect(detectStack(["package.json", "src-tauri/Cargo.toml"])).toBe("node");
  });
});

describe("quickStartsFor", () => {
  it("returns three chips for every stack", () => {
    for (const files of [[], ["fxmanifest.lua"], ["package.json"], ["Cargo.toml"], ["go.mod"], ["pyproject.toml"]]) {
      expect(quickStartsFor(files)).toHaveLength(3);
    }
  });

  it("returns stack-specific prompts, not the generic set", () => {
    const rust = quickStartsFor(["Cargo.toml"]);
    expect(rust.some((q) => q.prompt.includes("Cargo.toml"))).toBe(true);
    expect(rust.map((q) => q.title)).toContain("Map the crate");
  });
});
