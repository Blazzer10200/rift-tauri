import { describe, it, expect, beforeEach, vi } from "vitest";

// Mock Tauri IPC before importing the store. The playback harness never lets a
// real turn reach the backend — it drives the stream accumulators directly — so
// invoke/listen/dialog only need to be inert.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
// toast is a UI singleton; the send/steer orchestrator calls toast.push on the
// steer-success path. Stub it so the harness stays headless.
vi.mock("./toast.svelte", () => ({ toast: { push: vi.fn() } }));

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
    effort: "quick",
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

  it("surfaces a known error result.subtype as lastError", () => {
    const tab = freshTab();
    beginTurn(tab);
    feed(tab, [resultEnv({ subtype: "error_max_turns" })]);
    expect(tab.lastError).toBe("Run ended with subtype: error_max_turns");
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

// ── Send / queue / steer orchestrator ────────────────────────────────────────
// These drive the REAL send() entry point with a mocked backend (invoke), so the
// turn-init, auth gate, queue-while-streaming, queue drain on completion, and
// steer paths are all exercised end-to-end — not the hand-rolled beginTurn setup
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
    assistant.currentConvoId = null; // no active tab → store-level notice
    assistant.auth = { pill: "red", summary: "Claude isn't set up" } as never;
    mockInvoke.mockClear();
    const before = assistant.telemetry.turns.length;

    await assistant.send("hi");

    expect(assistant.telemetry.turns.length).toBe(before);
    expect(assistant.lastNotice).toBe("Claude isn't set up");
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
});

describe("playback — steer", () => {
  it("injects into the live stream when the tab is streaming", async () => {
    const { tab } = readyStore();
    await assistant.send("go");
    mockInvoke.mockClear();

    await assistant.steer("focus on the tests");

    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_steer",
      expect.objectContaining({ text: "focus on the tests" }),
    );
    expect(tab.queue).toHaveLength(0); // steered, not queued
    expect(assistant.telemetry.snapshot().summary.eventCounts["turn.steer"]).toBe(1);
  });

  it("falls back to the queue when the tab isn't streaming", async () => {
    const { tab } = readyStore();
    mockInvoke.mockClear();
    await assistant.steer("do this next");
    expect(tab.queue.map((q) => q.text)).toEqual(["do this next"]);
    expect(mockInvoke).not.toHaveBeenCalledWith("assistant_steer", expect.anything());
  });

  it("re-queues when the backend reports the turn already ended (no_active_turn)", async () => {
    const { tab } = readyStore();
    await assistant.send("go");
    mockInvoke.mockResolvedValueOnce("no_active_turn");
    await assistant.steer("too late");
    expect(tab.queue.map((q) => q.text)).toEqual(["too late"]);
  });
});

describe("playback — steer-mode chips (Rail-v2)", () => {
  it("drain skips steer chips, then flushes them into the new turn at its first stream line", async () => {
    const { tab } = readyStore();
    await assistant.send("first");
    feed(tab, [textDelta("reply one")]); // turn 1's first line — latch fires with no steer chips

    // Steer chip ahead of a queue-mode chip: drain must NOT fire it as a turn.
    tab.queue = [{ id: "sc1", text: "watch the edge cases", mode: "steer" }];
    await assistant.send("second"); // streaming → queued behind the steer chip
    expect(tab.queue.map((q) => q.text)).toEqual(["watch the edge cases", "second"]);

    tab.onDone();
    await settle();

    // The queue-mode chip became turn 2; the steer chip stayed parked.
    expect(tab.streaming).toBe(true);
    expect(assistant.telemetry.turns).toHaveLength(2);
    expect(assistant.telemetry.turns[1].promptPreview).toBe("second");
    expect(tab.queue.map((q) => q.text)).toEqual(["watch the edge cases"]);

    // Turn 2's first stream line → steer chip injects into the running turn.
    mockInvoke.mockClear();
    feed(tab, [textDelta("turn two begins")]);
    await settle();
    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_steer",
      expect.objectContaining({ text: "watch the edge cases" }),
    );
    expect(tab.queue).toHaveLength(0);
  });

  it("an all-steer queue degrades its head to a normal send so it can't strand", async () => {
    const { tab } = readyStore();
    await assistant.send("go");
    feed(tab, [textDelta("working")]);
    tab.queue = [{ id: "sc1", text: "only a steer", mode: "steer" }];
    mockInvoke.mockClear();

    tab.onDone();
    await settle();

    expect(mockInvoke).toHaveBeenCalledWith(
      "assistant_send",
      expect.objectContaining({ prompt: "only a steer" }),
    );
    expect(tab.queue).toHaveLength(0);
  });
});
