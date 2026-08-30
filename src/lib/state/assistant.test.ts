import { afterEach, describe, it, expect, beforeEach, vi } from "vitest";

// Mock Tauri IPC before importing the store. assistant.svelte.ts wires
// `invoke` + `listen` at construction time; the test only exercises pure
// derivations on the singleton's state, so the mocks never actually fire.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { assistant } from "./assistant.svelte.js";
import { recordTurnUsage } from "./assistant/streaming.js";
import { restoreTabs } from "./assistant/tabs.js";
import { buildSaveRecord, tabsStorageKey } from "./assistant/persistence.js";
import { shell } from "./shell.svelte.js";
import { createPaneState } from "./assistant/types.js";

const pane = (tabId: string | null = null) => createPaneState(tabId);

describe("provider readiness", () => {
  afterEach(() => {
    assistant.auth = null;
    assistant.openAiStatus = null;
    assistant.openAiModels = null;
    assistant.openAiModelsError = null;
  });

  it("requires the Claude CLI even when an API key is configured", () => {
    assistant.auth = { cliPresent: false, pill: "yellow" } as any;
    expect(assistant.authReadyForModel("sonnet")).toBe(false);
    assistant.auth = { cliPresent: true, pill: "yellow" } as any;
    expect(assistant.authReadyForModel("sonnet")).toBe(true);
  });

  it("requires confirmed model access for OpenAI", () => {
    assistant.openAiStatus = { ready: true, apiKeyConfigured: true } as any;
    expect(assistant.authReadyForModel("gpt-5.6")).toBe(false);
    assistant.openAiModels = [{ id: "gpt-5.6", available: false }] as any;
    expect(assistant.authReadyForModel("gpt-5.6")).toBe(false);
    assistant.openAiModels = [{ id: "gpt-5.6", available: true }] as any;
    expect(assistant.authReadyForModel("gpt-5.6")).toBe(true);
  });

  it("reads each split pane's provider instead of the focused pane", () => {
    assistant.auth = { cliPresent: true, pill: "green" } as any;
    assistant.openAiStatus = { ready: false, apiKeyConfigured: false } as any;
    const claudePane = { modelOverride: "sonnet" } as any;
    const openAiPane = { modelOverride: "gpt-5.6" } as any;
    expect(assistant.authReadyFor(claudePane)).toBe(true);
    expect(assistant.authReadyFor(openAiPane)).toBe(false);
  });
});

// Minimal turn record. TurnRecord is a private type so we build a structural
// stand-in and cast — the accumulator only reads the fields below.
type StubTurn = {
  ts: number;
  convoId: string;
  cliSessionId: string;
  isFirstTurn: boolean;
  model: "sonnet" | "opus" | "haiku";
  effort: "none" | "smart" | "deep";
  effortFlag: "low" | "medium" | "high" | null;
  promptLen: number;
  promptPreview: string;
  attachmentsCount: number;
  attachmentsBytes: number;
  envelopeUsage: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  resultUsage: { input: number; output: number; cacheRead: number; cacheCreate: number } | null;
  modelId: string | null;
  costUsd: number | null;
  deltaCount: number;
  streamEventCount: number;
  assistantEnvCount: number;
  maxStreamGapMs: number;
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
  thinkingBlocks: { startedAt: number; durationMs: number; charCount: number; hasSignature: boolean }[];
  envelopeFallback: boolean;
  blankTurn: boolean;
  firstPaintAt: number | null;
  doneAt: number | null;
  endKind: "success" | "user-stop" | "session-lost" | "error" | null;
};

const stubTurn = (overrides: Partial<StubTurn>): StubTurn => ({
  ts: 1_000,
  convoId: "c1",
  cliSessionId: "sess-1",
  isFirstTurn: false,
  model: "sonnet",
  effort: "smart",
  effortFlag: "medium",
  promptLen: 10,
  promptPreview: "hi",
  attachmentsCount: 0,
  attachmentsBytes: 0,
  envelopeUsage: null,
  resultUsage: null,
  modelId: "claude-sonnet-4-6",
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
  endKind: "success",
  ...overrides,
});

