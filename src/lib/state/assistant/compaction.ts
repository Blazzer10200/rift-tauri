// M7 (per docs/design/assistant-svelte-split.md) — summarize + compact
// pipeline lifted out of `src/lib/state/assistant.svelte.ts` as free fns on a
// host ref. Per-tab fields (compactingNow, forceNextFirstTurn,
// pendingCompactionSummary, lastCompactionAt, compactionHistory) STAY on
// TabState; the trivial autoCompactThreshold/compactModel setters STAY on the
// store. Only the two pipeline methods move. The auto-compact trigger lives in
// the stream onDone path (M8), not here.
//
// Bodies ported verbatim (this.* → host.*) so the load-bearing edge cases
// survive: S124 (pre-staged streaming boundary block patched live), the
// remint-on-success flow, the 4-message floor, and the compactingNow guard.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  BoundaryBlock,
  ChatMessage,
  CompactionHistoryEntry,
  SummarizeResult,
} from "./types";

/** Structural subset of TabState the compaction pipeline reads/writes. */
export type CompactableTab = {
  streaming: boolean;
  compactingNow: boolean;
  messages: ChatMessage[];
  cliSessionId: string;
  convoCreatedAt: number | null;
  forceNextFirstTurn: boolean;
  pendingCompactionSummary: string | null;
  lastCompactionAt: number;
  compactionHistory: CompactionHistoryEntry[];
  resetUsage(): void;
};

/** Subset of AssistantStore the compaction pipeline touches. */
export type CompactionHost = {
  currentCliSessionId: string | null;
  tasks: { content: string; status: "pending" | "in_progress" | "completed" }[];
  lastError: string | null;
  lastNotice: string | null;
  compactModel: "haiku" | "sonnet";
  ctxPct: number;
  ctxWindow: number;
  activeTab: CompactableTab | null;
  tabFor(id: string | null): CompactableTab | null;
  scheduleSave(flush?: boolean): void;
};

export async function summarizeCurrentSession(
  host: CompactionHost,
  focus?: string,
): Promise<SummarizeResult | null> {
  const sid = host.currentCliSessionId;
  if (!sid) {
    host.lastError = "No active session yet — send a message first.";
    return null;
  }
  const tasksJson = JSON.stringify(
    host.tasks.map((t) => ({ content: t.content, status: t.status })),
  );
  try {
    const res = await invoke<SummarizeResult>("assistant_summarize_session", {
      sessionId: sid,
      focus: focus ?? null,
      tasksJson,
    });
    return res;
  } catch (e) {
    host.lastError = `Summarize failed: ${String(e)}`;
    return null;
  }
}

/** Compaction Phase C: full compact action. Summarizes the current
 *  session via Phase B, remints the CLI session id via the backend,
 *  pushes a BoundaryBlock into messages, and stages the summary onto
 *  the next send so the fresh CLI session has context.
 *
 *  Guards (any failed → abort with notice/error, no state change):
 *   - not currently streaming
 *   - not already compacting
 *   - at least 4 messages worth compacting
 *   - have an active tab + cliSessionId
 *
 *  Cost is fully internal — no UI confirmation here; the Compact button
 *  in the header should confirm before calling (Phase E1 polish). */
