// Dev-only fixtures for the /dev/gallery stream showroom: ONE scripted replay
// that streams EVERY renderable block kind through the real StreamTurn
// pipeline, in sequence, with live motion — thinking, forming inputs, results
// landing, plan items ticking, the honest footer. Never imported by app code.

import type { ChatMessage, ToolBlock } from "$lib/state/assistant.svelte";

const E = "\x1b["; // ANSI escape intro

const LONG_OUTPUT = Array.from({ length: 64 }, (_, i) =>
  i === 63 ? `${E}32m✓${E}0m build finished in 12.4s`
  : `[${String(i + 1).padStart(2, "0")}/64] compiling crate rift-tauri v0.134.0 — unit ${i + 1}`,
).join("\n");

export type ReplayStep = { at: number; apply: (m: ChatMessage) => void };

export function replayBase(): ChatMessage {
  return { id: "gal-replay", role: "assistant", blocks: [] };
}

export function buildReplaySteps(): ReplayStep[] {
  const steps: ReplayStep[] = [];
  let t = 0;
  let n = 0;
  const add = (delay: number, apply: (m: ChatMessage) => void) => {
    t += delay;
    steps.push({ at: t, apply });
  };
  const findTool = (m: ChatMessage, id: string): ToolBlock | undefined =>
    m.blocks.find((b): b is ToolBlock => b.type === "tool" && b.id === id);

  // Push a settled tool in one beat (the quick, boring case).
  const done = (delay: number, name: string, input: Record<string, unknown>, result: string | null, over: Partial<ToolBlock> = {}) =>
    add(delay, (m) => m.blocks.push({
      type: "tool", id: `gal-${name}-${++n}`, name, input,
      result, isError: false, status: "done", durationMs: 400, ...over,
    }));

  // Start a tool pending, land its result later (visible spinner in between).
  const start = (delay: number, id: string, name: string, input: Record<string, unknown>, forming = false) =>
    add(delay, (m) => m.blocks.push({
      type: "tool", id, name, input, result: null, isError: false,
      status: "pending", startedAt: Date.now(), ...(forming ? { inputPartial: true } : {}),
    }));
  const finish = (delay: number, id: string, result: string | null, over: Partial<ToolBlock> = {}) =>
    add(delay, (m) => {
      const tb = findTool(m, id);
      if (tb) Object.assign(tb, { result, status: "error" in over && over.isError ? "error" : "done", durationMs: 900, ...over });
    });

  const say = (delay: number, text: string) => add(delay, (m) => m.blocks.push({ type: "text", text }));

  // ── 1 · thinking: active → settled ────────────────────────────────────────
  add(0, (m) => m.blocks.push({ type: "thinking", text: "", hasSignature: true, startedAt: Date.now(), durationMs: null, status: "active" }));
  add(900, (m) => { const b = m.blocks[0]; if (b.type === "thinking") b.text = "Walking every block the transcript can render — "; });
  add(800, (m) => { const b = m.blocks[0]; if (b.type === "thinking") b.text += "quiet rows first, then the rich cards."; });
  add(700, (m) => { const b = m.blocks[0]; if (b.type === "thinking") { b.status = "done"; b.durationMs = 2400; } });

  // ── 2 · prose + markdown code block ───────────────────────────────────────
  say(400, "Starting with prose — fenced code renders with highlight + copy:\n\n```ts\nconst kind = nameToKind(tool.name); // 15 kinds\n```\n\nInline code like `inputPartial` stays monospace.");

  // ── 3 · quiet work rows: read · grep · glob · list_dir · mcp ─────────────
  say(1400, "Now the quiet tools — they batch into one work line:");
  done(600, "Read", { file_path: "src/lib/components/assistant/stream/streamModel.ts", offset: 311, limit: 4 },
    "   311→function nameToKind(name: string): TKind {\n   312→  const n = shortName(name);\n   313→  if (n === \"Read\") return \"read\";\n   314→  if (n === \"Grep\") return \"grep\";");
  done(500, "Grep", { pattern: "inputPartial", path: "src/lib/state" },
    "src/lib/state/assistant/types.ts:81:  inputPartial?: boolean;\nsrc/lib/state/assistant/streaming.ts:449:  inputPartial: true,");
  done(500, "Glob", { pattern: "**/*.test.ts", path: "src/lib" },
    "src/lib/state/workspace.test.ts\nsrc/lib/state/assistant.playback.test.ts\nsrc/lib/utils/autocorrect.test.ts");
  done(500, "mcp__rift__list_dir", { path: "src/lib/components/assistant/stream" },
    "src/lib/components/assistant/stream\nStreamTurn.svelte\nStreamShell.svelte\nWorkLine.svelte");
  done(500, "ScheduleWakeup", { reason: "watching CI run 29874806692" }, "scheduled: 480s");

  // ── 4 · shell flavors: bash (ANSI) → pwsh → cmd ───────────────────────────
  say(900, "Shell flavors — bash, pwsh, cmd, each with its identity badge:");
  const sh1 = "gal-sh-bash";
  start(600, sh1, "Bash", { command: "npm run check:tokens && echo done" });
  finish(1400, sh1, `${E}32m✓${E}0m tokens ok — 0 violations\n${E}1mdone${E}0m`);
  const sh2 = "gal-sh-pwsh";
  start(500, sh2, "PowerShell", { command: "Get-Process rift-tauri | Select-Object Id, Path" });
  finish(1200, sh2, "  Id Path\n  -- ----\n41520 C:\\...\\src-tauri\\target\\debug\\rift-tauri.exe");
  done(700, "Bash", { command: "cmd /c ver" }, "Microsoft Windows [Version 10.0.26200]");

  // ── 5 · forming ("running…") → command lands → red error card ────────────
  say(900, "A forming block — the input is still streaming, so it reads running…");
  const shF = "gal-sh-forming";
  start(700, shF, "Bash", {}, true);
  add(2200, (m) => { const tb = findTool(m, shF); if (tb) { tb.input = { command: "cargo build --release" }; tb.inputPartial = undefined; } });
  finish(1500, shF, `${E}31merror[E0308]${E}0m: mismatched types\n  --> src/assistant/turn.rs:812:9\n   = note: expected \`String\`, found \`&str\``, { isError: true, status: "error" });

  // ── 6 · long output: reveal tiers + head/tail fold ────────────────────────
  say(900, "Long output folds — head + tail with a hidden-lines divider:");
  const shL = "gal-sh-long";
  start(600, shL, "Bash", { command: "cargo build 2>&1" });
  finish(1800, shL, LONG_OUTPUT, { durationMs: 12400 });

  // ── 7 · poll collapse: 3 identical commands → one card + count ───────────
  say(900, "A wait loop — three identical runs collapse into one card:");
  done(600, "Bash", { command: "gh run watch 29874806692" }, "* release  Queued", { durationMs: 5000 });
  done(900, "Bash", { command: "gh run watch 29874806692" }, "* release  In progress — svelte-check ✓", { durationMs: 5000 });
  done(900, "Bash", { command: "gh run watch 29874806692" }, `${E}32m✓${E}0m release  Completed — all checks passed`, { durationMs: 5000 });

  // ── 8 · test + lint pills ─────────────────────────────────────────────────
  say(900, "Test and lint runs get parsed pass/fail pills:");
  const shT = "gal-sh-test";
  start(600, shT, "Bash", { command: "npx vitest run" });
  finish(2000, shT, " Test Files  28 passed (28)\n Tests  651 passed (651)", { durationMs: 9300 });
  done(900, "Bash", { command: "npm run check" },
    "svelte-check found 3 errors and 1 warning in 2 files\n\nsrc/lib/components/assistant/Composer.svelte:41:7\nError: 'typed' is declared but its value is never read",
    { isError: true, status: "error", durationMs: 14100 });

  // ── 9 · file changes: Edit + Write + MultiEdit batch ─────────────────────
  say(900, "File changes batch with live +/− counts and inline diffs:");
  done(700, "Edit", {
    file_path: "src/lib/state/assistant/streaming.ts",
    old_string: "b.id === block.id && b.inputPartial",
    new_string: "b.id === block.id && b.status === \"pending\"",
  }, "ok", { durationMs: 120 });
  done(800, "Write", {
    file_path: "src/lib/utils/autocorrect.ts",
    content: "export const TYPO_MAP = {\n  teh: \"the\",\n  dont: \"don't\",\n  im: \"I'm\",\n};\n",
  }, "ok", { durationMs: 90 });
  done(800, "MultiEdit", {
    file_path: "src/lib/state/workspace.test.ts",
    edits: [
      { old_string: "\"chat\", \"ai-health\"]", new_string: "\"chat\", \"ai-health\", \"diagnostics\"]" },
      { old_string: "toBe(5)", new_string: "toBe(6)" },
    ],
  }, "ok", { durationMs: 150 });

  // ── 10 · web search + fetch ───────────────────────────────────────────────
  say(900, "Web search and fetch cards:");
  const web = "gal-web";
  start(600, web, "WebSearch", { query: "tauri 2 window drag region capability" });
  finish(1400, web, "10 results", { durationMs: 3200 });
  done(800, "WebFetch", { url: "https://v2.tauri.app/security/capabilities/" }, "Capabilities gate window permissions…", { durationMs: 2800 });

  // ── 11 · agent card ───────────────────────────────────────────────────────
  say(900, "A delegated sub-agent:");
  const ag = "gal-agent";
  start(600, ag, "Agent", { subagent_type: "recon", description: "map stream component usage" });
  finish(2200, ag, "StreamTurn routes 15 kinds: plan→StreamPlan, web→StreamWeb, test/lint→StreamResult…", { durationMs: 41000 });

  // ── 12 · plan card with items ticking ────────────────────────────────────
  say(900, "The plan checklist — watch items tick over:");
  const plan = "gal-plan";
  add(600, (m) => m.blocks.push({
    type: "tool", id: plan, name: "TodoWrite",
    input: { todos: [
      { content: "Walk every block kind", status: "in_progress" },
      { content: "Show the footer variants", status: "pending" },
      { content: "Hand back to Blazzer", status: "pending" },
    ] },
    result: "ok", isError: false, status: "done", durationMs: 50,
  }));
  add(1600, (m) => { const tb = findTool(m, plan); if (tb) tb.input = { todos: [
    { content: "Walk every block kind", status: "completed" },
    { content: "Show the footer variants", status: "in_progress" },
    { content: "Hand back to Blazzer", status: "pending" },
  ] }; });
  add(1600, (m) => { const tb = findTool(m, plan); if (tb) tb.input = { todos: [
    { content: "Walk every block kind", status: "completed" },
    { content: "Show the footer variants", status: "completed" },
    { content: "Hand back to Blazzer", status: "in_progress" },
  ] }; });

  // ── 13 · ask user: pending → answered chips ──────────────────────────────
  say(900, "An ask — options render as chips, then the answered state:");
  const ask = "gal-ask";
  start(600, ask, "ask_user", {
    questions: [{
      question: "Which block should we redesign first?",
      header: "Redesign",
      options: [
        { label: "Terminal cards", description: "Shell/test/lint" },
        { label: "Work rows", description: "The quiet batches" },
      ],
      multiSelect: false,
    }],
  });
  finish(2400, ask, "Q: Which block should we redesign first?\nA: Terminal cards", { durationMs: 2400 });

  // ── 14 · mid-turn steer ───────────────────────────────────────────────────
  add(900, (m) => m.blocks.push({ type: "steer", id: "gal-steer", text: "also show me the plan proposal card", at: Date.now(), images: 0, files: 0 }));

  // ── 15 · plan proposal (ExitPlanMode) → footer reads "Planned" ───────────
  done(900, "ExitPlanMode", {
    plan: "## Block redesign — proposal\n\n1. **Terminal cards** — calmer chrome, clearer flavor badge\n2. **Work rows** — tighter grouping\n3. **Result pills** — one visual language for pass/fail\n\nGallery replay is the reference for every state.",
  }, "ok", { durationMs: 200 });

  // ── 16 · closing prose + honest meta ─────────────────────────────────────
  say(1200, "That's every block kind the transcript renders — flip the density settings above and replay to compare.");
  add(800, (m) => { m.turnDurationMs = 47000; m.costUsd = 0.42; m.outputTokens = 12800; });

  return steps;
}
