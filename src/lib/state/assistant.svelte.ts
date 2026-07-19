// Assistant page state.
//
// Spawns the user's installed `claude` CLI through Rust commands; the CLI
// streams NDJSON which the backend forwards verbatim on `assistant://stream`.
// Wires Rift's MCP server (read_file / list_dir / grep) so assistant turns
// can interleave text, tool calls, and TodoWrite-driven task lists.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast, notify } from "./toast.svelte";
import { workspace } from "./workspace.svelte";
import { humanizeError } from "../utils/humanizeError";
import { browserDock } from "./browserDock.svelte";

// M0 split (2026-05-26): type defs lifted to `./assistant/types`. Re-exported
// here so external callers like `import type { Block } from "$lib/state/assistant.svelte"`
// keep working. See `docs/design/assistant-svelte-split.md`.
export type {
  ToolBlock,
  ThinkingBlock,
  Block,
  ChatMessage,
  ModelSel,
  QueueItem,
} from "./assistant/types";
import type {
  WorkspaceState,
  AuthStatus,
  Block,
  ChatMessage,
  ConversationMeta,
  ThinkingEffort,
  ModelSel,
  PermissionMode,
  RiftPlan,
  TrustLevel,
  PermissionPromptInfo,
  PermissionSuggestion,
  TurnRecord,
  PaneState,
  QueueItem,
} from "./assistant/types";
import { MAX_PANES } from "./assistant/types";

// M1 split (2026-05-26): helpers lifted to `./assistant/helpers`.
import {
  loadModel,
  saveModel,
  loadEffort,
  saveEffort,
  clampEffort,
  loadThinkingEnabled,
  saveThinkingEnabled,
  loadPermissionMode,
  savePermissionMode,
  loadFastMode,
  saveFastMode,
  loadPlan,
  savePlan,
  planContextCap,
  migrateThinkingPins,
  ctxWindowForModelId,
  isStaleTurnEpoch,
} from "./assistant/helpers";

// cont.276: stream/done/error listener bodies extracted for testability —
// init() registers 1-line thunks onto these (see listeners.ts header).
import {
  handleStreamEvent,
  handleDoneEvent,
  handleErrorEvent,
  handleShellRowsEvent,
  type ListenerHost,
  type StreamPayload,
  type DonePayload,
  type ErrorPayload,
  type ShellRowsPayload,
  type ShellRow,
} from "./assistant/listeners";

// Run before any per-workspace thinking pin is read (field init below +
// applyWorkspacePrefs). Clears stale pre-v0.65.0 `thinkingEnabled::<root>=on`
// pins so every folder falls back to the off-by-default baseline. One-time,
// idempotent, SSR-safe (no-ops without localStorage). See helpers.ts.
migrateThinkingPins();