describe("assistant.ctxWindowFor()", () => {
  beforeEach(() => assistant.setPlan("max"));
  afterEach(() => assistant.setPlan("free"));

  it("uses the selected 1M cap when no tab is passed", () => {
    // With no tab/model, the window falls through to the explicitly selected
    // 1M cap. The app default remains the conservative 200K choice.
    expect(assistant.ctxWindowFor(null)).toBe(1_000_000);
  });

  it("returns 200K for haiku models", () => {
    const tab = { lastModelId: "claude-haiku-4-5-20251001", lastTurnUsage: null } as any;
    expect(assistant.ctxWindowFor(tab)).toBe(200_000);
  });

  it("returns 1M for sonnet 4.6/5 + opus 4.6/4.7, 200K for sonnet 4.5", () => {
    // Sonnet 4.5 is CLI-gated at 200K and the backend never sends it `[1m]`
    // (config.rs SONNET_1M_GATED excludes it) — the gauge must report 200K to
    // match, else a resumed pre-rename 4.5 session over-reports the window 5×.
    expect(assistant.ctxWindowFor({ lastModelId: "claude-sonnet-4-5", lastTurnUsage: null } as any)).toBe(200_000);
    expect(assistant.ctxWindowFor({ lastModelId: "claude-sonnet-4-6", lastTurnUsage: null } as any)).toBe(1_000_000);
    expect(assistant.ctxWindowFor({ lastModelId: "claude-sonnet-5", lastTurnUsage: null } as any)).toBe(1_000_000);
    expect(assistant.ctxWindowFor({ lastModelId: "claude-opus-4-6", lastTurnUsage: null } as any)).toBe(1_000_000);
    expect(assistant.ctxWindowFor({ lastModelId: "claude-opus-4-7", lastTurnUsage: null } as any)).toBe(1_000_000);
  });

  it("returns 1M for fable 5", () => {
    const tab = { lastModelId: "claude-fable-5", lastTurnUsage: null } as any;
    expect(assistant.ctxWindowFor(tab)).toBe(1_000_000);
  });

  it("returns 1M for any model with the [1m] suffix", () => {
    const tab = { lastModelId: "claude-opus-4-7[1m]", lastTurnUsage: null } as any;
    expect(assistant.ctxWindowFor(tab)).toBe(1_000_000);
  });

  it("falls back to 200K for unrecognized model ids", () => {
    const tab = { lastModelId: "claude-mystery-3-0", lastTurnUsage: null } as any;
    expect(assistant.ctxWindowFor(tab)).toBe(200_000);
  });

  it("prefers the CLI-reported window while the tab is on that model", () => {
    // Ground truth beats the plan×model estimate: a Free-plan account whose
    // API gates sonnet at 200K reports contextWindow:200000 even though the
    // default `max` plan estimate would claim 1M.
    const tab = {
      lastModelId: "claude-sonnet-5",
      lastTurnUsage: null,
      reportedCtxWindow: { model: "claude-sonnet-5", window: 200_000 },
    } as any;
    expect(assistant.ctxWindowFor(tab)).toBe(200_000);
  });

  it("matches a [1m]-suffixed lastModelId against the normalized reported model", () => {
    const tab = {
      lastModelId: "claude-sonnet-5[1m]",
      lastTurnUsage: null,
      reportedCtxWindow: { model: "claude-sonnet-5", window: 1_000_000 },
    } as any;
    expect(assistant.ctxWindowFor(tab)).toBe(1_000_000);
  });

  it("ignores a stale reported window after a mid-chat model switch", () => {
    // lastModelId moved to opus but the report is still sonnet's — fall back
    // to the estimate until the next result re-reports for the new model.
    const tab = {
      lastModelId: "claude-opus-4-8",
      lastTurnUsage: null,
      reportedCtxWindow: { model: "claude-sonnet-5", window: 200_000 },
    } as any;
    expect(assistant.ctxWindowFor(tab)).toBe(1_000_000);
  });
});

describe("assistant.ctxTokensFor()", () => {
  it("returns 0 for null tab", () => {
    expect(assistant.ctxTokensFor(null)).toBe(0);
  });

  it("returns 0 when no usage recorded yet", () => {
    const tab = { lastModelId: "claude-sonnet-4-6", lastTurnUsage: null } as any;
    expect(assistant.ctxTokensFor(tab)).toBe(0);
  });

  it("sums input + cacheRead + cacheCreate (NOT output)", () => {
    const tab = {
      lastModelId: "claude-sonnet-4-6",
      lastTurnUsage: { input: 100, output: 9999, cacheRead: 2000, cacheCreate: 500 },
    } as any;
    expect(assistant.ctxTokensFor(tab)).toBe(2600); // 100 + 2000 + 500
  });
});

describe("recordTurnUsage — ctx pill reads point-in-time, not cumulative task usage", () => {
  it("the assistant envelope drives lastTurnUsage; the result event must not inflate it", () => {
    const tab = assistant.ensureTab("ctx-regression-convo", "ctx-regression-sess") as any;
    // Final assistant envelope of a long task = the true window occupancy now.
    recordTurnUsage(
      tab,
      { input_tokens: 120, cache_read_input_tokens: 198_000, cache_creation_input_tokens: 2_000, output_tokens: 540 },
      false,
    );
    // Result event sums usage across EVERY loop step — a >1M cache_read is normal
    // on a long task (anthropics/claude-agent-sdk-python#548). It must NOT drive
    // the pill, or auto-compact false-fires the instant a task finishes.
    recordTurnUsage(
      tab,
      { input_tokens: 1_792, cache_read_input_tokens: 1_346_549, cache_creation_input_tokens: 53_078, output_tokens: 20_027 },
      true,
    );
    expect(tab.lastTurnUsage).toEqual({ input: 120, output: 540, cacheRead: 198_000, cacheCreate: 2_000 });
    expect(assistant.ctxTokensFor(tab)).toBe(200_120); // 120 + 198_000 + 2_000 — nowhere near 1.3M
    // Result still feeds the (intentionally cumulative) session totals.
    expect(tab.sessionUsage.totalCacheRead).toBe(1_346_549);
    expect(tab.sessionUsage.turns).toBe(1);
  });
});

describe("assistant.ctxPctFor()", () => {
  beforeEach(() => assistant.setPlan("max"));
  afterEach(() => assistant.setPlan("free"));

  it("returns 0 when no usage", () => {
    const tab = { lastModelId: "claude-sonnet-4-6", lastTurnUsage: null } as any;
    expect(assistant.ctxPctFor(tab)).toBe(0);
  });

  it("computes percentage against the 1M window for big-ctx models", () => {
    const tab = {
      lastModelId: "claude-opus-4-7",
      lastTurnUsage: { input: 250_000, output: 1, cacheRead: 0, cacheCreate: 0 },
    } as any;
    // 250_000 / 1_000_000 = 25%
    expect(assistant.ctxPctFor(tab)).toBeCloseTo(25, 5);
  });

  it("computes percentage against the 200K window for haiku", () => {
    const tab = {
      lastModelId: "claude-haiku-4-5-20251001",
      lastTurnUsage: { input: 50_000, output: 0, cacheRead: 0, cacheCreate: 0 },
    } as any;
    // 50_000 / 200_000 = 25%
    expect(assistant.ctxPctFor(tab)).toBeCloseTo(25, 5);
  });

  it("clamps to 100% when usage exceeds window", () => {
    const tab = {
      lastModelId: "claude-haiku-4-5-20251001",
      lastTurnUsage: { input: 999_999_999, output: 0, cacheRead: 0, cacheCreate: 0 },
    } as any;
    expect(assistant.ctxPctFor(tab)).toBe(100);
  });
});

