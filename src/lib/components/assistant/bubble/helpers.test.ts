import { describe, expect, it } from "vitest";
import type { Block, ThinkingBlock, ToolBlock } from "../../../state/assistant/types";
import {
  coalesceToolGroups, elapsedFor, formatDuration, formatDurationMs, groupDurationMs,
  isCardTool, isGroupableChip,
  isInlineDiffTool, lineDelta, nodeKind, numberActions, parseTextBlock,
  mergeSplitProse, reconcileSplitHeaders, shortModel, shortToolName, statusOf, summarizeGroup,
  type TimelineUnit,
} from "./helpers";

const tool = (name: string, status: ToolBlock["status"] = "done", isError = false): ToolBlock =>
  ({ type: "tool", id: `t_${name}`, name, input: {}, result: null, isError, status });
const text = (t: string): Block => ({ type: "text", text: t });
const unit = (b: Block, key: string): TimelineUnit =>
  ({ kind: "block", block: b, key, status: statusOf(b) });

describe("tool classifiers", () => {
  it("classifies edit-family as inline diff, with and without the MCP prefix", () => {
    expect(isInlineDiffTool("Edit")).toBe(true);
    expect(isInlineDiffTool("mcp__rift__MultiEdit")).toBe(true);
    expect(isInlineDiffTool("Write")).toBe(true);
    expect(isInlineDiffTool("Read")).toBe(false);
  });
  it("classifies card tools and excludes both families from grouping", () => {
    expect(isCardTool("Agent")).toBe(true);
    expect(isCardTool("mcp__rift__TodoWrite")).toBe(true);
    expect(isGroupableChip("Read")).toBe(true);
    expect(isGroupableChip("Edit")).toBe(false);
    expect(isGroupableChip("Agent")).toBe(false);
  });
  it("strips the MCP prefix in shortToolName", () => {
    expect(shortToolName("mcp__rift__grep")).toBe("grep");
    expect(shortToolName("Bash")).toBe("Bash");
  });
});

describe("parseTextBlock", () => {
  it("returns one prose segment for plain text", () => {
    expect(parseTextBlock("hello\nworld")).toEqual([{ kind: "prose", text: "hello\nworld" }]);
  });
  it("splits multiple step headers into separate segments", () => {
    const segs = parseTextBlock("## Step 1 — Setup\nbody A\n## Step 2 — Run\nbody B");
    expect(segs).toEqual([
      { kind: "header", stepNum: 1, title: "Setup" },
      { kind: "prose", text: "body A" },
      { kind: "header", stepNum: 2, title: "Run" },
      { kind: "prose", text: "body B" },
    ]);
  });
  it("accepts colon/dash separators, bold wrap, and case-insensitive 'step'", () => {
    expect(parseTextBlock("**step 3: Deploy**")[0]).toEqual({ kind: "header", stepNum: 3, title: "Deploy" });
    expect(parseTextBlock("Step 4 - Verify")[0]).toEqual({ kind: "header", stepNum: 4, title: "Verify" });
  });
  it("synthesizes a title when the header has none", () => {
    expect(parseTextBlock("## Step 5 — ")[0]).toEqual({ kind: "header", stepNum: 5, title: "Step 5" });
  });
});

describe("reconcileSplitHeaders", () => {
  it("reconstructs a header torn across a tool call", () => {
    const blocks: Block[] = [text("## S"), tool("Bash"), text("tep 1 — Long bash\nrest of prose")];
    const out = reconcileSplitHeaders(blocks);
    expect(out.map((b) => b.type)).toEqual(["text", "tool", "text"]);
    expect((out[0] as { text: string }).text).toBe("## Step 1 — Long bash");
    expect((out[2] as { text: string }).text).toBe("rest of prose");
  });
  it("leaves a non-header partial untouched", () => {
    const blocks: Block[] = [text("## Hello"), tool("Bash"), text(" world, no step here")];
    expect(reconcileSplitHeaders(blocks)).toEqual(blocks);
  });
  it("leaves a partial with no following text block untouched", () => {
    const blocks: Block[] = [text("## S"), tool("Bash")];
    expect(reconcileSplitHeaders(blocks)).toEqual(blocks);
  });
});

