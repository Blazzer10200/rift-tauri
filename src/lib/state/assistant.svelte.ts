// Assistant page state.
//
// Spawns the user's installed `claude` CLI through Rust commands; the CLI
// streams NDJSON which the backend forwards verbatim on `assistant://stream`.
// Wires Rift's MCP server (read_file / list_dir / grep) so assistant turns
// can interleave text, tool calls, and TodoWrite-driven task lists.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "./toast.svelte";

// M0 split (2026-05-26): type defs lifted to `./assistant/types`. Re-exported
// here so external callers like `import type { Block } from "$lib/state/assistant.svelte"`
// keep working. See `docs/design/assistant-svelte-split.md`.
export type {
  WorkspaceState,
  AuthStatus,
  ClaudeInstall,
  ToolBlock,
  TextBlock,
  ThinkingBlock,
  BoundaryBlock,
  ImageBlock,
  Block,
  ChatMessage,
  ConversationMeta,
  SummarizeResult,
  ThinkingEffort,
  PaneState,
} from "./assistant/types";
export { MAX_PANES } from "./assistant/types";
import type {
  WorkspaceState,
  AuthStatus,
  ToolBlock,
  ThinkingBlock,
  BoundaryBlock,
  ImageBlock,
  Block,
  ChatMessage,
  ConversationMeta,
  SummarizeResult,
  CompactionHistoryEntry,
  ConversationRecord,
  ContentBlock,
  StreamDelta,
  StreamEvent,
  StreamEnvelope,
  ThinkingEffort,
  ModelSel,
  PermissionMode,
  TrustLevel,
  PermissionPromptInfo,
  PermissionSuggestion,
  TurnRecord,
  PaneState,
} from "./assistant/types";
import { MAX_PANES } from "./assistant/types";

// M1 split (2026-05-26): helpers lifted to `./assistant/helpers`. Re-export
// the one externally-imported symbol so call sites stay unchanged.
import {
  loadModel,
  saveModel,
  loadEffort,
  saveEffort,
  loadPermissionMode,
  savePermissionMode,
  loadDockWidth,
  messagesHaveContextSignals,
} from "./assistant/helpers";
export { messagesHaveContextSignals } from "./assistant/helpers";

// M2 split (2026-05-26): SessionTelemetry class lifted to `./assistant/telemetry`.
import { SessionTelemetry } from "./assistant/telemetry";
// M4 split (2026-05-26): attachment free fns in `./assistant/attachments`.
import {
  addAttachment as attAdd,
  removeAttachment as attRemove,
  clearAttachments as attClear,
} from "./assistant/attachments";
// M3 split (2026-05-26): workspace free fns in `./assistant/workspace`.
import {
  refreshWorkspace as wsRefresh,
  pickFolder as wsPickFolder,
  setRoot as wsSetRoot,
  clearRoot as wsClearRoot,
  removeRecentRoot as wsRemoveRecentRoot,
  loadWorkspaceFiles as wsLoadFiles,
  loadWorkspaceBranch as wsLoadBranch,
} from "./assistant/workspace";
// M5 split (2026-05-26): conversation persistence + tab-list save in
// `./assistant/persistence`. loadConversation + deleteConversation stay on
// the class (M5b — gated on M6 tabs lifecycle extraction).
import {
  refreshConversations as persistRefresh,
  buildSaveRecord as persistBuildRecord,
  flushNow as persistFlushNow,
  scheduleSave as persistSchedule,
  renameConversation as persistRename,
  persistTabs as persistTabsImpl,
  loadConversation as persistLoad,
  deleteConversation as persistDelete,
  deleteAllConversations as persistDeleteAll,
} from "./assistant/persistence";
import { saveSessionLog, pruneSessionLogs, ingestUsage } from "./assistant/sessionLog";
// M6 split (2026-05-27): tab lifecycle + split-pane management in
// `./assistant/tabs`. The TabState registry (ensureTab/dropTab/wireTab/
// tabByCliSession) + scroll cache stay on the class; only the lifecycle
// logic moves. Store methods below are thin thunks onto these.
import {
  addPane as tabsAddPane,
  closePane as tabsClosePane,
  setFocusedPane as tabsSetFocusedPane,
  dropTabIntoPane as tabsDropTabIntoPane,
  restoreTabs as tabsRestore,
  openTab as tabsOpenTab,
  closeTab as tabsCloseTab,
  newTab as tabsNewTab,
  clearConversation as tabsClearConversation,
  reorderTabs as tabsReorder,
  cycleTab as tabsCycle,
  closeOtherTabs as tabsCloseOthers,
  closeAllTabs as tabsCloseAll,
  closeTabsToRight as tabsCloseToRight,
} from "./assistant/tabs";
// M7 split (2026-05-27): summarize + compact pipeline in `./assistant/compaction`.
// Per-tab compaction fields stay on TabState; threshold/model setters stay on
// the store. The two pipeline methods below are thin thunks onto these.
import {
  summarizeCurrentSession as compactSummarize,
  compactConversation as compactRun,
} from "./assistant/compaction";
// M8 split (2026-06-09): the stream pump (envelope/delta parsing, thinking
// timing, tool→activity routing, ask_user FIFO binding, usage accounting,
// rAF text pacer) in `./assistant/streaming`. TabState keeps the $state
// fields + IoC hooks; its methods below are thin thunks onto these.
import {
  beginTurn as streamBeginTurn,
  drainTick as streamDrainTick,
  flushPendingText as streamFlushPendingText,
  tryBindAskUser as streamTryBindAskUser,
  onStreamLine as streamOnLine,
  onStreamDone as streamOnDone,
  onStreamError as streamOnError,
} from "./assistant/streaming";
// M9 split (2026-06-09): send orchestrator (turn dispatch + slash commands +
// queue drain + steer + stop + retry/copy/recall) in `./assistant/send`.
// enhancePrompt + the turn-complete hook wiring stay on the store.
import {
  send as sendImpl,
  drainQueue as sendDrainQueue,
  stop as sendStop,
  steer as sendSteer,
  removeQueued as sendRemoveQueued,
  retryLast as sendRetryLast,
  copyLastAssistant as sendCopyLastAssistant,
  recallPrompt as sendRecallPrompt,
} from "./assistant/send";

/** Per-conversation streaming state. One TabState per open chat tab; the
 *  AssistantStore holds a Map keyed by Rift convoId and delegates all
 *  per-stream reads/writes (messages, activity, tasks, etc.) to the active
 *  tab's state via getters. Concurrent live streaming on 2+ tabs works
 *  because each tab has its own messages array + pacer state + thinking
 *  tracking, and backend events route by session_id to the right tab.
 *
 *  Cross-cutting effects (dock open, tasksUpdatedAt bump, queue drain,
 *  scheduleSave) reach back into AssistantStore via the callback hooks
 *  set when the tab is created. */
export class TabState {
  /** CLI session UUID — every assistant://stream|done|error event carries
   *  this so the store can dispatch to the right tab. Mutable: compaction
   *  remints it without destroying the TabState.
   *  #143: now $state so reassignment (compaction reminting) is reactive. */
  cliSessionId = $state<string>("");
  /** #143: per-tab convo metadata. Was store-level before; moved to TabState
   *  so a 700ms scheduleSave debounce can't dispatch against whichever tab
   *  is active when the timer fires. */
  convoCreatedAt = $state<number | null>(null);
  convoTitle = $state<string | null>(null);
  /** Guards the one-shot smart-title generation (assistant_generate_title).
   *  False on a fresh tab → generated once after the first assistant turn;
   *  set true on disk-load (record already has a title) + manual rename so
   *  auto-gen never clobbers an existing/user-chosen title. In-memory only. */
  titleGenerated = $state<boolean>(false);
  /** #145: per-tab save debounce timer — was store-level (single slot).
   *  Each tab tracking its own timer means flushNow() on beforeunload can
   *  iterate every unsaved tab instead of dropping background-tab edits. */
  saveTimer: ReturnType<typeof setTimeout> | null = null;

