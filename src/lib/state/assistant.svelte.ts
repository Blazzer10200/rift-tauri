// Assistant page state.
//
// Spawns the user's installed `claude` CLI through Rust commands; the CLI
// streams NDJSON which the backend forwards verbatim on `assistant://stream`.
// Wires Rift's MCP server (read_file / list_dir / grep) so assistant turns
// can interleave text, tool calls, and TodoWrite-driven task lists.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { accessibility } from "./accessibility.svelte";

export type WorkspaceState = {
  current: string | null;
  recent: string[];
};

export type AuthStatus = {
  cliPresent: boolean;
  cliVersion: string | null;
  loggedIn: boolean;
  authMethod: string | null;
  apiProvider: string | null;
  email: string | null;
  subscriptionType: string | null;
  apiKeyConfigured: boolean;
  pill: "green" | "yellow" | "red";
  summary: string;
};

export type ToolBlock = {
  type: "tool";
  id: string;
  name: string;
  input: Record<string, unknown>;
  result: string | null;
  isError: boolean;
  status: "pending" | "done" | "error";
  // S124: wall-clock start (ms epoch) on tool_use, end (ms epoch) on
  // tool_result. Lets the chip render an inline duration badge when the
  // call was slow (>1s). Optional — legacy records omit both fields.
  startedAt?: number;
  durationMs?: number;
};

export type TextBlock = {
  type: "text";
  text: string;
};

export type ThinkingBlock = {
  type: "thinking";
  // Plaintext reasoning if the API streamed it. Often empty in -p mode —
  // Anthropic encrypts thinking content and only emits the signature, in
  // which case we show duration + a "reasoning recorded" hint instead.
  text: string;
  // Encrypted signature blob received (presence flag — we don't render it).
  hasSignature: boolean;
  // Wall-clock start (ms epoch) — set on content_block_start. Used by the
  // bubble to render a live elapsed counter during active reasoning. Without
  // this the UI sat at "Thinking …" for the full 17-40s of an Opus reasoning
  // block; the live counter converts dead air into a heartbeat.
  startedAt: number;
  // Wall-clock duration of the reasoning step. Null while still active.
  durationMs: number | null;
  status: "active" | "done";
};

/** Compaction Phase C: synthetic block that marks the boundary where a
 *  CLI session was retired in favor of a summary. Renders as a collapsed
 *  pill ("Conversation compacted · N turns archived") with the summary
 *  text on expand. Owned by a `role: "system"` message — third role
 *  alongside user/assistant. */
export type BoundaryBlock = {
  type: "boundary";
  summary: string;
  at: number;
  archivedCount: number;
  costUsd: number;
  summaryModel: string;
  // S124: true while the summarize call is in-flight; flipped to false on
  // the final 'done' event. Drives the streaming spinner in MessageBubble.
  streaming?: boolean;
  // Phase E1: ctx% snapshot at the moment the compact fired (pre) and the
  // estimated ctx% the new session starts at (post = summary tokens / window).
  // Rendered as "Ctx X% → est Y%" in the boundary pill so the user sees
  // headroom won. Optional for legacy boundaries pre-E1.
  ctxPctBefore?: number;
  ctxPctEstAfter?: number;
};

export type Block = TextBlock | ToolBlock | ThinkingBlock | BoundaryBlock;

export type ChatMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  blocks: Block[];
  /** Per-turn cost in USD captured from the CLI `result` envelope. Only set
   *  on assistant messages after the turn completes. */
  costUsd?: number | null;
  /** Resolved model id captured from the CLI `system:init` envelope. */
  model?: string | null;
};

export type ConversationMeta = {
  id: string;
  title: string;
  model: string;
  messageCount: number;
  createdAt: number;
  updatedAt: number;
  /** Phase E5: flattened compactionHistory summaries for HistoryDrawer
   *  search. Absent or empty for convos that never compacted. */
  compactionSummaries?: string[];
};

/** Compaction Phase B output. Mirrors `assistant::SummarizeResult` in
 *  `assistant/mod.rs` (camelCase serde). Phase C consumes the summary
 *  text + cost figures when minting a boundary message. */
export type SummarizeResult = {
  summary: string;
  model: string;
  costUsd: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreateTokens: number;
};

type CompactionHistoryEntry = {
  at: number;
  priorSessionId: string;
  newSessionId: string;
  summary: string;
  costUsd: number;
  summaryModel: string;
  archivedCount: number;
};

type ConversationRecord = {
  id: string;
  title: string;
  model: string;
  createdAt: number;
  updatedAt: number;
  messages: ChatMessage[];
  // CLI session UUID (--session-id / --resume target). Decoupled from `id` in
  // S103 so compaction can mint a fresh CLI session without breaking tab
  // persistence. Optional for backward compat — legacy convos fall back to
  // `id` on load.
  cliSessionId?: string;
  // Phase E prerequisite: ordered list of compactions that happened on this
  // convo. The BoundaryBlock in `messages` is the user-visible artifact; this
  // is the structured record for search / cleanup sweep. Absent for legacy.
  compactionHistory?: CompactionHistoryEntry[];
};

// Minimal stream-json envelope shape we care about.
type ContentBlock =
  | { type: "text"; text: string }
  | { type: "thinking"; thinking?: string; signature?: string }
  | { type: "tool_use"; id: string; name: string; input?: Record<string, unknown> }
  | { type: "tool_result"; tool_use_id: string; content?: unknown; is_error?: boolean };

type StreamDelta = {
  type?: string;
  text?: string;
  thinking?: string;
  signature?: string;
};

type StreamEvent = {
  type?: string;
  index?: number;
  content_block?: ContentBlock;
  delta?: StreamDelta;
};

type StreamEnvelope =
  | { type: "system"; subtype?: string; [k: string]: unknown }
  | { type: "stream_event"; event?: StreamEvent; [k: string]: unknown }
  | { type: "assistant"; message: { content: ContentBlock[] } }
  | { type: "user"; message: { content: ContentBlock[] } }
  | { type: "result"; subtype?: string; result?: string; total_cost_usd?: number; [k: string]: unknown };

type RemoteLockEvt = {
  file_path: string;
  user: string;
  host: string;
  since: string;
};

type RemoteShellEvt = {
  command: string;
  remote_root: string;
  at: string;
};

const MODEL_KEY = "rift.assistant.model";
const EFFORT_KEY = "rift.assistant.thinkingEffort";

export type ThinkingEffort = "none" | "quick" | "deep";

function loadModel(): "sonnet" | "opus" | "haiku" {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(MODEL_KEY) : null;
    if (v === "sonnet" || v === "opus" || v === "haiku") return v;
  } catch {
    /* SSR or storage disabled */
  }
  return "sonnet";
}

function saveModel(v: "sonnet" | "opus" | "haiku") {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(MODEL_KEY, v);
  } catch {
    /* storage disabled */
  }
}

function loadEffort(): ThinkingEffort {
  try {
    const v = typeof localStorage !== "undefined" ? localStorage.getItem(EFFORT_KEY) : null;
    if (v === "none" || v === "quick" || v === "deep") return v;
  } catch {
    /* SSR or storage disabled */
  }
  return "quick";
}

function saveEffort(v: ThinkingEffort) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(EFFORT_KEY, v);
  } catch {
    /* storage disabled */
  }
}

function flattenToolResult(content: unknown): string {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .map((c) => (typeof c === "object" && c && "text" in c ? String((c as { text: unknown }).text ?? "") : ""))
      .join("");
  }
  return "";
}

/** Telemetry record for a single Claude turn. Filled progressively as
 *  envelopes arrive; finalized on done / error / session-lost. Captured into
 *  AssistantStore.telemetry.turns so a `/diag` export can show the full
 *  session shape (cache behavior, tool patterns, blank turns, model switches,
 *  cost trend). */
