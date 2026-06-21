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
});
