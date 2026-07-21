import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

// Mock Tauri IPC before importing the store. The playback harness never lets a
// real turn reach the backend — it drives the stream accumulators directly — so
// invoke/listen/dialog only need to be inert.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
// toast is a UI singleton the send orchestrator pushes to (e.g. error paths).
// Stub it so the harness stays headless.
vi.mock("./toast.svelte", () => {
  const push = vi.fn();
  return {
    toast: { push },
    notify: { ok: vi.fn(), info: vi.fn(), warn: vi.fn(), danger: vi.fn() },
  };
});

// onStream's text path paces itself through requestAnimationFrame (enqueueText →
// drainTick re-arms rAF until pendingText empties). The test env is `node` — no
// rAF — so we back it with a queue and a synchronous pump. pumpRaf() drains the
// queue to empty, which is exactly what a burst of real animation frames does
// between two CLI stream frames: prior text deltas finish painting before the
// next structural event (a tool block) lands. Faithfully reproduces inter-frame
// block ordering without real timers.
let rafSeq = 0;
const rafCbs = new Map<number, FrameRequestCallback>();
globalThis.requestAnimationFrame = ((cb: FrameRequestCallback) => {
  const h = ++rafSeq;
  rafCbs.set(h, cb);
  return h;
}) as typeof requestAnimationFrame;
globalThis.cancelAnimationFrame = ((h: number) => {
  rafCbs.delete(h);
}) as typeof cancelAnimationFrame;

function pumpRaf() {
  let guard = 0;
  while (rafCbs.size > 0) {
    if (++guard > 100_000) throw new Error("rAF pump did not settle");
    const [h, cb] = rafCbs.entries().next().value as [number, FrameRequestCallback];
    rafCbs.delete(h);
    cb(performance.now());
  }
}

import { assistant } from "./assistant.svelte.js";
import { send as sendDirect } from "./assistant/send.js";
import { finalizeInflightBlocks } from "./assistant/streaming.js";
import { notify, toast } from "./toast.svelte";
import { invoke } from "@tauri-apps/api/core";
import type { TurnRecord } from "./assistant/types.js";

const mockInvoke = vi.mocked(invoke);

type Tab = ReturnType<typeof assistant.ensureTab>;

// ── Harness ────────────────────────────────────────────────────────────────
// A "turn" in the real app is set up by AssistantStore.send(): it calls
// tab.beginTurn() (per-turn reset + streaming=true), pushes an empty assistant
// placeholder message, caches its index, and builds the TurnRecord. We replicate
// exactly that setup here, reusing the real beginTurn(), then feed recorded
// NDJSON frames through the real onStream/onDone/onError. Anything the
// accumulators touch is real code — only the CLI subprocess + backend forwarding
// is bypassed.

let seq = 0;

function beginTurn(tab: Tab, overrides: Partial<TurnRecord> = {}): TurnRecord {
  tab.beginTurn();
  const id = `asst-${++seq}`;
  tab.messages = [...tab.messages, { id, role: "assistant", blocks: [] }];
  tab.streamingMsgId = id;
  tab.streamingMsgIdx = tab.messages.length - 1;
  const rec: TurnRecord = {
    ts: Date.now(),
    convoId: "playback",
    cliSessionId: tab.cliSessionId,
    isFirstTurn: false,
    model: "sonnet",
    effort: "smart",
    effortFlag: "medium",
    promptLen: 4,
    promptPreview: "test",
    attachmentsCount: 0,
    attachmentsBytes: 0,
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
    thinkingMsBeforeFirstPaint: null,
    doneAt: null,
    endKind: null,
    ...overrides,
  };
  assistant.telemetry.turns.push(rec);
  tab.currentTurnRecord = rec;
  return rec;
}

// Feed one or more recorded frames. Objects are JSON-encoded (the real wire
// format the backend forwards verbatim); raw strings pass through untouched so
// we can exercise the non-JSON dribble path.
function feed(tab: Tab, frames: Array<Record<string, unknown> | string>) {
  for (const f of frames) {
    tab.onStream(typeof f === "string" ? f : JSON.stringify(f));
    pumpRaf(); // let queued text paint before the next frame, like real rAF
  }
}

// Recorded-frame builders mirroring the Claude CLI's stream-json envelopes.
const sysModel = (model: string) => ({ type: "system", subtype: "init", model });
const textDelta = (text: string, index = 0) => ({
  type: "stream_event",
  event: { type: "content_block_delta", index, delta: { type: "text_delta", text } },
});
const thinkStart = (index: number) => ({
  type: "stream_event",
  event: { type: "content_block_start", index, content_block: { type: "thinking" } },
});
const thinkDelta = (thinking: string, index: number) => ({
  type: "stream_event",
  event: { type: "content_block_delta", index, delta: { type: "thinking_delta", thinking } },
});
const sigDelta = (index: number) => ({
  type: "stream_event",
  event: { type: "content_block_delta", index, delta: { type: "signature_delta", signature: "sig" } },
});
const blockStop = (index: number) => ({
  type: "stream_event",
  event: { type: "content_block_stop", index },
});
const toolUseEnv = (id: string, name: string, input: Record<string, unknown> = {}) => ({
  type: "assistant",
  message: { content: [{ type: "tool_use", id, name, input }] },
});
const toolResultEnv = (toolUseId: string, content: unknown, isError = false) => ({
  type: "user",
  message: { content: [{ type: "tool_result", tool_use_id: toolUseId, content, is_error: isError }] },
});
const assistantUsageEnv = (usage: Record<string, number>) => ({
  type: "assistant",
  message: { content: [], usage },
});
const resultEnv = (over: Record<string, unknown> = {}) => ({
  type: "result",
  subtype: "success",
  ...over,
});

let tabSeq = 0;
function freshTab(): Tab {
  const id = `pb-${++tabSeq}`;
  return assistant.ensureTab(id, id);
}

const textBlocks = (tab: Tab, msgId: string) =>
  tab.messages.find((m) => m.id === msgId)?.blocks ?? [];

beforeEach(() => {
  assistant.telemetry.turns = [];
  assistant.telemetry.events = [];
});

// ── Text streaming ───────────────────────────────────────────────────────────
describe("playback — text deltas", () => {
  it("coalesces text_delta events into a single text block + counts deltas", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [sysModel("claude-sonnet-4-6"), textDelta("Hello"), textDelta(", "), textDelta("world")]);

    const blocks = textBlocks(tab, id);
    expect(blocks).toEqual([{ type: "text", text: "Hello, world" }]);
    expect(rec.deltaCount).toBe(3);
    expect(rec.firstPaintAt).not.toBeNull();
    expect(rec.streamEventCount).toBe(3);
  });

  it("treats a non-JSON line as raw text when streaming (CLI dribble fallback)", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, ["plain text not json"]);

    expect(textBlocks(tab, id)).toEqual([{ type: "text", text: "plain text not json" }]);
    expect(rec.deltaCount).toBe(1);
  });
});

