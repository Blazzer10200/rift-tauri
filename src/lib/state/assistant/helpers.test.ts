import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChatMessage, ToolBlock } from "./types";
import {
  FABLE_DISABLED, FABLE_SUNSET_MS, clampEffort, effortToFlag, fableAvailable,
  flattenToolResult, messagesHaveContextSignals, migrateThinkingPins, modelFamily,
  previewToolInput,
} from "./helpers";

const tool = (name: string, status: ToolBlock["status"], extra: Partial<ToolBlock> = {}): ToolBlock =>
  ({ type: "tool", id: `t_${name}`, name, input: {}, result: null, isError: false, status, ...extra });
const msg = (blocks: ChatMessage["blocks"], id = "m1"): ChatMessage => ({ id, role: "assistant", blocks });

afterEach(() => vi.useRealTimers());

describe("fableAvailable", () => {
  it("manual kill-switch forces unavailable regardless of date", () => {
    // Fable pulled 2026-06-14 (US-gov disablement). While FABLE_DISABLED the
    // gate is false even before the date sunset; this test self-heals when the
    // flag flips back to re-enable.
    if (!FABLE_DISABLED) return;
    vi.useFakeTimers();
    vi.setSystemTime(FABLE_SUNSET_MS - 1);
    expect(fableAvailable()).toBe(false);
  });

  it("flips at the Jun 22 2026 EOD-UTC sunset when enabled", () => {
    if (FABLE_DISABLED) return; // kill-switch overrides the date gate
    vi.useFakeTimers();
    vi.setSystemTime(FABLE_SUNSET_MS - 1);
    expect(fableAvailable()).toBe(true);
    vi.setSystemTime(FABLE_SUNSET_MS);
    expect(fableAvailable()).toBe(false);
  });
});

describe("migrateThinkingPins", () => {
  const installLS = (seed: Record<string, string>) => {
    const store = new Map(Object.entries(seed));
    const ls = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
      key: (i: number) => Array.from(store.keys())[i] ?? null,
      get length() { return store.size; },
    };
    vi.stubGlobal("localStorage", ls);
    return store;
  };
  afterEach(() => vi.unstubAllGlobals());

  it("clears every per-folder thinkingEnabled pin but leaves model/effort pins + global baseline", () => {
    const store = installLS({
      "rift.assistant.thinkingEnabled::C:/proj/a": "on",
      "rift.assistant.thinkingEnabled::C:/proj/b": "off",
      "rift.assistant.thinkingEnabled": "off",          // global baseline — keep
      "rift.assistant.thinkingEffort::C:/proj/a": "deep", // effort pin — keep
      "rift.assistant.model::C:/proj/a": "opus",          // model pin — keep
    });
    migrateThinkingPins();
    expect(store.has("rift.assistant.thinkingEnabled::C:/proj/a")).toBe(false);
    expect(store.has("rift.assistant.thinkingEnabled::C:/proj/b")).toBe(false);
    expect(store.get("rift.assistant.thinkingEnabled")).toBe("off");
    expect(store.get("rift.assistant.thinkingEffort::C:/proj/a")).toBe("deep");
    expect(store.get("rift.assistant.model::C:/proj/a")).toBe("opus");
    expect(store.get("rift.assistant.thinkingPinSweep.v1")).toBe("done");
  });

  it("is idempotent — a fresh on-pin written after the sweep survives a second run", () => {
    const store = installLS({ "rift.assistant.thinkingEnabled::C:/proj/a": "on" });
    migrateThinkingPins(); // sweeps the stale pin, marks done
    store.set("rift.assistant.thinkingEnabled::C:/proj/a", "on"); // user re-pins intentionally
    migrateThinkingPins(); // must NOT sweep again
    expect(store.get("rift.assistant.thinkingEnabled::C:/proj/a")).toBe("on");
  });
});

describe("modelFamily", () => {
  it("maps every selector to its aurora family", () => {
    expect(modelFamily("haiku")).toBe("haiku");
    expect(modelFamily("opus")).toBe("opus");
    expect(modelFamily("claude-opus-4-7")).toBe("opus");
    expect(modelFamily("claude-fable-5")).toBe("opus");
    expect(modelFamily("sonnet")).toBe("sonnet");
  });
});

