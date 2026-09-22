// Shared relative-time formatter. The 5 former copies (hubHelpers, welcomeShared,
// ConversationList, NotificationCenter, SettingsPage) diverged in more than wording
// — rounding (floor vs round), "just now" threshold, and which tiers exist — so each
// caller's exact prior output string is reproduced here via `style`, not collapsed
// into one shape.

export type RelTimeStyle = "short" | "long" | "notif" | "compact";

/**
 * - "long": "just now" / "Xm ago" / "Xh ago" / "Xd ago" / "Xw ago" (floor, 60s cutoff).
 * - "short": "now" / "Xm" / "Xh" / "Xd" / "Xw" (floor, 60s cutoff, no "ago" suffix).
 * - "notif": "just now" (<45s) / "Xm ago" / "Xh ago" / "Xd ago" (round, no week tier).
 * - "compact": "just now" (<10s) / "Xs ago" / "Xm ago" / "Xh ago" (round, no day/week tier).
 */
export function relTime(ts: number, now: number = Date.now(), style: RelTimeStyle = "short"): string {
  if (style === "notif") {
    const s = Math.max(0, Math.round((now - ts) / 1000));
    if (s < 45) return "just now";
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.round(m / 60);
    if (h < 24) return `${h}h ago`;
    const d = Math.round(h / 24);
    return `${d}d ago`;
  }

  if (style === "compact") {
    const s = Math.max(0, Math.round((now - ts) / 1000));
    if (s < 10) return "just now";
    if (s < 60) return `${s}s ago`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    return `${Math.round(m / 60)}h ago`;
  }

  const s = Math.max(0, (now - ts) / 1000);
  if (style === "short") {
    if (s < 60) return "now";
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    if (s < 86_400) return `${Math.floor(s / 3600)}h`;
    if (s < 604_800) return `${Math.floor(s / 86_400)}d`;
    return `${Math.floor(s / 604_800)}w`;
  }

  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h ago`;
  if (s < 604_800) return `${Math.floor(s / 86_400)}d ago`;
  return `${Math.floor(s / 604_800)}w ago`;
}