// M2 split (2026-05-26): SessionTelemetry class lifted to `./assistant/telemetry`.
import { SessionTelemetry } from "./assistant/telemetry";
// M4 split (2026-05-26): attachment free fns in `./assistant/attachments`.
import {
  addAttachment as attAdd,
  removeAttachment as attRemove,
  clearAttachments as attClear,
  addTextAttachment as txtAdd,
  removeTextAttachment as txtRemove,
  clearTextAttachments as txtClear,
  type TextAttachment,
} from "./assistant/attachments";
// M3 split (2026-05-26): workspace free fns in `./assistant/workspace`.
import {
  refreshWorkspace as wsRefresh,
  pickFolder as wsPickFolder,
  pickTabFolder as wsPickTabFolder,
  setTabRoot as wsSetTabRoot,
  setRoot as wsSetRoot,
  removeRecentRoot as wsRemoveRecentRoot,
  loadWorkspaceFiles as wsLoadFiles,
  loadWorkspaceBranch as wsLoadBranch,
  loadCustomCommands as wsLoadCustomCommands,
  type CustomCommand,
} from "./assistant/workspace";
// M5 split (2026-05-26): conversation persistence + tab-list save in
// `./assistant/persistence`. loadConversation + deleteConversation stay on
// the class (M5b — gated on M6 tabs lifecycle extraction).
import {
  refreshConversations as persistRefresh,
  flushNow as persistFlushNow,
  scheduleSave as persistSchedule,
  renameConversation as persistRename,
  persistTabs as persistTabsImpl,
  loadConversation as persistLoad,
  deleteConversation as persistDelete,
  deleteAllConversations as persistDeleteAll,
} from "./assistant/persistence";
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
// queue drain + stop + retry/copy/recall) in `./assistant/send`.
// enhancePrompt + the turn-complete hook wiring stay on the store.
import {
  send as sendImpl,
  drainQueue as sendDrainQueue,
  stop as sendStop,
  removeQueued as sendRemoveQueued,
  retryLast as sendRetryLast,
  copyLastAssistant as sendCopyLastAssistant,
  recallPrompt as sendRecallPrompt,
} from "./assistant/send";
// Post-turn health pass — bg-tab completion toasts + once-per-session
// dead-wait / stale-cache / tool-error warnings.
import { checkTurnHealth, askUserStaleNudge } from "./assistant/healthAlerts";

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
  /** Last *real* turn timestamp — bumped on send() and on a result landing,
   *  NOT on open/switch/auto-save. The sidebar sorts + buckets by this so a
   *  chat doesn't jump to the top merely because it was opened (its tab-switch
   *  auto-save still advances updatedAt). Falls back to convoCreatedAt. */
  lastActivityAt = $state<number | null>(null);
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
  /** True while the in-flight turn is a manual /compact — the CLI compacts
   *  natively with no tools/text until the boundary lands, so without this the
   *  whole turn reads as a generic "Working…" hang. Drives the dedicated
   *  "Compacting conversation…" live status in StreamTurn. Set by send(),
   *  reset every beginTurn(). */
  compactingTurn = $state(false);
  tasks = $state<{ id: string; content: string; status: "pending" | "in_progress" | "completed" }[]>([]);
  taskCreateCount = $state(0);
  activity = $state<{ currentLabel: string | null; turnStartedAt: number | null }>({
    currentLabel: null,
    turnStartedAt: null,
  });
  lastError = $state<string | null>(null);
  totalCostUsd = $state<number | null>(null);
  lastTurnUsage = $state<{ input: number; output: number; cacheRead: number; cacheCreate: number } | null>(null);
  /** CLI-reported context window from the last result frame's
   *  `modelUsage[id].contextWindow` — ground truth for what the CLI actually
   *  ran (and auto-compacts) against. Overrides the plan×model estimate in
   *  ctxWindowFor while the tab is still on that model. */
  reportedCtxWindow = $state<{ model: string; window: number } | null>(null);
  sessionUsage = $state({ totalInput: 0, totalOutput: 0, totalCacheRead: 0, totalCacheCreate: 0, turns: 0 });
  /** Output tokens generated so far in the in-flight turn. Drives the live
   *  "1.2k tokens" readout in the spinner + composer pill. The CLI only reports
   *  real output_tokens at each message's end (no mid-stream usage), so this
   *  climbs as a char/4 estimate over the in-flight message (`liveOutputChars`)
   *  layered on the exact totals already banked from completed messages
   *  (`committedOutputTokens`), then snaps exact when each message lands. Reset
   *  at turn start. */
  liveOutputTokens = $state(0);
  /** Exact output_tokens banked from assistant messages already completed this
   *  turn (multi-message agentic loops). The in-flight estimate rides on top. */
  committedOutputTokens = 0;
  /** Chars streamed for the in-flight (not-yet-completed) message; ×0.25 ≈ its
   *  output tokens. Reset to 0 each time a message completes. */
  liveOutputChars = 0;
  lastModelId = $state<string | null>(null);
  /** Per-chat model override (ui-audit #5). Set when an old convo is opened
   *  (its saved model scopes to this tab) and on explicit pick; null = follow
   *  the global default. Opening a chat no longer rewrites the new-chat default. */
  modelOverride = $state<ModelSel | null>(null);
  /** The model this session is actually PINNED to backend-side — captured on the
   *  first send (and hydrated from disk on resume). The backend ignores a picker
   *  switch on a resumed session (thinking-block signatures are model-bound,
   *  turn.rs load_session_model), so this is the model the running turns truly
   *  use. `modelOverride` can drift ahead of it when the user switches the picker
   *  mid-chat; the picker surfaces the divergence + offers "New chat in <model>".
   *  null = first turn hasn't run yet (no pin), so a switch still takes effect. */
  pinnedModel = $state<ModelSel | null>(null);
  /** open_browser landing on a backgrounded tab parks its URL here instead of
   *  hijacking the focused pane's dock; AssistantPage consumes it when this tab
   *  regains focus. In-memory only — a stale preview URL isn't worth persisting. */
  pendingBrowserUrl = $state<string | null>(null);
  /** #30: cwd the CLI session is pinned to (resumed convos keep their original
   *  folder). Hydrated on disk-load; null = no pin known / fresh tab. The tabs
   *  bar badges the active tab when this differs from workspace.current. */
  sessionCwd = $state<string | null>(null);
  /** Per-tab project folder. The folder THIS tab's turns run in — set via the
   *  per-pane folder picker, inherited on new/clear, hydrated from `sessionCwd`
   *  on disk-load. null = follow the global workspace default. Distinct from
   *  `sessionCwd` (the backend's pinned cwd readout): `workspaceRoot` is the
   *  user-intended root the renderer passes to `assistant_send`, so two panes
   *  / windows can work in different directories. */
  workspaceRoot = $state<string | null>(null);
  promptHistory = $state<string[]>([]);
  /** Outbound message queue for THIS tab. send() pushes here when the tab is
   *  already streaming; onDone() pops the next one (queue order = send order,
   *  drag-to-reorder in the rail). Per-tab so a queued msg in Tab A can't drain
   *  into Tab B if the user switches mid-turn. `images`/`textFiles` snapshot the
   *  composer attachments at enqueue time so a queued message carries its image
   *  when it drains (the composer arrays are cleared right after enqueue). Both
   *  optional — text-only queued messages omit them; images are NOT persisted
   *  to disk (stripped on save) to keep localStorage lean. */
  queue = $state<QueueItem[]>([]);
  /** Per-tab composer draft. Was store-level before split-pane v2 — moved
   *  here so each pane can compose into its own tab concurrently w/o the
   *  focus-change stash/restore dance dropping characters under fast typing.
   *  Composer binds via `bind:value={tab.draft}`. */
  draft = $state<string>("");
  /** Predicted next user prompt from the CLI (`--prompt-suggestions`,
   *  2.1.201+). One per turn, landing right after the result envelope. Shown
   *  as a ghost chip in the composer while the draft is empty; consumed on
   *  click, cleared at the next beginTurn. In-memory only — never persisted. */
  promptSuggestion = $state<string | null>(null);
  /** MCP server statuses from the latest `system`/`init` frame — the CLI
   *  reports them at the start of every turn. Read by the /mcp slash command.
   *  In-memory only; null until the tab's first turn. */
  mcpServers = $state<{ name: string; status: string }[] | null>(null);
  /** Per-tab staged attachments. Same rationale as `draft`. send() snapshots
   *  + clears on dispatch. 20MiB cumulative cap enforced by addAttachment. */
  attachments = $state<{ id: string; mime: string; dataBase64: string; sizeBytes: number }[]>([]);
  /** Per-tab staged text-file attachments — inlined into the prompt at send,
   *  not sent as binary blocks. 1 MiB cumulative cap enforced by addTextAttachment. */
  textAttachments = $state<TextAttachment[]>([]);
  /** S124: in-flight sub-agent spawns. Pushed on Task/Agent tool_use, marked
   *  done on the matching tool_result. The CLI DOES multiplex sub-agent output
   *  into the same stream — nested frames carry `parent_tool_use_id` = this
   *  spawn's id (verified 2026-06-14). `blocks` accumulates that sub-agent's
   *  own transcript (text / thinking / tool steps) at envelope granularity —
   *  no token-level deltas for sub-agents — and feeds the live sub-agent dock.
   *  See applySubAgentFrame in streaming.ts. */
  agentSpawns = $state<{
    id: string;
    subagentType: string;
    description: string;
    startedAt: number;
    completedAt: number | null;
    isError: boolean;
    blocks: Block[];
    // "agent" = Task/Agent delegation; "skill" = a forking slash-command
    // (/plan etc.) lazily promoted when its first nested frame arrives. Omitted
    // on legacy/Task entries → treated as "agent" by the dock.
    kind?: "agent" | "skill";
  }[]>([]);

  /** Live shell processes under this session's CLI child (ActivityHud rows).
   *  Pushed by the backend per-turn poller over `assistant://shell-rows`;
   *  cleared on every turn terminal (the poller dies with the turn). */
  shellRows = $state<ShellRow[]>([]);

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
  /** Post-stop guard deadline. While Date.now() < this, the killed turn's
   *  terminal done/error event may still be in flight from the backend (events
   *  carry only session_id — no per-turn token), so send() defers the next
   *  turn on this tab until the handlers consume the stale event (clearing
   *  this) or the deadline passes. Prevents a stale terminal from finalizing
   *  the NEXT turn on the same session. */
  staleTerminalUntil = 0;
  /** #80: monotonic per-tab turn counter. send() bumps it before dispatch; the
   *  backend stamps it on every stream/done/error event of that turn, so the
   *  listeners can discard a stale event from a stopped/superseded turn even
   *  past the staleTerminalUntil deadline. 0 = no turn sent yet this app-life
   *  (deliberately NOT persisted — epochs only disambiguate live processes). */
  turnEpoch = 0;
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
  /** Live tool-input accumulation (S127, mirrors thinkingByIndex): stream index
   *  → forming tool block. `json` accumulates input_json_delta.partial_json;
   *  `extracted` memoizes the last caption-field snapshot so no-op deltas skip
   *  the block mutate. Cleared in beginTurn + finalizeInflightBlocks. */
  toolInputByIndex = new Map<number, { id: string; name: string; json: string; extracted: string }>();
  /** Wall-clock of the most recent `stream_event` arrival. Null between turns.
   *  Used to compute `maxStreamGapMs` on the in-flight TurnRecord. */
  lastStreamEventAt: number | null = null;
  /** Wall-clock of the last clean turn DONE. A CLI-initiated continuation that
   *  begins within a breath of this reopens the previous assistant bubble
   *  instead of scaffolding a second one (split "Worked for Ns" fix). */
  lastTurnDoneAt: number | null = null;
  dockAutoOpenedThisConvo = false;
  /** Id of the single inline plan block appended this turn (TaskCreate/TodoWrite).
   *  The newer CLI emits one TaskCreate per item, so instead of one block per
   *  call we append ONE plan block on the first task event of a turn and let it
   *  render from the live `tasks` aggregate. Reset to null in beginTurn. */
  planBlockId: string | null = null;
  /** Telemetry record for the in-flight turn. Set by AssistantStore.send()
   *  before invoking the backend, filled by stream handlers, finalized in
   *  onDone / onError. Null between turns. */
  currentTurnRecord: TurnRecord | null = null;

  /** Fired after a TodoWrite tool_use lands. Store uses it to bump
   *  `ui.tasksUpdatedAt` and auto-open the dock the first time per convo. */
  onTodoApplied?: (tab: TabState, opensDock: boolean) => void;
  /** Fired on onDone — store handles scheduleSave + queue drain. */
  onTurnComplete?: (tab: TabState) => void;
  /** Fired when an ask_user card has sat unanswered past the nudge window —
   *  the turn (and CLI subprocess) is blocked on it. Store routes to a toast. */
  onAskUserStale?: (tab: TabState) => void;
  /** Translates a tool name + input into a short activity-bar label.
   *  Lives on the store (knows nothing tab-specific); passed in via this hook
   *  so TabState doesn't grow its own copy. */
  shortToolLabel?: (name: string, input?: Record<string, unknown>) => string;
  /** The user's plan context-window cap, read by the stream pump so the CLI
   *  compaction pill's pre/post % match the gauge. IoC hook (the plan lives on
   *  the store, not the tab); defaults to 1M when unwired so the pill never
   *  over-reports if the hook is somehow absent. */
  planCap?: () => number;

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