  messages = $state<ChatMessage[]>([]);
  streaming = $state(false);
  tasks = $state<{ id: string; content: string; status: "pending" | "in_progress" | "completed" }[]>([]);
  taskCreateCount = $state(0);
  activity = $state<{ currentLabel: string | null; turnStartedAt: number | null }>({
    currentLabel: null,
    turnStartedAt: null,
  });
  lastError = $state<string | null>(null);
  totalCostUsd = $state<number | null>(null);
  lastTurnUsage = $state<{ input: number; output: number; cacheRead: number; cacheCreate: number } | null>(null);
  sessionUsage = $state({ totalInput: 0, totalOutput: 0, totalCacheRead: 0, totalCacheCreate: 0, turns: 0 });
  lastModelId = $state<string | null>(null);
  promptHistory = $state<string[]>([]);
  /** Outbound message queue for THIS tab. send() pushes here when the tab is
   *  already streaming; onDone() pops the next one. Per-tab so a queued msg
   *  in Tab A can't drain into Tab B if the user switches mid-turn. */
  queue = $state<{ id: string; text: string }[]>([]);
  /** Per-tab composer draft. Was store-level before split-pane v2 — moved
   *  here so each pane can compose into its own tab concurrently w/o the
   *  focus-change stash/restore dance dropping characters under fast typing.
   *  Composer binds via `bind:value={tab.draft}`. */
  draft = $state<string>("");
  /** Per-tab staged attachments. Same rationale as `draft`. send() snapshots
   *  + clears on dispatch. 20MiB cumulative cap enforced by addAttachment. */
  attachments = $state<{ id: string; mime: string; dataBase64: string; sizeBytes: number }[]>([]);
  /** Compaction Phase C: summary seeded by compactConversation() that the
   *  next send() drains into the `prior_context_summary` invoke arg. Null
   *  outside of the one-turn post-compaction window. Per-tab so concurrent
   *  compactions on different tabs don't cross-contaminate. */
  pendingCompactionSummary = $state<string | null>(null);
  /** S124 fix: scheduleSave's doSave() unconditionally writes
   *  `tab.convoCreatedAt = record.createdAt` and buildSaveRecord falls
   *  back to Date.now() when null, so compaction's `convoCreatedAt = null`
   *  gets clobbered before the next send reads it — causing isFirstTurn
   *  to be false → --resume on a non-existent new JSONL → session-lost
   *  recovery → priorSummary lost. This flag is the authoritative signal
   *  to send() that the next dispatch MUST be first-turn regardless of
   *  convoCreatedAt's persistence-driven value. Cleared on first read. */
  forceNextFirstTurn = $state(false);
  /** Compaction Phase D guard: prevents auto-trigger from re-firing on a
   *  failed compaction (ctx pill stays high → effect re-runs). Set when
   *  compactConversation() starts; cleared on success OR failure. */
  compactingNow = $state(false);
  /** Compaction Phase D cooldown: wall-clock ms of the last successful
   *  compaction. The auto-trigger effect checks this against a 5min floor
   *  before re-firing — a failed compaction at $0.91 × runaway = real
   *  money, so erring long. */
  lastCompactionAt = $state(0);
  /** Phase E prerequisite: structured log of compactions on this tab. Pushed
   *  by compactConversation() on success; hydrated from ConversationRecord on
   *  load. Persists alongside messages. */
  compactionHistory = $state<CompactionHistoryEntry[]>([]);
  /** S124: in-flight sub-agent spawns. Pushed on Task tool_use, marked done
   *  on the matching tool_result. CLI does NOT stream intermediate sub-agent
   *  activity — we only know spawn + final result. */
  agentSpawns = $state<{
    id: string;
    subagentType: string;
    description: string;
    startedAt: number;
    completedAt: number | null;
    isError: boolean;
  }[]>([]);

  /** Live bindings for `mcp__rift__ask_user` tool calls: toolUseId →
   *  bridge requestId. Populated when the tool_use envelope and the
   *  `assistant://ask-user` event have BOTH arrived for the same call; the
   *  two arrive in arbitrary order so the FIFO buffers below absorb whichever
   *  lands first. Reactive so ToolChip's `$derived` lookup refreshes when a
   *  late binding lands. Entry stays until the user answers; after the
   *  matching tool_result arrives the chip switches to "done" and the binding
   *  is no longer read. */
  askUserBindings = $state<Map<string, string>>(new Map());
  /** FIFO of request_ids whose `assistant://ask-user` event arrived before
   *  the matching tool_use envelope (bridge faster than CLI stdout). */
  unboundAskUserRequestIds: string[] = [];
  /** FIFO of ask_user toolUseIds whose request_id hasn't shown up yet. */
  unboundAskUserToolUseIds: string[] = [];

  /** Live `can_use_tool` permission asks for this tab: toolUseId → info. The
   *  control-channel ask carries the tool_use_id directly (it pairs to the
   *  already-streamed tool chip), so unlike ask_user no FIFO reordering is
   *  needed. Reactive so ToolChip's `$derived` lookup activates the moment the
   *  ask lands. Entry removed once the user decides (or the turn ends). */
  permissionPrompts = $state<Map<string, PermissionPromptInfo>>(new Map());

  // Non-reactive per-stream internals.
  streamingMsgId: string | null = null;
  // #146/#234: cached index of the streaming assistant msg so mutateStreaming
  // can index-replace instead of full-map. Set in send() right after the
  // placeholder push; cleared wherever streamingMsgId is cleared.
  streamingMsgIdx: number | null = null;
  seenToolUseIds = new Set<string>();
  deltaCount = 0;
  envelopeTextBuffer = "";
  rawLineLog: string[] = [];
  pendingText = "";
  drainHandle: ReturnType<typeof requestAnimationFrame> | null = null;
  lastDrainAt = 0;
  thinkingByIndex = new Map<number, { blockOffset: number; startedAt: number }>();
  activeThinkingIndex: number | null = null;
  /** Wall-clock of the most recent `stream_event` arrival. Null between turns.
   *  Used to compute `maxStreamGapMs` on the in-flight TurnRecord. */
  lastStreamEventAt: number | null = null;
  dockAutoOpenedThisConvo = false;
  /** Telemetry record for the in-flight turn. Set by AssistantStore.send()
   *  before invoking the backend, filled by stream handlers, finalized in
   *  onDone / onError. Null between turns. */
  currentTurnRecord: TurnRecord | null = null;

  /** Fired after a TodoWrite tool_use lands. Store uses it to bump
   *  `ui.tasksUpdatedAt` and auto-open the dock the first time per convo. */
  onTodoApplied?: (tab: TabState, opensDock: boolean) => void;
  /** Fired on onDone — store handles scheduleSave + queue drain. */
  onTurnComplete?: (tab: TabState) => void;
  /** Translates a tool name + input into a short activity-bar label.
   *  Lives on the store (knows nothing tab-specific); passed in via this hook
   *  so TabState doesn't grow its own copy. */
  shortToolLabel?: (name: string, input?: Record<string, unknown>) => string;

  constructor(cliSessionId: string) {
    this.cliSessionId = cliSessionId;
  }

  resetUsage() {
    this.lastTurnUsage = null;
    this.sessionUsage = { totalInput: 0, totalOutput: 0, totalCacheRead: 0, totalCacheCreate: 0, turns: 0 };
    this.lastModelId = null;
  }

  /** Called at the start of every send(). Clears per-turn pacer / thinking
   *  / dedupe state and flips streaming on. M8: body in ./assistant/streaming. */
  beginTurn() {
    streamBeginTurn(this);
  }

  /** rAF pacer callback — a stable per-tab arrow so enqueue/re-arm target one
   *  identity across frames. Public b/c the streaming module re-arms via
   *  `requestAnimationFrame(tab.drainTick)`. */
  drainTick = () => {
    streamDrainTick(this);
  };

  flushPendingText() {
    streamFlushPendingText(this);
  }

  /** Drain the two ask_user FIFOs — see ./assistant/streaming. */
  tryBindAskUser() {
    streamTryBindAskUser(this);
  }

  onStream(raw: string) {
    streamOnLine(this, raw);
  }

  onDone() {
    streamOnDone(this);
  }

  onError(msg: string) {
    streamOnError(this, msg);
  }
}

// 2a: one saved custom-provider endpoint. Secret stays in the OS keychain on
// the Rust side; `hasKey` is the only signal the renderer gets about it.
export type ProviderDto = {
  id: string;
  name: string;
  baseUrl: string;
  model: string | null;
  hasKey: boolean;
  active: boolean;
};

class AssistantStore {
  auth = $state<AuthStatus | null>(null);
  authChecking = $state(false);
  authError = $state<string | null>(null);
  /** True while an in-app `claude auth login` is running + being polled. Drives
   *  the recovery banner's "Signing in…" state. */
  loginInProgress = $state(false);
  /** Epoch ms of the last completed auth probe (success or failure). Drives the
   *  "last checked Xm ago" freshness label in Settings → Assistant. */
  authLastProbed = $state<number | null>(null);

  /** Per-conversation streaming state, keyed by Rift convoId. One entry per
   *  open chat tab. The store's UI-facing `messages` / `streaming` / `activity`
   *  / etc. getters delegate to `activeTab`; event handlers route by
   *  `session_id` to whichever tab owns that CLI session. Concurrent live
   *  streaming on 2+ tabs works because each tab carries its own messages
   *  buffer, pacer state, and thinking tracker. */
  // M5: relaxed from `private` so extracted persistence module can iterate.
  // Still internal-by-convention — no external module reads `assistant.tabs`.
  tabs = $state(new Map<string, TabState>());

  /** The TabState bound to `currentConvoId`, or null if no tab is active.
   *  Getter-derived so it tracks both `tabs` and `currentConvoId` reactively. */
  get activeTab(): TabState | null {
    return this.currentConvoId ? this.tabs.get(this.currentConvoId) ?? null : null;
  }

  // ── Per-tab UI surface — delegated getters so components read
  //    `assistant.messages` etc. exactly like before. Sentinel defaults
  //    keep empty-state renders safe when no tab is active.
  get messages(): ChatMessage[] { return this.activeTab?.messages ?? []; }
  get streaming(): boolean { return this.activeTab?.streaming ?? false; }
  get tasks() { return this.activeTab?.tasks ?? []; }