describe("effortToFlag (must mirror src-tauri assistant turn.rs mapping)", () => {
  it("suppresses effort entirely on haiku", () => {
    expect(effortToFlag("deep", "haiku")).toBeNull();
  });
  it("maps none/quick/smart/deep/ultra to low/medium/medium/high/xhigh", () => {
    expect(effortToFlag("none", "opus")).toBe("low");
    expect(effortToFlag("quick", "opus")).toBe("medium");
    expect(effortToFlag("smart", "opus")).toBe("medium"); // responsive default
    expect(effortToFlag("deep", "opus")).toBe("high");
    expect(effortToFlag("ultra", "claude-fable-5")).toBe("xhigh");
  });
  it("clamps an out-of-range tier to the model ceiling before mapping", () => {
    // Sonnet tops out at deep(high): a stale ultra(xhigh) pref must NOT send xhigh.
    expect(effortToFlag("ultra", "sonnet")).toBe("high");
    // smart on Sonnet is the responsive default → medium (not the old high).
    expect(effortToFlag("smart", "sonnet")).toBe("medium");
  });
});

describe("clampEffort (model effort ceiling)", () => {
  it("caps Sonnet at deep(high) and leaves Opus/Fable untouched", () => {
    expect(clampEffort("ultra", "sonnet")).toBe("deep"); // xhigh not accepted on Sonnet → down to high
    expect(clampEffort("deep", "sonnet")).toBe("deep");  // in range now (Sonnet accepts high)
    expect(clampEffort("quick", "sonnet")).toBe("quick"); // already in range
    expect(clampEffort("ultra", "opus")).toBe("ultra");
    expect(clampEffort("ultra", "claude-fable-5")).toBe("ultra");
  });
  it("floors Haiku to none (rejects effort wholesale)", () => {
    expect(clampEffort("ultra", "haiku")).toBe("none");
  });
});

describe("effortToFlag — default tier maps to medium (responsive interactive default)", () => {
  it("smart and quick both → medium; none → low", () => {
    expect(effortToFlag("smart", "opus")).toBe("medium");
    expect(effortToFlag("quick", "opus")).toBe("medium");
    expect(effortToFlag("none", "opus")).toBe("low");
  });
  it("deep → high, ultra → xhigh on a model that reaches them", () => {
    expect(effortToFlag("deep", "opus")).toBe("high");
    expect(effortToFlag("ultra", "opus")).toBe("xhigh");
  });
  it("Sonnet reaches deep(high) but xhigh clamps to high (no xhigh on Sonnet)", () => {
    expect(effortToFlag("deep", "sonnet")).toBe("high");
    expect(effortToFlag("ultra", "sonnet")).toBe("high"); // ultra clamps to sonnet's deep ceiling → high
  });
  it("haiku rejects effort wholesale → null", () => {
    expect(effortToFlag("smart", "haiku")).toBeNull();
    expect(effortToFlag("deep", "haiku")).toBeNull();
  });
});

describe("flattenToolResult + previewToolInput", () => {
  it("flattens strings, text-part arrays, and rejects the rest", () => {
    expect(flattenToolResult("plain")).toBe("plain");
    expect(flattenToolResult([{ text: "a" }, { other: 1 }, { text: "b" }])).toBe("ab");
    expect(flattenToolResult({ not: "array" })).toBe("");
  });
  it("previews the first known field in priority order and caps at 120", () => {
    expect(previewToolInput("Bash", { command: "ls", file_path: "x" })).toBe("ls");
    expect(previewToolInput("Read", { file_path: "src/a.ts" })).toBe("src/a.ts");
    expect(previewToolInput("X", { command: "c".repeat(150) })).toBe("c".repeat(120) + "…");
    expect(previewToolInput("X", {})).toBeNull();
    expect(previewToolInput("X", undefined)).toBeNull();
  });
});

describe("messagesHaveContextSignals", () => {
  it("bails true on the first write/web tool, false otherwise", () => {
    expect(messagesHaveContextSignals([msg([tool("Read", "done")])])).toBe(false);
    expect(messagesHaveContextSignals([msg([tool("Edit", "done")])])).toBe(true);
    expect(messagesHaveContextSignals([])).toBe(false);
  });
});

