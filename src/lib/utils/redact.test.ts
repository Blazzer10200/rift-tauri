import { describe, it, expect } from "vitest";

import { scrubUser } from "./redact.js";

describe("scrubUser", () => {
  it("redacts the Windows username, keeping the rest of the path", () => {
    expect(scrubUser("C:\\Users\\BLAZZER\\AppData\\Local\\Temp")).toBe(
      "C:\\Users\\<user>\\AppData\\Local\\Temp",
    );
    // Forward-slash Windows paths (as Tauri / web layers often emit) too.
    expect(scrubUser("C:/Users/BLAZZER/Documents")).toBe("C:/Users/<user>/Documents");
    // Lower-case drive letter still matches.
    expect(scrubUser("d:\\Users\\jane\\x")).toBe("d:\\Users\\<user>\\x");
  });

  it("redacts Linux and macOS home paths", () => {
    expect(scrubUser("/home/alice/project/file.rs")).toBe("/home/<user>/project/file.rs");
    expect(scrubUser("/Users/bob/dev/app")).toBe("/Users/<user>/dev/app");
  });

  it("redacts every occurrence in one string", () => {
    expect(scrubUser("C:\\Users\\BLAZZER\\a and C:\\Users\\BLAZZER\\b")).toBe(
      "C:\\Users\\<user>\\a and C:\\Users\\<user>\\b",
    );
  });

  it("leaves strings without a home path untouched", () => {
    expect(scrubUser("just some text, no path")).toBe("just some text, no path");
    expect(scrubUser("C:\\Windows\\System32")).toBe("C:\\Windows\\System32");
  });

  it("returns an empty string for null/undefined/empty", () => {
    expect(scrubUser(null)).toBe("");
    expect(scrubUser(undefined)).toBe("");
    expect(scrubUser("")).toBe("");
  });
});
