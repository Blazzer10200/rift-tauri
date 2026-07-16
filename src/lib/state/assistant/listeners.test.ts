import { describe, it, expect, beforeEach, vi } from "vitest";

// The done handler pushes the bg_task warning through the toast singleton —
// stub it so the suite stays headless and calls are assertable.
vi.mock("../toast.svelte", () => ({
  toast: { push: vi.fn() },
  notify: { ok: vi.fn(), info: vi.fn(), warn: vi.fn(), danger: vi.fn() },
}));

import {
  handleStreamEvent,
  handleDoneEvent,
  handleErrorEvent,
  handleShellRowsEvent,
  type ListenerHost,
  type ListenerTab,
} from "./listeners";
import { notify } from "../toast.svelte";

const mockWarn = vi.mocked(notify.warn);

type FakeTab = ListenerTab & {
  streamed: string[];
  doneCount: number;
  errors: string[];
};

function makeTab(epoch = 0): FakeTab {
  const tab: FakeTab = {
    turnEpoch: epoch,
    staleTerminalUntil: 0,
    shellRows: [],
    streamed: [],
    doneCount: 0,
    errors: [],
    onStream(raw: string) { tab.streamed.push(raw); },
    onDone() { tab.doneCount++; },
    onError(m: string) { tab.errors.push(m); },
  };
  return tab;
}

function makeHost(opts: { active?: FakeTab | null; tabs?: Record<string, FakeTab> } = {}): ListenerHost {
  const tabs = opts.tabs ?? {};
  return {
    activeTab: opts.active ?? null,
    tabBySession: (sid: string) => tabs[sid] ?? null,
    bgTaskWarnedSessions: new Set<string>(),
  };
}

beforeEach(() => {
  mockWarn.mockClear();
});

describe("handleStreamEvent", () => {
  it("legacy bare-string payload routes to activeTab", () => {
    const active = makeTab();
    handleStreamEvent(makeHost({ active }), '{"type":"ping"}');
    expect(active.streamed).toEqual(['{"type":"ping"}']);
  });

  it("legacy string with no activeTab is a no-op, not a crash", () => {
    expect(() => handleStreamEvent(makeHost(), "line")).not.toThrow();
  });

  it("session_id routes to that tab, not the active one", () => {
    const active = makeTab();
    const bg = makeTab();
    handleStreamEvent(
      makeHost({ active, tabs: { s1: bg } }),
      { session_id: "s1", line: "frame" },
    );
    expect(bg.streamed).toEqual(["frame"]);
    expect(active.streamed).toEqual([]);
  });

  it("unknown session_id drops the frame (never falls back to activeTab)", () => {
    const active = makeTab();
    handleStreamEvent(makeHost({ active }), { session_id: "ghost", line: "frame" });
    expect(active.streamed).toEqual([]);
  });

  it("missing / non-string line is dropped", () => {
    const active = makeTab();
    handleStreamEvent(makeHost({ active }), { line: undefined });
    handleStreamEvent(makeHost({ active }), {});
    handleStreamEvent(makeHost({ active }), null);
    expect(active.streamed).toEqual([]);
  });

  it("#80: stale-epoch frame is dropped WITHOUT consuming the stop gate", () => {
    const tab = makeTab(3);
    tab.staleTerminalUntil = 99;
    handleStreamEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1", line: "old", turn_epoch: 2 });
    expect(tab.streamed).toEqual([]);
    // a mere frame doesn't consume the gate — the stale terminal is still inbound
    expect(tab.staleTerminalUntil).toBe(99);
  });

  it("#80: matching epoch paints; legacy epoch-less payload paints", () => {
    const tab = makeTab(3);
    const host = makeHost({ tabs: { s1: tab } });
    handleStreamEvent(host, { session_id: "s1", line: "live", turn_epoch: 3 });
    handleStreamEvent(host, { session_id: "s1", line: "legacy" });
    expect(tab.streamed).toEqual(["live", "legacy"]);
  });
});

