import { describe, expect, it } from "vitest";
import { leafName, prettyPath, shortPath } from "./path";

describe("leafName", () => {
  it("returns the last segment of a posix or windows path", () => {
    expect(leafName("/home/user/project")).toBe("project");
    expect(leafName("C:\\AI Workflow\\projects\\rift-tauri")).toBe("rift-tauri");
  });
  it("tolerates a trailing slash and a bare name", () => {
    expect(leafName("/home/user/project/")).toBe("project");
    expect(leafName("rift")).toBe("rift");
  });
});

describe("prettyPath", () => {
  it("strips windows extended-length prefixes", () => {
    expect(prettyPath("\\\\?\\C:\\Users\\x")).toBe("C:\\Users\\x");
    expect(prettyPath("//?/C:/Users/x")).toBe("C:/Users/x");
  });
  it("leaves a normal path untouched", () => {
    expect(prettyPath("C:\\Users\\x")).toBe("C:\\Users\\x");
  });
});

describe("shortPath", () => {
  it("keeps the last two segments, ellipsis-prefixed", () => {
    expect(shortPath("/home/user/projects/rift")).toBe("…/projects/rift");
    expect(shortPath("C:\\AI Workflow\\projects\\rift")).toBe("…/projects/rift");
  });
  it("returns short paths unchanged", () => {
    expect(shortPath("/home")).toBe("/home");
    expect(shortPath("a/b")).toBe("a/b");
    expect(shortPath("/home/user/project/")).toBe("…/user/project");
  });
});
