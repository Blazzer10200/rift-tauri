import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

// Inert IPC — scheduleSave/flushAllAwait tests exercise scheduling logic only.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import {
  buildSaveRecord,
  flushAllAwait,
  inferChatGptRoute,
  resolveChatGptRoute,
  scheduleSave,
} from "./persistence";
import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

// buildSaveRecord's workspaceRoot resolution is the regression surface: a tab's
// saved folder must come from the tab's OWN root, else the GLOBAL workspace
// default — NEVER the focused pane's root (`activeRoot`), which would misfile a
// background/unfiled tab under an unrelated project. These build a minimal host
// + tab shaped like the structural PersistenceHost / SaveableTab subsets the
// function actually reads.

function host(over: Record<string, unknown> = {}) {
  return {
    // Fields buildSaveRecord touches; activeRoot is deliberately DIFFERENT from
    // workspaceCurrent so a leak is visible.
    model: "sonnet",
    activeRoot: "C:/proj/FOCUSED",
    workspaceCurrent: "C:/proj/GLOBAL",
    ...over,
  } as unknown as Parameters<typeof buildSaveRecord>[0];
}

function tab(over: Record<string, unknown> = {}) {
  return {
    messages: [{ role: "user", blocks: [{ type: "text", text: "hi" }] }],
    convoTitle: "t",
    convoCreatedAt: 1,
    lastActivityAt: 2,
    cliSessionId: "sid",
    modelOverride: null,
    lastTurnUsage: null,
    workspaceRoot: null,
    ...over,
  } as unknown as Parameters<typeof buildSaveRecord>[2];
}

describe("buildSaveRecord — workspaceRoot resolution", () => {
  it("uses the tab's own root when set", () => {
    const rec = buildSaveRecord(host(), "c1", tab({ workspaceRoot: "C:/proj/OWN" }));
    expect(rec.workspaceRoot).toBe("C:/proj/OWN");
  });

  it("falls back to the GLOBAL default, not the focused pane's root", () => {
    const rec = buildSaveRecord(host(), "c1", tab({ workspaceRoot: null }));
    expect(rec.workspaceRoot).toBe("C:/proj/GLOBAL");
    expect(rec.workspaceRoot).not.toBe("C:/proj/FOCUSED");
  });

  it("stays unfiled (null) when the tab has no root and no global default", () => {
    const rec = buildSaveRecord(host({ workspaceCurrent: null }), "c1", tab({ workspaceRoot: null }));
    expect(rec.workspaceRoot).toBeNull();
  });
});

