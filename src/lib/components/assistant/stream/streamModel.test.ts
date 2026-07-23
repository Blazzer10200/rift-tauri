import { describe, it, expect } from "vitest";
import { messageToTurn, parseAskUserResult, groupNames, workLineMode, isFillerSay, classifySay, outputPeek, groupBlocks, shellFlavor, resultMeta, splitOutput, nextRevealTier, isPlanArtifact, REVEAL_COLLAPSED, REVEAL_EXPANDED, REVEAL_SLACK, stripAnsi, ansiLines, classifyShellLine, shellCheckKind, parseCheckSummary, parseGrepLine, parseReadOutput, splitOutputFold, FOLD_TAIL, trimCmd, shellLabel } from "./streamModel";
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

  it("read tool has no diff counts; input flows through for ReadResult", () => {
    const [t] = toolsOf([tool("Read", "done", { file_path: "/a/foo.ts" })]);
    expect(t.add).toBeNull();
    expect(t.del).toBeNull();
    expect(t.input).toEqual({ file_path: "/a/foo.ts" });
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
    // NB: a check/test-runner command would upgrade to lint/test kind now —
    // a plain command exercises the shell fallback this test is about.
    const ts = toolsOf([tool("Bash", "done", { command: "git status" })]);
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

  it("exactly one line past N → shows it instead of a '+1 more' indicator", () => {
    expect(outputPeek("a\nb\nc\nd", 3)).toEqual({ lines: ["a", "b", "c", "d"], more: 0 });
  });

  // Occurrence #4 of the blank-transcript wedge: `git push` / `gh run watch`
  // output ends with identical repeated lines; StreamShell's peek each was keyed
  // by line CONTENT → each_key_duplicate threw and aborted every Svelte flush.
  // outputPeek must pass duplicates through verbatim — render keys must be
  // positional (index), never the line text. Pins the data contract.
  it("keeps duplicate trailing lines verbatim (render must key by index)", () => {
    const out = "remote:\nremote:\nremote:";
    expect(outputPeek(out, 3)).toEqual({ lines: ["remote:", "remote:", "remote:"], more: 0 });
  });
});

describe("splitOutput — progressive reveal tiers", () => {
  const lines = (n: number) => Array.from({ length: n }, (_, i) => `line ${i + 1}`).join("\n");

  it("null / empty → nothing", () => {
    expect(splitOutput(null, "collapsed")).toEqual({ lines: [], shown: 0, hidden: 0, total: 0 });
    expect(splitOutput("", "all")).toEqual({ lines: [], shown: 0, hidden: 0, total: 0 });
  });

  it("output shorter than the collapsed cap shows fully, nothing hidden", () => {
    const r = splitOutput(lines(5), "collapsed");
    expect(r.total).toBe(5);
    expect(r.shown).toBe(5);
    expect(r.hidden).toBe(0);
  });

  it("collapsed caps at REVEAL_COLLAPSED, reports the remainder as hidden", () => {
    const r = splitOutput(lines(100), "collapsed");
    expect(r.shown).toBe(REVEAL_COLLAPSED);
    expect(r.hidden).toBe(100 - REVEAL_COLLAPSED);
  });

  it("a tail within the slack shows fully — no 'Show 4 more lines' stub", () => {
    // 16 lines vs a 12-line cap: the 4-line tail renders instead of a button.
    const r = splitOutput(lines(REVEAL_COLLAPSED + 4), "collapsed");
    expect(r.shown).toBe(REVEAL_COLLAPSED + 4);
    expect(r.hidden).toBe(0);
    // Boundary: exactly cap + slack still shows everything…
    expect(splitOutput(lines(REVEAL_COLLAPSED + REVEAL_SLACK), "collapsed").hidden).toBe(0);
    // …one past it caps normally.
    const over = splitOutput(lines(REVEAL_COLLAPSED + REVEAL_SLACK + 1), "collapsed");
    expect(over.shown).toBe(REVEAL_COLLAPSED);
    expect(over.hidden).toBe(REVEAL_SLACK + 1);
  });

  it("expanded caps at REVEAL_EXPANDED", () => {
    const r = splitOutput(lines(100), "expanded");
    expect(r.shown).toBe(REVEAL_EXPANDED);
    expect(r.hidden).toBe(100 - REVEAL_EXPANDED);
  });

  it("expanded tier absorbs a slack-sized tail too", () => {
    const r = splitOutput(lines(REVEAL_EXPANDED + REVEAL_SLACK), "expanded");
    expect(r.shown).toBe(REVEAL_EXPANDED + REVEAL_SLACK);
    expect(r.hidden).toBe(0);
  });

  it("all shows everything, nothing hidden", () => {
    const r = splitOutput(lines(100), "all");
    expect(r.shown).toBe(100);
    expect(r.hidden).toBe(0);
  });
});