describe("assistant.telemetry.snapshot() — byModel rollup", () => {
  beforeEach(() => {
    assistant.telemetry.turns = [];
    assistant.telemetry.events = [];
  });

  it("returns empty rollup when no turns recorded", () => {
    const s = assistant.telemetry.snapshot();
    expect(s.turnCount).toBe(0);
    expect(s.summary.totalTurns).toBe(0);
    expect(s.summary.totalCostUsd).toBe(0);
    expect(s.summary.byModel).toEqual({});
  });

  it("accumulates input/output/cache tokens per model bucket", () => {
    assistant.telemetry.turns = [
      stubTurn({
        modelId: "claude-sonnet-4-6",
        resultUsage: { input: 100, output: 200, cacheRead: 50, cacheCreate: 10 },
        costUsd: 0.01,
      }),
      stubTurn({
        modelId: "claude-sonnet-4-6",
        resultUsage: { input: 300, output: 400, cacheRead: 70, cacheCreate: 5 },
        costUsd: 0.02,
      }),
      stubTurn({
        modelId: "claude-opus-4-7",
        resultUsage: { input: 1000, output: 2000, cacheRead: 500, cacheCreate: 100 },
        costUsd: 0.5,
      }),
    ] as any;

    const s = assistant.telemetry.snapshot();
    expect(s.summary.totalTurns).toBe(3);
    expect(s.summary.totalCostUsd).toBeCloseTo(0.53, 4);

    const sonnet = s.summary.byModel["claude-sonnet-4-6"];
    expect(sonnet).toBeDefined();
    expect(sonnet.turns).toBe(2);
    expect(sonnet.inputTokens).toBe(400); // 100 + 300
    expect(sonnet.outputTokens).toBe(600); // 200 + 400
    expect(sonnet.cacheReadTokens).toBe(120); // 50 + 70
    expect(sonnet.cacheCreateTokens).toBe(15); // 10 + 5
    expect(sonnet.costUsd).toBeCloseTo(0.03, 4);

    const opus = s.summary.byModel["claude-opus-4-7"];
    expect(opus.turns).toBe(1);
    expect(opus.inputTokens).toBe(1000);
  });

  it("falls back to envelopeUsage when resultUsage is null", () => {
    assistant.telemetry.turns = [
      stubTurn({
        modelId: "claude-sonnet-4-6",
        envelopeUsage: { input: 11, output: 22, cacheRead: 33, cacheCreate: 44 },
        resultUsage: null,
      }),
    ] as any;
    const s = assistant.telemetry.snapshot();
    const bucket = s.summary.byModel["claude-sonnet-4-6"];
    expect(bucket.inputTokens).toBe(11);
    expect(bucket.outputTokens).toBe(22);
    expect(bucket.cacheReadTokens).toBe(33);
    expect(bucket.cacheCreateTokens).toBe(44);
  });

  it("skips error turns with no resolved modelId (no phantom bucket)", () => {
    assistant.telemetry.turns = [
      stubTurn({
        modelId: null,
        endKind: "error",
        resultUsage: { input: 5, output: 5, cacheRead: 0, cacheCreate: 0 },
      }),
      stubTurn({
        modelId: "claude-sonnet-4-6",
        endKind: "success",
        resultUsage: { input: 10, output: 10, cacheRead: 0, cacheCreate: 0 },
      }),
    ] as any;
    const s = assistant.telemetry.snapshot();
    // totalTurns counts every entry on the array, but byModel only includes
    // resolved modelIds.
    expect(s.summary.totalTurns).toBe(2);
    expect(Object.keys(s.summary.byModel)).toEqual(["claude-sonnet-4-6"]);
    expect(s.summary.byModel["claude-sonnet-4-6"].inputTokens).toBe(10);
  });

  it("tracks blank-turn and thinking-turn counts", () => {
    assistant.telemetry.turns = [
      stubTurn({ modelId: "claude-sonnet-4-6", blankTurn: true }),
      stubTurn({ modelId: "claude-sonnet-4-6", thinkingCount: 2 }),
      stubTurn({ modelId: "claude-sonnet-4-6" }),
    ] as any;
    const s = assistant.telemetry.snapshot();
    expect(s.summary.blankTurns).toBe(1);
    expect(s.summary.thinkingTurns).toBe(1);
    const b = s.summary.byModel["claude-sonnet-4-6"];
    expect(b.blankTurns).toBe(1);
    expect(b.thinkingTurns).toBe(1);
  });

  it("counts cumulative tool calls + errors across all turns", () => {
    const mkTool = (name: string, isError = false) => ({
      name,
      id: `t-${Math.random()}`,
      startedAt: 0,
      completedAt: 100,
      durationMs: 100,
      isError,
      inputPreview: null,
    });
    assistant.telemetry.turns = [
      stubTurn({
        modelId: "claude-sonnet-4-6",
        toolUses: [mkTool("Read"), mkTool("Grep")],
      }),
      stubTurn({
        modelId: "claude-sonnet-4-6",
        toolUses: [mkTool("Bash", true)],
      }),
    ] as any;
    const s = assistant.telemetry.snapshot();
    expect(s.summary.toolCallTotal).toBe(3);
    expect(s.summary.toolErrorTotal).toBe(1);
    expect(s.summary.toolNameCounts).toEqual({ Read: 1, Grep: 1, Bash: 1 });
  });

  it("event() pushes to events with kind + ts + detail", () => {
    assistant.telemetry.event("tab.switch", { from: "a", to: "b" });
    assistant.telemetry.event("tab.switch", { from: "b", to: "c" });
    assistant.telemetry.event("pane.split.on", {});
    const s = assistant.telemetry.snapshot();
    expect(s.summary.eventCounts).toEqual({
      "tab.switch": 2,
      "pane.split.on": 1,
    });
  });

  it("flags stale-cache turns (continuation paid cacheCreate w/ zero cacheRead)", () => {
    assistant.telemetry.turns = [
      stubTurn({
        modelId: "claude-sonnet-4-6",
        isFirstTurn: false,
        endKind: "success",
        resultUsage: { input: 10, output: 10, cacheRead: 0, cacheCreate: 1000 },
      }),
      stubTurn({
        modelId: "claude-sonnet-4-6",
        isFirstTurn: false,
        endKind: "success",
        resultUsage: { input: 10, output: 10, cacheRead: 800, cacheCreate: 0 },
      }),
      // First-turn cacheCreate is expected; must NOT count as stale.
      stubTurn({
        modelId: "claude-sonnet-4-6",
        isFirstTurn: true,
        endKind: "success",
        resultUsage: { input: 10, output: 10, cacheRead: 0, cacheCreate: 50_000 },
      }),
    ] as any;
    const s = assistant.telemetry.snapshot();
    expect(s.summary.staleCacheTurns).toBe(1);
  });
});