type TurnRecord = {
  // Identity
  ts: number;                                      // turn start (ms epoch)
  convoId: string;
  cliSessionId: string;
  isFirstTurn: boolean;
  model: "sonnet" | "opus" | "haiku";
  effort: "none" | "quick" | "deep";
  /** Actual `--effort` flag the CLI is invoked with (mirrors mod.rs mapping).
   *  Haiku doesn't get an effort flag → null. */
  effortFlag: "low" | "medium" | "high" | null;
  // Input
  promptLen: number;
  /** First ~120 chars of the user's prompt, post-trim. Lets a `/diag` reader
   *  identify which turn was which without the raw text dump. */
  promptPreview: string;
  attachmentsCount: number;
  attachmentsBytes: number;
  // Usage (filled progressively — envelope = mid-stream, result = final)
  envelopeUsage: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  resultUsage: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  modelId: string | null;                          // resolved "claude-sonnet-4-6" etc.
  costUsd: number | null;
  // Stream stats
  deltaCount: number;
  /** Number of `stream_event` envelopes received this turn. Zero is the
   *  smoking gun for `--include-partial-messages` not being honored. */
  streamEventCount: number;
  /** Number of `assistant` envelopes (per-message snapshots). */
  assistantEnvCount: number;
  /** Longest pause btw consecutive `stream_event` envelopes this turn (ms).
   *  CONFLATED w/ tool wall-time: a 12s Bash will register as a 12s gap b/c
   *  the CLI sits idle waiting for `tool_result`. Cross-ref with the largest
   *  `toolUses[].durationMs` of the same turn — if maxStreamGapMs ≈ that
   *  tool's duration, it's just tool wait. If it's bigger or there's no
   *  matching tool, it's a real API/network stall. */
  maxStreamGapMs: number;
  /** Per-tool record w/ wall timing + error status + a short input preview.
   *  `completedAt` stays null if the `tool_result` never arrived (turn ended
   *  early or tool hung). `inputPreview` is first ~120 chars of the most
   *  diagnostic field (command for Bash, file_path for Read/Write/Edit,
   *  pattern for Grep/Glob, url for WebFetch, query for WebSearch) — answers
   *  "the Bash ran 12s, doing WHAT?". */
  toolUses: {
    name: string;
    id: string;
    startedAt: number;
    completedAt: number | null;
    durationMs: number | null;
    isError: boolean | null;
    inputPreview: string | null;
  }[];
  thinkingCount: number;
  thinkingTotalMs: number;
  /** Per-block detail filled in endThinking. Lets a `/diag` reader see if a
   *  turn had one long think vs many short interleaved thinks. */
  /** `charCount` stays 0 in `-p` mode (API encrypts thinking plaintext);
   *  `hasSignature` is the truthier "did we get a real thinking block" signal. */
  thinkingBlocks: { startedAt: number; durationMs: number; charCount: number; hasSignature: boolean }[];
  envelopeFallback: boolean;                       // fired the "zero deltas, flush envelope" path
  blankTurn: boolean;                              // ended w/ no text + no tools
  // Timing
  firstPaintAt: number | null;
  doneAt: number | null;
  endKind: "success" | "user-stop" | "session-lost" | "error" | null;
  errorMsg?: string;
};

/** First-priority field to preview for each known tool. Returns first ~120
 *  chars of that field's string value, or null. Keeps /diag readable while
 *  still answering "what did this tool actually do?". */
function previewToolInput(name: string, input: Record<string, unknown> | undefined): string | null {
  if (!input) return null;
  const fields = ["command", "file_path", "pattern", "path", "url", "query"] as const;
  for (const f of fields) {
    const v = input[f];
    if (typeof v === "string" && v.length > 0) {
      return v.length > 120 ? v.slice(0, 120) + "…" : v;
    }
  }
  return null;
}

/** Effort → CLI flag mapping. Must mirror src-tauri/src/assistant/mod.rs. */
function effortToFlag(
  effort: "none" | "quick" | "deep",
  model: "sonnet" | "opus" | "haiku",
): "low" | "medium" | "high" | null {
  if (model === "haiku") return null;
  if (effort === "none") return "low";
  if (effort === "deep") return "high";
  return "medium";
}

/** Session-wide telemetry singleton. */
class SessionTelemetry {
  startedAt = Date.now();
  turns: TurnRecord[] = [];
  /** Non-turn lifecycle events: tab open/close/new/switch, slash commands,
   *  workspace changes, session-lost recoveries, etc. Cheap to capture. */
  events: { ts: number; kind: string; detail?: unknown }[] = [];

  event(kind: string, detail?: unknown) {
    this.events.push({ ts: Date.now(), kind, detail });
  }

  /** JSON snapshot for /diag clipboard export. */
  snapshot() {
    return {
      startedAt: this.startedAt,
      capturedAt: Date.now(),
      durationMs: Date.now() - this.startedAt,
      turnCount: this.turns.length,
      summary: this.summarize(),
      turns: this.turns,
      events: this.events,
    };
  }

  /** Per-session rollup. Self-summarizing JSON so a `/diag` reader doesn't
   *  have to fold over `turns[]` to see the basics. */
  private summarize() {
    const byModel: Record<string, {
      turns: number;
      costUsd: number;
      inputTokens: number;
      outputTokens: number;
      cacheReadTokens: number;
      cacheCreateTokens: number;
      thinkingTurns: number;
      blankTurns: number;
      envelopeFallbacks: number;
      avgTtfpMs: number | null;
      avgDoneMs: number | null;
    }> = {};
    let totalCost = 0;
    let blank = 0;
    let envFallback = 0;
    let thinkingTurns = 0;
    let toolCallTotal = 0;
    let toolErrorTotal = 0;
    let slowestTool: { name: string; durationMs: number; turnIdx: number } | null = null;
    const toolNameCounts: Record<string, number> = {};
    let slowestTurn: { idx: number; durationMs: number } | null = null;
    let costliestTurn: { idx: number; costUsd: number } | null = null;
    let firstTurnCostUsd: number | null = null;
    let coldStartCacheCreate: number | null = null;
    let totalOutputTokens = 0;
    let totalStreamMs = 0;
    let mostParallelTurn: { idx: number; maxConcurrentTools: number } | null = null;
    let staleCacheTurns = 0;
    const ttfps: number[] = [];
    const doneTimes: number[] = [];
    for (let i = 0; i < this.turns.length; i++) {
      const t = this.turns[i];
      // Skip user-stop / error turns w/ no resolved modelId from the byModel
      // rollup — they otherwise create a phantom "opus"/"sonnet"/"haiku"
      // bucket alongside the real "claude-opus-4-7" etc.
      if (t.modelId == null && t.endKind !== "success") continue;
      const key = t.modelId || t.model;
      const bucket = byModel[key] ||= {
        turns: 0, costUsd: 0, inputTokens: 0, outputTokens: 0,
        cacheReadTokens: 0, cacheCreateTokens: 0,
        thinkingTurns: 0, blankTurns: 0, envelopeFallbacks: 0,
        avgTtfpMs: null, avgDoneMs: null,
      };
      bucket.turns += 1;
      bucket.costUsd += t.costUsd ?? 0;
      const u = t.resultUsage || t.envelopeUsage;
      if (u) {
        bucket.inputTokens += u.input;
        bucket.outputTokens += u.output;
        bucket.cacheReadTokens += u.cacheRead;
        bucket.cacheCreateTokens += u.cacheCreate;
      }
      if (t.thinkingCount > 0) { bucket.thinkingTurns += 1; thinkingTurns += 1; }
      if (t.blankTurn) { bucket.blankTurns += 1; blank += 1; }
      if (t.envelopeFallback) { bucket.envelopeFallbacks += 1; envFallback += 1; }
      totalCost += t.costUsd ?? 0;
      if (t.firstPaintAt != null) ttfps.push(t.firstPaintAt - t.ts);
      if (t.doneAt != null) {
        const dur = t.doneAt - t.ts;
        doneTimes.push(dur);
        if (!slowestTurn || dur > slowestTurn.durationMs) slowestTurn = { idx: i, durationMs: dur };
      }
      if (t.costUsd != null && (!costliestTurn || t.costUsd > costliestTurn.costUsd)) {
        costliestTurn = { idx: i, costUsd: t.costUsd };
      }
      // Tool rollup + parallelism detection via sweep-line over intervals.
      const intervals: { ts: number; delta: 1 | -1 }[] = [];
      for (const tu of t.toolUses) {
        toolCallTotal += 1;
        toolNameCounts[tu.name] = (toolNameCounts[tu.name] ?? 0) + 1;
        if (tu.isError === true) toolErrorTotal += 1;
        if (tu.durationMs != null && (!slowestTool || tu.durationMs > slowestTool.durationMs)) {
          slowestTool = { name: tu.name, durationMs: tu.durationMs, turnIdx: i };
        }
        if (tu.completedAt != null) {
          intervals.push({ ts: tu.startedAt, delta: 1 });
          intervals.push({ ts: tu.completedAt, delta: -1 });
        }
      }
      if (intervals.length > 0) {
        intervals.sort((a, b) => a.ts - b.ts || b.delta - a.delta);
        let active = 0;
        let peak = 0;
        for (const iv of intervals) {
          active += iv.delta;
          if (active > peak) peak = active;
        }
        if (!mostParallelTurn || peak > mostParallelTurn.maxConcurrentTools) {
          mostParallelTurn = { idx: i, maxConcurrentTools: peak };
        }
      }
      // Cold-start surfacing: the first turn typically pays the SessionStart
      // 40-50K cache_creation tax; we record turn[0]'s cost+cacheCreate to
      // make that tax legible without folding turns[].
      if (i === 0) {
        firstTurnCostUsd = t.costUsd ?? null;
        const u0 = t.resultUsage || t.envelopeUsage;
        coldStartCacheCreate = u0?.cacheCreate ?? null;
      }
      // Stale-cache flag: a continuation turn that paid full cache_create but
      // got zero cache_read = the API isn't reusing our prefix. Flagged what
      // surfaced the sonnet cache anomaly during effort A/B.
      if (!t.isFirstTurn && t.endKind === "success") {
        const uForCache = t.resultUsage || t.envelopeUsage;
        if (uForCache && uForCache.cacheRead === 0 && uForCache.cacheCreate > 0) {
          staleCacheTurns += 1;
        }
      }
      // Streaming velocity accumulator.
      if (t.firstPaintAt != null && t.doneAt != null && t.doneAt > t.firstPaintAt) {
        const u = t.resultUsage || t.envelopeUsage;
        if (u) {
          totalOutputTokens += u.output;
          totalStreamMs += t.doneAt - t.firstPaintAt;
        }
      }
    }
    // Per-model timing averages
    for (const key of Object.keys(byModel)) {
      const bucket = byModel[key];
      const tns = this.turns.filter((t) => (t.modelId || t.model) === key);
      const t1 = tns.map((t) => (t.firstPaintAt != null ? t.firstPaintAt - t.ts : null)).filter((n): n is number => n != null);
      const t2 = tns.map((t) => (t.doneAt != null ? t.doneAt - t.ts : null)).filter((n): n is number => n != null);
      bucket.avgTtfpMs = t1.length ? Math.round(t1.reduce((a, b) => a + b, 0) / t1.length) : null;
      bucket.avgDoneMs = t2.length ? Math.round(t2.reduce((a, b) => a + b, 0) / t2.length) : null;
    }
    return {
      totalTurns: this.turns.length,
      totalCostUsd: Math.round(totalCost * 10000) / 10000,
      blankTurns: blank,
      envelopeFallbacks: envFallback,
      thinkingTurns,
      avgTtfpMs: ttfps.length ? Math.round(ttfps.reduce((a, b) => a + b, 0) / ttfps.length) : null,
      avgDoneMs: doneTimes.length ? Math.round(doneTimes.reduce((a, b) => a + b, 0) / doneTimes.length) : null,
      toolCallTotal,
      toolErrorTotal,
      toolNameCounts,
      slowestTool,
      slowestTurn,
      costliestTurn,
      firstTurnCostUsd,
      coldStartCacheCreate,
      mostParallelTurn,
      staleCacheTurns,
      outputTokensPerSec: totalStreamMs > 0
        ? Math.round((totalOutputTokens / totalStreamMs) * 1000)
        : null,
      byModel,
      eventCounts: this.events.reduce<Record<string, number>>((acc, e) => {
        acc[e.kind] = (acc[e.kind] ?? 0) + 1;
        return acc;
      }, {}),
    };
  }

