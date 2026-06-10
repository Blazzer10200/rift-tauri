// Pure helpers extracted from ChatTabsBar.svelte (#20 H0 — chattabsbar-split.md).
// Unit-tested in helpers.test.ts (menuKeydown excluded — needs a live DOM).

// #150: ARIA menu keyboard contract — ArrowUp/Down + Home/End move focus
// between the role=menuitem(checkbox) buttons. Shared by proj + view menus.
export function menuKeydown(e: KeyboardEvent, container: HTMLElement | undefined) {
  if (!container) return;
  const items = [...container.querySelectorAll<HTMLButtonElement>('[role^="menuitem"]')];
  if (items.length === 0) return;
  const cur = items.indexOf(document.activeElement as HTMLButtonElement);
  let next: number;
  switch (e.key) {
    case "ArrowDown": next = cur < 0 ? 0 : (cur + 1) % items.length; break;
    case "ArrowUp":   next = cur < 0 ? items.length - 1 : (cur - 1 + items.length) % items.length; break;
    case "Home":      next = 0; break;
    case "End":       next = items.length - 1; break;
    default: return;
  }
  e.preventDefault();
  items[next]?.focus();
}

export function leafName(p: string): string {
  const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
  const parts = norm.split("/");
  return parts[parts.length - 1] || norm;
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