describe("nextRevealTier — step the cap up", () => {
  it("collapsed → expanded only when there's more than the cap + slack", () => {
    expect(nextRevealTier("collapsed", REVEAL_COLLAPSED + REVEAL_SLACK + 1)).toBe("expanded");
    // Within the slack the tier already shows everything — no next step.
    expect(nextRevealTier("collapsed", REVEAL_COLLAPSED + REVEAL_SLACK)).toBeNull();
    expect(nextRevealTier("collapsed", REVEAL_COLLAPSED)).toBeNull();
  });

  it("expanded → all only when there's more than the cap + slack", () => {
    expect(nextRevealTier("expanded", REVEAL_EXPANDED + REVEAL_SLACK + 1)).toBe("all");
    expect(nextRevealTier("expanded", REVEAL_EXPANDED + REVEAL_SLACK)).toBeNull();
    expect(nextRevealTier("expanded", REVEAL_EXPANDED)).toBeNull();
  });

  it("all is terminal", () => {
    expect(nextRevealTier("all", 9999)).toBeNull();
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

  it("a bare FINISHED command with NO output stays in the lightweight WorkLine batch", () => {
    const groups = groupBlocks(messageToTurn(msg([bash(null)])).blocks);
    const work = groups.find((g) => g.type === "work");
    expect(work && work.type === "work" && work.segs.every((s) => s.seg !== "rich")).toBe(true);
  });

  it("a PENDING command (no output yet) is rich immediately — no WorkLine→block flicker", () => {
    const groups = groupBlocks(messageToTurn(msg([bash(null, "pending")])).blocks);
    const work = groups.find((g) => g.type === "work");
    expect(work && work.type === "work" && work.segs.some((s) => s.seg === "rich" && s.tool.kind === "shell")).toBe(true);
  });
});

describe("groupBlocks — say-fragment stitching (sentence split across a tool)", () => {
  const says = (groups: ReturnType<typeof groupBlocks>) =>
    groups.filter((g) => g.type === "say").map((g) => (g as { text: string }).text);

  it("rejoins a sentence the CLI split mid-word across a tool call", () => {
    // "I'll do all three in p" → [tool] → "arallel." must read as one beat.
    const groups = groupBlocks(messageToTurn(msg([
      text("I'll do all three in p"), tool("Read"), text("arallel."),
    ])).blocks);
    expect(says(groups)).toEqual(["I'll do all three in parallel."]);
  });

  it("stitches across multiple interleaved tools until the sentence closes", () => {
    const groups = groupBlocks(messageToTurn(msg([
      text("Now"), tool("Read"), text(" let me check the "), tool("Grep"), text("config."),
    ])).blocks);
    expect(says(groups)).toEqual(["Now let me check the config."]);
  });

  it("does NOT merge two complete sentences (prior ends with a period)", () => {
    const groups = groupBlocks(messageToTurn(msg([
      text("Done."), tool("Read"), text("Next I'll build."),
    ])).blocks);
    expect(says(groups)).toEqual(["Done.", "Next I'll build."]);
  });

  it("does NOT merge when the fragment starts a new capitalized sentence", () => {
    const groups = groupBlocks(messageToTurn(msg([
      text("reading the file"), tool("Read"), text("The result is clear"),
    ])).blocks);
    // prior is mid-sentence but the continuation starts capital → keep separate
    expect(says(groups)).toEqual(["reading the file", "The result is clear"]);
  });

  it("keeps a colon-terminated forward-pointing beat separate", () => {
    const groups = groupBlocks(messageToTurn(msg([
      text("Now the build:"), tool("Bash"), text("compiled clean."),
    ])).blocks);
    expect(says(groups)).toEqual(["Now the build:", "compiled clean."]);
  });

  it("a trailing newline is a paragraph break, not a stream split", () => {
    const groups = groupBlocks(messageToTurn(msg([
      text("First paragraph\n"), tool("Read"), text("second thought"),
    ])).blocks);
    // Two beats — the newline blocks the stitch (raw text keeps its newline,
    // which the Markdown renderer collapses). The point is they stay SEPARATE.
    expect(says(groups).length).toBe(2);
    expect(says(groups)[0].trim()).toBe("First paragraph");
    expect(says(groups)[1]).toBe("second thought");
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

describe("S128 — CLI tool-name compat (kinds + captions)", () => {
  const toolOf = (m: ChatMessage): StreamTool | null => {
    const b = messageToTurn(m).blocks.find((x) => x.type === "tool");
    return b && b.type === "tool" ? b.tool : null;
  };

  it("TaskOutput/TaskStop are shell-shaped bg-task ops, NOT plan", () => {
    const tails = toolOf(msg([{ ...tool("TaskOutput", "done", { task_id: "t7" }), result: "build ok" }]))!;
    expect(tails.kind).toBe("shell");
    expect(tails.cap).toBe("tail t7");
    const stop = toolOf(msg([tool("TaskStop", "done", { task_id: "t7" })]))!;
    expect(stop.kind).toBe("shell");
    expect(stop.cap).toBe("stop t7");
    // legacy names keep their captions too
    expect(toolOf(msg([tool("BashOutput", "done", { bash_id: "b1" })]))!.cap).toBe("tail b1");
  });

  it("TaskCreate/TaskUpdate/TaskList/TaskGet remain plan kind", () => {
    for (const n of ["TaskCreate", "TaskUpdate", "TaskList", "TaskGet", "TodoWrite"]) {
      expect(toolOf(msg([tool(n)]))!.kind).toBe("plan");
    }
  });

  it("ExitPlanMode → exitplan kind, carries input.plan for the proposal card", () => {
    const t = toolOf(msg([tool("ExitPlanMode", "pending", { plan: "## Step 1\ndo the thing" })]))!;
    expect(t.kind).toBe("exitplan");
    expect(t.cap).toBe("Proposed a plan");
    expect(t.input).toEqual({ plan: "## Step 1\ndo the thing" });
  });

  it("a plan-mode turn outcome is 'planned', even though the CLI wrote its plan artifact", () => {
    // Live-caught 2026-07-08: the CLI Writes ~/.claude/plans/<slug>.md during
    // ExitPlanMode, which read as "Applied 1 file" on a turn that changed
    // nothing in the user's repo.
    const t = messageToTurn(msg([
      tool("Read"),
      tool("Write", "done", { file_path: "C:/Users/x/.claude/plans/my-plan.md" }),
      tool("ExitPlanMode", "done", { plan: "## The plan" }),
    ]));
    expect(t.outcome).toBe("planned");
  });

  it("isPlanArtifact spots ~/.claude/plans/ writes (backslashes normalized upstream)", () => {
    // #91: the plan-artifact Write's diff folds by default in WriteBatch —
    // its content duplicates the StreamExitPlan card rendered right below.
    expect(isPlanArtifact("C:/Users/x/.claude/plans/my-plan.md")).toBe(true);
    const t = toolOf(msg([tool("Write", "done", { file_path: "C:\\Users\\x\\.claude\\plans\\p.md" })]))!;
    expect(isPlanArtifact(t.path)).toBe(true);
    expect(isPlanArtifact("C:/repo/src/plans/roadmap.md")).toBe(false);
    expect(isPlanArtifact(null)).toBe(false);
  });

  it("PowerShell captions the command like Bash and flavors pwsh", () => {
    const t = toolOf(msg([tool("PowerShell", "done", { command: "Get-Process rift" })]))!;
    expect(t.kind).toBe("shell");
    expect(t.cap).toBe("Get-Process rift");
    expect(t.flavor).toBe("pwsh");
  });

  it("Skill / SlashCommand / Workflow / AskUserQuestion / LSP get real captions, not bare names", () => {
    expect(toolOf(msg([tool("Skill", "done", { skill: "check", args: "--fix" })]))!.cap).toBe("/check --fix");
    expect(toolOf(msg([tool("SlashCommand", "done", { command: "/cost" })]))!.cap).toBe("/cost");
    expect(toolOf(msg([tool("Workflow", "done", { name: "review-changes" })]))!.cap).toBe("review-changes");
    expect(toolOf(msg([tool("AskUserQuestion", "done", { questions: [{}, {}] })]))!.cap).toBe("2 questions");
    expect(toolOf(msg([tool("LSP", "done", { operation: "findReferences", filePath: "/s/a.ts" })]))!.cap).toBe("findReferences · a.ts");
  });

  it("a live-forming edit defers diff counts and marks forming", () => {
    const forming = {
      ...tool("Edit", "pending", { file_path: "/a/x.ts", old_string: "trunc" }),
      inputPartial: true,
    };
    const t = toolOf(msg([forming]))!;
    expect(t.forming).toBe(true);
    expect(t.add).toBeNull();
    expect(t.del).toBeNull();
    // once complete, the same block yields counts again
    const done = tool("Edit", "done", { file_path: "/a/x.ts", old_string: "a", new_string: "a\nb" });
    const t2 = toolOf(msg([done]))!;
    expect(t2.forming).toBeUndefined();
    expect(t2.add).toBe(1);
  });
});

describe("ansiLines — SGR color segments (carried across lines) + stripAnsi", () => {
  it("plain text passes through with null cls", () => {
    expect(ansiLines("hello\nworld")).toEqual([
      [{ text: "hello", cls: null }],
      [{ text: "world", cls: null }],
    ]);
  });

  it("maps fg color + resets on 0", () => {
    expect(ansiLines("\x1b[31mred\x1b[0m plain")[0]).toEqual([
      { text: "red", cls: "a-red" },
      { text: " plain", cls: null },
    ]);
  });

  it("carries color state across lines (real terminals do)", () => {
    const lines = ansiLines("\x1b[32mline1\nline2\x1b[0m");
    expect(lines[0][0]).toEqual({ text: "line1", cls: "a-green" });
    expect(lines[1][0]).toEqual({ text: "line2", cls: "a-green" });
  });

  it("bright fg + bold compose; bare \x1b[m resets", () => {
    expect(ansiLines("\x1b[1;91mboom\x1b[m ok")[0]).toEqual([
      { text: "boom", cls: "a-red a-bold" },
      { text: " ok", cls: null },
    ]);
  });

  it("stripAnsi scrubs SGR sequences", () => {
    expect(stripAnsi("\x1b[31;1mred\x1b[0m")).toBe("red");
  });
});

describe("classifyShellLine — conservative semantic tone", () => {
  it("strong signals only; '0 errors' reads ok, not err", () => {
    expect(classifyShellLine("✓ built in 2.1s")).toBe("ok");
    expect(classifyShellLine("0 errors and 0 warnings")).toBe("ok");
    expect(classifyShellLine("error[E0308]: mismatched types")).toBe("err");
    expect(classifyShellLine("warning: unused variable `x`")).toBe("warn");
    expect(classifyShellLine("Compiling rift v0.100.0")).toBe("out");
  });
});

describe("shellCheckKind — test/lint command classification", () => {
  it("test runners", () => {
    expect(shellCheckKind("npx vitest run")).toBe("test");
    expect(shellCheckKind("cargo test --workspace")).toBe("test");
    expect(shellCheckKind("npm run test")).toBe("test");
  });
  it("linters / type-checkers", () => {
    expect(shellCheckKind("cargo check --manifest-path src-tauri/Cargo.toml")).toBe("lint");
    expect(shellCheckKind("cargo clippy -- -D warnings")).toBe("lint");
    expect(shellCheckKind("npm run check")).toBe("lint");
    expect(shellCheckKind("npx svelte-check --tsconfig ./tsconfig.json")).toBe("lint");
  });
  it("ordinary commands stay null", () => {
    expect(shellCheckKind("git checkout main")).toBeNull();
    expect(shellCheckKind("ls -la")).toBeNull();
    expect(shellCheckKind(null)).toBeNull();
  });
});

describe("parseCheckSummary — pass/fail counts from runner output", () => {
  it("vitest summary line (ignores the Test Files line)", () => {
    const out = " Test Files  1 failed | 3 passed (4)\n Tests  2 failed | 40 passed (42)\n";
    expect(parseCheckSummary(out)).toEqual({ pass: 40, fail: 2 });
  });
  it("cargo test — sums multiple result lines", () => {
    const out = "test result: ok. 12 passed; 0 failed; 0 ignored\n...\ntest result: FAILED. 3 passed; 2 failed; 0 ignored";
    expect(parseCheckSummary(out)).toEqual({ pass: 15, fail: 2 });
  });
  it("pytest banner", () => {
    expect(parseCheckSummary("========= 1 failed, 3 passed in 0.12s =========")).toEqual({ pass: 3, fail: 1 });
  });
  it("svelte-check error count (fail-only shape)", () => {
    expect(parseCheckSummary("svelte-check found 2 errors and 0 warnings in 1 file")).toEqual({ pass: null, fail: 2 });
  });
  it("unrecognizable output → null (caller falls back to status pills)", () => {
    expect(parseCheckSummary("compiled fine, nothing to see")).toBeNull();
    expect(parseCheckSummary(null)).toBeNull();
  });
});

describe("parseGrepLine — path:line:content rows", () => {
  it("unix relative path", () => {
    expect(parseGrepLine("src/lib/a.ts:42: const x = 1")).toEqual({ path: "src/lib/a.ts", line: 42, text: " const x = 1" });
  });
  it("windows drive-letter absolute path (colon in drive survives)", () => {
    expect(parseGrepLine("C:\\AI Workflow\\x.rs:7:fn main() {}")).toEqual({ path: "C:\\AI Workflow\\x.rs", line: 7, text: "fn main() {}" });
  });
  it("content containing colons stays intact", () => {
    expect(parseGrepLine("a.ts:3:let x: number = 1")).toEqual({ path: "a.ts", line: 3, text: "let x: number = 1" });
  });
  it("non-match lines → null", () => {
    expect(parseGrepLine("no colons here")).toBeNull();
    expect(parseGrepLine("Found 3 matches")).toBeNull();
  });
});

describe("parseReadOutput — CLI gutter sniff vs raw file content", () => {
  it("strips a cat -n style arrow gutter and reports the real start line", () => {
    expect(parseReadOutput("   120→import x\n   121→export y")).toEqual({ start: 120, code: "import x\nexport y" });
  });
  it("tab-separated gutter works too", () => {
    expect(parseReadOutput("     1\tfoo\n     2\tbar")).toEqual({ start: 1, code: "foo\nbar" });
  });
  it("raw content (rift MCP read_file) → null so the caller numbers from offset", () => {
    expect(parseReadOutput("just code\nno gutter at all")).toBeNull();
    expect(parseReadOutput("")).toBeNull();
  });
});

describe("splitOutputFold — head+tail reveal", () => {
  const long = Array.from({ length: 50 }, (_, i) => `l${i}`).join("\n");
  it("long output folds around a hidden middle, tail preserved", () => {
    const v = splitOutputFold(long, "collapsed");
    expect(v.head).toBe(REVEAL_COLLAPSED - FOLD_TAIL);
    expect(v.tail).toBe(FOLD_TAIL);
    expect(v.hidden).toBe(50 - REVEAL_COLLAPSED);
    expect(v.total).toBe(50);
  });
  it("short output shows fully — no fold", () => {
    const v = splitOutputFold("a\nb\nc", "collapsed");
    expect(v.head).toBe(3);
    expect(v.tail).toBe(0);
    expect(v.hidden).toBe(0);
  });
  it("'all' tier never hides anything", () => {
    const v = splitOutputFold(long, "all");
    expect(v.hidden).toBe(0);
    expect(v.head).toBe(50);
  });
});

describe("adaptTool — test/lint upgrade + raw ANSI on shell results", () => {
  const toolOf = (m: ChatMessage): StreamTool | null => {
    const b = messageToTurn(m).blocks.find((x) => x.type === "tool");
    return b && b.type === "tool" ? b.tool : null;
  };

  it("a vitest command upgrades to kind 'test' with parsed counts", () => {
    const tb = { ...tool("Bash", "done", { command: "npx vitest run" }), result: " Tests  2 failed | 40 passed (42)" };
    const t = toolOf(msg([tb]))!;
    expect(t.kind).toBe("test");
    expect(t.fail).toBe(2);
    expect(t.pass).toBe(40);
  });

  it("cargo check upgrades to 'lint'; plain shell keeps kind + RAW ansi result", () => {
    const lint = toolOf(msg([tool("Bash", "done", { command: "cargo check" })]))!;
    expect(lint.kind).toBe("lint");
    const tb = { ...tool("Bash", "done", { command: "ls" }), result: "\x1b[32mok\x1b[0m" };
    const t = toolOf(msg([tb]))!;
    expect(t.kind).toBe("shell");
    expect(t.result).toBe("\x1b[32mok\x1b[0m"); // raw — OutputBlock renders the color
  });

  it("read/grep rows carry input through for the structured renderers", () => {
    const read = { ...tool("Read", "done", { file_path: "/a/b.ts", offset: 40 }), result: "x" };
    expect(toolOf(msg([read]))!.input).toEqual({ file_path: "/a/b.ts", offset: 40 });
    const grep = { ...tool("Grep", "done", { pattern: "foo" }), result: "a.ts:1:foo" };
    expect(toolOf(msg([grep]))!.input).toEqual({ pattern: "foo" });
  });
});

describe("shellLabel — strips the CLI's snapshot-wrapper prefix from HUD shell rows", () => {
  it("strips pathed bash.exe with -c -l and env assignment (real CLI wrapper shape)", () => {
    expect(shellLabel("C:\\Program Files\\Git\\bin\\bash.exe -c -l SNAPSHOT_FILE=/tmp/snap.sh cargo test --workspace"))
      .toBe("cargo test --workspace");
  });
  it("strips bare powershell with flag-style args", () => {
    expect(shellLabel("powershell -NoProfile -Command npm run build")).toBe("npm run build");
  });
  it("strips cmd.exe /c", () => {
    expect(shellLabel('"C:\\Windows\\System32\\cmd.exe" /c dir /b')).toBe("dir /b");
  });
  it("leaves an unwrapped command untouched", () => {
    expect(shellLabel("cargo build --release")).toBe("cargo build --release");
  });
});

describe("trimCmd — shell caption keeps the verb and the tail", () => {
  it("short commands pass through untouched", () => {
    expect(trimCmd("npm run check")).toBe("npm run check");
  });
  it("drops a leading cd-prefix (quoted path, && chain)", () => {
    expect(trimCmd('cd "c:/AI Workflow/projects/rift-tauri" && npm run check')).toBe("npm run check");
  });
  it("drops a cd-prefix with a semicolon chain and unquoted path", () => {
    expect(trimCmd("cd /tmp; ls -la")).toBe("ls -la");
  });
  it("middle-ellipsizes an over-budget command so both ends survive", () => {
    const long = "npx vitest run " + "src/very/long/path/".repeat(6) + "file.test.ts 2>&1 | tail -8";
    const out = trimCmd(long, 70);
    expect(out.length).toBe(70);
    expect(out.startsWith("npx vitest run ")).toBe(true);
    expect(out.endsWith("| tail -8")).toBe(true);
    expect(out).toContain("…");
  });
  it("does not strip a bare cd command (nothing chained after it)", () => {
    expect(trimCmd('cd "c:/somewhere"')).toBe('cd "c:/somewhere"');
  });
});

describe("coalescePolls — identical consecutive shell runs collapse to one card", () => {
  const shell = (command: string, result: string) =>
    ({ ...tool("Bash", "done", { command }), result });
  const workSegs = (blocks: unknown[]) => {
    const groups = groupBlocks(messageToTurn(msg(blocks)).blocks);
    const work = groups.find((g) => g.type === "work");
    return work && work.type === "work" ? work.segs : [];
  };

  it("3 identical runs → one rich seg with poll=3 carrying the LATEST output", () => {
    const segs = workSegs([
      shell("curl -s localhost:9222", "down 1"),
      shell("curl -s localhost:9222", "down 2"),
      shell("curl -s localhost:9222", "UP"),
    ]);
    expect(segs.length).toBe(1);
    const s = segs[0] as { seg: string; tool: { result: string }; poll?: number };
    expect(s.seg).toBe("rich");
    expect(s.poll).toBe(3);
    expect(s.tool.result).toBe("UP");
  });

  it("2 identical runs stay separate (a re-run, not a wait loop)", () => {
    const segs = workSegs([
      shell("git status", "dirty"),
      shell("git status", "clean"),
    ]);
    expect(segs.length).toBe(2);
    expect(segs.every((s) => s.seg === "rich" && !("poll" in s && s.poll))).toBe(true);
  });

  it("a different command ends the run — poll card + the new command's own card", () => {
    const segs = workSegs([
      shell("sleep-check", "no"),
      shell("sleep-check", "no"),
      shell("sleep-check", "yes"),
      shell("npm run build", "built"),
    ]);
    expect(segs.length).toBe(2);
    expect((segs[0] as { poll?: number }).poll).toBe(3);
    expect((segs[1] as { poll?: number }).poll).toBeUndefined();
  });
});

// ── #100: per-block adaptation memo ─────────────────────────────────────────
// messageToTurn re-runs on every stream delta; the memo keeps StreamTool object
// identity stable for unchanged blocks so child components skip re-render (the
// main-thread-saturation fix). Store blocks are immutable — a changed block is
// a NEW object — so identity is the correct cache key.
describe("messageToTurn — per-block memoization (#100)", () => {
  it("unchanged block objects yield identical StreamTool identity across re-runs", () => {
    const shell = tool("Bash", "pending", { command: "echo hi" });
    const read = tool("Read", "done", { file_path: "/a/b.ts" });
    const pick = (m: ReturnType<typeof messageToTurn>) =>
      m.blocks.filter((b): b is { type: "tool"; tool: StreamTool } => b.type === "tool").map((b) => b.tool);
    const first = pick(messageToTurn(msg([shell, read])));
    const second = pick(messageToTurn(msg([shell, read, text("done")])));
    expect(second[0]).toBe(first[0]);
    expect(second[1]).toBe(first[1]);
  });

  it("a replaced block object (immutable store update) re-adapts; siblings keep identity", () => {
    const shell = tool("Bash", "pending", { command: "sleep 1" });
    const read = tool("Read", "done", { file_path: "/a/c.ts" });
    const pick = (m: ReturnType<typeof messageToTurn>) =>
      m.blocks.filter((b): b is { type: "tool"; tool: StreamTool } => b.type === "tool").map((b) => b.tool);
    const first = pick(messageToTurn(msg([shell, read])));
    const settled = { ...shell, status: "done", result: "ok" };
    const second = pick(messageToTurn(msg([settled, read])));
    expect(second[0]).not.toBe(first[0]);
    expect(second[0].status).toBe("done");
    expect(second[1]).toBe(first[1]);
  });
});