// ── Tool lifecycle ───────────────────────────────────────────────────────────
describe("playback — tool_use → tool_result lifecycle", () => {
  it("appends a pending tool block then fills it on the matching tool_result", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [toolUseEnv("tu-1", "Read", { file_path: "/a.ts" })]);

    let tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ type: "tool", id: "tu-1", name: "Read", status: "pending", result: null });
    expect(rec.toolUses).toHaveLength(1);
    expect(rec.toolUses[0]).toMatchObject({ name: "Read", id: "tu-1", inputPreview: "/a.ts", completedAt: null });

    feed(tab, [toolResultEnv("tu-1", "file contents here", false)]);
    tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ id: "tu-1", status: "done", result: "file contents here", isError: false });
    expect(rec.toolUses[0].completedAt).not.toBeNull();
    expect(rec.toolUses[0].isError).toBe(false);
  });

  it("marks an errored tool_result as status=error and flattens array content", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      toolUseEnv("tu-2", "Bash", { command: "exit 1" }),
      toolResultEnv("tu-2", [{ type: "text", text: "boom" }], true),
    ]);
    const tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ id: "tu-2", status: "error", result: "boom", isError: true });
    expect(rec.toolUses[0].isError).toBe(true);
  });

  it("de-dupes a tool_use re-sent with the same id (stream + envelope overlap)", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [toolUseEnv("dup", "Grep", { pattern: "x" }), toolUseEnv("dup", "Grep", { pattern: "x" })]);
    const tools = textBlocks(tab, id).filter((b) => b.type === "tool");
    expect(tools).toHaveLength(1);
    expect(rec.toolUses).toHaveLength(1);
  });

  it("keeps text before and after a tool as separate, ordered blocks (no fusion / reorder)", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      textDelta("before the edit."),
      toolUseEnv("tu-mid", "Edit", { file_path: "/a.ts" }),
      toolResultEnv("tu-mid", "ok", false),
      textDelta("after the edit."),
    ]);
    const kinds = textBlocks(tab, id).map((b) => b.type);
    expect(kinds).toEqual(["text", "tool", "text"]);
    const texts = textBlocks(tab, id)
      .filter((b) => b.type === "text")
      .map((b) => (b as { text: string }).text);
    expect(texts).toEqual(["before the edit.", "after the edit."]);
  });
});

// ── Sub-agent live routing ───────────────────────────────────────────────────
// The CLI multiplexes Task/Agent sub-agent output into the same stream, tagging
// each nested frame with parent_tool_use_id = the spawning tool_use id. Those
// frames must land in the spawn's own `blocks` sub-transcript (the live dock),
// never in the main bubble.
const agentSpawnEnv = (id: string, subagent_type: string, description: string) => ({
  type: "assistant",
  message: { content: [{ type: "tool_use", id, name: "Task", input: { subagent_type, description } }] },
});
const nestedTextEnv = (parentId: string, text: string) => ({
  type: "assistant",
  parent_tool_use_id: parentId,
  message: { content: [{ type: "text", text }] },
});
const nestedToolUseEnv = (parentId: string, id: string, name: string, input: Record<string, unknown> = {}) => ({
  type: "assistant",
  parent_tool_use_id: parentId,
  message: { content: [{ type: "tool_use", id, name, input }] },
});
const nestedToolResultEnv = (parentId: string, toolUseId: string, content: unknown, isError = false) => ({
  type: "user",
  parent_tool_use_id: parentId,
  message: { content: [{ type: "tool_result", tool_use_id: toolUseId, content, is_error: isError }] },
});
// The CLI's per-sub-agent terminal envelope: a `result` carrying the spawn's
// parent_tool_use_id. This is the live "this agent finished" signal.
const nestedResultEnv = (parentId: string, subtype = "success") => ({
  type: "result",
  subtype,
  parent_tool_use_id: parentId,
});

describe("playback — sub-agent live routing", () => {
  it("diverts parent-tagged frames into the spawn's sub-transcript, not the main bubble", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      agentSpawnEnv("task-1", "recon", "map the files"),
      nestedTextEnv("task-1", "scanning"),
      nestedToolUseEnv("task-1", "n-tu-1", "Grep", { pattern: "foo" }),
      nestedToolResultEnv("task-1", "n-tu-1", "3 matches", false),
      nestedTextEnv("task-1", "found 3"),
    ]);

    // Post-inline-card redesign (3b3740c): the Task tool_use itself renders as a
    // first-class INLINE card in the main bubble (one "tool" block named "Task"),
    // while the NESTED sub-agent frames (scanning / Grep / found 3) still divert to
    // agentSpawns[i].blocks and must NOT leak into the main bubble. So the bubble
    // holds exactly the Task card — nothing from the sub-transcript.
    const bubbleBlocks = textBlocks(tab, id).filter((b) => b.type === "tool" || b.type === "text");
    expect(bubbleBlocks).toHaveLength(1);
    expect(bubbleBlocks[0]).toMatchObject({ type: "tool", name: "Task" });

    const agent = tab.agentSpawns.find((a) => a.id === "task-1")!;
    expect(agent).toMatchObject({ subagentType: "recon", description: "map the files", completedAt: null });
    expect(agent.blocks.map((b) => b.type)).toEqual(["text", "tool", "text"]);
    expect(agent.blocks[0]).toEqual({ type: "text", text: "scanning" });
    expect(agent.blocks[1]).toMatchObject({ type: "tool", id: "n-tu-1", name: "Grep", status: "done", result: "3 matches" });
    expect(agent.blocks[2]).toEqual({ type: "text", text: "found 3" });
  });

  it("marks the spawn done when the top-level Task tool_result arrives (parent null)", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      agentSpawnEnv("task-2", "scout", "research"),
      nestedTextEnv("task-2", "working"),
      toolResultEnv("task-2", "summary text", false), // top-level → completes spawn
    ]);
    const agent = tab.agentSpawns.find((a) => a.id === "task-2")!;
    expect(agent.completedAt).not.toBeNull();
    expect(agent.isError).toBe(false);
    expect(agent.blocks.map((b) => b.type)).toEqual(["text"]); // sub-transcript intact
  });

  it("marks the spawn done on its own per-agent result envelope (no top-level Task tool_result)", () => {
    // The reported bug: recon's frames all land (incl. its result) but the
    // top-level Task tool_result never arrives, so the spawn stays running
    // forever. The per-agent `result` envelope must flip it done LIVE.
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      agentSpawnEnv("task-r", "recon", "find stale refs"),
      nestedTextEnv("task-r", "scanning"),
      nestedResultEnv("task-r"), // sub-agent's own terminal signal — no top-level tool_result
    ]);
    const agent = tab.agentSpawns.find((a) => a.id === "task-r")!;
    expect(agent.completedAt).not.toBeNull();
    expect(agent.isError).toBe(false);
    expect(agent.blocks.map((b) => b.type)).toEqual(["text"]); // sub-transcript intact
  });

  it("a late top-level tool_result does NOT flip an already-settled spawn (no done→error flicker)", () => {
    // Ordering edge: the per-agent result envelope settles the spawn as success
    // FIRST; a late top-level Task tool_result carrying is_error:true must not
    // re-close it and flip the clean ✓ to error. fillToolResult is idempotent on
    // an already-closed spawn (mirrors markSpawnDone).
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      agentSpawnEnv("task-o", "recon", "ordering"),
      nestedResultEnv("task-o"), // per-agent result FIRST → done, isError=false
    ]);
    expect(tab.agentSpawns.find((a) => a.id === "task-o")!.isError).toBe(false);
    feed(tab, [toolResultEnv("task-o", "late error", true)]); // late top-level error
    // Still success — the first terminal signal wins; no post-hoc flip.
    expect(tab.agentSpawns.find((a) => a.id === "task-o")!.isError).toBe(false);
  });

  it("an error result envelope marks the spawn done + errored", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      agentSpawnEnv("task-e", "recon", "boom"),
      nestedResultEnv("task-e", "error_during_execution"),
    ]);
    const agent = tab.agentSpawns.find((a) => a.id === "task-e")!;
    expect(agent.completedAt).not.toBeNull();
    expect(agent.isError).toBe(true);
  });

  it("turn-end sweep closes any spawn whose terminal signal never arrived", () => {
    // Safety net: even if neither the top-level tool_result NOR the per-agent
    // result envelope lands, onDone must close the orphan so the dock never
    // spins forever and the model never reads a finished spawn as live.
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      agentSpawnEnv("task-x", "recon", "orphaned"),
      nestedTextEnv("task-x", "working"),
    ]);
    expect(tab.agentSpawns.find((a) => a.id === "task-x")!.completedAt).toBeNull(); // still running mid-turn
    tab.onDone();
    const swept = tab.agentSpawns.find((a) => a.id === "task-x")!;
    expect(swept.completedAt).not.toBeNull(); // swept closed
    // A spawn reaching the sweep still-open is abnormal (its result envelope was
    // lost / it was interrupted) — mark it errored, not a clean ✓, so an aborted
    // turn doesn't show green-check sub-agents. Mirrors main-turn tool→error.
    expect(swept.isError).toBe(true);
  });

  it("the error-terminal path (backend Stalled) also sweeps a spinning spawn closed", () => {
    // The 40-min-hang fix: when a wedged sub-agent trips the backend watchdog,
    // the turn ends via ERROR_EVENT → tab.onError, NOT onDone. That path must
    // sweep the still-spinning spawn the same way, or the dock keeps spinning
    // even after the backend killed the wedged child.
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      agentSpawnEnv("task-w", "recon", "wedged then watchdog-killed"),
      nestedTextEnv("task-w", "Starting up"),
    ]);
    expect(tab.agentSpawns.find((a) => a.id === "task-w")!.completedAt).toBeNull(); // mid-turn, spinning
    tab.onError("Claude stopped responding — no output, so Rift ended the turn.");
    expect(tab.agentSpawns.find((a) => a.id === "task-w")!.completedAt).not.toBeNull(); // swept on error too
  });

  it("drops a frame whose parent matches no tracked spawn — no phantom agent, no main leak", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [nestedTextEnv("ghost", "orphan")]);
    expect(tab.agentSpawns.find((a) => a.id === "ghost")).toBeUndefined();
    expect(textBlocks(tab, id)).toEqual([]); // not leaked into the main bubble
  });
});

