// Shared time-of-day greeting — consumed by both the Workspace page hero and the
// empty-Chat welcome (AssistantWelcome). Single source so the two surfaces can
// never drift, and so the boundaries stay unit-testable without a live clock.

export function greeting(hr: number): string {
  if (hr < 5) return "Still up";
  if (hr < 12) return "Good morning";
  if (hr < 18) return "Good afternoon";
  return "Good evening";
}

// Coarse "time ago" for conversation cards. Keeps the same buckets both surfaces
// already used (just-now / m / h / d, then a locale date past a week).
export function fmtAgo(ms: number, now: number = Date.now()): string {
  const diff = now - ms;
  const min = 60_000, hr = 60 * min, day = 24 * hr;
  if (diff < min) return "just now";
  if (diff < hr) return `${Math.floor(diff / min)}m ago`;
  if (diff < day) return `${Math.floor(diff / hr)}h ago`;
  if (diff < 7 * day) return `${Math.floor(diff / day)}d ago`;
  return new Date(ms).toLocaleDateString();
}
