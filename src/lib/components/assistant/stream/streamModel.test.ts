import { describe, it, expect } from "vitest";
import { messageToTurn } from "./streamModel";
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
});