// ── Thinking blocks ──────────────────────────────────────────────────────────
describe("playback — thinking blocks", () => {
  it("opens, streams, signs, and closes a thinking block", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      thinkStart(0),
      thinkDelta("let me ", 0),
      thinkDelta("reason", 0),
      sigDelta(0),
      blockStop(0),
    ]);
    const think = textBlocks(tab, id).find((b) => b.type === "thinking");
    expect(think).toMatchObject({ type: "thinking", text: "let me reason", hasSignature: true, status: "done" });
    expect((think as { durationMs: number | null }).durationMs).not.toBeNull();
  });
});

// ── Usage / cost / model ─────────────────────────────────────────────────────
describe("playback — usage, cost, model attribution", () => {
  it("assistant-envelope usage drives the ctx pill; result usage feeds session totals", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    feed(tab, [
      assistantUsageEnv({
        input_tokens: 100,
        cache_read_input_tokens: 50_000,
        cache_creation_input_tokens: 2_000,
        output_tokens: 80,
      }),
      resultEnv({
        total_cost_usd: 0.012,
        usage: {
          input_tokens: 1_000,
          cache_read_input_tokens: 900_000,
          cache_creation_input_tokens: 10_000,
          output_tokens: 500,
        },
      }),
    ]);

    // Point-in-time window occupancy = last assistant envelope, NOT the cumulative result.
    expect(tab.lastTurnUsage).toEqual({ input: 100, output: 80, cacheRead: 50_000, cacheCreate: 2_000 });
    expect(assistant.ctxTokensFor(tab as never)).toBe(52_100); // 100 + 50_000 + 2_000
    // Session totals come off the result event.
    expect(tab.sessionUsage.totalCacheRead).toBe(900_000);
    expect(tab.sessionUsage.turns).toBe(1);
    // Cost + telemetry capture.
    expect(tab.totalCostUsd).toBeCloseTo(0.012, 6);
    expect(rec.costUsd).toBeCloseTo(0.012, 6);
    expect(rec.resultUsage).toEqual({ input: 1_000, output: 500, cacheRead: 900_000, cacheCreate: 10_000 });
    expect(rec.envelopeUsage).toEqual({ input: 100, output: 80, cacheRead: 50_000, cacheCreate: 2_000 });
  });

  it("system envelope sets lastModelId + the turn record modelId", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    feed(tab, [sysModel("claude-opus-4-8")]);
    expect(tab.lastModelId).toBe("claude-opus-4-8");
    expect(rec.modelId).toBe("claude-opus-4-8");
  });

  it("result modelUsage sets reportedCtxWindow — turn-model match wins, keys [1m]-normalized", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      sysModel("claude-sonnet-5[1m]"),
      textDelta("ok"),
      resultEnv({
        // modelUsage is session-CUMULATIVE — after a mid-chat switch both
        // models appear; the turn's own (sonnet) entry must win over the
        // larger opus sibling, and both key + lastModelId normalize [1m].
        modelUsage: {
          "claude-opus-4-8": { contextWindow: 1_000_000 },
          "claude-sonnet-5[1m]": { contextWindow: 200_000 },
        },
      }),
    ]);
    expect(tab.reportedCtxWindow).toEqual({ model: "claude-sonnet-5", window: 200_000 });
    // No fast confirmation on this result → no stamp.
    expect(tab.messages.find((m) => m.id === id)?.fast).toBeUndefined();
  });

  it("a CLI-confirmed fast turn stamps message.fast (honest badge, result-time only)", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      sysModel("claude-opus-4-8"),
      textDelta("ok"),
      resultEnv({ fast_mode_state: "on", usage: { speed: "fast" } }),
    ]);
    expect(tab.messages.find((m) => m.id === id)?.fast).toBe(true);
  });

  it("surfaces a known error result.subtype as a plain-English lastError", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [resultEnv({ subtype: "error_max_turns" })]);
    expect(tab.lastError).toBe("The run hit its maximum number of turns and stopped.");
  });

  it("surfaces model_context_window_exceeded with a recovery hint", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [resultEnv({ subtype: "model_context_window_exceeded" })]);
    expect(tab.lastError).toContain("fresh chat");
  });

  it("surfaces error_max_budget_usd (Rift's own --max-budget-usd cap) as lastError", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [resultEnv({ subtype: "error_max_budget_usd", is_error: true })]);
    expect(tab.lastError).toContain("spend cap");
  });

  it("falls back to the CLI's errors[] text on an unknown error subtype", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      resultEnv({ subtype: "error_totally_new", is_error: true, errors: ["Something specific broke"] }),
    ]);
    expect(tab.lastError).toBe("Something specific broke");
  });

  it("keeps a non-error unknown subtype silent (no false alarm)", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [resultEnv({ subtype: "some_benign_marker" })]);
    expect(tab.lastError).toBeNull();
  });

  it("stays silent when only user-level MCP servers are unhealthy (rift is fine)", () => {
    const tab = freshTab();
    beginTurn(tab);
    const pushMock = vi.mocked(toast.push);
    pushMock.mockClear();
    feed(tab, [{
      type: "system",
      subtype: "init",
      mcp_servers: [
        { name: "rift", status: "connected" },
        { name: "claude.ai Google Calendar", status: "needs-auth" },
        { name: "claude.ai Stripe", status: "failed" },
      ],
    }]);
    expect(pushMock).not.toHaveBeenCalled();
  });

  it("toasts once (latched) when the init frame reports a failed rift MCP server", () => {
    const tab = freshTab();
    beginTurn(tab);
    const pushMock = vi.mocked(toast.push);
    pushMock.mockClear();
    const initEnv = {
      type: "system",
      subtype: "init",
      mcp_servers: [{ name: "rift", status: "failed" }],
    };
    feed(tab, [initEnv]);
    expect(pushMock).toHaveBeenCalledTimes(1);
    expect(pushMock).toHaveBeenCalledWith(
      expect.objectContaining({ severity: "danger", title: expect.stringContaining("Rift workspace tools") }),
    );
    // Latched once per app session — a repeat failure doesn't re-toast.
    feed(tab, [initEnv]);
    expect(pushMock).toHaveBeenCalledTimes(1);
  });

  it("ignores healthy/pending/disabled MCP statuses on init", () => {
    const tab = freshTab();
    beginTurn(tab);
    const pushMock = vi.mocked(toast.push);
    pushMock.mockClear();
    feed(tab, [{
      type: "system",
      subtype: "init",
      mcp_servers: [
        { name: "rift", status: "connected" },
        { name: "user-server", status: "pending" },
        { name: "other", status: "disabled" },
      ],
    }]);
    expect(pushMock).not.toHaveBeenCalled();
  });

  it("stores the init frame's MCP server list on the tab for /mcp", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [{
      type: "system",
      subtype: "init",
      mcp_servers: [
        { name: "rift", status: "connected" },
        { name: "claude.ai Google Calendar", status: "needs-auth" },
      ],
    }]);
    expect(tab.mcpServers).toEqual([
      { name: "rift", status: "connected" },
      { name: "claude.ai Google Calendar", status: "needs-auth" },
    ]);
  });

  it("flags a max_tokens stop_reason on the streaming message", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      { type: "stream_event", event: { type: "message_delta", delta: { stop_reason: "max_tokens" } } },
    ]);
    const last = tab.messages[tab.messages.length - 1];
    expect(last?.stopReason).toBe("max_tokens");
  });
});

