import { describe, it, expect } from "vitest";
import { emphasisIntervals, emphasizeHtml, parseUnifiedPatch, unifiedPatchCounts } from "./diffEmphasis";

describe("App Server unified patches", () => {
  it("pairs replacements and preserves hunk metadata", () => {
    const patch = "@@ -2,2 +2,3 @@\n keep\n-old\n+new\n+extra\n";
    expect(parseUnifiedPatch(patch)).toEqual([
      { kind: "meta", text: "@@ -2,2 +2,3 @@" },
      { kind: "ctx", left: "keep", right: "keep" },
      { kind: "mod", left: "old", right: "new" },
      { kind: "add", left: null, right: "extra" },
    ]);
    expect(unifiedPatchCounts(patch)).toEqual({ add: 2, del: 1 });
  });

  it("renders raw deleted-file contents as removals", () => {
    expect(parseUnifiedPatch("first\nsecond\n", "delete")).toEqual([
      { kind: "del", left: "first", right: null },
      { kind: "del", left: "second", right: null },
    ]);
    expect(unifiedPatchCounts("first\nsecond\n", "delete")).toEqual({ add: 0, del: 2 });
  });
});

describe("emphasisIntervals", () => {
  it("marks only the changed word in a small edit", () => {
    const left = 'const x = "smart";';
    const right = 'const x = "deep";';
    const r = emphasisIntervals(left, right);
    expect(r).not.toBeNull();
    expect(r!.del.length).toBe(1);
    expect(r!.add.length).toBe(1);
    // The changed token is covered; the untouched prefix is not.
    expect(left.slice(r!.del[0][0], r!.del[0][1])).toContain("smart");
    expect(left.slice(r!.del[0][0], r!.del[0][1])).not.toContain("const");
    expect(right.slice(r!.add[0][0], r!.add[0][1])).toContain("deep");
  });

  it("keeps space-separated changed words as separate intervals", () => {
    const r = emphasisIntervals("let a b c;", "let x y c;");
    expect(r).not.toBeNull();
    expect(r!.del.length).toBe(2); // 'a' and 'b' — unchanged space between
    expect(r!.add.length).toBe(2);
  });

  it("null when the pair is mostly a rewrite (emphasis would be noise)", () => {
    expect(emphasisIntervals("return fetchUser(id);", "console.log('bye');")).toBeNull();
  });

  it("null on blank sides and on identical lines", () => {
    expect(emphasisIntervals("", "anything")).toBeNull();
    expect(emphasisIntervals("same line", "same line")).toBeNull();
  });
});

describe("emphasizeHtml — plain (no shiki base)", () => {
  it("wraps interval slices and escapes everything", () => {
    const out = emphasizeHtml('a < "b" & c', [[4, 7]], "dem-add", null);
    expect(out).toBe('a &lt; <span class="dem-add">&quot;b&quot;</span> &amp; c');
  });

  it("no intervals → base passthrough", () => {
    expect(emphasizeHtml("text", [], "dem-add", null)).toBeNull();
    expect(emphasizeHtml("text", [], "dem-add", "<span>text</span>")).toBe("<span>text</span>");
  });
});

describe("emphasizeHtml — layered over flat token HTML", () => {
  it("splits a token at interval bounds, keeping the token wrapper", () => {
    const base = '<span style="color:#f00">const</span><span style="color:#0f0"> value</span>';
    // raw = "const value", emphasize "value" (chars 6..11)
    const out = emphasizeHtml("const value", [[6, 11]], "dem-add", base);
    expect(out).toBe(
      '<span style="color:#f00">const</span><span style="color:#0f0"> <span class="dem-add">value</span></span>',
    );
  });

  it("spans an interval across token boundaries", () => {
    const base = '<span style="color:#f00">ab</span><span style="color:#0f0">cd</span>';
    const out = emphasizeHtml("abcd", [[1, 3]], "dem-del", base);
    expect(out).toBe(
      '<span style="color:#f00">a<span class="dem-del">b</span></span><span style="color:#0f0"><span class="dem-del">c</span>d</span>',
    );
  });

  it("decodes entities for offset math and re-escapes on output", () => {
    const base = '<span style="color:#f00">&lt;div&gt;</span>';
    // raw = "<div>", emphasize "div" (chars 1..4)
    const out = emphasizeHtml("<div>", [[1, 4]], "dem-add", base);
    expect(out).toBe('<span style="color:#f00">&lt;<span class="dem-add">div</span>&gt;</span>');
  });

  it("bails to the untouched base on nested spans or offset drift", () => {
    const nested = '<span><span>x</span></span>';
    expect(emphasizeHtml("x", [[0, 1]], "dem-add", nested)).toBe(nested);
    const drifted = '<span style="color:#f00">short</span>';
    expect(emphasizeHtml("much longer raw line", [[0, 4]], "dem-add", drifted)).toBe(drifted);
  });

  it("handles bare text outside any token", () => {
    const base = 'plain <span style="color:#0f0">tok</span>';
    const out = emphasizeHtml("plain tok", [[0, 5]], "dem-del", base);
    expect(out).toBe('<span class="dem-del">plain</span> <span style="color:#0f0">tok</span>');
  });
});
