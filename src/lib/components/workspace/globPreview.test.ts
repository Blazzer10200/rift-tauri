import { describe, it, expect } from "vitest";
import {
  globToRegExp,
  validateGlobs,
  globSummary,
  PATTERN_MAX_LEN,
  PATTERNS_MAX,
} from "./globPreview.js";

// Pure module — no Tauri IPC involved, no mock needed.

// ---------------------------------------------------------------------------
// globToRegExp
// ---------------------------------------------------------------------------

describe("globToRegExp — single * (one segment only)", () => {
  it("*.ts matches foo.ts", () => {
    expect(globToRegExp("*.ts").test("foo.ts")).toBe(true);
  });

  it("*.ts does NOT match a/foo.ts (crosses /)", () => {
    expect(globToRegExp("*.ts").test("a/foo.ts")).toBe(false);
  });

  it("*.ts does NOT match nested/deep/bar.ts", () => {
    expect(globToRegExp("*.ts").test("nested/deep/bar.ts")).toBe(false);
  });

  it("src/*.ts matches src/main.ts but NOT src/a/b.ts", () => {
    const re = globToRegExp("src/*.ts");
    expect(re.test("src/main.ts")).toBe(true);
    expect(re.test("src/a/b.ts")).toBe(false);
  });
});

describe("globToRegExp — ** (any depth, zero-or-more segments)", () => {
  it("src/**/*.ts matches src/main.ts (ZERO intermediate segments)", () => {
    expect(globToRegExp("src/**/*.ts").test("src/main.ts")).toBe(true);
  });

  it("src/**/*.ts matches src/a/b.ts (one intermediate segment)", () => {
    expect(globToRegExp("src/**/*.ts").test("src/a/b.ts")).toBe(true);
  });

  it("src/**/*.ts matches src/a/b/c/deep.ts (many intermediate segments)", () => {
    expect(globToRegExp("src/**/*.ts").test("src/a/b/c/deep.ts")).toBe(true);
  });

  it("src/**/*.ts does NOT match other/main.ts", () => {
    expect(globToRegExp("src/**/*.ts").test("other/main.ts")).toBe(false);
  });

  it("vendor/** matches bare 'vendor' (zero trailing segments)", () => {
    expect(globToRegExp("vendor/**").test("vendor")).toBe(true);
  });

  it("vendor/** matches vendor/dep/x.js", () => {
    expect(globToRegExp("vendor/**").test("vendor/dep/x.js")).toBe(true);
  });

  it("vendor/** matches vendor/x", () => {
    expect(globToRegExp("vendor/**").test("vendor/x")).toBe(true);
  });

  it("vendor/** does NOT match other/vendor/x", () => {
    expect(globToRegExp("vendor/**").test("other/vendor/x")).toBe(false);
  });
});

describe("globToRegExp — bare leading ** ", () => {
  it("**/*.ts matches a top-level file (zero leading segments)", () => {
    expect(globToRegExp("**/*.ts").test("top.ts")).toBe(true);
  });

  it("**/*.ts matches a nested file (one+ leading segments)", () => {
    expect(globToRegExp("**/*.ts").test("a/b.ts")).toBe(true);
    expect(globToRegExp("**/*.ts").test("a/b/c.ts")).toBe(true);
  });

  it("**/*.ts does NOT match a non-.ts file", () => {
    expect(globToRegExp("**/*.ts").test("a/b.js")).toBe(false);
  });
});

describe("globToRegExp — anchored exact (^…$), no partial match", () => {
  it("'ts' does NOT partial-match foo.ts (anchors intact)", () => {
    expect(globToRegExp("ts").test("foo.ts")).toBe(false);
  });

  it("'*.ts' does NOT match foo.ts.bak (no trailing slop)", () => {
    expect(globToRegExp("*.ts").test("foo.ts.bak")).toBe(false);
  });
});

describe("globToRegExp — total function: never throws, escapes metachars", () => {
  // Every regex metachar is escaped before new RegExp(), so a pattern with
  // brackets/parens/backslashes compiles to a LITERAL match, never an error.
  // This pins the invariant — a future edit that drops an escape would throw
  // here instead of leaking a raw regex into the matcher.
  it.each(["[", "]", "(", ")", "{", "}", "\\", "a\\", "(((", "src/**/[", "a{2,"])(
    "%s does not throw and matches itself literally",
    (p) => {
      expect(() => globToRegExp(p)).not.toThrow();
      expect(globToRegExp(p).test(p)).toBe(true);
    },
  );
});