// ── #98 backward-compat: init slash_commands + unknown content blocks ────────
describe("playback — CLI backward-compat surfaces", () => {
  it("captures the init frame's slash_commands into the cliCommands store", async () => {
    const { cliCommands } = await import("./assistant/cliCommands.svelte.js");
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [{
      type: "system",
      subtype: "init",
      slash_commands: ["/compact", "code-review", "/compact", 42, "bad name"],
    }]);
    expect(cliCommands.names).toEqual(["code-review", "compact"]);
    // Older CLI omitting the field leaves the last-known set standing.
    feed(tab, [{ type: "system", subtype: "init" }]);
    expect(cliCommands.names).toEqual(["code-review", "compact"]);
  });

  it("renders one dedup'd marker for unknown assistant content block types", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const tab = freshTab();
      beginTurn(tab);
      feed(tab, [{
        type: "assistant",
        message: {
          content: [
            { type: "citation", cited_text: "a" },
            { type: "tool_use", id: "tu-98", name: "Read", input: {} },
            { type: "citation", cited_text: "b" },
          ],
        },
      }]);
      const last = tab.messages[tab.messages.length - 1];
      const unknowns = last.blocks.filter((b) => b.type === "unknown");
      expect(unknowns).toHaveLength(1);
      expect(unknowns[0]).toMatchObject({ type: "unknown", blockType: "citation" });
      // Known blocks still land untouched alongside the marker.
      expect(last.blocks.some((b) => b.type === "tool" && b.name === "Read")).toBe(true);
      expect(warn).toHaveBeenCalled();
    } finally {
      warn.mockRestore();
    }
  });
});

// ── Live output-token counter ────────────────────────────────────────────────
describe("playback — live output-token counter", () => {
  it("resets at turn start, climbs from streamed chars, snaps exact per message", () => {
    const tab = freshTab();
    beginTurn(tab);
    expect(tab.liveOutputTokens).toBe(0);

    // No mid-stream usage from the CLI, so the count climbs as a chars/4
    // estimate over the in-flight message.
    feed(tab, [textDelta("y".repeat(80))]);
    expect(tab.liveOutputTokens).toBe(20); // round(80/4)
    feed(tab, [textDelta("y".repeat(40))]);
    expect(tab.liveOutputTokens).toBe(30); // round(120/4)

    // The message's real usage envelope snaps the count exact + clears the estimate.
    feed(tab, [assistantUsageEnv({ input_tokens: 1, output_tokens: 95 })]);
    expect(tab.liveOutputTokens).toBe(95);

    // A second message's estimate rides on top of the banked exact total.
    feed(tab, [textDelta("y".repeat(40))]);
    expect(tab.liveOutputTokens).toBe(105); // 95 + round(40/4)
    feed(tab, [assistantUsageEnv({ input_tokens: 1, output_tokens: 30 })]);
    expect(tab.liveOutputTokens).toBe(125); // 95 + 30
  });

  it("counts thinking deltas toward the live estimate", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [thinkStart(0), thinkDelta("z".repeat(60), 0)]);
    expect(tab.liveOutputTokens).toBe(15); // round(60/4)
  });

  it("never rolls backward when the real usage lands below the estimate", () => {
    const tab = freshTab();
    beginTurn(tab);
    // chars/4 overshoots what the usage envelope will report — the visible
    // meter must hold its high-water mark, not tick down (2026-07-15 recording:
    // 429→409, 442→434 read as a broken counter).
    feed(tab, [textDelta("y".repeat(400))]);
    expect(tab.liveOutputTokens).toBe(100); // round(400/4)
    feed(tab, [assistantUsageEnv({ input_tokens: 1, output_tokens: 60 })]);
    expect(tab.liveOutputTokens).toBe(100); // clamped — not 60
    // The next message's estimate still rides on the REAL banked total (60),
    // so the meter resumes climbing only once it passes the high-water mark.
    feed(tab, [textDelta("y".repeat(200))]);
    expect(tab.liveOutputTokens).toBe(110); // 60 + round(200/4)
    // A fresh turn re-arms the clamp from zero.
    beginTurn(tab);
    expect(tab.liveOutputTokens).toBe(0);
  });
});

// ── onDone finalization ──────────────────────────────────────────────────────
describe("playback — onDone finalization", () => {
  it("finalizes a successful turn: streaming off, endKind success, doneAt set", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    feed(tab, [textDelta("done")]);
    tab.onDone();
    expect(tab.streaming).toBe(false);
    expect(tab.currentTurnRecord).toBeNull();
    expect(rec.endKind).toBe("success");
    expect(rec.doneAt).not.toBeNull();
    expect(rec.blankTurn).toBe(false);
  });

  it("flags a blank turn (no text, no tools) as endKind error with a fingerprint message", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    feed(tab, [resultEnv()]); // a result frame but zero text + zero tools
    tab.onDone();
    expect(rec.blankTurn).toBe(true);
    expect(rec.endKind).toBe("error");
    expect(tab.lastError).toMatch(/Blank response/);
  });

  it("drops the CLI's no-op resume turn (No response requested.) without erroring", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      { type: "assistant", message: { content: [{ type: "text", text: "No response requested." }] } },
      resultEnv(),
    ]);
    tab.onDone();
    expect(tab.messages.find((m) => m.id === id)).toBeUndefined();
    expect(tab.lastError).toBeNull();
    expect(rec.blankTurn).toBe(false);
    expect(rec.endKind).toBe("success");
  });

  it("drops a fully-suppressed no-op turn via the replayed CLI continue marker", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    // The canned reply was stop-sequence-suppressed — the stream only carries
    // the CLI's own injected user turn plus the result frame.
    feed(tab, [
      { type: "user", message: { content: "Continue from where you left off." } },
      resultEnv(),
    ]);
    tab.onDone();
    expect(tab.messages.find((m) => m.id === id)).toBeUndefined();
    expect(tab.lastError).toBeNull();
    expect(rec.blankTurn).toBe(false);
  });

  it("falls back to the assistant envelope text when no stream deltas arrived", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    // assistant envelope carries a text block but NO content_block_delta events.
    feed(tab, [{ type: "assistant", message: { content: [{ type: "text", text: "envelope-only" }] } }]);
    expect(textBlocks(tab, id)).toEqual([]); // nothing painted yet — buffered
    tab.onDone();
    expect(textBlocks(tab, id)).toEqual([{ type: "text", text: "envelope-only" }]);
    expect(rec.envelopeFallback).toBe(true);
    expect(rec.blankTurn).toBe(false);
  });

  it("a tool-only turn (no text) is NOT blank", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    feed(tab, [toolUseEnv("t", "Read", { file_path: "/x" }), toolResultEnv("t", "ok")]);
    tab.onDone();
    expect(rec.blankTurn).toBe(false);
    expect(rec.endKind).toBe("success");
  });

  it("onDone is a no-op when not streaming", () => {
    const tab = freshTab();
    tab.streaming = false;
    tab.onDone();
    expect(tab.currentTurnRecord).toBeNull();
  });
});