describe("tab switching never kills background streams (multi-tab regression)", () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_load_conversation") {
        return {
          id: args.id, title: "t", model: "sonnet", createdAt: 1, updatedAt: 1,
          messages: [], cliSessionId: args.id,
        };
      }
      if (cmd === "assistant_list_conversations") return assistant.conversations;
      return undefined;
    });
  });

  const userMsg = () => ({
    id: crypto.randomUUID(),
    role: "user" as const,
    blocks: [{ type: "text" as const, text: "hi" }],
  });

  it("switching away to a saved (not-yet-open) tab leaves the streaming tab running", async () => {
    const tabA = assistant.ensureTab("bg-away-a", "bg-away-a") as any;
    tabA.messages = [userMsg()];
    tabA.streaming = true;
    tabA.streamingMsgId = "m-live";
    assistant.conversations = [
      { id: "bg-away-a", title: "a", model: "sonnet", createdAt: 1, updatedAt: 1 },
      { id: "bg-away-b", title: "b", model: "sonnet", createdAt: 1, updatedAt: 1 },
    ] as any;
    assistant.openTabs = ["bg-away-a"];
    assistant.currentConvoId = "bg-away-a";

    await assistant.openTab("bg-away-b");

    expect(assistant.currentConvoId).toBe("bg-away-b");
    expect(tabA.streaming).toBe(true);
    expect(tabA.streamingMsgId).toBe("m-live");
    expect(invokeMock).not.toHaveBeenCalledWith("assistant_stop", expect.anything());
  });

  it("switching back to a live streaming tab pointer-switches without disk reload", async () => {
    const tabA = assistant.ensureTab("bg-back-a", "bg-back-a") as any;
    const liveMsgs = [userMsg(), { id: "m-stream", role: "assistant", blocks: [{ type: "text", text: "partial…" }] }];
    tabA.messages = liveMsgs;
    tabA.streaming = true;
    tabA.streamingMsgId = "m-stream";
    assistant.ensureTab("bg-back-b", "bg-back-b");
    assistant.conversations = [
      { id: "bg-back-a", title: "a", model: "sonnet", createdAt: 1, updatedAt: 1 },
    ] as any;
    assistant.openTabs = ["bg-back-a", "bg-back-b"];
    assistant.currentConvoId = "bg-back-b";

    await assistant.openTab("bg-back-a");

    expect(assistant.currentConvoId).toBe("bg-back-a");
    expect(invokeMock).not.toHaveBeenCalledWith("assistant_load_conversation", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("assistant_stop", expect.anything());
    expect(tabA.streaming).toBe(true);
    expect(tabA.messages.map((m: any) => m.id)).toEqual(liveMsgs.map((m) => m.id));
  });
});

describe("loadConversation hydrates the persisted per-tab root (cont.269 regression)", () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => invokeMock.mockReset());
  // loadConversation refreshes the sidebar cache — keep `conversations` an
  // array so unmocked list calls can't leak undefined into later describes.
  afterEach(() => { assistant.conversations = [] as any; });

  it("restores workspaceRoot from the ConversationRecord (was write-only: saved, never read back)", async () => {
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_load_conversation") {
        return {
          id: args.id, title: "t", model: "sonnet", createdAt: 1, updatedAt: 1,
          messages: [], cliSessionId: args.id, workspaceRoot: "C:/proj/rooted",
        };
      }
      if (cmd === "assistant_session_cwd") return null;
      if (cmd === "assistant_list_conversations") return [];
      return undefined;
    });
    await assistant.loadConversation("root-restore-1");
    expect((assistant.tabFor("root-restore-1") as any).workspaceRoot).toBe("C:/proj/rooted");
  });

  it("legacy record without workspaceRoot falls back to the pinned session cwd", async () => {
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_load_conversation") {
        return { id: args.id, title: "t", model: "sonnet", createdAt: 1, updatedAt: 1, messages: [], cliSessionId: args.id };
      }
      if (cmd === "assistant_session_cwd") return "C:/proj/pinned-cwd";
      if (cmd === "assistant_list_conversations") return [];
      return undefined;
    });
    await assistant.loadConversation("root-restore-2");
    await new Promise((r) => setTimeout(r, 0)); // session-cwd lookup resolves async
    expect((assistant.tabFor("root-restore-2") as any).workspaceRoot).toBe("C:/proj/pinned-cwd");
  });
});