describe("globToRegExp — ? (one non-/ char)", () => {
  it("? matches a single non-slash character", () => {
    expect(globToRegExp("?.ts").test("a.ts")).toBe(true);
  });

  it("? does NOT match a slash", () => {
    expect(globToRegExp("?.ts").test("/.ts")).toBe(false);
  });

  it("? does NOT match zero chars", () => {
    expect(globToRegExp("?.ts").test(".ts")).toBe(false);
  });

  it("? does NOT match two chars", () => {
    expect(globToRegExp("?.ts").test("ab.ts")).toBe(false);
  });

  it("src/?.ts matches src/a.ts", () => {
    expect(globToRegExp("src/?.ts").test("src/a.ts")).toBe(true);
  });

  it("src/?.ts does NOT match src/ab.ts", () => {
    expect(globToRegExp("src/?.ts").test("src/ab.ts")).toBe(false);
  });
});

describe("globToRegExp — . is literal (not regex wildcard)", () => {
  it("a.txt matches a.txt", () => {
    expect(globToRegExp("a.txt").test("a.txt")).toBe(true);
  });

  it("a.txt does NOT match axtxt (dot is not a wildcard)", () => {
    expect(globToRegExp("a.txt").test("axtxt")).toBe(false);
  });

  it("a.txt does NOT match a/txt", () => {
    expect(globToRegExp("a.txt").test("a/txt")).toBe(false);
  });
});