class AssistantStore {
  auth = $state<AuthStatus | null>(null);
  authChecking = $state(false);
  authError = $state<string | null>(null);
  /** Auth is usable for a turn — green (signed in) or yellow (API key / degraded
   *  but functional). The single source of truth for the "can we send?" gate,
   *  which was copy-pasted as `pill === "green" || pill === "yellow"` across the
   *  composer, send orchestrator, and pre-warm. */
  get authReady(): boolean {
    return this.auth?.pill === "green" || this.auth?.pill === "yellow";
  }
  /** True while an in-app `claude auth login` is running + being polled. Drives
   *  the recovery banner's "Signing in…" state. */
  loginInProgress = $state(false);
  /** Epoch ms of the last completed auth probe (success or failure). Drives the
   *  "last checked Xm ago" freshness label in Settings → Assistant. */
  authLastProbed = $state<number | null>(null);

  /** CLI session ids already warned about an unkeepable background task, so the
   *  bg-task notice fires once per session, not once per turn (#62 fix B). */
  private bgTaskWarnedSessions = new Set<string>();

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

  /** Every tab with an in-flight turn, keyed by convoId — the Harness
   *  mission-control view reads this instead of reaching into `tabs`. */
  get liveTabs(): { convoId: string; tab: TabState }[] {
    const out: { convoId: string; tab: TabState }[] = [];
    for (const [convoId, tab] of this.tabs) {
      if (tab.streaming) out.push({ convoId, tab });
    }
    return out;
  }

