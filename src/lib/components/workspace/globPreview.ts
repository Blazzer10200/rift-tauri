// Client-side mirror of the backend glob dialect (src-tauri/.../mcp_server.rs
// glob_to_regex) so the project editor can flag a broken pattern inline,
// BEFORE the save round-trip. Kept deliberately in lockstep with the Rust
// matcher: * = one path segment, ? = one non-'/', ** = any depth (collapsing
// its adjacent '/' so it spans zero-or-more segments). Drift here = a pattern
// the UI says is fine but the backend rejects (or vice versa) — change both.

/** Per-pattern length cap — mirrors PATTERN_MAX_LEN in projects.rs. */
export const PATTERN_MAX_LEN = 512;
/** Max patterns per list — mirrors PATTERNS_MAX in projects.rs. */
export const PATTERNS_MAX = 64;

const META = new Set([".", "+", "(", ")", "|", "^", "$", "{", "}", "[", "]", "\\"]);

/** Compile a single glob to a RegExp using the backend dialect, or throw the
 *  same shape of error the backend would. Exported for tests. */
export function globToRegExp(glob: string): RegExp {
  let out = "^";
  const chars = [...glob];
  for (let i = 0; i < chars.length; i++) {
    const c = chars[i];
    if (c === "*") {
      if (chars[i + 1] === "*") {
        i++; // consume second *
        if (chars[i + 1] === "/") {
          i++; // consume the '/'
          out += "(?:.*/)?";
        } else if (i + 1 >= chars.length && out.endsWith("/")) {
          out = out.slice(0, -1) + "(?:/.*)?";
        } else {
          out += ".*";
        }
      } else {
        out += "[^/]*";
      }
    } else if (c === "?") {
      out += "[^/]";
    } else if (META.has(c)) {
      out += "\\" + c;
    } else {
      out += c;
    }
  }
  out += "$";
  return new RegExp(out);
}

export type GlobCheck = { pattern: string; ok: boolean; error?: string };

/** Validate the raw textarea text (one glob per line). Blank lines are dropped.
 *  Returns one entry per non-blank line plus an overall list-level error
 *  (count/length caps) that maps to what the backend would reject. */
export function validateGlobs(text: string): { checks: GlobCheck[]; listError: string | null } {
  const lines = text
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  const checks: GlobCheck[] = lines.map((pattern) => {
    if (pattern.length > PATTERN_MAX_LEN) {
      return { pattern, ok: false, error: `too long (max ${PATTERN_MAX_LEN})` };
    }
    try {
      globToRegExp(pattern);
      return { pattern, ok: true };
    } catch (e) {
      return { pattern, ok: false, error: String(e instanceof Error ? e.message : e) };
    }
  });

  let listError: string | null = null;
  if (lines.length > PATTERNS_MAX) {
    listError = `too many patterns (max ${PATTERNS_MAX}, got ${lines.length})`;
  }
  return { checks, listError };
}

/** Convenience for the editor: the count of invalid patterns + the first error
 *  message, for a compact inline hint. */
export function globSummary(text: string): { invalid: number; firstError: string | null } {
  const { checks, listError } = validateGlobs(text);
  const bad = checks.filter((c) => !c.ok);
  return {
    invalid: bad.length,
    firstError: listError ?? bad[0]?.error ?? null,
  };
}