describe("globToRegExp — plain literal paths", () => {
  it("exact path matches itself", () => {
    expect(globToRegExp("src/lib/index.ts").test("src/lib/index.ts")).toBe(true);
  });

  it("exact path does NOT match a different path", () => {
    expect(globToRegExp("src/lib/index.ts").test("src/lib/other.ts")).toBe(false);
  });

  it("bare filename matches only itself", () => {
    expect(globToRegExp("README.md").test("README.md")).toBe(true);
    expect(globToRegExp("README.md").test("docs/README.md")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// validateGlobs
// ---------------------------------------------------------------------------

describe("validateGlobs — blank/whitespace lines dropped", () => {
  it("empty text returns no checks and no listError", () => {
    const { checks, listError } = validateGlobs("");
    expect(checks).toHaveLength(0);
    expect(listError).toBeNull();
  });

  it("whitespace-only lines are ignored", () => {
    const { checks } = validateGlobs("   \n\t\n  ");
    expect(checks).toHaveLength(0);
  });

  it("mixed blank and valid lines: only non-blank returned", () => {
    const { checks } = validateGlobs("\n*.ts\n\n*.js\n");
    expect(checks).toHaveLength(2);
    expect(checks[0].pattern).toBe("*.ts");
    expect(checks[1].pattern).toBe("*.js");
  });

  it("Windows CRLF line endings split the same as LF", () => {
    const { checks } = validateGlobs("*.ts\r\nsrc/**\r\n\r\n*.js");
    expect(checks.map((c) => c.pattern)).toEqual(["*.ts", "src/**", "*.js"]);
  });
});

describe("validateGlobs — per-pattern length cap", () => {
  it("pattern at exactly PATTERN_MAX_LEN is ok", () => {
    const pattern = "a".repeat(PATTERN_MAX_LEN);
    const { checks } = validateGlobs(pattern);
    expect(checks[0].ok).toBe(true);
  });

  it("pattern longer than PATTERN_MAX_LEN is flagged ok:false with a length error", () => {
    const pattern = "a".repeat(PATTERN_MAX_LEN + 1);
    const { checks } = validateGlobs(pattern);
    expect(checks[0].ok).toBe(false);
    expect(checks[0].error).toMatch(/too long/);
    expect(checks[0].error).toMatch(String(PATTERN_MAX_LEN));
  });

  it("long pattern entry has the pattern echoed back", () => {
    const pattern = "x".repeat(PATTERN_MAX_LEN + 5);
    const { checks } = validateGlobs(pattern);
    expect(checks[0].pattern).toBe(pattern);
  });
});

describe("validateGlobs — PATTERNS_MAX list cap", () => {
  it("exactly PATTERNS_MAX non-blank lines: no listError", () => {
    const text = Array.from({ length: PATTERNS_MAX }, (_, i) => `*.ext${i}`).join("\n");
    const { listError } = validateGlobs(text);
    expect(listError).toBeNull();
  });

  it("PATTERNS_MAX + 1 patterns triggers listError", () => {
    const text = Array.from({ length: PATTERNS_MAX + 1 }, (_, i) => `*.ext${i}`).join("\n");
    const { listError } = validateGlobs(text);
    expect(listError).not.toBeNull();
    expect(listError).toMatch(/too many/);
    expect(listError).toMatch(String(PATTERNS_MAX));
  });

  it("listError includes the actual count", () => {
    const count = PATTERNS_MAX + 3;
    const text = Array.from({ length: count }, (_, i) => `pat${i}`).join("\n");
    const { listError } = validateGlobs(text);
    expect(listError).toMatch(String(count));
  });

  it("blank lines interspersed do not count toward the limit", () => {
    // PATTERNS_MAX valid lines + many blank lines = should NOT trigger
    const lines: string[] = [];
    for (let i = 0; i < PATTERNS_MAX; i++) {
      lines.push(`*.ext${i}`);
      lines.push(""); // blank between each
    }
    const { listError } = validateGlobs(lines.join("\n"));
    expect(listError).toBeNull();
  });
});

describe("validateGlobs — GlobCheck shape", () => {
  it("valid pattern returns {pattern, ok:true} with no error field", () => {
    const { checks } = validateGlobs("*.ts");
    expect(checks[0]).toMatchObject({ pattern: "*.ts", ok: true });
    expect(checks[0].error).toBeUndefined();
  });

  it("invalid (too-long) pattern returns {pattern, ok:false, error: string}", () => {
    const pattern = "z".repeat(PATTERN_MAX_LEN + 1);
    const { checks } = validateGlobs(pattern);
    expect(checks[0].ok).toBe(false);
    expect(typeof checks[0].error).toBe("string");
  });
});

// ---------------------------------------------------------------------------
// globSummary
// ---------------------------------------------------------------------------

describe("globSummary", () => {
  it("empty text => invalid:0, firstError:null", () => {
    expect(globSummary("")).toEqual({ total: 0, invalid: 0, firstError: null });
  });

  it("all-valid patterns => invalid:0, firstError:null", () => {
    expect(globSummary("*.ts\nsrc/**/*.js\nvendor/**")).toEqual({
      total: 3,
      invalid: 0,
      firstError: null,
    });
  });

  it("one bad pattern => invalid:1, firstError is the error message", () => {
    const longPat = "a".repeat(PATTERN_MAX_LEN + 1);
    const { invalid, firstError } = globSummary(longPat);
    expect(invalid).toBe(1);
    expect(firstError).toMatch(/too long/);
  });

  it("multiple bad patterns: invalid reflects count, firstError is from first bad one", () => {
    const longPat1 = "b".repeat(PATTERN_MAX_LEN + 1);
    const longPat2 = "c".repeat(PATTERN_MAX_LEN + 2);
    const { invalid, firstError } = globSummary(`${longPat1}\n${longPat2}`);
    expect(invalid).toBe(2);
    // firstError must be from the first bad pattern
    expect(firstError).toMatch(/too long/);
  });

  it("listError takes precedence over per-pattern error in firstError", () => {
    // Build a list that exceeds PATTERNS_MAX, including one too-long pattern
    const longPat = "d".repeat(PATTERN_MAX_LEN + 1);
    const normalPats = Array.from({ length: PATTERNS_MAX }, (_, i) => `*.x${i}`).join("\n");
    const text = `${longPat}\n${normalPats}`; // total = PATTERNS_MAX + 1
    const { invalid, firstError } = globSummary(text);
    // listError wins for firstError, but the per-pattern bad count is still tracked
    expect(firstError).toMatch(/too many/);
    expect(invalid).toBe(1);
  });

  it("whitespace-only text => invalid:0, firstError:null", () => {
    expect(globSummary("   \n\t\n")).toEqual({ total: 0, invalid: 0, firstError: null });
  });
});
