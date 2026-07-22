// M8 (per docs/design/assistant-svelte-split.md) — the stream pump lifted out
// of `src/lib/state/assistant.svelte.ts` as free fns operating on a TabState
// ref. Per brief: TabState stays a class (the $state cluster + IoC hooks live
// there); the METHOD BODIES move here and the class methods become 1-line
// thunks (`onStream(raw) { onStreamLine(this, raw); }`), so the external
// `tab.onStream(...)` call shape from init()'s listeners is unchanged.
//
// Unlike M3-M7's structural host types, this module imports the TabState type
// directly (type-only — erased at compile time, no runtime cycle): the pump
// touches ~30 fields plus the self-referential IoC hooks, and a shape-copy
// that wide would drift. Bodies ported verbatim (this.* → tab.*) so the
// baked-in invariants survive: #146/#234 (streamingMsgIdx index-replace),
// #147 ($state proxy identity — match thinking blocks by startedAt), #178
// (content-keyed todo ids), #182 (post-done dribble logging), S105 A3 (result
// subtype whitelist), S124 (agent-spawn tracking), the rAF text pacer, and
// the envelope-vs-result usage split driving the ctx pill.

import type { TabState } from "../assistant.svelte";
import type { Block, ChatMessage, StreamEnvelope, ThinkingBlock, ToolBlock } from "./types";
import { ctxWindowForModelId, flattenToolResult, previewToolInput } from "./helpers";
import { checkMcpInitHealth } from "./healthAlerts";
import { cliCommands } from "./cliCommands.svelte";
import { browserDock } from "../browserDock.svelte";
import { github } from "../github.svelte";

// S124: agentSpawns appends per Task/Agent/Skill and is never reset within a
// conversation (the dock shows the running history), so a long session grew it
// unboundedly + made the per-frame .some()/.findIndex() scans O(n). Cap to the
// most-recent MAX_SPAWNS so the dock keeps recent history without unbounded growth.
const MAX_SPAWNS = 200;
function capSpawns(list: TabState["agentSpawns"]): TabState["agentSpawns"] {
  return list.length > MAX_SPAWNS ? list.slice(list.length - MAX_SPAWNS) : list;
}

// DesignSync writes land in the cloud claude.ai/design project — pop the
// browser dock to it the first time a sync mutates this tab, so the result is
// visible without the user hunting for it. Once-per-tab (WeakSet) so repeat
// syncs in one conversation don't keep stealing the dock.
const DESIGN_DOCK_OPENED = new WeakSet<TabState>();
const DESIGN_WRITE_METHODS = new Set(["create_project", "write_files", "finalize_plan"]);
const SUPPRESSED_TOOL_NAMES = new Set(["ToolSearch", "TaskList", "TaskGet", "TaskStop", "TaskOutput"]);

// Whitelist (S105 A3): known CLI result-error subtypes surface as user-visible
// errors w/ a plain-English message; anything else logs but doesn't false-alarm.
// Pre-emptive guard for CLI subtypes we haven't seen yet.
const RESULT_ERROR_MESSAGES: Record<string, string> = {
  error_max_turns: "The run hit its maximum number of turns and stopped.",
  error_during_execution: "The run stopped on an execution error.",
  error_max_thinking_tokens: "The run hit its thinking-token budget and stopped.",
  error_max_budget_usd:
    "The run hit your per-turn spend cap and stopped. Raise or remove it in Settings → CLI session.",
  model_context_window_exceeded:
    "The context window filled up and couldn't be compacted further. Start a fresh chat to keep going.",
};

/** Gap under which a CLI-initiated continuation merges into the previous
 *  assistant bubble instead of opening a new one (see maybeBeginContinuation). */
const CONTINUATION_MERGE_MS = 8000;

/** Called at the start of every send(). Clears per-turn pacer / thinking
 *  / dedupe state and flips streaming on. */
export function beginTurn(tab: TabState) {
  tab.lastError = null;
  tab.promptSuggestion = null;
  tab.seenToolUseIds.clear();
  tab.deltaCount = 0;
  tab.envelopeTextBuffer = "";
  tab.rawLineLog = [];
  if (tab.drainHandle !== null) {
    cancelAnimationFrame(tab.drainHandle);
    tab.drainHandle = null;
  }
  tab.pendingText = "";
  tab.thinkingByIndex.clear();
  tab.toolInputByIndex.clear();
  tab.activeThinkingIndex = null;
  tab.lastStreamEventAt = null;
  tab.liveOutputTokens = 0;
  tab.committedOutputTokens = 0;
  tab.compactingTurn = false;
  tab.liveOutputChars = 0;
  tab.activity = { currentLabel: null, turnStartedAt: Date.now() };
  // RR10: re-anchor the task-id counter to the live tasks length each turn.
  // tasks[] persists across turns (the dock), so leaving the counter alone let
  // a new turn's TaskCreate mint id "1" — colliding with a prior turn's task —
  // and a TaskUpdate{taskId:"1"} would then patch the wrong (older) task.
  tab.taskCreateCount = tab.tasks.length;
  tab.planBlockId = null;
  // Belt-and-suspenders: the three terminal paths (onStreamDone/onStreamError/
  // stop) already clear the ask_user pairing FIFOs, but a late `assistant://
  // ask-user` IPC event can race PAST a stop() and push a stale requestId onto
  // the just-stopped tab. Left there, beginTurn's FIFO would pair that dead
  // requestId with the NEXT turn's first ask_user toolUseId — binding a live
  // Allow/Deny chip to an already-resolved backend oneshot. Clear here too so a
  // new turn always starts with empty pairing state (nothing pushed yet).
  tab.unboundAskUserRequestIds = [];
  tab.unboundAskUserToolUseIds = [];
  if (tab.askUserBindings.size > 0) tab.askUserBindings = new Map();
  tab.streaming = true;
}

/** Count Agent tool launches that ran in the background — their tool_result
 *  is the CLI's "Async agent launched successfully…" stub, and the real
 *  report arrives later as a continuation turn. */
function countBgAgentLaunches(m: ChatMessage): number {
  let n = 0;
  for (const b of m.blocks) {
    if (b.type === "tool" && b.name === "Agent" && typeof b.result === "string" && b.result.startsWith("Async agent launched")) n++;
  }
  return n;
}

/** CLI-initiated continuation turn: a background agent/task finished after the
 *  previous turn's DONE and the CLI re-invoked the model on its own (turn.rs
 *  idle-drain forwards those frames). No send() scaffolded streaming state, so
 *  scaffold on the turn-opening `system/init` (so the partial-message deltas
 *  that follow paint live, not all-at-once at done) or on a contentful
 *  top-level assistant frame (init dropped/reordered). Anything weaker (task
 *  bookkeeping, usage-only envelopes) never scaffolds. Nested sub-agent frames
 *  can't reach this — onStreamLine's parent_tool_use_id branch returns first.
 *  The backend idle-drain is the real gate: an idle tab only sees these frames
 *  when a genuine continuation turn is running. */
function maybeBeginContinuation(tab: TabState, env: StreamEnvelope): boolean {
  if (tab.streaming) return false;
  const isInit = env.type === "system" && (env as { subtype?: string }).subtype === "init";
  const isContentfulAssistant =
    env.type === "assistant" && Array.isArray(env.message?.content) && env.message.content.length > 0;
  if (!isInit && !isContentfulAssistant) return false;
  beginTurn(tab);
  // A continuation that begins moments after the previous DONE reads as the
  // SAME turn to the user (the classic symptom: "Worked for <1s" done-header,
  // then a second block keeps thinking below it). Reopen the previous
  // assistant bubble and stream into it; the result handler sums duration and
  // cost. A genuinely late follow-up (background agent minutes later) still
  // gets its own bubble.
  const lastIdx = tab.messages.length - 1;
  let last = lastIdx >= 0 ? tab.messages[lastIdx] : null;
  // A continuation IS a background agent reporting back — retire one pending
  // launch so the previous footer stops claiming "still working" for it.
  if (last && last.role === "assistant" && (last.bgAgentsPending ?? 0) > 0) {
    last = {
      ...last,
      bgAgentsPending: (last.bgAgentsPending ?? 0) - 1,
      bgAgentsResolved: (last.bgAgentsResolved ?? 0) + 1,
    };
    tab.messages[lastIdx] = last;
  }
  const doneAgoMs = tab.lastTurnDoneAt !== null ? Date.now() - tab.lastTurnDoneAt : Infinity;
  if (import.meta.env.DEV) {
    console.debug(`[assistant] continuation turn: ${Math.round(doneAgoMs / 100) / 10}s after done, last=${last?.role ?? "none"} → ${last?.role === "assistant" && doneAgoMs < CONTINUATION_MERGE_MS ? "merge" : "new bubble"}`);
  }
  if (last && last.role === "assistant" && doneAgoMs < CONTINUATION_MERGE_MS) {
    tab.streamingMsgId = last.id;
    tab.streamingMsgIdx = lastIdx;
    // Paragraph-break a trailing text block so the continuation's text doesn't
    // fuse onto the previous sentence (appendText concatenates into it).
    const tail = last.blocks[last.blocks.length - 1];
    if (tail && tail.type === "text" && tail.text.trim().length > 0 && !tail.text.endsWith("\n\n")) {
      const blocks = last.blocks.slice();
      blocks[blocks.length - 1] = { ...tail, text: tail.text + "\n\n" };
      tab.messages[lastIdx] = { ...last, blocks };
    }
    tab.activity = { currentLabel: "Continuing…", turnStartedAt: Date.now() };
    return true;
  }
  const asst: ChatMessage = { id: crypto.randomUUID(), role: "assistant", blocks: [], ts: Date.now() };
  tab.messages = [...tab.messages, asst];
  tab.streamingMsgId = asst.id;
  tab.streamingMsgIdx = tab.messages.length - 1;
  tab.activity = { currentLabel: "Background agent finished — responding", turnStartedAt: Date.now() };
  return true;
}

