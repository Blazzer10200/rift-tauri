import { describe, it, expect } from "vitest";
import { messageToTurn, parseAskUserResult, groupNames, workLineMode, isFillerSay, classifySay, outputPeek, groupBlocks, shellFlavor, resultMeta } from "./streamModel";
import type { StreamTool } from "./streamModel";
import type { ChatMessage } from "$lib/state/assistant.svelte";

// Minimal ChatMessage builder — only the fields messageToTurn reads.
function msg(blocks: unknown[], costUsd: number | null = 0.05): ChatMessage {
  return { id: "m1", role: "assistant", blocks, costUsd } as unknown as ChatMessage;
}
const tool = (name: string, status: "done" | "error" | "pending" = "done", input: Record<string, unknown> = {}) =>
  ({ type: "tool", id: name + Math.random(), name, input, result: null, isError: status === "error", status });
const text = (t: string) => ({ type: "text", text: t });

describe("messageToTurn — outcome classification", () => {
  it("pure text reply → 'text', no footer meta", () => {
    const t = messageToTurn(msg([text("Here's the answer.")]));
    expect(t.outcome).toBe("text");
    expect(t.meta).toBeNull(); // no status stamp on a plain answer
    expect(t.files).toBe(0);
  });

  it("read-only tools (no mutation) → 'ran', not 'applied'", () => {
    const t = messageToTurn(msg([tool("Read"), tool("Grep"), tool("Bash"), text("Done looking.")]));
    expect(t.outcome).toBe("ran");
    expect(t.files).toBe(0);
    expect(t.meta).not.toBeNull(); // work happened → meta line shows
  });

  it("a successful Edit → 'applied' with file count", () => {
    const t = messageToTurn(msg([tool("Edit", "done", { file_path: "/a/foo.ts" }), text("Patched it.")]));
    expect(t.outcome).toBe("applied");
    expect(t.files).toBe(1);
  });

  it("Write (create) counts as applied", () => {
    const t = messageToTurn(msg([tool("Write", "done", { file_path: "/a/new.ts" })]));
    expect(t.outcome).toBe("applied");
    expect(t.files).toBe(1);
  });

  it("distinct files are de-duped; same file edited twice = 1", () => {
    const t = messageToTurn(msg([
      tool("Edit", "done", { file_path: "/a/foo.ts" }),
      tool("Edit", "done", { file_path: "/a/foo.ts" }),
      tool("Write", "done", { file_path: "/a/bar.ts" }),
    ]));
    expect(t.outcome).toBe("applied");
    expect(t.files).toBe(2);
  });

  it("edits attempted but all errored → 'failed', not 'applied'", () => {
    const t = messageToTurn(msg([tool("Edit", "error", { file_path: "/a/foo.ts" })]));
    expect(t.outcome).toBe("failed");
    expect(t.files).toBe(0);
  });

  it("mixed: one edit ok, one errored → 'applied' (a real change landed)", () => {
    const t = messageToTurn(msg([
      tool("Edit", "error", { file_path: "/a/foo.ts" }),
      tool("Edit", "done", { file_path: "/a/bar.ts" }),
    ]));
    expect(t.outcome).toBe("applied");
    expect(t.files).toBe(1);
  });

  it("reads + a successful edit → 'applied' (mutation wins over read-only)", () => {
    const t = messageToTurn(msg([tool("Read"), tool("Edit", "done", { file_path: "/a/foo.ts" }), tool("Bash")]));
    expect(t.outcome).toBe("applied");
    expect(t.files).toBe(1);
  });

  it("cost omitted on a working turn → meta still shows time, cost null", () => {
    const t = messageToTurn(msg([tool("Bash")], null));
    expect(t.outcome).toBe("ran");
    expect(t.meta?.cost).toBeNull();
    expect(typeof t.meta?.time).toBe("string");
  });

  it("same basename in different dirs counts as 2 distinct files (path, not name)", () => {
    const t = messageToTurn(msg([
      tool("Edit", "done", { file_path: "/a/x/foo.ts", old_string: "1", new_string: "2" }),
      tool("Edit", "done", { file_path: "/a/y/foo.ts", old_string: "1", new_string: "2" }),
    ]));
    expect(t.files).toBe(2);
  });
});