describe("drag a tab onto a pane half enters split (single-pane)", () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_load_conversation") {
        return { id: args.id, title: "t", model: "sonnet", createdAt: 1, updatedAt: 1, messages: [], cliSessionId: args.id };
      }
      return undefined;
    });
    assistant.panes = [pane()];
    assistant.focusedPaneIdx = 0;
  });

  it("dragging the CURRENTLY-SHOWN tab still splits, pairing the other open tab (the reported bug)", () => {
    assistant.ensureTab("drag-a", "drag-a");
    assistant.ensureTab("drag-b", "drag-b");
    assistant.openTabs = ["drag-a", "drag-b"];
    assistant.currentConvoId = "drag-a";
    assistant.panes = [pane("drag-a")];

    // Drop the visible tab (drag-a) onto the right half.
    assistant.dropTabIntoPane("drag-a", 1);

    expect(assistant.splitActive).toBe(true);
    expect(assistant.panes.map((p) => p.tabId)).toEqual(["drag-b", "drag-a"]);
    expect(assistant.focusedPaneIdx).toBe(1);
  });

  it("dragging a DIFFERENT tab pairs it with the current tab (existing behavior preserved)", () => {
    assistant.ensureTab("pair-a", "pair-a");
    assistant.ensureTab("pair-b", "pair-b");
    assistant.openTabs = ["pair-a", "pair-b"];
    assistant.currentConvoId = "pair-a";
    assistant.panes = [pane("pair-a")];

    assistant.dropTabIntoPane("pair-b", 1);

    expect(assistant.splitActive).toBe(true);
    expect(assistant.panes.map((p) => p.tabId)).toEqual(["pair-a", "pair-b"]);
  });

  it("dragging the only open tab splits with an empty counterpart slot (no silent no-op)", () => {
    assistant.ensureTab("solo", "solo");
    assistant.openTabs = ["solo"];
    assistant.currentConvoId = "solo";
    assistant.panes = [pane("solo")];

    assistant.dropTabIntoPane("solo", 1);

    expect(assistant.splitActive).toBe(true);
    expect(assistant.panes.map((p) => p.tabId)).toEqual([null, "solo"]);
  });
});

describe("openProjectInPane — open a project into a (split) pane", () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      // setTabRoot canonicalizes via the backend → echo the path back.
      if (cmd === "assistant_set_tab_root") return args.path;
      if (cmd === "assistant_set_root") return { current: args.path, recent: [] };
      if (cmd === "assistant_list_recent_roots") return { current: null, recent: [] };
      return undefined;
    });
    assistant.panes = [pane()];
    assistant.focusedPaneIdx = 0;
    assistant.openTabs = [];
    assistant.currentConvoId = null;
    // Headless env reports width-for-1-pane; this suite tests routing, not the
    // width-fit guard — let the split grow up to MAX.
    vi.spyOn(shell, "maxPanesForWidth").mockReturnValue(4);
  });

  it("splitNew grows the split and scopes the new pane's tab to the project root (no global mutation)", async () => {
    const grew = await assistant.openProjectInPane("C:/proj/alpha", { splitNew: true });

    expect(grew).toBe(true);
    expect(assistant.splitActive).toBe(true);
    // Focus + new tab landed in the freshly-added (last) pane.
    expect(assistant.focusedPaneIdx).toBe(assistant.panes.length - 1);
    const tabId = assistant.currentConvoId!;
    expect(assistant.tabFor(tabId)!.workspaceRoot).toBe("C:/proj/alpha");
    // Scopes via per-tab root (assistant_set_tab_root), NOT the global root —
    // assistant_set_root must never fire.
    expect(invokeMock).toHaveBeenCalledWith("assistant_set_tab_root", { path: "C:/proj/alpha" });
    expect(invokeMock).not.toHaveBeenCalledWith("assistant_set_root", expect.anything());
  });

  it("paneIdx beyond current panes auto-grows; falls back to focused pane when split can't grow", async () => {
    // Fill to MAX so addPane can't grow → returns false, opens in focused pane.
    assistant.panes = [pane(), pane(), pane(), pane()];
    assistant.focusedPaneIdx = 1;
    const grew = await assistant.openProjectInPane("C:/proj/beta", { splitNew: true });

    expect(grew).toBe(false);
    expect(assistant.focusedPaneIdx).toBe(1);
    expect(assistant.tabFor(assistant.currentConvoId!)!.workspaceRoot).toBe("C:/proj/beta");
  });
});