// ── onError ──────────────────────────────────────────────────────────────────
describe("playback — onError", () => {
  it("sets lastError, stops streaming, drops the empty placeholder, finalizes the record", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    tab.onError("API 401");
    expect(tab.lastError).toBe("API 401");
    expect(tab.streaming).toBe(false);
    expect(tab.messages.find((m) => m.id === id)).toBeUndefined(); // empty placeholder removed
    expect(rec.endKind).toBe("error");
    expect(rec.errorMsg).toBe("API 401");
    expect(tab.currentTurnRecord).toBeNull();
  });

  it("keeps a placeholder that already painted content on error", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [textDelta("partial")]);
    tab.onError("stream broke");
    expect(tab.messages.find((m) => m.id === id)).toBeDefined(); // had blocks → kept
  });
});

// ── End-to-end recorded conversation ─────────────────────────────────────────
describe("playback — full recorded turn", () => {
  it("replays a realistic system→think→text→tool→text→result sequence", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      sysModel("claude-sonnet-4-6"),
      thinkStart(0),
      thinkDelta("plan the read", 0),
      sigDelta(0),
      blockStop(0),
      textDelta("Reading the file.\n"),
      toolUseEnv("tu-9", "Read", { file_path: "/src/main.rs" }),
      toolResultEnv("tu-9", "fn main() {}", false),
      textDelta("It is an empty main."),
      assistantUsageEnv({ input_tokens: 200, cache_read_input_tokens: 10_000, cache_creation_input_tokens: 0, output_tokens: 40 }),
      resultEnv({ total_cost_usd: 0.003, usage: { input_tokens: 200, output_tokens: 40, cache_read_input_tokens: 10_000, cache_creation_input_tokens: 0 } }),
    ]);
    tab.onDone();

    const blocks = textBlocks(tab, id);
    const kinds = blocks.map((b) => b.type);
    expect(kinds).toEqual(["thinking", "text", "tool", "text"]);
    expect((blocks[1] as { text: string }).text).toBe("Reading the file.\n");
    expect((blocks[3] as { text: string }).text).toBe("It is an empty main.");
    expect(blocks[2]).toMatchObject({ id: "tu-9", status: "done", result: "fn main() {}" });

    expect(rec.endKind).toBe("success");
    expect(rec.modelId).toBe("claude-sonnet-4-6");
    expect(rec.toolUses).toHaveLength(1);
    expect(rec.thinkingBlocks).toHaveLength(1);
    expect(rec.costUsd).toBeCloseTo(0.003, 6);
    expect(rec.blankTurn).toBe(false);
    expect(assistant.telemetry.turns).toContain(rec);
  });
});

// ── Send / queue orchestrator ────────────────────────────────────────────────
// These drive the REAL send() entry point with a mocked backend (invoke), so the
// turn-init, auth gate, queue-while-streaming, and queue drain on completion
// paths are all exercised end-to-end — not the hand-rolled beginTurn setup
// the stream-pump tests above use.

let convoSeq = 0;
function readyStore(): { tab: Tab; convoId: string } {
  const convoId = `send-${++convoSeq}`;
  assistant.auth = { pill: "green" } as never; // pass the send() auth chokepoint
  assistant.lastNotice = null;
  assistant.currentConvoId = convoId; // activeTab keys off this
  const tab = assistant.ensureTab(convoId, convoId);
  return { tab, convoId };
}

// Let queueMicrotask-scheduled drains + the async send() body run to the point
// where it flips streaming on (everything up to the awaited invoke is sync).
const settle = () => new Promise((r) => setTimeout(r, 0));

// Mid-stream sends now steer-first (assistant_steer into the live turn) and only
// queue when the backend refuses. The bare vi.fn() mock resolves everything, so
// steering "succeeds" by default — queue-path tests must refuse it explicitly.
const refuseSteer = () =>
  mockInvoke.mockImplementation(((cmd: string) =>
    cmd === "assistant_steer"
      ? Promise.reject(new Error("no turn in progress"))
      : Promise.resolve(undefined)) as never);

describe("playback — send() turn initialization", () => {
  it("builds the user + assistant messages, the turn record, and invokes the backend", async () => {
    const { tab } = readyStore();
    mockInvoke.mockClear();
    await assistant.send("hello world");

    expect(tab.streaming).toBe(true);
    expect(tab.messages.map((m) => m.role)).toEqual(["user", "assistant"]);
    expect(tab.messages[0].blocks).toEqual([{ type: "text", text: "hello world" }]);
    expect(tab.messages[1].blocks).toEqual([]); // streaming placeholder
    expect(tab.streamingMsgId).toBe(tab.messages[1].id);

    expect(assistant.telemetry.turns).toHaveLength(1);
    const rec = assistant.telemetry.turns[0];
    expect(rec).toMatchObject({ promptLen: 11, promptPreview: "hello world", isFirstTurn: true });
    expect(rec.endKind).toBeNull(); // still in flight

    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_send",
      expect.objectContaining({ prompt: "hello world", isFirstTurn: true }),
    );
  });

  it("blocks the send when auth isn't green/yellow — no turn, surfaces a notice", async () => {
    assistant.currentConvoId = null; // no active tab
    assistant.auth = { pill: "red", summary: "Claude isn't set up" } as never;
    mockInvoke.mockClear();
    vi.mocked(notify.danger).mockClear();
    const before = assistant.telemetry.turns.length;

    await assistant.send("hi");

    expect(assistant.telemetry.turns.length).toBe(before);
    expect(notify.danger).toHaveBeenCalledWith("Claude isn't set up", expect.anything());
    expect(mockInvoke).not.toHaveBeenCalledWith("assistant_send", expect.anything());
  });

  it("drops an empty prompt with no attachments", async () => {
    readyStore();
    mockInvoke.mockClear();
    await assistant.send("   ");
    expect(mockInvoke).not.toHaveBeenCalledWith("assistant_send", expect.anything());
    expect(assistant.telemetry.turns).toHaveLength(0);
  });
});

