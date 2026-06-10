import { describe, expect, it } from "vitest";
import { leafName, prettyPath, shortK } from "./helpers";

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

describe("shortK", () => {
  it("formats each magnitude tier", () => {
    expect(shortK(999)).toBe("999");
    expect(shortK(1_500)).toBe("1.5K");
    expect(shortK(25_400)).toBe("25K");
    expect(shortK(2_345_678)).toBe("2.35M");
  });
});
