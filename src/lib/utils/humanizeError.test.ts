import { describe, it, expect } from "vitest";
import { humanizeError } from "./humanizeError";

describe("humanizeError", () => {
  it("maps timeout shapes to friendly guidance", () => {
    expect(humanizeError("operation timed out after 90s")).toMatch(/timed out/i);
    expect(humanizeError(new Error("deadline exceeded"))).toMatch(/timed out/i);
  });

  it("maps TLS/cert failures to a proxy hint", () => {
    expect(humanizeError("self-signed certificate in chain")).toMatch(/security check|proxy/i);
  });

  it("maps locked-file errors", () => {
    expect(humanizeError("The process cannot access the file because it is being used by another process (os error 32)"))
      .toMatch(/in use by another program/i);
  });

  it("maps auth failures", () => {
    expect(humanizeError("HTTP 401 Unauthorized")).toMatch(/sign in/i);
  });

  it("strips ANSI + wrapper prefixes and scrubs the username from passthrough text", () => {
    const out = humanizeError("Error: \x1b[31mboom\x1b[0m at C:\\Users\\someuser\\app");
    expect(out).not.toContain("\x1b");
    expect(out).not.toMatch(/^Error:/);
    expect(out).toContain("<user>");
    expect(out).not.toContain("someuser");
  });

  it("passes unknown errors through, bounded", () => {
    const long = "weird-" + "x".repeat(400);
    const out = humanizeError(long);
    expect(out.length).toBeLessThanOrEqual(241);
    expect(out.endsWith("…")).toBe(true);
  });

  it("handles null / non-string inputs", () => {
    expect(humanizeError(null)).toBe("An unexpected error occurred.");
    expect(humanizeError({ message: "plain object message" })).toBe("plain object message");
  });
});