  reset() {
    this.startedAt = Date.now();
    this.turns = [];
    this.events = [];
  }
}

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
  /** #145: per-tab save debounce timer — was store-level (single slot).
   *  Each tab tracking its own timer means flushNow() on beforeunload can
   *  iterate every unsaved tab instead of dropping background-tab edits. */
  saveTimer: ReturnType<typeof setTimeout> | null = null;

  messages = $state<ChatMessage[]>([]);
  streaming = $state(false);
  tasks = $state<{ id: string; content: string; status: "pending" | "in_progress" | "completed" }[]>([]);
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

  // Non-reactive per-stream internals.
  streamingMsgId: string | null = null;
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
        this.mutateStreaming((m) => ({
          ...m,
          blocks: m.blocks.map((b) =>
            b.type === "thinking" && b === existing
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
    const next = raw
      .filter((t) => typeof t?.content === "string")
      .map((t, i) => ({
        id: `todo-${i}-${t.content!.slice(0, 24)}`,
        content: t.content!,
        status: (t.status === "in_progress" || t.status === "completed" ? t.status : "pending") as
          | "pending"
          | "in_progress"
          | "completed",
      }));
    this.tasks = next;
    if (next.length > 0 && !this.dockAutoOpenedThisConvo) {
      this.dockAutoOpenedThisConvo = true;
      return true;
    }
    return false;
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
    if (block.name === "Task" || block.name === "Agent") {
      const subagentType = String((block.input?.subagent_type as string) ?? "fork");
      const description = String((block.input?.description as string) ?? "(no description)");
      this.agentSpawns = [
        ...this.agentSpawns,
        { id: block.id, subagentType, description, startedAt: Date.now(), completedAt: null, isError: false },
      ];
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
    // #1: only update the pill on the `result` event. Envelope arrives
    // first w/ a partial count that the result corrects on complex turns
    // (thinking blocks, tool loops); rendering both made the pill visibly
    // jump. Pill now sits on the previous turn's confirmed value through
    // the in-flight turn and lands on the new value once result arrives.
    if (accumulate) {
      this.lastTurnUsage = turn;
      this.sessionUsage = {
        totalInput: this.sessionUsage.totalInput + turn.input,
        totalOutput: this.sessionUsage.totalOutput + turn.output,
        totalCacheRead: this.sessionUsage.totalCacheRead + turn.cacheRead,
        totalCacheCreate: this.sessionUsage.totalCacheCreate + turn.cacheCreate,
        turns: this.sessionUsage.turns + 1,
      };
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
        const msgUsage = (env.message as { usage?: Record<string, unknown> } | undefined)?.usage;
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
    this.seenToolUseIds.clear();
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
    this.seenToolUseIds.clear();
    // Finalize telemetry.
    if (this.currentTurnRecord) {
      this.currentTurnRecord.doneAt = Date.now();
      this.currentTurnRecord.endKind = "error";
      this.currentTurnRecord.errorMsg = msg;
      this.currentTurnRecord = null;
    }
  }
}

/** Per-pane reference into the openTabs list. v2 split UI: `panes` is always
 *  an array of length ≥1. Length 1 = single-pane (no visible split). Length
 *  2..MAX_PANES = horizontal split. Each pane shows the chat for its `tabId`;
 *  `null` tabId = empty pane (drop a tab into it from the tabsbar). */
export type PaneState = { tabId: string | null };

/** Hard cap on horizontal panes. 4 × min-width 320px = 1280px — fits any
 *  modern window. Bump if you're on ultrawide; UI is array-driven so the
 *  only knob is this constant. */
export const MAX_PANES = 4;

class AssistantStore {
  auth = $state<AuthStatus | null>(null);
  authChecking = $state(false);
  authError = $state<string | null>(null);

  /** Per-conversation streaming state, keyed by Rift convoId. One entry per
   *  open chat tab. The store's UI-facing `messages` / `streaming` / `activity`
   *  / etc. getters delegate to `activeTab`; event handlers route by
   *  `session_id` to whichever tab owns that CLI session. Concurrent live
   *  streaming on 2+ tabs works because each tab carries its own messages
   *  buffer, pacer state, and thinking tracker. */
  private tabs = $state(new Map<string, TabState>());

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
  get ctxWindow(): number {
    const model = this.lastModelId;
    if (!model) return 200_000;
    if (/\[1m\]/i.test(model)) return 1_000_000;
    const id = model.toLowerCase();
    if (id.includes("haiku")) return 200_000;
    if (/sonnet-4-[56]/.test(id) || /opus-4-[67]/.test(id)) return 1_000_000;
    return 200_000;
  }
  get ctxTokens(): number {
    const u = this.lastTurnUsage;
    return u ? u.input + u.cacheRead + u.cacheCreate : 0;
  }
  get ctxPct(): number {
    const w = this.ctxWindow;
    return w > 0 ? Math.min(100, (this.ctxTokens / w) * 100) : 0;
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
  addPane() {
    if (this.panes.length >= MAX_PANES) return;
    const taken = new Set(this.panes.map((p) => p.tabId).filter((x): x is string => !!x));
    const fill = this.openTabs.find((id) => !taken.has(id)) ?? null;
    const insertAt = this.focusedPaneIdx + 1;
    const next = this.panes.slice();
    next.splice(insertAt, 0, { tabId: fill });
    this.panes = next;
    this.telemetry.event("pane.add", { count: next.length, fill });
    // Focus the freshly-added pane so subsequent newTab/openTab assigns to it.
    this.stashTabUi(this.currentConvoId);
    this.focusedPaneIdx = insertAt;
    if (fill) {
      const inMeta = this.conversations.some((c) => c.id === fill);
      if (inMeta && !this.tabs.get(fill)) {
        void this.loadConversation(fill);
      } else {
        this.currentConvoId = fill;
      }
    }
    this.restoreTabUi(fill);
    this.persistTabs();
  }

  /** Close a pane (the pane container, not the tab inside it). Tabs stay in
   *  openTabs — closing a pane just unhooks it. Last pane never closes (always
   *  length≥1). Focused idx is clamped to the new array bounds. Persists. */
  closePane(idx: number) {
    if (this.panes.length <= 1) return;
    if (idx < 0 || idx >= this.panes.length) return;
    const next = this.panes.slice();
    next.splice(idx, 1);
    this.panes = next;
    this.telemetry.event("pane.close", { remaining: next.length });
    // Clamp focused. If we closed the focused pane (or one before it), shift left.
    let newFocus = this.focusedPaneIdx;
    if (idx < this.focusedPaneIdx) newFocus -= 1;
    else if (idx === this.focusedPaneIdx) newFocus = Math.min(idx, next.length - 1);
    newFocus = Math.max(0, Math.min(newFocus, next.length - 1));
    if (newFocus !== this.focusedPaneIdx) {
      this.setFocusedPane(newFocus);
    } else {
      this.persistTabs();
    }
  }

  /** Move focus to a pane. Stashes outgoing composer draft + restores incoming
   *  so each pane carries its own draft. No-op in single-pane mode. */
  setFocusedPane(idx: number) {
    if (idx < 0 || idx >= this.panes.length) return;
    if (this.focusedPaneIdx === idx && this.currentConvoId === this.panes[idx].tabId) return;
    this.stashTabUi(this.currentConvoId);
    this.focusedPaneIdx = idx;
    const next = this.panes[idx].tabId;
    if (next) {
      const inMeta = this.conversations.some((c) => c.id === next);
      if (inMeta && !this.tabs.get(next)) {
        void this.loadConversation(next);
      } else {
        this.currentConvoId = next;
      }
    } else {
      this.currentConvoId = null;
    }
    this.restoreTabUi(next);
    this.persistTabs();
  }

  /** Assign a tab to the currently-focused pane. Called by openTab/newTab so
   *  the focused pane's slot follows the active selection. Works in both
   *  single-pane (length=1) and split modes. */
  private assignFocusedPane(tabId: string | null) {
    const cur = this.panes[this.focusedPaneIdx];
    if (!cur || cur.tabId === tabId) return;
    const next = this.panes.slice();
    next[this.focusedPaneIdx] = { tabId };
    this.panes = next;
  }

  /** Drop a tab from the tabsbar into a specific pane.
   *  - Single-pane mode (panes.length===1) + dropping a DIFFERENT tab on a
   *    half → enter 2-pane split (existing behavior).
   *  - Multi-pane mode → if target pane already holds this tab, just focus it.
   *    If a SIBLING pane holds it, swap. Else assign + focus.
   *  - paneIdx === panes.length is a sentinel meaning "drop in a new pane at
   *    the end" — auto-adds (cap-aware) and assigns. */
  dropTabIntoPane(tabId: string, paneIdx: number) {
    if (!this.openTabs.includes(tabId)) return;
    if (paneIdx < 0) return;

    // Sentinel: "add new pane at end". Cap-respecting.
    if (paneIdx >= this.panes.length) {
      if (this.panes.length >= MAX_PANES) return;
      const next = this.panes.slice();
      next.push({ tabId });
      this.panes = next;
      const newIdx = next.length - 1;
      this.stashTabUi(this.currentConvoId);
      this.focusedPaneIdx = newIdx;
      const inMeta = this.conversations.some((c) => c.id === tabId);
      if (inMeta && !this.tabs.get(tabId)) {
        void this.loadConversation(tabId);
      } else {
        this.currentConvoId = tabId;
      }
      this.restoreTabUi(tabId);
      this.persistTabs();
      return;
    }

    if (this.panes.length === 1) {
      // Single-pane → drop on a half = enter split. paneIdx is 0 or 1 from
      // the half-detect. If the dragged tab IS the only-pane tab, ignore.
      if (tabId === this.currentConvoId) return;
      const other = paneIdx === 0 ? 1 : 0;
      const next: PaneState[] = [{ tabId: null }, { tabId: null }];
      next[paneIdx] = { tabId };
      next[other] = { tabId: this.currentConvoId };
      this.panes = next;
      this.telemetry.event("pane.split.on", { via: "drag", p0: next[0].tabId, p1: next[1].tabId });
    } else {
      // Already split: same tab in target = focus only.
      if (this.panes[paneIdx].tabId === tabId) {
        this.setFocusedPane(paneIdx);
        return;
      }
      // Same tab in a SIBLING pane = swap (mirror UX).
      const siblingIdx = this.panes.findIndex((p, i) => i !== paneIdx && p.tabId === tabId);
      if (siblingIdx !== -1) {
        const swapped = this.panes.slice();
        swapped[siblingIdx] = { tabId: this.panes[paneIdx].tabId };
        swapped[paneIdx] = { tabId };
        this.panes = swapped;
        this.setFocusedPane(paneIdx);
        return;
      }
      const next = this.panes.slice();
      next[paneIdx] = { tabId };
      this.panes = next;
    }
    // Move focus to the freshly-dropped pane + sync currentConvoId.
    this.stashTabUi(this.currentConvoId);
    this.focusedPaneIdx = paneIdx;
    if (tabId !== this.currentConvoId) {
      const inMeta = this.conversations.some((c) => c.id === tabId);
      if (inMeta && !this.tabs.get(tabId)) {
        void this.loadConversation(tabId);
      } else {
        this.currentConvoId = tabId;
      }
    }
    this.restoreTabUi(tabId);
    this.persistTabs();
  }

  /** When a tab closes, scrub it from any pane that pointed at it. Panes
   *  become empty (null); the pane container stays so the user can drop a
   *  different tab in or close the pane manually. */
  private scrubTabFromPanes(id: string) {
    let changed = false;
    const next = this.panes.map((p) => {
      if (p.tabId === id) { changed = true; return { tabId: null }; }
      return p;
    });
    if (changed) this.panes = next;
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
  private ensureTab(convoId: string, cliSessionId: string): TabState {
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
  private dropTab(convoId: string) {
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

  apiKey = $state<string | null>(null);
  useFullConfig = $state<boolean>(true);
  maxBudgetUsd = $state<number | null>(null);
  allowRemoteShell = $state<boolean>(false);
  // Phase D: fraction (0-1] of ctx window that auto-fires compactConversation.
  // Null = manual only (matches the user's DISABLE_AUTO_COMPACT=1 stance).
  autoCompactThreshold = $state<number | null>(null);
  // Phase D: model alias used by summarize call. "haiku" default ($0.91 vs
  // $2.73 on sonnet for a 900K-token summarize).
  compactModel = $state<"haiku" | "sonnet">("haiku");
  remoteShellLockedByOther = $state<{ user: string; host: string; sinceMs: number } | null>(null);
  remoteShellBannerSeen = $state<boolean>(false);
  remoteShellLastEvent = $state<{ command: string; remoteRoot: string; at: string } | null>(null);

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

  composerDraft = $state("");
  // Pasted/dropped binary attachments staged for the next send. Each carries
  // base64 + mime so the backend can emit a stream-json `image` content
  // block. `previewUrl` is a data URL for the in-composer thumbnail; it's
  // cheap to keep here since the same bytes are already in dataBase64.
  composerAttachments = $state<{ id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[]>([]);
  // queue moved to TabState (S105 follow-up) — per-tab so a queued msg in
  // Tab A can't drain into Tab B if the user switches mid-turn. UI binds via
  // the `queue` getter below which delegates to activeTab.
  // User's chosen model — flipped by /model slash command. Carried through
  // to assistant_send so the CLI uses sonnet/opus/haiku per their choice.
  // Initialized from localStorage so the choice survives reloads.
  model = $state<"sonnet" | "opus" | "haiku">(loadModel());
  // Extended-thinking budget tier. "none" = no extended thinking (fastest);
  // "quick" = 2K budget (default, balanced); "deep" = 10K (heavy reasoning).
  // Haiku ignores this server-side. Persisted to localStorage.
  thinkingEffort = $state<ThinkingEffort>(loadEffort());
  // `dockOpen` drives the inline TasksDock in AssistantPage. `historyOpen`
  // is retained as a no-op flag for back-compat w/ any remaining slash
  // command — History is now its own workspace, not an overlay.
  ui = $state({ dockOpen: false, tasksUpdatedAt: 0, historyOpen: false });

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
  private get convoCreatedAt(): number | null { return this.activeTab?.convoCreatedAt ?? null; }
  private set convoCreatedAt(v: number | null) {
    if (this.activeTab) this.activeTab.convoCreatedAt = v;
  }
  private get convoTitle(): string | null { return this.activeTab?.convoTitle ?? null; }
  private set convoTitle(v: string | null) {
    if (this.activeTab) this.activeTab.convoTitle = v;
  }

  // tasks + activity now live on TabState (see top-of-class getters).

  // Per-tab UI state cache. Survives tab switches so the composer draft,
  // staged attachments, and scroll position aren't wiped when the user clicks
  // away to check another convo. Saved in openTab/closeTab/newTab before the
  // active id flips; restored after the flip. Entries pruned when a tab is
  // closed (the convo itself stays on disk; only the in-memory UI scratch is
  // dropped).
  private tabDrafts = new Map<string, string>();
  private tabAttachments = new Map<
    string,
    { id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[]
  >();
  private tabScroll = new Map<string, number>();

  private unlistens: UnlistenFn[] = [];
  // streamingMsgId / seenToolUseIds / dockAutoOpenedThisConvo / deltaCount /
  // envelopeTextBuffer / rawLineLog / pendingText / drainHandle / lastDrainAt /
  // thinkingByIndex / activeThinkingIndex now live on TabState.

  setModel(v: "sonnet" | "opus" | "haiku") {
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

  /** One-shot-per-session-per-kind notice when model/effort flips on a tab
   *  that already has turns. Sonnet's cache empirically does NOT survive
   *  effort changes (S106 measurement: 0 cacheRead on 3 consecutive sonnet
   *  turns w/ effort flips vs healthy reuse without). Opus is more forgiving.
   *  Notice is fire-once so it's a hint, not a nag. */
  private cacheBustHintShown = { model: false, effort: false };
  private cacheBustHint(kind: "model" | "effort") {
    if (this.cacheBustHintShown[kind]) return;
    this.cacheBustHintShown[kind] = true;
    this.lastNotice =
      kind === "effort"
        ? "Heads up — changing effort mid-conversation can bust the prompt cache (esp. on Sonnet). Next turn may pay full cache_create."
        : "Heads up — switching models mid-conversation rebuilds the prefix cache from scratch. Next turn will pay full cache_create.";
  }

  /** Snapshot the OUTGOING tab's composer + attachments into the cache.
   *  Call BEFORE flipping currentConvoId. Pass the convoId being left.
   *  Scroll position is captured separately by AssistantPage. */
  private stashTabUi(id: string | null) {
    if (!id) return;
    if (this.composerDraft.length > 0) {
      this.tabDrafts.set(id, this.composerDraft);
    } else {
      this.tabDrafts.delete(id);
    }
    if (this.composerAttachments.length > 0) {
      this.tabAttachments.set(id, this.composerAttachments);
    } else {
      this.tabAttachments.delete(id);
    }
  }

  /** Pull cached composer + attachments for the INCOMING tab. Call AFTER
   *  flipping currentConvoId. Missing entry → blank state. */
  private restoreTabUi(id: string | null) {
    this.composerDraft = (id && this.tabDrafts.get(id)) || "";
    this.composerAttachments = (id && this.tabAttachments.get(id)) || [];
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

  /** Drop all per-tab UI scratch for a closed tab. */
  private pruneTabUi(id: string) {
    this.tabDrafts.delete(id);
    this.tabAttachments.delete(id);
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
      await listen<{ session_id?: string; exit_code?: number } | { exit_code: number }>(
        "assistant://done",
        (e) => {
          const p = e.payload as { session_id?: string } | undefined;
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
      this.apiKey = await invoke<string | null>("assistant_get_api_key");
    } catch (e) {
      console.warn("assistant_get_api_key failed", e);
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
      this.allowRemoteShell = await invoke<boolean>("assistant_get_allow_remote_shell");
    } catch (e) {
      console.warn("assistant_get_allow_remote_shell failed", e);
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
      this.remoteShellBannerSeen = localStorage.getItem("rift.assistant.remoteShellBannerSeen") === "1";
    } catch { /* localStorage unavailable in some test contexts */ }

    this.unlistens.push(
      await listen<RemoteLockEvt[]>("autosync://locks", (e) => this.onLocksUpdate(e.payload)),
      await listen<RemoteShellEvt>("assistant://remote-shell-fired", (e) => this.onRemoteShellFired(e.payload)),
      await listen<{ session_id: string; prompt: string }>(
        "assistant://session-lost",
        (e) => this.onSessionLost(e.payload),
      ),
    );

    await this.refreshConversations();
    await this.refreshWorkspace();
    await this.restoreTabs();

    // Best-effort flush on window close so we don't lose the last turn
    // sitting inside the 700ms scheduleSave debounce. See flushNow() doc.
    if (typeof window !== "undefined") {
      window.addEventListener("beforeunload", () => this.flushNow());
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

  private onLocksUpdate(locks: RemoteLockEvt[]) {
    const shell = locks.find((l) => l.file_path.endsWith("/.rift-shell"));
    if (shell) {
      const sinceMs = Date.parse(shell.since);
      this.remoteShellLockedByOther = {
        user: shell.user,
        host: shell.host,
        sinceMs: Number.isFinite(sinceMs) ? sinceMs : Date.now(),
      };
    } else {
      this.remoteShellLockedByOther = null;
    }
  }

  private onRemoteShellFired(evt: RemoteShellEvt) {
    this.remoteShellLastEvent = {
      command: evt.command,
      remoteRoot: evt.remote_root,
      at: evt.at,
    };
  }

  ackRemoteShellBanner() {
    this.remoteShellBannerSeen = true;
    try {
      localStorage.setItem("rift.assistant.remoteShellBannerSeen", "1");
    } catch { /* same as above */ }
  }

  async refreshWorkspace() {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_get_workspace");
    } catch (e) {
      console.warn("assistant_get_workspace failed", e);
    }
  }

  /** Native folder picker → set as active root. Returns false if user cancelled. */
  async pickFolder(): Promise<boolean> {
    try {
      const result = await openDialog({ directory: true, multiple: false });
      const path = typeof result === "string" ? result : null;
      if (!path) return false;
      await this.setRoot(path);
      return true;
    } catch (e) {
      this.lastError = `Open folder failed: ${String(e)}`;
      return false;
    }
  }

  async setRoot(path: string) {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_set_root", { path });
      this.workspaceFiles = [];
      this.lastNotice = `Workspace: ${path}`;
    } catch (e) {
      this.lastError = `Set workspace failed: ${String(e)}`;
    }
  }

  async clearRoot() {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_clear_root");
      this.workspaceFiles = [];
    } catch (e) {
      console.warn("assistant_clear_root failed", e);
    }
  }

  async removeRecentRoot(path: string) {
    try {
      this.workspace = await invoke<WorkspaceState>("assistant_remove_recent_root", { path });
    } catch (e) {
      console.warn("assistant_remove_recent_root failed", e);
    }
  }

  /** Lazy-load relative file paths under the current workspace root. Caches
   *  per-root in `workspaceFiles`; concurrent calls are de-duped via the
   *  `workspaceFilesLoadingFor` guard. */
  async loadWorkspaceFiles() {
    const root = this.workspace.current;
    if (!root) { this.workspaceFiles = []; return; }
    if (this.workspaceFilesLoadingFor === root) return;
    this.workspaceFilesLoadingFor = root;
    try {
      this.workspaceFiles = await invoke<string[]>("assistant_list_workspace_files");
    } catch (e) {
      console.warn("assistant_list_workspace_files failed", e);
    } finally {
      this.workspaceFilesLoadingFor = null;
    }
  }

  async refreshConversations() {
    try {
      this.conversations = await invoke<ConversationMeta[]>("assistant_list_conversations");
    } catch (e) {
      console.warn("assistant_list_conversations failed", e);
    }
  }

  /** Derive a human-friendly title from the first user message. #145: now
   *  takes the tab as an arg so a debounced doSave reads from the originating
   *  tab's messages, not whichever tab is active when the timer fires. */
  private deriveTitle(tab: TabState): string {
    const first = tab.messages.find((m) => m.role === "user");
    if (!first) return "New conversation";
    const text = first.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim()
      .replace(/\s+/g, " ");
    return text.length > 60 ? text.slice(0, 60) + "…" : text || "New conversation";
  }

  /** Build the on-disk record + fire-and-forget save for a single tab.
   *  Shared by flushNow + scheduleSave so the snapshot semantics live in one
   *  place. #145: cliSessionId / createdAt / title all sourced from the tab
   *  passed in, not store-level — debounced save can't redirect mid-flight. */
  private buildSaveRecord(convoId: string, tab: TabState): ConversationRecord {
    return {
      id: convoId,
      title: tab.convoTitle ?? this.deriveTitle(tab),
      model: this.model,
      createdAt: tab.convoCreatedAt ?? Date.now(),
      updatedAt: Date.now(),
      messages: tab.messages,
      cliSessionId: tab.cliSessionId || convoId,
      compactionHistory: tab.compactionHistory.length > 0 ? tab.compactionHistory : undefined,
    };
  }

  /** Best-effort synchronous flush of all open tabs. Wired to
   *  `beforeunload` in init() — without this, a window close within the
   *  scheduleSave 700ms debounce loses the last turn. We fire the IPC
   *  without awaiting (browser drops pending promises on unload), but the
   *  Tauri runtime typically completes the in-flight invoke before the
   *  process actually exits.
   *  #145: iterate every tab with content, not just the active one. A
   *  background-tab turn that finished mid-debounce would otherwise be lost. */
  flushNow() {
    for (const [convoId, tab] of this.tabs) {
      if (tab.messages.length === 0) continue;
      if (tab.saveTimer) {
        clearTimeout(tab.saveTimer);
        tab.saveTimer = null;
      }
      const record = this.buildSaveRecord(convoId, tab);
      tab.convoTitle = record.title;
      tab.convoCreatedAt = record.createdAt;
      void invoke("assistant_save_conversation", { convo: record }).catch((e) => {
        console.warn("flushNow save failed", e);
      });
    }
  }

  /** Persist the current conversation. Debounced — callers can fire freely;
   *  only one disk write per ~700ms per tab. Set `flush=true` to write
   *  immediately. #145: snapshots (tab, convoId) at call time so a 700ms
   *  delay can't dispatch the save against whichever tab is active when the
   *  timer fires. */
  private scheduleSave(flush = false) {
    const convoId = this.currentConvoId;
    const tab = this.activeTab;
    if (!tab || !convoId || tab.messages.length === 0) return;
    if (tab.saveTimer) {
      clearTimeout(tab.saveTimer);
      tab.saveTimer = null;
    }
    const doSave = async () => {
      tab.saveTimer = null;
      const record = this.buildSaveRecord(convoId, tab);
      tab.convoTitle = record.title;
      tab.convoCreatedAt = record.createdAt;
      try {
        await invoke("assistant_save_conversation", { convo: record });
        await this.refreshConversations();
      } catch (e) {
        console.warn("assistant_save_conversation failed", e);
      }
    };
    if (flush) void doSave();
    else tab.saveTimer = setTimeout(doSave, 700);
  }

  /** Start a fresh conversation. Flushes the current one first so nothing
   *  is lost when the user clicks `+ New`. */
  async newConversation() {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0) this.scheduleSave(true);
    if (this.currentConvoId) this.dropTab(this.currentConvoId);
    this.queue = [];
    this.lastNotice = null;
    this.ui.dockOpen = false;
    this.currentConvoId = null;
    this.currentCliSessionId = null;
    this.convoCreatedAt = null;
    this.convoTitle = null;
  }

  async loadConversation(id: string) {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0 && this.currentConvoId && this.currentConvoId !== id) {
      this.scheduleSave(true);
    }
    try {
      const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
      // Legacy convos lack cliSessionId — fall back to id so --resume still
      // hits the original JSONL. New convos persist cliSessionId explicitly.
      const cliSid = convo.cliSessionId ?? convo.id;
      const tab = this.ensureTab(convo.id, cliSid);
      // Re-hydrate from disk — overwrites in-memory state if the tab was
      // previously open with stale data.
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
      this.currentConvoId = convo.id;
      this.currentCliSessionId = cliSid;
      this.convoCreatedAt = convo.createdAt;
      this.convoTitle = convo.title;
      this.queue = [];
      this.lastNotice = null;
      this.ui.historyOpen = false;
      if (convo.model === "sonnet" || convo.model === "opus" || convo.model === "haiku") {
        this.setModel(convo.model);
      }
    } catch (e) {
      this.lastError = `Failed to load conversation: ${String(e)}`;
    }
  }

  async deleteConversation(id: string) {
    try {
      await invoke("assistant_delete_conversation", { id });
      if (this.openTabs.includes(id)) {
        // Reuse closeTab so neighbor-pick + active-switch logic stays in one place.
        await this.closeTab(id);
      } else if (this.currentConvoId === id) {
        this.dropTab(id);
        this.currentConvoId = null;
        this.currentCliSessionId = null;
        this.convoCreatedAt = null;
        this.convoTitle = null;
      } else {
        // Convo was open as a TabState (e.g. background) but not the active tab.
        this.dropTab(id);
      }
      await this.refreshConversations();
    } catch (e) {
      this.lastError = `Failed to delete conversation: ${String(e)}`;
    }
  }

  // ── v0.4 tabs ────────────────────────────────────────────────────────
  private persistTabs() {
    try {
      localStorage.setItem(
        "rift.ui.tabs.v1",
        JSON.stringify({
          openTabs: this.openTabs,
          activeTabId: this.currentConvoId,
          panes: this.panes,
          focusedPaneIdx: this.focusedPaneIdx,
        }),
      );
    } catch { /* localStorage unavailable */ }
  }

  private async restoreTabs() {
    try {
      const raw = localStorage.getItem("rift.ui.tabs.v1");
      if (!raw) return;
      const parsed = JSON.parse(raw) as {
        openTabs?: unknown;
        activeTabId?: unknown;
        panes?: unknown;
        focusedPaneIdx?: unknown;
      };
      const ids = Array.isArray(parsed.openTabs)
        ? parsed.openTabs.filter((s): s is string => typeof s === "string")
        : [];
      const existing = new Set(this.conversations.map((c) => c.id));
      const valid = ids.filter((id) => existing.has(id));
      this.openTabs = valid;
      const active = typeof parsed.activeTabId === "string" ? parsed.activeTabId : null;
      if (active && valid.includes(active)) {
        await this.loadConversation(active);
      } else if (valid.length > 0) {
        await this.loadConversation(valid[0]);
      }
      // Restore split state — N-pane shape. Accepts length 1..MAX_PANES.
      // Stale tab refs are pruned to null (pane survives, empty). Legacy
      // null/missing keeps single-pane default.
      if (Array.isArray(parsed.panes) && parsed.panes.length >= 1 && parsed.panes.length <= MAX_PANES) {
        const norm = (p: unknown): PaneState => {
          const id = (p as { tabId?: unknown })?.tabId;
          return { tabId: typeof id === "string" && valid.includes(id) ? id : null };
        };
        const restored = parsed.panes.map(norm);
        // Keep at least one pane; if all restored panes are empty and we're
        // single-length, that's fine — assignFocusedPane will fill on next open.
        this.panes = restored.length > 0 ? restored : [{ tabId: null }];
        const fi = typeof parsed.focusedPaneIdx === "number" ? parsed.focusedPaneIdx : 0;
        this.focusedPaneIdx = Math.max(0, Math.min(fi, this.panes.length - 1));
        // Sync currentConvoId to focused pane if needed.
        const focused = this.panes[this.focusedPaneIdx].tabId;
        if (focused && focused !== this.currentConvoId) {
          await this.loadConversation(focused);
        }
      }
      this.persistTabs();
    } catch (e) {
      console.warn("restoreTabs failed", e);
    }
  }

  /** Open a saved convo as a tab. Push to openTabs if not already there;
   *  activate + load from disk. Unsaved new-tab ids (minted by newTab() but
   *  no send yet → no disk record) drop into a fresh in-memory state instead
   *  of disk-load. Singleton stream pipeline — mid-stream switch is handled
   *  by loadConversation() calling stop(). */
  async openTab(id: string) {
    if (!this.openTabs.includes(id)) {
      this.openTabs = [...this.openTabs, id];
    }
    if (this.currentConvoId === id) {
      this.persistTabs();
      return;
    }
    this.telemetry.event("tab.switch", { from: this.currentConvoId, to: id });
    if (this.messages.length > 0 && this.currentConvoId) {
      this.scheduleSave(true);
    }
    // Stash outgoing tab's composer + attachments before any state change.
    this.stashTabUi(this.currentConvoId);
    const inMeta = this.conversations.some((c) => c.id === id);
    if (inMeta) {
      await this.loadConversation(id);
    } else {
      // Fresh in-memory tab (no disk record yet). Mint a TabState with
      // cliSessionId seeded from convoId — first send() finalizes. Don't
      // stop() here: if another tab was streaming, leave it running in the
      // bg. #143: per-tab fields are already null on the fresh TabState;
      // writing them via store setters would clobber cliSessionId.
      this.ensureTab(id, id);
      this.currentConvoId = id;
      this.queue = [];
      this.lastNotice = null;
    }
    // Restore incoming tab's composer + attachments (loadConversation cleared
    // them; we re-fill from cache if the user had a draft mid-typing).
    this.restoreTabUi(id);
    this.assignFocusedPane(id);
    this.persistTabs();
  }

  /** Close a tab. Removes from openTabs; convo stays on disk → still in History.
   *  Active-tab close picks the right neighbor (or left if at end); last-tab
   *  close drops to empty state w/ currentConvoId=null. */
  async closeTab(id: string) {
    const idx = this.openTabs.indexOf(id);
    if (idx === -1) return;
    const wasActive = this.currentConvoId === id;
    this.telemetry.event("tab.close", { convoId: id, wasActive });
    const next = this.openTabs.slice();
    next.splice(idx, 1);
    this.openTabs = next;
    // Drop the closing tab's UI scratch + TabState. The convo itself stays
    // on disk via scheduleSave below; only in-memory streaming state is retired.
    this.pruneTabUi(id);
    this.scrubTabFromPanes(id);
    if (wasActive) {
      // Save unsaved tail of the closing tab before switching/clearing.
      if (this.messages.length > 0 && this.convoCreatedAt) {
        this.scheduleSave(true);
      }
      if (this.streaming) await this.stop();
    }
    this.dropTab(id);
    if (wasActive) {
      if (next.length === 0) {
        this.currentConvoId = null;
        this.currentCliSessionId = null;
        this.convoCreatedAt = null;
        this.convoTitle = null;
        this.queue = [];
        this.lastNotice = null;
      } else {
        // Right-priority: the entry that shifted into idx, else last.
        const neighbor = next[idx] ?? next[next.length - 1];
        const inMeta = this.conversations.some((c) => c.id === neighbor);
        if (inMeta) {
          await this.loadConversation(neighbor);
        } else {
          // #143: ensureTab seeds the fresh tab's cliSessionId to neighbor;
          // don't store-write null afterwards or the setter clobbers it.
          this.ensureTab(neighbor, neighbor);
          this.currentConvoId = neighbor;
          this.queue = [];
          this.lastNotice = null;
        }
        this.restoreTabUi(neighbor);
        this.assignFocusedPane(neighbor);
      }
    }
    this.persistTabs();
  }

  /** Open a fresh empty tab. Mints currentConvoId up-front so the tab can
   *  render before the first send; convoCreatedAt stays null so send() still
   *  flags isFirstTurn=true and the CLI gets --session-id, not --resume. */
  async newTab() {
    // Don't stop the previous tab's stream — newTab leaves background tabs
    // streaming. Save unsaved tail of the previous tab before swapping.
    if (this.messages.length > 0 && this.currentConvoId) {
      this.scheduleSave(true);
    }
    // Snapshot outgoing tab's composer state before we mint the new one.
    this.stashTabUi(this.currentConvoId);
    const id = crypto.randomUUID();
    this.openTabs = [...this.openTabs, id];
    // Fresh TabState — empty messages, no streaming. cliSessionId defaults
    // to the convoId; first send() finalizes if needed.
    this.ensureTab(id, id);
    this.telemetry.event("tab.new", { convoId: id });
    this.currentConvoId = id;
    // #143: per-tab fields default to null/<id> on the freshly minted
    // TabState; writing through the store setters here would clobber
    // cliSessionId back to "" (loses ensureTab's seed value).
    this.queue = [];
    this.lastNotice = null;
    // Fresh tab → empty composer (no cache entry yet).
    this.composerDraft = "";
    this.composerAttachments = [];
    this.assignFocusedPane(id);
    this.persistTabs();
  }

  reorderTabs(fromIdx: number, toIdx: number) {
    if (fromIdx === toIdx) return;
    if (fromIdx < 0 || fromIdx >= this.openTabs.length) return;
    const next = this.openTabs.slice();
    const [moved] = next.splice(fromIdx, 1);
    const clamped = Math.max(0, Math.min(toIdx, next.length));
    next.splice(clamped, 0, moved);
    this.openTabs = next;
    this.persistTabs();
  }

  async cycleTab(direction: 1 | -1) {
    if (this.openTabs.length === 0) return;
    const cur = this.currentConvoId ? this.openTabs.indexOf(this.currentConvoId) : -1;
    const n = this.openTabs.length;
    const nextIdx = ((cur < 0 ? 0 : cur + direction) + n) % n;
    await this.openTab(this.openTabs[nextIdx]);
  }

  async closeOtherTabs(keepId: string) {
    const others = this.openTabs.filter((id) => id !== keepId);
    if (others.length === 0) return;
    // #144: tear down per-tab state for removed tabs so tabs Map +
    // tabDrafts/tabAttachments/tabScroll don't accumulate over long sessions.
    for (const id of others) {
      this.dropTab(id);
      this.pruneTabUi(id);
    }
    this.openTabs = [keepId];
    if (this.currentConvoId !== keepId) {
      await this.loadConversation(keepId);
    }
    this.persistTabs();
  }

  /** Wipe all open tabs and drop into the empty-tabs state. Flushes the
   *  current convo if it has messages so nothing's lost; closes streams. */
  async closeAllTabs() {
    if (this.streaming) await this.stop();
    if (this.messages.length > 0 && this.convoCreatedAt) {
      this.scheduleSave(true);
    }
    // Drop every TabState; the convos persisted to disk above.
    this.tabs = new Map();
    this.openTabs = [];
    this.currentConvoId = null;
    this.currentCliSessionId = null;
    this.convoCreatedAt = null;
    this.convoTitle = null;
    this.queue = [];
    this.lastNotice = null;
    this.persistTabs();
  }

  async closeTabsToRight(anchorId: string) {
    const idx = this.openTabs.indexOf(anchorId);
    if (idx === -1 || idx === this.openTabs.length - 1) return;
    const kept = this.openTabs.slice(0, idx + 1);
    const removed = this.openTabs.slice(idx + 1);
    const removedActive = this.currentConvoId && !kept.includes(this.currentConvoId);
    // #144
    for (const id of removed) {
      this.dropTab(id);
      this.pruneTabUi(id);
    }
    this.openTabs = kept;
    if (removedActive) {
      await this.loadConversation(anchorId);
    }
    this.persistTabs();
  }

  // ── /v0.4 tabs ───────────────────────────────────────────────────────

  async renameConversation(id: string, title: string) {
    const trimmed = title.trim();
    if (!trimmed) return;
    try {
      const convo = await invoke<ConversationRecord>("assistant_load_conversation", { id });
      convo.title = trimmed.slice(0, 120);
      convo.updatedAt = Date.now();
      await invoke("assistant_save_conversation", { convo });
      if (this.currentConvoId === id) this.convoTitle = convo.title;
      await this.refreshConversations();
    } catch (e) {
      this.lastError = `Failed to rename conversation: ${String(e)}`;
    }
  }

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
    }
  }

  async setApiKey(key: string | null) {
    const v = key && key.trim().length > 0 ? key.trim() : null;
    await invoke("assistant_set_api_key", { apiKey: v });
    this.apiKey = v;
    await this.refreshAuth();
  }

  async setUseFullConfig(value: boolean) {
    await invoke("assistant_set_use_full_config", { value });
    this.useFullConfig = value;
  }

  async setMaxBudgetUsd(value: number | null) {
    const v = value !== null && Number.isFinite(value) && value > 0 ? value : null;
    await invoke("assistant_set_max_budget_usd", { value: v });
    this.maxBudgetUsd = v;
  }

  async setAllowRemoteShell(value: boolean) {
    await invoke("assistant_set_allow_remote_shell", { value });
    this.allowRemoteShell = value;
  }

  async setAutoCompactThreshold(value: number | null) {
    const v = value !== null && Number.isFinite(value) && value > 0 && value <= 1 ? value : null;
    await invoke("assistant_set_auto_compact_threshold", { value: v });
    this.autoCompactThreshold = v;
  }

  async setCompactModel(value: "haiku" | "sonnet") {
    await invoke("assistant_set_compact_model", { value });
    this.compactModel = value;
  }

  async send(prompt: string) {
    const trimmed = prompt.trim();
    // Empty prompts are allowed when attachments are staged (paste-and-go).
    // Drop only if BOTH the prompt and attachments are empty.
    if (!trimmed && this.composerAttachments.length === 0) return;
    // Try-handle as a slash command first; if it matched, we're done.
    if (trimmed.startsWith("/") && this.runSlash(trimmed)) return;
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
    tab.messages = [
      ...tab.messages,
      { id: crypto.randomUUID(), role: "user", blocks: [{ type: "text", text: bubbleText }] },
    ];
    const asst: ChatMessage = { id: crypto.randomUUID(), role: "assistant", blocks: [] };
    tab.messages = [...tab.messages, asst];
    tab.streamingMsgId = asst.id;
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
        priorContextSummary: priorSummary,
      });
    } catch (e) {
      tab.onError(String(e));
    }
  }

  /** Stage a binary attachment for the next send. Returns false if the size
   *  cap would be exceeded; the composer surfaces a notice on rejection. */
  addAttachment(att: { mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }): boolean {
    // 20 MiB cumulative cap — mirrors the backend guard so we reject before
    // round-tripping a hopeless payload.
    const CAP = 20 * 1024 * 1024;
    const current = this.composerAttachments.reduce((s, a) => s + a.sizeBytes, 0);
    if (current + att.sizeBytes > CAP) return false;
    this.composerAttachments = [
      ...this.composerAttachments,
      { id: crypto.randomUUID(), ...att },
    ];
    return true;
  }

  removeAttachment(id: string) {
    this.composerAttachments = this.composerAttachments.filter((a) => a.id !== id);
  }

  clearAttachments() {
    this.composerAttachments = [];
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
   *  surprising. Known follow-up: scheduleSave still scoped to active tab,
   *  so a bg-tab turn waits until tab activation for persistence (covered
   *  on close via flushNow). */
  private handleTurnComplete(tab: TabState) {
    this.scheduleSave();
    if (tab === this.activeTab && tab.queue.length > 0 && !tab.streaming) {
      const [next, ...rest] = tab.queue;
      tab.queue = rest;
      queueMicrotask(() => void this.send(next.text));
    }
  }

  /** Stop the active tab's in-flight stream. Pre-clears the tab's streaming
   *  flag synchronously so any late `done` event for this session is
   *  idempotent (the kill propagates the late event AFTER the user may have
   *  already switched tabs). Background tabs keep streaming. */
  async stop() {
    const tab = this.activeTab;
    if (!tab || !tab.streaming) return;
    const sid = tab.cliSessionId;
    tab.streaming = false;
    tab.streamingMsgId = null;
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

  removeQueued(id: string) {
    this.queue = this.queue.filter((q) => q.id !== id);
  }

  /** Compaction Phase B: one-shot summarize of the current CLI session.
   *  Pure read — does NOT mutate `messages`, doesn't archive, doesn't
   *  remint. Phase C wires this into the actual compaction flow; until
   *  then it's reachable only via the `/summarize` debug slash.
   *
   *  Returns the SummarizeResult on success, or null if no active session
   *  or the call fails (error surfaces via `lastError`). The cost/usage
   *  fields let Phase C populate the boundary message pill. */
  async summarizeCurrentSession(focus?: string): Promise<SummarizeResult | null> {
    const sid = this.currentCliSessionId;
    if (!sid) {
      this.lastError = "No active session yet — send a message first.";
      return null;
    }
    const tasksJson = JSON.stringify(
      this.tasks.map((t) => ({ content: t.content, status: t.status })),
    );
    try {
      const res = await invoke<SummarizeResult>("assistant_summarize_session", {
        sessionId: sid,
        focus: focus ?? null,
        tasksJson,
      });
      return res;
    } catch (e) {
      this.lastError = `Summarize failed: ${String(e)}`;
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
  async compactConversation(focus?: string): Promise<boolean> {
    const tab = this.activeTab;
    if (!tab) {
      this.lastError = "No active tab.";
      return false;
    }
    if (tab.streaming) {
      this.lastError = "Wait for the current turn to finish before compacting.";
      return false;
    }
    if (tab.compactingNow) {
      this.lastError = "Compaction already in progress.";
      return false;
    }
    if (tab.messages.length < 4) {
      this.lastError = "Conversation too short to compact (need ≥4 messages).";
      return false;
    }
    const oldSid = tab.cliSessionId;
    if (!oldSid) {
      this.lastError = "No CLI session to compact.";
      return false;
    }

    tab.compactingNow = true;
    this.lastNotice = "Compacting conversation…";

    // S124: pre-stage the boundary message w/ streaming:true BEFORE the
    // summarize call. As progress events land, we patch the same block in
    // place so the user sees the summary fill live.
    const archivedCount = tab.messages.length;
    const boundaryId = crypto.randomUUID();
    const placeholderModel = this.compactModel ?? "haiku";
    const ctxPctBefore = this.ctxPct;
    const ctxWindowAtCompact = this.ctxWindow;
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
      const res = await this.summarizeCurrentSession(focus);
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
        this.lastError = `Remint failed: ${String(e)}`;
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

      this.scheduleSave(true);
      const inTk = res.inputTokens + res.cacheReadTokens + res.cacheCreateTokens;
      this.lastNotice =
        `Compacted ${archivedCount} message(s) · $${res.costUsd.toFixed(4)} · ${inTk.toLocaleString()} in / ${res.outputTokens.toLocaleString()} out · ${res.model}. Next turn seeds the new session with the summary.`;
      return true;
    } finally {
      tab.compactingNow = false;
      if (progressUnlisten) progressUnlisten();
    }
  }

  /** Client-side slash commands. Returns true if input was consumed. */
  private runSlash(input: string): boolean {
    const [cmd, ...rest] = input.slice(1).split(/\s+/);
    const arg = rest.join(" ").trim();
    switch (cmd.toLowerCase()) {
      case "clear":
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
          "Rift MCP: read_file / list_dir / grep (workspace-scoped helpers)" +
          (this.allowRemoteShell ? "; remote_bash (russh exec on the active SSH session)." : ".");
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
          "Slash commands: /new · /history · /model · /retry · /copy · /stop · /tools · /cost · /compact · /summarize · /openincli · /diag · /diag-clear · /help. " +
          "Aliases: /clear → /new. /openincli copies a `claude --resume` command for the standalone CLI. " +
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
      tab.promptHistory = [];
      tab.dockAutoOpenedThisConvo = false;
    }
    this.lastNotice = null;
    this.queue = [];
    this.ui.dockOpen = false;
  }
}

export const assistant = new AssistantStore();