  // Phase D: ctx-pill derivations lifted off AssistantHeader so the
  // auto-trigger $effect can read them. Header consumes assistant.ctxPct etc.
  get ctxWindow(): number { return this.ctxWindowFor(this.activeTab); }
  get ctxTokens(): number { return this.ctxTokensFor(this.activeTab); }
  get ctxPct(): number { return this.ctxPctFor(this.activeTab); }

  /** Per-tab ctx helpers — let the auto-compact effect iterate `panes[]` so
   *  a background-pane tab can't sail past the threshold silently. */
  ctxWindowFor(tab: TabState | null): number {
    const model = tab?.lastModelId ?? null;
    if (!model) return 200_000;
    if (/\[1m\]/i.test(model)) return 1_000_000;
    const id = model.toLowerCase();
    if (id.includes("haiku")) return 200_000;
    if (/sonnet-4-[56]/.test(id) || /opus-4-[678]/.test(id)) return 1_000_000;
    return 200_000;
  }
  ctxTokensFor(tab: TabState | null): number {
    const u = tab?.lastTurnUsage ?? null;
    return u ? u.input + u.cacheRead + u.cacheCreate : 0;
  }
  ctxPctFor(tab: TabState | null): number {
    const w = this.ctxWindowFor(tab);
    return w > 0 ? Math.min(100, (this.ctxTokensFor(tab) / w) * 100) : 0;
  }
  /** Pre-emption banner text — non-null when ctx is within 10pp of the
   *  user's auto-compact threshold but hasn't crossed yet. Lets the user
   *  compact early w/ a focus string if they want fine control. */
  get compactWarning(): string | null {
    const t = this.autoCompactThreshold;
    if (!t) return null;
    const tab = this.activeTab;
    if (!tab || tab.compactingNow) return null;
    const pct = this.ctxPct;
    const threshPct = t * 100;
    if (pct >= threshPct) return null; // crossed — the effect handles it
    if (pct < threshPct - 10) return null;
    return `Approaching auto-compact at ${Math.round(threshPct)}%`;
  }
  get activity() {
    return this.activeTab?.activity ?? { currentLabel: null, turnStartedAt: null };
  }
  /** Fallback for store-level errors (auth, workspace, delete, etc.) when
   *  no tab is active. Tab errors take precedence when a tab is present. */
  private storeLastError = $state<string | null>(null);
  get lastError(): string | null {
    return this.activeTab ? this.activeTab.lastError : this.storeLastError;
  }
  set lastError(v: string | null) {
    if (this.activeTab) this.activeTab.lastError = v;
    else this.storeLastError = v;
  }
  get totalCostUsd(): number | null { return this.activeTab?.totalCostUsd ?? null; }
  get lastTurnUsage() { return this.activeTab?.lastTurnUsage ?? null; }
  get sessionUsage() {
    return this.activeTab?.sessionUsage ?? { totalInput: 0, totalOutput: 0, totalCacheRead: 0, totalCacheCreate: 0, turns: 0 };
  }
  get lastModelId(): string | null { return this.activeTab?.lastModelId ?? null; }
  get promptHistory(): string[] { return this.activeTab?.promptHistory ?? []; }
  get queue() { return this.activeTab?.queue ?? []; }
  set queue(v: { id: string; text: string }[]) {
    if (this.activeTab) this.activeTab.queue = v;
  }

  /** Public read accessor for a tab's state — used by AssistantPane /
   *  StatusHub in split mode so each pane scopes its rendering to its own
   *  tab rather than the activeTab. Returns null for unknown ids. */
  tabFor(id: string | null): TabState | null {
    return id ? this.tabs.get(id) ?? null : null;
  }

  get splitActive(): boolean {
    return this.panes.length > 1;
  }

  /** Returns the focused pane's tabId. Always defined (panes always length≥1). */
  get focusedPaneTabId(): string | null {
    return this.panes[this.focusedPaneIdx]?.tabId ?? this.currentConvoId;
  }

  get canAddPane(): boolean {
    return this.panes.length < MAX_PANES;
  }

  /** Add a new pane to the right of the focused one. Caps at MAX_PANES.
   *  New pane is auto-filled with the next openTab not already in any pane,
   *  else stays empty (drop a tab in from the tabsbar). Focus moves to new
   *  pane. Persists. */
  addPane() { tabsAddPane(this); this.drainQueue(this.activeTab); }

  /** Close a pane (the pane container, not the tab inside it). Tabs stay in
   *  openTabs — closing a pane just unhooks it. Last pane never closes (always
   *  length≥1). Focused idx is clamped to the new array bounds. Persists. */
  closePane(idx: number) { tabsClosePane(this, idx); }

  /** Move focus to a pane. Stashes outgoing composer draft + restores incoming
   *  so each pane carries its own draft. No-op in single-pane mode. */
  setFocusedPane(idx: number) { tabsSetFocusedPane(this, idx); this.drainQueue(this.activeTab); }

  /** Drop a tab from the tabsbar into a specific pane. See tabs.ts for the
   *  single→split / sibling-swap / end-sentinel behavior. */
  dropTabIntoPane(tabId: string, paneIdx: number) { tabsDropTabIntoPane(this, tabId, paneIdx); }

  /** Look up the TabState whose CLI session matches the event's session_id.
   *  Linear scan over open tabs is fine — typical user has <10. */
  private tabByCliSession(sid: string): TabState | null {
    for (const t of this.tabs.values()) {
      if (t.cliSessionId === sid) return t;
    }
    return null;
  }

  /** Get-or-create the TabState for a convo. Used by send() on first turn
   *  and by tab lifecycle methods. Reassigning the map triggers reactivity. */
  // M5b: relaxed from `private` so persistence module's loadConversation can call.
  ensureTab(convoId: string, cliSessionId: string): TabState {
    const existing = this.tabs.get(convoId);
    if (existing) return existing;
    const tab = new TabState(cliSessionId);
    this.wireTab(tab);
    const next = new Map(this.tabs);
    next.set(convoId, tab);
    this.tabs = next;
    return tab;
  }

  /** Tear down a tab's TabState. Called from closeTab / closeAllTabs. */
  // M5b: relaxed from `private` so persistence module's deleteConversation can call.
  dropTab(convoId: string) {
    const tab = this.tabs.get(convoId);
    if (!tab) return;
    // #139: cancel the outstanding rAF + finalize any pending text BEFORE
    // removing the tab from the map. Without this, drainTick continues firing
    // on the next frame and writes to the dropped TabState — pendingText
    // grows in a tab that nobody renders, and the rAF chain self-perpetuates
    // (drainTick re-arms itself at the tail).
    tab.flushPendingText();
    const next = new Map(this.tabs);
    next.delete(convoId);
    this.tabs = next;
  }

  /** Attach cross-cutting hooks to a freshly-minted TabState. */
  private wireTab(tab: TabState) {
    tab.shortToolLabel = (name, input) => this.shortToolLabel(name, input);
    tab.onTodoApplied = (_t, opensDock) => {
      this.ui.tasksUpdatedAt = Date.now();
      if (opensDock) this.ui.dockOpen = true;
    };
    tab.onTurnComplete = (t) => this.handleTurnComplete(t);
  }

  // Informational system notice (slash-command output, /help text, etc.).
  // Rendered as a dismissible info banner separate from error styling.
  // Cross-cutting (not per-tab) — same notice shows regardless of which tab
  // is active because it's typically user-action-triggered (slash command,
  // workspace change, etc.).
  lastNotice = $state<string | null>(null);

  /** Session-wide telemetry — every turn structurally captured + UI lifecycle
   *  events. Drained via `/diag` slash command (clipboard JSON). Reset via
   *  `/diag-clear`. Non-reactive: callers don't render off this, they only
   *  serialize-and-export. */
  telemetry = new SessionTelemetry();
  /** Debounce handle for session-log persistence (see recordSessionLog). */
  private sessionLogTimer: ReturnType<typeof setTimeout> | null = null;

  /** Persist the current session's telemetry snapshot to disk (debounced).
   *  Fired after every completed turn so a crash/HMR reload loses at most the
   *  in-flight turn. Skips empty sessions so dev hot-reloads don't litter the
   *  log dir with zero-turn files. `flush=true` writes immediately (window
   *  close). Best-effort: saveSessionLog swallows IPC errors to a warn. */
  recordSessionLog(flush = false): void {
    if (this.telemetry.turns.length === 0) return;
    const write = () => {
      this.sessionLogTimer = null;
      const snap = this.telemetry.snapshot();
      const payload = {
        ...snap,
        model: this.model,
        workspace: this.workspace.current,
      };
      void saveSessionLog(payload);
      // Mirror finalized turns into the durable SQLite usage store before the
      // session-log ring buffer can prune them (cost cockpit, §1a).
      void ingestUsage(payload);
    };
    if (this.sessionLogTimer) {
      clearTimeout(this.sessionLogTimer);
      this.sessionLogTimer = null;
    }
    if (flush) write();
    else this.sessionLogTimer = setTimeout(write, 1500);
  }

