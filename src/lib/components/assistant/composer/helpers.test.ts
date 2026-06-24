import { describe, expect, it } from "vitest";
import { attachImageFiles, bytesToBase64, effortIdxFromX, fmtClock, fmtSize, fuzzyScore, isFileDrag, summarizeAttach, summarizeTextAttach } from "./helpers";

describe("fmtClock", () => {
  it("formats zero and sub-second values", () => {
    expect(fmtClock(0)).toBe("0:00");
    expect(fmtClock(999)).toBe("0:00");
  });
  it("pads seconds and rolls minutes", () => {
    expect(fmtClock(5_000)).toBe("0:05");
    expect(fmtClock(65_000)).toBe("1:05");
    expect(fmtClock(600_000)).toBe("10:00");
  });
  it("clamps negative input to 0:00", () => {
    expect(fmtClock(-5_000)).toBe("0:00");
  });
});

describe("fuzzyScore", () => {
  it("returns 0 for an empty query", () => {
    expect(fuzzyScore("lib/foo/Composer.svelte", "")).toBe(0);
  });
  it("ranks basename substring above basename fuzzy above path fuzzy", () => {
    const substring = fuzzyScore("lib/foo/Composer.svelte", "comp")!;
    const baseFuzzy = fuzzyScore("lib/foo/Composer.svelte", "cmp")!;
    const pathFuzzy = fuzzyScore("compress/other.lua", "cmpo")!;
    expect(substring).toBeGreaterThan(baseFuzzy);
    expect(baseFuzzy).toBeGreaterThan(pathFuzzy);
  });
  it("prefers earlier substring hits in the basename", () => {
    expect(fuzzyScore("a/Composer.svelte", "comp")!).toBeGreaterThan(
      fuzzyScore("a/MyComposer.svelte", "comp")!,
    );
  });
  it("basename match beats a directory-only match per the design comment", () => {
    expect(fuzzyScore("lib/foo/Composer.svelte", "comp")!).toBeGreaterThan(
      fuzzyScore("compress/other.lua", "comp")!,
    );
  });
  it("is case-insensitive", () => {
    expect(fuzzyScore("lib/foo/Composer.svelte", "COMP")).toBe(
      fuzzyScore("lib/foo/Composer.svelte", "comp"),
    );
  });
  it("returns null when characters are missing", () => {
    expect(fuzzyScore("lib/foo/Composer.svelte", "zzz")).toBeNull();
  });
});

describe("effortIdxFromX", () => {
  const track = { left: 100, width: 300 };
  it("clamps below and above the track", () => {
    expect(effortIdxFromX(0, track, 4)).toBe(0);
    expect(effortIdxFromX(1000, track, 4)).toBe(3);
  });
  it("rounds to the nearest stop", () => {
    expect(effortIdxFromX(100, track, 4)).toBe(0);
    expect(effortIdxFromX(250, track, 4)).toBe(2); // frac 0.5 → 1.5 rounds up
    expect(effortIdxFromX(400, track, 4)).toBe(3);
  });
  it("handles a single stop without dividing into negatives", () => {
    expect(effortIdxFromX(250, track, 1)).toBe(0);
    expect(effortIdxFromX(250, track, 0)).toBe(0);
  });
});

describe("bytesToBase64", () => {
  it("encodes small buffers", () => {
    expect(bytesToBase64(new TextEncoder().encode("hello").buffer as ArrayBuffer)).toBe("aGVsbG8=");
    expect(bytesToBase64(new ArrayBuffer(0))).toBe("");
  });
  it("encodes buffers larger than the 0x8000 chunk size", () => {
    const big = new Uint8Array(0x8000 + 16);
    big.fill(65);
    const decoded = atob(bytesToBase64(big.buffer));
    expect(decoded.length).toBe(big.length);
    expect(decoded[0]).toBe("A");
    expect(decoded[decoded.length - 1]).toBe("A");
  });
});

describe("fmtSize", () => {
  it("formats B / KB / MB tiers", () => {
    expect(fmtSize(512)).toBe("512 B");
    expect(fmtSize(2048)).toBe("2 KB");
    expect(fmtSize(1024 * 1024 - 1)).toBe("1024 KB");
    expect(fmtSize(1_572_864)).toBe("1.5 MB");
  });
});

describe("isFileDrag", () => {
  const drag = (types: string[] | null) =>
    ({ dataTransfer: types ? { types } : null }) as unknown as DragEvent;
  it("detects Files in dataTransfer.types", () => {
    expect(isFileDrag(drag(["text/plain", "Files"]))).toBe(true);
  });
  it("rejects text-only drags and missing dataTransfer", () => {
    expect(isFileDrag(drag(["text/html"]))).toBe(false);
    expect(isFileDrag(drag(null))).toBe(false);
  });
});

describe("attachImageFiles", () => {
  const png = (name: string, bytes = 4) => new File([new Uint8Array(bytes)], name, { type: "image/png" });
  const txt = (name: string) => new File([new Uint8Array(4)], name, { type: "text/plain" });

  it("attaches images and collects non-image + oversized rejects", async () => {
    const big = new File([new Uint8Array(20 * 1024 * 1024 + 1)], "huge.png", { type: "image/png" });
    const res = await attachImageFiles([png("a.png"), txt("code.ts"), big], () => true);
    expect(res.attached).toBe(1);
    expect(res.nonImage).toEqual(["code.ts"]);
    expect(res.tooLarge).toEqual(["huge.png"]);
    expect(res.limitHit).toBe(false);
  });

  it("flags limitHit when the add callback refuses", async () => {
    const res = await attachImageFiles([png("a.png")], () => false);
    expect(res.attached).toBe(0);
    expect(res.limitHit).toBe(true);
  });
});

describe("summarizeAttach", () => {
  it("returns null when everything attached cleanly", () => {
    expect(summarizeAttach({ attached: 2, nonImage: [], tooLarge: [], failed: [], limitHit: false })).toBeNull();
  });
  it("no longer flags non-image files (they route to the text-attach path)", () => {
    // nonImage is populated by attachImageFiles but summarizeAttach ignores it —
    // non-image files are handled by attachTextFiles / summarizeTextAttach now.
    expect(summarizeAttach({ attached: 0, nonImage: ["code.ts"], tooLarge: [], failed: [], limitHit: false })).toBeNull();
  });
  it("joins the remaining rejection categories with a separator", () => {
    const s = summarizeAttach({ attached: 1, nonImage: ["a", "b"], tooLarge: ["big.png"], failed: [], limitHit: true })!;
    expect(s).toContain("big.png");
    expect(s).toContain("limit reached");
    expect(s).toContain(" · ");
    expect(s).not.toContain("non-image");
  });
});

describe("summarizeTextAttach", () => {
  it("returns null when everything attached cleanly", () => {
    expect(summarizeTextAttach({ attached: 2, binary: [], truncated: [], failed: [], limitHit: false })).toBeNull();
  });
  it("names a single binary file that couldn't attach as text", () => {
    const s = summarizeTextAttach({ attached: 0, binary: ["app.exe"], truncated: [], failed: [], limitHit: false });
    expect(s).toContain("app.exe");
    expect(s).toContain("binary");
  });
  it("reports truncation and the per-turn text cap", () => {
    const s = summarizeTextAttach({ attached: 1, binary: [], truncated: ["huge.log"], failed: [], limitHit: true })!;
    expect(s).toContain("huge.log");
    expect(s).toContain("truncated");
    expect(s).toContain("1 MB");
    expect(s).toContain(" · ");
  });
});
