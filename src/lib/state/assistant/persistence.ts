// M5 (per docs/design/assistant-svelte-split.md) — conversation persistence
// IPC + tab-list localStorage lifted out of `src/lib/state/assistant.svelte.ts`
// as free fns operating on a host ref. Follows M3/M4 precedent: $state fields
// (conversations, currentConvoId, openTabs, panes, focusedPaneIdx, convoTitle,
// per-tab saveTimer/messages/convoTitle/convoCreatedAt/cliSessionId/compactionHistory)
// STAY where they are; only the IPC + serialization logic moves here.
//
// Scope (this pass): refreshConversations, buildSaveRecord, flushNow,
// scheduleSave, renameConversation, persistTabs. loadConversation +
// deleteConversation stay on AssistantStore — they orchestrate
// stop/ensureTab/closeTab across modules that haven't been extracted yet
// (M6 tabs lifecycle). Move them in M5b after M6 lands.

import { invoke } from "@tauri-apps/api/core";
import type {
  ChatMessage,
  CompactionHistoryEntry,
  ConversationMeta,
  ConversationRecord,
  ModelSel,
  PaneState,
} from "./types";

/** Subset of TabState fields the save plumbing reads/writes. Structural —
 *  no class import, matches the live $state fields declared on TabState. */
export type SaveableTab = {
  messages: ChatMessage[];
  saveTimer: ReturnType<typeof setTimeout> | null;
  convoTitle: string | null;
  convoCreatedAt: number | null;
  cliSessionId: string;
  compactionHistory: CompactionHistoryEntry[];
};

/** Wider tab shape needed by loadConversation — adds the fields it resets
 *  when re-hydrating from disk. Structural; must remain a subset of TabState. */
export type LoadableTab = SaveableTab & {
  tasks: { id: string; content: string; status: "pending" | "in_progress" | "completed" }[];
  lastError: string | null;
  totalCostUsd: number | null;
  resetUsage(): void;
  promptHistory: string[];
  dockAutoOpenedThisConvo: boolean;
};

/** Subset of AssistantStore that persistence touches. */
export type PersistenceHost = {
  conversations: ConversationMeta[];
  tabs: Map<string, SaveableTab>;
  currentConvoId: string | null;
  currentCliSessionId: string | null;
  activeTab: SaveableTab | null;
  model: string;
  openTabs: string[];
  panes: PaneState[];
  focusedPaneIdx: number;
  convoTitle: string | null;
  convoCreatedAt: number | null;
  lastError: string | null;
  lastNotice: string | null;
  streaming: boolean;
  messages: ChatMessage[];
  queue: { id: string; text: string }[];
  ui: { historyOpen: boolean; dockOpen: boolean };
  stop(): Promise<void>;
  ensureTab(convoId: string, cliSessionId: string): LoadableTab;
  setModel(m: ModelSel): void;
  closeTab(id: string): Promise<void>;
  dropTab(id: string): void;
};

export async function refreshConversations(host: PersistenceHost): Promise<void> {
  try {
    host.conversations = await invoke<ConversationMeta[]>("assistant_list_conversations");
  } catch (e) {
    console.warn("assistant_list_conversations failed", e);
  }
}

/** Derive a human-friendly title from the first user message. #145: takes the
 *  tab as an arg so a debounced doSave reads from the originating tab's
 *  messages, not whichever tab is active when the timer fires. */
function deriveTitle(tab: SaveableTab): string {
  const first = tab.messages.find((m) => m.role === "user");
  if (!first) return "New conversation";
  const text = first.blocks
    .map((b) => (b.type === "text" ? b.text : ""))
    .join("")
    .trim()
    .replace(/\s+/g, " ");
  return text.length > 60 ? text.slice(0, 60) + "…" : text || "New conversation";
}

/** Build the on-disk record for a single tab. Shared by flushNow + scheduleSave
 *  so snapshot semantics live in one place. #145: cliSessionId / createdAt /
 *  title all sourced from the tab passed in, not store-level — debounced save
 *  can't redirect mid-flight. */
export function buildSaveRecord(
  host: PersistenceHost,
  convoId: string,
  tab: SaveableTab,
): ConversationRecord {
  return {
    id: convoId,
    title: tab.convoTitle ?? deriveTitle(tab),
    model: host.model,
    createdAt: tab.convoCreatedAt ?? Date.now(),
    updatedAt: Date.now(),
    messages: tab.messages,
    cliSessionId: tab.cliSessionId || convoId,
    compactionHistory: tab.compactionHistory.length > 0 ? tab.compactionHistory : undefined,
  };
}

/** Best-effort synchronous flush of all open tabs. Wired to `beforeunload` —
 *  without this, a window close within the scheduleSave 700ms debounce loses
 *  the last turn. Fires IPC without awaiting (browser drops pending promises
 *  on unload); Tauri runtime typically completes the in-flight invoke before
 *  process exit. #145: iterate every tab with content, not just the active one. */