  /** Phase 6 (#37): the value never crosses IPC — only whether one is set. */
  hasApiKey = $state<boolean>(false);
  /** True once the api-key presence probe has resolved. Gates the first-run
   *  onboarding so it never flashes before we know whether a key is set. */
  configLoaded = $state<boolean>(false);
  useFullConfig = $state<boolean>(true);
  maxBudgetUsd = $state<number | null>(null);
  // Trust level gating the local git tools (mcp__rift__git_*). Loaded from the
  // backend; defaults to "readonly" when unset. Settings seg treats full ⊇ standard.
  trustLevel = $state<TrustLevel>("readonly");
  // Phase D: fraction (0-1] of ctx window that auto-fires compactConversation.
  // Null = manual only (matches the user's DISABLE_AUTO_COMPACT=1 stance).
  autoCompactThreshold = $state<number | null>(null);
  // Phase D: model alias used by summarize call. "haiku" default ($0.91 vs
  // $2.73 on sonnet for a 900K-token summarize).
  compactModel = $state<"haiku" | "sonnet">("haiku");
  // June-15 hedge (legacy single-provider — superseded by `providers`; kept so
  // any pre-2a config still reads). null = Anthropic.
  baseUrl = $state<string | null>(null);
  providerModel = $state<string | null>(null);
  // 2a multi-provider list (cc-switch pattern). Empty = Anthropic only. The
  // `active` one routes turns; secrets live in the keychain (see ProviderDto).
  providers = $state<ProviderDto[]>([]);
  activeProvider = $derived(this.providers.find((p) => p.active) ?? null);
  // Flips true after providers (+ legacy fields) are fetched — distinct from
  // configLoaded (set earlier), so the Settings draft-seed waits for real values.
  providerConfigLoaded = $state<boolean>(false);
  // 3c compression toggle (headroom-style local proxy). Off by default; the
  // Python proxy runtime is a soft dep Rift never bundles or spawns. When on
  // AND no custom provider is active, turns route through the proxy via
  // ANTHROPIC_BASE_URL. `compressionProxyUrl` null = use `compressionDefaultUrl`.
  compressionEnabled = $state<boolean>(false);
  compressionProxyUrl = $state<string | null>(null);
  compressionDefaultUrl = $state<string>("http://127.0.0.1:8787");

  // The Assistant's open project folder + recent-folder list. Decoupled from
  // Sync's server folders; populated by `assistant_get_workspace` on init and
  // updated whenever the user opens, switches, or clears a folder. Empty
  // `current` falls back to AutoSync folders on the Rust side.
  workspace = $state<WorkspaceState>({ current: null, recent: [] });

  // Cached relative file paths under the workspace root, populated on first
  // `@` trigger and re-loaded whenever the workspace root changes. Drives the
  // composer's `@`-file mention picker. Walk is cheap (~ms for typical FiveM
  // resource folder) so we re-fetch on each open rather than invalidate via
  // a watcher.
  workspaceFiles = $state<string[]>([]);
  workspaceFilesLoadingFor = $state<string | null>(null);
  workspaceBranch = $state<string | null>(null);

  // composerDraft + composerAttachments live on TabState in v2.1 split-pane.
  // These getter/setter shims delegate to the focused-pane's tab so non-pane
  // call-sites (slash commands, EmptyState fallback, telemetry, send()) keep
  // working unchanged. Pane-aware components (Composer) bind to `tab.draft`
  // directly so each pane composes into its own tab concurrently.
  get composerDraft(): string { return this.activeTab?.draft ?? ""; }
  set composerDraft(v: string) { if (this.activeTab) this.activeTab.draft = v; }
  get composerAttachments(): { id: string; mime: string; dataBase64: string; sizeBytes: number }[] {
    return this.activeTab?.attachments ?? [];
  }
  set composerAttachments(v: { id: string; mime: string; dataBase64: string; sizeBytes: number }[]) {
    if (this.activeTab) this.activeTab.attachments = v;
  }
  // queue moved to TabState (S105 follow-up) — per-tab so a queued msg in
  // Tab A can't drain into Tab B if the user switches mid-turn. UI binds via
  // the `queue` getter below which delegates to activeTab.
  // User's chosen model — flipped by /model slash command. Carried through
  // to assistant_send so the CLI uses sonnet/opus/haiku per their choice.
  // Initialized from localStorage so the choice survives reloads.
  model = $state<ModelSel>(loadModel());
  // Extended-thinking budget tier. "none" = no extended thinking (fastest);
  // "quick" = 2K budget (default, balanced); "deep" = 10K (heavy reasoning).
  // Haiku ignores this server-side. Persisted to localStorage.
  thinkingEffort = $state<ThinkingEffort>(loadEffort());
  // Permission mode passed to the CLI's `--permission-mode`. Global (matches
  // model/effort). `bypassPermissions` until the user picks otherwise so
  // existing behavior is unchanged. Persisted to localStorage.
  permissionMode = $state<PermissionMode>(loadPermissionMode());
  // `dockOpen` drives the inline TasksDock in AssistantPage. `historyOpen`
  // is retained as a no-op flag for back-compat w/ any remaining slash
  // command — History is now its own workspace, not an overlay.
  // dockOpen defaults true — the activity dock is a permanent surface now (not a
  // toggle-to-peek panel). New/clear no longer force it shut; the Composer
  // affordance still hides it on demand.
  ui = $state({ dockOpen: true, tasksUpdatedAt: 0, historyOpen: false, panelTab: "session" as "session" | "activity", dockWidth: loadDockWidth(), diffOpen: false, diffTarget: null as string | null });

  // Conversation history.
  //   - `currentConvoId` is null before the first message is sent; first
  //     `send()` assigns a fresh UUID and persists from there.
  //   - `conversations` is the metadata cache for the drawer; refreshed
  //     after every save/delete/rename.
  //   - `createdAt` is set when the convo starts, kept stable across saves.
  //   - `openTabs` (v0.4) is the ordered list of convo ids visible as tabs in
  //     the top tab bar. Tabs share the singleton stream pipeline (mid-stream
  //     switch = stop stream; concurrent live UI deferred to v0.4.1).
  currentConvoId = $state<string | null>(null);
  conversations = $state<ConversationMeta[]>([]);
  openTabs = $state<string[]>([]);
  /** v2 split-pane state. Always an array of length 1..MAX_PANES. Length 1 =
   *  single-pane (no visible split). `currentConvoId` always mirrors
   *  `panes[focusedPaneIdx].tabId` so existing send/openTab/closeTab paths
   *  keep working without per-pane branching. */
  panes = $state<PaneState[]>([{ tabId: null }]);
  focusedPaneIdx = $state(0);
  /** Set on tab dragstart so AssistantPane can render drop affordance.
   *  Cleared on dragend. Cross-component drag state. */
  draggingTabId = $state<string | null>(null);

  // #143: currentCliSessionId / convoCreatedAt / convoTitle now live on
  // TabState. These getters/setters delegate to the active tab so existing
  // call sites keep working unchanged. Writes when activeTab is null no-op,
  // which matches the prior teardown pattern (dropTab → store-field=null
  // was effectively clearing already-gone state).
  get currentCliSessionId(): string | null {
    const t = this.activeTab;
    return t ? (t.cliSessionId || null) : null;
  }
  set currentCliSessionId(v: string | null) {
    if (this.activeTab) this.activeTab.cliSessionId = v ?? "";
  }
  // M5b: relaxed for persistence module loadConversation host access.
  get convoCreatedAt(): number | null { return this.activeTab?.convoCreatedAt ?? null; }
  set convoCreatedAt(v: number | null) {
    if (this.activeTab) this.activeTab.convoCreatedAt = v;
  }
  // M5: relaxed from `private` so persistence module can read/write through host ref.
  get convoTitle(): string | null { return this.activeTab?.convoTitle ?? null; }
  set convoTitle(v: string | null) {
    if (this.activeTab) this.activeTab.convoTitle = v;
  }

  // tasks + activity now live on TabState (see top-of-class getters).

  // Per-tab UI state. Draft + attachments live on TabState directly (split-
  // pane v2.1: each pane composes into its own tab concurrently). Only scroll
  // is still kept here — it's a transient DOM measurement, not user input.
  private tabScroll = new Map<string, number>();

  private unlistens: UnlistenFn[] = [];
  // #177: keep the beforeunload listener reachable for removal in destroy().
  // Anonymous closures used to leak across HMR cycles.
  private beforeUnloadHandler: (() => void) | null = null;
  // #185: re-entrance latch for retryLast — fast double-click would
  // otherwise pop two user+assistant pairs.
  /** #185 retry re-entrance guard — public so the M9 send module can gate on it. */
  retrying = false;
  // streamingMsgId / seenToolUseIds / dockAutoOpenedThisConvo / deltaCount /
  // envelopeTextBuffer / rawLineLog / pendingText / drainHandle / lastDrainAt /
  // thinkingByIndex / activeThinkingIndex now live on TabState.