describe("playback — queue while streaming, drain on completion", () => {
  beforeEach(refuseSteer);
  afterEach(() => mockInvoke.mockReset());

  it("queues a second send mid-stream, then fires it when the first turn completes", async () => {
    const { tab } = readyStore();
    await assistant.send("first");
    expect(tab.streaming).toBe(true);
    expect(assistant.telemetry.turns).toHaveLength(1);

    // Second send while the first is streaming → queued, not dropped, no new turn.
    await assistant.send("second");
    expect(tab.queue.map((q) => q.text)).toEqual(["second"]);
    expect(assistant.telemetry.turns).toHaveLength(1);

    // Finish the first turn with real content (a blank turn sets lastError,
    // which intentionally blocks the drain) → handleTurnComplete drains the
    // queue on the next microtask.
    feed(tab, [textDelta("first reply")]);
    tab.onDone();
    await settle();

    expect(tab.queue).toHaveLength(0);
    expect(tab.streaming).toBe(true); // second turn now in flight
    expect(assistant.telemetry.turns).toHaveLength(2);
    expect(assistant.telemetry.turns[1]).toMatchObject({ promptPreview: "second", isFirstTurn: false });
  });

  it("carries a queued message's image attachment through the drain (regression: queued image was dropped)", async () => {
    const { tab } = readyStore();
    await assistant.send("first");
    expect(tab.streaming).toBe(true);

    // Stage an image, then send while streaming → the message queues AND the
    // queue item snapshots the image (the composer is cleared on enqueue).
    tab.attachments = [{ id: "img-1", mime: "image/png", dataBase64: "QUJD", sizeBytes: 3 }];
    await assistant.send("with image");
    expect(tab.queue).toHaveLength(1);
    expect(tab.queue[0].images).toEqual([
      { id: "img-1", mime: "image/png", dataBase64: "QUJD", sizeBytes: 3 },
    ]);
    expect(tab.attachments).toEqual([]); // composer cleared on enqueue, no double-send

    // Finish the first turn → drain. The drained turn's user message must carry
    // the image as an image block (was previously dropped — only text survived).
    feed(tab, [textDelta("first reply")]);
    tab.onDone();
    await settle();

    expect(tab.queue).toHaveLength(0);
    const drainedUser = tab.messages.findLast((m) => m.role === "user");
    expect(drainedUser?.blocks).toContainEqual({
      type: "image", mime: "image/png", dataBase64: "QUJD", sizeBytes: 3,
    });
    expect(drainedUser?.blocks).toContainEqual({ type: "text", text: "with image" });
  });

  it("a drain leaves the composer's staged attachments alone (regression: drain restored onto shared tab state)", async () => {
    const { tab } = readyStore();
    await assistant.send("first");

    // Queue an image-bearing message mid-stream…
    tab.attachments = [{ id: "img-q", mime: "image/png", dataBase64: "UUVE", sizeBytes: 3 }];
    await assistant.send("queued with image");
    expect(tab.queue).toHaveLength(1);

    // …then stage a NEW composer attachment for the next message the user is
    // composing. The old drain restored the queued snapshot onto tab.attachments,
    // clobbering this staging (and a concurrent send could steal the queued one).
    tab.attachments = [{ id: "img-mine", mime: "image/jpeg", dataBase64: "TUlORQ==", sizeBytes: 4 }];

    feed(tab, [textDelta("reply")]);
    tab.onDone();
    await settle();

    // Drained turn carries the QUEUED image, not the composer's.
    const drainedUser = tab.messages.findLast((m) => m.role === "user");
    expect(drainedUser?.blocks).toContainEqual({
      type: "image", mime: "image/png", dataBase64: "UUVE", sizeBytes: 3,
    });
    expect(drainedUser?.blocks).not.toContainEqual(
      expect.objectContaining({ dataBase64: "TUlORQ==" }),
    );
    // The user's composer staging survives the drain untouched.
    expect(tab.attachments).toEqual([
      { id: "img-mine", mime: "image/jpeg", dataBase64: "TUlORQ==", sizeBytes: 4 },
    ]);
  });

  it("a payload send that finds the tab busy re-parks at the queue FRONT (order preserved)", async () => {
    const { tab, convoId } = readyStore();
    await assistant.send("first");
    expect(tab.streaming).toBe(true);
    await assistant.send("second"); // parked normally → back of queue
    expect(tab.queue.map((q) => q.text)).toEqual(["second"]);

    // A drained-head re-entry (payload + requeueFront) must park AHEAD of it,
    // its attachments riding the item — never staged on the composer.
    await sendDirect(assistant, "head", convoId, {
      payload: { images: [{ id: "i", mime: "image/png", dataBase64: "QQ==", sizeBytes: 1 }], textFiles: [] },
      requeueFront: true,
    });
    expect(tab.queue.map((q) => q.text)).toEqual(["head", "second"]);
    expect(tab.queue[0].images).toHaveLength(1);
    expect(tab.attachments).toEqual([]);
  });
});

describe("playback — steer (mid-turn send injects into the live turn)", () => {
  it("steers a mid-stream send: no queue, no new turn, marker in the streaming bubble", async () => {
    const { tab } = readyStore();
    await assistant.send("first");
    expect(tab.streaming).toBe(true);
    mockInvoke.mockClear();

    await assistant.send("also include PINEAPPLE");

    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_steer",
      expect.objectContaining({ sessionId: tab.cliSessionId, prompt: "also include PINEAPPLE" }),
    );
    expect(tab.queue).toHaveLength(0);
    expect(assistant.telemetry.turns).toHaveLength(1); // absorbed into the live turn
    const streamMsg = tab.messages[tab.messages.length - 1];
    expect(streamMsg.role).toBe("assistant");
    expect(streamMsg.blocks).toContainEqual(
      expect.objectContaining({ type: "steer", text: "also include PINEAPPLE" }),
    );
  });

  it("falls back to the queue when the backend refuses the steer, and removes the marker", async () => {
    const { tab } = readyStore();
    await assistant.send("first");
    refuseSteer();

    await assistant.send("too late");

    expect(tab.queue.map((q) => q.text)).toEqual(["too late"]);
    const streamMsg = tab.messages[tab.messages.length - 1];
    expect(streamMsg.blocks.filter((b) => b.type === "steer")).toHaveLength(0);
    mockInvoke.mockReset();
  });

  it("skips straight to the queue when the tab has no CLI session yet", async () => {
    const { tab } = readyStore();
    await assistant.send("first");
    tab.cliSessionId = ""; // no session → nothing to steer into
    mockInvoke.mockClear();

    await assistant.send("second");

    expect(tab.queue.map((q) => q.text)).toEqual(["second"]);
    expect(mockInvoke).not.toHaveBeenCalledWith("assistant_steer", expect.anything());
  });
});

describe("playback — queue (type while streaming → fires after the turn)", () => {
  beforeEach(refuseSteer);
  afterEach(() => mockInvoke.mockReset());

  it("drains the queue head as the next turn, in queue order", async () => {
    const { tab } = readyStore();
    await assistant.send("first");
    feed(tab, [textDelta("reply one")]);

    // Two follow-ups typed while busy → both queue, in order.
    await assistant.send("second");
    await assistant.send("third");
    expect(tab.queue.map((q) => q.text)).toEqual(["second", "third"]);

    tab.onDone();
    await settle();

    // Head ("second") became turn 2; "third" stays queued.
    expect(tab.streaming).toBe(true);
    expect(assistant.telemetry.turns).toHaveLength(2);
    expect(assistant.telemetry.turns[1].promptPreview).toBe("second");
    expect(tab.queue.map((q) => q.text)).toEqual(["third"]);
  });

  it("removeQueued drops a parked chip before it drains", async () => {
    const { tab } = readyStore();
    await assistant.send("go");
    feed(tab, [textDelta("working")]);
    tab.queue = [{ id: "c1", text: "drop me" }, { id: "c2", text: "keep me" }];

    assistant.removeQueued("c1");

    expect(tab.queue.map((q) => q.text)).toEqual(["keep me"]);
  });
});