describe("pane/workspace identity isolation", () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_set_tab_root") return args.path;
      if (cmd === "assistant_set_root") return { current: args.path, recent: [args.path] };
      if (cmd === "assistant_get_workspace") return (assistant as any).workspace;
      if (cmd === "assistant_session_cwd") return null;
      return undefined;
    });
    (assistant as any).tabs = new Map();
    (assistant as any).workspace = { current: "C:/proj/a", recent: ["C:/proj/a"] };
    assistant.openTabs = [];
    assistant.panes = [pane()];
    assistant.focusedPaneIdx = 0;
    assistant.currentConvoId = null;
    assistant.conversations = [] as any;
    shell.setAllProjects(false);
    vi.spyOn(shell, "maxPanesForWidth").mockReturnValue(4);
  });

  it("captures the global root when a tab is created instead of following future switches", async () => {
    const aId = await assistant.newTab();
    expect(assistant.tabFor(aId)!.workspaceRoot).toBe("C:/proj/a");

    await assistant.openProjectInPane("C:/proj/b", { splitNew: true });
    const bId = assistant.currentConvoId!;
    expect(assistant.tabFor(bId)!.workspaceRoot).toBe("C:/proj/b");

    shell.setAllProjects(true);
    await assistant.setRoot("C:/proj/c");

    expect((assistant as any).workspace.current).toBe("C:/proj/c");
    expect(assistant.tabFor(aId)!.workspaceRoot).toBe("C:/proj/a");
    expect(assistant.tabFor(bId)!.workspaceRoot).toBe("C:/proj/c");
    expect(assistant.panes.map((p) => assistant.tabFor(p.tabId)?.workspaceRoot)).toEqual([
      "C:/proj/a",
      "C:/proj/c",
    ]);
    // All is a history view only; it never becomes a path/project identity.
    expect(shell.conversationScope).toEqual({ kind: "all" });
  });

  it("treats a null tab root as explicit local mode, not a mutable global fallback", () => {
    const tab = assistant.ensureTab("local-tab", "local-tab");
    tab.workspaceRoot = null;
    (assistant as any).workspace = { current: "C:/proj/global", recent: [] };
    expect(assistant.effectiveRoot(tab)).toBeNull();
    expect(assistant.effectiveRoot(null)).toBe("C:/proj/global");
  });

  it("sets a project on the tab minted for a stable pane even if focus moves mid-await", async () => {
    const aId = await assistant.newTab();
    const aPaneId = assistant.panes[0].id;
    assistant.addPane();
    const bPaneId = assistant.panes[1].id;

    let release!: (value: string) => void;
    const canonical = new Promise<string>((resolve) => { release = resolve; });
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "assistant_set_tab_root") return canonical;
      if (cmd === "assistant_get_workspace") return (assistant as any).workspace;
      return undefined;
    });

    const opening = assistant.openProjectInPane("C:/proj/b", { paneId: bPaneId });
    await Promise.resolve();
    const mintedId = assistant.panes.find((p) => p.id === bPaneId)?.tabId;
    expect(mintedId).toBeTruthy();

    assistant.setFocusedPaneById(aPaneId);
    release("C:/proj/b");
    await opening;

    expect(assistant.currentConvoId).toBe(aId);
    expect(assistant.tabFor(aId)!.workspaceRoot).toBe("C:/proj/a");
    expect(assistant.tabFor(mintedId!)!.workspaceRoot).toBe("C:/proj/b");
    expect(assistant.panes.find((p) => p.id === bPaneId)?.tabId).toBe(mintedId);
  });

  it("hydrates a late conversation into its originating pane without activating another pane", async () => {
    const a = crypto.randomUUID();
    const b = crypto.randomUUID();
    const pa = pane(a);
    const pb = pane(b);
    assistant.panes = [pa, pb];
    assistant.openTabs = [a, b];
    assistant.conversations = [
      { id: a, title: "A", model: "sonnet", createdAt: 1, updatedAt: 1 },
      { id: b, title: "B", model: "sonnet", createdAt: 1, updatedAt: 1 },
    ] as any;

    const resolvers = new Map<string, (value: any) => void>();
    invokeMock.mockImplementation((cmd: string, args?: any) => {
      if (cmd === "assistant_load_conversation") {
        return new Promise((resolve) => resolvers.set(args.id, resolve));
      }
      if (cmd === "assistant_session_cwd") return Promise.resolve(null);
      return Promise.resolve(undefined);
    });

    assistant.setFocusedPaneById(pa.id);
    assistant.setFocusedPaneById(pb.id);
    resolvers.get(a)!({
      id: a, title: "A", model: "sonnet", createdAt: 1, updatedAt: 1,
      messages: [], cliSessionId: a, workspaceRoot: "C:/proj/a",
    });
    await Promise.resolve();
    await Promise.resolve();

    expect(assistant.currentConvoId).not.toBe(a);
    expect(assistant.panes).toEqual([pa, pb]);

    resolvers.get(b)!({
      id: b, title: "B", model: "sonnet", createdAt: 1, updatedAt: 1,
      messages: [], cliSessionId: b, workspaceRoot: "C:/proj/b",
    });
    await Promise.resolve();
    await Promise.resolve();

    expect(assistant.currentConvoId).toBe(b);
    expect(assistant.tabFor(a)!.workspaceRoot).toBe("C:/proj/a");
    expect(assistant.tabFor(b)!.workspaceRoot).toBe("C:/proj/b");
  });

  it("refuses two conversation records that claim the same provider session", async () => {
    assistant.ensureTab("first", "shared-session");
    assistant.conversations = [
      { id: "first", title: "First", model: "sonnet", createdAt: 1, updatedAt: 1 },
      { id: "second", title: "Second", model: "sonnet", createdAt: 1, updatedAt: 1 },
    ] as any;
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_load_conversation") {
        return {
          id: args.id,
          title: "Second",
          model: "sonnet",
          createdAt: 1,
          updatedAt: 1,
          messages: [],
          cliSessionId: "shared-session",
        };
      }
      return null;
    });

    const loaded = await assistant.loadConversation("second", { activate: false });

    expect(loaded).toBe(false);
    expect(assistant.tabFor("second")).toBeNull();
    expect(assistant.tabFor("first")?.cliSessionId).toBe("shared-session");
  });

  it("starts a cross-provider chat in the pane that requested it even when another pane is focused", async () => {
    const aId = await assistant.newTab();
    const aPaneId = assistant.panes[0].id;
    const a = assistant.tabFor(aId)!;
    a.modelOverride = "sonnet";
    a.messages = [{ id: "u", role: "user", blocks: [{ type: "text", text: "started" }] }];

    await assistant.openProjectInPane("C:/proj/b", { splitNew: true });
    const bId = assistant.currentConvoId!;
    const bPaneId = assistant.panes[1].id;
    expect(assistant.panes[assistant.focusedPaneIdx]?.id).toBe(bPaneId);

    await assistant.selectModel("gpt-5.6-sol", a, { paneId: aPaneId, sourceTabId: aId });

    const replacementId = assistant.panes.find((p) => p.id === aPaneId)?.tabId;
    expect(replacementId).toBeTruthy();
    expect(replacementId).not.toBe(aId);
    expect(assistant.modelFor(assistant.tabFor(replacementId!))).toBe("gpt-5.6-sol");
    expect(assistant.panes.find((p) => p.id === bPaneId)?.tabId).toBe(bId);
    expect(assistant.modelFor(assistant.tabFor(bId))).toBe("sonnet");
  });

  it("preserves a background tab's prompt when its provider session is lost", () => {
    const a = assistant.ensureTab("lost-a", "lost-session-a");
    const b = assistant.ensureTab("lost-b", "lost-session-b");
    a.messages = [
      { id: "u", role: "user", blocks: [{ type: "text", text: "keep this prompt" }] },
      { id: "a", role: "assistant", blocks: [] },
    ];
    a.streaming = true;
    assistant.openTabs = ["lost-a", "lost-b"];
    assistant.panes = [pane("lost-a"), pane("lost-b")];
    assistant.focusedPaneIdx = 1;
    assistant.currentConvoId = "lost-b";

    (assistant as any).onSessionLost({ session_id: "lost-session-a" });

    expect(a.streaming).toBe(false);
    expect(a.messages).toHaveLength(1);
    expect(a.messages[0].role).toBe("user");
    expect(a.lastError).toContain("prompt is preserved");
    expect(b.messages).toEqual([]);
    expect(assistant.currentConvoId).toBe("lost-b");
  });
});

