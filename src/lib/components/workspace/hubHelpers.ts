// Pure per-project activity aggregation for the Workspace hub. Everything
// derives client-side from the ConversationMeta cache (workspaceRoot /
// lastActivityAt / costUsd) — zero IPC. Unit-tested in hubHelpers.test.ts.

import { rootKey } from "$lib/utils/path";

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
  const s = Math.max(0, (now - ts) / 1000);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h ago`;
  if (s < 604_800) return `${Math.floor(s / 86_400)}d ago`;
  return `${Math.floor(s / 604_800)}w ago`;
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

