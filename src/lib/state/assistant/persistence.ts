// M5 (per docs/design/assistant-svelte-split.md) — conversation persistence
// IPC + tab-list localStorage lifted out of `src/lib/state/assistant.svelte.ts`
// as free fns operating on a host ref. Follows M3/M4 precedent: $state fields
// (conversations, currentConvoId, openTabs, panes, focusedPaneIdx, convoTitle,
// per-tab saveTimer/messages/convoTitle/convoCreatedAt/cliSessionId)
// STAY where they are; only the IPC + serialization logic moves here.
//
// Scope (this pass): refreshConversations, buildSaveRecord, flushNow,
// scheduleSave, renameConversation, persistTabs. loadConversation +
// deleteConversation stay on AssistantStore — they orchestrate
// stop/ensureTab/closeTab across modules that haven't been extracted yet
// (M6 tabs lifecycle). Move them in M5b after M6 lands.

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { asModelSel } from "./helpers";
import type {
  ChatMessage,
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
  titleGenerated: boolean;
  modelOverride: ModelSel | null;
  lastTurnUsage: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  // Per-project scope: the folder this tab's turns run in. Stamped onto the
  // saved record so the sidebar can filter chats to the open project.
  workspaceRoot: string | null;
};

/** Wider tab shape needed by loadConversation — adds the fields it resets
 *  when re-hydrating from disk. Structural; must remain a subset of TabState. */
export type LoadableTab = SaveableTab & {
  tasks: { id: string; content: string; status: "pending" | "in_progress" | "completed" }[];
  sessionCwd: string | null;
  workspaceRoot: string | null;
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
  messages: ChatMessage[];
  queue: { id: string; text: string }[];
  // Effective folder of the focused tab — fallback scope for a save whose tab
  // has no explicit per-pane root (matches AssistantStore.activeRoot).
  activeRoot: string | null;
  ensureTab(convoId: string, cliSessionId: string): LoadableTab;
  closeTab(id: string): Promise<void>;
  dropTab(id: string): void;
};

// Monotonic load token per host — guards loadConversation against a stale-IPC
// race: click A then B, A's invoke resolves last → without this it clobbers
// host state with A while the user last selected B (next send targets A's
// session). Each call captures the token; only the latest may write host fields.
const loadGeneration = new WeakMap<PersistenceHost, number>();

export async function refreshConversations(host: PersistenceHost): Promise<void> {
  try {
    host.conversations = await invoke<ConversationMeta[]>("assistant_list_conversations");
  } catch (e) {
    console.warn("assistant_list_conversations failed", e);
    host.lastError = `Failed to refresh conversations: ${String(e)}`;
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
    model: tab.modelOverride ?? host.model,
    createdAt: tab.convoCreatedAt ?? Date.now(),
    updatedAt: Date.now(),
    messages: tab.messages,
    cliSessionId: tab.cliSessionId || convoId,
    lastTurnUsage: tab.lastTurnUsage ?? undefined,
    // Scope the convo to the tab's folder (or the focused root as fallback) so
    // the sidebar shows it only under its project. null = unfiled.
    workspaceRoot: tab.workspaceRoot ?? host.activeRoot ?? null,
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
export function scheduleSave(host: PersistenceHost, flush = false, forConvoId?: string): void {
  // bg-tab fix: persist the tab named by `forConvoId` (the tab whose turn just
  // completed), not whichever tab is active when the call or the 700ms debounce
  // fires. Defaults to the active convo for the common foreground callers.
  const convoId = forConvoId ?? host.currentConvoId;
  const tab = convoId ? host.tabs.get(convoId) ?? null : null;
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
      // Fire-and-forget: once the first real exchange is on disk, replace the
      // raw-first-message title with a model-generated one. Never blocks save.
      void maybeGenerateTitle(host, convoId, tab);
    } catch (e) {
      console.warn("assistant_save_conversation failed", e);
    }
  };
  if (flush) void doSave();
  else tab.saveTimer = setTimeout(doSave, 700);
}

/** One-shot smart-title pass. Replaces the `deriveTitle` raw-first-message
 *  title with a 3-6 word model-generated phrase after the first assistant
 *  turn lands. Guarded by `tab.titleGenerated` so it runs at most once per
 *  conversation; claims the flag up-front so the debounced save loop can't
 *  fire a second concurrent call while the model request is in flight. */
async function maybeGenerateTitle(
  host: PersistenceHost,
  convoId: string,
  tab: SaveableTab,
): Promise<void> {
  if (tab.titleGenerated) return;
  const firstUser = tab.messages.find((m) => m.role === "user");
  const hasAssistant = tab.messages.some((m) => m.role === "assistant");
  // Only title a real exchange — a lone user message isn't worth a model call.
  if (!firstUser || !hasAssistant) return;
  const text = firstUser.blocks
    .map((b) => (b.type === "text" ? b.text : ""))
    .join("")
    .trim();
  if (!text) return;
  // Claim the slot before awaiting so a save firing mid-flight no-ops here.
  tab.titleGenerated = true;
  try {
    const title = (await invoke<string>("assistant_generate_title", { prompt: text })).trim();
    if (!title) return;
    tab.convoTitle = title;
    // Re-persist with the new title + refresh so tiles/History pick it up.
    const record = buildSaveRecord(host, convoId, tab);
    await invoke("assistant_save_conversation", { convo: record });
    await refreshConversations(host);
  } catch (e) {
    // A failed model call shouldn't retry-spam every subsequent save — the
    // derived title stays as the fallback. Flag remains true on purpose.
    console.warn("assistant_generate_title failed", e);
  }
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
    // Manual title wins — block any pending auto-gen from clobbering it.
    const renamedTab = host.tabs.get(id);
    if (renamedTab) renamedTab.titleGenerated = true;
    await refreshConversations(host);
  } catch (e) {
    host.lastError = `Failed to rename conversation: ${String(e)}`;
  }
}