describe("handleDoneEvent", () => {
  it("no session_id finalizes the active tab", () => {
    const active = makeTab();
    handleDoneEvent(makeHost({ active }), {});
    expect(active.doneCount).toBe(1);
  });

  it("session_id routes the terminal to the owning tab", () => {
    const active = makeTab();
    const bg = makeTab();
    handleDoneEvent(makeHost({ active, tabs: { s1: bg } }), { session_id: "s1" });
    expect(bg.doneCount).toBe(1);
    expect(active.doneCount).toBe(0);
  });

  it("#80: stale-epoch DONE consumes the stop gate and skips onDone", () => {
    const tab = makeTab(3);
    tab.staleTerminalUntil = Date.now() + 2000;
    handleDoneEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1", turn_epoch: 2 });
    expect(tab.doneCount).toBe(0);
    expect(tab.staleTerminalUntil).toBe(0);
  });

  it("#80: current-epoch DONE finalizes normally", () => {
    const tab = makeTab(3);
    handleDoneEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1", turn_epoch: 3 });
    expect(tab.doneCount).toBe(1);
  });

  it("bg_task warns once per session, not per turn", () => {
    const tab = makeTab();
    const host = makeHost({ tabs: { s1: tab } });
    handleDoneEvent(host, { session_id: "s1", bg_task: true });
    handleDoneEvent(host, { session_id: "s1", bg_task: true });
    expect(mockWarn).toHaveBeenCalledTimes(1);
  });

  it("bg_task warns again for a different session", () => {
    const host = makeHost({ tabs: { s1: makeTab(), s2: makeTab() } });
    handleDoneEvent(host, { session_id: "s1", bg_task: true });
    handleDoneEvent(host, { session_id: "s2", bg_task: true });
    expect(mockWarn).toHaveBeenCalledTimes(2);
  });

  it("bg_task warning still fires on a stale-epoch DONE (the superseded turn really backgrounded work)", () => {
    const tab = makeTab(3);
    handleDoneEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1", bg_task: true, turn_epoch: 2 });
    expect(tab.doneCount).toBe(0);
    expect(mockWarn).toHaveBeenCalledTimes(1);
  });

  it("bg_task with no session_id dedupes under the __active__ key", () => {
    const active = makeTab();
    const host = makeHost({ active });
    handleDoneEvent(host, { bg_task: true });
    handleDoneEvent(host, { bg_task: true });
    expect(mockWarn).toHaveBeenCalledTimes(1);
    expect(host.bgTaskWarnedSessions.has("__active__")).toBe(true);
  });

  it("warn registry is bounded: 200 cap evicts the oldest entry", () => {
    const host = makeHost({ active: makeTab() });
    for (let i = 0; i < 200; i++) host.bgTaskWarnedSessions.add(`old-${i}`);
    handleDoneEvent(host, { session_id: "s-new", bg_task: true });
    expect(host.bgTaskWarnedSessions.size).toBe(200);
    expect(host.bgTaskWarnedSessions.has("old-0")).toBe(false);
    expect(host.bgTaskWarnedSessions.has("s-new")).toBe(true);
  });

  it("unknown session DONE is a no-op (no crash, no active-tab finalize)", () => {
    const active = makeTab();
    handleDoneEvent(makeHost({ active }), { session_id: "ghost" });
    expect(active.doneCount).toBe(0);
  });
});

describe("handleErrorEvent", () => {
  it("legacy bare-string payload routes to activeTab", () => {
    const active = makeTab();
    handleErrorEvent(makeHost({ active }), "boom");
    expect(active.errors).toEqual(["boom"]);
  });

  it("session_id routes to the owning tab; non-string message dropped", () => {
    const tab = makeTab();
    const host = makeHost({ tabs: { s1: tab } });
    handleErrorEvent(host, { session_id: "s1", message: "err" });
    handleErrorEvent(host, { session_id: "s1" });
    handleErrorEvent(host, null);
    expect(tab.errors).toEqual(["err"]);
  });

  it("#80: stale-epoch error consumes the stop gate and is dropped", () => {
    const tab = makeTab(3);
    tab.staleTerminalUntil = Date.now() + 2000;
    handleErrorEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1", message: "old err", turn_epoch: 2 });
    expect(tab.errors).toEqual([]);
    expect(tab.staleTerminalUntil).toBe(0);
  });

  it("#80: current-epoch error banners normally", () => {
    const tab = makeTab(3);
    handleErrorEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1", message: "live err", turn_epoch: 3 });
    expect(tab.errors).toEqual(["live err"]);
  });
});

describe("handleShellRowsEvent", () => {
  const row = { pid: 4242, exe: "pwsh.exe", cmd: "pwsh -c npm run dev", started_at: 1_700_000_000 };

  it("session_id routes rows to the owning tab; empty rows clear", () => {
    const tab = makeTab();
    const host = makeHost({ tabs: { s1: tab } });
    handleShellRowsEvent(host, { session_id: "s1", rows: [row] });
    expect(tab.shellRows).toEqual([row]);
    handleShellRowsEvent(host, { session_id: "s1", rows: [] });
    expect(tab.shellRows).toEqual([]);
  });

  it("missing/invalid rows payload is dropped", () => {
    const tab = makeTab();
    tab.shellRows = [row];
    handleShellRowsEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1" });
    handleShellRowsEvent(makeHost({ tabs: { s1: tab } }), null);
    expect(tab.shellRows).toEqual([row]);
  });

  it("#80: stale-epoch rows must not paint into the next turn's HUD", () => {
    const tab = makeTab(3);
    handleShellRowsEvent(makeHost({ tabs: { s1: tab } }), { session_id: "s1", rows: [row], turn_epoch: 2 });
    expect(tab.shellRows).toEqual([]);
  });
});