describe("mergeSplitProse", () => {
  it("stitches a sentence split mid-word by an early tool_use, placing it after the tool", () => {
    const blocks: Block[] = [text("Let me check the current state of your projec"), tool("git_status"), text("t.")];
    const out = mergeSplitProse(blocks);
    expect(out.map((b) => b.type)).toEqual(["tool", "text"]);
    expect((out[1] as { text: string }).text).toBe("Let me check the current state of your project.");
  });
  it("stitches across multiple interim tools", () => {
    const blocks: Block[] = [text("Reading the config and the"), tool("Read"), tool("Grep"), text(" lockfile now.")];
    const out = mergeSplitProse(blocks);
    expect(out.map((b) => b.type)).toEqual(["tool", "tool", "text"]);
    expect((out[2] as { text: string }).text).toBe("Reading the config and the lockfile now.");
  });
  it("leaves deliberate sentence→tool→new-sentence prose untouched", () => {
    const blocks: Block[] = [text("Let me check the repo."), tool("git_status"), text("Found three commits.")];
    expect(mergeSplitProse(blocks)).toEqual(blocks);
  });
  it("does not stitch when the tail starts a new capitalized sentence", () => {
    const blocks: Block[] = [text("Checking now"), tool("Bash"), text("Done — all clean.")];
    expect(mergeSplitProse(blocks)).toEqual(blocks);
  });
  it("leaves a head with no interim tool untouched", () => {
    const blocks: Block[] = [text("ends mid"), text("sentence")];
    expect(mergeSplitProse(blocks)).toEqual(blocks);
  });
});

describe("coalesceToolGroups", () => {
  it("folds 3+ consecutive chips into a toolgroup, keeps runs of 2 inline", () => {
    const three = [unit(tool("Read"), "a"), unit(tool("Grep"), "b"), unit(tool("Bash"), "c")];
    const folded = coalesceToolGroups(three);
    expect(folded).toHaveLength(1);
    expect(folded[0].kind).toBe("toolgroup");
    expect(folded[0].key).toBe("tg_a");
    expect(coalesceToolGroups(three.slice(0, 2))).toHaveLength(2);
  });
  it("breaks the run on an edit block and aggregates status error > pending > done", () => {
    const units = [
      unit(tool("Read"), "a"), unit(tool("Grep", "pending"), "b"), unit(tool("Bash"), "c"),
      unit(tool("Edit"), "d"),
    ];
    const out = coalesceToolGroups(units);
    expect(out.map((u) => u.kind)).toEqual(["toolgroup", "block"]);
    expect((out[0] as { status: string }).status).toBe("pending");
    const withError = [unit(tool("Read", "error", true), "a"), unit(tool("Grep"), "b"), unit(tool("Bash"), "c")];
    expect((coalesceToolGroups(withError)[0] as { status: string }).status).toBe("error");
  });
  it("absorbs quick interleaved thoughts into a run; tools (not units) hit the threshold", () => {
    const quick = (): ThinkingBlock =>
      ({ type: "thinking", text: "", hasSignature: false, startedAt: 0, status: "done", durationMs: 500 });
    // tool · quick-thought · tool · quick-thought · tool → 3 tools → one group.
    const woven = [
      unit(tool("Read"), "a"), unit(quick(), "q1"), unit(tool("Grep"), "b"),
      unit(quick(), "q2"), unit(tool("Bash"), "c"),
    ];
    const folded = coalesceToolGroups(woven);
    expect(folded).toHaveLength(1);
    expect(folded[0].kind).toBe("toolgroup");
    // Only 2 tools (a quick thought between) stays inline — thought doesn't pad the count.
    const twoTools = [unit(tool("Read"), "a"), unit(quick(), "q1"), unit(tool("Grep"), "b")];
    expect(coalesceToolGroups(twoTools).every((u) => u.kind === "block")).toBe(true);
  });
});