  setModel(v: ModelSel) {
    if (this.model === v) return;
    const prev = this.model;
    this.model = v;
    saveModel(v, this.workspace.current);
    const midConvo = (this.activeTab?.messages.length ?? 0) > 0;
    this.telemetry.event("model.change", { from: prev, to: v, midConvo });
    if (midConvo) this.cacheBustHint("model");
  }

  setThinkingEffort(v: ThinkingEffort) {
    if (this.thinkingEffort === v) return;
    const prev = this.thinkingEffort;
    this.thinkingEffort = v;
    saveEffort(v, this.workspace.current);
    const midConvo = (this.activeTab?.messages.length ?? 0) > 0;
    this.telemetry.event("effort.change", { from: prev, to: v, midConvo });
    if (midConvo) this.cacheBustHint("effort");
  }

  setPermissionMode(v: PermissionMode) {
    if (this.permissionMode === v) return;
    const prev = this.permissionMode;
    this.permissionMode = v;
    savePermissionMode(v);
    this.telemetry.event("permission_mode.change", { from: prev, to: v });
  }

  /** One-shot-per-session-per-kind notice when model/effort flips on a tab
   *  that already has turns. Sonnet's cache empirically does NOT survive
   *  effort changes (S106 measurement: 0 cacheRead on 3 consecutive sonnet
   *  turns w/ effort flips vs healthy reuse without). Opus is more forgiving.
   *  Notice is fire-once so it's a hint, not a nag. */
  private cacheBustHintShown = { model: false, effort: false };
  private cacheBustHint(kind: "model" | "effort") {
    if (this.cacheBustHintShown[kind]) return;
    this.cacheBustHintShown[kind] = true;
    // Ephemeral heads-up → toast stack (top-right), not the composer notice
    // banner. It's a transient FYI, not a blocking notice, so it auto-dismisses
    // and stays out of the chat column.
    toast.push({
      severity: "info",
      // icon omitted — ToastHost supplies the info-severity default (CR5: keeps
      // lucide-svelte UI imports out of this state module).
      title: kind === "effort" ? "Effort changed mid-conversation" : "Model switched mid-conversation",
      detail: kind === "effort"
        ? "May bust the prompt cache (esp. Sonnet) — next turn could pay full cache_create."
        : "Rebuilds the prefix cache — next turn will pay full cache_create.",
    });
  }

  // Draft + attachments live on TabState directly in v2.1 — focus changes no
  // longer need to stash/restore. stashTabUi/restoreTabUi are kept as no-ops
  // so the call-sites in addPane/closePane/setFocusedPane/dropTabIntoPane
  // don't need surgery; the per-tab fields already carry the right value.
  // M6: relaxed from `private` so the tabs module calls them through the host ref.
  stashTabUi(_id: string | null) { /* no-op since v2.1 */ }
  restoreTabUi(_id: string | null) { /* no-op since v2.1 */ }

  /** AssistantPage writes the active tab's scrollTop here on scroll, then
   *  reads it back on tab activation. Kept in the store so it survives
   *  remounts without re-querying the DOM. */
  setTabScroll(id: string, top: number) {
    this.tabScroll.set(id, top);
  }
  getTabScroll(id: string): number | undefined {
    return this.tabScroll.get(id);
  }

  /** Drop all per-tab UI scratch for a closed tab. Draft + attachments live
   *  on TabState now, so dropTab() teardown handles those; only scroll cache
   *  needs explicit pruning here. */
  // M6: relaxed from `private` so the tabs module calls it through the host ref.
  pruneTabUi(id: string) {
    this.tabScroll.delete(id);
  }

  clearQueue() {
    this.queue = [];
  }

