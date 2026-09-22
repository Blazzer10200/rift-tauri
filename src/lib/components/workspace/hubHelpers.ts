// Pure per-project activity aggregation for the Workspace hub. Everything
// derives client-side from the ConversationMeta cache (workspaceRoot /
// lastActivityAt / costUsd) — zero IPC. Unit-tested in hubHelpers.test.ts.
//
// Also home to the shared time-of-day greeting + coarse "time ago" formatter
// (merged in from the former welcomeShared.ts) — consumed by the Workspace
// page hero and the empty-Chat welcome (AssistantWelcome) so the two surfaces
// can never drift.

import { rootKey } from "$lib/utils/path";
import { relTime as sharedRelTime } from "$lib/utils/relTime";

export function greeting(hr: number): string {
  if (hr < 5) return "Still up";
  if (hr < 12) return "Good morning";
  if (hr < 18) return "Good afternoon";
  return "Good evening";
}

// Coarse "time ago" for conversation cards. Keeps the same buckets both surfaces
// already used (just-now / m / h / d, then a locale date past a week).
export function fmtAgo(ms: number, now: number = Date.now()): string {
  if (now - ms < 7 * 86_400_000) return sharedRelTime(ms, now, "long");
  return new Date(ms).toLocaleDateString();
}

/** Structural subset of ConversationMeta the hub needs — keeps this module
 *  free of state imports so it stays pure/testable. */
export type ChatLike = {
  id: string;
  title: string;
  messageCount: number;
  createdAt: number;
  lastActivityAt?: number;
  costUsd: number;
  lastSnippet?: string;
  workspaceRoot?: string | null;
};

/** Per-project activity rollup shown on a project card. */
export type ProjectPulse = {
  chats: number;
  cost: number;
  /** Most recent real activity across the project's chats, or null when none. */
  lastAt: number | null;
};

export const EMPTY_PULSE: ProjectPulse = { chats: 0, cost: 0, lastAt: null };

/** Real-activity timestamp for ranking — never open/switch bumps. */
export const chatLastAt = (c: ChatLike): number => c.lastActivityAt ?? c.createdAt;

/** "just now" / "5m ago" / "3h ago" / "2d ago" / "4w ago". */
export function relTime(ts: number, now: number): string {
  return sharedRelTime(ts, now, "long");
}

/** One pass over all conversations → rollup per canonical root key. Unfiled
 *  chats (no workspaceRoot) don't land anywhere — rootKey("") is skipped. */
export function pulseByRoot(convos: ChatLike[]): Map<string, ProjectPulse> {
  const map = new Map<string, ProjectPulse>();
  for (const c of convos) {
    const key = rootKey(c.workspaceRoot ?? "");
    if (!key) continue;
    const p = map.get(key) ?? { chats: 0, cost: 0, lastAt: null };
    p.chats++;
    p.cost += c.costUsd || 0;
    const at = chatLastAt(c);
    if (p.lastAt == null || at > p.lastAt) p.lastAt = at;
    map.set(key, p);
  }
  return map;
}