// ── streaming pipeline ────────────────────────────────────────────────────

function mutateStreaming(tab: TabState, fn: (m: ChatMessage) => ChatMessage) {
  if (!tab.streamingMsgId) return;
  const idx = tab.streamingMsgIdx;
  if (idx !== null && idx >= 0 && idx < tab.messages.length) {
    const m = tab.messages[idx];
    if (m && m.id === tab.streamingMsgId) {
      tab.messages[idx] = fn(m);
      return;
    }
  }
  tab.messages = tab.messages.map((m) => (m.id === tab.streamingMsgId ? fn(m) : m));
}

/** Synthesize a visible system-role boundary message for a CLI `compact_boundary`
 *  event. Inserted just before the in-flight assistant bubble (keeping
 *  streamingMsgIdx valid) so it lands at the point compaction actually fired. */
function appendCliCompaction(tab: TabState, env: StreamEnvelope) {
  const meta =
    (env as { compact_metadata?: { trigger?: string; pre_tokens?: number; post_tokens?: number } })
      .compact_metadata ?? {};
  const model = tab.lastModelId ?? "";
  const w = ctxWindowForModelId(model, tab.planCap?.());
  const pre = typeof meta.pre_tokens === "number" ? meta.pre_tokens : undefined;
  const post = typeof meta.post_tokens === "number" ? meta.post_tokens : undefined;
  const boundary: Block = {
    type: "boundary",
    summary: "",
    at: Date.now(),
    archivedCount: 0,
    costUsd: 0,
    summaryModel: model,
    streaming: false,
    source: "cli",
    trigger: meta.trigger === "manual" ? "manual" : "auto",
    preTokens: pre,
    postTokens: post,
    ctxPctBefore: pre !== undefined && w > 0 ? (pre / w) * 100 : undefined,
    ctxPctEstAfter: post !== undefined && w > 0 ? (post / w) * 100 : undefined,
  };
  const msg: ChatMessage = { id: crypto.randomUUID(), role: "system", blocks: [boundary] };
  const idx = tab.streamingMsgIdx;
  if (idx !== null && idx >= 0 && idx < tab.messages.length && tab.messages[idx]?.id === tab.streamingMsgId) {
    tab.messages = [...tab.messages.slice(0, idx), msg, ...tab.messages.slice(idx)];
    tab.streamingMsgIdx = idx + 1;
  } else {
    tab.messages = [...tab.messages, msg];
  }
}

function beginThinking(tab: TabState, index: number) {
  if (tab.thinkingByIndex.has(index)) return;
  const startedAt = Date.now();
  // RR9: keep ALL the per-thinking side effects atomic with the block push.
  // mutateStreaming early-returns (no-op) when streamingMsgId is null — e.g. a
  // late content_block_start landing after stop() cleared it. Previously the
  // map-set lived inside the callback but activeThinkingIndex / "Thinking…" /
  // thinkingCount ran unconditionally outside it, so on the no-op path the
  // spinner label stuck on (nothing clears it until the next beginTurn) and
  // thinkingCount inflated. `applied` tracks whether the push actually ran.
  let applied = false;
  mutateStreaming(tab, (m) => {
    const blocks = m.blocks.slice();
    tab.thinkingByIndex.set(index, { blockOffset: blocks.length, startedAt });
    blocks.push({
      type: "thinking",
      text: "",
      hasSignature: false,
      startedAt,
      durationMs: null,
      status: "active",
    });
    applied = true;
    return { ...m, blocks };
  });
  if (!applied) return;
  tab.activeThinkingIndex = index;
  tab.activity = { ...tab.activity, currentLabel: "Thinking…" };
  if (tab.currentTurnRecord) tab.currentTurnRecord.thinkingCount += 1;
}

function mutateThinking(tab: TabState, index: number, fn: (b: ThinkingBlock) => ThinkingBlock) {
  const entry = tab.thinkingByIndex.get(index);
  if (!entry) return;
  mutateStreaming(tab, (m) => {
    const blocks = m.blocks.slice();
    const target = blocks[entry.blockOffset];
    if (target && target.type === "thinking") {
      blocks[entry.blockOffset] = fn(target);
    }
    return { ...m, blocks };
  });
}

function appendThinkingText(tab: TabState, index: number, chunk: string) {
  if (!chunk) return;
  mutateThinking(tab, index, (b) => ({ ...b, text: b.text + chunk }));
  tab.liveOutputChars += chunk.length;
  refreshLiveTokens(tab);
}

function markThinkingSignature(tab: TabState, index: number) {
  mutateThinking(tab, index, (b) => (b.hasSignature ? b : { ...b, hasSignature: true }));
}

/** Stamp the turn's first visible-output moment ONCE, snapshotting how much
 *  thinking had accrued up to that point. healthAlerts' deadWait subtracts this
 *  pre-paint snapshot (not the full-turn cumulative thinkingTotalMs) so a long
 *  post-paint thinking round can't drive deadWait negative and hide a real
 *  slow-start. No-op after the first call. */
function markFirstPaint(tab: TabState) {
  const rec = tab.currentTurnRecord;
  if (!rec || rec.firstPaintAt != null) return;
  rec.firstPaintAt = Date.now();
  rec.thinkingMsBeforeFirstPaint = rec.thinkingTotalMs;
}

function endThinking(tab: TabState, index: number) {
  const entry = tab.thinkingByIndex.get(index);
  if (!entry) return;
  const durationMs = Date.now() - entry.startedAt;
  if (tab.currentTurnRecord) {
    tab.currentTurnRecord.thinkingTotalMs += durationMs;
    // Capture per-block detail. `charCount` stays 0 in -p mode (encrypted)
    // but is wired in case a future API version emits plaintext deltas;
    // `hasSignature` is the truthier "real thinking happened" signal today.
    let charCount = 0;
    let hasSignature = false;
    const msg = tab.streamingMsgId ? tab.messages.find((m) => m.id === tab.streamingMsgId) : null;
    if (msg) {
      const block = msg.blocks[entry.blockOffset];
      if (block && block.type === "thinking") {
        charCount = block.text.length;
        hasSignature = block.hasSignature;
      }
    }
    tab.currentTurnRecord.thinkingBlocks.push({
      startedAt: entry.startedAt,
      durationMs,
      charCount,
      hasSignature,
    });
  }
  mutateThinking(tab, index, (b) => ({ ...b, status: "done", durationMs }));
  if (tab.activeThinkingIndex === index) {
    tab.activeThinkingIndex = null;
    if (tab.activity.currentLabel === "Thinking…") {
      tab.activity = { ...tab.activity, currentLabel: null };
    }
  }
  // Drop the entry so the CLI's agentic loop — which reuses `index=0` for
  // each new thinking block after a tool round-trip — can re-`beginThinking`
  // cleanly. Without this, `thinkingCount` stays at 1 and `thinkingBlocks`
  // double-counts the same block w/ ever-growing cumulative durations.
  tab.thinkingByIndex.delete(index);
}