// Split-pane: two panes, two tabs, two folders. The store hangs everything off
// the focused-pane globals (currentConvoId / activeTab); these guard the bg pane
// against having its turn / queue / persistence routed to the focused pane.
describe("split-pane — background-pane isolation", () => {
  function twoPaneStore() {
    const a = `pane-a-${++convoSeq}`;
    const b = `pane-b-${++convoSeq}`;
    assistant.auth = { pill: "green" } as never;
    const tabA = assistant.ensureTab(a, a);
    const tabB = assistant.ensureTab(b, b);
    assistant.openTabs = [a, b];
    assistant.panes = [{ tabId: a }, { tabId: b }];
    assistant.focusedPaneIdx = 0;
    assistant.currentConvoId = a; // pane A is focused
    return { a, b, tabA, tabB };
  }

  it("drains a queued message on a VISIBLE non-focused pane (was stranded forever)", async () => {
    const { b, tabA, tabB } = twoPaneStore();
    void tabA;
    // Pane B (unfocused but visible) is mid-turn with a queued follow-up.
    tabB.streaming = true;
    tabB.queue = [{ id: "q1", text: "next on B" }];
    mockInvoke.mockClear();

    // B's turn completes while focus is still on A. onTurnComplete is the hook
    // every terminal path fires (wired by the store in wireTab) — it runs the
    // queue drain. Pre-split this drain bailed because tabB !== activeTab.
    tabB.streaming = false;
    tabB.onTurnComplete?.(tabB);
    await settle();

    // The queued message fired as B's next turn — not stranded, not sent into A.
    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_send",
      expect.objectContaining({ sessionId: b, prompt: "next on B" }),
    );
    expect(tabB.queue).toHaveLength(0);
  });

  it("closing a background tab flushes its unsaved messages before dropping it", async () => {
    const { a, b, tabB } = twoPaneStore();
    // Pane B (not active) has an unsaved exchange.
    tabB.messages = [
      { id: "u1", role: "user", blocks: [{ type: "text", text: "hi" }] },
      { id: "a1", role: "assistant", blocks: [{ type: "text", text: "hello" }] },
    ];
    tabB.convoCreatedAt = Date.now();
    assistant.currentConvoId = a; // A stays focused — B is the bg tab being closed
    mockInvoke.mockClear();

    await assistant.closeTab(b);

    // The bg tab's tail was persisted (scheduleSave → assistant_save_conversation)
    // rather than lost when its TabState was dropped.
    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_save_conversation",
      expect.anything(),
    );
  });

  it("a queue drain on a visible bg pane fires into THAT pane without stealing focus (#78)", async () => {
    const { a, b, tabB } = twoPaneStore();
    // Pane B (unfocused, visible) finishes a turn with a queued follow-up. The
    // drain must send into B and leave the user's focus on A — the pre-#78 drain
    // routed through store.send(text, tabId) which called setFocusedPane.
    tabB.streaming = true;
    tabB.queue = [{ id: "q1", text: "drain B" }];
    mockInvoke.mockClear();

    tabB.streaming = false;
    tabB.onTurnComplete?.(tabB);
    await settle();

    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_send",
      expect.objectContaining({ sessionId: b, prompt: "drain B" }),
    );
    // Focus never moved: currentConvoId + focusedPaneIdx still point at A.
    expect(assistant.currentConvoId).toBe(a);
    expect(assistant.focusedPaneIdx).toBe(0);
  });

  it("an explicit-target send writes into the target tab, not the focused one (#78)", async () => {
    const { a, b, tabA, tabB } = twoPaneStore();
    mockInvoke.mockClear();

    // Send addressed to pane B while A is focused (the Retry/Continue path).
    // Both tabs are live in `tabs`, so setFocusedPane's meta-load branch is a
    // no-op; seed `conversations` so its `.some()` lookup doesn't throw in the
    // test harness (the real store always has this array).
    assistant.conversations = [];
    await assistant.send("into B", b);
    await settle();

    // The user + assistant messages landed on B; A is untouched.
    expect(tabB.messages.map((m) => m.role)).toEqual(["user", "assistant"]);
    expect(tabB.messages[0].blocks).toEqual([{ type: "text", text: "into B" }]);
    expect(tabA.messages).toHaveLength(0);
    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_send",
      expect.objectContaining({ sessionId: b, prompt: "into B" }),
    );
    // Explicit target in a pane still focuses it (this IS a user action) — but
    // the messages were written to B regardless, which is the isolation guarantee.
    void a;
  });

});

// ── prompt_suggestion (#87) ──────────────────────────────────────────────────
// Wire shape confirmed against the 2.1.201 exe:
// { type: "prompt_suggestion", suggestion, uuid, session_id }.
describe("playback — prompt_suggestion ghost chip", () => {
  it("stores the trimmed suggestion on the tab; the next beginTurn clears it", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      { type: "prompt_suggestion", suggestion: "  run the tests  ", uuid: "u1", session_id: tab.cliSessionId },
    ]);
    expect(tab.promptSuggestion).toBe("run the tests");

    // A new turn invalidates the stale suggestion.
    beginTurn(tab);
    expect(tab.promptSuggestion).toBeNull();
  });

  it("ignores blank / non-string suggestions", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [
      { type: "prompt_suggestion", suggestion: "   " },
      { type: "prompt_suggestion", suggestion: 42 },
      { type: "prompt_suggestion" },
    ]);
    expect(tab.promptSuggestion).toBeNull();
  });
});

// ── CLI-initiated continuation turns (turn.rs idle-drain) ──────────────────
// A background agent finishing between turns makes the CLI re-invoke the model
// on its own; the backend forwards those frames with NO send() having scaffolded
// streaming state. maybeBeginContinuation must scaffold exactly once, and ONLY
// on a contentful top-level assistant frame — anything weaker painting a bubble
// would turn post-done dribble into phantom messages.
describe("playback — continuation turn (background agent finished)", () => {
  it("scaffolds on the turn-opening init, then paints the streamed deltas live", () => {
    const tab = freshTab();
    const before = tab.messages.length;
    expect(tab.streaming).toBe(false);
    // Real continuation wire order (idle-drain forwards): init → deltas →
    // complete assistant envelope → result.
    feed(tab, [sysModel("claude-opus-4-8")]);
    expect(tab.streaming).toBe(true);
    expect(tab.messages.length).toBe(before + 1);
    expect(tab.activity.currentLabel).toBe("Background agent finished — responding");
    feed(tab, [
      textDelta("Agent finished — "),
      textDelta("result: 4"),
      { type: "assistant", message: { content: [{ type: "text", text: "Agent finished — result: 4" }] } },
      resultEnv(),
    ]);
    const msg = tab.messages[tab.messages.length - 1];
    expect(msg.role).toBe("assistant");
    expect(JSON.stringify(textBlocks(tab, msg.id))).toContain("result: 4");
    tab.onDone();
    expect(tab.streaming).toBe(false);
  });

  it("scaffolds on a contentful assistant frame when no init preceded it", () => {
    const tab = freshTab();
    const before = tab.messages.length;
    feed(tab, [
      { type: "assistant", message: { content: [{ type: "text", text: "late follow-up" }] } },
    ]);
    expect(tab.streaming).toBe(true);
    expect(tab.messages.length).toBe(before + 1);
    tab.onDone();
  });

  it("does NOT scaffold on contentless / nested / non-JSON idle frames", () => {
    const tab = freshTab();
    const before = tab.messages.length;
    feed(tab, [
      { type: "assistant", message: { content: [] } },
      { type: "assistant", parent_tool_use_id: "t1", message: { content: [{ type: "text", text: "nested" }] } },
      "post-done non-json dribble",
    ]);
    expect(tab.streaming).toBe(false);
    expect(tab.messages.length).toBe(before);
  });

  it("never double-scaffolds while a turn is already streaming", () => {
    const tab = freshTab();
    beginTurn(tab);
    const before = tab.messages.length;
    feed(tab, [
      { type: "assistant", message: { content: [{ type: "text", text: "mid-turn text" }] } },
    ]);
    expect(tab.messages.length).toBe(before); // painted into the live bubble
  });
});

// ── Live tool-block forming (S127) ───────────────────────────────────────────
// content_block_start(tool_use) → input_json_delta chunks → content_block_stop
// forms a pending block live; the assistant envelope only finalizes input.
const toolStart = (index: number, id: string, name: string) => ({
  type: "stream_event",
  event: { type: "content_block_start", index, content_block: { type: "tool_use", id, name, input: {} } },
});
const inputDelta = (index: number, partial: string) => ({
  type: "stream_event",
  event: { type: "content_block_delta", index, delta: { type: "input_json_delta", partial_json: partial } },
});