// Pull StreamTools out of an adapted turn for field assertions.
function toolsOf(blocks: unknown[]) {
  const t = messageToTurn(msg(blocks));
  return t.blocks.filter((b): b is { type: "tool"; tool: import("./streamModel").StreamTool } => b.type === "tool").map((b) => b.tool);
}

describe("streamModel — path surfacing", () => {
  it("read tool exposes full path + collapsed dir prefix", () => {
    const [t] = toolsOf([tool("Read", "done", { file_path: "C:\\repo\\src\\lib\\foo.ts" })]);
    expect(t.path).toBe("C:/repo/src/lib/foo.ts"); // backslashes normalized
    expect(t.cap).toBe("foo.ts");
    expect(t.dir).toBe("…/src/lib/"); // last two segments, trailing slash
  });

  it("short path keeps full dir (no ellipsis)", () => {
    const [t] = toolsOf([tool("Read", "done", { file_path: "/a/foo.ts" })]);
    expect(t.dir).toBe("a/");
  });

  it("bare filename (no dir) → dir null", () => {
    const [t] = toolsOf([tool("Read", "done", { file_path: "foo.ts" })]);
    expect(t.dir).toBeNull();
  });
});

describe("streamModel — edit diff counts + input passthrough", () => {
  it("Edit populates add/del from old/new_string and carries input", () => {
    const [t] = toolsOf([tool("Edit", "done", {
      file_path: "/a/foo.ts",
      old_string: "line1\nline2\nline3",
      new_string: "line1\nCHANGED\nline3\nline4",
    })]);
    expect(t.add).toBe(2); // CHANGED + line4
    expect(t.del).toBe(1); // line2
    expect(t.input).toBeTruthy(); // raw input flows through for the inline EditDiff
  });

  it("Write (new file) counts every line as an addition, zero deletions", () => {
    const [t] = toolsOf([tool("Write", "done", { file_path: "/a/new.ts", content: "a\nb\nc" })]);
    expect(t.add).toBe(3);
    expect(t.del).toBe(0);
  });

  it("MultiEdit sums counts across sub-edits", () => {
    const [t] = toolsOf([tool("MultiEdit", "done", {
      file_path: "/a/foo.ts",
      edits: [
        { old_string: "a", new_string: "A\nB" }, // +2 -1
        { old_string: "c", new_string: "C" },     // +1 -1
      ],
    })]);
    expect(t.add).toBe(3);
    expect(t.del).toBe(2);
  });

  it("read tool carries no diff input (counts stay null)", () => {
    const [t] = toolsOf([tool("Read", "done", { file_path: "/a/foo.ts" })]);
    expect(t.add).toBeNull();
    expect(t.del).toBeNull();
    expect(t.input ?? null).toBeNull();
  });

  it("diff counts are stable across re-renders of the same tool id (memo)", () => {
    // messageToTurn re-runs on every stream frame; the per-id memo must return
    // identical counts each time (correctness preserved, not just cached).
    const edit = {
      type: "tool", id: "fixed-edit-id", name: "Edit",
      input: { file_path: "/a/foo.ts", old_string: "a\nb\nc", new_string: "a\nX\nc\nd" },
      result: null, isError: false, status: "done",
    };
    const first = messageToTurn(msg([edit])).blocks.find((b) => b.type === "tool")!.tool;
    const second = messageToTurn(msg([edit])).blocks.find((b) => b.type === "tool")!.tool;
    expect(first.add).toBe(second.add);
    expect(first.del).toBe(second.del);
    expect(first.add).toBe(2); // X + d
    expect(first.del).toBe(1); // b
  });
});