  async init() {
    if (this.unlistens.length > 0) return;
    // Backend tags every stream/done/error event w/ the originating CLI
    // session_id (S104). We route by session_id to the right TabState so
    // background tabs can keep painting concurrently with the foreground.
    // Legacy payload shape (bare string) routes to activeTab for forward-
    // compat during dev hot-reload.
    this.unlistens.push(
      await listen<{ session_id?: string; line?: string } | string>(
        "assistant://stream",
        (e) => {
          if (typeof e.payload === "string") {
            this.activeTab?.onStream(e.payload);
            return;
          }
          const { session_id, line } = e.payload ?? {};
          const tab = session_id ? this.tabByCliSession(session_id) : this.activeTab;
          if (tab && typeof line === "string") tab.onStream(line);
        },
      ),
      await listen<{ session_id?: string; exit_code?: number }>(
        "assistant://done",
        (e) => {
          const p = e.payload;
          const sid = p?.session_id;
          const tab = sid ? this.tabByCliSession(sid) : this.activeTab;
          tab?.onDone();
        },
      ),
      await listen<{ session_id?: string; message?: string } | string>(
        "assistant://error",
        (e) => {
          if (typeof e.payload === "string") {
            this.activeTab?.onError(e.payload);
            return;
          }
          const { session_id, message } = e.payload ?? {};
          const tab = session_id ? this.tabByCliSession(session_id) : this.activeTab;
          if (tab && typeof message === "string") tab.onError(message);
        },
      ),
    );
    await this.refreshAuth();
    try {
      this.hasApiKey = await invoke<boolean>("assistant_get_api_key_present");
    } catch (e) {
      console.warn("assistant_get_api_key_present failed", e);
    } finally {
      this.configLoaded = true;
    }
    try {
      this.useFullConfig = await invoke<boolean>("assistant_get_use_full_config");
    } catch (e) {
      console.warn("assistant_get_use_full_config failed", e);
    }
    try {
      this.maxBudgetUsd = await invoke<number | null>("assistant_get_max_budget_usd");
    } catch (e) {
      console.warn("assistant_get_max_budget_usd failed", e);
    }
    try {
      this.trustLevel = await invoke<TrustLevel>("assistant_get_trust_level");
    } catch (e) {
      console.warn("assistant_get_trust_level failed", e);
    }
    try {
      this.autoCompactThreshold = await invoke<number | null>("assistant_get_auto_compact_threshold");
    } catch (e) {
      console.warn("assistant_get_auto_compact_threshold failed", e);
    }
    try {
      const m = await invoke<string>("assistant_get_compact_model");
      this.compactModel = m === "sonnet" ? "sonnet" : "haiku";
    } catch (e) {
      console.warn("assistant_get_compact_model failed", e);
    }
    try {
      this.baseUrl = await invoke<string | null>("assistant_get_base_url");
    } catch (e) {
      console.warn("assistant_get_base_url failed", e);
    }
    try {
      this.providerModel = await invoke<string | null>("assistant_get_provider_model");
    } catch (e) {
      console.warn("assistant_get_provider_model failed", e);
    }
    try {
      this.providers = await invoke<ProviderDto[]>("assistant_list_providers");
    } catch (e) {
      console.warn("assistant_list_providers failed", e);
    } finally {
      this.providerConfigLoaded = true;
    }
    try {
      const c = await invoke<{ enabled: boolean; proxyUrl: string | null; defaultUrl: string }>(
        "assistant_get_compression",
      );
      this.compressionEnabled = c.enabled;
      this.compressionProxyUrl = c.proxyUrl;
      this.compressionDefaultUrl = c.defaultUrl;
    } catch (e) {
      console.warn("assistant_get_compression failed", e);
    }
    this.unlistens.push(
      await listen<{ session_id: string; prompt: string }>(
        "assistant://session-lost",
        (e) => this.onSessionLost(e.payload),
      ),
      await listen<{ request_id: string; session_id: string; questions: unknown }>(
        "assistant://ask-user",
        (e) => this.onAskUser(e.payload),
      ),
      await listen<{
        request_id: string;
        session_id: string;
        tool_use_id: string;
        tool_name: string;
        input: unknown;
        suggestions: PermissionSuggestion[] | null;
      }>(
        "assistant://permission-request",
        (e) => this.onPermissionRequest(e.payload),
      ),
    );

    await this.refreshConversations();
    await this.refreshWorkspace();
    await this.restoreTabs();

    // Trim the persisted session-log ring buffer on launch (keep recent 40).
    void pruneSessionLogs(40);

    // Best-effort flush on window close so we don't lose the last turn
    // sitting inside the 700ms scheduleSave debounce. See flushNow() doc.
    // #177: store the handler so destroy() can remove it; anonymous arrow
    // would leak across HMR cycles.
    if (typeof window !== "undefined") {
      this.beforeUnloadHandler = () => this.flushNow();
      window.addEventListener("beforeunload", this.beforeUnloadHandler);
    }

    // #180: HMR teardown — clear listeners on module dispose so a fresh
    // init() during dev hot-reload starts from a clean slate.
    if (typeof import.meta !== "undefined" && (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot) {
      (import.meta as { hot: { dispose: (cb: () => void) => void } }).hot.dispose(() => this.destroy());
    }
  }

  /** Tear down all listeners + handlers. Safe to call multiple times.
   *  Wired automatically via `import.meta.hot.dispose` so HMR doesn't stack
   *  duplicate listeners. AppShell may also call this on unmount. */
  destroy() {
    for (const u of this.unlistens) {
      try { u(); } catch (e) { console.warn("[assistant] unlisten threw", e); }
    }
    this.unlistens = [];
    if (this.beforeUnloadHandler && typeof window !== "undefined") {
      window.removeEventListener("beforeunload", this.beforeUnloadHandler);
      this.beforeUnloadHandler = null;
    }
  }

  /** Auto-recovery: claude's --resume index lost track of our session JSONL.
   *  Pop the failed user+assistant message pair, null convoCreatedAt so the
   *  next send uses --session-id, surface a friendly notice, then re-send
   *  the prompt. Tab-aware: ignore if the lost session isn't current
   *  (user switched tabs while the error was in flight). */
  private onSessionLost(payload: { session_id: string; prompt: string }) {
    // Find the tab whose CLI session failed (may not be the active tab if the
    // user switched mid-recovery). After S103 decoupling cliSessionId may
    // differ from convoId (post-compaction).
    const tab = this.tabByCliSession(payload.session_id);
    if (!tab) return;
    this.telemetry.event("session.lost", { sid: payload.session_id, willRetry: tab === this.activeTab });
    if (tab.currentTurnRecord) {
      tab.currentTurnRecord.doneAt = Date.now();
      tab.currentTurnRecord.endKind = "session-lost";
      tab.currentTurnRecord = null;
    }
    tab.streaming = false;
    // Drop the empty assistant message + the user message that failed.
    // send() will re-add them on retry.
    const msgs = tab.messages.slice();
    if (msgs.length >= 2 && msgs[msgs.length - 1].role === "assistant") {
      msgs.pop();
      if (msgs[msgs.length - 1]?.role === "user") msgs.pop();
    }
    tab.messages = msgs;
    tab.streamingMsgId = null;
    tab.streamingMsgIdx = null;
    tab.lastError = null;
    this.lastNotice = "Session was lost — retrying as a fresh start";
    // Auto-retry only when the lost tab is active. Bg-tab retry would require
    // routing send() to a specific tab; for now the user re-clicks send.
    if (this.activeTab === tab) {
      this.convoCreatedAt = null;
      this.convoTitle = null;
      void this.send(payload.prompt);
    }
  }

  /** `assistant://ask-user` arrived from the bridge — pair the request_id
   *  with a pending ask_user tool block in the matching tab. The bridge
   *  emits AFTER it registered the oneshot, so by the time we read here
   *  the parent is already awaiting an answer. */
  private onAskUser(payload: { request_id: string; session_id: string; questions: unknown }) {
    const tab = this.tabByCliSession(payload.session_id);
    if (!tab) {
      // No matching tab — the convo was closed mid-flight. Best-effort
      // cancel by replying w/ cancelled:true so the MCP child unblocks.
      void invoke("assistant_answer_ask_user", {
        requestId: payload.request_id,
        answer: { cancelled: true },
      }).catch(() => { /* parent already timed out — ignore */ });
      return;
    }
    tab.unboundAskUserRequestIds.push(payload.request_id);
    tab.tryBindAskUser();
  }

  /** Look up the bridge request_id for an ask_user tool block in the active
   *  tab. Returns null until the binding lands (one of two arrival orders).
   *  Called from ToolChip.svelte via a `$derived` so the chip activates
   *  the moment its requestId is known. */
  askUserRequestIdFor(toolUseId: string): string | null {
    return this.activeTab?.askUserBindings.get(toolUseId) ?? null;
  }

  /** Submit the user's choice for an `mcp__rift__ask_user` tool call.
   *  Resolves the parent bridge oneshot — the MCP child unblocks, returns
   *  the answer as the tool_result, and the existing stream pipeline
   *  flips the chip to "done" via fillToolResult. No optimistic update
   *  here beyond clearing the binding map; the chip handles its own
   *  "sending…" affordance.
   *
   *  `answer` shape: `{ answers: [{question, answer}, ...] }` for normal
   *  submissions, or `{ cancelled: true }` if the user dismissed. */
  async submitAskUserAnswer(toolUseId: string, answer: Record<string, unknown>): Promise<void> {
    const tab = this.activeTab;
    if (!tab) return;
    const requestId = tab.askUserBindings.get(toolUseId);
    if (!requestId) return;
    try {
      await invoke("assistant_answer_ask_user", { requestId, answer });
    } finally {
      // Pop the binding regardless — re-submission on the same toolUseId
      // would be a UI bug, and the tool_result envelope is the authoritative
      // "done" signal.
      const next = new Map(tab.askUserBindings);
      next.delete(toolUseId);
      tab.askUserBindings = next;
    }
  }

  /** `assistant://permission-request` arrived — the CLI wants to run a gated
   *  tool in a prompting mode. Pair it to the matching tab by tool_use_id so
   *  the streamed tool chip renders Allow / Deny. If the tab is gone (closed
   *  mid-flight), auto-deny so the CLI's control_response doesn't hang. */
  private onPermissionRequest(payload: {
    request_id: string;
    session_id: string;
    tool_use_id: string;
    tool_name: string;
    suggestions: PermissionSuggestion[] | null;
  }) {
    const tab = this.tabByCliSession(payload.session_id);
    if (!tab) {
      void invoke("assistant_answer_permission", {
        requestId: payload.request_id,
        decision: { behavior: "deny", message: "Conversation closed before approval." },
      }).catch(() => { /* turn already ended — ignore */ });
      return;
    }
    const next = new Map(tab.permissionPrompts);
    next.set(payload.tool_use_id, {
      requestId: payload.request_id,
      toolName: payload.tool_name,
      suggestions: payload.suggestions ?? [],
    });
    tab.permissionPrompts = next;
  }

  /** Look up a pending permission ask for a tool block in the active tab.
   *  Called from ToolChip.svelte via a `$derived` so the chip's Allow/Deny
   *  buttons appear the moment the ask lands. */
  permissionPromptFor(toolUseId: string): PermissionPromptInfo | null {
    return this.activeTab?.permissionPrompts.get(toolUseId) ?? null;
  }

  /** Answer a `can_use_tool` ask. `allow` writes `{behavior:"allow"}` (the CLI
   *  reuses the original input); `deny` writes `{behavior:"deny", message}`.
   *  Resolves the backend oneshot, which writes the control_response to the
   *  CLI's stdin and unblocks tool execution. The chip flips to its normal
   *  running/done state via the existing stream pipeline. */
  async submitPermissionDecision(toolUseId: string, allow: boolean): Promise<void> {
    const tab = this.activeTab;
    if (!tab) return;
    const info = tab.permissionPrompts.get(toolUseId);
    if (!info) return;
    const decision = allow
      ? { behavior: "allow" }
      : { behavior: "deny", message: "User declined this action." };
    try {
      await invoke("assistant_answer_permission", { requestId: info.requestId, decision });
    } finally {
      const next = new Map(tab.permissionPrompts);
      next.delete(toolUseId);
      tab.permissionPrompts = next;
    }
  }


  // M3 split (2026-05-26): workspace IPC ops in `./assistant/workspace`.
  // Fields stay on Store; methods become thunks routing to free fns.
  refreshWorkspace() { return wsRefresh(this); }
  pickFolder() { return wsPickFolder(this); }
  /** Re-apply the active workspace's saved model + effort. Called after the
   *  workspace root resolves/changes so each project keeps its own choice
   *  (heavy repo → Sonnet/none; everything else → whatever you last used).
   *  Direct $state writes — no telemetry/cache-bust; this is not a user flip. */
  applyWorkspacePrefs() {
    const ws = this.workspace.current;
    this.model = loadModel(ws);
    this.thinkingEffort = loadEffort(ws);
  }
  setRoot(path: string) { return wsSetRoot(this, path); }
  clearRoot() { return wsClearRoot(this); }
  removeRecentRoot(path: string) { return wsRemoveRecentRoot(this, path); }
  loadWorkspaceFiles() { return wsLoadFiles(this); }
  loadWorkspaceBranch() { return wsLoadBranch(this); }

  refreshConversations() { return persistRefresh(this); }

  // M5: deriveTitle + buildSaveRecord moved to ./assistant/persistence. Kept
  // as private thunk for any in-class callers that still reference it.
  private buildSaveRecord(convoId: string, tab: TabState): ConversationRecord {
    return persistBuildRecord(this, convoId, tab);
  }

  flushNow() { persistFlushNow(this); this.recordSessionLog(true); }

  // M6: relaxed from `private` so the tabs module calls it through the host ref.
  scheduleSave(flush = false, convoId?: string) { persistSchedule(this, flush, convoId); }

  /** Start a fresh conversation. Flushes the current one first so nothing
   *  is lost when the user clicks `+ New`. */
  async newConversation() {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0) this.scheduleSave(true);
    if (this.currentConvoId) this.dropTab(this.currentConvoId);
    this.queue = [];
    this.lastNotice = null;
    this.currentConvoId = null;
    this.currentCliSessionId = null;
    this.convoCreatedAt = null;
    this.convoTitle = null;
  }

  loadConversation(id: string) { return persistLoad(this, id); }
  deleteConversation(id: string) { return persistDelete(this, id); }
  async deleteAllConversations() {
    if (this.streaming) await this.stop();
    await persistDeleteAll(this);
    await this.newTab();
  }

