import { describe, expect, it } from "vitest";
import { boundaryAutocorrect, correctText, correctWord, finalWordAutocorrect, isBoundaryChar } from "./autocorrect";

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

  it("fixes typos from the generated dictionary", () => {
    expect(correctWord("becasue")).toBe("because");
    expect(correctWord("abandonned")).toBe("abandoned");
    expect(correctWord("amercia")).toBe("America");
    expect(correctWord("Amercia")).toBe("America");
  });

  it("never fires on valid English words the source list marks as typos", () => {
    // "posses"/"loosing" are real words — the generator filters them out.
    expect(correctWord("posses")).toBeNull();
    expect(correctWord("loosing")).toBeNull();
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

  it("fixes generated-dictionary typos at a boundary", () => {
    const v = "only becasue ";
    expect(apply(v, v.length)).toBe("only because ");
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

  it("fixes curated chat typos the generated list misses", () => {
    expect(correctWord("accuratly")).toBe("accurately");
    expect(correctWord("intergrate")).toBe("integrate");
    expect(correctWord("Intergrate")).toBe("Integrate");
    expect(correctWord("thats")).toBe("that's");
  });
});

describe("finalWordAutocorrect", () => {
  const applyFinal = (value: string): string | null => {
    const fix = finalWordAutocorrect(value);
    if (!fix) return null;
    return value.slice(0, fix.start) + fix.replacement + value.slice(fix.end);
  };

  it("fixes the trailing word with no boundary char (Enter-send path)", () => {
    expect(applyFinal("fix teh")).toBe("fix the");
    expect(applyFinal("that looks wierd")).toBe("that looks weird");
  });

  it("null on clean, empty, glued, or slash-command input", () => {
    expect(applyFinal("all good")).toBeNull();
    expect(applyFinal("")).toBeNull();
    expect(applyFinal("src/teh")).toBeNull();
    expect(applyFinal("/diag im")).toBeNull();
  });
});

describe("correctText (right-click Auto-correct pass)", () => {
  it("fixes dictionary typos across the whole text", () => {
    expect(correctText("i dont know wich one")).toBe("I don't know which one");
  });

  it("capitalizes sentence starts and collapses space runs", () => {
    expect(correctText("hello.  it works")).toBe("Hello. It works");
  });

  it("leaves paths, flags, and code-glued words alone", () => {
    expect(correctText("run src/teh --teh a.teh")).toBe("Run src/teh --teh a.teh");
    expect(correctText("var teh2 = 1")).toBe("Var teh2 = 1");
  });

  it("no-ops on already-clean text", () => {
    expect(correctText("All clean here.")).toBe("All clean here.");
  });
});
