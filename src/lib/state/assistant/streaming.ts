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
import { flattenToolResult, previewToolInput } from "./helpers";

/** Called at the start of every send(). Clears per-turn pacer / thinking
 *  / dedupe state and flips streaming on. */
export function beginTurn(tab: TabState) {
  tab.lastError = null;
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
  tab.activeThinkingIndex = null;
  tab.lastStreamEventAt = null;
  tab.turnStartNotified = false;
  tab.liveOutputTokens = 0;
  tab.committedOutputTokens = 0;
  tab.liveOutputChars = 0;
  tab.activity = { currentLabel: null, turnStartedAt: Date.now() };
  tab.streaming = true;
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

function beginThinking(tab: TabState, index: number) {
  if (tab.thinkingByIndex.has(index)) return;
  tab.activeThinkingIndex = index;
  const startedAt = Date.now();
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
    return { ...m, blocks };
  });
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
  const existing = msg.blocks.find((b) => b.type === "thinking") as ThinkingBlock | undefined;
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
  tab.liveOutputTokens = tab.committedOutputTokens + Math.round(tab.liveOutputChars / 4);
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
  // Drain the char buffer into the message faster than before (≈0.25s window,
  // 180 c/s floor, was 0.4s/120) so this char-pacer adds less latency on top of
  // Markdown's word-reveal — the two used to compound into a visible lag behind
  // generation. The word-reveal stays the visible cadence authority.
  const rate = Math.max(180, tab.pendingText.length / 0.25);
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

