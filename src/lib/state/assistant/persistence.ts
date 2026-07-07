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
  QueueItem,
} from "./types";

/** Subset of TabState fields the save plumbing reads/writes. Structural —
 *  no class import, matches the live $state fields declared on TabState. */
type SaveableTab = {
  messages: ChatMessage[];
  saveTimer: ReturnType<typeof setTimeout> | null;
  convoTitle: string | null;
  convoCreatedAt: number | null;
  lastActivityAt: number | null;
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
type LoadableTab = SaveableTab & {
  tasks: { id: string; content: string; status: "pending" | "in_progress" | "completed" }[];
  sessionCwd: string | null;
  workspaceRoot: string | null;
  lastError: string | null;
  totalCostUsd: number | null;
  resetUsage(): void;
  promptHistory: string[];
  dockAutoOpenedThisConvo: boolean;
  pinnedModel: ModelSel | null;
};

/** Subset of AssistantStore that persistence touches. */
type PersistenceHost = {
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
  queue: QueueItem[];
  // Effective folder of the focused tab — fallback scope for a save whose tab
  // has no explicit per-pane root (matches AssistantStore.activeRoot).
  activeRoot: string | null;
  // The GLOBAL workspace default root (AssistantStore.workspace.current). The
  // correct fallback when saving a tab that has no per-tab root: `activeRoot`
  // is scoped to the FOCUSED pane, so using it would stamp a background/unfiled
  // tab with an unrelated project's folder.
  workspaceCurrent: string | null;
  ensureTab(convoId: string, cliSessionId: string): LoadableTab;
  closeTab(id: string): Promise<void>;
  dropTab(id: string): void;
  pruneTabUi(id: string): void;
};

// Monotonic load token per host — guards loadConversation against a stale-IPC
// race: click A then B, A's invoke resolves last → without this it clobbers
// host state with A while the user last selected B (next send targets A's
// session). Each call captures the token; only the latest may write host fields.
const loadGeneration = new WeakMap<PersistenceHost, number>();

// Tombstones for convos deleted this session. A save racing past its delete —
// the 700ms debounce timer, or maybeGenerateTitle's in-flight model call —
// silently re-creates the file on disk, leaving an undeletable ghost row in the
// sidebar. Every save path checks this before writing.
const deletedIds = new Set<string>();

// cont.275 hunt: the tombstone stops saves that CHECK after the delete, but a
// save already past its check races the delete on the Rust blocking pool
// (spawn_blocking tasks have no FIFO order) — its rename can land AFTER the
// remove and resurrect the file as an undeletable ghost that reappears next
// launch (deletedIds is per-session). Track in-flight writes per convo so the
// delete paths can drain them before removing the file.
const inflightSaves = new Map<string, Promise<unknown>>();

/** Register a save IPC as in-flight for `convoId` until it settles. Chains on
 *  any prior in-flight write so a drain awaits ALL of them. Returns `write`
 *  unchanged so callers keep their own error handling. */
function trackSave(convoId: string, write: Promise<unknown>): Promise<unknown> {
  const prev = inflightSaves.get(convoId);
  const entry = prev ? Promise.allSettled([prev, write]) : write.catch(() => {});
  inflightSaves.set(convoId, entry);
  void entry.then(() => {
    if (inflightSaves.get(convoId) === entry) inflightSaves.delete(convoId);
  });
  return write;
}

/** Await every tracked in-flight save for `id` (no-op when none). */
async function drainInflightSaves(id: string): Promise<void> {
  const pending = inflightSaves.get(id);
  if (pending) await pending.then(() => {}, () => {});
}

export async function refreshConversations(host: PersistenceHost): Promise<void> {
  try {
    host.conversations = await invoke<ConversationMeta[]>("assistant_list_conversations");
  } catch (e) {
    console.warn("assistant_list_conversations failed", e);
    host.lastError = `Failed to refresh conversations: ${String(e)}`;
  }
}

// Computed once at module init — window label is static for the window lifetime.
const _windowLabel = (() => {
  try { return getCurrentWindow().label; } catch { return "main"; }
})();
const _tabsStorageKey = _windowLabel === "main"
  ? "rift.ui.tabs.v1"
  : `rift.ui.tabs.${_windowLabel}.v1`;

// #37: secondary windows are ephemeral by design — their per-label tab state
// must not resurrect in a later launch (labels now carry a launch nonce, so a
// fresh launch can't collide into an old key anyway). Sweep leftovers once per
// main-window boot so orphaned window-* keys don't accumulate in localStorage.
if (_windowLabel === "main") {
  try {
    for (const k of Object.keys(localStorage)) {
      if (k.startsWith("rift.ui.tabs.window-") && k.endsWith(".v1")) localStorage.removeItem(k);
    }
  } catch {
    // storage unavailable — nothing to prune
  }
}

// #37 cross-window sync: after THIS window mutates the shared conversation store
// (save / delete / rename), tell every other window to re-pull its list so a
// chat created or removed here shows up there without a reload. Fire-and-forget
// — a failed broadcast just means the other window refreshes on its next own
// action, never blocks the mutation that triggered it.
function broadcastConvosChanged(): void {
  void invoke("broadcast_convos_changed", { originLabel: _windowLabel }).catch(() => {});
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
    // Only real turns advance this (send / result), so a tab-switch auto-save
    // bumps updatedAt but leaves the sidebar order untouched. Falls back to
    // creation time so a never-sent draft still sorts sanely.
    lastActivityAt: tab.lastActivityAt ?? tab.convoCreatedAt ?? Date.now(),
    messages: tab.messages,
    cliSessionId: tab.cliSessionId || convoId,
    lastTurnUsage: tab.lastTurnUsage ?? undefined,
    // Scope the convo to the tab's OWN folder, else the GLOBAL workspace
    // default — never `host.activeRoot`, which is the focused pane's root and
    // would misfile a background/unfiled tab under an unrelated project (a
    // background-tab save while another pane is focused elsewhere). null =
    // unfiled, and a genuinely-unfiled tab stays unfiled.
    workspaceRoot: tab.workspaceRoot ?? host.workspaceCurrent ?? null,
  };
}