  /** Queued-message counts by convoId — sidebar rows badge parked messages
   *  (a backgrounded tab defers its drain until re-activated, so without a
   *  badge a queued send is invisible until the user happens to return). */
  get queuedCounts(): Map<string, number> {
    const out = new Map<string, number>();
    for (const [convoId, tab] of this.tabs) {
      if (tab.queue.length > 0) out.set(convoId, tab.queue.length);
    }
    return out;
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

  /** Per-tab ctx helpers. */
  ctxWindowFor(tab: TabState | null): number {
    // Prefer the model the turn ACTUALLY ran on (lastModelId), but fall back to
    // the model the tab is about to use (its override, else the global default)
    // on a fresh, turn-less tab — otherwise lastModelId is null and the gauge
    // shows the 200K null-fallback even for a Max user on a 1M model, which reads
    // as "out of line" before the first send.
    const model = tab?.lastModelId ?? tab?.modelOverride ?? this.model;
    // CLI ground truth first: the result frame reports the window the CLI
    // actually ran against (folds in the [1m] selector AND account-side gating
    // the user-set plan can't see). Valid only while the tab is still on that
    // model — a mid-chat switch falls back to the estimate until the next
    // result re-reports.
    const rep = tab?.reportedCtxWindow;
    if (rep && model && model.replace(/\[1m\]$/i, "") === rep.model) return rep.window;
    return ctxWindowForModelId(model, this.planCap);
  }
  ctxTokensFor(tab: TabState | null): number {
    const u = tab?.lastTurnUsage ?? null;
    return u ? u.input + u.cacheRead + u.cacheCreate : 0;
  }
  ctxPctFor(tab: TabState | null): number {
    const w = this.ctxWindowFor(tab);
    return w > 0 ? Math.min(100, (this.ctxTokensFor(tab) / w) * 100) : 0;
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
  get liveOutputTokens(): number { return this.activeTab?.liveOutputTokens ?? 0; }
  get lastModelId(): string | null { return this.activeTab?.lastModelId ?? null; }
  get promptHistory(): string[] { return this.activeTab?.promptHistory ?? []; }
  get queue() { return this.activeTab?.queue ?? []; }
  set queue(v: QueueItem[]) {
    if (this.activeTab) this.activeTab.queue = v;
  }

  /** Public read accessor for a tab's state — used by AssistantPane /
   *  StatusHub in split mode so each pane scopes its rendering to its own
   *  tab rather than the activeTab. Returns null for unknown ids. */
  tabFor(id: string | null): TabState | null {
    return id ? this.tabs.get(id) ?? null : null;
  }

  /** The folder a tab's turns run in: its own per-tab root, else the global
   *  workspace default. Used for the per-pane picker display, the @-mention
   *  walk, and the root passed to `assistant_send`. */
  effectiveRoot(tab: TabState | null): string | null {
    return tab?.workspaceRoot ?? this.workspace.current ?? null;
  }

  /** Effective root of the focused tab — drives the global @-mention walk +
   *  branch probe (both modal to the focused composer). */
  get activeRoot(): string | null {
    return this.effectiveRoot(this.activeTab);
  }

  /** The global workspace default root (independent of which pane is focused).
   *  Persistence uses this — not `activeRoot` — as the fallback scope when
   *  saving a tab with no per-tab root, so a background save never inherits the
   *  focused pane's project. */
  get workspaceCurrent(): string | null {
    return this.workspace.current ?? null;
  }

  /** No project folder open, but the backend scratch workspace is available →
   *  turns silently run in `Documents\Rift Workspace` (legacy
   *  `%LOCALAPPDATA%\Rift\local`) with the full tool set. Drives the "Local"
   *  badge + welcome card. `effectiveRoot` stays null in this mode (the backend
   *  fills the scratch dir per turn). */
  get isLocalMode(): boolean {
    return !this.workspace.current && !!this.localScratchPath;
  }

  get splitActive(): boolean {
    return this.panes.length > 1;
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

  /** Open a project (root folder) as a fresh chat in a specific pane, scoping
   *  that pane to the project WITHOUT touching the global workspace root — the
   *  mechanism behind "open project into a split pane". `paneIdx` beyond the
   *  current panes (or `splitNew`) first grows the split (cap-aware) so two
   *  projects can sit side-by-side from one gesture. Returns false if the split
   *  couldn't grow (width/cap), in which case the project opens in the focused
   *  pane instead. */
  async openProjectInPane(root: string, opts?: { paneIdx?: number; splitNew?: boolean }): Promise<boolean> {
    let targetIdx = opts?.paneIdx ?? this.focusedPaneIdx;
    if (opts?.splitNew || targetIdx >= this.panes.length) {
      // Want a NEW pane to the side — try to grow the split first.
      const before = this.panes.length;
      this.addPane();
      const grew = this.panes.length > before;
      // addPane focuses + (maybe) fills the new pane; we override its contents
      // below with the project's own fresh tab. If it couldn't grow, fall back
      // to the focused pane so the gesture still does something useful.
      targetIdx = grew ? this.panes.length - 1 : this.focusedPaneIdx;
      if (!grew) {
        this.setFocusedPane(targetIdx);
        await this.newTab();
        await this.setTabRoot(this.currentConvoId, root);
        return false;
      }
    }
    this.setFocusedPane(targetIdx);
    await this.newTab();
    await this.setTabRoot(this.currentConvoId, root);
    return true;
  }

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
    // Cancel the debounced save too — else the 700ms timer fires against a tab
    // no longer in the map (ghost save), which can resurrect a just-deleted convo.
    if (tab.saveTimer) {
      clearTimeout(tab.saveTimer);
      tab.saveTimer = null;
    }
    const next = new Map(this.tabs);
    next.delete(convoId);
    this.tabs = next;
  }

  /** Attach cross-cutting hooks to a freshly-minted TabState. */
  private wireTab(tab: TabState) {
    tab.shortToolLabel = (name, input) => this.shortToolLabel(name, input);
    tab.onTodoApplied = (_t, _opensDock) => {
      this.ui.tasksUpdatedAt = Date.now();
    };
    tab.onTurnComplete = (t) => this.handleTurnComplete(t);
    tab.onAskUserStale = (t) => askUserStaleNudge(this, t);
    tab.planCap = () => this.planCap;
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
  // The Assistant's open project folder + recent-folder list. Populated by
  // `assistant_get_workspace` on init and updated whenever the user opens,
  // switches, or clears a folder.
  workspace = $state<WorkspaceState>({ current: null, recent: [] });
  /** True once `refreshWorkspace()` has resolved the persisted folder on boot.
   *  Gates the cold "no folder" welcome so it never flashes for a frame before
   *  the rehydrated root lands (same pattern as `configLoaded`). */
  workspaceReady = $state<boolean>(false);
  /** Backend-resolved persistent scratch workspace (`%LOCALAPPDATA%\Rift\local`).
   *  Populated once by `refreshWorkspace()`; drives the "Local" badge only — the
   *  backend re-resolves the real path per turn (never renderer-supplied). */
  localScratchPath = $state<string | null>(null);

  // Cached relative file paths under the workspace root, populated on first
  // `@` trigger and re-loaded whenever the workspace root changes. Drives the
  // composer's `@`-file mention picker. Walk is cheap (~ms for typical FiveM
  // resource folder) so we re-fetch on each open rather than invalidate via
  // a watcher.
  workspaceFiles = $state<string[]>([]);
  workspaceFilesLoadingFor = $state<string | null>(null);
  workspaceBranch = $state<string | null>(null);
  // Custom slash commands (user + project skills/commands from `.claude` dirs).
  // Refreshed on every slash-menu open — scan is a few dir reads, so freshness
  // beats caching. Drives the composer's `/` menu custom sections.
  customCommands = $state<CustomCommand[]>([]);
  customCommandsLoadingFor = $state<string | null>(null);

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
  get composerTextAttachments(): TextAttachment[] {
    return this.activeTab?.textAttachments ?? [];
  }
  set composerTextAttachments(v: TextAttachment[]) {
    if (this.activeTab) this.activeTab.textAttachments = v;
  }
  // queue moved to TabState (S105 follow-up) — per-tab so a queued msg in
  // Tab A can't drain into Tab B if the user switches mid-turn. UI binds via
  // the `queue` getter below which delegates to activeTab.
  // User's chosen model — flipped by /model slash command. Carried through
  // to assistant_send so the CLI uses sonnet/opus/haiku per their choice.
  // Initialized from localStorage so the choice survives reloads.
  model = $state<ModelSel>(loadModel());
  // Extended-thinking effort tier (CLI `--effort` ladder): "none"→low ·
  // "smart"→medium (default) · "deep"→high · "ultra"→xhigh + ultracode.
  // Haiku ignores this server-side. Persisted to localStorage.
  thinkingEffort = $state<ThinkingEffort>(clampEffort(loadEffort(), this.model));
  // Extended-thinking master switch. On (default) = current behavior; off routes
  // the cloud turn through the no-think shim for fastest TTFT. Persisted, per-ws.
  thinkingEnabled = $state<boolean>(loadThinkingEnabled());
  // Permission mode passed to the CLI's `--permission-mode`. Global (matches
  // model/effort). `bypassPermissions` until the user picks otherwise so
  // existing behavior is unchanged. Persisted to localStorage.
  permissionMode = $state<PermissionMode>(loadPermissionMode());
  // Fast mode (Opus fast output) — rides `--settings {"fastMode":true}` on
  // fast-eligible models. Global, persisted, default off. The backend re-gates
  // by model family + CLI version, so this can be sent unconditionally.
  fastMode = $state<boolean>(loadFastMode());
  // Subscription plan (USER-SET — no programmatic plan signal exists for OAuth
  // users). Drives the context-window cap applied to every model's native window
  // (see ctxWindowFor). Global, persisted; default `max` (1M). Free/uncredited-Pro
  // users set it once in Settings to cap the gauge honestly at 200K.
  plan = $state<RiftPlan>(loadPlan());
  // usageOpen: "ctx" = compact conversation-context popover (composer ring),
  // "full" = plan-limits panel (/usage command; status bar owns its own copy).
  ui = $state({ tasksUpdatedAt: 0, usageOpen: false as false | "ctx" | "full" });

  // Conversation history.
  //   - `currentConvoId` is null before the first message is sent; first
  //     `send()` assigns a fresh UUID and persists from there.
  //   - `conversations` is the metadata cache for the drawer; refreshed
  //     after every save/delete/rename.
  //   - `createdAt` is set when the convo starts, kept stable across saves.
  //   - `openTabs` (v0.4) is the ordered list of convo ids visible as tabs in
  //     the top tab bar. Each tab owns its own stream (routed by session_id) —
  //     switching tabs leaves background turns running.
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
  /** Set on PROJECT-chip dragstart (sidebar rail / Workspace card) so panes
   *  light up the same drop affordance as a tab drag. Holds the project's root
   *  folder; the pane drop opens it as a fresh chat scoped to that root.
   *  Mutually exclusive with draggingTabId in practice. */
  draggingProjectRoot = $state<string | null>(null);

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
  // init() is called from AppShell, AssistantPage, and SettingsPage mounts —
  // same-flush calls must share one run. The unlistens guard alone races (no
  // push happens until after the first await), double-registering every
  // listener → every stream line applied twice.
  private initPromise: Promise<void> | null = null;
  // B1: destroy() bumps this so an initInner() still awaiting its listen() calls
  // can detect it was torn down mid-flight and bail before re-pushing a second
  // set of listeners (the unlistens-length guard alone races across destroy()).
  private initGen = 0;
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

  /** Model the ACTIVE chat talks to: its per-chat override (seeded when an
   *  old convo is opened) else the global default. Per-chat surfaces
   *  (composer, send, tabsbar, live harness) read this; Home quick-ask and
   *  onboarding read `model` — the new-chat default. (ui-audit #5) */
  get effectiveModel(): ModelSel { return this.activeTab?.modelOverride ?? this.model; }

  /** The model the active chat's turns have been RUNNING on (seeded on the
   *  first turn, advanced by send() when a mid-chat switch takes effect), or
   *  null if no turn has run yet. Drives the picker's "this chat" tag + the
   *  switch-pending note. */
  get sessionPinnedModel(): ModelSel | null { return this.activeTab?.pinnedModel ?? null; }

  /** True when the picker selection differs from the chat's running model —
   *  i.e. the user switched mid-chat and the switch takes effect on the next
   *  message (send() inserts the transcript marker; turn.rs re-pins). Drives
   *  the picker's "switches on your next message" note. */
  get sessionModelDiverged(): boolean {
    const pinned = this.sessionPinnedModel;
    return pinned !== null && pinned !== this.effectiveModel;
  }

  setModel(v: ModelSel) {
    const prev = this.effectiveModel;
    // Re-picking the already-effective model is a no-op — falling through here
    // when a tab override diverges from the global default would silently
    // rewrite the global default on a same-model reselect.
    if (prev === v) return;
    // Split-pane: a pick inside a pane with its OWN folder scopes to that chat
    // + that folder's pin. Only a pick in a pane following the global root
    // moves the shared new-chat default (cont.339 model-leak fix).
    if (!this.activeTab?.workspaceRoot) this.model = v;
    if (this.activeTab) this.activeTab.modelOverride = v;
    saveModel(v, this.activeRoot);
    // Coerce effort down to the new model's ceiling so the slider and the tier
    // we actually send can't exceed what the model honors (e.g. Opus@ultra →
    // Sonnet caps at smart). No-op when already in range. setThinkingEffort
    // handles the persist + cache-bust + telemetry and early-returns on no change.
    this.setThinkingEffort(clampEffort(this.thinkingEffort, v));
    const midConvo = (this.activeTab?.messages.length ?? 0) > 0;
    this.telemetry.event("model.change", { from: prev, to: v, midConvo });
    if (midConvo && prev !== v) this.cacheBustHint("model");
  }

  setThinkingEffort(v: ThinkingEffort) {
    if (this.thinkingEffort === v) return;
    const prev = this.thinkingEffort;
    this.thinkingEffort = v;
    saveEffort(v, this.activeRoot);
    const midConvo = (this.activeTab?.messages.length ?? 0) > 0;
    this.telemetry.event("effort.change", { from: prev, to: v, midConvo });
    if (midConvo) this.cacheBustHint("effort");
  }

  /** Set the unified thinking dial. Writes BOTH backing fields atomically:
   *  `enabled` (the master switch) + the chosen effort tier (only meaningful
   *  when on). Off keeps the last effort tier stored so flipping back on
   *  restores it — but a dial rung always carries an explicit tier, so the
   *  on-rungs never leave a stale tier behind. One write per change keeps the
   *  warm-pool cache-bust to a single hint instead of two. */
  setThinkingDial(enabled: boolean, effort?: ThinkingEffort) {
    const nextEffort = effort ?? this.thinkingEffort;
    const changed = this.thinkingEnabled !== enabled || this.thinkingEffort !== nextEffort;
    if (!changed) return;
    const prevEnabled = this.thinkingEnabled;
    const prevEffort = this.thinkingEffort;
    this.thinkingEnabled = enabled;
    saveThinkingEnabled(enabled, this.activeRoot);
    if (nextEffort !== prevEffort) {
      this.thinkingEffort = nextEffort;
      saveEffort(nextEffort, this.activeRoot);
    }
    const midConvo = (this.activeTab?.messages.length ?? 0) > 0;
    this.telemetry.event("thinking.dial", {
      enabled, effort: nextEffort, fromEnabled: prevEnabled, fromEffort: prevEffort, midConvo,
    });
    if (midConvo) this.cacheBustHint("thinking");
  }

  setPermissionMode(v: PermissionMode) {
    if (this.permissionMode === v) return;
    const prev = this.permissionMode;
    this.permissionMode = v;
    savePermissionMode(v);
    this.telemetry.event("permission_mode.change", { from: prev, to: v });
  }

  /** Latched once per session — the enable-side billing disclosure below. */
  private fastModeCostWarned = false;
  setFastMode(v: boolean) {
    if (this.fastMode === v) return;
    this.fastMode = v;
    saveFastMode(v);
    this.telemetry.event("fast_mode.change", { to: v });
    // BILLING DISCLOSURE (owner incident 2026-07-14): fast mode is PAY-PER-USE
    // — the CLI bills it from usage credits, NOT plan limits (its own TUI shows
    // "Fast mode ON · Draws from usage credits"). Rift must say so at the
    // moment of consent, every enable path, not bury it in a tooltip.
    if (v && !this.fastModeCostWarned) {
      this.fastModeCostWarned = true;
      toast.push({
        severity: "warn",
        title: "Fast mode is pay-per-use",
        detail:
          "Fast Opus turns draw from your usage credits (extra usage) — real billing beyond your plan limits. " +
          "The ⚡ chip marks each turn that actually ran fast. Turn the toggle off to stop.",
      });
    }
    // Mid-conversation flip changes the SpawnKey (fastMode is baked into
    // --settings at spawn) → same cache-bust hint as effort so the prewarm
    // can hide the respawn behind typing time.
    if ((this.activeTab?.messages.length ?? 0) > 0) this.cacheBustHint("fast");
  }

  setPlan(v: RiftPlan) {
    if (this.plan === v) return;
    const prev = this.plan;
    this.plan = v;
    savePlan(v);
    this.telemetry.event("plan.change", { from: prev, to: v });
  }

  /** Claude-session prefs → factory defaults. Credentials stay: the API key and
   *  spending cap are not preferences and are never touched by a reset. */
  async resetSessionDefaults() {
    this.setPlan("max");
    await this.setTrustLevel("readonly");
    await this.setUseFullConfig(true);
  }

  /** Context-window ceiling the user's current plan grants. Derived so the gauge
   *  recomputes when `plan` flips. */
  get planCap(): number {
    return planContextCap(this.plan);
  }

  /** One-shot-per-session-per-kind notice when model/effort flips on a tab
   *  that already has turns. Sonnet's cache empirically does NOT survive
   *  effort changes (S106 measurement: 0 cacheRead on 3 consecutive sonnet
   *  turns w/ effort flips vs healthy reuse without). Opus is more forgiving.
   *  Notice is fire-once so it's a hint, not a nag. */
  private cacheBustHintShown = { model: false, effort: false, thinking: false, fast: false };
  private cacheBustHint(kind: "model" | "effort" | "thinking" | "fast") {
    if (this.cacheBustHintShown[kind]) return;
    this.cacheBustHintShown[kind] = true;
    // Ephemeral heads-up → toast stack (top-right), not the composer notice
    // banner. It's a transient FYI, not a blocking notice, so it auto-dismisses
    // and stays out of the chat column.
    toast.push({
      severity: "info",
      // icon omitted — ToastHost supplies the info-severity default (CR5: keeps
      // lucide-svelte UI imports out of this state module).
      title: kind === "effort"
        ? "Effort changed mid-conversation"
        : kind === "thinking"
        ? "Thinking toggled mid-conversation"
        : kind === "fast"
        ? "Fast mode toggled mid-conversation"
        : "Model switched mid-conversation",
      detail: kind === "model"
        ? "Rebuilds the prefix cache — next turn will pay full cache_create."
        : "May bust the prompt cache (esp. Sonnet) — next turn could pay full cache_create.",
    });
  }

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

  init(): Promise<void> {
    this.initPromise ??= this.initInner();
    return this.initPromise;
  }

  private async initInner() {
    if (this.unlistens.length > 0) return;
    const gen = this.initGen;
    // Backend tags every stream/done/error event w/ the originating CLI
    // session_id (S104). We route by session_id to the right TabState so
    // background tabs can keep painting concurrently with the foreground.
    // Bodies live in ./assistant/listeners (cont.276 — epoch gates + bg_task
    // dedup + payload unions are vitest'd there); these stay 1-line thunks.
    const store = this;
    const listenerHost: ListenerHost = {
      get activeTab() { return store.activeTab; },
      tabBySession: (sid) => store.tabByCliSession(sid),
      bgTaskWarnedSessions: this.bgTaskWarnedSessions,
    };
    this.unlistens.push(
      await listen<StreamPayload>(
        "assistant://stream",
        (e) => handleStreamEvent(listenerHost, e.payload),
      ),
      await listen<DonePayload>(
        "assistant://done",
        (e) => handleDoneEvent(listenerHost, e.payload),
      ),
      await listen<ErrorPayload>(
        "assistant://error",
        (e) => handleErrorEvent(listenerHost, e.payload),
      ),
      await listen<ShellRowsPayload>(
        "assistant://shell-rows",
        (e) => handleShellRowsEvent(listenerHost, e.payload),
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
    this.unlistens.push(
      await listen<{ session_id: string; prompt: string; turn_epoch?: number }>(
        "assistant://session-lost",
        (e) => this.onSessionLost(e.payload),
      ),
      await listen<{ request_id: string; session_id: string; questions: unknown }>(
        "assistant://ask-user",
        (e) => this.onAskUser(e.payload),
      ),
      // mcp__rift__open_browser: the bridge validated the scheme; show the
      // page in the dock (opens it if closed — WebBrowserPage consumes
      // browserDock.pendingUrl on mount). The dock is a focused-pane-modal
      // singleton (browserDock.svelte.ts) — a BACKGROUND pane's turn must NOT
      // hijack it out from under the pane the user is looking at. Route by
      // session_id (like every other session-bearing event) and only open when
      // the owning tab is the focused one; a bg-pane open_browser is dropped
      // rather than stealing the dock.
      await listen<{ url: string; session_id: string }>(
        "assistant://open-browser",
        (e) => {
          const tab = this.tabByCliSession(e.payload.session_id);
          if (!tab) return; // unknown session — drop
          if (tab === this.activeTab) {
            // #85: the dock only mounts on the chat workspace (WebBrowserPage
            // consumes pendingUrl on mount) — an open_browser landing while the
            // user sits on Settings/AI Health would otherwise queue invisibly
            // until they wander back. Route to chat first, then open.
            workspace.setActive("chat");
            browserDock.openUrl(e.payload.url);
          } else {
            // A background pane must not hijack the dock out from under the
            // focused pane — park the URL on ITS tab; opened when that tab
            // regains focus (was: silently dropped, so the model reported
            // "opened" for a page nobody ever saw).
            tab.pendingBrowserUrl = e.payload.url;
          }
        },
      ),
      // mcp__rift__notify: severity is allowlisted bridge-side, lengths capped.
      // Route by session_id so a background pane's turn doesn't fire a toast
      // attributed to the focused pane (every other session-bearing event —
      // stream/done/error/ask-user/permission — routes through tabByCliSession;
      // this one was the lone exception). Unknown session = drop, don't pop a
      // toast that belongs to no visible tab.
      await listen<{ title: string; detail: string | null; severity: "info" | "ok" | "warn" | "danger"; session_id: string }>(
        "assistant://notify",
        (e) => {
          if (!this.tabByCliSession(e.payload.session_id)) return;
          toast.push({
            severity: e.payload.severity ?? "info",
            title: e.payload.title,
            detail: e.payload.detail ?? undefined,
          });
        },
      ),
      // #37 multi-window: another window mutated the shared conversation store
      // (broadcast_convos_changed skips the origin, so this only fires for
      // changes made elsewhere) → re-pull our list so the sidebar stays in sync.
      await listen("convos-changed", () => void this.refreshConversations()),
      // Upstream trouble (nothink shim → assistant://provider-upstream): the
      // shim also fronts cloud turns with thinking OFF, so this still fires
      // for plain Claude/Anthropic turns. The CLI silently retries 429/5xx
      // for minutes, so without this a dead-air stall reads as "Rift hung".
      // Backend throttles re-emits (10s / status change).
      await listen<{ status: number }>(
        "assistant://provider-upstream",
        (e) => {
          // Only meaningful while some tab is actually streaming.
          if (![...this.tabs.values()].some((t) => t.streaming)) return;
          const st = e.payload.status;
          const authFail = st === 401 || st === 403;
          toast.push({
            severity: "warn",
            title: st === 429
              ? "Anthropic is rate-limiting (429)"
              : authFail
                ? `Anthropic rejected the API key (${st})`
                : `Anthropic endpoint error (${st})`,
            detail: authFail
              ? "Check the key in Settings — turns keep failing until it's fixed."
              : "Rift retries a few times, then ends the turn with a visible error instead of hanging.",
          });
        },
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
    // B1: a destroy() that landed while the listen() calls above were awaiting
    // already emptied unlistens[] — our just-registered handlers are now
    // orphaned (a re-init would stack a second set). Tear them down and bail.
    if (gen !== this.initGen) {
      for (const u of this.unlistens) { try { u(); } catch { /* already gone */ } }
      this.unlistens = [];
      return;
    }

    await this.refreshConversations();
    await this.refreshWorkspace();
    this.workspaceReady = true;
    await this.restoreTabs();

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
    this.initPromise = null;
    this.initGen++;
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
  private onSessionLost(payload: { session_id: string; prompt?: string; turn_epoch?: number }) {
    // Find the tab whose CLI session failed (may not be the active tab if the
    // user switched mid-recovery). After S103 decoupling cliSessionId may
    // differ from convoId (post-compaction).
    const tab = this.tabByCliSession(payload.session_id);
    if (!tab) return;
    // #80: a session-lost for a stopped/superseded turn must not pop the LIVE
    // turn's tail messages or double-send its prompt — consume the stop gate
    // (this EOF is that turn's terminal) and drop it, mirroring done/error.
    if (isStaleTurnEpoch(tab.turnEpoch, payload.turn_epoch)) {
      tab.staleTerminalUntil = 0;
      return;
    }
    // The backend SESSION_LOST_EVENT carries only { session_id }; recover the
    // prompt to retry from the last user message (still present pre-pop) so the
    // auto-retry re-sends the real turn instead of `undefined`.
    const lastUser = [...tab.messages].reverse().find((m) => m.role === "user");
    const retryPrompt =
      payload.prompt ??
      (lastUser?.blocks.map((b) => (b.type === "text" ? b.text : "")).join("").trim() ?? "");
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
    // RR7: cancel the rAF text-drain pacer + zero pendingText. Without this, any
    // text buffered when the session was lost keeps the drainTick loop re-arming
    // each frame (appendText early-returns on null streamingMsgId), burning
    // frames until it self-drains — matches the onStreamError terminal path.
    tab.flushPendingText();
    notify.warn("Session was lost — retrying as a fresh start");
    // Auto-retry only when the lost tab is active. Bg-tab retry would require
    // routing send() to a specific tab; for now the user re-clicks send.
    if (this.activeTab === tab && retryPrompt) {
      this.convoCreatedAt = null;
      this.convoTitle = null;
      void this.send(retryPrompt);
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

  /** Find the tab whose `askUserBindings` holds this tool_use id. tool_use ids
   *  are globally unique, so a scan across all tabs is unambiguous. Necessary
   *  because the binding is stored on the tab that OWNS the CLI session (via
   *  `tabByCliSession` in onAskUser), which may not be the foreground
   *  `activeTab` — the user can switch tabs while a turn awaits an answer. */
  private tabHoldingAskBinding(toolUseId: string): TabState | null {
    for (const t of this.tabs.values()) {
      if (t.askUserBindings.has(toolUseId)) return t;
    }
    return null;
  }

  /** Look up the bridge request_id for an ask_user tool block. Returns null
   *  until the binding lands (one of two arrival orders). Called from
   *  ToolChip.svelte via a `$derived` so the chip activates the moment its
   *  requestId is known. Resolves through the OWNING tab, not `activeTab`,
   *  so the question is interactive even from a background/other-pane tab. */
  askUserRequestIdFor(toolUseId: string): string | null {
    return this.tabHoldingAskBinding(toolUseId)?.askUserBindings.get(toolUseId) ?? null;
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
    const tab = this.tabHoldingAskBinding(toolUseId);
    if (!tab) return;
    const requestId = tab.askUserBindings.get(toolUseId);
    if (!requestId) return;
    // RR8: pop the binding ONLY on success. The old `finally` deleted it even
    // when invoke threw, which drove the chip's askRequestId to null and
    // permanently disabled BOTH Submit and Dismiss — an unrecoverable lockup.
    // Preserving the binding on error lets the user retry from the chip.
    await invoke("assistant_answer_ask_user", { requestId, answer });
    const next = new Map(tab.askUserBindings);
    next.delete(toolUseId);
    tab.askUserBindings = next;
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

  /** Find the tab whose `permissionPrompts` holds this tool_use id (globally
   *  unique). Same owning-tab-vs-activeTab concern as ask_user — the prompt is
   *  registered on the tab that owns the CLI session, not necessarily the
   *  foreground tab. */
  private tabHoldingPermission(toolUseId: string): TabState | null {
    for (const t of this.tabs.values()) {
      if (t.permissionPrompts.has(toolUseId)) return t;
    }
    return null;
  }

  /** Look up a pending permission ask for a tool block. Called from
   *  ToolChip.svelte via a `$derived` so the chip's Allow/Deny buttons appear
   *  the moment the ask lands. Resolves through the OWNING tab, not
   *  `activeTab`, so Allow/Deny work from a background/other-pane tab. */
  permissionPromptFor(toolUseId: string): PermissionPromptInfo | null {
    return this.tabHoldingPermission(toolUseId)?.permissionPrompts.get(toolUseId) ?? null;
  }

  /** Answer a `can_use_tool` ask. `allow` writes `{behavior:"allow"}` (the CLI
   *  reuses the original input); `deny` writes `{behavior:"deny", message}`.
   *  Resolves the backend oneshot, which writes the control_response to the
   *  CLI's stdin and unblocks tool execution. The chip flips to its normal
   *  running/done state via the existing stream pipeline. */
  async submitPermissionDecision(toolUseId: string, allow: boolean): Promise<void> {
    const tab = this.tabHoldingPermission(toolUseId);
    if (!tab) return;
    const info = tab.permissionPrompts.get(toolUseId);
    if (!info) return;
    const decision = allow
      ? { behavior: "allow" }
      : { behavior: "deny", message: "User declined this action." };
    // RR8 (same as submitAskUserAnswer): pop the binding ONLY on success. A
    // `finally` would delete it even when invoke threw, flipping the chip to
    // "answered" while the backend oneshot stays unresolved — leaving the CLI
    // gate hung with no retry path. Preserve it on error so Allow/Deny stays live.
    await invoke("assistant_answer_permission", { requestId: info.requestId, decision });
    const next = new Map(tab.permissionPrompts);
    next.delete(toolUseId);
    tab.permissionPrompts = next;
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
    // A workspace's effort pin is stored independently of its model pin, so it
    // can outrank the model's ceiling — clamp on load too.
    this.thinkingEffort = clampEffort(loadEffort(ws), this.model);
    this.thinkingEnabled = loadThinkingEnabled(ws);
  }
  setRoot(path: string) { return wsSetRoot(this, path); }
  /** Per-pane folder picker / setter — scopes the chosen folder to one tab. */
  pickTabFolder(tabId: string | null) { return wsPickTabFolder(this, tabId); }
  setTabRoot(tabId: string | null, path: string) { return wsSetTabRoot(this, tabId, path); }
  removeRecentRoot(path: string) { return wsRemoveRecentRoot(this, path); }
  loadWorkspaceFiles() { return wsLoadFiles(this); }
  loadWorkspaceBranch() { return wsLoadBranch(this); }
  loadCustomCommands() { return wsLoadCustomCommands(this); }

  refreshConversations() { return persistRefresh(this); }

  flushNow() { persistFlushNow(this); }

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
    for (const id of this.openTabs) {
      const t = this.tabs.get(id);
      if (t?.streaming) await this.stop(id);
    }
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

  /** Start a fresh chat pinned to `id`. The honest answer to "I picked a new
   *  model mid-conversation": a resumed session is pinned to the model it was
   *  created with unless the user switches it. This mints a new tab and sets
   *  the model up front so its first turn runs against the user's pick —
   *  the fresh-start alternative to switching the current chat mid-flight. */
  async newChatWithModel(id: ModelSel) {
    await this.newTab();
    this.setModel(id);
  }

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
    notify.info("A sign-in window opened", { detail: "Complete the login there, then come back — Rift detects it automatically.", timeoutMs: 12000 });
    // Poll the probe until the session is usable, or give up after ~3 min so a
    // user who closes the login window without finishing isn't stuck "Signing
    // in…" forever.
    const deadline = Date.now() + 180_000;
    // Capture the init generation so a destroy()/HMR mid-poll cancels the loop
    // instead of writing auth state into a dead store for up to 3 min.
    const gen = this.initGen;
    try {
      while (Date.now() < deadline) {
        await new Promise((r) => setTimeout(r, 2500));
        if (gen !== this.initGen) return;
        await this.refreshAuth();
        if (gen !== this.initGen) return;
        const pill = this.auth?.pill;
        if (pill === "green" || pill === "yellow") {
          this.lastError = null;
          notify.ok("Signed in — you're good to go", { detail: "Resend your message to continue." });
          return;
        }
      }
      notify.warn("Sign-in didn't complete", { detail: "Finish in the console window that opened (or close it and try again), then resend." });
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
      notify.ok("Auth looks good now", { detail: "Resend your message to continue." });
    }
  }

  async setApiKey(key: string | null) {
    const v = key && key.trim().length > 0 ? key.trim() : null;
    try {
      await invoke("assistant_set_api_key", { apiKey: v });
      this.hasApiKey = v !== null;
      await this.refreshAuth();
    } catch (e) {
      notify.danger("Couldn't save API key", { detail: humanizeError(e) });
      throw e;
    }
  }

  async setUseFullConfig(value: boolean) {
    try {
      await invoke("assistant_set_use_full_config", { value });
      this.useFullConfig = value;
    } catch (e) {
      notify.danger("Couldn't change config setting", { detail: humanizeError(e) });
      throw e;
    }
  }

  async setMaxBudgetUsd(value: number | null) {
    const v = value !== null && Number.isFinite(value) && value > 0 ? value : null;
    try {
      await invoke("assistant_set_max_budget_usd", { value: v });
      this.maxBudgetUsd = v;
    } catch (e) {
      notify.danger("Couldn't set budget cap", { detail: humanizeError(e) });
      throw e;
    }
  }

  async setTrustLevel(value: TrustLevel) {
    try {
      await invoke("assistant_set_trust_level", { value });
      this.trustLevel = value;
    } catch (e) {
      notify.danger("Couldn't change trust level", { detail: humanizeError(e) });
      throw e;
    }
  }

  /** Turn dispatch (incl. client-side slash commands). M9: body in ./assistant/send. */
  async send(prompt: string, tabId?: string | null) {
    // Split-pane: a pane's composer fires with its own tabId. send() (via
    // sendImpl) and every activeTab-scoped getter it reads — streaming, queue,
    // composerAttachments, effectiveModel — key off currentConvoId, so retarget
    // it synchronously to the firing pane's tab first. Without this the turn
    // lands in whichever pane is focused, not the one that fired (#split-send).
    // The composer only renders for a loaded tab, so setFocusedPane never hits
    // its async loadConversation path here.
    if (tabId && tabId !== this.currentConvoId && this.tabFor(tabId)) {
      const idx = this.panes.findIndex((p) => p.tabId === tabId);
      if (idx >= 0) this.setFocusedPane(idx);
      else this.currentConvoId = tabId;
    }
    // Pass the explicit target through when it resolves — sendImpl then scopes
    // every tab read/write to it (belt-and-suspenders on top of the retarget).
    return sendImpl(this, prompt, tabId && this.tabFor(tabId) ? tabId : undefined);
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

  addTextAttachment(
    att: { name: string; text: string; sizeBytes: number; truncated: boolean },
    tabId?: string | null,
  ): boolean {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    return tab ? txtAdd(tab, att) : false;
  }

  removeTextAttachment(id: string, tabId?: string | null) {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    if (tab) txtRemove(tab, id);
  }

  clearTextAttachments(tabId?: string | null) {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    if (tab) txtClear(tab);
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
    checkTurnHealth(this, tab, convoId);
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

  removeQueued(id: string, tabId?: string) {
    sendRemoveQueued(this, id, tabId);
  }

  /** Composer wand: rewrite a rough draft into a clearer prompt. Stateless —
   *  the backend streams a headless rewrite token-by-token over
   *  `assistant://enhance-stream`, then resolves to the authoritative final
   *  text. `onDelta` receives the accumulated text on each chunk. `opts` steers
   *  it: `model` (default sonnet), `directive` (refine instruction), `cwd`
   *  (workspace dir → grounded read-only pass over the real code), `context`
   *  (conversation tail so mid-thread drafts resolve references), `previous`
   *  (last rewrite — a refine edits it instead of re-rolling). Callbacks:
   *  `onRequestId` hands back the id for `cancelEnhance`, `onStatus` gets
   *  grounded-lookup progress lines, `onMeta` the cost/duration footer from the
   *  terminal frame. Throws on failure so the caller can surface it. */
  async enhancePrompt(
    text: string,
    onDelta?: (full: string) => void,
    opts?: {
      model?: string;
      directive?: string;
      cwd?: string;
      context?: string;
      previous?: string;
      onRequestId?: (id: string) => void;
      onStatus?: (status: string) => void;
      onMeta?: (meta: { costUsd: number | null; durationMs: number | null }) => void;
    },
  ): Promise<string> {
    const requestId = crypto.randomUUID();
    opts?.onRequestId?.(requestId);
    let acc = "";
    const unlisten = await listen<{
      request_id: string;
      delta?: string;
      status?: string;
      done?: boolean;
      cost_usd?: number | null;
      duration_ms?: number | null;
    }>("assistant://enhance-stream", (e) => {
      if (e.payload.request_id !== requestId) return;
      if (e.payload.delta) {
        acc += e.payload.delta;
        onDelta?.(acc);
      }
      if (e.payload.status) opts?.onStatus?.(e.payload.status);
      if (e.payload.done) {
        opts?.onMeta?.({
          costUsd: e.payload.cost_usd ?? null,
          durationMs: e.payload.duration_ms ?? null,
        });
      }
    });
    try {
      return await invoke<string>("assistant_enhance_prompt", {
        requestId,
        prompt: text,
        model: opts?.model,
        directive: opts?.directive,
        cwd: opts?.cwd,
        context: opts?.context,
        previous: opts?.previous,
      });
    } finally {
      unlisten();
    }
  }

  /** Kill an in-flight enhance spawn (Discard while streaming). Best-effort —
   *  the pending enhancePrompt rejects with "enhance cancelled". */
  cancelEnhance(requestId: string) {
    void invoke("assistant_enhance_cancel", { requestId }).catch((e) =>
      console.warn("enhance cancel failed:", e),
    );
  }

  /** Re-send the most recent user prompt — see ./assistant/send.
   *  `tabId` scopes the retry to that pane's tab (split-pane Retry). */
  async retryLast(tabId?: string | null) {
    return sendRetryLast(this, tabId);
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
