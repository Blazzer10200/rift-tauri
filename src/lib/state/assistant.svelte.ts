// Assistant page state.
//
// Spawns the user's installed `claude` CLI through Rust commands; the CLI
// streams NDJSON which the backend forwards verbatim on `assistant://stream`.
// Wires Rift's MCP server (read_file / list_dir / grep) so assistant turns
// can interleave text, tool calls, and TodoWrite-driven task lists.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { accessibility } from "./accessibility.svelte";
import { toast } from "./toast.svelte";

// M0 split (2026-05-26): type defs lifted to `./assistant/types`. Re-exported
// here so external callers like `import type { Block } from "$lib/state/assistant.svelte"`
// keep working. See `docs/design/assistant-svelte-split.md`.
export type {
  WorkspaceState,
  AuthStatus,
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
  flattenToolResult,
  previewToolInput,
  messagesHaveContextSignals,
  effortToFlag,
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
import { saveSessionLog, pruneSessionLogs } from "./assistant/sessionLog";
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
class TabState {
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
  attachments = $state<{ id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[]>([]);
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
   *  / dedupe state and flips streaming on. */
  beginTurn() {
    this.lastError = null;
    this.seenToolUseIds.clear();
    this.deltaCount = 0;
    this.envelopeTextBuffer = "";
    this.rawLineLog = [];
    if (this.drainHandle !== null) {
      cancelAnimationFrame(this.drainHandle);
      this.drainHandle = null;
    }
    this.pendingText = "";
    this.thinkingByIndex.clear();
    this.activeThinkingIndex = null;
    this.lastStreamEventAt = null;
    this.activity = { currentLabel: null, turnStartedAt: Date.now() };
    this.streaming = true;
  }

  // ── streaming pipeline ────────────────────────────────────────────────

  private mutateStreaming(fn: (m: ChatMessage) => ChatMessage) {
    if (!this.streamingMsgId) return;
    const idx = this.streamingMsgIdx;
    if (idx !== null && idx >= 0 && idx < this.messages.length) {
      const m = this.messages[idx];
      if (m && m.id === this.streamingMsgId) {
        this.messages[idx] = fn(m);
        return;
      }
    }
    this.messages = this.messages.map((m) => (m.id === this.streamingMsgId ? fn(m) : m));
  }

  private beginThinking(index: number) {
    if (this.thinkingByIndex.has(index)) return;
    this.activeThinkingIndex = index;
    const startedAt = Date.now();
    this.mutateStreaming((m) => {
      const blocks = m.blocks.slice();
      this.thinkingByIndex.set(index, { blockOffset: blocks.length, startedAt });
      blocks.push({
        type: "thinking",
        text: "",
        hasSignature: false,
        startedAt,
        durationMs: null,
        status: "active",
      });
      return { ...m, blocks };
    });
    this.activity = { ...this.activity, currentLabel: "Thinking…" };
    if (this.currentTurnRecord) this.currentTurnRecord.thinkingCount += 1;
  }

  private mutateThinking(index: number, fn: (b: ThinkingBlock) => ThinkingBlock) {
    const entry = this.thinkingByIndex.get(index);
    if (!entry) return;
    this.mutateStreaming((m) => {
      const blocks = m.blocks.slice();
      const target = blocks[entry.blockOffset];
      if (target && target.type === "thinking") {
        blocks[entry.blockOffset] = fn(target);
      }
      return { ...m, blocks };
    });
  }

  private appendThinkingText(index: number, chunk: string) {
    if (!chunk) return;
    this.mutateThinking(index, (b) => ({ ...b, text: b.text + chunk }));
  }

  private markThinkingSignature(index: number) {
    this.mutateThinking(index, (b) => (b.hasSignature ? b : { ...b, hasSignature: true }));
  }

  private endThinking(index: number) {
    const entry = this.thinkingByIndex.get(index);
    if (!entry) return;
    const durationMs = Date.now() - entry.startedAt;
    if (this.currentTurnRecord) {
      this.currentTurnRecord.thinkingTotalMs += durationMs;
      // Capture per-block detail. `charCount` stays 0 in -p mode (encrypted)
      // but is wired in case a future API version emits plaintext deltas;
      // `hasSignature` is the truthier "real thinking happened" signal today.
      let charCount = 0;
      let hasSignature = false;
      const msg = this.streamingMsgId ? this.messages.find((m) => m.id === this.streamingMsgId) : null;
      if (msg) {
        const block = msg.blocks[entry.blockOffset];
        if (block && block.type === "thinking") {
          charCount = block.text.length;
          hasSignature = block.hasSignature;
        }
      }
      this.currentTurnRecord.thinkingBlocks.push({
        startedAt: entry.startedAt,
        durationMs,
        charCount,
        hasSignature,
      });
    }
    this.mutateThinking(index, (b) => ({ ...b, status: "done", durationMs }));
    if (this.activeThinkingIndex === index) {
      this.activeThinkingIndex = null;
      if (this.activity.currentLabel === "Thinking…") {
        this.activity = { ...this.activity, currentLabel: null };
      }
    }
    // Drop the entry so the CLI's agentic loop — which reuses `index=0` for
    // each new thinking block after a tool round-trip — can re-`beginThinking`
    // cleanly. Without this, `thinkingCount` stays at 1 and `thinkingBlocks`
    // double-counts the same block w/ ever-growing cumulative durations.
    this.thinkingByIndex.delete(index);
  }

  private ensureThinkingFromEnvelope(block: { thinking?: string; signature?: string }) {
    if (!this.streamingMsgId) return;
    const msg = this.messages.find((m) => m.id === this.streamingMsgId);
    if (!msg) return;
    const existing = msg.blocks.find((b) => b.type === "thinking") as ThinkingBlock | undefined;
    const envText = typeof block.thinking === "string" ? block.thinking : "";
    const envSig = !!block.signature && block.signature.length > 0;
    if (existing) {
      if (envText.length > existing.text.length || (envSig && !existing.hasSignature)) {
        // #147: $state proxies aren't referentially equal across read sites,
        // so `b === existing` was always false → every call appended a new
        // block. Match by stable startedAt instead.
        const key = existing.startedAt;
        this.mutateStreaming((m) => ({
          ...m,
          blocks: m.blocks.map((b) =>
            b.type === "thinking" && b.startedAt === key
              ? { ...b, text: envText.length > b.text.length ? envText : b.text, hasSignature: b.hasSignature || envSig }
              : b,
          ),
        }));
      }
      return;
    }
    this.mutateStreaming((m) => ({
      ...m,
      blocks: [
        ...m.blocks,
        {
          type: "thinking",
          text: envText,
          hasSignature: envSig,
          startedAt: Date.now(),
          durationMs: null,
          status: "done",
        },
      ],
    }));
  }

  private appendText(chunk: string) {
    if (!chunk) return;
    this.mutateStreaming((m) => {
      const blocks = m.blocks.slice();
      const last = blocks[blocks.length - 1];
      if (last && last.type === "text") {
        blocks[blocks.length - 1] = { type: "text", text: last.text + chunk };
      } else {
        blocks.push({ type: "text", text: chunk });
      }
      return { ...m, blocks };
    });
  }

  private enqueueText(chunk: string) {
    if (!chunk) return;
    this.pendingText += chunk;
    if (this.drainHandle === null) {
      this.lastDrainAt = performance.now();
      this.drainHandle = requestAnimationFrame(this.drainTick);
    }
  }

  private drainTick = () => {
    if (this.pendingText.length === 0) {
      this.drainHandle = null;
      return;
    }
    const now = performance.now();
    const dt = Math.min(now - this.lastDrainAt, 100);
    this.lastDrainAt = now;
    const rate = Math.max(120, this.pendingText.length / 0.4);
    const n = Math.min(this.pendingText.length, Math.max(1, Math.round((rate * dt) / 1000)));
    const chunk = this.pendingText.slice(0, n);
    this.pendingText = this.pendingText.slice(n);
    this.appendText(chunk);
    this.drainHandle = requestAnimationFrame(this.drainTick);
  };

  flushPendingText() {
    if (this.drainHandle !== null) {
      cancelAnimationFrame(this.drainHandle);
      this.drainHandle = null;
    }
    if (this.pendingText.length > 0) {
      this.appendText(this.pendingText);
      this.pendingText = "";
    }
  }

  private applyTodoWrite(input: Record<string, unknown> | undefined): boolean {
    const raw = (input?.todos ?? []) as Array<{ content?: string; status?: string }>;
    // #178: content-keyed ids so a reorder/insert in the model's TodoWrite
    // doesn't force every downstream {#each} to destroy + remount. Existing
    // ids are reused when content matches; new content gets a fresh id.
    const byContent = new Map(this.tasks.map((t) => [t.content, t.id]));
    const used = new Set<string>();
    const next = raw
      .filter((t) => typeof t?.content === "string")
      .map((t) => {
        const content = t.content!;
        let id = byContent.get(content);
        if (id && !used.has(id)) {
          used.add(id);
        } else {
          let candidate = `todo-${content.slice(0, 24)}`;
          let n = 1;
          while (used.has(candidate)) candidate = `todo-${content.slice(0, 24)}-${n++}`;
          id = candidate;
          used.add(id);
        }
        return {
          id,
          content,
          status: (t.status === "in_progress" || t.status === "completed" ? t.status : "pending") as
            | "pending"
            | "in_progress"
            | "completed",
        };
      });
    this.tasks = next;
    if (next.length > 0 && !this.dockAutoOpenedThisConvo) {
      this.dockAutoOpenedThisConvo = true;
      return true;
    }
    return false;
  }

  // Newer Claude CLI task API: TaskCreate adds one task at a time (subject +
  // description + activeForm); TaskUpdate flips status by 1-based creation index
  // (taskId "1".."N"). Both feed the same dock Plan card as TodoWrite.
  private applyTaskCreate(input: Record<string, unknown> | undefined): boolean {
    const subject = typeof input?.subject === "string" ? (input.subject as string) : null;
    if (!subject) return false;
    this.taskCreateCount += 1;
    const id = String(this.taskCreateCount);
    this.tasks = [...this.tasks, { id, content: subject, status: "pending" }];
    if (!this.dockAutoOpenedThisConvo) {
      this.dockAutoOpenedThisConvo = true;
      return true;
    }
    return false;
  }

  private applyTaskUpdate(input: Record<string, unknown> | undefined): void {
    const taskId = input?.taskId != null ? String(input.taskId) : null;
    if (!taskId) return;
    const raw = input?.status;
    const status = (raw === "in_progress" || raw === "completed" ? raw : "pending") as
      | "pending"
      | "in_progress"
      | "completed";
    this.tasks = this.tasks.map((t) => (t.id === taskId ? { ...t, status } : t));
  }

  private appendToolUse(block: { id: string; name: string; input?: Record<string, unknown> }) {
    if (this.seenToolUseIds.has(block.id)) return;
    this.seenToolUseIds.add(block.id);
    if (this.currentTurnRecord) {
      this.currentTurnRecord.toolUses.push({
        name: block.name,
        id: block.id,
        startedAt: Date.now(),
        completedAt: null,
        durationMs: null,
        isError: null,
        inputPreview: previewToolInput(block.name, block.input),
      });
    }
    if (block.name === "TodoWrite") {
      const opensDock = this.applyTodoWrite(block.input);
      this.onTodoApplied?.(this, opensDock);
      return;
    }
    if (block.name === "TaskCreate") {
      const opensDock = this.applyTaskCreate(block.input);
      this.onTodoApplied?.(this, opensDock);
      return;
    }
    if (block.name === "TaskUpdate") {
      this.applyTaskUpdate(block.input);
      this.onTodoApplied?.(this, false);
      return;
    }
    if (block.name === "Task" || block.name === "Agent") {
      const subagentType = String(block.input?.subagent_type ?? "fork");
      const description = String(block.input?.description ?? "(no description)");
      this.agentSpawns = [
        ...this.agentSpawns,
        { id: block.id, subagentType, description, startedAt: Date.now(), completedAt: null, isError: false },
      ];
      return;
    }
    const DENY = new Set(["ToolSearch"]);
    if (DENY.has(block.name)) return;
    this.activity = {
      ...this.activity,
      currentLabel: this.shortToolLabel ? this.shortToolLabel(block.name, block.input) : block.name,
    };
    this.mutateStreaming((m) => ({
      ...m,
      blocks: [
        ...m.blocks,
        {
          type: "tool",
          id: block.id,
          name: block.name,
          input: block.input ?? {},
          result: null,
          isError: false,
          status: "pending",
          startedAt: Date.now(),
        },
      ],
    }));
    // ask_user pairing — see TabState.askUserBindings doc.
    if (block.name === "mcp__rift__ask_user") {
      this.unboundAskUserToolUseIds.push(block.id);
      this.tryBindAskUser();
    }
  }

  /** Drain the two ask_user FIFOs as long as both have entries. Each pair
   *  binds a toolUseId to a requestId in `askUserBindings`, making the chip
   *  in MessageBubble able to invoke the answer-submit command. */
  tryBindAskUser() {
    while (
      this.unboundAskUserToolUseIds.length > 0 &&
      this.unboundAskUserRequestIds.length > 0
    ) {
      const toolUseId = this.unboundAskUserToolUseIds.shift()!;
      const requestId = this.unboundAskUserRequestIds.shift()!;
      const next = new Map(this.askUserBindings);
      next.set(toolUseId, requestId);
      this.askUserBindings = next;
    }
  }

  private recordTurnUsage(u: Record<string, unknown>, accumulate: boolean) {
    const num = (v: unknown): number => (typeof v === "number" && Number.isFinite(v) ? v : 0);
    const turn = {
      input: num(u.input_tokens),
      output: num(u.output_tokens),
      cacheRead: num(u.cache_read_input_tokens),
      cacheCreate: num(u.cache_creation_input_tokens),
    };
    if (turn.input + turn.output + turn.cacheRead + turn.cacheCreate === 0) return;
    // Telemetry capture into the in-flight turn record. Both envelope +
    // result land here so S106's `byModel` / divergence metrics keep their
    // signal -- the pill below is the only thing that ignores envelope.
    if (this.currentTurnRecord) {
      if (accumulate) this.currentTurnRecord.resultUsage = turn;
      else this.currentTurnRecord.envelopeUsage = turn;
    }
    // The ctx pill must reflect point-in-time window occupancy, NOT cumulative
    // task usage. The `assistant` envelope (accumulate=false) carries one
    // request's usage — its input + cache_read is exactly how full the window
    // is right now. The `result` event (accumulate=true) sums usage across
    // every step of the agentic loop, so one long task reports >1M cache_read
    // (confirmed: anthropics/claude-agent-sdk-python#548). Driving the pill off
    // `result` spiked it to ~full the instant a task finished and tripped
    // auto-compact for no reason. So: the last envelope drives the pill; the
    // result feeds session totals + cost only. Mid-turn envelopes climb the
    // pill live, which is fine — the auto-compact effect is gated on !streaming.
    if (accumulate) {
      // result = cumulative-per-task → session stats only (intentionally summed).
      this.sessionUsage = {
        totalInput: this.sessionUsage.totalInput + turn.input,
        totalOutput: this.sessionUsage.totalOutput + turn.output,
        totalCacheRead: this.sessionUsage.totalCacheRead + turn.cacheRead,
        totalCacheCreate: this.sessionUsage.totalCacheCreate + turn.cacheCreate,
        turns: this.sessionUsage.turns + 1,
      };
    } else {
      // assistant envelope = point-in-time window occupancy → drives the pill.
      this.lastTurnUsage = turn;
    }
  }

  private fillToolResult(toolUseId: string, content: string, isError: boolean) {
    if (this.currentTurnRecord) {
      const rec = this.currentTurnRecord.toolUses.find((t) => t.id === toolUseId);
      if (rec) {
        const now = Date.now();
        rec.completedAt = now;
        rec.durationMs = now - rec.startedAt;
        rec.isError = isError;
      }
    }
    // S124: mark matching agent spawn done.
    const agentIdx = this.agentSpawns.findIndex((a) => a.id === toolUseId);
    if (agentIdx !== -1) {
      const next = this.agentSpawns.slice();
      next[agentIdx] = { ...next[agentIdx], completedAt: Date.now(), isError };
      this.agentSpawns = next;
    }
    const now = Date.now();
    this.mutateStreaming((m) => ({
      ...m,
      blocks: m.blocks.map((b) => {
        if (b.type !== "tool" || b.id !== toolUseId) return b;
        const durationMs = typeof b.startedAt === "number" ? now - b.startedAt : undefined;
        return { ...b, result: content, isError, status: isError ? "error" : "done", durationMs };
      }),
    }));
  }

  onStream(raw: string) {
    if (this.rawLineLog.length >= 200) this.rawLineLog.shift();
    this.rawLineLog.push(raw);
    let env: StreamEnvelope;
    try {
      env = JSON.parse(raw) as StreamEnvelope;
    } catch {
      if (this.streaming && this.streamingMsgId && raw.length > 0) {
        const prefix = this.deltaCount > 0 ? "\n" : "";
        this.deltaCount++;
        if (this.currentTurnRecord) {
          this.currentTurnRecord.deltaCount = this.deltaCount;
          if (this.currentTurnRecord.firstPaintAt == null) {
            this.currentTurnRecord.firstPaintAt = Date.now();
          }
        }
        this.enqueueText(prefix + raw);
      } else if (raw.length > 0) {
        // #182: post-done CLI dribble was silently dropped — surface in console
        // for observability so we know if a known CLI bug regresses.
        console.debug("[assistant] orphaned non-JSON line (post-done)", raw.slice(0, 80));
      }
      return;
    }
    switch (env.type) {
      case "stream_event": {
        if (this.currentTurnRecord) {
          this.currentTurnRecord.streamEventCount += 1;
          const now = Date.now();
          // Anchor against last event arrival, falling back to first-paint or
          // turn-start so the metric is defined even on the first stream_event.
          const last = this.lastStreamEventAt ?? this.currentTurnRecord.firstPaintAt ?? this.currentTurnRecord.ts;
          const gap = now - last;
          if (gap > this.currentTurnRecord.maxStreamGapMs) {
            this.currentTurnRecord.maxStreamGapMs = gap;
          }
          this.lastStreamEventAt = now;
        }
        const ev = env.event;
        const evType = ev?.type;
        const idx = typeof ev?.index === "number" ? ev.index : null;
        if (evType === "content_block_start" && ev?.content_block?.type === "thinking" && idx !== null) {
          this.beginThinking(idx);
        } else if (evType === "content_block_delta") {
          const d = ev?.delta;
          if (d?.type === "text_delta" && d.text) {
            this.deltaCount++;
            if (this.currentTurnRecord) {
              this.currentTurnRecord.deltaCount = this.deltaCount;
              if (this.currentTurnRecord.firstPaintAt == null) {
                this.currentTurnRecord.firstPaintAt = Date.now();
              }
            }
            this.enqueueText(d.text);
          } else if (d?.type === "thinking_delta" && typeof d.thinking === "string" && idx !== null) {
            this.appendThinkingText(idx, d.thinking);
          } else if (d?.type === "signature_delta" && idx !== null) {
            this.markThinkingSignature(idx);
          }
        } else if (evType === "content_block_stop" && idx !== null) {
          this.endThinking(idx);
        }
        break;
      }
      case "assistant": {
        if (this.currentTurnRecord) this.currentTurnRecord.assistantEnvCount += 1;
        const msgUsage = env.message?.usage;
        if (msgUsage) this.recordTurnUsage(msgUsage, false);
        for (const block of env.message?.content ?? []) {
          if (block.type === "tool_use") {
            this.appendToolUse(block);
          } else if (block.type === "text" && typeof block.text === "string") {
            this.envelopeTextBuffer += block.text;
          } else if (block.type === "thinking") {
            this.ensureThinkingFromEnvelope(block);
          }
        }
        break;
      }
      case "user": {
        for (const block of env.message?.content ?? []) {
          if (block.type === "tool_result") {
            this.fillToolResult(
              block.tool_use_id,
              flattenToolResult(block.content),
              block.is_error === true,
            );
          }
        }
        break;
      }
      case "result": {
        if (typeof env.total_cost_usd === "number") {
          this.totalCostUsd = (this.totalCostUsd ?? 0) + env.total_cost_usd;
          const turnCost = env.total_cost_usd;
          this.mutateStreaming((m) => ({ ...m, costUsd: turnCost }));
          if (this.currentTurnRecord) this.currentTurnRecord.costUsd = turnCost;
        }
        const resultUsage = (env as { usage?: Record<string, unknown> }).usage;
        if (resultUsage) this.recordTurnUsage(resultUsage, true);
        if (env.subtype && env.subtype !== "success") {
          // Whitelist (S105 A3): known CLI error subtypes surface as user-visible
          // errors; anything else logs but doesn't false-alarm. Pre-emptive guard
          // for post-compaction CLI subtypes we haven't seen yet.
          const KNOWN_ERRORS = new Set([
            "error_max_turns",
            "error_during_execution",
            "error_max_thinking_tokens",
          ]);
          if (KNOWN_ERRORS.has(env.subtype)) {
            this.lastError = `Run ended with subtype: ${env.subtype}`;
          } else {
            console.warn("[assistant] unrecognized result.subtype", env.subtype, env);
          }
        }
        break;
      }
      case "system": {
        const sysModel = typeof env.model === "string" ? env.model : null;
        if (sysModel) {
          this.lastModelId = sysModel;
          this.mutateStreaming((m) => ({ ...m, model: sysModel }));
          if (this.currentTurnRecord) this.currentTurnRecord.modelId = sysModel;
        }
        break;
      }
      default:
        break;
    }
  }

  onDone() {
    this.flushPendingText();
    let envelopeFallback = false;
    let blankTurn = false;
    if (this.deltaCount === 0 && this.envelopeTextBuffer.length > 0) {
      this.appendText(this.envelopeTextBuffer);
      envelopeFallback = true;
    } else if (this.deltaCount === 0 && this.envelopeTextBuffer.length === 0) {
      const msg = this.streamingMsgId
        ? this.messages.find((m) => m.id === this.streamingMsgId)
        : null;
      const hadTools = !!msg && msg.blocks.some((b) => b.type === "tool");
      if (!hadTools) blankTurn = true;
      if (!hadTools) {
        const lines = this.rawLineLog.slice();
        console.warn("[assistant] turn ended with no text and no tools. Raw stream lines:", lines);
        const types: string[] = [];
        const nonJsonSamples: string[] = [];
        for (const ln of lines) {
          try {
            const parsed = JSON.parse(ln) as { type?: string; subtype?: string };
            types.push(parsed.subtype ? `${parsed.type}:${parsed.subtype}` : (parsed.type ?? "?"));
          } catch {
            types.push("non-json");
            if (nonJsonSamples.length < 3) {
              nonJsonSamples.push(ln.length > 240 ? ln.slice(0, 240) + "…" : ln);
            }
          }
        }
        const fingerprint = `[${types.join(", ")}]`;
        const tail =
          nonJsonSamples.length > 0
            ? ` Non-JSON output: ${nonJsonSamples.map((s) => `"${s}"`).join(" | ")}`
            : " Full NDJSON in DevTools console.";
        this.lastError = `Blank response — CLI emitted ${lines.length} line(s): ${fingerprint}.${tail}`;
      }
    }
    this.streaming = false;
    this.streamingMsgId = null;
    this.streamingMsgIdx = null;
    this.seenToolUseIds.clear();
    // Drop any unanswered permission asks — the backend auto-denies on turn
    // end, so a lingering Allow/Deny chip would be dead.
    if (this.permissionPrompts.size > 0) this.permissionPrompts = new Map();
    this.activity = { ...this.activity, currentLabel: null };
    // Finalize telemetry for this turn.
    if (this.currentTurnRecord) {
      this.currentTurnRecord.doneAt = Date.now();
      this.currentTurnRecord.envelopeFallback = envelopeFallback;
      this.currentTurnRecord.blankTurn = blankTurn;
      if (!this.currentTurnRecord.endKind) {
        this.currentTurnRecord.endKind = blankTurn ? "error" : "success";
        if (blankTurn) this.currentTurnRecord.errorMsg = this.lastError ?? "blank turn";
      }
      this.currentTurnRecord = null;
    }
    this.onTurnComplete?.(this);
  }

  onError(msg: string) {
    this.lastError = msg;
    this.streaming = false;
    if (this.drainHandle !== null) {
      cancelAnimationFrame(this.drainHandle);
      this.drainHandle = null;
    }
    this.pendingText = "";
    if (this.streamingMsgId) {
      const id = this.streamingMsgId;
      this.messages = this.messages.filter((m) => !(m.id === id && m.blocks.length === 0));
      this.streamingMsgId = null;
    }
    this.streamingMsgIdx = null;
    this.seenToolUseIds.clear();
    if (this.permissionPrompts.size > 0) this.permissionPrompts = new Map();
    // Finalize telemetry.
    if (this.currentTurnRecord) {
      this.currentTurnRecord.doneAt = Date.now();
      this.currentTurnRecord.endKind = "error";
      this.currentTurnRecord.errorMsg = msg;
      this.currentTurnRecord = null;
    }
    // Mirror onDone: a turn that ends in error (or partial-stream-then-error,
    // which the user perceives as "completed") is still terminal — fire the
    // completion hook so the store drains any queued message instead of
    // leaving the chat stuck in queue mode.
    this.onTurnComplete?.(this);
  }
}

class AssistantStore {
  auth = $state<AuthStatus | null>(null);
  authChecking = $state(false);
  authError = $state<string | null>(null);
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
      void saveSessionLog({
        ...snap,
        model: this.model,
        workspace: this.workspace.current,
      });
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
  get composerAttachments(): { id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[] {
    return this.activeTab?.attachments ?? [];
  }
  set composerAttachments(v: { id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[]) {
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
  private retrying = false;
  // streamingMsgId / seenToolUseIds / dockAutoOpenedThisConvo / deltaCount /
  // envelopeTextBuffer / rawLineLog / pendingText / drainHandle / lastDrainAt /
  // thinkingByIndex / activeThinkingIndex now live on TabState.

  setModel(v: ModelSel) {
    if (this.model === v) return;
    const prev = this.model;
    this.model = v;
    saveModel(v);
    const midConvo = (this.activeTab?.messages.length ?? 0) > 0;
    this.telemetry.event("model.change", { from: prev, to: v, midConvo });
    if (midConvo) this.cacheBustHint("model");
  }

  setThinkingEffort(v: ThinkingEffort) {
    if (this.thinkingEffort === v) return;
    const prev = this.thinkingEffort;
    this.thinkingEffort = v;
    saveEffort(v);
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

  async send(prompt: string) {
    const trimmed = prompt.trim();
    // Empty prompts are allowed when attachments are staged (paste-and-go).
    // Drop only if BOTH the prompt and attachments are empty.
    if (!trimmed && this.composerAttachments.length === 0) return;
    // Try-handle as a slash command first; if it matched, we're done.
    if (trimmed.startsWith("/") && this.runSlash(trimmed)) return;
    // Auth chokepoint — every send path funnels here (composer Enter/button,
    // queue drains, programmatic retries). A turn with no usable Claude session
    // dies as "claude exited with 1"; block it, re-probe (state may be stale),
    // and surface the reason. Slash commands above are local, so they still run.
    if (!(this.auth?.pill === "green" || this.auth?.pill === "yellow")) {
      this.lastNotice =
        this.auth?.summary ??
        "Claude isn't set up on this machine — open Settings to sign in or add an API key.";
      void this.refreshAuth();
      return;
    }
    // Already streaming on this tab → queue instead of dropping.
    if (this.streaming) {
      this.queue = [...this.queue, { id: crypto.randomUUID(), text: trimmed }];
      return;
    }
    // Phase 2 (S72): the CLI owns conversation state now. First turn mints a
    // UUID and passes `--session-id`; subsequent turns pass `--resume`.
    // v0.4: newTab() mints currentConvoId up-front so the tab can render
    // before send() — gate isFirstTurn on convoCreatedAt instead so the very
    // first send still passes --session-id, not --resume.
    if (!this.currentConvoId) {
      this.currentConvoId = crypto.randomUUID();
    }
    // #143: per-tab fields live on TabState now — ensureTab BEFORE touching
    // them so the writes don't no-op via the store's activeTab=null setter
    // path. ensureTab seeds cliSessionId to convoId for fresh tabs; compaction
    // remints it later without recreating the tab.
    const tab = this.ensureTab(this.currentConvoId, this.currentConvoId);
    const isFirstTurn = !tab.convoCreatedAt || tab.forceNextFirstTurn;
    tab.forceNextFirstTurn = false;
    if (!tab.cliSessionId) {
      tab.cliSessionId = this.currentConvoId;
    }
    if (!tab.convoCreatedAt) {
      tab.convoCreatedAt = Date.now();
      tab.convoTitle = null;
    }
    // v0.4: catches the raw newConversation→send path (slash /new) so tabs
    // never drift out of sync with the streaming convo.
    if (!this.openTabs.includes(this.currentConvoId)) {
      this.openTabs = [...this.openTabs, this.currentConvoId];
      this.persistTabs();
    }
    tab.beginTurn();
    this.lastNotice = null;
    // #184: clear stale error banner so it doesn't bleed into the new turn.
    // Setter routes to tab.lastError when activeTab is set, store-level otherwise.
    this.lastError = null;
    // Telemetry: build the turn record + attach to tab. TabState fills it as
    // envelopes arrive; finalized in onDone/onError.
    const attachBytes = this.composerAttachments.reduce((s, a) => s + a.sizeBytes, 0);
    const turnRecord: TurnRecord = {
      ts: Date.now(),
      convoId: this.currentConvoId,
      cliSessionId: tab.cliSessionId,
      isFirstTurn,
      model: this.model,
      effort: this.thinkingEffort,
      effortFlag: effortToFlag(this.thinkingEffort, this.model),
      promptLen: trimmed.length,
      promptPreview: trimmed.length > 120 ? trimmed.slice(0, 120) + "…" : trimmed,
      attachmentsCount: this.composerAttachments.length,
      attachmentsBytes: attachBytes,
      envelopeUsage: null,
      resultUsage: null,
      modelId: null,
      costUsd: null,
      deltaCount: 0,
      streamEventCount: 0,
      assistantEnvCount: 0,
      maxStreamGapMs: 0,
      toolUses: [],
      thinkingCount: 0,
      thinkingTotalMs: 0,
      thinkingBlocks: [],
      envelopeFallback: false,
      blankTurn: false,
      firstPaintAt: null,
      doneAt: null,
      endKind: null,
    };
    this.telemetry.turns.push(turnRecord);
    tab.currentTurnRecord = turnRecord;
    // Track for /retry and Up-arrow recall. De-dupe consecutive identicals.
    if (tab.promptHistory[tab.promptHistory.length - 1] !== trimmed) {
      tab.promptHistory = [...tab.promptHistory, trimmed].slice(-50);
    }
    // User bubble text: when paste-and-go with no text, show an attachment
    // marker so the bubble isn't blank.
    const attachCount = this.composerAttachments.length;
    const bubbleText =
      trimmed.length > 0
        ? trimmed
        : attachCount === 1
        ? "📎 1 image"
        : `📎 ${attachCount} images`;
    // Build the user message blocks — image blocks (one per attachment) first,
    // then the text block. Order matches the visual stack (thumbs above text)
    // in MessageBubble's user-side render path.
    const userBlocks: Block[] = [];
    for (const a of this.composerAttachments) {
      userBlocks.push({
        type: "image",
        mime: a.mime,
        dataBase64: a.dataBase64,
        sizeBytes: a.sizeBytes,
      });
    }
    userBlocks.push({ type: "text", text: bubbleText });
    tab.messages = [
      ...tab.messages,
      { id: crypto.randomUUID(), role: "user", blocks: userBlocks },
    ];
    const asst: ChatMessage = { id: crypto.randomUUID(), role: "assistant", blocks: [] };
    tab.messages = [...tab.messages, asst];
    tab.streamingMsgId = asst.id;
    // #146: asst placeholder is at the tail of messages; cache its index so
    // mutateStreaming can index-replace instead of scanning the full array.
    tab.streamingMsgIdx = tab.messages.length - 1;
    // Snapshot attachments for this turn + clear the composer so a fast retype
    // doesn't accidentally re-attach.
    const turnAttachments = this.composerAttachments.map((a) => ({
      mime: a.mime,
      dataBase64: a.dataBase64,
    }));
    this.composerAttachments = [];
    // Phase C: drain pendingCompactionSummary onto THIS turn only.
    // The new CLI session was minted at compactConversation() but is
    // empty — this summary is the model's only context for what came
    // before. Cleared immediately after dispatch; never persists across
    // turns. If the invoke itself fails the summary is lost (next send
    // starts cold) — acceptable since the boundary message stays in the
    // UI for the user to copy out if they need to manually re-seed.
    const priorSummary = tab.pendingCompactionSummary ?? null;
    tab.pendingCompactionSummary = null;
    try {
      await invoke("assistant_send", {
        prompt: trimmed,
        sessionId: tab.cliSessionId,
        isFirstTurn,
        model: this.model,
        attachments: turnAttachments.length > 0 ? turnAttachments : null,
        dyslexiaMode: accessibility.dyslexiaMode,
        thinkingEffort: this.thinkingEffort,
        permissionMode: this.permissionMode,
        priorContextSummary: priorSummary,
      });
    } catch (e) {
      tab.onError(String(e));
    }
  }

  /** Stage a binary attachment for the next send. Returns false if the size
   *  cap would be exceeded; the composer surfaces a notice on rejection. */
  /** Stage a binary attachment on `tabId`'s tab — defaults to the active
   *  (focused-pane) tab when omitted. 20 MiB cumulative cap mirrors the
   *  backend guard. Returns false on overflow. */
  // M4 split (2026-05-26): per-tab attachment logic in `./assistant/attachments`.
  // Store methods stay as thin tab-resolving thunks routing to active/specified tab.
  addAttachment(
    att: { mime: string; dataBase64: string; previewUrl: string; sizeBytes: number },
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

  /** Fire the next queued message on `tab`, if any. Idempotent + guarded so
   *  it's safe to call from every terminal turn path (onDone / onError /
   *  session-lost) AND on tab activation — backgrounded completions defer the
   *  drain (auto-sending into a tab the user isn't looking at is surprising),
   *  so returning to the tab must re-trigger it or the queue strands forever.
   *  Bails unless `tab` is the active tab and idle. */
  private drainQueue(tab: TabState | null) {
    if (!tab || tab !== this.activeTab || tab.streaming || tab.queue.length === 0) return;
    const [next, ...rest] = tab.queue;
    tab.queue = rest;
    // #148: capture the active convo at pop time; if the user switches tabs OR
    // a new turn starts before the microtask fires, re-queue the head and bail.
    // The next completion or tab activation re-drains — never a silent strand.
    const capturedConvoId = this.currentConvoId;
    queueMicrotask(() => {
      if (this.currentConvoId !== capturedConvoId || tab.streaming) {
        if ([...this.tabs.values()].includes(tab)) {
          tab.queue = [next, ...tab.queue];
        }
        return;
      }
      this.send(next.text).catch(e => tab.onError(String(e)));
    });
  }

  /** Stop a tab's in-flight stream. Defaults to the focused-pane tab when
   *  `tabId` is omitted. Pre-clears the tab's streaming flag synchronously
   *  so any late `done` event for this session is idempotent. Other tabs
   *  keep streaming. */
  async stop(tabId?: string | null) {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    if (!tab || !tab.streaming) return;
    const sid = tab.cliSessionId;
    // #179: flush pacer-buffered text into the message BEFORE clearing
    // streamingMsgId — otherwise mutateStreaming's early-return drops it.
    tab.flushPendingText();
    tab.streaming = false;
    tab.streamingMsgId = null;
    tab.streamingMsgIdx = null;
    tab.seenToolUseIds.clear();
    tab.activity = { ...tab.activity, currentLabel: null };
    // Telemetry finalize as user-stop before the late done event lands.
    if (tab.currentTurnRecord) {
      tab.currentTurnRecord.doneAt = Date.now();
      tab.currentTurnRecord.endKind = "user-stop";
      tab.currentTurnRecord = null;
    }
    this.telemetry.event("turn.stop", { convoId: tab.cliSessionId });
    try {
      await invoke("assistant_stop", { sessionId: sid });
    } catch (e) {
      console.warn("assistant_stop failed", e);
    }
  }

  /** Steer the RUNNING turn: inject `text` into the live CLI stdin so the agent
   *  course-corrects at its next loop step (no restart, no lost work). Unlike
   *  the queue, this does NOT wait for the turn to finish. Falls back to the
   *  queue if the turn already ended (or the tab isn't streaming). Defaults to
   *  the focused-pane tab when `tabId` is omitted. */
  async steer(text: string, tabId?: string | null) {
    const trimmed = text.trim();
    if (!trimmed) return;
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    if (!tab) return;
    const enqueue = () => {
      tab.queue = [...tab.queue, { id: crypto.randomUUID(), text: trimmed }];
    };
    // No active turn locally → nothing to steer; queue as a normal follow-up.
    if (!tab.streaming) {
      enqueue();
      return;
    }
    const sid = tab.cliSessionId;
    try {
      const res = await invoke<string>("assistant_steer", { sessionId: sid, text: trimmed });
      if (res === "no_active_turn") {
        // Turn ended between keypress and IPC — don't lose the message.
        enqueue();
        return;
      }
      this.telemetry.event("turn.steer", { convoId: sid });
      toast.push({
        severity: "info",
        title: "Steering",
        detail: trimmed.length > 60 ? trimmed.slice(0, 60) + "…" : trimmed,
        timeoutMs: 2500,
      });
    } catch (e) {
      console.warn("assistant_steer failed", e);
      enqueue();
    }
  }

  removeQueued(id: string, tabId?: string) {
    const tab = tabId ? this.tabFor(tabId) : this.activeTab;
    if (tab) tab.queue = tab.queue.filter((q) => q.id !== id);
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

  /** Client-side slash commands. Returns true if input was consumed. */
  private runSlash(input: string): boolean {
    const [cmd, ...rest] = input.slice(1).split(/\s+/);
    const arg = rest.join(" ").trim();
    switch (cmd.toLowerCase()) {
      case "clear":
        void this.clearConversation();
        return true;
      case "new":
        void this.newTab();
        return true;
      case "history":
        this.ui.historyOpen = !this.ui.historyOpen;
        return true;
      case "stop":
        void this.stop();
        return true;
      case "model": {
        const v = arg.toLowerCase();
        if (v === "sonnet" || v === "opus" || v === "haiku") {
          this.setModel(v);
          this.lastNotice = `Model switched to ${v}.`;
        } else {
          this.lastError = `Unknown model "${arg}". Use sonnet, opus, or haiku.`;
        }
        return true;
      }
      case "retry":
        void this.retryLast();
        return true;
      case "copy":
        void this.copyLastAssistant();
        return true;
      case "cost":
        this.lastNotice =
          this.totalCostUsd != null
            ? `Session cost: $${this.totalCostUsd.toFixed(4)} USD across ${this.messages.filter((m) => m.role === "assistant").length} turn(s).`
            : "No cost recorded yet — send a message first.";
        return true;
      case "tools":
        this.lastNotice =
          "Tools available this turn: " +
          "Read / Write / Edit (files); Bash (shell, in workspace cwd); " +
          "Glob (filename patterns); Grep (content search); " +
          "WebFetch / WebSearch (open web); " +
          "TodoWrite (multi-step plans → Tasks dock). " +
          "Rift MCP: read_file / list_dir / grep (workspace-scoped helpers); " +
          "git_status / git_diff / git_log (and pull/commit/push when trust permits).";
        return true;
      case "diag": {
        const snap = this.telemetry.snapshot();
        const json = JSON.stringify(snap, null, 2);
        const sizeKb = Math.round(json.length / 102.4) / 10;
        navigator.clipboard
          .writeText(json)
          .then(() => {
            this.lastNotice = `Telemetry copied — ${snap.turnCount} turn(s), ${snap.events.length} event(s), ${sizeKb}KB. Paste into a code block here.`;
          })
          .catch((e) => { this.lastError = `Clipboard write failed: ${String(e)}`; });
        return true;
      }
      case "diag-clear":
        this.telemetry.reset();
        this.lastNotice = "Telemetry buffer cleared — fresh capture starting now.";
        return true;
      case "stats": {
        // Inline-readable session summary — same data as /diag's `summary`
        // block but rendered as a short notice line so you can pattern-hunt
        // without dumping JSON. Cheap to fire repeatedly mid-session.
        const snap = this.telemetry.snapshot();
        const s = snap.summary;
        if (s.totalTurns === 0) {
          this.lastNotice = "No turns captured yet this session — send a message first.";
          return true;
        }
        const slowT = s.slowestTurn ? ` slowest turn #${s.slowestTurn.idx} ${(s.slowestTurn.durationMs / 1000).toFixed(1)}s` : "";
        const costT = s.costliestTurn ? ` costliest #${s.costliestTurn.idx} $${s.costliestTurn.costUsd.toFixed(3)}` : "";
        const slowTool = s.slowestTool ? ` slowest tool ${s.slowestTool.name} ${(s.slowestTool.durationMs / 1000).toFixed(1)}s` : "";
        const stale = s.staleCacheTurns > 0 ? ` ⚠ ${s.staleCacheTurns} stale-cache turn(s)` : "";
        const tps = s.outputTokensPerSec != null ? `, ${s.outputTokensPerSec} tok/s` : "";
        this.lastNotice =
          `${s.totalTurns} turn(s), $${s.totalCostUsd.toFixed(3)}, ` +
          `avg TTFP ${s.avgTtfpMs ?? "—"}ms, ${s.toolCallTotal} tool call(s)${tps}.` +
          slowT + costT + slowTool + stale;
        return true;
      }
      case "compact": {
        // Phase C: full compact action. arg becomes the focus hint.
        void this.compactConversation(arg || undefined);
        return true;
      }
      case "summarize": {
        // Compaction Phase B debug — dry-runs the summarize primitive
        // and renders the result as a notice. No state mutation; the
        // actual compaction flow lands in Phase C.
        this.lastNotice = "Summarizing… (cheap model, no state change)";
        void this.summarizeCurrentSession(arg || undefined).then((res) => {
          if (!res) return; // error already on lastError
          const tk = res.inputTokens + res.cacheReadTokens + res.cacheCreateTokens;
          this.lastNotice =
            `Summary ($${res.costUsd.toFixed(4)} · ${tk.toLocaleString()} in / ${res.outputTokens.toLocaleString()} out · ${res.model}):\n\n${res.summary}`;
        });
        return true;
      }
      case "openincli": {
        const sid = this.currentCliSessionId;
        const ws = this.workspace.current;
        if (!sid) {
          this.lastError = "No active session yet — send a message first.";
          return true;
        }
        const cmd = ws ? `cd "${ws}" && claude --resume ${sid}` : `claude --resume ${sid}`;
        navigator.clipboard
          .writeText(cmd)
          .then(() => { this.lastNotice = `Copied to clipboard: ${cmd}`; })
          .catch((e) => { this.lastError = `Clipboard write failed: ${String(e)}`; });
        return true;
      }
      case "help":
        this.lastNotice =
          "Slash commands: /new · /clear · /history · /model · /retry · /copy · /stop · /tools · /cost · /compact · /summarize · /openincli · /diag · /diag-clear · /help. " +
          "/clear wipes the current chat in place (old convo saved to History); /new opens a separate tab. /openincli copies a `claude --resume` command for the standalone CLI. " +
          "/compact summarizes the current session + remints the CLI session id; the next turn carries the summary forward. " +
          "/summarize dry-runs Phase-B compaction summarize (no state change). " +
          "/diag exports session telemetry as JSON to clipboard. Up-arrow recalls previous prompts.";
        return true;
      default:
        return false;
    }
  }

  /** Re-send the most recent user prompt. Drops the prior user+assistant
   *  pair from the visible history so the retry looks like a redo, not a
   *  duplicate. Aborts an in-flight stream first. */
  async retryLast() {
    // #185: re-entrance guard so a fast double-click only strips one pair.
    if (this.retrying) return;
    this.retrying = true;
    try {
      const tab = this.activeTab;
      const last = tab?.promptHistory[tab.promptHistory.length - 1];
      if (!last || !tab) {
        this.lastError = "No previous prompt to retry.";
        return;
      }
      if (tab.streaming) {
        await this.stop();
      }
      // Strip the trailing assistant turn (if any) and the matching user turn
      // so the replayed history doesn't double-include the prompt.
      const msgs = tab.messages.slice();
      if (msgs[msgs.length - 1]?.role === "assistant") msgs.pop();
      if (msgs[msgs.length - 1]?.role === "user") msgs.pop();
      tab.messages = msgs;
      await this.send(last);
    } finally {
      this.retrying = false;
    }
  }

  /** Copy the latest assistant message's text content to the clipboard. */
  async copyLastAssistant() {
    const last = [...this.messages].reverse().find((m) => m.role === "assistant");
    if (!last) {
      this.lastError = "No assistant response to copy.";
      return;
    }
    const text = last.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim();
    if (!text) {
      this.lastError = "Last response had no text content.";
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      this.lastNotice = `Copied ${text.length.toLocaleString()} chars to clipboard.`;
    } catch (e) {
      this.lastError = `Clipboard write failed: ${String(e)}`;
    }
  }

  /** Up-arrow recall. Returns the n-th-most-recent prompt, or null. */
  recallPrompt(offsetFromEnd: number): string | null {
    const idx = this.promptHistory.length - 1 - offsetFromEnd;
    return idx >= 0 ? this.promptHistory[idx] : null;
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

export const assistant = new AssistantStore();