export function flushNow(host: PersistenceHost): void {
  for (const [convoId, tab] of host.tabs) {
    if (tab.messages.length === 0) continue;
    if (tab.saveTimer) {
      clearTimeout(tab.saveTimer);
      tab.saveTimer = null;
    }
    const record = buildSaveRecord(host, convoId, tab);
    tab.convoTitle = record.title;
    tab.convoCreatedAt = record.createdAt;
    void invoke("assistant_save_conversation", { convo: record }).catch((e) => {
      console.warn("flushNow save failed", e);
    });
  }
}

/** Persist the active tab's conversation. Debounced — callers fire freely;
 *  one disk write per ~700ms per tab. `flush=true` writes immediately. #145:
 *  snapshots (tab, convoId) at call time so a 700ms delay can't dispatch the
 *  save against whichever tab is active when the timer fires. */
export function scheduleSave(host: PersistenceHost, flush = false): void {
  const convoId = host.currentConvoId;
  const tab = host.activeTab;
  if (!tab || !convoId || tab.messages.length === 0) return;
  if (tab.saveTimer) {
    clearTimeout(tab.saveTimer);
    tab.saveTimer = null;
  }
  const doSave = async () => {
    tab.saveTimer = null;
    const record = buildSaveRecord(host, convoId, tab);
    tab.convoTitle = record.title;
    tab.convoCreatedAt = record.createdAt;
    try {
      await invoke("assistant_save_conversation", { convo: record });
      await refreshConversations(host);
    } catch (e) {
      console.warn("assistant_save_conversation failed", e);
    }
  };
  if (flush) void doSave();
  else tab.saveTimer = setTimeout(doSave, 700);
}

export async function renameConversation(
  host: PersistenceHost,
  id: string,
  title: string,
): Promise<void> {
  const trimmed = title.trim();
  if (!trimmed) return;
  try {
    const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
    convo.title = trimmed.slice(0, 120);
    convo.updatedAt = Date.now();
    await invoke("assistant_save_conversation", { convo });
    if (host.currentConvoId === id) host.convoTitle = convo.title;
    await refreshConversations(host);
  } catch (e) {
    host.lastError = `Failed to rename conversation: ${String(e)}`;
  }
}

export async function loadConversation(host: PersistenceHost, id: string): Promise<void> {
  if (host.streaming) await host.stop();
  if (host.messages.length > 0 && host.currentConvoId && host.currentConvoId !== id) {
    scheduleSave(host, true);
  }
  try {
    const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
    // Legacy convos lack cliSessionId — fall back to id so --resume still hits
    // the original JSONL. New convos persist cliSessionId explicitly.
    const cliSid = convo.cliSessionId ?? convo.id;
    const tab = host.ensureTab(convo.id, cliSid);
    tab.messages = convo.messages ?? [];
    tab.cliSessionId = cliSid;
    tab.compactionHistory = convo.compactionHistory ?? [];
    tab.tasks = [];
    tab.lastError = null;
    tab.totalCostUsd = null;
    tab.resetUsage();
    tab.promptHistory = (convo.messages ?? [])
      .filter((m) => m.role === "user")
      .map((m) => m.blocks.map((b) => (b.type === "text" ? b.text : "")).join("").trim())
      .filter((s) => s.length > 0)
      .slice(-50);
    tab.dockAutoOpenedThisConvo = false;
    host.currentConvoId = convo.id;
    host.currentCliSessionId = cliSid;
    host.convoCreatedAt = convo.createdAt;
    host.convoTitle = convo.title;
    host.queue = [];
    host.lastNotice = null;
    host.ui.historyOpen = false;
    if (
      convo.model === "sonnet" || convo.model === "opus" ||
      convo.model === "claude-opus-4-7" || convo.model === "haiku"
    ) {
      host.setModel(convo.model);
    }
  } catch (e) {
    host.lastError = `Failed to load conversation: ${String(e)}`;
  }
}

export async function deleteConversation(host: PersistenceHost, id: string): Promise<void> {
  try {
    await invoke("assistant_delete_conversation", { id });
    if (host.openTabs.includes(id)) {
      // Reuse closeTab so neighbor-pick + active-switch logic stays in one place.
      await host.closeTab(id);
    } else if (host.currentConvoId === id) {
      host.dropTab(id);
      host.currentConvoId = null;
      host.currentCliSessionId = null;
      host.convoCreatedAt = null;
      host.convoTitle = null;
    } else {
      // Convo was open as a TabState (background) but not the active tab.
      host.dropTab(id);
    }
    await refreshConversations(host);
    // #149: an openTab(id) that started before the delete may have racy-pushed
    // id into openTabs while we were awaiting the IPC; close it now so the user
    // doesn't end up with a tab pointing at a deleted convo.
    if (host.openTabs.includes(id)) {
      await host.closeTab(id);
    }
  } catch (e) {
    host.lastError = `Failed to delete conversation: ${String(e)}`;
  }
}

export function persistTabs(host: PersistenceHost): void {
  try {
    localStorage.setItem(
      "rift.ui.tabs.v1",
      JSON.stringify({
        openTabs: host.openTabs,
        activeTabId: host.currentConvoId,
        panes: host.panes,
        focusedPaneIdx: host.focusedPaneIdx,
      }),
    );
  } catch { /* localStorage unavailable */ }
}