/** Best-effort synchronous flush of all open tabs. Wired to `beforeunload` —
 *  without this, a window close within the scheduleSave 700ms debounce loses
 *  the last turn. Fires IPC without awaiting (browser drops pending promises
 *  on unload); Tauri runtime typically completes the in-flight invoke before
 *  process exit. #145: iterate every tab with content, not just the active one. */
export function flushNow(host: PersistenceHost): void {
  for (const [convoId, tab] of host.tabs) {
    if (tab.messages.length === 0 || deletedIds.has(convoId)) continue;
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
    if (deletedIds.has(convoId)) return;
    const record = buildSaveRecord(host, convoId, tab);
    tab.convoTitle = record.title;
    tab.convoCreatedAt = record.createdAt;
    try {
      await trackSave(convoId, invoke("assistant_save_conversation", { convo: record }));
      await refreshConversations(host);
      broadcastConvosChanged(); // #37: other windows pick up the new/updated chat
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
    if (!title || deletedIds.has(convoId)) return;
    tab.convoTitle = title;
    // Re-persist with the new title + refresh so tiles/History pick it up.
    const record = buildSaveRecord(host, convoId, tab);
    await trackSave(convoId, invoke("assistant_save_conversation", { convo: record }));
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
  if (!trimmed || deletedIds.has(id)) return;
  const newTitle = trimmed.slice(0, 120);
  try {
    // RR10: a live tab is authoritative over its disk snapshot — load-then-save
    // off disk would drop messages a concurrent turn-save wrote between awaits.
    const liveTab = host.tabs.get(id);
    if (liveTab) {
      liveTab.convoTitle = newTitle;
      liveTab.titleGenerated = true;
      if (host.currentConvoId === id) host.convoTitle = newTitle;
      scheduleSave(host, true, id);
      await refreshConversations(host);
      return;
    }
    const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
    convo.title = newTitle;
    convo.updatedAt = Date.now();
    await trackSave(id, invoke("assistant_save_conversation", { convo }));
    if (host.currentConvoId === id) host.convoTitle = convo.title;
    await refreshConversations(host);
    broadcastConvosChanged(); // #37: propagate the rename to other windows
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
    // Hydrate last-activity from disk so reopening doesn't reset the order;
    // legacy records lacking it fall back to their createdAt.
    tab.lastActivityAt = convo.lastActivityAt ?? convo.createdAt;
    tab.tasks = [];
    tab.lastError = null;
    tab.totalCostUsd = null;
    tab.resetUsage();
    // #32: hydrate the ctx meter from the saved final-turn usage — without
    // this a restored convo shows a blank gauge until the next turn lands.
    tab.lastTurnUsage = convo.lastTurnUsage ?? null;
    // The record's workspaceRoot was SAVED (buildSaveRecord) but never read
    // back — a restored convo silently lost its per-tab root, so turns ran
    // root-less (no workspace MCP tools, builtin reads against the wrong cwd;
    // found live cont.269 when a reload made Read return another project's
    // README). Applied BEFORE the async session-cwd lookup below, whose
    // `!tab.workspaceRoot` guard then only fills legacy records lacking it.
    tab.workspaceRoot = convo.workspaceRoot ?? null;
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
    // A resumed convo is pinned backend-side to the model it was created with
    // (turn.rs load_session_model), so its saved model IS its pinned model —
    // hydrate it so the picker's "this session" tag + "New chat in <model>"
    // honesty works on reopened chats too.
    tab.pinnedModel = asModelSel(convo.model);
  } catch (e) {
    host.lastError = `Failed to load conversation: ${String(e)}`;
    // ensureTab already registered a half-built TabState under `id`; evict it so
    // the fast-path above doesn't surface the broken tab forever (a retry would
    // otherwise short-circuit before the disk load). Next open re-attempts.
    host.dropTab(id);
  }
}

export async function deleteConversation(host: PersistenceHost, id: string): Promise<void> {
  // Tombstone FIRST so a debounced save / title-gen landing mid-delete can't
  // re-create the file. Rolled back only on a failed delete.
  deletedIds.add(id);
  // Then drain any write that already passed its tombstone check — without
  // this its rename can land after the remove and resurrect the file.
  await drainInflightSaves(id);
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
    broadcastConvosChanged(); // #37: propagate the delete to other windows
    // #149: an openTab(id) that started before the delete may have racy-pushed
    // id into openTabs while we were awaiting the IPC; close it now so the user
    // doesn't end up with a tab pointing at a deleted convo.
    if (host.openTabs.includes(id)) {
      await host.closeTab(id);
    }
  } catch (e) {
    deletedIds.delete(id); // delete failed — the convo still exists; let saves resume
    host.lastError = `Failed to delete conversation: ${String(e)}`;
  }
}

export async function deleteAllConversations(host: PersistenceHost): Promise<void> {
  const ids = host.conversations.map((c) => c.id);
  if (ids.length === 0) return;
  // RR10: cancel every armed save debounce first — an orphaned timer firing
  // after the delete loop would resurrect a just-deleted convo file.
  for (const [, tab] of host.tabs) {
    if (tab.saveTimer) {
      clearTimeout(tab.saveTimer);
      tab.saveTimer = null;
    }
  }
  // allSettled (not Promise.all): a partial backend failure must still tear down
  // the tabs whose convo WAS deleted — Promise.all rejects on the first failure
  // and skips teardown, leaving tabs that point at deleted sessions (a later send
  // then --resumes a dead JSONL).
  // Drain in-flight writes for every id first — same resurrect race as the
  // single-delete path (a write past its tombstone check outliving the remove).
  await Promise.allSettled(
    ids.flatMap((id) => { const p = inflightSaves.get(id); return p ? [p] : []; }),
  );
  const results = await Promise.allSettled(
    ids.map((id) => invoke("assistant_delete_conversation", { id })),
  );
  const deletedOk = new Set(ids.filter((_, i) => results[i]?.status === "fulfilled"));
  for (const id of deletedOk) deletedIds.add(id); // tombstone vs racing saves
  const failed = ids.length - deletedOk.size;
  if (failed > 0) {
    host.lastError = `Failed to delete ${failed} of ${ids.length} conversation(s)`;
  }
  // Re-arm the save debounce for survivors whose delete FAILED — their timer
  // was cleared up front (RR10) but never fires again otherwise, silently
  // dropping any pending edits.
  for (const id of ids) {
    if (deletedOk.has(id)) continue;
    const tab = host.tabs.get(id);
    if (tab && tab.messages.length > 0) scheduleSave(host, false, id);
  }
  // Drop only the tabs whose convo was actually deleted; survivors stay open.
  // dropTab (not closeTab) since there's no neighbor worth picking after a purge.
  for (const id of [...host.openTabs]) {
    if (deletedOk.has(id)) { host.dropTab(id); host.pruneTabUi(id); }
  }
  host.openTabs = host.openTabs.filter((id) => !deletedOk.has(id));
  if (host.currentConvoId && deletedOk.has(host.currentConvoId)) {
    host.currentConvoId = null;
    host.currentCliSessionId = null;
    host.convoCreatedAt = null;
    host.convoTitle = null;
    host.queue = [];
    host.lastNotice = null;
  }
  if (host.openTabs.length === 0) {
    host.panes = [{ tabId: null }];
    host.focusedPaneIdx = 0;
  } else {
    host.panes = host.panes.map((p) => (p.tabId && deletedOk.has(p.tabId) ? { tabId: null } : p));
  }
  // Re-sync from backend regardless — reflects exactly what survived.
  await refreshConversations(host);
  broadcastConvosChanged(); // #37: propagate the purge to other windows
}

// #37: namespace the open-tabs record per window label. Two windows share the
// same web origin, so a single key would let them stomp each other's tab list.
// The `main` window keeps the legacy key for backward compat; secondary windows
// (`window-<n>`) get a per-label suffix. Key is cached at module init.
export function tabsStorageKey(): string {
  return _tabsStorageKey;
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