export async function loadConversation(host: PersistenceHost, id: string): Promise<void> {
  // Streams are per-tab (routed by session_id) — switching must NOT stop the
  // outgoing tab's in-flight turn; it keeps streaming in the background.
  if (host.messages.length > 0 && host.currentConvoId && host.currentConvoId !== id) {
    scheduleSave(host, true);
  }
  // A live TabState is authoritative over its disk snapshot (disk is only ever
  // written FROM memory) — re-hydrating would clobber an in-flight bg stream's
  // messages + streaming indexes. Pointer-switch instead.
  if (host.tabs.get(id)) {
    host.currentConvoId = id;
    host.lastNotice = null;
    return;
  }
  const gen = (loadGeneration.get(host) ?? 0) + 1;
  loadGeneration.set(host, gen);
  try {
    const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
    // A newer loadConversation started while this IPC was in flight — discard
    // this stale result so the last click wins (Tauri has no invoke-cancel).
    if (loadGeneration.get(host) !== gen) return;
    // Legacy convos lack cliSessionId — fall back to id so --resume still hits
    // the original JSONL. New convos persist cliSessionId explicitly.
    const cliSid = convo.cliSessionId ?? convo.id;
    const tab = host.ensureTab(convo.id, cliSid);
    tab.messages = convo.messages ?? [];
    tab.cliSessionId = cliSid;
    tab.tasks = [];
    tab.lastError = null;
    tab.totalCostUsd = null;
    tab.resetUsage();
    // #32: hydrate the ctx meter from the saved final-turn usage — without
    // this a restored convo shows a blank gauge until the next turn lands.
    tab.lastTurnUsage = convo.lastTurnUsage ?? null;
    // #30: resumed sessions stay pinned to their original folder — fetch the
    // pin so the tabs bar can badge a cwd that differs from the workspace.
    tab.sessionCwd = null;
    void invoke<string | null>("assistant_session_cwd", { id: cliSid })
      .then((cwd) => {
        tab.sessionCwd = cwd ?? null;
        // A resumed convo's folder is its pinned cwd — surface it as the tab's
        // root so the per-pane display + @-mention walk reflect where its turns
        // actually run (only when no explicit per-tab root is already set).
        if (cwd && !tab.workspaceRoot) tab.workspaceRoot = cwd;
      })
      .catch((e) => console.warn("assistant_session_cwd lookup failed:", e));
    tab.promptHistory = (convo.messages ?? [])
      .filter((m) => m.role === "user")
      // blocks is unvalidated passthrough from Rust (serde_json::Value) — a
      // legacy/corrupt record can omit it; guard so one bad message can't throw.
      .map((m) => (m.blocks ?? []).map((b) => (b.type === "text" ? b.text : "")).join("").trim())
      .filter((s) => s.length > 0)
      .slice(-50);
    tab.dockAutoOpenedThisConvo = false;
    // Disk record already carries a title — don't regenerate on every open.
    tab.titleGenerated = true;
    host.currentConvoId = convo.id;
    host.currentCliSessionId = cliSid;
    host.convoCreatedAt = convo.createdAt;
    host.convoTitle = convo.title;
    host.queue = [];
    host.lastNotice = null;
    // ui-audit #5: the saved model scopes to THIS tab only — opening an old
    // chat must not rewrite the global new-chat default (or toast about it).
    tab.modelOverride = asModelSel(convo.model);
  } catch (e) {
    host.lastError = `Failed to load conversation: ${String(e)}`;
    // ensureTab already registered a half-built TabState under `id`; evict it so
    // the fast-path above doesn't surface the broken tab forever (a retry would
    // otherwise short-circuit before the disk load). Next open re-attempts.
    host.dropTab(id);
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

export async function deleteAllConversations(host: PersistenceHost): Promise<void> {
  const ids = host.conversations.map((c) => c.id);
  if (ids.length === 0) return;
  try {
    for (const id of ids) {
      await invoke("assistant_delete_conversation", { id });
    }
    // Wipe to a clean slate — drop every open tab + reset active-convo fields.
    // dropTab (not closeTab) since there's no neighbor worth picking after a purge.
    for (const id of [...host.openTabs]) host.dropTab(id);
    host.openTabs = [];
    host.currentConvoId = null;
    host.currentCliSessionId = null;
    host.convoCreatedAt = null;
    host.convoTitle = null;
    host.queue = [];
    host.lastNotice = null;
    host.panes = [{ tabId: null }];
    host.focusedPaneIdx = 0;
  } catch (e) {
    host.lastError = `Failed to delete all conversations: ${String(e)}`;
  } finally {
    // Re-sync from backend regardless — a mid-loop failure leaves some convos
    // deleted server-side, so the in-memory list must reflect what survived.
    await refreshConversations(host);
  }
}

// #37: namespace the open-tabs record per window label. Two windows share the
// same web origin, so a single key would let them stomp each other's tab list.
// The `main` window keeps the legacy key for backward compat; secondary windows
// (`window-<n>`) get a per-label suffix.
export function tabsStorageKey(): string {
  let label = "main";
  try { label = getCurrentWindow().label; } catch { /* non-Tauri / SSR */ }
  return label === "main" ? "rift.ui.tabs.v1" : `rift.ui.tabs.${label}.v1`;
}

export function persistTabs(host: PersistenceHost): void {
  try {
    localStorage.setItem(
      tabsStorageKey(),
      JSON.stringify({
        openTabs: host.openTabs,
        activeTabId: host.currentConvoId,
        panes: host.panes,
        focusedPaneIdx: host.focusedPaneIdx,
      }),
    );
  } catch { /* localStorage unavailable */ }
}
