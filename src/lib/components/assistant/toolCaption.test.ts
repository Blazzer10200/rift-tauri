import { describe, expect, it } from "vitest";
import { agentNowLine, basename, captionForGroup, captionForTool } from "./toolCaption";

// basename is a deliberate sibling of tabsbar/helpers.ts::leafName — these cases
// mirror leafName's vitest so the two cannot silently drift.
describe("basename", () => {
  it("returns the last path segment for posix and windows paths", () => {
    expect(basename("/home/user/project")).toBe("project");
    expect(basename("C:\\AI Workflow\\projects\\rift-tauri")).toBe("rift-tauri");
  });
  it("ignores a single trailing slash and survives a bare name", () => {
    expect(basename("/home/user/project/")).toBe("project");
    expect(basename("rift")).toBe("rift");
  });
});

describe("captionForTool", () => {
  it("names the file for read/write/edit families, falling back when absent", () => {
    expect(captionForTool("Read", { file_path: "src/lib/helpers.ts" })).toBe("Reading helpers.ts");
    expect(captionForTool("Read")).toBe("Reading a file");
    expect(captionForTool("Write", { file_path: "C:\\proj\\new.rs" })).toBe("Creating new.rs");
    expect(captionForTool("MultiEdit", { file_path: "a/b/mod.rs" })).toBe("Editing mod.rs");
  });

  it("strips the mcp__rift__ prefix before matching", () => {
    expect(captionForTool("mcp__rift__read_file", { path: "x/y/z.txt" })).toBe("Reading z.txt");
    expect(captionForTool("mcp__rift__grep", { pattern: "foo" })).toBe("Searching for foo");
  });

  it("prefers the Bash description, else backticks the first command token", () => {
    expect(captionForTool("Bash", { description: "Install deps", command: "npm i" })).toBe("Install deps");
    expect(captionForTool("Bash", { command: "git status --short" })).toBe("Running `git`");
    expect(captionForTool("Bash")).toBe("Running a command");
    const long = "a".repeat(30);
    expect(captionForTool("Bash", { command: `${long} arg` })).toBe(`Running \`${"a".repeat(23)}…\``);
  });

  it("clips long descriptions at 60 chars", () => {
    const desc = "d".repeat(80);
    expect(captionForTool("Bash", { description: desc })).toBe("d".repeat(59) + "…");
  });

  it("describes searches with pattern and scope", () => {
    expect(captionForTool("Grep", { pattern: "TODO", path: "src/lib" })).toBe("Searching for TODO in lib");
    expect(captionForTool("Grep", { pattern: "TODO" })).toBe("Searching for TODO");
    expect(captionForTool("Grep")).toBe("Searching files");
    expect(captionForTool("Glob", { pattern: "**/*.svelte" })).toBe("Finding **/*.svelte");
  });

  it("extracts the host for WebFetch and survives invalid URLs", () => {
    expect(captionForTool("WebFetch", { url: "https://docs.rs/tokio/latest" })).toBe("Fetching docs.rs");
    expect(captionForTool("WebFetch", { url: "not a url" })).toBe("Fetching not a url");
  });

  it("covers agent, skill, and task captions", () => {
    expect(captionForTool("Agent", { description: "Map sections" })).toBe("Delegating: Map sections");
    expect(captionForTool("Agent", { subagent_type: "recon" })).toBe("Delegating to recon");
    expect(captionForTool("Skill", { skill: "check" })).toBe("Using the check skill");
    expect(captionForTool("TaskCreate", { subject: "Ship it" })).toBe("Planning · Ship it");
    expect(captionForTool("TodoWrite")).toBe("Updating the task list");
  });

  it("falls back to 'Running <name>' for unknown tools", () => {
    expect(captionForTool("SomeNewTool")).toBe("Running SomeNewTool");
  });
});

describe("captionForGroup", () => {
  const tool = (name: string) => ({ type: "tool", name });

  it("uses a single verb for homogeneous groups", () => {
    expect(captionForGroup([tool("Read"), tool("Read"), tool("Read")])).toBe("Reading 3 files");
    expect(captionForGroup([tool("Bash"), tool("Bash")])).toBe("Running 2 commands");
    expect(captionForGroup([tool("mcp__rift__grep"), tool("grep")])).toBe("Running 2 searches");
  });

  it("counts an all-distinct mix as actions", () => {
    expect(captionForGroup([tool("Read"), tool("Grep"), tool("Bash"), tool("Glob")])).toBe("Running 4 actions");
  });

  it("leads with the dominant kind when one is ≥2", () => {
    expect(captionForGroup([tool("Read"), tool("Read"), tool("Read"), tool("Grep")])).toBe("Reading 3 files +1 more");
    expect(captionForGroup([tool("Bash"), tool("Bash"), tool("Read"), tool("Glob")])).toBe("Running 2 commands +2 more");
  });

  it("falls back for unknown homogeneous names and empty input", () => {
    expect(captionForGroup([tool("LS"), tool("LS")])).toBe("Running 2 LS calls");
    expect(captionForGroup([])).toBe("Running tools");
    expect(captionForGroup([{ type: "text" }])).toBe("Running tools");
  });
});

// Shared by StreamAgent (card) + AgentHud (periscope) — the tail-first scan.
describe("agentNowLine", () => {
  it("prefers the pending tool nearest the tail", () => {
    expect(agentNowLine([
      { type: "tool", name: "Read", input: { file_path: "/a/x.ts" }, status: "done" },
      { type: "tool", name: "Grep", input: { pattern: "auth" }, status: "pending" },
    ])).toEqual({ label: "Searching for auth", thinking: false });
  });
  it("reports active thinking when no tool is pending", () => {
    expect(agentNowLine([
      { type: "tool", name: "Read", input: {}, status: "done" },
      { type: "thinking", status: "active" },
    ])).toEqual({ label: "Thinking", thinking: true });
  });
  it("falls back to the most recent settled step, then to spinning up", () => {
    expect(agentNowLine([
      { type: "tool", name: "Bash", input: { command: "cargo test" }, status: "done" },
    ])).toEqual({ label: "Running `cargo`", thinking: false });
    expect(agentNowLine([])).toEqual({ label: "Spinning up", thinking: true });
  });
});