// The answered-state parser mirrors the EXACT backend format
// (mcp_server.rs::format_ask_user_result): "Q: <q>\nA: <label>", blocks joined
// by "\n", multi-select labels joined by ", ". If the backend format changes,
// these break — that's the point (the parser is brittle by necessity).
describe("parseAskUserResult — answered ask_user → chips", () => {
  it("single question, single answer", () => {
    expect(parseAskUserResult("Q: Pick a color\nA: Blue")).toEqual([
      { question: "Pick a color", answers: ["Blue"] },
    ]);
  });

  it("multi-select answer splits on the US delimiter", () => {
    expect(parseAskUserResult("Q: Pick features\nA: Auth\u{1f}Billing\u{1f}Search")).toEqual([
      { question: "Pick features", answers: ["Auth", "Billing", "Search"] },
    ]);
  });

  it("a label containing ', ' survives intact (A1)", () => {
    expect(parseAskUserResult("Q: Pick\nA: Auth, Billing\u{1f}Search")).toEqual([
      { question: "Pick", answers: ["Auth, Billing", "Search"] },
    ]);
  });

  it("legacy comma-space join still parses (back-compat)", () => {
    expect(parseAskUserResult("Q: Pick features\nA: Auth, Billing, Search")).toEqual([
      { question: "Pick features", answers: ["Auth", "Billing", "Search"] },
    ]);
  });

  it("multiple questions each parse to their own pair", () => {
    const raw = "Q: First?\nA: Yes\nQ: Second?\nA: No";
    expect(parseAskUserResult(raw)).toEqual([
      { question: "First?", answers: ["Yes"] },
      { question: "Second?", answers: ["No"] },
    ]);
  });

  it("dismissal sentence → empty (caller shows neutral state)", () => {
    expect(parseAskUserResult("User dismissed the question without answering. Fall back…")).toEqual([]);
  });

  it("null/empty result → empty", () => {
    expect(parseAskUserResult(null)).toEqual([]);
    expect(parseAskUserResult("")).toEqual([]);
    expect(parseAskUserResult(undefined)).toEqual([]);
  });

  it("custom 'Other' free-text answer survives verbatim", () => {
    expect(parseAskUserResult("Q: How?\nA: Some bespoke approach I typed")).toEqual([
      { question: "How?", answers: ["Some bespoke approach I typed"] },
    ]);
  });

  it("unparseable text (no A: line) → empty, never throws", () => {
    expect(parseAskUserResult("just some random text")).toEqual([]);
  });
});

describe("groupNames — names on the collapsed work row", () => {
  it("single-kind reads → 'Read a.ts, b.ts'", () => {
    const ts = toolsOf([
      tool("Read", "done", { file_path: "/a/a.ts" }),
      tool("Read", "done", { file_path: "/a/b.ts" }),
    ]);
    expect(groupNames(ts)).toBe("Read a.ts, b.ts");
  });

  it("caps at 3 names with a '+N more' tail", () => {
    const ts = toolsOf([
      tool("Read", "done", { file_path: "/a/a.ts" }),
      tool("Read", "done", { file_path: "/a/b.ts" }),
      tool("Read", "done", { file_path: "/a/c.ts" }),
      tool("Read", "done", { file_path: "/a/d.ts" }),
      tool("Read", "done", { file_path: "/a/e.ts" }),
    ]);
    expect(groupNames(ts)).toBe("Read a.ts, b.ts, c.ts +2 more");
  });

  it("de-dupes repeated names", () => {
    const ts = toolsOf([
      tool("Read", "done", { file_path: "/a/a.ts" }),
      tool("Read", "done", { file_path: "/a/a.ts" }),
    ]);
    expect(groupNames(ts)).toBe("Read a.ts");
  });

  it("shell falls back to the count summary (command text isn't a target name)", () => {
    const ts = toolsOf([tool("Bash", "done", { command: "npm run check" })]);
    expect(groupNames(ts)).toBe("Ran 1 command");
  });

  it("mixed kinds name their targets per-kind, dominant-first", () => {
    const ts = toolsOf([
      tool("Read", "done", { file_path: "/a/a.ts" }),
      tool("Read", "done", { file_path: "/a/b.ts" }),
      tool("Grep", "done", { pattern: "foo" }),
      tool("Bash", "done", { command: "ls" }),
    ]);
    // Lead kind (read) names up to 2 targets; namable trailing kinds (grep)
    // name 1; non-namable (shell) just count.
    expect(groupNames(ts)).toBe('Read a.ts, b.ts · Searched "foo" · ran 1');
  });

  it("grep names its pattern target", () => {
    const ts = toolsOf([tool("Grep", "done", { pattern: "foo" })]);
    expect(groupNames(ts)).toBe('Searched "foo"');
  });
});

