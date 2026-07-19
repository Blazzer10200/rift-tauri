// #98.1 — the init-frame slash-command normalizer. The store itself is a thin
// $state wrapper; the validation/normalization is the part worth pinning.
import { describe, it, expect } from "vitest";
import { normalizeCliCommands } from "./cliCommands.svelte";

describe("normalizeCliCommands", () => {
  it("accepts plain names and strips a leading slash", () => {
    expect(normalizeCliCommands(["compact", "/code-review", "mcp"])).toEqual([
      "code-review",
      "compact",
      "mcp",
    ]);
  });

  it("dedups, sorts, and drops garbage entries", () => {
    expect(
      normalizeCliCommands(["b", "b", "/b", "a", 42, null, "has space", "-lead", "", "x".repeat(80)]),
    ).toEqual(["a", "b"]);
  });

  it("keeps namespaced and dotted names (frontend:build, v2.check)", () => {
    expect(normalizeCliCommands(["frontend:build", "v2.check", "git_ship"])).toEqual([
      "frontend:build",
      "git_ship",
      "v2.check",
    ]);
  });

  it("returns empty on non-array input (older CLI omits the field)", () => {
    expect(normalizeCliCommands(undefined)).toEqual([]);
    expect(normalizeCliCommands("nope")).toEqual([]);
    expect(normalizeCliCommands({})).toEqual([]);
  });
});
