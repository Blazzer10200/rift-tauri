import { describe, expect, it } from "vitest";
import { leafName, shortK } from "./helpers";

// Path helpers (leafName/shortPath/prettyPath) now live in utils/path.ts and are
// fully tested there; one smoke check that the re-export chain resolves.
describe("path re-exports", () => {
  it("re-exports leafName from utils/path", () => {
    expect(leafName("C:\\a\\b")).toBe("b");
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
