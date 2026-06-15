import { describe, expect, it } from "vitest";
import { leafName, prettyPath, shortK, shortPath } from "./helpers";

describe("leafName", () => {
  it("returns the last segment for unix and windows paths", () => {
    expect(leafName("/home/user/project")).toBe("project");
    expect(leafName("C:\\AI Workflow\\projects\\rift-tauri")).toBe("rift-tauri");
  });
  it("ignores a trailing slash and survives bare names", () => {
    expect(leafName("/home/user/project/")).toBe("project");
    expect(leafName("rift")).toBe("rift");
  });
});

describe("prettyPath", () => {
  it("strips the Windows extended-length prefix in both spellings", () => {
    expect(prettyPath("\\\\?\\C:\\Users\\x")).toBe("C:\\Users\\x");
    expect(prettyPath("//?/C:/Users/x")).toBe("C:/Users/x");
  });
  it("leaves normal paths alone", () => {
    expect(prettyPath("C:\\Users\\x")).toBe("C:\\Users\\x");
  });
});

describe("shortPath", () => {
  it("keeps the last two segments with an ellipsis prefix", () => {
    expect(shortPath("/home/user/projects/rift")).toBe("…/projects/rift");
    expect(shortPath("C:\\AI Workflow\\projects\\rift")).toBe("…/projects/rift");
  });
  it("returns ≤2-segment paths unchanged and respects trailing slashes", () => {
    expect(shortPath("/home")).toBe("/home");
    expect(shortPath("a/b")).toBe("a/b");
    expect(shortPath("/home/user/project/")).toBe("…/user/project");
  });
});

describe("shortK", () => {
  it("formats each magnitude tier", () => {
    expect(shortK(999)).toBe("999");
    expect(shortK(1_500)).toBe("1.5K");
    expect(shortK(25_400)).toBe("25K");
    expect(shortK(2_345_678)).toBe("2.35M");
  });
});
