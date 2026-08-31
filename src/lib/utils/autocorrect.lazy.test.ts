import { describe, expect, it } from "vitest";
import {
  correctWord,
  fuzzyAutocorrectReady,
  warmAutocorrectDictionary,
} from "./autocorrect";

describe("lazy fuzzy-autocorrect dictionary", () => {
  it("keeps exact fixes synchronous while fuzzy correction waits for readiness", async () => {
    expect(fuzzyAutocorrectReady()).toBe(false);
    expect(correctWord("teh")).toBe("the");
    expect(correctWord("responsibilty")).toBeNull();

    await warmAutocorrectDictionary();

    expect(fuzzyAutocorrectReady()).toBe(true);
    expect(correctWord("responsibilty")).toBe("responsibility");
  });

  it("deduplicates concurrent warm-up calls", async () => {
    const first = warmAutocorrectDictionary();
    const second = warmAutocorrectDictionary();
    expect(second).toBe(first);
    await expect(Promise.all([first, second])).resolves.toEqual([undefined, undefined]);
  });
});