export async function compactConversation(
  host: CompactionHost,
  focus?: string,
  tabId?: string | null,
): Promise<boolean> {
  const tab = tabId ? host.tabFor(tabId) : host.activeTab;
  if (!tab) {
    host.lastError = "No active tab.";
    return false;
  }
  if (tab.streaming) {
    host.lastError = "Wait for the current turn to finish before compacting.";
    return false;
  }
  if (tab.compactingNow) {
    host.lastError = "Compaction already in progress.";
    return false;
  }
  if (tab.messages.length < 4) {
    host.lastError = "Conversation too short to compact (need ≥4 messages).";
    return false;
  }
  const oldSid = tab.cliSessionId;
  if (!oldSid) {
    host.lastError = "No CLI session to compact.";
    return false;
  }

  tab.compactingNow = true;
  host.lastNotice = "Compacting conversation…";

  // S124: pre-stage the boundary message w/ streaming:true BEFORE the
  // summarize call. As progress events land, we patch the same block in
  // place so the user sees the summary fill live.
  const archivedCount = tab.messages.length;
  const boundaryId = crypto.randomUUID();
  const placeholderModel = host.compactModel ?? "haiku";
  const ctxPctBefore = host.ctxPct;
  const ctxWindowAtCompact = host.ctxWindow;
  const stagedBoundary: ChatMessage = {
    id: boundaryId,
    role: "system",
    blocks: [
      {
        type: "boundary",
        summary: "",
        at: Date.now(),
        archivedCount,
        costUsd: 0,
        summaryModel: placeholderModel,
        streaming: true,
        ctxPctBefore,
      },
    ],
  };
  tab.messages = [...tab.messages, stagedBoundary];

  // Live updater — replace the boundary block's summary field as the
  // backend emits progress chunks. Tab-aware so background tabs don't
  // get clobbered if a user switches mid-compact.
  const patchBoundary = (patch: Partial<BoundaryBlock>) => {
    const idx = tab.messages.findIndex((m) => m.id === boundaryId);
    if (idx === -1) return;
    const msg = tab.messages[idx];
    const block = msg.blocks[0];
    if (block?.type !== "boundary") return;
    const nextBlock: BoundaryBlock = { ...block, ...patch };
    const nextMsg: ChatMessage = { ...msg, blocks: [nextBlock] };
    const next = tab.messages.slice();
    next[idx] = nextMsg;
    tab.messages = next;
  };
  let progressUnlisten: UnlistenFn | null = null;
  try {
    progressUnlisten = await listen<{
      session_id: string;
      summary_so_far: string;
      status: "streaming" | "done";
    }>("assistant://summarize-progress", (e) => {
      if (e.payload.session_id !== oldSid) return;
      patchBoundary({ summary: e.payload.summary_so_far });
    });
    const res = await summarizeCurrentSession(host, focus);
    if (!res) {
      // summarizeCurrentSession already set lastError. Drop the staged
      // boundary so the chat doesn't keep a half-rendered pill.
      tab.messages = tab.messages.filter((m) => m.id !== boundaryId);
      return false;
    }
    const newSid = crypto.randomUUID();
    try {
      await invoke("assistant_remint_session", {
        oldSessionId: oldSid,
        newSessionId: newSid,
      });
    } catch (e) {
      host.lastError = `Remint failed: ${String(e)}`;
      tab.messages = tab.messages.filter((m) => m.id !== boundaryId);
      return false;
    }

    // Finalize the staged boundary with the real summary + cost + model
    // and clear streaming. archivedCount stays the snapshot from pre-compact.
    // E1: post-compact ctx estimate — the new session starts with only the
    // summary in context (seeded as <system-reminder> on the next user turn).
    const ctxPctEstAfter =
      ctxWindowAtCompact > 0
        ? Math.min(100, (res.outputTokens / ctxWindowAtCompact) * 100)
        : 0;
    patchBoundary({
      summary: res.summary,
      costUsd: res.costUsd,
      summaryModel: res.model,
      streaming: false,
      ctxPctEstAfter,
    });

    // Flip the tab's CLI handle to the new session and force the next
    // send into first-turn mode (mints --session-id <new> instead of
    // --resume <new>, which would fail since there's no JSONL yet).
    tab.cliSessionId = newSid;
    tab.convoCreatedAt = null;
    tab.forceNextFirstTurn = true;
    tab.pendingCompactionSummary = res.summary;
    tab.resetUsage();
    const now = Date.now();
    tab.lastCompactionAt = now;
    tab.compactionHistory = [
      ...tab.compactionHistory,
      {
        at: now,
        priorSessionId: oldSid,
        newSessionId: newSid,
        summary: res.summary,
        costUsd: res.costUsd,
        summaryModel: res.model,
        archivedCount,
      },
    ];

    host.scheduleSave(true);
    const inTk = res.inputTokens + res.cacheReadTokens + res.cacheCreateTokens;
    host.lastNotice =
      `Compacted ${archivedCount} message(s) · $${res.costUsd.toFixed(4)} · ${inTk.toLocaleString()} in / ${res.outputTokens.toLocaleString()} out · ${res.model}. Next turn seeds the new session with the summary.`;
    return true;
  } finally {
    tab.compactingNow = false;
    if (progressUnlisten) progressUnlisten();
  }
}