describe("workLineMode — tool-detail tier → WorkLine render mode", () => {
  it("minimal → collapsed", () => expect(workLineMode("minimal")).toBe("collapsed"));
  it("balanced → rows", () => expect(workLineMode("balanced")).toBe("rows"));
  it("detailed → expanded", () => expect(workLineMode("detailed")).toBe("expanded"));
});

describe("isFillerSay — trim throwaway transitional prose", () => {
  it("drops a single 'Let me…' sentence", () => {
    expect(isFillerSay("Let me look at the layout file.")).toBe(true);
  });

  it("drops 'Now I'll…' / 'First,…' lead-ins", () => {
    expect(isFillerSay("Now I'll check the routes.")).toBe(true);
    expect(isFillerSay("First, I need to read the config.")).toBe(true);
  });

  it("keeps real multi-sentence prose", () => {
    expect(isFillerSay("Let me explain. The bug is in the parser, here's why it matters.")).toBe(false);
  });

  it("keeps long single sentences (real content, even if it starts with a lead-in)", () => {
    const long = "Let me walk through exactly what the warm pool does on each turn, because the eviction timing is the whole reason this felt slow before.";
    expect(isFillerSay(long)).toBe(false);
  });

  it("keeps prose that doesn't open with a filler lead-in", () => {
    expect(isFillerSay("The answer is 42.")).toBe(false);
  });

  it("keeps anything containing a code fence", () => {
    expect(isFillerSay("Let me run ```npm test```")).toBe(false);
  });

  it("empty / whitespace → not filler (nothing to drop)", () => {
    expect(isFillerSay("   ")).toBe(false);
  });
});

describe("classifySay — narration weight (filler / connective / prose)", () => {
  it("pure lead-in → filler", () => {
    expect(classifySay("Let me look at the layout file.")).toBe("filler");
    expect(classifySay("Now I'll check the routes.")).toBe("filler");
  });

  it("the real between-tools beats from a build session → connective", () => {
    // These are the exact lines that read as 'chat' in the screenshots.
    expect(classifySay("Now kill any running instance and build:")).toBe("connective");
    expect(classifySay("Frontend built clean. Now the Tauri release build (compiles Rust + embeds frontend):")).toBe("connective");
    expect(classifySay("The bin exe is still running (locked). Kill it harder, then copy:")).toBe("connective");
    expect(classifySay("4.56MB exe copied. Launch it:")).toBe("connective");
  });

  it("a trailing-colon pointer is connective even without a step-report lead", () => {
    expect(classifySay("Wiring the Normal-reset behavior into store.rs:")).toBe("connective");
  });

  it("real answers / explanations → prose (never demoted)", () => {
    expect(classifySay("The answer is 42.")).toBe("prose");
    expect(classifySay("Your second monitor was a different color because vibrance was only ever written to the primary output.")).toBe("prose");
  });

  it("multi-line or code-bearing → prose", () => {
    expect(classifySay("Done.\nHere's what changed and why.")).toBe("prose");
    expect(classifySay("Run ```npm test``` to confirm.")).toBe("prose");
  });
});

describe("outputPeek — trailing-lines preview for command output", () => {
  it("null / empty → no lines", () => {
    expect(outputPeek(null)).toEqual({ lines: [], more: 0 });
    expect(outputPeek("")).toEqual({ lines: [], more: 0 });
    expect(outputPeek("   \n  ")).toEqual({ lines: [], more: 0 });
  });

  it("short output → all lines, nothing elided", () => {
    expect(outputPeek("one\ntwo")).toEqual({ lines: ["one", "two"], more: 0 });
  });

  it("long output → last N lines + elided count, skipping blanks", () => {
    const out = "a\nb\n\nc\nd\ne"; // 5 non-empty lines
    expect(outputPeek(out, 3)).toEqual({ lines: ["c", "d", "e"], more: 2 });
  });
});