describe("group duration", () => {
  it("sums tool durations, ignores non-tools, and formats sub-second as ms", () => {
    const blocks: Block[] = [
      { ...tool("Read"), durationMs: 120 } as Block,
      { ...tool("Grep"), durationMs: 80 } as Block,
      { type: "thinking", text: "", hasSignature: false, startedAt: 0, status: "done", durationMs: 9000 } as Block,
    ];
    expect(groupDurationMs(blocks)).toBe(200);
    expect(formatDurationMs(200)).toBe("200ms");
    expect(formatDurationMs(1500)).toBe("1s");
  });
});

describe("numberActions", () => {
  it("numbers tools sequentially and attaches the preceding divider title as caption", () => {
    const units: TimelineUnit[] = [
      { kind: "divider", stepNum: 1, title: "Setup", key: "d0" },
      unit(tool("Bash"), "a"),
      unit(tool("Read"), "b"),
    ];
    const out = numberActions(units);
    expect(out).toHaveLength(2);
    expect(out[0]).toMatchObject({ stepNum: 1, caption: "Setup" });
    expect(out[1].stepNum).toBe(2);
    expect(typeof (out[1] as { caption?: string }).caption).toBe("string");
  });
  it("re-emits an orphan divider before prose and keeps a trailing divider", () => {
    const units: TimelineUnit[] = [
      { kind: "divider", stepNum: 1, title: "Plan", key: "d0" },
      unit(text("some prose"), "p"),
      { kind: "divider", stepNum: 2, title: "Tail", key: "d1" },
    ];
    const out = numberActions(units);
    expect(out.map((u) => u.kind)).toEqual(["divider", "block", "divider"]);
    expect(out[2]).toMatchObject({ title: "Tail", key: "od_tail" });
  });
});

describe("status + kind", () => {
  it("maps tool/thinking states", () => {
    expect(statusOf(tool("Read", "pending"))).toBe("pending");
    expect(statusOf(tool("Read", "done", true))).toBe("error");
    expect(statusOf(text("x"))).toBe("neutral");
  });
  it("nodeKind routes edits vs chips", () => {
    expect(nodeKind(tool("Edit"))).toBe("edit");
    expect(nodeKind(tool("Read"))).toBe("tool");
    expect(nodeKind(text("x"))).toBe("prose");
  });
});

describe("formatDuration + elapsedFor", () => {
  it("formats sub-second, seconds, and minute combinations", () => {
    expect(formatDuration(500)).toBe("<1s");
    expect(formatDuration(5_000)).toBe("5s");
    expect(formatDuration(60_000)).toBe("1m");
    expect(formatDuration(90_000)).toBe("1m 30s");
  });
  it("uses stored duration when done, live elapsed when active", () => {
    const base: Omit<ThinkingBlock, "status" | "durationMs"> =
      { type: "thinking", text: "", hasSignature: false, startedAt: 1_000 };
    expect(elapsedFor({ ...base, status: "done", durationMs: 3_000 }, 99_000)).toBe("3s");
    expect(elapsedFor({ ...base, status: "active", durationMs: null }, 6_000)).toBe("5s");
  });
});

describe("formatters", () => {
  it("summarizeGroup rolls up counts and caps at 4 names", () => {
    const blocks = [tool("Read"), tool("Read"), tool("Grep"), tool("Bash"), tool("Glob"), tool("LS")];
    expect(summarizeGroup(blocks)).toBe("Read ×2 · Grep · Bash · Glob +1");
  });
  it("shortModel tightens ids and passes unknowns through", () => {
    expect(shortModel("claude-sonnet-4-6-20251001")).toBe("Sonnet 4.6");
    expect(shortModel("claude-opus-4-7[1m]")).toBe("Opus 4.7");
    expect(shortModel("claude-sonnet-5")).toBe("Sonnet 5"); // dateless major-only
    expect(shortModel("claude-fable-5")).toBe("Fable 5");
    expect(shortModel("ollama/llama3")).toBe("ollama/llama3"); // unknown passes through
  });
  it("lineDelta counts real diffs, rejects non-strings, approximates huge inputs", () => {
    expect(lineDelta("a\nb\nc", "a\nx\nc")).toEqual({ adds: 1, dels: 1 });
    expect(lineDelta(null, "x")).toEqual({ adds: 0, dels: 0 });
    const big = "y\n".repeat(120_000);
    expect(lineDelta(big, big).adds).toBeGreaterThan(0);
  });
});
