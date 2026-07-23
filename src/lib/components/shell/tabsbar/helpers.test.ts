import { describe, expect, it } from "vitest";
import { leafName } from "./helpers";

// Path helpers (leafName/shortPath/prettyPath) now live in utils/path.ts and are
// fully tested there; one smoke check that the re-export chain resolves.
describe("path re-exports", () => {
  it("re-exports leafName from utils/path", () => {
    expect(leafName("C:\\a\\b")).toBe("b");
  });
});