describe("playback — live tool-block forming (S127)", () => {
  it("forms a pending block at content_block_start and fills caption fields as they stream", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [toolStart(1, "tf-1", "Bash")]);

    let tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ id: "tf-1", name: "Bash", status: "pending", inputPartial: true });
    expect(rec.toolUses).toHaveLength(1);

    // Tail capture: the still-open command string is provisionally visible.
    feed(tab, [inputDelta(1, '{"command":"cargo ')]);
    tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ input: { command: "cargo " }, inputPartial: true });

    // Completed pair replaces the provisional value.
    feed(tab, [inputDelta(1, 'build"}')]);
    tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ input: { command: "cargo build" } });

    // content_block_stop parses the full JSON and finalizes.
    feed(tab, [blockStop(1)]);
    tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ input: { command: "cargo build" }, status: "pending" });
    expect((tool as { inputPartial?: boolean }).inputPartial).toBeUndefined();
    expect(rec.toolUses[0].inputPreview).toBe("cargo build");
  });

  it("the assistant envelope finalizes a formed block — no duplicate, authoritative input", () => {
    const tab = freshTab();
    const rec = beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      toolStart(0, "tf-2", "Edit"),
      inputDelta(0, '{"file_path":"/a/foo.ts","old_string":"x'),
      // no blockStop — envelope arrives with the complete input
      toolUseEnv("tf-2", "Edit", { file_path: "/a/foo.ts", old_string: "x", new_string: "y" }),
    ]);
    const tools = textBlocks(tab, id).filter((b) => b.type === "tool");
    expect(tools).toHaveLength(1);
    expect(tools[0]).toMatchObject({
      id: "tf-2",
      input: { file_path: "/a/foo.ts", old_string: "x", new_string: "y" },
    });
    expect((tools[0] as { inputPartial?: boolean }).inputPartial).toBeUndefined();
    expect(rec.toolUses).toHaveLength(1);

    // and the result still settles it by id
    feed(tab, [toolResultEnv("tf-2", "ok", false)]);
    expect(textBlocks(tab, id).find((b) => b.type === "tool")).toMatchObject({ status: "done", result: "ok" });
  });

  it("side-effectful names (TodoWrite/Task/Agent/suppressed) never live-form", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [
      toolStart(0, "tf-todo", "TodoWrite"),
      toolStart(1, "tf-agent", "Agent"),
      toolStart(2, "tf-tsearch", "ToolSearch"),
    ]);
    expect(textBlocks(tab, id).filter((b) => b.type === "tool")).toHaveLength(0);

    // The envelope path still handles them normally afterwards.
    feed(tab, [toolUseEnv("tf-todo", "TodoWrite", { todos: [{ content: "step 1", status: "pending" }] })]);
    const plan = textBlocks(tab, id).find((b) => b.type === "tool" && b.name === "TodoWrite");
    expect(plan).toBeTruthy();
  });

  it("huge input stays bounded: only caption fields are extracted, diff fields ignored", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    const bigChunk = '{"file_path":"/a/big.ts","content":"' + "x".repeat(20_000);
    feed(tab, [toolStart(0, "tf-3", "Write"), inputDelta(0, bigChunk)]);
    const tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ input: { file_path: "/a/big.ts" }, inputPartial: true });
    expect((tool as { input: Record<string, unknown> }).input.content).toBeUndefined();
  });

  it("a user Stop mid-forming sweeps the block to error and clears the partial flag", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    feed(tab, [toolStart(0, "tf-4", "Bash"), inputDelta(0, '{"command":"sleep 999')]);
    // finalizeInflightBlocks is what every terminal path (error/Stop) runs.
    finalizeInflightBlocks(tab);
    const tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ status: "error" });
    expect((tool as { inputPartial?: boolean }).inputPartial).toBeUndefined();
  });

  it("#100: the envelope heals a block whose forming deltas were lost (finalized empty)", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    // Deltas never arrive → content_block_stop finalizes with EMPTY input and
    // clears inputPartial. The envelope must still land the real command.
    feed(tab, [toolStart(0, "tf-5", "Bash"), blockStop(0)]);
    let tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ input: {} });
    expect((tool as { inputPartial?: boolean }).inputPartial).toBeUndefined();

    feed(tab, [toolUseEnv("tf-5", "Bash", { command: "cargo build" })]);
    tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ input: { command: "cargo build" }, status: "pending" });
  });

  it("#100: a tool result clears a stuck forming flag", () => {
    const tab = freshTab();
    beginTurn(tab);
    const id = tab.streamingMsgId!;
    // No stop, no envelope — the result alone must end the forming state.
    feed(tab, [toolStart(0, "tf-6", "Bash"), inputDelta(0, '{"command":"npm run ch'), toolResultEnv("tf-6", "ok", false)]);
    const tool = textBlocks(tab, id).find((b) => b.type === "tool");
    expect(tool).toMatchObject({ status: "done", result: "ok" });
    expect((tool as { inputPartial?: boolean }).inputPartial).toBeUndefined();
  });
});

// ── CLI-initiated continuation turns ─────────────────────────────────────────
// A background agent finishing after `result` makes the CLI re-invoke the model
// on its own. One that begins moments after the DONE must reopen the previous
// bubble (the "Worked for <1s" split-header bug); a late one gets its own.
describe("playback — continuation merge", () => {
  const contAssistant = (text: string) => ({
    type: "assistant",
    message: { content: [{ type: "text", text }] },
  });

  it("merges a continuation that begins within the merge window into the previous bubble, summing duration + cost", () => {
    const tab = freshTab();
    beginTurn(tab);
    const firstId = tab.streamingMsgId!;
    feed(tab, [textDelta("started"), blockStop(0), resultEnv({ duration_ms: 5000, total_cost_usd: 0.1 })]);
    tab.onDone();
    expect(tab.streaming).toBe(false);
    const countAfterFirst = tab.messages.length;

    // Continuation lands right after the DONE (lastTurnDoneAt just stamped).
    feed(tab, [contAssistant("Background agent finished")]);
    expect(tab.streaming).toBe(true);
    expect(tab.messages.length).toBe(countAfterFirst); // reopened, no new bubble
    expect(tab.streamingMsgId).toBe(firstId);
    feed(tab, [resultEnv({ duration_ms: 3000, total_cost_usd: 0.2 })]);
    tab.onDone();

    const m = tab.messages.find((x) => x.id === firstId)!;
    expect(m.turnDurationMs).toBe(8000);
    expect(m.costUsd).toBeCloseTo(0.3);
    // Paragraph break between the two turns' text, not a fused sentence.
    const txt = m.blocks.filter((b) => b.type === "text").map((b) => (b as { text: string }).text).join("|");
    expect(txt).toBe("started\n\nBackground agent finished");
  });

  it("a continuation past the merge window still opens its own bubble", () => {
    const tab = freshTab();
    beginTurn(tab);
    const firstId = tab.streamingMsgId!;
    feed(tab, [textDelta("started"), blockStop(0), resultEnv({ duration_ms: 5000 })]);
    tab.onDone();
    tab.lastTurnDoneAt = Date.now() - 9000; // simulate a late background agent
    const countAfterFirst = tab.messages.length;

    feed(tab, [contAssistant("Background agent finished")]);
    expect(tab.streaming).toBe(true);
    expect(tab.messages.length).toBe(countAfterFirst + 1);
    expect(tab.streamingMsgId).not.toBe(firstId);
  });

  it("never merges into an errored turn (error terminal clears the stamp)", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [textDelta("partial"), blockStop(0)]);
    tab.onError("boom");
    expect(tab.lastTurnDoneAt).toBeNull();
    const countAfterError = tab.messages.length;

    feed(tab, [contAssistant("Background agent finished")]);
    expect(tab.messages.length).toBe(countAfterError + 1); // new bubble, error bubble untouched
  });
});
