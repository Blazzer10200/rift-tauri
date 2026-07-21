import { describe, expect, it } from "vitest";
import { boundaryAutocorrect, correctWord, isBoundaryChar } from "./autocorrect";

const apply = (value: string, caret: number): string | null => {
  const fix = boundaryAutocorrect(value, caret);
  if (!fix) return null;
  return value.slice(0, fix.start) + fix.replacement + value.slice(fix.end);
};

describe("correctWord", () => {
  it("fixes dictionary typos and preserves capitalization shape", () => {
    expect(correctWord("teh")).toBe("the");
    expect(correctWord("Teh")).toBe("The");
    expect(correctWord("TEH")).toBe("THE");
    expect(correctWord("dont")).toBe("don't");
    expect(correctWord("im")).toBe("I'm");
    expect(correctWord("i")).toBe("I");
  });

  it("returns null for words that need no change", () => {
    expect(correctWord("the")).toBeNull();
    expect(correctWord("rust")).toBeNull();
    expect(correctWord("I")).toBeNull();
  });
});

describe("boundaryAutocorrect", () => {
  it("fixes the word just finished by a space", () => {
    const v = "fix teh ";
    expect(apply(v, v.length)).toBe("fix the ");
  });

  it("fires on punctuation boundaries too", () => {
    const v = "is that wierd?";
    expect(apply(v, v.length)).toBe("is that weird?");
  });

  it("capitalizes standalone i", () => {
    const v = "well i ";
    expect(apply(v, v.length)).toBe("well I ");
  });

  it("no-ops when the finished word is clean", () => {
    const v = "all good here ";
    expect(apply(v, v.length)).toBeNull();
  });

  it("never fires inside a slash command", () => {
    const v = "/diag im ";
    expect(apply(v, v.length)).toBeNull();
  });

  it("never fires on words glued to path/flag/code punctuation", () => {
    for (const v of ["src/teh ", "--teh ", "a.teh ", "`teh ", "@teh "]) {
      expect(apply(v, v.length)).toBeNull();
    }
  });

  it("still fires after an opening quote or paren", () => {
    const v = '("teh ';
    expect(apply(v, v.length)).toBe('("the ');
  });

  it("only reacts to boundary chars at the caret", () => {
    expect(apply("teh", 3)).toBeNull(); // still typing the word
    expect(isBoundaryChar(" ")).toBe(true);
    expect(isBoundaryChar("x")).toBe(false);
  });

  it("mid-text caret fixes only the word before it", () => {
    const v = "say teh word";
    // caret right after the space that finished "teh"
    expect(apply(v, 8)).toBe("say the word");
  });
});