function appendToolUse(tab: TabState, block: { id: string; name: string; input?: Record<string, unknown> }) {
  if (tab.seenToolUseIds.has(block.id)) return;
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
    tab.onTodoApplied?.(tab, opensDock);
    return;
  }
  if (block.name === "TaskCreate") {
    const opensDock = applyTaskCreate(tab, block.input);
    tab.onTodoApplied?.(tab, opensDock);
    return;
  }
  if (block.name === "TaskUpdate") {
    applyTaskUpdate(tab, block.input);
    tab.onTodoApplied?.(tab, false);
    return;
  }
  if (block.name === "Task" || block.name === "Agent") {
    const subagentType = String(block.input?.subagent_type ?? "fork");
    const description = String(block.input?.description ?? "(no description)");
    tab.agentSpawns = [
      ...tab.agentSpawns,
      { id: block.id, subagentType, description, startedAt: Date.now(), completedAt: null, isError: false, blocks: [] },
    ];
    return;
  }
  const DENY = new Set(["ToolSearch"]);
  if (DENY.has(block.name)) return;
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
    // This message just completed — bank its exact output and clear the
    // in-flight char estimate, snapping the live count to the real total.
    tab.committedOutputTokens += turn.output;
    tab.liveOutputChars = 0;
    tab.liveOutputTokens = tab.committedOutputTokens;
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
  // S124: mark matching agent spawn done.
  const agentIdx = tab.agentSpawns.findIndex((a) => a.id === toolUseId);
  if (agentIdx !== -1) {
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
      return { ...b, result: content, isError, status: isError ? "error" : "done", durationMs };
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

/** A nested sub-agent frame (parent_tool_use_id === a known spawn's id). The CLI
 *  multiplexes Task/Agent sub-agent output into the same stream; we divert it
 *  here into that agent's own transcript for the live dock — and OUT of the main
 *  bubble (pre-routing these leaked in as stray main-turn chips/text). Sub-agent
 *  content arrives at envelope granularity only (no token deltas), so each
 *  assistant/user envelope maps straight to appended/filled blocks. */
function applySubAgentFrame(tab: TabState, agentId: string, env: StreamEnvelope) {
  if (env.type === "assistant") {
    for (const block of env.message?.content ?? []) {
      if (block.type === "text" && typeof block.text === "string" && block.text.length > 0) {
        const text = block.text;
        mutateAgent(tab, agentId, (blocks) => [...blocks, { type: "text", text }]);
      } else if (block.type === "thinking") {
        const text = typeof block.thinking === "string" ? block.thinking : "";
        const hasSignature = typeof block.signature === "string" && block.signature.length > 0;
        mutateAgent(tab, agentId, (blocks) => [
          ...blocks,
          { type: "thinking", text, hasSignature, startedAt: Date.now(), durationMs: null, status: "done" },
        ]);
      } else if (block.type === "tool_use") {
        const { id, name } = block;
        const input = block.input ?? {};
        mutateAgent(tab, agentId, (blocks) => [
          ...blocks,
          { type: "tool", id, name, input, result: null, isError: false, status: "pending", startedAt: Date.now() },
        ]);
      }
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
  }
}

/** Find an in-flight Skill/SlashCommand tool block by id — the lazy-promotion
 *  lookup for forking slash commands (/plan etc.) whose sub-agents multiplex
 *  under the skill's tool_use id. */
function findPendingSkillBlock(tab: TabState, id: string): ToolBlock | null {
  for (const m of tab.messages) {
    for (const b of m.blocks) {
      if (b.type === "tool" && b.id === id && (b.name === "Skill" || b.name === "SlashCommand")) {
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
  const subagentType = isSkill
    ? String(block.input?.skill ?? "skill")
    : String(block.input?.command ?? "command");
  const args = isSkill && typeof block.input?.args === "string" ? (block.input.args as string) : "";
  const description = isSkill
    ? `/${String(block.input?.skill ?? "")}${args ? ` ${args}` : ""}`.trim()
    : String(block.input?.command ?? "(command)");
  tab.agentSpawns = [
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
  ];
}

export function onStreamLine(tab: TabState, raw: string) {
  if (tab.rawLineLog.length >= 200) tab.rawLineLog.shift();
  tab.rawLineLog.push(raw);
  // Rail-v2: first line of a turn proves the CLI is live (the backend's steer
  // registry registers before the reader spawns) — flush steer-mode chips.
  if (tab.streaming && !tab.turnStartNotified) {
    tab.turnStartNotified = true;
    tab.onTurnStarted?.(tab);
  }
  let env: StreamEnvelope;
  try {
    env = JSON.parse(raw) as StreamEnvelope;
  } catch {
    if (tab.streaming && tab.streamingMsgId && raw.length > 0) {
      const prefix = tab.deltaCount > 0 ? "\n" : "";
      tab.deltaCount++;
      if (tab.currentTurnRecord) {
        tab.currentTurnRecord.deltaCount = tab.deltaCount;
        if (tab.currentTurnRecord.firstPaintAt == null) {
          tab.currentTurnRecord.firstPaintAt = Date.now();
        }
      }
      enqueueText(tab, prefix + raw);
    } else if (raw.length > 0) {
      // #182: post-done CLI dribble was silently dropped — surface in console
      // for observability so we know if a known CLI bug regresses.
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
      if (evType === "content_block_start" && ev?.content_block?.type === "thinking" && idx !== null) {
        beginThinking(tab, idx);
      } else if (evType === "content_block_delta") {
        const d = ev?.delta;
        if (d?.type === "text_delta" && d.text) {
          tab.deltaCount++;
          if (tab.currentTurnRecord) {
            tab.currentTurnRecord.deltaCount = tab.deltaCount;
            if (tab.currentTurnRecord.firstPaintAt == null) {
              tab.currentTurnRecord.firstPaintAt = Date.now();
            }
          }
          enqueueText(tab, d.text);
        } else if (d?.type === "thinking_delta" && typeof d.thinking === "string" && idx !== null) {
          appendThinkingText(tab, idx, d.thinking);
        } else if (d?.type === "signature_delta" && idx !== null) {
          markThinkingSignature(tab, idx);
        }
      } else if (evType === "content_block_stop" && idx !== null) {
        endThinking(tab, idx);
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
      if (typeof env.total_cost_usd === "number") {
        tab.totalCostUsd = (tab.totalCostUsd ?? 0) + env.total_cost_usd;
        const turnCost = env.total_cost_usd;
        mutateStreaming(tab, (m) => ({ ...m, costUsd: turnCost }));
        if (tab.currentTurnRecord) tab.currentTurnRecord.costUsd = turnCost;
      }
      const resultUsage = (env as { usage?: Record<string, unknown> }).usage;
      if (resultUsage) recordTurnUsage(tab, resultUsage, true);
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
          tab.lastError = `Run ended with subtype: ${env.subtype}`;
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
      break;
    }
    default:
      break;
  }
}

export function onStreamDone(tab: TabState) {
  if (!tab.streaming) return;
  flushPendingText(tab);
  let envelopeFallback = false;
  let blankTurn = false;
  if (tab.deltaCount === 0 && tab.envelopeTextBuffer.length > 0) {
    appendText(tab, tab.envelopeTextBuffer);
    envelopeFallback = true;
  } else if (tab.deltaCount === 0 && tab.envelopeTextBuffer.length === 0) {
    const msg = tab.streamingMsgId
      ? tab.messages.find((m) => m.id === tab.streamingMsgId)
      : null;
    const hadTools = !!msg && msg.blocks.some((b) => b.type === "tool");
    if (!hadTools) blankTurn = true;
    if (!hadTools) {
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
  tab.streaming = false;
  tab.streamingMsgId = null;
  tab.streamingMsgIdx = null;
  tab.seenToolUseIds.clear();
  // Drop any unanswered permission asks — the backend auto-denies on turn
  // end, so a lingering Allow/Deny chip would be dead.
  if (tab.permissionPrompts.size > 0) tab.permissionPrompts = new Map();
  tab.unboundAskUserRequestIds = [];
  tab.unboundAskUserToolUseIds = [];
  tab.activity = { ...tab.activity, currentLabel: null };
  // Finalize telemetry for this turn.
  if (tab.currentTurnRecord) {
    tab.currentTurnRecord.doneAt = Date.now();
    tab.currentTurnRecord.envelopeFallback = envelopeFallback;
    tab.currentTurnRecord.blankTurn = blankTurn;
    if (!tab.currentTurnRecord.endKind) {
      tab.currentTurnRecord.endKind = blankTurn ? "error" : "success";
      if (blankTurn) tab.currentTurnRecord.errorMsg = tab.lastError ?? "blank turn";
    }
    tab.currentTurnRecord = null;
  }
  tab.onTurnComplete?.(tab);
}

export function onStreamError(tab: TabState, msg: string) {
  tab.lastError = msg;
  tab.streaming = false;
  if (tab.drainHandle !== null) {
    cancelAnimationFrame(tab.drainHandle);
    tab.drainHandle = null;
  }
  tab.pendingText = "";
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
