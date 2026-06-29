// Canonical path helpers — the single home for path-leaf / short-path / pretty-
// path logic that had drifted into ~6 reimplementations (tabsbar `leafName`,
// toolCaption `basename`, streamModel inline arrow, plus inline
// `.replace(/[\\/]+$/,"").split(/[\\/]/).pop()` chains in AssistantPane /
// FilePathMenu / ConversationList). Lives under utils/ — a neutral boundary both
// the shell and assistant families import without crossing into each other.
// Unit-tested in path.test.ts.

/** Last path segment (the "leaf"), handling both `\` and `/` separators and a
 *  trailing slash. `leafName("C:\\a\\b") === "b"`, `leafName("/a/b/") === "b"`. */
export function leafName(p: string): string {
  const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
  const parts = norm.split("/");
  return parts[parts.length - 1] || norm;
}

/** Last two path segments, ellipsis-prefixed — a compact path for recents lists. */
export function shortPath(p: string): string {
  const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
  const parts = norm.split("/");
  if (parts.length <= 2) return norm;
  return `…/${parts.slice(-2).join("/")}`;
}

/** Cleaned path for tooltips — drops Windows extended-length `\\?\` noise. */
export function prettyPath(p: string): string {
  return p.replace(/^\\\\\?\\/, "").replace(/^\/\/\?\//, "");
}

/** Canonicalize a root path for comparison: strip a trailing separator and
 *  lowercase (Windows paths are case-insensitive + drive-letter casing varies).
 *  The single home for the "is this the same folder?" key — was duplicated as
 *  `projects.svelte.ts::projectRootKey` and `ConversationList::rootKey`. */
export function rootKey(r: string | null | undefined): string {
  return (r ?? "").replace(/[\\/]+$/, "").toLowerCase();
}