describe("adaptTool — shell carries its stdout; rich only when it has output", () => {
  // Build a Bash tool block with a real result (the `tool` helper forces null).
  const bash = (result: string | null, status: "done" | "pending" = "done") =>
    ({ type: "tool", id: "b" + Math.random(), name: "Bash", input: { command: "ls" }, result, isError: false, status });

  it("a Bash result is forwarded onto the adapted shell tool", () => {
    const t = messageToTurn(msg([bash("file1.txt\nfile2.txt")])).blocks.find((b) => b.type === "tool")!.tool;
    expect(t.kind).toBe("shell");
    expect(t.result).toBe("file1.txt\nfile2.txt");
  });

  it("a shell command WITH output groups as its own rich block", () => {
    const groups = groupBlocks(messageToTurn(msg([bash("some output")])).blocks);
    const work = groups.find((g) => g.type === "work");
    expect(work?.type).toBe("work");
    // rich = its own seg carrying the single tool (not batched into 'other')
    expect(work && work.type === "work" && work.segs.some((s) => s.seg === "rich" && s.tool.kind === "shell")).toBe(true);
  });

  it("a bare command with NO output stays in the lightweight WorkLine batch", () => {
    const groups = groupBlocks(messageToTurn(msg([bash(null)])).blocks);
    const work = groups.find((g) => g.type === "work");
    expect(work && work.type === "work" && work.segs.every((s) => s.seg !== "rich")).toBe(true);
  });
});

describe("streamModel — shell flavor + detail surfacing (transcript revamp)", () => {
  const toolOf = (m: ChatMessage) => {
    const b = messageToTurn(m).blocks.find((x) => x.type === "tool");
    return b && b.type === "tool" ? b.tool : null;
  };

  it("PowerShell is a shell-kind tool with a pwsh flavor and the command as caption", () => {
    const tb = { ...tool("PowerShell", "done", { command: "Get-Date" }), result: "Thursday, July 2" };
    const t = toolOf(msg([tb]))!;
    expect(t.kind).toBe("shell");
    expect(t.flavor).toBe("pwsh");
    expect(t.cap).toBe("Get-Date");
    expect(t.result).toBe("Thursday, July 2");
  });

  it("shellFlavor: Bash → bash; Bash shelling to cmd.exe → cmd", () => {
    expect(shellFlavor("Bash", "ls -la")).toBe("bash");
    expect(shellFlavor("Bash", "cmd /c dir")).toBe("cmd");
    expect(shellFlavor("Bash", "CMD.EXE /C echo hi")).toBe("cmd");
    expect(shellFlavor("PowerShell", null)).toBe("pwsh");
  });

  it("mcp tools carry input + result through, and caption peeks the input", () => {
    const tb = { ...tool("ScheduleWakeup", "done", { reason: "watching the CI run finish" }), result: "wakeup set" };
    const t = toolOf(msg([tb]))!;
    expect(t.kind).toBe("mcp");
    expect(t.cap).toContain("ScheduleWakeup");
    expect(t.cap).toContain("watching the CI run");
    expect(t.result).toBe("wakeup set");
    expect(t.input).toEqual({ reason: "watching the CI run finish" });
  });

  it("resultMeta: read → line count; grep → match count; Glob → files; no result → null", () => {
    const read = { ...tool("Read", "done", { file_path: "/a/b.ts" }), result: "l1\nl2\nl3" };
    expect(resultMeta(toolOf(msg([read]))!)).toBe("3 lines");
    const grep = { ...tool("Grep", "done", { pattern: "x" }), result: "a.ts:1: x\nb.ts:2: x" };
    expect(resultMeta(toolOf(msg([grep]))!)).toBe("2 matches");
    const glob = { ...tool("Glob", "done", { pattern: "*.md" }), result: "a.md" };
    expect(resultMeta(toolOf(msg([glob]))!)).toBe("1 file");
    const none = { ...tool("Grep", "done", { pattern: "x" }), result: "No matches found" };
    expect(resultMeta(toolOf(msg([none]))!)).toBe("no matches");
    expect(resultMeta(toolOf(msg([tool("Read")]))!)).toBeNull();
  });
});