  // ── v0.4 tabs ────────────────────────────────────────────────────────
  // M6: relaxed from `private` so the tabs module calls it through the host ref.
  persistTabs() { persistTabsImpl(this); }

  private restoreTabs() { return tabsRestore(this); }

  /** Open a saved convo as a tab. Push to openTabs if not already there;
   *  activate + load from disk. Unsaved new-tab ids (minted by newTab() but
   *  no send yet → no disk record) drop into a fresh in-memory state instead
   *  of disk-load. Singleton stream pipeline — mid-stream switch is handled
   *  by loadConversation() calling stop(). */
  async openTab(id: string) { await tabsOpenTab(this, id); this.drainQueue(this.activeTab); }

  /** Close a tab. Removes from openTabs; convo stays on disk → still in History.
   *  Active-tab close picks the right neighbor (or left if at end); last-tab
   *  close drops to empty state w/ currentConvoId=null. */
  closeTab(id: string) { return tabsCloseTab(this, id); }

  /** Open a fresh empty tab. Mints currentConvoId up-front so the tab can
   *  render before the first send; convoCreatedAt stays null so send() still
   *  flags isFirstTurn=true and the CLI gets --session-id, not --resume. */
  newTab() { return tabsNewTab(this); }

  /** Clear the active conversation in place (Claude Code `/clear` semantics):
   *  flush the old convo to History, re-key the same tab/pane to a fresh empty
   *  session. Distinct from newTab() — does not append a second tab. */
  clearConversation() { return tabsClearConversation(this); }

  reorderTabs(fromIdx: number, toIdx: number) { tabsReorder(this, fromIdx, toIdx); }

  async cycleTab(direction: 1 | -1) { await tabsCycle(this, direction); this.drainQueue(this.activeTab); }

  closeOtherTabs(keepId: string) { return tabsCloseOthers(this, keepId); }

  /** Wipe all open tabs and drop into the empty-tabs state. */
  closeAllTabs() { return tabsCloseAll(this); }

  closeTabsToRight(anchorId: string) { return tabsCloseToRight(this, anchorId); }

  // ── /v0.4 tabs ───────────────────────────────────────────────────────

  renameConversation(id: string, title: string) { return persistRename(this, id, title); }

  async refreshAuth() {
    this.authChecking = true;
    this.authError = null;
    try {
      this.auth = await invoke<AuthStatus>("assistant_auth_probe");
    } catch (e) {
      this.authError = String(e);
      this.auth = null;
    } finally {
      this.authChecking = false;
      this.authLastProbed = Date.now();
    }
  }

  /** In-app sign-in recovery. Opens the Claude CLI's `auth login` in its own
   *  console (browser OAuth), then polls the auth probe until the session flips
   *  green — at which point the auth error is cleared and the user is told they
   *  can retry. The credentials land in the CLI's own store, so this fixes the
   *  401 for real (probe + every turn read the same store). `useConsole` picks
   *  an Anthropic API account instead of the default claude.ai subscription. */
  async startLogin(useConsole = false) {
    if (this.loginInProgress) return;
    this.loginInProgress = true;
    try {
      await invoke("assistant_open_login", { console: useConsole });
    } catch (e) {
      this.lastError = `Couldn't start sign-in: ${String(e)}`;
      this.loginInProgress = false;
      return;
    }
    // A separate console window opens for the OAuth flow — it can surface behind
    // Rift, so tell the user where to look immediately rather than only on timeout.
    this.lastNotice = "A sign-in window opened — complete the login there, then come back. Rift will detect it automatically.";
    // Poll the probe until the session is usable, or give up after ~3 min so a
    // user who closes the login window without finishing isn't stuck "Signing
    // in…" forever.
    const deadline = Date.now() + 180_000;
    try {
      while (Date.now() < deadline) {
        await new Promise((r) => setTimeout(r, 2500));
        await this.refreshAuth();
        const pill = this.auth?.pill;
        if (pill === "green" || pill === "yellow") {
          this.lastError = null;
          this.lastNotice = "Signed in — you're good to go. Resend your message to continue.";
          return;
        }
      }
      this.lastNotice =
        "Sign-in didn't complete. Finish in the console window that opened (or close it and try again), then resend.";
    } finally {
      this.loginInProgress = false;
    }
  }

  /** Re-probe auth from the recovery banner. Unlike the bare `refreshAuth`, this
   *  clears the error banner when the session comes back usable — so a user who
   *  fixed auth out-of-band (or via the in-app sign-in) sees the wall disappear
   *  instead of a stale 401 lingering until their next send. */
  async recheckAuth() {
    await this.refreshAuth();
    const pill = this.auth?.pill;
    if (pill === "green" || pill === "yellow") {
      this.lastError = null;
      this.lastNotice = "Auth looks good now — resend your message to continue.";
    }
  }