function ensureThinkingFromEnvelope(tab: TabState, block: { thinking?: string; signature?: string }) {
  if (!tab.streamingMsgId) return;
  const msg = tab.messages.find((m) => m.id === tab.streamingMsgId);
  if (!msg) return;
  // RR9: findLast, not find. In agentic loops endThinking deletes the
  // thinkingByIndex entry so index 0 is reused each tool round → a message
  // accumulates one ThinkingBlock per round. The envelope for round N carries
  // round N's text/signature; find() returns block 0 (round 1, already done) so
  // in encrypted-thinking mode (no thinking_delta — envelope is the only text
  // source) every block after the first stayed empty. The most recently pushed
  // thinking block is always the current round's.
  const existing = msg.blocks.findLast((b) => b.type === "thinking") as ThinkingBlock | undefined;
  const envText = typeof block.thinking === "string" ? block.thinking : "";
  const envSig = !!block.signature && block.signature.length > 0;
  if (existing) {
    if (envText.length > existing.text.length || (envSig && !existing.hasSignature)) {
      // #147: $state proxies aren't referentially equal across read sites,
      // so `b === existing` was always false → every call appended a new
      // block. Match by stable startedAt instead.
      const key = existing.startedAt;
      mutateStreaming(tab, (m) => ({
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
  mutateStreaming(tab, (m) => ({
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

function appendText(tab: TabState, chunk: string) {
  if (!chunk) return;
  mutateStreaming(tab, (m) => {
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

function enqueueText(tab: TabState, chunk: string) {
  if (!chunk) return;
  tab.pendingText += chunk;
  tab.liveOutputChars += chunk.length;
  refreshLiveTokens(tab);
  // #100: rAF never fires on a hidden page — the pacer would park the whole
  // backlog until refocus, then flood it through the render path in one burst.
  // Pacing exists for visible reading rhythm; invisible text just lands.
  if (typeof document !== "undefined" && document.hidden) {
    flushPendingText(tab);
    return;
  }
  if (tab.drainHandle === null) {
    tab.lastDrainAt = performance.now();
    tab.drainHandle = requestAnimationFrame(tab.drainTick);
  }
}

/** Recompute the live output-token readout: exact totals banked from completed
 *  messages + a char/4 estimate for the in-flight message. The CLI gives no
 *  mid-stream usage, so the estimate is what makes the counter climb (CC-style)
 *  until each message's real count snaps in via `recordTurnUsage`. */
function refreshLiveTokens(tab: TabState) {
  // Monotonic per turn: the chars/4 estimate can overshoot the real usage that
  // later snaps in, and a live counter that ticks BACKWARD reads as broken
  // (caught on the 2026-07-15 session recording — 429→409, 442→434). Hold the
  // high-water mark; beginTurn's reset to 0 re-arms it each turn.
  tab.liveOutputTokens = Math.max(
    tab.liveOutputTokens,
    tab.committedOutputTokens + Math.round(tab.liveOutputChars / 4),
  );
}

/** Body of the per-tab rAF pacer. TabState keeps a stable bound arrow
 *  (`drainTick = () => drainTick(this)`) so rAF re-arm targets one identity. */
export function drainTick(tab: TabState) {
  if (tab.pendingText.length === 0) {
    tab.drainHandle = null;
    return;
  }
  const now = performance.now();
  const dt = Math.min(now - tab.lastDrainAt, 100);
  tab.lastDrainAt = now;
  // Drain the char buffer into the message faster than before (≈0.1s window,
  // 360 c/s floor, was 0.25s/180) so this char-pacer adds less latency on top of
  // Markdown's word-reveal — the two used to compound into a visible lag behind
  // generation. The word-reveal stays the visible cadence authority.
  const rate = Math.max(360, tab.pendingText.length / 0.1);
  const n = Math.min(tab.pendingText.length, Math.max(1, Math.round((rate * dt) / 1000)));
  const chunk = tab.pendingText.slice(0, n);
  tab.pendingText = tab.pendingText.slice(n);
  appendText(tab, chunk);
  tab.drainHandle = requestAnimationFrame(tab.drainTick);
}

export function flushPendingText(tab: TabState) {
  if (tab.drainHandle !== null) {
    cancelAnimationFrame(tab.drainHandle);
    tab.drainHandle = null;
  }
  if (tab.pendingText.length > 0) {
    appendText(tab, tab.pendingText);
    tab.pendingText = "";
  }
}

/** Append a mid-turn steer marker inside the streaming assistant bubble at the
 *  current stream position (send-while-streaming → assistant_steer). Flushes
 *  the pacer first so the marker lands after everything already generated —
 *  and appendText's tail-merge then starts a FRESH text block after it, so
 *  post-steer prose never fuses into pre-steer prose. Returns the block id
 *  (for removal if delivery fails) or null when nothing is streaming. */
export function appendSteerBlock(tab: TabState, text: string, images: number, files: number): string | null {
  if (!tab.streamingMsgId) return null;
  flushPendingText(tab);
  const id = crypto.randomUUID();
  let applied = false;
  mutateStreaming(tab, (m) => {
    applied = true;
    return {
      ...m,
      blocks: [
        ...m.blocks,
        {
          type: "steer",
          id,
          text,
          at: Date.now(),
          ...(images > 0 ? { images } : {}),
          ...(files > 0 ? { files } : {}),
        },
      ],
    };
  });
  return applied ? id : null;
}

/** Remove a steer marker whose backend delivery failed (the message falls back
 *  to the outbound queue). Scans all messages — the turn may have ended (and
 *  streamingMsgId cleared) between optimistic render and the failure ack. */
export function removeSteerBlock(tab: TabState, id: string) {
  for (let i = 0; i < tab.messages.length; i++) {
    const m = tab.messages[i];
    if (m.blocks.some((b) => b.type === "steer" && b.id === id)) {
      tab.messages[i] = { ...m, blocks: m.blocks.filter((b) => !(b.type === "steer" && b.id === id)) };
      return;
    }
  }
}

function applyTodoWrite(tab: TabState, input: Record<string, unknown> | undefined): boolean {
  const raw = (input?.todos ?? []) as Array<{ content?: string; status?: string }>;
  // #178: content-keyed ids so a reorder/insert in the model's TodoWrite
  // doesn't force every downstream {#each} to destroy + remount. Existing
  // ids are reused when content matches; new content gets a fresh id.
  const byContent = new Map(tab.tasks.map((t) => [t.content, t.id]));
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
  tab.tasks = next;
  if (next.length > 0 && !tab.dockAutoOpenedThisConvo) {
    tab.dockAutoOpenedThisConvo = true;
    return true;
  }
  return false;
}

// Newer Claude CLI task API: TaskCreate adds one task at a time (subject +
// description + activeForm); TaskUpdate flips status by 1-based creation index
// (taskId "1".."N"). Both feed the same dock Plan card as TodoWrite.
function applyTaskCreate(tab: TabState, input: Record<string, unknown> | undefined): boolean {
  const subject = typeof input?.subject === "string" ? (input.subject as string) : null;
  if (!subject) return false;
  tab.taskCreateCount += 1;
  const id = String(tab.taskCreateCount);
  tab.tasks = [...tab.tasks, { id, content: subject, status: "pending" }];
  if (!tab.dockAutoOpenedThisConvo) {
    tab.dockAutoOpenedThisConvo = true;
    return true;
  }
  return false;
}

function applyTaskUpdate(tab: TabState, input: Record<string, unknown> | undefined): void {
  const taskId = input?.taskId != null ? String(input.taskId) : null;
  if (!taskId) return;
  const raw = input?.status;
  const status = (raw === "in_progress" || raw === "completed" ? raw : "pending") as
    | "pending"
    | "in_progress"
    | "completed";
  tab.tasks = tab.tasks.map((t) => (t.id === taskId ? { ...t, status } : t));
}

// Ensure exactly ONE inline plan block exists for this turn AND keep its
// `input.todos` synced to the live `tab.tasks` aggregate. The plan card
// (StreamPlan via StreamTurn) prefers the live aggregate, but `tab.tasks` is
// runtime-only (reset to [] on reload — persistence.ts), so a block carrying
// `input:{}` rendered an empty/invisible card on reopen. Mirroring the tasks
// into the block's own `input.todos` makes it self-describing → the persisted
// card survives reload. Called after every apply*(), so `tab.tasks` is current.
function ensurePlanBlock(tab: TabState) {
  const todos = tab.tasks.map((t) => ({ content: t.content, status: t.status }));
  if (tab.planBlockId) {
    const pid = tab.planBlockId;
    mutateStreaming(tab, (m) => ({
      ...m,
      blocks: m.blocks.map((b) => (b.type === "tool" && b.id === pid ? { ...b, input: { todos } } : b)),
    }));
    return;
  }
  const id = `plan-${tab.streamingMsgId ?? "x"}-${tab.tasks.length}`;
  tab.planBlockId = id;
  mutateStreaming(tab, (m) => ({
    ...m,
    blocks: [
      ...m.blocks,
      { type: "tool", id, name: "TodoWrite", input: { todos }, result: null, isError: false, status: "done", startedAt: Date.now() },
    ],
  }));
}

// ── Live tool-block forming (S127) ──────────────────────────────────────────
// With --include-partial-messages the CLI streams each tool_use as it FORMS:
// content_block_start (id+name, empty input) → input_json_delta chunks (the
// input JSON typing itself out) → content_block_stop. All three were ignored —
// a tool block only appeared once the complete assistant envelope landed, so a
// long Write/Edit (10s+ of args streaming) showed NOTHING happening. Now the
// pending block appears the moment the model commits to the call, its caption
// fills live as recognizable fields complete, and the envelope merely finalizes
// input (upsert by id — no duplicate).
//
// Names with side-effectful append paths (plan aggregation, agent spawns,
// ask_user binding) and suppressed names keep their envelope-only path —
// forming them early would run those side effects on empty input.
const FORMING_SKIP = new Set([
  ...SUPPRESSED_TOOL_NAMES,
  "TodoWrite", "TaskCreate", "TaskUpdate", "Task", "Agent", "mcp__rift__ask_user",
]);

// Caption-worthy string fields worth extracting from a *partial* input JSON —
// mirrors streamModel caption()/pathOf() sources. Deliberately excludes
// old_string/new_string/content (huge, diff-only). Inside a JSON string every
// `"` is escaped as `\"`, so a bare `"key"` match can only be a real key.
const FORM_FIELD_RE = /"(file_path|command|pattern|path|url|query|prompt|description|notebook_path|skill|name|operation)"\s*:\s*"((?:[^"\\]|\\.)*)"/g;
// Trailing still-open string value — lets a shell command "type" live.
const FORM_TAIL_RE = /"(command|file_path|pattern|url|query)"\s*:\s*"((?:[^"\\]|\\.)*)$/;
const FORM_SCAN_CAP = 6144; // caption fields stream first; never scan a huge blob

function unescapeJsonString(raw: string): string | null {
  try { return JSON.parse(`"${raw}"`) as string; } catch { return null; }
}

/** Bounded extraction of completed (plus one trailing incomplete) caption
 *  fields from partial input JSON. Never throws; {} until something usable. */
function extractFormingFields(json: string): Record<string, string> {
  const win = json.length > FORM_SCAN_CAP ? json.slice(0, FORM_SCAN_CAP) : json;
  const out: Record<string, string> = {};
  for (const m of win.matchAll(FORM_FIELD_RE)) {
    if (out[m[1]] === undefined) {
      const v = unescapeJsonString(m[2]);
      if (v !== null) out[m[1]] = v;
    }
  }
  const tail = win.match(FORM_TAIL_RE);
  if (tail && out[tail[1]] === undefined) {
    const v = unescapeJsonString(tail[2]);
    if (v !== null && v.length > 0) out[tail[1]] = v;
  }
  return out;
}

function beginToolForming(
  tab: TabState,
  index: number,
  cb: { id: string; name: string; input?: Record<string, unknown> },
) {
  if (FORMING_SKIP.has(cb.name)) return;
  if (tab.seenToolUseIds.has(cb.id)) return;
  let applied = false;
  mutateStreaming(tab, (m) => {
    applied = true;
    return {
      ...m,
      blocks: [
        ...m.blocks,
        {
          type: "tool",
          id: cb.id,
          name: cb.name,
          input: cb.input ?? {},
          result: null,
          isError: false,
          status: "pending",
          startedAt: Date.now(),
          inputPartial: true,
        },
      ],
    };
  });
  // Late frame after stop() cleared streamingMsgId → nothing was pushed; keep
  // all bookkeeping consistent with that no-op (mirrors beginThinking RR9).
  if (!applied) return;
  tab.seenToolUseIds.add(cb.id);
  tab.toolInputByIndex.set(index, { id: cb.id, name: cb.name, json: "", extracted: "" });
  if (tab.currentTurnRecord) {
    tab.currentTurnRecord.toolUses.push({
      name: cb.name,
      id: cb.id,
      startedAt: Date.now(),
      completedAt: null,
      durationMs: null,
      isError: null,
      inputPreview: null,
    });
  }
  tab.activity = {
    ...tab.activity,
    currentLabel: tab.shortToolLabel ? tab.shortToolLabel(cb.name, cb.input) : cb.name,
  };
}

function appendToolInputDelta(tab: TabState, index: number, partialJson: string) {
  const entry = tab.toolInputByIndex.get(index);
  if (!entry) return;
  entry.json += partialJson;
  // Tool args are real output tokens — keep the live meter honest during a
  // big Write instead of freezing until the envelope lands.
  tab.liveOutputChars += partialJson.length;
  refreshLiveTokens(tab);
  // Past the scan cap the extraction window is frozen — fields can't change,
  // so skip the per-delta rescan (big Write inputs stream thousands of deltas).
  if (entry.json.length - partialJson.length >= FORM_SCAN_CAP) return;
  const fields = extractFormingFields(entry.json);
  const snap = JSON.stringify(fields);
  if (snap === entry.extracted) return;
  entry.extracted = snap;
  const id = entry.id;
  mutateStreaming(tab, (m) => ({
    ...m,
    blocks: m.blocks.map((b) =>
      b.type === "tool" && b.id === id && b.inputPartial ? { ...b, input: fields } : b,
    ),
  }));
}

/** content_block_stop for a forming tool: the accumulated JSON is complete —
 *  parse + finalize now instead of waiting for the assistant envelope. Parse
 *  failure just leaves the block partial (the envelope finalizes it). */
function endToolForming(tab: TabState, index: number) {
  const entry = tab.toolInputByIndex.get(index);
  if (!entry) return;
  tab.toolInputByIndex.delete(index);
  let full: Record<string, unknown> | null = entry.json === "" ? {} : null;
  if (full === null) {
    try {
      const p: unknown = JSON.parse(entry.json);
      if (p && typeof p === "object" && !Array.isArray(p)) full = p as Record<string, unknown>;
    } catch { /* truncated/odd JSON — envelope will finalize */ }
  }
  if (full === null) return;
  const id = entry.id;
  const input = full;
  mutateStreaming(tab, (m) => ({
    ...m,
    blocks: m.blocks.map((b) =>
      b.type === "tool" && b.id === id && b.inputPartial
        ? { ...b, input, inputPartial: undefined }
        : b,
    ),
  }));
  if (tab.currentTurnRecord) {
    const rec = tab.currentTurnRecord.toolUses.find((t) => t.id === id);
    if (rec) rec.inputPreview = previewToolInput(entry.name, input);
  }
  tab.activity = {
    ...tab.activity,
    currentLabel: tab.shortToolLabel ? tab.shortToolLabel(entry.name, input) : entry.name,
  };
}

/** The assistant envelope re-delivers a tool_use we already live-formed —
 *  upsert its authoritative complete input onto the existing block. Applied
 *  UNCONDITIONALLY (#100 self-heal): a block whose forming deltas were lost
 *  can finalize empty via endToolForming, and the envelope is the only frame
 *  that can still supply the real input. */
function finalizeFormedTool(tab: TabState, block: { id: string; name: string; input?: Record<string, unknown> }) {
  const input = block.input ?? {};
  mutateStreaming(tab, (m) => ({
    ...m,
    blocks: m.blocks.map((b) =>
      b.type === "tool" && b.id === block.id && b.status === "pending"
        ? { ...b, name: block.name, input, inputPartial: undefined }
        : b,
    ),
  }));
  if (tab.currentTurnRecord) {
    const rec = tab.currentTurnRecord.toolUses.find((t) => t.id === block.id);
    if (rec && rec.inputPreview === null) rec.inputPreview = previewToolInput(block.name, block.input);
  }
}

function appendToolUse(tab: TabState, block: { id: string; name: string; input?: Record<string, unknown> }) {
  // The dock's "read by assistant" chip. MUST sit above the formed-tool early
  // return below — live-streamed tools are pre-formed from stream_events, so
  // for them this envelope call is the only guaranteed once-per-tool hook.
  if (block.name === "mcp__rift__read_browser_page" || block.name === "mcp__rift__read_browser_console") {
    browserDock.noteAssistantRead(block.name.endsWith("console") ? "console" : "page");
  }
  if (tab.seenToolUseIds.has(block.id)) {
    // Already live-formed from stream_events — the envelope finalizes input.
    finalizeFormedTool(tab, block);
    return;
  }
  tab.seenToolUseIds.add(block.id);
  if (tab.currentTurnRecord) {
    tab.currentTurnRecord.toolUses.push({
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
    const opensDock = applyTodoWrite(tab, block.input);
    ensurePlanBlock(tab);
    tab.onTodoApplied?.(tab, opensDock);
    return;
  }
  if (block.name === "TaskCreate") {
    const opensDock = applyTaskCreate(tab, block.input);
    ensurePlanBlock(tab);
    tab.onTodoApplied?.(tab, opensDock);
    return;
  }
  if (block.name === "TaskUpdate") {
    applyTaskUpdate(tab, block.input);
    ensurePlanBlock(tab);
    tab.onTodoApplied?.(tab, false);
    return;
  }
  if (block.name === "Task" || block.name === "Agent") {
    const subagentType = String(block.input?.subagent_type ?? "fork");
    const description = String(block.input?.description ?? "(no description)");
    tab.agentSpawns = capSpawns([
      ...tab.agentSpawns,
      { id: block.id, subagentType, description, startedAt: Date.now(), completedAt: null, isError: false, blocks: [] },
    ]);
    // NO early return: fall through so the Task ALSO appends a normal bubble tool
    // block. That makes the delegation a first-class INLINE card in the transcript
    // — StreamAgent live (streamModel maps "Task"/"Agent" → kind "agent"), AgentCard
    // once persisted — matched to its spawn by id. The nested sub-agent frames still
    // route to agentSpawns[i].blocks (applySubAgentFrame), never the main bubble.
  }
  // Suppress: ToolSearch (internal schema fetch) + the read/control Task tools
  // (TaskList/TaskGet/TaskStop/TaskOutput — newer CLI). TaskCreate/TaskUpdate
  // drive the plan card above; these four are informational and would otherwise
  // render as noisy generic tool chips in the main bubble.
  if (SUPPRESSED_TOOL_NAMES.has(block.name)) return;
  if (
    block.name === "DesignSync" &&
    !DESIGN_DOCK_OPENED.has(tab) &&
    DESIGN_WRITE_METHODS.has(String(block.input?.method ?? ""))
  ) {
    DESIGN_DOCK_OPENED.add(tab);
    browserDock.openUrl("https://claude.ai/design");
  }
  // GitHub chip: a remote-mutating tool ran this turn — mark for a forced
  // status refresh at turn end (flushed in the `result` envelope).
  if (/(?:^|_)(git_push|git_pull|git_commit|gh_pr_create)$/.test(block.name)) github.noteRemoteMutation();
  tab.activity = {
    ...tab.activity,
    currentLabel: tab.shortToolLabel ? tab.shortToolLabel(block.name, block.input) : block.name,
  };
  mutateStreaming(tab, (m) => ({
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
    tab.unboundAskUserToolUseIds.push(block.id);
    tryBindAskUser(tab);
    // The whole turn (and the CLI subprocess) blocks on this answer — nudge
    // via the store hook if it's still unanswered after the grace period.
    const rec = tab.currentTurnRecord;
    setTimeout(() => {
      const tu = rec?.toolUses.find((t) => t.id === block.id);
      if (tab.streaming && tab.currentTurnRecord === rec && tu && tu.completedAt == null) {
        tab.onAskUserStale?.(tab);
      }
    }, ASK_USER_NUDGE_MS);
  }
}

const ASK_USER_NUDGE_MS = 60_000;

/** #98.3: append a visible marker for a content-block type this build doesn't
 *  know. Once per (message, type) — repeats of the same unknown type in one
 *  bubble don't stack. Always leaves a console breadcrumb (matches the
 *  unrecognized-stream_event policy above). */
function appendUnknownBlock(tab: TabState, block: unknown) {
  const blockType = String((block as { type?: unknown })?.type ?? "?");
  console.warn("[assistant] unknown content block type from CLI — rendering a marker", blockType, block);
  mutateStreaming(tab, (m) =>
    m.blocks.some((b) => b.type === "unknown" && b.blockType === blockType)
      ? m
      : { ...m, blocks: [...m.blocks, { type: "unknown", blockType, at: Date.now() }] },
  );
}

/** Drain the two ask_user FIFOs as long as both have entries. Each pair
 *  binds a toolUseId to a requestId in `askUserBindings`, making the chip
 *  in MessageBubble able to invoke the answer-submit command. */
export function tryBindAskUser(tab: TabState) {
  while (
    tab.unboundAskUserToolUseIds.length > 0 &&
    tab.unboundAskUserRequestIds.length > 0
  ) {
    const toolUseId = tab.unboundAskUserToolUseIds.shift()!;
    const requestId = tab.unboundAskUserRequestIds.shift()!;
    const next = new Map(tab.askUserBindings);
    next.set(toolUseId, requestId);
    tab.askUserBindings = next;
  }
}

export function recordTurnUsage(tab: TabState, u: Record<string, unknown>, accumulate: boolean) {
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
  if (tab.currentTurnRecord) {
    if (accumulate) tab.currentTurnRecord.resultUsage = turn;
    else tab.currentTurnRecord.envelopeUsage = turn;
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
    tab.sessionUsage = {
      totalInput: tab.sessionUsage.totalInput + turn.input,
      totalOutput: tab.sessionUsage.totalOutput + turn.output,
      totalCacheRead: tab.sessionUsage.totalCacheRead + turn.cacheRead,
      totalCacheCreate: tab.sessionUsage.totalCacheCreate + turn.cacheCreate,
      turns: tab.sessionUsage.turns + 1,
    };
  } else {
    // assistant envelope = point-in-time window occupancy → drives the pill.
    tab.lastTurnUsage = turn;
    // A turn just completed — advance the sidebar's activity clock so the row
    // sorts to "now" on real work (not on mere open/switch).
    tab.lastActivityAt = Date.now();
    // This message just completed — bank its exact output and clear the
    // in-flight char estimate, snapping the live count to the real total.
    tab.committedOutputTokens += turn.output;
    tab.liveOutputChars = 0;
    // Same monotonic clamp as refreshLiveTokens — snapping exact must never
    // roll the visible meter down mid-turn.
    tab.liveOutputTokens = Math.max(tab.liveOutputTokens, tab.committedOutputTokens);
  }
}

function fillToolResult(tab: TabState, toolUseId: string, content: string, isError: boolean) {
  if (tab.currentTurnRecord) {
    const rec = tab.currentTurnRecord.toolUses.find((t) => t.id === toolUseId);
    if (rec) {
      const now = Date.now();
      rec.completedAt = now;
      rec.durationMs = now - rec.startedAt;
      rec.isError = isError;
    }
  }
  // S124: mark matching agent spawn done — but only if it isn't already closed.
  // The per-agent `result` envelope (markSpawnDone) usually settles a spawn first;
  // a late top-level Task tool_result must NOT re-close it, or a spawn already
  // shown as a clean ✓ could flip to error post-hoc (done→error flicker) when the
  // two terminal signals disagree. Idempotent, matching markSpawnDone.
  const agentIdx = tab.agentSpawns.findIndex((a) => a.id === toolUseId);
  if (agentIdx !== -1 && tab.agentSpawns[agentIdx].completedAt == null) {
    const next = tab.agentSpawns.slice();
    next[agentIdx] = { ...next[agentIdx], completedAt: Date.now(), isError };
    tab.agentSpawns = next;
  }
  const now = Date.now();
  mutateStreaming(tab, (m) => ({
    ...m,
    blocks: m.blocks.map((b) => {
      if (b.type !== "tool" || b.id !== toolUseId) return b;
      const durationMs = typeof b.startedAt === "number" ? now - b.startedAt : undefined;
      // A tool with a result is definitionally not forming anymore (#100).
      return { ...b, result: content, isError, status: isError ? "error" : "done", durationMs, inputPartial: undefined };
    }),
  }));
}

/** Update one sub-agent's block sub-transcript immutably (reactive reassign). */
function mutateAgent(tab: TabState, agentId: string, fn: (blocks: Block[]) => Block[]) {
  const idx = tab.agentSpawns.findIndex((a) => a.id === agentId);
  if (idx === -1) return;
  const next = tab.agentSpawns.slice();
  next[idx] = { ...next[idx], blocks: fn(next[idx].blocks.slice()) };
  tab.agentSpawns = next;
}

/** Mark a spawn done if it isn't already. The dock's ONLY pre-existing completion
 *  signal was the top-level Task `tool_result` (fillToolResult) — but that
 *  envelope can be missed, mis-ordered, or absent entirely for forking-skill
 *  spawns promoted under a Skill id, leaving the spawn stuck `completedAt: null`
 *  forever (perpetual spinner / "N working", and the model reasoning off a
 *  spawn it thinks is still running). This closes a spawn from any terminal
 *  signal: its own per-agent `result` envelope (live) or the turn-end sweep.
 *  Idempotent — a no-op once already completed, so the live path and the sweep
 *  can't double-fire. */
function markSpawnDone(tab: TabState, agentId: string, isError: boolean) {
  const idx = tab.agentSpawns.findIndex((a) => a.id === agentId);
  if (idx === -1 || tab.agentSpawns[idx].completedAt != null) return;
  const next = tab.agentSpawns.slice();
  next[idx] = { ...next[idx], completedAt: Date.now(), isError };
  tab.agentSpawns = next;
}

/** A nested sub-agent frame (parent_tool_use_id === a known spawn's id). The CLI
 *  multiplexes Task/Agent sub-agent output into the same stream; we divert it
 *  here into that agent's own transcript for the live dock — and OUT of the main
 *  bubble (pre-routing these leaked in as stray main-turn chips/text). Sub-agent
 *  content arrives at envelope granularity only (no token deltas), so each
 *  assistant/user envelope maps straight to appended/filled blocks. */
function applySubAgentFrame(tab: TabState, agentId: string, env: StreamEnvelope) {
  if (env.type === "assistant") {
    const now = Date.now();
    const newBlocks: Block[] = [];
    for (const block of env.message?.content ?? []) {
      if (block.type === "text" && typeof block.text === "string" && block.text.length > 0) {
        newBlocks.push({ type: "text", text: block.text });
      } else if (block.type === "thinking") {
        const text = typeof block.thinking === "string" ? block.thinking : "";
        const hasSignature = typeof block.signature === "string" && block.signature.length > 0;
        newBlocks.push({ type: "thinking", text, hasSignature, startedAt: now, durationMs: null, status: "done" });
      } else if (block.type === "tool_use") {
        const { id, name } = block;
        const input = block.input ?? {};
        newBlocks.push({ type: "tool", id, name, input, result: null, isError: false, status: "pending", startedAt: now });
      } else if (block.type !== "text") {
        // #98.3 (agent mini-transcript): breadcrumb only — the dock is
        // ephemeral, the main-bubble path renders the visible marker.
        console.warn("[assistant] unknown sub-agent content block type", (block as { type?: unknown })?.type);
      }
    }
    if (newBlocks.length > 0) {
      mutateAgent(tab, agentId, (blocks) => [...blocks, ...newBlocks]);
    }
  } else if (env.type === "user") {
    for (const block of env.message?.content ?? []) {
      if (block.type === "tool_result") {
        const targetId = block.tool_use_id;
        const content = flattenToolResult(block.content);
        const isError = block.is_error === true;
        mutateAgent(tab, agentId, (blocks) =>
          blocks.map((b) =>
            b.type === "tool" && b.id === targetId
              ? {
                  ...b,
                  result: content,
                  isError,
                  status: isError ? "error" : "done",
                  durationMs: typeof b.startedAt === "number" ? Date.now() - b.startedAt : undefined,
                }
              : b,
          ),
        );
      }
    }
  } else if (env.type === "result") {
    // The sub-agent emitted its own terminal `result` envelope (carrying this
    // spawn's parent_tool_use_id) — the live "this agent finished" signal. Flip
    // the parent spawn to done NOW so the dock settles the moment recon/scout
    // returns, instead of waiting on the top-level Task tool_result (which may
    // never match — see markSpawnDone). subtype "success" → done; anything else
    // → error.
    const isError = typeof env.subtype === "string" && env.subtype !== "success";
    markSpawnDone(tab, agentId, isError);
  }
}

/** Find an in-flight Skill/SlashCommand/Workflow tool block by id — the lazy-
 *  promotion lookup for tools whose sub-agents multiplex under the parent's
 *  tool_use id (forking slash commands, Workflow orchestration runs). */
function findPendingSkillBlock(tab: TabState, id: string): ToolBlock | null {
  for (const m of tab.messages) {
    for (const b of m.blocks) {
      if (
        b.type === "tool" && b.id === id &&
        (b.name === "Skill" || b.name === "SlashCommand" || b.name === "Workflow")
      ) {
        return b;
      }
    }
  }
  return null;
}

/** Spin up a live dock spawn for a forking skill on its first nested frame.
 *  The Skill ToolBlock stays in the bubble (its final result persists there);
 *  this spawn carries only the live sub-transcript and is dropped at turn end.
 *  Lazy (not eager in appendToolUse) so a non-forking skill — /check, /handoff —
 *  never leaves a dead, empty dock section. */
function promoteSkillSpawn(tab: TabState, block: ToolBlock) {
  if (tab.agentSpawns.some((a) => a.id === block.id)) return;
  const isSkill = block.name === "Skill";
  const isWorkflow = block.name === "Workflow";
  const subagentType = isSkill
    ? String(block.input?.skill ?? "skill")
    : isWorkflow
      ? "workflow"
      : String(block.input?.command ?? "command");
  const args = isSkill && typeof block.input?.args === "string" ? (block.input.args as string) : "";
  const description = isSkill
    ? `/${String(block.input?.skill ?? "")}${args ? ` ${args}` : ""}`.trim()
    : isWorkflow
      ? String(block.input?.name ?? "workflow run")
      : String(block.input?.command ?? "(command)");
  tab.agentSpawns = capSpawns([
    ...tab.agentSpawns,
    {
      id: block.id,
      subagentType,
      description,
      startedAt: typeof block.startedAt === "number" ? block.startedAt : Date.now(),
      completedAt: null,
      isError: false,
      blocks: [],
      kind: "skill",
    },
  ]);
}

export function onStreamLine(tab: TabState, raw: string) {
  if (tab.rawLineLog.length >= 200) tab.rawLineLog.shift();
  tab.rawLineLog.push(raw);
  let env: StreamEnvelope;
  try {
    env = JSON.parse(raw) as StreamEnvelope;
  } catch {
    if (tab.streaming && tab.streamingMsgId && raw.length > 0) {
      const prefix = tab.deltaCount > 0 ? "\n" : "";
      tab.deltaCount++;
      if (tab.currentTurnRecord) {
        tab.currentTurnRecord.deltaCount = tab.deltaCount;
        markFirstPaint(tab);
      }
      enqueueText(tab, prefix + raw);
    } else if (raw.length > 0 && import.meta.env.DEV) {
      // #182: post-done CLI dribble — logged in dev only.
      console.debug("[assistant] orphaned non-JSON line (post-done)", raw.slice(0, 80));
    }
    return;
  }
  // Nested sub-agent frame: a non-empty parent_tool_use_id means this belongs to
  // a spawned Task/Agent, never the main bubble. Route to that agent's own
  // sub-transcript (live dock) if we're tracking the spawn; otherwise drop it
  // (e.g. a deeper nested sub-agent we don't surface yet) rather than let it leak
  // into the main message. Either way, STOP — don't fall through to the switch.
  const parentId = (env as { parent_tool_use_id?: string | null }).parent_tool_use_id;
  if (typeof parentId === "string" && parentId.length > 0) {
    if (tab.agentSpawns.some((a) => a.id === parentId)) {
      applySubAgentFrame(tab, parentId, env);
    } else {
      // A forking skill (/plan etc.) multiplexes its sub-agent output under the
      // Skill tool_use id, which isn't a registered spawn. Promote it lazily on
      // this first nested frame, then route — so the live dock fills instead of
      // the frame being silently dropped.
      const skill = findPendingSkillBlock(tab, parentId);
      if (skill) {
        promoteSkillSpawn(tab, skill);
        applySubAgentFrame(tab, parentId, env);
      }
    }
    return;
  }
  // Idle tab + a real assistant frame = a CLI-initiated continuation turn
  // (background agent finished). Scaffold it so the switch below streams into
  // a fresh bubble instead of no-op'ing against a null streamingMsgId.
  if (!tab.streaming) maybeBeginContinuation(tab, env);
  switch (env.type) {
    case "stream_event": {
      if (tab.currentTurnRecord) {
        tab.currentTurnRecord.streamEventCount += 1;
        const now = Date.now();
        // Anchor against last event arrival, falling back to first-paint or
        // turn-start so the metric is defined even on the first stream_event.
        const last = tab.lastStreamEventAt ?? tab.currentTurnRecord.firstPaintAt ?? tab.currentTurnRecord.ts;
        const gap = now - last;
        if (gap > tab.currentTurnRecord.maxStreamGapMs) {
          tab.currentTurnRecord.maxStreamGapMs = gap;
        }
        tab.lastStreamEventAt = now;
      }
      const ev = env.event;
      const evType = ev?.type;
      const idx = typeof ev?.index === "number" ? ev.index : null;
      if (evType === "content_block_start" && idx !== null) {
        const cb = ev?.content_block;
        if (cb?.type === "thinking") {
          beginThinking(tab, idx);
        } else if (cb?.type === "tool_use" && typeof cb.id === "string" && typeof cb.name === "string") {
          beginToolForming(tab, idx, cb);
        }
      } else if (evType === "content_block_delta") {
        const d = ev?.delta;
        if (d?.type === "text_delta" && d.text) {
          tab.deltaCount++;
          if (tab.currentTurnRecord) {
            tab.currentTurnRecord.deltaCount = tab.deltaCount;
            markFirstPaint(tab);
          }
          enqueueText(tab, d.text);
        } else if (d?.type === "thinking_delta" && typeof d.thinking === "string" && idx !== null) {
          appendThinkingText(tab, idx, d.thinking);
        } else if (d?.type === "signature_delta" && idx !== null) {
          markThinkingSignature(tab, idx);
        } else if (d?.type === "input_json_delta" && typeof d.partial_json === "string" && idx !== null) {
          appendToolInputDelta(tab, idx, d.partial_json);
        }
      } else if (evType === "content_block_stop" && idx !== null) {
        endThinking(tab, idx);
        endToolForming(tab, idx);
      } else if (evType === "message_delta") {
        // Terminal stop reason for this assistant message. Only the noteworthy
        // ones land on the bubble — a normal end_turn/tool_use is silent.
        const sr = ev?.delta?.stop_reason;
        if (sr === "max_tokens" || sr === "refusal") {
          mutateStreaming(tab, (m) => ({ ...m, stopReason: sr }));
        }
      } else if (evType != null && evType !== "message_start" && evType !== "message_stop" && evType !== "ping") {
        // A partial-message event shape we don't recognize (a newer CLI). The
        // message_start/stop/ping frames are deliberately no-op'd (envelope-level
        // handling owns turn lifecycle); anything else is worth a breadcrumb so a
        // future streaming-delta type isn't dropped without a trace.
        console.warn("[assistant] unrecognized stream_event inner type", evType, ev);
      }
      break;
    }
    case "assistant": {
      if (tab.currentTurnRecord) tab.currentTurnRecord.assistantEnvCount += 1;
      const msgUsage = env.message?.usage;
      if (msgUsage) recordTurnUsage(tab, msgUsage, false);
      for (const block of env.message?.content ?? []) {
        if (block.type === "tool_use") {
          appendToolUse(tab, block);
        } else if (block.type === "text" && typeof block.text === "string") {
          tab.envelopeTextBuffer += block.text;
        } else if (block.type === "thinking") {
          ensureThinkingFromEnvelope(tab, block);
        } else {
          // #98.3: a block type this build predates (newer CLI) — surface a
          // muted marker instead of silently dropping the content.
          appendUnknownBlock(tab, block);
        }
      }
      break;
    }
    case "user": {
      for (const block of env.message?.content ?? []) {
        if (block.type === "tool_result") {
          fillToolResult(
            tab,
            block.tool_use_id,
            flattenToolResult(block.content),
            block.is_error === true,
          );
        }
      }
      break;
    }
    case "result": {
      // GitHub chip: the turn is over — if it pushed / opened a PR, the remote
      // state changed; refresh the branch status once.
      github.flushRemoteMutation();
      if (typeof env.total_cost_usd === "number") {
        tab.totalCostUsd = (tab.totalCostUsd ?? 0) + env.total_cost_usd;
        const turnCost = env.total_cost_usd;
        // Accumulate, don't overwrite: a message only carries a cost/duration
        // already when a merged continuation reopened it (one result per
        // message otherwise), so summing keeps the merged header honest.
        mutateStreaming(tab, (m) => ({ ...m, costUsd: (m.costUsd ?? 0) + turnCost }));
        if (tab.currentTurnRecord) tab.currentTurnRecord.costUsd = turnCost;
      }
      // Wall-clock of the turn (CLI-measured, tool waits included) — drives
      // the honest "Worked for Ns" header instead of the thinking+tool sum.
      const wallMs = (env as { duration_ms?: unknown }).duration_ms;
      if (typeof wallMs === "number" && wallMs > 0) {
        mutateStreaming(tab, (m) => ({ ...m, turnDurationMs: (m.turnDurationMs ?? 0) + wallMs }));
      }
      const resultUsage = (env as { usage?: Record<string, unknown> }).usage;
      // Token total for the done receipt — same accumulate-don't-overwrite rule
      // as costUsd (merged continuations reopen the message).
      const outTok = resultUsage && typeof resultUsage.output_tokens === "number"
        ? (resultUsage.output_tokens as number) : null;
      if (outTok != null && outTok > 0) {
        mutateStreaming(tab, (m) => ({ ...m, outputTokens: (m.outputTokens ?? 0) + outTok }));
      }
      if (resultUsage) recordTurnUsage(tab, resultUsage, true);
      // Ground-truth context window (#94 self-correction): the CLI reports the
      // window each model actually ran against — modelUsage[id].contextWindow —
      // which folds in the [1m] selector AND account-side gating the user-set
      // plan estimate can't see. Prefer the turn's own model; else the largest
      // entry. ctxWindowFor prefers this over plan×model while the tab stays
      // on that model.
      // Keys + lastModelId both normalized: init reports the model WITH the
      // [1m] selector and modelUsage keys can carry it too (probe-verified) —
      // and modelUsage is session-CUMULATIVE after a mid-chat switch, so the
      // turn-model match (not the largest entry) is what keeps this honest.
      const mu = (env as { modelUsage?: Record<string, { contextWindow?: number }> }).modelUsage;
      if (mu) {
        const norm = (s: string) => s.replace(/\[1m\]$/i, "");
        const turnModel = norm(tab.lastModelId ?? "");
        let pick: { model: string; window: number } | null = null;
        for (const [id, u] of Object.entries(mu)) {
          const w = typeof u?.contextWindow === "number" ? u.contextWindow : 0;
          if (w <= 0) continue;
          if (norm(id) === turnModel) { pick = { model: norm(id), window: w }; break; }
          if (!pick || w > pick.window) pick = { model: norm(id), window: w };
        }
        if (pick) tab.reportedCtxWindow = pick;
      }
      // Fast-mode honesty stamp: only when the CLI CONFIRMS the turn ran fast
      // (fast_mode_state / usage.speed) — a requested-but-unavailable fast
      // turn gets no badge.
      const fms = (env as { fast_mode_state?: unknown }).fast_mode_state;
      const speed = (resultUsage as { speed?: unknown } | undefined)?.speed;
      if (fms === "on" || speed === "fast") {
        mutateStreaming(tab, (m) => ({ ...m, fast: true }));
      }
      // Honest done receipt: background Agent launches that haven't reported
      // back yet — the footer says "agent still working" instead of "Done".
      mutateStreaming(tab, (m) => {
        const pending = Math.max(0, countBgAgentLaunches(m) - (m.bgAgentsResolved ?? 0));
        return (m.bgAgentsPending ?? 0) === pending ? m : { ...m, bgAgentsPending: pending };
      });
      if (env.subtype && env.subtype !== "success") {
        const msg = RESULT_ERROR_MESSAGES[env.subtype];
        if (msg) {
          tab.lastError = msg;
        } else if ((env as { is_error?: boolean }).is_error === true) {
          // Unknown error subtype from a newer CLI: surface the CLI's own
          // errors[] text instead of ending the turn with no explanation.
          const errs = (env as { errors?: unknown[] }).errors;
          const cliMsg = Array.isArray(errs)
            ? errs.find((e): e is string => typeof e === "string" && e.length > 0)
            : undefined;
          tab.lastError =
            cliMsg ?? `The run stopped early (${env.subtype.replace(/^error_/, "").replace(/_/g, " ")}).`;
        } else {
          console.warn("[assistant] unrecognized result.subtype", env.subtype, env);
        }
      }
      break;
    }
    case "system": {
      const sysModel = typeof env.model === "string" ? env.model : null;
      if (sysModel) {
        tab.lastModelId = sysModel;
        mutateStreaming(tab, (m) => ({ ...m, model: sysModel }));
        if (tab.currentTurnRecord) tab.currentTurnRecord.modelId = sysModel;
      }
      // The CLI auto-compacts when its context window fills and emits a
      // `compact_boundary` system event. Surface it as a visible transcript
      // marker so the conversation never silently resets under the user.
      // (microcompact_boundary — the lighter tool-output trim — is intentionally
      // not surfaced; it fires often and would clutter the transcript.)
      if (env.subtype === "compact_boundary") appendCliCompaction(tab, env);
      // First frame of every turn lists MCP server health — a failed rift
      // server means dead workspace tools; toast instead of staying silent.
      // The list is also kept on the tab so /mcp can answer without a turn.
      if (env.subtype === "init") {
        const raw = (env as { mcp_servers?: unknown }).mcp_servers;
        if (Array.isArray(raw)) {
          tab.mcpServers = raw
            .filter((s): s is { name?: unknown; status?: unknown } => typeof s === "object" && s !== null)
            .map((s) => ({
              name: typeof s.name === "string" ? s.name : "?",
              status: typeof s.status === "string" ? s.status : "unknown",
            }));
        }
        checkMcpInitHealth(env as { mcp_servers?: unknown });
        // #98.1: the CLI announcing its own slash-command set (builtins +
        // bundled plugin skills the disk scan can't see) — feeds the
        // composer menu's CLI section, persisted across restarts.
        cliCommands.setFromInit((env as { slash_commands?: unknown }).slash_commands);
      }
      break;
    }
    case "prompt_suggestion": {
      // #87: composer ghost chip — shown while the draft is empty, consumed on
      // click, cleared at the next beginTurn. Never rendered into the transcript.
      const s = env.suggestion;
      if (typeof s === "string" && s.trim().length > 0) tab.promptSuggestion = s.trim();
      break;
    }
    default: {
      // An event kind this build doesn't recognize — almost always a newer CLI
      // emitting something we predate. Don't render it (we don't know its shape),
      // but leave a breadcrumb so a future-CLI display gap is debuggable instead
      // of silently vanishing. The onStreamDone blank-turn net still catches the
      // case where unknown events were ALL a turn produced. `env` narrows to
      // `never` here (the union is exhaustive) — read the runtime type back off
      // an unknown cast, since the actual CLI can emit outside our closed union.
      const unknownType = (env as { type?: unknown }).type;
      console.warn("[assistant] unrecognized stream event type", unknownType, env);
      break;
    }
  }
}

export function onStreamDone(tab: TabState) {
  if (!tab.streaming) {
    // Terminal event for an already-settled turn (user Stop) — consume the
    // post-stop gate so a deferred next send can start immediately.
    tab.staleTerminalUntil = 0;
    return;
  }
  flushPendingText(tab);
  let envelopeFallback = false;
  let blankTurn = false;
  // CLI no-op turn: resuming a session that was interrupted mid-turn (user
  // Stop, killed child), the CLI injects its own "Continue from where you left
  // off." user turn and — with nothing to add — answers a canned "No response
  // requested.", usually stop-sequence-suppressed into a near-silent stream.
  // That's a deliberate no-op, not a malformed stream: drop the dead bubble
  // instead of surfacing the "Blank response" error (the "good response just
  // blank" turn, Discord-Bot session 2026-07-17).
  const NOOP_TEXT = "No response requested.";
  const CLI_CONTINUE = "Continue from where you left off.";
  const streamMsg = tab.streamingMsgId
    ? tab.messages.find((m) => m.id === tab.streamingMsgId)
    : null;
  const cliNoop =
    tab.deltaCount === 0 &&
    !!streamMsg &&
    streamMsg.blocks.length === 0 &&
    (tab.envelopeTextBuffer.trim() === NOOP_TEXT ||
      tab.rawLineLog.some((ln) => {
        try {
          const p = JSON.parse(ln) as {
            type?: string;
            message?: { content?: string | Array<{ type?: string; text?: string }> };
          };
          const c = p.message?.content;
          const texts = typeof c === "string" ? [c] : (c ?? []).map((b) => (b.type === "text" ? (b.text ?? "") : ""));
          if (p.type === "assistant") return texts.some((t) => t.trim() === NOOP_TEXT);
          if (p.type === "user") return texts.some((t) => t.trim() === CLI_CONTINUE);
          return false;
        } catch {
          return false;
        }
      }));
  if (cliNoop) {
    tab.messages = tab.messages.filter((m) => m.id !== tab.streamingMsgId);
  } else if (tab.deltaCount === 0 && tab.envelopeTextBuffer.length > 0) {
    appendText(tab, tab.envelopeTextBuffer);
    envelopeFallback = true;
  } else if (tab.deltaCount === 0 && tab.envelopeTextBuffer.length === 0) {
    const msg = tab.streamingMsgId
      ? tab.messages.find((m) => m.id === tab.streamingMsgId)
      : null;
    // Thinking counts as content: a thinking-only turn (e.g. the thinking
    // budget tripped before any text/tool) is an errored turn, not a
    // blank/malformed stream — the result-subtype handler already messaged it.
    const hadContent = !!msg && msg.blocks.some((b) => b.type === "tool" || b.type === "thinking");
    // A compaction-only turn (manual /compact or auto) legitimately ends with
    // no assistant text — the boundary pill IS the response, not a blank stream.
    // Exact subtype match on purpose: microcompact_boundary doesn't count.
    const compacted =
      !hadContent &&
      tab.rawLineLog.some((ln) => {
        try {
          const p = JSON.parse(ln) as { type?: string; subtype?: string };
          return p.type === "system" && p.subtype === "compact_boundary";
        } catch {
          return false;
        }
      });
    if (!hadContent && !compacted) blankTurn = true;
    // Don't clobber a friendly error the `result` subtype handler set moments
    // ago with the raw NDJSON dump — that dump is a last-resort diagnostic.
    if (!hadContent && !compacted && tab.lastError == null) {
      const lines = tab.rawLineLog.slice();
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
      tab.lastError = `Blank response — CLI emitted ${lines.length} line(s): ${fingerprint}.${tail}`;
    }
  }
  finalizeInflightBlocks(tab);
  tab.streaming = false;
  tab.streamingMsgId = null;
  tab.streamingMsgIdx = null;
  // Clean-DONE stamp only (not error/stop): gates the continuation merge in
  // maybeBeginContinuation — never merge new frames into an errored bubble.
  tab.lastTurnDoneAt = tab.lastError == null ? Date.now() : null;
  tab.seenToolUseIds.clear();
  // Drop any unanswered permission asks — the backend auto-denies on turn
  // end, so a lingering Allow/Deny chip would be dead.
  if (tab.permissionPrompts.size > 0) tab.permissionPrompts = new Map();
  tab.unboundAskUserRequestIds = [];
  tab.unboundAskUserToolUseIds = [];
  // RR7: clear ask_user toolUseId→requestId bindings too. A turn that ends
  // before the user answers an ask_user chip would otherwise leave the binding
  // live, keeping the dead chip's Allow/Deny interactive (a click then hits a
  // resolved/auto-denied oneshot) — mirrors the permissionPrompts clear above.
  if (tab.askUserBindings.size > 0) tab.askUserBindings = new Map();
  tab.activity = { ...tab.activity, currentLabel: null };
  // Finalize telemetry for this turn.
  if (tab.currentTurnRecord) {
    tab.currentTurnRecord.doneAt = Date.now();
    tab.currentTurnRecord.envelopeFallback = envelopeFallback;
    tab.currentTurnRecord.blankTurn = blankTurn;
    if (!tab.currentTurnRecord.endKind) {
      // lastError is cleared at turn start, so non-null here means THIS turn
      // errored (result subtype) even when it produced thinking/tool content.
      const errored = blankTurn || tab.lastError != null;
      tab.currentTurnRecord.endKind = errored ? "error" : "success";
      if (errored) tab.currentTurnRecord.errorMsg = tab.lastError ?? "blank turn";
    }
    tab.currentTurnRecord = null;
  }
  tab.onTurnComplete?.(tab);
}

/** RR10: settle any in-flight thinking/tool blocks to a terminal status BEFORE
 *  the caller clears streamingMsgId (the mutate helpers no-op once it's null).
 *  Any terminal path that stops a stream mid-reasoning — error OR a user Stop —
 *  must call this, else a thinking chip persists stuck status:"active" and tool
 *  chips stuck status:"pending" forever in the saved history. Shared by
 *  onStreamError and send.ts::stop(). Does NOT clear streamingMsgId itself. */
export function finalizeInflightBlocks(tab: TabState) {
  for (const index of [...tab.thinkingByIndex.keys()]) {
    const entry = tab.thinkingByIndex.get(index);
    const durationMs = entry ? Date.now() - entry.startedAt : 0;
    mutateThinking(tab, index, (b) =>
      b.status === "active" ? { ...b, status: "done", durationMs } : b,
    );
  }
  tab.thinkingByIndex.clear();
  tab.toolInputByIndex.clear();
  tab.activeThinkingIndex = null;
  if (tab.activity.currentLabel === "Thinking…") {
    tab.activity = { ...tab.activity, currentLabel: null };
  }
  // Safety net: this turn is settling (normal done, error, or user Stop), so no
  // spawn can still be running. Close any spawn left `completedAt: null` — its
  // terminal signal (top-level Task tool_result OR per-agent result envelope)
  // was missed or absent. Without this a dock section spins forever and the
  // model reads the spawn as live (the reported bug). Shared by every terminal
  // path via onStreamError/stop, not just onStreamDone.
  //
  // A spawn reaching this sweep still-open is already abnormal — a healthy
  // sub-agent emits its own `result` envelope and is closed via markSpawnDone
  // well before turn-end. So mark it errored, not a clean ✓: it was interrupted
  // (user Stop / turn error) or its terminal signal was lost. This mirrors the
  // pending→error flip applied to main-turn tool blocks just below, so an
  // aborted turn reads consistently (no green-check sub-agents under a red turn).
  if (tab.agentSpawns.some((a) => a.completedAt == null)) {
    const now = Date.now();
    tab.agentSpawns = tab.agentSpawns.map((a) =>
      a.completedAt == null ? { ...a, completedAt: now, isError: true } : a,
    );
  }
  // Shell rows are live-turn-only: the backend poller dies with the turn, so
  // anything left here is stale. Clear on every terminal path.
  if (tab.shellRows.length > 0) tab.shellRows = [];
  if (tab.streamingMsgId) {
    mutateStreaming(tab, (m) => ({
      ...m,
      blocks: m.blocks.map((b) =>
        b.type === "tool" && b.status === "pending"
          ? { ...b, status: "error", inputPartial: undefined }
          : b,
      ),
    }));
  }
}

export function onStreamError(tab: TabState, msg: string) {
  if (!tab.streaming) {
    // Stale terminal for a turn the user already stopped (the stop marker can
    // be consumed by a racing next send, remapping its DONE to ERROR) — a
    // settled turn must not surface a late error banner. Consume the gate.
    tab.staleTerminalUntil = 0;
    return;
  }
  tab.lastError = msg;
  tab.streaming = false;
  tab.lastTurnDoneAt = null; // errored turn — a trailing continuation must not merge into it
  if (tab.drainHandle !== null) {
    cancelAnimationFrame(tab.drainHandle);
    tab.drainHandle = null;
  }
  tab.pendingText = "";
  finalizeInflightBlocks(tab);
  if (tab.streamingMsgId) {
    const id = tab.streamingMsgId;
    tab.messages = tab.messages.filter((m) => !(m.id === id && m.blocks.length === 0));
    tab.streamingMsgId = null;
  }
  tab.streamingMsgIdx = null;
  tab.seenToolUseIds.clear();
  if (tab.permissionPrompts.size > 0) tab.permissionPrompts = new Map();
  tab.unboundAskUserRequestIds = [];
  tab.unboundAskUserToolUseIds = [];
  // RR7: see onStreamDone — clear ask_user bindings on the error terminal path
  // too, so a dead chip can't stay interactive after the turn errors.
  if (tab.askUserBindings.size > 0) tab.askUserBindings = new Map();
  // Finalize telemetry.
  if (tab.currentTurnRecord) {
    tab.currentTurnRecord.doneAt = Date.now();
    tab.currentTurnRecord.endKind = "error";
    tab.currentTurnRecord.errorMsg = msg;
    tab.currentTurnRecord = null;
  }
  // Mirror onDone: a turn that ends in error (or partial-stream-then-error,
  // which the user perceives as "completed") is still terminal — fire the
  // completion hook so the store drains any queued message instead of
  // leaving the chat stuck in queue mode.
  tab.onTurnComplete?.(tab);
}
