import { describe, expect, it } from "vitest";
import {
  addPersonalWord,
  boundaryAutocorrect,
  cachedOracleKnown,
  correctText,
  correctWord,
  finalWordAutocorrect,
  isBoundaryChar,
  oracleKnows,
  removePersonalWord,
  setSpellOracle,
  setWorkspaceVocab,
  setWorkspaceVocabFromPaths,
} from "./autocorrect";

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

describe("fuzzy layer (edit-distance vs wordFreq)", () => {
  it("fixes typos the exact dictionary misses", () => {
    expect(correctWord("wrok")).toBe("work");
    expect(correctWord("hte")).toBe("the");
    expect(correctWord("responsibilty")).toBe("responsibility");
    expect(correctWord("beacuseee")).toBeNull(); // nothing plausible → leave
  });

  it("never touches real words, even rare-ish ones", () => {
    for (const w of ["the", "work", "asynchronous", "throughput", "hierarchy"]) {
      expect(correctWord(w)).toBeNull();
    }
  });

  it("never touches chat/dev vocabulary", () => {
    for (const w of ["svelte", "tauri", "gonna", "lol", "npm", "async", "regex"]) {
      expect(correctWord(w)).toBeNull();
    }
  });

  it("never touches regular inflections missing from the frequency list", () => {
    // The misfire class: the 50k list holds lemmas, not every inflection, so
    // these all read as "unknown" and used to get rewritten to whatever common
    // word sat one edit away. Their stems are known — that's what saves them.
    for (const w of ["greps", "unstick", "prefetching", "rebased", "reranks", "misfires"]) {
      expect(correctWord(w)).toBeNull();
    }
  });

  it("never touches everyday dev-chat plurals and inflections", () => {
    for (const w of ["workspaces", "runtimes", "endpoints", "webhooks", "debounced", "memoized", "linters", "commits", "renders", "caches"]) {
      expect(correctWord(w)).toBeNull();
    }
  });

  it("never touches Rift's own product vocabulary", () => {
    for (const w of ["shiki", "velopack", "opus", "sonnet", "claude", "runes"]) {
      expect(correctWord(w)).toBeNull();
    }
  });

  it("never touches identifiers, acronyms, or proper nouns mid-sentence", () => {
    expect(correctWord("myVar")).toBeNull();
    expect(correctWord("SvelteKit")).toBeNull();
    expect(correctWord("HTTP")).toBeNull();
    expect(correctWord("Blazzer")).toBeNull(); // capitalized, not sentence start
  });

  it("corrects a capitalized word when told it starts a sentence", () => {
    expect(correctWord("Grofile", true)).toBe("Profile"); // very common target
    expect(correctWord("Blazzer", true)).toBeNull(); // "blazer" too rare to overrule a name
  });

  it("sentence-start detection flows through boundaryAutocorrect", () => {
    const v = "done. Grofile next ";
    expect(apply(v, 14)).toBe("done. Profile next "); // caret after "Grofile "
    const mid = "the Grofile ";
    expect(apply(mid, mid.length)).toBeNull(); // mid-sentence cap = proper noun
  });

  it("leaves very short and apostrophe words to the exact map only", () => {
    expect(correctWord("hi")).toBeNull();
    expect(correctWord("it'sx")).toBeNull();
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

describe("brand names + learned vocabulary (the FiveM bug class)", () => {
  it("leaves gaming/platform names alone", () => {
    // Pre-fix these got eaten by the fuzzy layer ("fivem"→"five", "redm"→"red").
    for (const w of ["fivem", "redm", "gta", "twitch", "nvidia"]) {
      expect(correctWord(w)).toBeNull();
    }
    expect(correctWord("Fivem", true)).toBeNull(); // sentence-start variant
  });

  it("personal dictionary beats even the exact typo map", () => {
    expect(correctWord("teh")).toBe("the");
    addPersonalWord("teh");
    expect(correctWord("teh")).toBeNull();
    removePersonalWord("teh");
    expect(correctWord("teh")).toBe("the");
  });

  it("workspace vocabulary from file paths suppresses corrections", () => {
    expect(correctWord("quixel")).not.toBeNull(); // fuzzy layer wants this one
    setWorkspaceVocabFromPaths(["scripts/QuixelLoader/quixel-map.lua"]);
    expect(correctWord("quixel")).toBeNull();
    expect(correctWord("loader")).toBeNull();
    setWorkspaceVocab([]);
    expect(correctWord("quixel")).not.toBeNull();
  });

  it("boundary fixes carry layer + typed word for the oracle gate", () => {
    const exact = boundaryAutocorrect("fix teh ", 8);
    expect(exact).toMatchObject({ fuzzy: false, word: "teh" });
    // "wrok" sits in the generated exact map — use a word only the fuzzy
    // layer can reach so the flag is exercised.
    const fuzzy = boundaryAutocorrect("responsibilty ", 14);
    expect(fuzzy).toMatchObject({ fuzzy: true, word: "responsibilty", replacement: "responsibility" });
  });

  it("a cached OS-oracle verdict marks a word known", async () => {
    expect(cachedOracleKnown("responsibilty")).toBeUndefined();
    setSpellOracle(async () => true);
    expect(correctWord("responsibilty")).toBe("responsibility"); // unchecked → still corrected
    await oracleKnows("responsibilty");
    expect(cachedOracleKnown("responsibilty")).toBe(true);
    expect(correctWord("responsibilty")).toBeNull(); // OS vouched for it
    setSpellOracle(null);
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
