import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChatMessage, ToolBlock } from "./types";
import {
  FABLE_DISABLED, FABLE_SUNSET_MS, clampEffort, effortToFlag, fableAvailable, firstLine,
  flattenToolResult, shellLabel, liveActivity, messagesHaveContextSignals, modelFamily,
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
  it("maps none/quick/smart/deep/ultra to low/medium/high/xhigh/xhigh", () => {
    expect(effortToFlag("none", "sonnet")).toBe("low");
    expect(effortToFlag("quick", "sonnet")).toBe("medium");
    expect(effortToFlag("smart", "sonnet")).toBe("high");
    expect(effortToFlag("deep", "opus")).toBe("xhigh");
    expect(effortToFlag("ultra", "claude-fable-5")).toBe("xhigh");
  });
  it("clamps an out-of-range tier to the model ceiling before mapping", () => {
    // Sonnet tops out at smart(high): a stale deep/ultra pref must NOT send xhigh.
    expect(effortToFlag("deep", "sonnet")).toBe("high");
    expect(effortToFlag("ultra", "sonnet")).toBe("high");
  });
});

describe("clampEffort (model effort ceiling)", () => {
  it("caps Sonnet at smart and leaves Opus/Fable untouched", () => {
    expect(clampEffort("ultra", "sonnet")).toBe("smart");
    expect(clampEffort("deep", "sonnet")).toBe("smart");
    expect(clampEffort("quick", "sonnet")).toBe("quick"); // already in range
    expect(clampEffort("ultra", "opus")).toBe("ultra");
    expect(clampEffort("ultra", "claude-fable-5")).toBe("ultra");
  });
  it("floors Haiku to none (rejects effort wholesale)", () => {
    expect(clampEffort("ultra", "haiku")).toBe("none");
  });
});

describe("flattenToolResult + previewToolInput + firstLine", () => {
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
  it("firstLine takes line one and caps at 60", () => {
    expect(firstLine("git status\nsecond")).toBe("git status");
    expect(firstLine("x".repeat(80))).toBe("x".repeat(59) + "…");
  });
  it("shellLabel strips cd-hops and middle-truncates keeping the tail", () => {
    expect(shellLabel('cd "C:/AI Workflow/projects/rift-tauri" && npm run check')).toBe("npm run check");
    expect(shellLabel("cd /a && cd 'b c' ; git status")).toBe("git status");
    expect(shellLabel('cd "C:/only/path"')).toBe('cd "C:/only/path"');
    const long = "git log --oneline " + "x".repeat(60) + " --tail-marker";
    const lbl = shellLabel(long);
    expect(lbl.length).toBe(60);
    expect(lbl.startsWith("git log --oneline")).toBe(true);
    expect(lbl.endsWith("--tail-marker")).toBe(true);
  });
});

describe("messagesHaveContextSignals", () => {
  it("bails true on the first write/web tool, false otherwise", () => {
    expect(messagesHaveContextSignals([msg([tool("Read", "done")])])).toBe(false);
    expect(messagesHaveContextSignals([msg([tool("Edit", "done")])])).toBe(true);
    expect(messagesHaveContextSignals([])).toBe(false);
  });
});

describe("liveActivity", () => {
  it("collects pending shells, generic tools, active thinking, and live agents — sorted by start", () => {
    const messages = [msg([
      { type: "thinking", text: "", hasSignature: false, startedAt: 30, durationMs: null, status: "active" },
      tool("Bash", "pending", { input: { command: "cargo check\nmore" }, startedAt: 10 }),
      tool("Read", "pending", { input: { file_path: "a/b.rs" }, startedAt: 20 }),
      tool("Grep", "done"),
      tool("Agent", "pending", { startedAt: 5 }),
    ])];
    const spawns = [
      { id: "ag1", subagentType: "recon", description: "map file", startedAt: 15, completedAt: null },
      { id: "ag2", subagentType: "scout", description: "done one", startedAt: 1, completedAt: 99 },
    ];
    const out = liveActivity(messages, spawns, 999);
    expect(out.map((i) => i.kind)).toEqual(["shell", "agent", "tool", "thinking"]);
    expect(out[0].label).toBe("cargo check");
    expect(out[1]).toMatchObject({ sub: "recon", label: "map file" });
    expect(out[2].label).toBe("Reading b.rs");
  });
  it("falls back to fallbackTs when a pending shell has no startedAt", () => {
    const messages = [msg([tool("Bash", "pending", { input: { command: "ls" } })])];
    expect(liveActivity(messages, [], 1234)[0].startedAt).toBe(1234);
  });
});
