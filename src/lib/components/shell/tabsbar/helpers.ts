// Pure helpers extracted from ChatTabsBar.svelte (#20 H0 — chattabsbar-split.md).
// Unit-tested in helpers.test.ts.

export function leafName(p: string): string {
  const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
  const parts = norm.split("/");
  return parts[parts.length - 1] || norm;
}

// Last two path segments, ellipsis-prefixed — compact path for recents lists.
export function shortPath(p: string): string {
  const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
  const parts = norm.split("/");
  if (parts.length <= 2) return norm;
  return `…/${parts.slice(-2).join("/")}`;
}

// Cleaned path for tooltips — drops Windows extended-length `\\?\` noise.
export function prettyPath(p: string): string {
  return p.replace(/^\\\\\?\\/, "").replace(/^\/\/\?\//, "");
}

export function shortK(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 10_000) return `${Math.round(n / 1000)}K`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return String(n);
}