describe("conversation permission isolation", () => {
  beforeEach(() => {
    (assistant as any).tabs = new Map();
    (assistant as any).workspace = { current: "C:/proj/a", recent: ["C:/proj/a"] };
    assistant.openTabs = [];
    assistant.panes = [pane()];
    assistant.focusedPaneIdx = 0;
    assistant.currentConvoId = null;
    assistant.permissionMode = "bypassPermissions";
  });

  it("keeps explicit and provider-driven mode changes on their owning tabs", async () => {
    const aId = await assistant.newTab();
    const a = assistant.tabFor(aId)!;
    assistant.addPane();
    const bId = await assistant.newTab();
    const b = assistant.tabFor(bId)!;

    assistant.setPermissionMode("default", a);
    expect(assistant.permissionModeFor(a)).toBe("default");
    expect(assistant.permissionModeFor(b)).toBe("bypassPermissions");

    // A background provider event must neither restyle nor widen its sibling.
    b.onEnterPlanMode?.(b);
    expect(assistant.permissionModeFor(b)).toBe("plan");
    expect(b.planReturnMode).toBe("bypassPermissions");
    expect(assistant.permissionModeFor(a)).toBe("default");
    expect(a.planReturnMode).toBeNull();
  });

  it("persists the tab-owned permission mode in its conversation record", async () => {
    const id = await assistant.newTab();
    const tab = assistant.tabFor(id)!;
    tab.permissionMode = "acceptEdits";
    tab.messages = [{ id: "u1", role: "user", blocks: [{ type: "text", text: "hello" }] }];

    const record = buildSaveRecord(assistant as any, id, tab);

    expect(record.permissionMode).toBe("acceptEdits");
  });
});

describe("split-pane model/effort scoping — a pick in one pane never restyles a sibling", () => {
  it("setModel pins sibling tabs that followed the global default, then moves it", () => {
    assistant.model = "sonnet";
    const a = assistant.ensureTab("scope-a", "scope-a");
    const b = assistant.ensureTab("scope-b", "scope-b");
    a.modelOverride = null;
    b.modelOverride = null;

    assistant.setModel("opus", a);

    expect(assistant.modelFor(a)).toBe("opus");
    // Sibling keeps what it was showing — pinned, not leaked.
    expect(assistant.modelFor(b)).toBe("sonnet");
    // The new-chat default still moves (the pick came from a global-root pane).
    expect(assistant.model).toBe("opus");
  });

  it("setThinkingDial scopes the effort pick to its tab; the sibling keeps its tier", () => {
    assistant.thinkingEnabled = true;
    assistant.thinkingEffort = "smart";
    const a = assistant.ensureTab("scope-c", "scope-c");
    const b = assistant.ensureTab("scope-d", "scope-d");
    a.effortOverride = null; a.thinkingOverride = null;
    b.effortOverride = null; b.thinkingOverride = null;

    assistant.setThinkingDial(true, "ultra", a);

    expect(assistant.effortFor(a)).toBe("ultra");
    expect(assistant.effortFor(b)).toBe("smart");
    expect(assistant.thinkingOnFor(b)).toBe(true);
  });
});

describe("assistant.sessionUsage default", () => {
  it("returns zeroed structure when no active tab", () => {
    // Test runs in node env with no tabs initialized → the getter falls back
    // to the empty default. We assert the structural contract.
    const u = assistant.sessionUsage;
    expect(u).toEqual({
      totalInput: 0,
      totalOutput: 0,
      totalCacheRead: 0,
      totalCacheCreate: 0,
      turns: 0,
    });
  });
});

describe("deleteAllConversations — partial backend failure (orphan-tab regression)", () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("drops the deleted convo's tab but keeps a tab whose delete REJECTED", async () => {
    assistant.ensureTab("del-ok", "del-ok");
    assistant.ensureTab("del-fail", "del-fail");
    assistant.openTabs = ["del-ok", "del-fail"];
    assistant.currentConvoId = "del-ok";
    assistant.panes = [pane("del-ok")];
    assistant.focusedPaneIdx = 0;
    assistant.conversations = [
      { id: "del-ok", title: "ok", model: "sonnet", createdAt: 1, updatedAt: 1 },
      { id: "del-fail", title: "fail", model: "sonnet", createdAt: 1, updatedAt: 1 },
    ] as any;

    // Backend deletes "del-ok" but fails "del-fail" (e.g. mid-restart). With the
    // old Promise.all this rejected before any teardown, orphaning both tabs.
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_delete_conversation") {
        if (args.id === "del-fail") throw new Error("backend mid-restart");
        return undefined;
      }
      if (cmd === "assistant_list_conversations") {
        return [{ id: "del-fail", title: "fail", model: "sonnet", createdAt: 1, updatedAt: 1 }];
      }
      return undefined;
    });

    await assistant.deleteAllConversations();

    // del-ok was deleted → its tab is torn down; del-fail's delete REJECTED → its
    // tab survives. The old Promise.all rejected before teardown, orphaning del-ok
    // (it would still be in openTabs + tabs here). newTab() runs after the purge,
    // so currentConvoId is a fresh tab — assert only that it isn't a deleted convo.
    expect(assistant.openTabs).not.toContain("del-ok");
    expect(assistant.openTabs).toContain("del-fail");
    expect((assistant as any).tabs.has("del-ok")).toBe(false);
    expect((assistant as any).tabs.has("del-fail")).toBe(true);
    expect(assistant.currentConvoId).not.toBe("del-ok");
  });
});