  async setApiKey(key: string | null) {
    const v = key && key.trim().length > 0 ? key.trim() : null;
    try {
      await invoke("assistant_set_api_key", { apiKey: v });
      this.hasApiKey = v !== null;
      await this.refreshAuth();
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async setUseFullConfig(value: boolean) {
    try {
      await invoke("assistant_set_use_full_config", { value });
      this.useFullConfig = value;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async setMaxBudgetUsd(value: number | null) {
    const v = value !== null && Number.isFinite(value) && value > 0 ? value : null;
    try {
      await invoke("assistant_set_max_budget_usd", { value: v });
      this.maxBudgetUsd = v;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async setTrustLevel(value: TrustLevel) {
    try {
      await invoke("assistant_set_trust_level", { value });
      this.trustLevel = value;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async setAutoCompactThreshold(value: number | null) {
    const v = value !== null && Number.isFinite(value) && value > 0 && value <= 1 ? value : null;
    try {
      await invoke("assistant_set_auto_compact_threshold", { value: v });
      this.autoCompactThreshold = v;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async setCompactModel(value: "haiku" | "sonnet") {
    try {
      await invoke("assistant_set_compact_model", { value });
      this.compactModel = value;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async setBaseUrl(value: string | null) {
    const v = value && value.trim().length > 0 ? value.trim() : null;
    try {
      await invoke("assistant_set_base_url", { value: v });
      this.baseUrl = v;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async setProviderModel(value: string | null) {
    const v = value && value.trim().length > 0 ? value.trim() : null;
    try {
      await invoke("assistant_set_provider_model", { value: v });
      this.providerModel = v;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  // ── 2a multi-provider list ──
  async refreshProviders() {
    this.providers = await invoke<ProviderDto[]>("assistant_list_providers");
  }

  /** Create (empty id) or update a provider. `apiKey` omitted on edit keeps the stored key. */
  async saveProvider(
    profile: { id?: string; name: string; baseUrl: string; model: string | null },
    apiKey: string | null,
  ): Promise<string> {
    try {
      const id = await invoke<string>("assistant_save_provider", {
        profile: { id: profile.id ?? null, name: profile.name, baseUrl: profile.baseUrl, model: profile.model },
        apiKey: apiKey && apiKey.trim().length > 0 ? apiKey.trim() : null,
      });
      await this.refreshProviders();
      return id;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  async deleteProvider(id: string) {
    try {
      await invoke("assistant_delete_provider", { id });
      await this.refreshProviders();
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  /** Set the routing provider (`null` = Anthropic). */
  async setActiveProvider(id: string | null) {
    try {
      await invoke("assistant_set_active_provider", { id });
      await this.refreshProviders();
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  // ── 3c compression toggle ──
  async setCompression(enabled: boolean, proxyUrl: string | null) {
    const v = proxyUrl && proxyUrl.trim().length > 0 ? proxyUrl.trim() : null;
    try {
      await invoke("assistant_set_compression", { enabled, proxyUrl: v });
      this.compressionEnabled = enabled;
      this.compressionProxyUrl = v;
    } catch (e) {
      this.lastNotice = String(e);
      throw e;
    }
  }

  /** Probe proxy reachability + headroom runtime presence. Pure observation. */
  async compressionEnvCheck(
    proxyUrl: string | null,
  ): Promise<{ proxyUrl: string; proxyReachable: boolean; headroomPresent: boolean; pythonPresent: boolean }> {
    const v = proxyUrl && proxyUrl.trim().length > 0 ? proxyUrl.trim() : null;
    return await invoke("compression_env_check", { proxyUrl: v });
  }

  /** Turn dispatch (incl. client-side slash commands). M9: body in ./assistant/send. */
  async send(prompt: string) {
    return sendImpl(this, prompt);
  }

  /** Stage a binary attachment for the next send. Returns false if the size
   *  cap would be exceeded; the composer surfaces a notice on rejection. */
  /** Stage a binary attachment on `tabId`'s tab — defaults to the active
   *  (focused-pane) tab when omitted. 20 MiB cumulative cap mirrors the
   *  backend guard. Returns false on overflow. */
  // M4 split (2026-05-26): per-tab attachment logic in `./assistant/attachments`.
  // Store methods stay as thin tab-resolving thunks routing to active/specified tab.
  addAttachment(
    att: { mime: string; dataBase64: string; sizeBytes: number },
    tabId?: string | null,
  ): boolean {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    return tab ? attAdd(tab, att) : false;
  }

  removeAttachment(id: string, tabId?: string | null) {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    if (tab) attRemove(tab, id);
  }

  clearAttachments(tabId?: string | null) {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    if (tab) attClear(tab);
  }

  /** User-driven pin from a chat checklist into the Tasks dock.
   *  Items arrive as plain text + checked flag from rendered HTML. */
  pinTasksFromChecklist(items: Array<{ content: string; checked: boolean }>) {
    if (items.length === 0) return;
    const tab = this.activeTab;
    if (!tab) return;
    tab.tasks = items.map((t, i) => ({
      id: `pin-${Date.now()}-${i}`,
      content: t.content,
      status: t.checked ? "completed" : "pending",
    }));
    this.ui.tasksUpdatedAt = Date.now();
    this.ui.dockOpen = true;
    tab.dockAutoOpenedThisConvo = true;
  }

  private shortToolLabel(name: string, input?: Record<string, unknown>): string {
    const base = name.replace(/^mcp__rift__/, "");
    const inp = input ?? {};
    const trim = (s: string, n = 70) => s.length > n ? s.slice(0, n - 1) + "…" : s;
    // File ops — Claude Code built-ins (PascalCase) + Rift MCP variants (snake_case).
    if ((base === "Read" || base === "read_file") && typeof (inp.file_path ?? inp.path) === "string")
      return `read ${inp.file_path ?? inp.path}`;
    if (base === "Write" && typeof inp.file_path === "string") return `write ${inp.file_path}`;
    if (base === "Edit" && typeof inp.file_path === "string") return `edit ${inp.file_path}`;
    if (base === "MultiEdit" && typeof inp.file_path === "string") {
      const n = Array.isArray(inp.edits) ? (inp.edits as unknown[]).length : 0;
      return `edit ${inp.file_path} (${n})`;
    }
    if (base === "NotebookEdit" && typeof inp.notebook_path === "string") return `notebook ${inp.notebook_path}`;
    // Shell.
    if (base === "Bash" && typeof inp.command === "string") return `$ ${trim(inp.command as string)}`;
    if (base === "BashOutput") return `bash output`;
    if (base === "KillBash" || base === "KillShell") return `kill bg shell`;
    if (base === "remote_bash" && typeof inp.command === "string") return `remote $ ${trim(inp.command as string)}`;
    // Search / nav.
    if (base === "Glob" && typeof inp.pattern === "string") return `glob ${inp.pattern}`;
    if ((base === "Grep" || base === "grep") && typeof inp.pattern === "string") return `grep "${inp.pattern}"`;
    if (base === "list_dir" && typeof inp.path === "string") return `list ${inp.path}`;
    // Web.
    if (base === "WebFetch" && typeof inp.url === "string") return `fetch ${inp.url}`;
    if (base === "WebSearch" && typeof inp.query === "string") return `search "${inp.query}"`;
    // Agentic / planning / meta.
    if (base === "Agent" && typeof inp.subagent_type === "string") return `agent: ${inp.subagent_type}`;
    if (base === "Agent" && typeof inp.description === "string") return `agent: ${trim(inp.description as string, 50)}`;
    if (base === "AskUserQuestion") return `asking…`;
    if (base === "ExitPlanMode") return `exit plan mode`;
    if (base === "SlashCommand" && typeof inp.command === "string") return `slash ${inp.command}`;
    if (base === "Skill" && typeof inp.skill === "string") return `skill ${inp.skill}`;
    if (base === "TodoWrite") {
      const n = Array.isArray(inp.todos) ? (inp.todos as unknown[]).length : 0;
      return `todos · ${n}`;
    }
    if (base === "TaskCreate" && typeof inp.subject === "string") return `plan: ${trim(inp.subject as string, 40)}`;
    if (base === "TaskUpdate") return `task ${inp.taskId ?? ""} · ${inp.status ?? ""}`.trim();
    return base;
  }

  // appendToolUse / recordTurnUsage / resetUsage / fillToolResult / onStream /
  // onDone / onError all moved to TabState. Cross-cutting effects (queue drain,
  // save, dock-open) reach back through the callback hooks set in wireTab().

  /** Called by TabState.onTurnComplete via the callback wired in wireTab().
   *  Persists the just-completed turn (debounced) and drains the queue if
   *  the turn that finished was on the active tab. Bg-tab queue drain is
   *  deferred until the user returns to that tab — auto-sending into a
   *  background tab while the user is composing in the foreground would be
   *  surprising. The save itself is scoped to the completed tab (its convoId
   *  is threaded through scheduleSave) so a bg-tab turn persists immediately
   *  even while another tab is focused — no longer deferred to re-activation. */
  private handleTurnComplete(tab: TabState) {
    // Resolve this tab's convoId (its Map key) so the save targets the
    // completed tab, not whichever tab is active. cliSessionId can diverge from
    // convoId post-compaction, so reverse-look-up the key rather than trust it.
    let convoId: string | undefined;
    for (const [cid, t] of this.tabs) {
      if (t === tab) {
        convoId = cid;
        break;
      }
    }
    this.scheduleSave(false, convoId);
    this.recordSessionLog();
    this.drainQueue(tab);
  }

  /** Fire the next queued message on the tab, if any — see ./assistant/send. */
  private drainQueue(tab: TabState | null) {
    sendDrainQueue(this, tab);
  }

  /** Stop a tab's in-flight stream — see ./assistant/send. */
  async stop(tabId?: string | null) {
    return sendStop(this, tabId);
  }

  /** Steer the RUNNING turn via live CLI stdin — see ./assistant/send. */
  async steer(text: string, tabId?: string | null) {
    return sendSteer(this, text, tabId);
  }

  removeQueued(id: string, tabId?: string) {
    sendRemoveQueued(this, id, tabId);
  }

  /** Composer wand: rewrite a rough draft into a clearer prompt. Stateless —
   *  the backend streams a headless rewrite token-by-token over
   *  `assistant://enhance-stream`, then resolves to the authoritative final
   *  text. `onDelta` receives the accumulated text on each chunk. `opts` steers
   *  it: `model` (default sonnet), `directive` (refine instruction), `cwd`
   *  (workspace dir → grounded read-only pass over the real code). Throws on
   *  failure so the caller can surface it. */
  async enhancePrompt(
    text: string,
    onDelta?: (full: string) => void,
    opts?: { model?: string; directive?: string; cwd?: string },
  ): Promise<string> {
    const requestId = crypto.randomUUID();
    let acc = "";
    const unlisten = await listen<{ request_id: string; delta?: string; done?: boolean }>(
      "assistant://enhance-stream",
      (e) => {
        if (e.payload.request_id !== requestId) return;
        if (e.payload.delta) {
          acc += e.payload.delta;
          onDelta?.(acc);
        }
      },
    );
    try {
      return await invoke<string>("assistant_enhance_prompt", {
        requestId,
        prompt: text,
        model: opts?.model,
        directive: opts?.directive,
        cwd: opts?.cwd,
      });
    } finally {
      unlisten();
    }
  }

  /** Compaction Phase B: one-shot summarize of the current CLI session.
   *  Pure read — does NOT mutate `messages`, doesn't archive, doesn't
   *  remint. Phase C wires this into the actual compaction flow; until
   *  then it's reachable only via the `/summarize` debug slash.
   *
   *  Returns the SummarizeResult on success, or null if no active session
   *  or the call fails (error surfaces via `lastError`). The cost/usage
   *  fields let Phase C populate the boundary message pill. */
  summarizeCurrentSession(focus?: string): Promise<SummarizeResult | null> {
    return compactSummarize(this, focus);
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
  compactConversation(focus?: string, tabId?: string | null): Promise<boolean> {
    return compactRun(this, focus, tabId);
  }

  /** Re-send the most recent user prompt — see ./assistant/send. */
  async retryLast() {
    return sendRetryLast(this);
  }

  /** Copy the latest assistant message's text to the clipboard. */
  async copyLastAssistant() {
    return sendCopyLastAssistant(this);
  }

  /** Up-arrow recall. Returns the n-th-most-recent prompt, or null. */
  recallPrompt(offsetFromEnd: number): string | null {
    return sendRecallPrompt(this, offsetFromEnd);
  }

  dismissNotice() {
    this.lastNotice = null;
  }

  /** Hard-reset everything visible in the active tab. Used by external
   *  callers that need a "wipe this conversation" — not the same as
   *  newTab/newConversation which also touch tab lifecycle. */
  clear() {
    const tab = this.activeTab;
    if (tab) {
      tab.messages = [];
      tab.lastError = null;
      tab.totalCostUsd = null;
      tab.tasks = [];
      tab.taskCreateCount = 0;
      tab.promptHistory = [];
      tab.dockAutoOpenedThisConvo = false;
    }
    this.lastNotice = null;
    this.queue = [];
  }
}

// Type-only export for the M9 send module (and future extractions) — the
// runtime singleton below stays the only constructed instance.
export type { AssistantStore };

export const assistant = new AssistantStore();