describe("persisted ChatGPT route", () => {
  it("serializes the route pinned by the conversation", () => {
    const rec = buildSaveRecord(host({ model: "gpt-5.6-sol" }), "c1", tab({
      modelOverride: "gpt-5.6-sol",
      chatGptRoute: "codex",
    }));
    expect(rec.chatGptRoute).toBe("codex");
  });

  it("infers legacy routes from provider-owned continuation state", () => {
    expect(inferChatGptRoute(undefined, "thread-1", [])).toBe("codex");
    expect(inferChatGptRoute(undefined, null, [{ role: "user" }])).toBe("openai");
    // A Codex thread proves the original subscription route even if an old
    // build later accumulated API history through the unsafe fallback.
    expect(inferChatGptRoute(undefined, "thread-1", [{ role: "user" }])).toBe("codex");
    expect(inferChatGptRoute(undefined, null, [])).toBeNull();
  });

  it("honors an explicit persisted route over legacy evidence", () => {
    expect(inferChatGptRoute("openai", "thread-1", [])).toBe("openai");
    expect(inferChatGptRoute("codex", null, [{ role: "user" }])).toBe("codex");
  });

  it("never falls back when a pinned route becomes unavailable", () => {
    expect(resolveChatGptRoute("codex", false, true)).toBeNull();
    expect(resolveChatGptRoute("openai", true, false)).toBeNull();
    expect(resolveChatGptRoute("openai", true, true)).toBe("openai");
  });

  it("prefers the subscription route only before the chat is pinned", () => {
    expect(resolveChatGptRoute(null, true, true)).toBe("codex");
    expect(resolveChatGptRoute(null, false, true)).toBe("openai");
    expect(resolveChatGptRoute(null, false, false)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// v0.131.0 incident regressions: debounce starvation + pre-exit awaited flush.

/** Live tab shape — the SaveableTab subset scheduleSave/flushAllAwait touch. */
function liveTab(messages = 1) {
  return {
    messages: Array.from({ length: messages }, (_, i) => ({
      id: `m${i}`,
      role: "user",
      blocks: [{ type: "text", text: `msg ${i}` }],
    })),
    saveTimer: null as ReturnType<typeof setTimeout> | null,
    saveFirstQueuedAt: null as number | null,
    convoTitle: "t",
    convoCreatedAt: 1000,
    lastActivityAt: 2000,
    cliSessionId: "sess",
    titleGenerated: true as boolean, // pre-claimed → maybeGenerateTitle no-ops
    modelOverride: null as string | null,
    lastTurnUsage: null,
    workspaceRoot: null,
  };
}

function liveHost(tabs: Record<string, ReturnType<typeof liveTab>>) {
  return {
    model: "sonnet",
    workspaceCurrent: null,
    activeRoot: null,
    conversations: [],
    lastError: null,
    tabs: new Map(Object.entries(tabs)),
    currentConvoId: Object.keys(tabs)[0] ?? null,
  } as unknown as Parameters<typeof scheduleSave>[0];
}

const saveCalls = () =>
  mockInvoke.mock.calls.filter(([cmd]) => cmd === "assistant_save_conversation");

describe("scheduleSave starvation guard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("fires after the quiet 700ms debounce (baseline)", async () => {
    const h = liveHost({ a: liveTab() });
    scheduleSave(h, false, "a");
    expect(saveCalls().length).toBe(0);
    await vi.advanceTimersByTimeAsync(750);
    expect(saveCalls().length).toBe(1);
  });

  it("cannot be starved by continuous re-scheduling", async () => {
    const h = liveHost({ a: liveTab() });
    // Re-schedule every 300ms, streaming-delta style — a pure trailing
    // debounce would defer forever (the v0.131.0 data-loss shape).
    for (let i = 0; i < 30; i++) {
      scheduleSave(h, false, "a");
      await vi.advanceTimersByTimeAsync(300);
    }
    expect(saveCalls().length).toBeGreaterThanOrEqual(1);
  });

  it("clears the max-wait clock once a save dispatches", async () => {
    const h = liveHost({ a: liveTab() });
    scheduleSave(h, false, "a");
    await vi.advanceTimersByTimeAsync(750);
    expect(saveCalls().length).toBe(1);
    const t = (h as unknown as { tabs: Map<string, { saveFirstQueuedAt: number | null }> }).tabs.get("a")!;
    expect(t.saveFirstQueuedAt).toBeNull();
  });
});

describe("flushAllAwait (pre-exit awaited flush)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("writes every tab with content and skips empty tabs", async () => {
    const h = liveHost({ a: liveTab(2), b: liveTab(1), empty: liveTab(0) });
    await flushAllAwait(h);
    const saved = saveCalls()
      .map(([, args]) => (args as { convo: { id: string } }).convo.id)
      .sort();
    expect(saved).toEqual(["a", "b"]);
  });

  it("cancels a pending debounce so nothing double-writes after exit", async () => {
    const h = liveHost({ a: liveTab() });
    scheduleSave(h, false, "a");
    await flushAllAwait(h);
    expect(saveCalls().length).toBe(1);
    await vi.advanceTimersByTimeAsync(2000);
    expect(saveCalls().length).toBe(1);
  });
});

describe("provider-safe conversation titles", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
  });

  it("never sends a ChatGPT conversation through the Claude title helper", async () => {
    const chat = liveTab(0);
    chat.messages = [
      { id: "u1", role: "user", blocks: [{ type: "text", text: "Fix the provider picker" }] },
      { id: "a1", role: "assistant", blocks: [{ type: "text", text: "Done." }] },
    ];
    chat.titleGenerated = false;
    chat.modelOverride = "gpt-5.6-sol";
    const h = liveHost({ chat });

    scheduleSave(h, true, "chat");

    await vi.waitFor(() => expect(chat.titleGenerated).toBe(true));
    expect(mockInvoke.mock.calls.some(([command]) => command === "assistant_generate_title")).toBe(false);
  });
});