// Two panes must never key the same tab — the pane {#each} in AssistantPage is
// keyed by tabId, so a duplicate throws each_key_duplicate and blanks the whole
// chat surface; persistTabs then poisons localStorage so it recurs on EVERY
// load (found live 2026-07-10). These pin the three guards.
describe("pane duplicate-key invariant", () => {
  const invokeMock = vi.mocked(invoke);
  // Node test env has no localStorage (persistTabs try/catches it away, but
  // restoreTabs needs a readable record) — shim one for this block only.
  const lsStore = new Map<string, string>();
  const hadLS = "localStorage" in globalThis;
  let prevWorkspace: unknown;

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "assistant_load_conversation") {
        return { id: args.id, title: "t", model: "sonnet", createdAt: 1, updatedAt: 1, messages: [], cliSessionId: args.id };
      }
      return undefined;
    });
    lsStore.clear();
    (globalThis as any).localStorage = {
      getItem: (k: string) => lsStore.get(k) ?? null,
      setItem: (k: string, v: string) => void lsStore.set(k, String(v)),
      removeItem: (k: string) => void lsStore.delete(k),
      clear: () => lsStore.clear(),
    };
    // effectiveRoot reads workspace.current on every pane move — stub it.
    prevWorkspace = (assistant as any).workspace;
    (assistant as any).workspace = { current: null, recent: [] };
    assistant.tabs = new Map();
    assistant.openTabs = [];
    assistant.currentConvoId = null;
    assistant.panes = [pane()];
    assistant.focusedPaneIdx = 0;
  });

  afterEach(() => {
    if (!hadLS) delete (globalThis as any).localStorage;
    (assistant as any).workspace = prevWorkspace;
    assistant.conversations = [] as any;
    assistant.tabs = new Map();
    assistant.openTabs = [];
    assistant.currentConvoId = null;
    assistant.panes = [pane()];
    assistant.focusedPaneIdx = 0;
  });

  it("restores unsent tabs with independent pane, workspace, draft, model, and permission state", async () => {
    const tabA = "550e8400-e29b-41d4-a716-4466554400d1";
    const tabB = "550e8400-e29b-41d4-a716-4466554400d2";
    localStorage.setItem(tabsStorageKey(), JSON.stringify({
      openTabs: [tabA, tabB],
      activeTabId: tabB,
      panes: [
        { id: "pane-a", tabId: tabA },
        { id: "pane-b", tabId: tabB },
      ],
      focusedPaneIdx: 1,
      ephemeralTabs: [
        {
          id: tabA,
          workspaceRoot: "C:/proj/a",
          model: "sonnet",
          effort: "smart",
          thinkingOn: true,
          permissionMode: "default",
          draft: "draft a",
        },
        {
          id: tabB,
          workspaceRoot: "C:/proj/b",
          model: "gpt-5.6-sol",
          effort: "deep",
          thinkingOn: false,
          permissionMode: "acceptEdits",
          draft: "draft b",
        },
      ],
    }));

    await restoreTabs(assistant as any);

    expect(assistant.panes).toEqual([
      { id: "pane-a", tabId: tabA },
      { id: "pane-b", tabId: tabB },
    ]);
    expect(assistant.focusedPaneIdx).toBe(1);
    expect(assistant.currentConvoId).toBe(tabB);
    expect(assistant.tabFor(tabA)).toMatchObject({
      workspaceRoot: "C:/proj/a",
      modelOverride: "sonnet",
      effortOverride: "smart",
      thinkingOverride: true,
      permissionMode: "default",
      draft: "draft a",
    });
    expect(assistant.tabFor(tabB)).toMatchObject({
      workspaceRoot: "C:/proj/b",
      modelOverride: "gpt-5.6-sol",
      effortOverride: "deep",
      thinkingOverride: false,
      permissionMode: "acceptEdits",
      draft: "draft b",
    });
    expect(invokeMock).not.toHaveBeenCalledWith("assistant_load_conversation", expect.anything());

    const healed = JSON.parse(localStorage.getItem(tabsStorageKey()) ?? "{}");
    expect(healed.panes).toEqual([
      { id: "pane-a", tabId: tabA },
      { id: "pane-b", tabId: tabB },
    ]);
    expect(healed.ephemeralTabs).toHaveLength(2);
  });

  it("restoreTabs self-heals a poisoned record with the same tab in two panes", async () => {
    assistant.conversations = [
      { id: "dup-a", title: "a", model: "sonnet", createdAt: 1, updatedAt: 1 },
    ] as any;
    localStorage.setItem(tabsStorageKey(), JSON.stringify({
      openTabs: ["dup-a"],
      activeTabId: "dup-a",
      panes: [{ tabId: "dup-a" }, { tabId: "dup-a" }, { tabId: null }],
      focusedPaneIdx: 2,
    }));

    await restoreTabs(assistant as any);

    const ids = assistant.panes.map((p) => p.tabId).filter(Boolean);
    expect(ids).toEqual(["dup-a"]); // later duplicate hydrated empty
    // Focus lands on the pane that actually shows the winner — the repoint
    // must not re-mint the duplicate the dedup just cleared.
    expect(assistant.panes[assistant.focusedPaneIdx]?.tabId).toBe("dup-a");
  });

  it("closeTab whose neighbor already renders in a sibling pane focuses it instead of duplicating", async () => {
    assistant.ensureTab("ka", "ka");
    assistant.ensureTab("kb", "kb");
    assistant.openTabs = ["ka", "kb"];
    assistant.currentConvoId = "kb";
    assistant.panes = [pane("ka"), pane("kb")];
    assistant.focusedPaneIdx = 1;

    await assistant.closeTab("kb");

    expect(assistant.panes.map((p) => p.tabId)).toEqual(["ka", null]);
    expect(assistant.focusedPaneIdx).toBe(0);
  });

  it("sentinel drop of an already-visible tab focuses its pane instead of adding a duplicate pane", () => {
    assistant.ensureTab("sa", "sa");
    assistant.ensureTab("sb", "sb");
    assistant.openTabs = ["sa", "sb"];
    assistant.currentConvoId = "sb";
    assistant.panes = [pane("sa"), pane("sb")];
    assistant.focusedPaneIdx = 1;

    assistant.dropTabIntoPane("sa", 2); // sentinel: "new pane at end"

    expect(assistant.panes.map((p) => p.tabId)).toEqual(["sa", "sb"]);
    expect(assistant.focusedPaneIdx).toBe(0);
  });
});
