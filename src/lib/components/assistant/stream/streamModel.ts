// Phase 4 — Codex-flavored STREAM mode adapter. Maps the live `ChatMessage`
// (Block[] from streaming.ts) onto the prototype `StreamTurn` render model
// (app/stream.jsx). Boxless, text-first turn: a "Working for Ns" header,
// collapsed reasoning, grouped tool lines, file-write batches, live footer.
//
// The live ToolBlock carries only { name, input, result, status, durationMs }
// — no kind/cap/diff/plan/sources — so this derives kind+caption from the tool
// name+input (mirroring ToolChip.svelte) and renders rich blocks (plan/web/
// agent) from whatever input is available, degrading to the lean form when the
// backend doesn't emit the extra metadata (sources, diff counts, pass/fail).

import type { Block, ChatMessage, ToolBlock } from "$lib/state/assistant.svelte";
import { leafName as basename } from "$lib/utils/path";
import { diffArrays } from "diff";

export type TKind =
  | "read" | "grep" | "edit" | "create" | "shell"
  | "agent" | "web" | "fetch" | "test" | "lint" | "mcp" | "plan" | "ask" | "exitplan";

export type PlanItem = { text: string; status: "done" | "active" | "todo" };

/** Map the store's aggregated task list (`tab.tasks`, maintained across
 *  TaskCreate/TaskUpdate/TodoWrite/checklist-pin) into the plan-card shape. The
 *  newer CLI emits one TaskCreate per item (no `todos[]` array), so a single
 *  tool block can't carry the whole plan — the aggregate in the store can.
 *  StreamTurn falls back to this when a plan block's own items are empty. */
export function tasksToPlanItems(
  tasks: { content: string; status: "pending" | "in_progress" | "completed" }[],
): PlanItem[] {
  return tasks
    .filter((t) => typeof t.content === "string" && t.content.length > 0)
    .map((t) => ({
      text: t.content,
      status: t.status === "completed" ? "done" : t.status === "in_progress" ? "active" : "todo",
    }));
}

export type StreamTool = {
  id: string;
  kind: TKind;
  cap: string;
  name: string;
  status: "pending" | "done" | "error";
  durSecs: number;
  add: number | null;
  del: number | null;
  path?: string | null; // full file_path (read/edit/create) — for path surfacing
  dir?: string | null;  // workspace-relative dir prefix of `path`
  input?: Record<string, unknown> | null; // raw tool input — feeds inline EditDiff
  items?: PlanItem[]; // plan
  query?: string; sources?: string[]; count?: number | null; // web/fetch
  fail?: number | null; pass?: number | null; // test/lint
  task?: string; result?: string | null; // agent · shell · read/grep/mcp detail
  flavor?: ShellFlavor; // shell only — which shell ran it (badge identity)
  /** Input still streaming in (live-forming block) — captions are provisional,
   *  diff counts/expand deferred until the complete input lands. */
  forming?: boolean;
};

/** Which shell a command ran under — drives the StreamShell identity badge.
 *  `PowerShell` is the CLI's dedicated tool; a Bash command that shells out to
 *  cmd.exe reads as cmd. Everything else on the Bash tool is bash. */
export type ShellFlavor = "bash" | "pwsh" | "cmd";
export function shellFlavor(name: string, command: string | null): ShellFlavor {
  if (name === "PowerShell") return "pwsh";
  if (command && /^\s*cmd(\.exe)?\s+\/c/i.test(command)) return "cmd";
  return "bash";
}

type StreamBlock =
  | { type: "say"; text: string }
  | { type: "tool"; tool: StreamTool };

// What the turn actually did, so the footer can be honest:
//  - "applied": at least one edit/create tool succeeded → green "Applied"
//  - "ran":     ran tools but changed no files (read/grep/shell/web/…) → muted "Done"
//  - "failed":  edits/creates were attempted but every one errored → "Changes failed"
//  - "planned": the turn ended in an ExitPlanMode proposal — the CLI writes the
//               plan artifact to ~/.claude/plans/, which is a real Write tool
//               call, so without this the footer claimed "Applied 1 file" on a
//               turn that changed nothing in the user's repo.
//  - "text":    no tools at all (a plain answer) → no status badge
export type TurnOutcome = "applied" | "ran" | "failed" | "planned" | "text";

export type TurnModel = {
  blocks: StreamBlock[];
  thinking: { active: boolean; durSecs: number; text: string } | null;
  outcome: TurnOutcome;
  files: number; // distinct files edited/created (only meaningful for "applied")
  meta: { time: string; cost: string | null } | null; // footer time·cost line
  totalSecs: number;
};

export type WorkSeg =
  | { seg: "rich"; tool: StreamTool }
  | { seg: "edit"; tools: StreamTool[] }
  | { seg: "other"; tools: StreamTool[] };

export type Group =
  | { type: "work"; segs: WorkSeg[] }
  | { type: "say"; text: string };

// ── helpers (mirror ToolChip.svelte) ────────────────────────────────────────
// `basename` = canonical `leafName` (imported above) — was a local inline copy.
const shortName = (n: string) => n.replace(/^mcp__rift__/, "");
const trim = (s: string, n = 60) => (s.length > n ? s.slice(0, n - 1) + "…" : s);
const hostOf = (u: string) => { try { return new URL(u).host; } catch { return u; } };

// Strip ANSI escape sequences (SGR colors, cursor moves) from tool output —
// PowerShell/cargo/npm emit them, and raw `\x1b[31;1m` renders as literal
// garbage in the transcript. Display + copy both want clean text.
// eslint-disable-next-line no-control-regex
const ANSI_RE = /\x1b\[[0-9;?]*[ -/]*[@-~]/g;
function stripAnsi(s: string): string {
  return s.replace(ANSI_RE, "");
}

export function fmtDur(t: number): string {
  const raw = Math.max(0, t);
  t = Math.round(raw);
  if (t === 0) return raw > 0 ? "<1s" : "0s"; // don't round real sub-second work down to "0s"
  if (t < 60) return t + "s";
  const m = Math.floor(t / 60), s = t % 60;
  return m + "m" + (s ? " " + s + "s" : "");
}

function nameToKind(name: string): TKind {
  const n = shortName(name);
  if (n === "Read" || n === "read_file" || n === "list_dir" || n === "NotebookRead") return "read";
  if (n === "Grep" || n === "grep" || n === "Glob") return "grep";
  if (n === "Edit" || n === "MultiEdit" || n === "NotebookEdit") return "edit";
  if (n === "Write") return "create";
  if (n === "Bash" || n === "remote_bash" || n === "BashOutput" || n === "KillBash" || n === "KillShell") return "shell";
  if (n === "PowerShell") return "shell"; // CC's dedicated PowerShell tool — same in/out shape as Bash
  // TaskOutput/TaskStop are the newer CLI's background-TASK ops (tail/stop a bg
  // shell or agent) — shell-shaped, NOT part of the TaskCreate/Update todo set.
  // Mapping them to "plan" rendered a bg-build tail as a checklist card.
  if (n === "TaskOutput" || n === "TaskStop") return "shell";
  if (n === "Agent" || n === "Task") return "agent";
  if (n === "WebSearch") return "web";
  if (n === "WebFetch") return "fetch";
  if (
    n === "TodoWrite" || n === "TaskCreate" || n === "TaskUpdate" ||
    n === "TaskList" || n === "TaskGet"
  ) return "plan";
  if (n === "ask_user") return "ask";
  // The plan-mode exit carries the FULL proposed plan markdown in input.plan —
  // the deliverable of plan mode. Dedicated kind so it renders readable, not
  // as a 48-char generic peek.
  if (n === "ExitPlanMode") return "exitplan";
  return "mcp";
}

function caption(tb: ToolBlock): string {
  const n = shortName(tb.name);
  const inp = tb.input ?? {};
  const fp = typeof inp.file_path === "string" ? basename(inp.file_path)
    : typeof inp.path === "string" ? basename(inp.path)
    : typeof inp.notebook_path === "string" ? basename(inp.notebook_path) : null;
  if (n === "Read" || n === "read_file") return fp ?? "file";
  if (n === "Write") return fp ?? "file";
  if (n === "Edit") return fp ?? "file";
  if (n === "MultiEdit") {
    const c = Array.isArray(inp.edits) ? inp.edits.length : 0;
    return fp ? `${fp} · ${c} edits` : `${c} edits`;
  }
  if (n === "NotebookEdit") return fp ?? "notebook";
  if (n === "Bash" || n === "remote_bash" || n === "PowerShell") return typeof inp.command === "string" ? trim(inp.command, 70) : "shell";
  if (n === "Glob") {
    const pat = typeof inp.pattern === "string" ? inp.pattern : "?";
    const scope = typeof inp.path === "string" ? ` in ${inp.path}` : "";
    return `${pat}${scope}`;
  }
  if (n === "Grep" || n === "grep") {
    const pat = typeof inp.pattern === "string" ? `"${inp.pattern}"` : "?";
    const scope = typeof inp.path === "string" ? ` in ${inp.path}` : "";
    return `${pat}${scope}`;
  }
  if (n === "list_dir") return typeof inp.path === "string" ? inp.path : "directory";
  if (n === "WebFetch") return typeof inp.url === "string" ? hostOf(inp.url) : "url";
  if (n === "WebSearch") return typeof inp.query === "string" ? trim(inp.query, 50) : "search";
  if (n === "Agent" || n === "Task") {
    const sa = typeof inp.subagent_type === "string" ? inp.subagent_type : "task";
    const desc = typeof inp.description === "string" ? ` · ${trim(inp.description, 40)}` : "";
    return `${sa}${desc}`;
  }
  if (n === "TodoWrite") {
    const c = Array.isArray(inp.todos) ? inp.todos.length : 0;
    return `${c} task${c === 1 ? "" : "s"}`;
  }
  if (n === "BashOutput" || n === "TaskOutput") {
    const id = typeof inp.bash_id === "string" ? inp.bash_id : typeof inp.task_id === "string" ? inp.task_id : null;
    return id ? `tail ${id}` : "tail background task";
  }
  if (n === "KillBash" || n === "KillShell" || n === "TaskStop") {
    const id = typeof inp.shell_id === "string" ? inp.shell_id : typeof inp.task_id === "string" ? inp.task_id : null;
    return id ? `stop ${id}` : "stop background task";
  }
  if (n === "AskUserQuestion") {
    const qs = Array.isArray(inp.questions) ? inp.questions.length : 0;
    return `${qs} question${qs === 1 ? "" : "s"}`;
  }
  if (n === "Skill") {
    const s = typeof inp.skill === "string" ? inp.skill : "skill";
    const args = typeof inp.args === "string" && inp.args ? ` ${trim(inp.args, 30)}` : "";
    return `/${s}${args}`;
  }
  if (n === "SlashCommand") return typeof inp.command === "string" ? trim(inp.command, 60) : "slash command";
  if (n === "Workflow") return typeof inp.name === "string" ? inp.name : "workflow";
  if (n === "ExitPlanMode") return "Proposed a plan";
  if (n === "EnterPlanMode") return "entered plan mode";
  if (n === "LSP") {
    const op = typeof inp.operation === "string" ? inp.operation : "lsp";
    const f = typeof inp.filePath === "string" ? ` · ${basename(inp.filePath)}` : "";
    return `${op}${f}`;
  }
  // Unknown/MCP tool: name + a peek at its most meaningful input, so "Called
  // ScheduleWakeup" becomes "ScheduleWakeup · reason: watching CI run". The
  // input IS the detail — hiding it made every MCP call an opaque stub.
  const peek = firstInputPeek(inp);
  return peek ? `${n} · ${peek}` : n;
}

// The most caption-worthy string field of a tool's input, preferred keys first
// (command/query/url/…), else the first short string value found. Trimmed hard
// so the caption stays one line; the full input renders on expand.
const PEEK_KEYS = ["command", "query", "url", "path", "file_path", "pattern", "prompt", "description", "reason", "message", "text", "title"];
function firstInputPeek(inp: Record<string, unknown>): string | null {
  for (const k of PEEK_KEYS) {
    const v = inp[k];
    if (typeof v === "string" && v.trim()) return trim(v.trim().replace(/\s+/g, " "), 48);
  }
  for (const v of Object.values(inp)) {
    if (typeof v === "string" && v.trim()) return trim(v.trim().replace(/\s+/g, " "), 48);
  }
  return null;
}

// Full file path off a tool's input (read/edit/create/notebook), normalized to
// forward slashes. Drives the path-surfacing crumb + the EditDiff header.
function pathOf(inp: Record<string, unknown>): string | null {
  const p =
    typeof inp.file_path === "string" ? inp.file_path
    : typeof inp.path === "string" ? inp.path
    : typeof inp.notebook_path === "string" ? inp.notebook_path
    : null;
  return p ? p.replace(/\\/g, "/").replace(/\/$/, "") : null;
}

// #91: in plan mode the CLI Writes the proposal to ~/.claude/plans/<slug>.md —
// a real Write whose expanded diff duplicates the StreamExitPlan card rendered
// right below it. The row stays (honest state); its diff just folds by default.
export function isPlanArtifact(path: string | null | undefined): boolean {
  return !!path && path.includes("/.claude/plans/");
}

// Dir prefix of a full path, collapsed to its last two segments so the filename
// stays readable (full path lives in the row's title/tooltip).
function dirOf(path: string | null): string | null {
  if (!path) return null;
  const idx = path.lastIndexOf("/");
  if (idx < 0) return null;
  const segs = path.slice(0, idx).split("/").filter(Boolean);
  if (segs.length === 0) return null;
  return (segs.length <= 2 ? segs.join("/") : "…/" + segs.slice(-2).join("/")) + "/";
}

// Memo keyed by tool id: an Edit/Write input is fixed at tool_use_start, but
// messageToTurn re-runs on every stream frame, so without this the O(n·m) line
// diff below recomputes for every settled edit on every token. Bounded so a long
// session can't grow it unboundedly.
const diffCountCache = new Map<string, { add: number; del: number } | null>();
function diffCountsCached(id: string, inp: Record<string, unknown>) {
  const hit = diffCountCache.get(id);
  if (hit !== undefined) return hit;
  const dc = diffCounts(inp);
  if (diffCountCache.size > 500) diffCountCache.clear();
  diffCountCache.set(id, dc);
  return dc;
}

// Cheap +adds / −dels from an Edit/Write/MultiEdit input, so the stream batch
// can show real line deltas (and roll them on the odometer) without rendering
// the whole diff. MultiEdit sums each sub-edit.
function diffCounts(inp: Record<string, unknown>): { add: number; del: number } | null {
  const pairs: Array<{ o: string; n: string }> = [];
  if (typeof inp.content === "string" && typeof inp.new_string !== "string") {
    pairs.push({ o: "", n: inp.content }); // Write (new file) → all adds
  } else if (typeof inp.old_string === "string" && typeof inp.new_string === "string") {
    pairs.push({ o: inp.old_string, n: inp.new_string });
  } else if (Array.isArray(inp.edits)) {
    for (const e of inp.edits as Array<Record<string, unknown>>) {
      if (typeof e?.old_string === "string" && typeof e?.new_string === "string") {
        pairs.push({ o: e.old_string, n: e.new_string });
      }
    }
  }
  if (pairs.length === 0) return null;
  let add = 0, del = 0;
  for (const { o, n } of pairs) {
    if (o.length + n.length > 200_000) continue; // skip huge blobs
    // Empty old → new-file/all-additions: split("") yields [""] which diff would
    // report as a phantom 1-line deletion. Count it as pure adds.
    if (o === "") { add += n === "" ? 0 : n.split("\n").length; continue; }
    for (const c of diffArrays(o.split("\n"), n.split("\n"))) {
      if (c.added) add += c.value.length;
      else if (c.removed) del += c.value.length;
    }
  }
  return { add, del };
}

function planItems(tb: ToolBlock): PlanItem[] {
  const raw = tb.input?.todos;
  if (!Array.isArray(raw)) return [];
  return raw
    .map((t: Record<string, unknown>) => {
      const text = typeof t.content === "string" ? t.content : "";
      const s = t.status;
      const status: PlanItem["status"] = s === "completed" ? "done" : s === "in_progress" ? "active" : "todo";
      return { text, status };
    })
    .filter((t) => t.text.length > 0);
}

function adaptTool(tb: ToolBlock): StreamTool {
  const kind = nameToKind(tb.name);
  const durSecs = typeof tb.durationMs === "number" ? tb.durationMs / 1000 : 0;
  const inp = tb.input ?? {};
  const path = pathOf(inp);
  const t: StreamTool = {
    id: tb.id, kind, name: shortName(tb.name), cap: caption(tb),
    status: tb.status, durSecs, add: null, del: null,
    path, dir: dirOf(path),
  };
  if (tb.inputPartial) t.forming = true;
  if (kind === "edit" || kind === "create") {
    // While the input is still streaming (live-forming), old/new strings are
    // truncated — a diff over them is wrong AND diffCountsCached memoizes by id,
    // so a partial count would stick. Defer both until the input is complete.
    if (!tb.inputPartial) {
      t.input = inp;
      const dc = diffCountsCached(tb.id, inp);
      if (dc) { t.add = dc.add; t.del = dc.del; }
    }
  }
  if (kind === "exitplan") t.input = inp; // input.plan = the proposed plan markdown
  if (kind === "plan") t.items = planItems(tb);
  if (kind === "web" || kind === "fetch") {
    t.query = t.cap; t.sources = []; t.count = null;
  }
  if (kind === "agent") {
    t.task = t.cap;
    t.result = tb.result && !tb.isError ? trim(tb.result.trim().split("\n")[0] ?? "", 90) : null;
  }
  if (kind === "ask") {
    // Carry the raw questions input + full tool_result through so the
    // interactive ask card can render options + the answered transcript.
    t.input = inp;
    t.result = tb.result ?? null;
  }
  if (kind === "shell") {
    // Carry the full stdout/stderr through so the live stream can show the
    // command's in-and-out (gated by the `commandOutput` pref at render time).
    // ANSI-stripped: PowerShell/cargo color codes render as literal `[31;1m`
    // garbage otherwise (observed on failed pwsh output).
    t.result = tb.result != null ? stripAnsi(tb.result) : null;
    t.flavor = shellFlavor(t.name, typeof inp.command === "string" ? inp.command : null);
  }
  if (kind === "read" || kind === "grep" || kind === "mcp") {
    // Forward the result for detail surfacing: read/grep rows show honest
    // line/match counts, MCP rows can expand to the actual response. The data
    // was always on the block — dropping it here made "show me what happened"
    // impossible downstream.
    t.result = tb.result != null ? stripAnsi(tb.result) : null;
    if (kind === "mcp") t.input = inp;
  }
  return t;
}

/** Honest one-glance result meta for a WorkLine row: what the tool came back
 *  with, not just that it ran. Read → line count; Grep/Glob/list_dir → match/
 *  entry count. Null when there's no result to summarize. */
export function resultMeta(t: StreamTool): string | null {
  if (typeof t.result !== "string" || t.result.trim().length === 0) return null;
  const trimmed = t.result.trim();
  const n = t.result.replace(/\s+$/, "").split("\n").filter((l) => l.trim().length > 0).length;
  if (t.kind === "read") {
    if (t.name === "list_dir") {
      // Backend always emits a header line (the resolved dir path) before the
      // per-entry lines — subtract it so an empty dir reads "0 entries", not
      // "1 line" off a header nobody asked to see counted.
      const entries = Math.max(0, n - 1);
      return entries === 1 ? "1 entry" : `${entries} entries`;
    }
    return n === 1 ? "1 line" : `${n} lines`;
  }
  if (t.kind === "grep") {
    if (/^No (files |matches )?found/i.test(trimmed) || /^\(no matches/i.test(trimmed)) return "no matches";
    const [one, many] = t.name === "Glob" ? ["file", "files"] : ["match", "matches"];
    return `${n} ${n === 1 ? one : many}`;
  }
  return null;
}

// Trailing-lines preview of a shell result for "peek" mode: the last `n`
// non-empty lines, so the user sees the tail (exit message / final output)
// without expanding. Returns the lines + whether anything was elided.
export function outputPeek(result: string | null | undefined, n = 3): { lines: string[]; more: number } {
  if (!result) return { lines: [], more: 0 };
  const all = result.replace(/\s+$/, "").split("\n");
  const nonEmpty = all.filter((l) => l.trim().length > 0);
  // +1 slack: a "+1 more line" indicator is as tall as the line it hides.
  if (nonEmpty.length <= n + 1) return { lines: nonEmpty, more: 0 };
  return { lines: nonEmpty.slice(-n), more: nonEmpty.length - n };
}

// ── Progressive output reveal ────────────────────────────────────────────────
// Long tool output (a 400-line `git log`, a full `cargo build`) shouldn't drop
// the reader into a tiny inner-scroll box — that hides how much there is and
// makes skimming a chore. Instead we cap the visible line count in TIERS and let
// the user step the cap up: collapsed (a glanceable head), then "Show more" to a
// taller expanded cap, and only PAST that does the block become a bounded scroll.
// `revealTier` returns the full line list plus how many the current tier shows,
// so the component can render `lines.slice(0, shown)` + a "+N more" affordance.
//
// The cap steps are line COUNTS, not pixels, so a wall of short lines and a few
// long ones both cap predictably. Kept pure + exported for unit tests.
export const REVEAL_COLLAPSED = 12; // first glance — a head tall enough to be useful
export const REVEAL_EXPANDED = 40;  // after one "Show more" — most output fits here
// Never hide a tail this short behind a click — a "Show 4 more lines" button
// costs more UI than the 4 lines it hides. A tier whose remainder fits in the
// slack just shows everything (and never offers a next tier).
export const REVEAL_SLACK = 8;
export type RevealTier = "collapsed" | "expanded" | "all";

export function splitOutput(
  result: string | null | undefined,
  tier: RevealTier,
): { lines: string[]; shown: number; hidden: number; total: number } {
  if (!result) return { lines: [], shown: 0, hidden: 0, total: 0 };
  const lines = result.replace(/\s+$/, "").split("\n");
  const total = lines.length;
  const cap = tier === "collapsed" ? REVEAL_COLLAPSED : tier === "expanded" ? REVEAL_EXPANDED : total;
  const shown = total <= cap + REVEAL_SLACK ? total : cap;
  return { lines, shown, hidden: Math.max(0, total - shown), total };
}

// The next tier up from the current one, given how many lines the output has.
// collapsed → expanded (if there's more past the collapsed cap + slack) → all.
// Returns null when nothing more can be revealed (already showing everything).
export function nextRevealTier(tier: RevealTier, total: number): RevealTier | null {
  if (tier === "collapsed") return total > REVEAL_COLLAPSED + REVEAL_SLACK ? "expanded" : null;
  if (tier === "expanded") return total > REVEAL_EXPANDED + REVEAL_SLACK ? "all" : null;
  return null;
}

// Map one live assistant ChatMessage → the StreamTurn render model.
export function messageToTurn(m: ChatMessage): TurnModel {
  const blocks: StreamBlock[] = [];
  let thinkText: string[] = [];
  let thinkSecs = 0;
  let thinkActive = false;
  let thinkSeen = false;
  let totalSecs = 0;

  for (const b of m.blocks as Block[]) {
    if (b.type === "text") {
      if (b.text.trim()) blocks.push({ type: "say", text: b.text });
    } else if (b.type === "thinking") {
      thinkSeen = true;
      if (b.text.trim()) thinkText.push(b.text.trim());
      if (typeof b.durationMs === "number") { thinkSecs += b.durationMs / 1000; totalSecs += b.durationMs / 1000; }
      if (b.status === "active") thinkActive = true;
    } else if (b.type === "tool") {
      const tool = adaptTool(b);
      totalSecs += tool.durSecs;
      blocks.push({ type: "tool", tool });
    }
    // boundary / image blocks are not part of a stream turn body
  }

  // Show the thinking indicator whenever the model actually thought — even with
  // NO readable text. Opus 4.7/4.8 default thinking.display to "omitted" (only a
  // signature streams, empty text), so gating on text alone hid the "Thought for
  // Xs" chip entirely → Opus thinking-on looked like it did nothing while still
  // costing thinking tokens. A bare thinking block w/ a real duration still earns
  // the chip. (text.length || active) kept so a 0-duration empty block is dropped
  // as noise; thinkSeen with measurable time surfaces the honest indicator.
  const thinking = (thinkText.length || thinkActive || (thinkSeen && thinkSecs > 0))
    ? { active: thinkActive, durSecs: thinkSecs, text: thinkText.join("\n\n") }
    : null;

  // Classify what the turn did from its tools (not from cost — every turn costs).
  const tools = blocks.filter((b): b is { type: "tool"; tool: StreamTool } => b.type === "tool").map((b) => b.tool);
  const mutators = tools.filter((t) => t.kind === "edit" || t.kind === "create");
  const okMutators = mutators.filter((t) => t.status !== "error");
  const changedFiles = new Set(okMutators.map((t) => t.path ?? t.cap)); // distinct by full path

  let outcome: TurnOutcome;
  if (tools.some((t) => t.kind === "exitplan")) outcome = "planned";
  else if (okMutators.length > 0) outcome = "applied";
  else if (mutators.length > 0) outcome = "failed"; // attempted edits, all errored
  else if (tools.length > 0) outcome = "ran"; // tools, but nothing mutating
  else outcome = "text"; // a plain answer

  // "Worked for Ns" — prefer the CLI's wall-clock for the whole turn (result
  // frame duration_ms; spawn→result, tool waits included). The summed
  // thinking+tool secs under-report badly — a pure-text turn shows "Worked
  // for 0s" over a 30s reply. Sum kept only for pre-field history.
  const wallSecs = typeof m.turnDurationMs === "number" && m.turnDurationMs > 0
    ? m.turnDurationMs / 1000
    : null;
  const shownSecs = wallSecs ?? totalSecs;

  // Footer time·cost line — shown for any turn that did work (not pure text).
  const cost = typeof m.costUsd === "number" && m.costUsd > 0 ? `$${m.costUsd.toFixed(2)}` : null;
  const meta = outcome === "text" ? null : { time: fmtDur(shownSecs), cost };

  return { blocks, thinking, outcome, files: changedFiles.size, meta, totalSecs: shownSecs };
}

// The CLI can split one narration sentence across a tool_use — the model emits
// "I'll do all three in p", then a Read tool, then "arallel." as a *second* text
// block. Rendered naively that's two prose beats with a word sliced in half
// ("in p" / "arallel."). Stitch a say-fragment back onto the preceding say when
// the earlier text stopped mid-sentence (no terminal ./!/?/: and no trailing
// newline) and the fragment reads as a continuation (starts lowercase, or with
// closing punctuation) — the model wrote one sentence; the tool just interrupted
// the stream. Tools keep their order; only the narration is made whole.
function stitchSayFragments(blocks: StreamBlock[]): StreamBlock[] {
  const out: StreamBlock[] = [];
  // Index into `out` of the most recent say block still open to continuation.
  let openSayIdx = -1;
  for (const b of blocks) {
    if (b.type === "say") {
      const frag = b.text;
      const prev = openSayIdx >= 0 ? out[openSayIdx] : null;
      if (prev && prev.type === "say" && isMidSentence(prev.text) && isContinuation(frag)) {
        // Direct concat: the CLI splits mid-token with no lost whitespace, so
        // "in p" + "arallel." rejoins to "in parallel." verbatim. Any real space
        // already lives at the end of prev or the start of frag.
        out[openSayIdx] = { type: "say", text: prev.text + frag };
      } else {
        out.push({ type: "say", text: frag });
        openSayIdx = out.length - 1;
      }
    } else {
      // A tool between two say blocks does NOT close the open say — the sentence
      // may resume after it (that's the exact split this fixes).
      out.push(b);
    }
  }
  return out;
}
// Text that stopped mid-sentence: no sentence-terminal punctuation, no colon
// (a colon is a real forward-pointing beat, keep it separate), no trailing
// newline (a newline is a deliberate paragraph break, not a stream split).
function isMidSentence(t: string): boolean {
  if (/\n\s*$/.test(t)) return false;
  // ends with . ! ? or : (optionally wrapped by a quote/bracket) → sentence done
  return !/[.!?:]["'`)\]]?\s*$/.test(t);
}
// A fragment that reads as the tail of an interrupted sentence: begins lowercase,
// or with closing/mid punctuation (")." , ";" …), never with a capital-letter
// sentence start.
function isContinuation(t: string): boolean {
  const s = t.replace(/^\s+/, "");
  if (s.length === 0) return false;
  return /^[a-z0-9,;:)\]'"`.\-]/.test(s);
}

// Group consecutive tool blocks into work runs (say blocks pass through).
export function groupBlocks(rawBlocks: StreamBlock[]): Group[] {
  const blocks = stitchSayFragments(rawBlocks);
  const out: Group[] = [];
  let work: StreamTool[] | null = null;
  for (const b of blocks) {
    if (b.type === "tool") {
      if (!work) work = [];
      work.push(b.tool);
    } else {
      if (work) { out.push({ type: "work", segs: segmentWork(work) }); work = null; }
      out.push({ type: "say", text: b.text });
    }
  }
  if (work) out.push({ type: "work", segs: segmentWork(work) });
  return out;
}

const isRichKind = (k: TKind) => k === "plan" || k === "web" || k === "fetch" || k === "test" || k === "lint" || k === "agent" || k === "ask" || k === "exitplan";

// A tool gets its own rich block when its KIND is inherently rich, OR it's a
// shell command that (a) is still RUNNING — so it renders as a terminal block
// the moment it starts, not as a WorkLine row that jumps into a block once
// output lands (the flicker seen mid-stream) — or (b) produced output. A bare
// FINISHED command with no output (a quick `cd`) stays in the lightweight
// WorkLine batch, keeping trivial steps calm. Pref-gating of the body at render.
const isRich = (t: StreamTool) =>
  isRichKind(t.kind) ||
  (t.kind === "shell" &&
    (t.status === "pending" || (typeof t.result === "string" && t.result.trim().length > 0)));

// Split a work run: rich tools each get their own block; edits batch; the rest
// collapse to one quiet WorkLine. Order preserved.
function segmentWork(tools: StreamTool[]): WorkSeg[] {
  const segs: WorkSeg[] = [];
  let cur: { seg: "edit" | "other"; tools: StreamTool[] } | null = null;
  for (const t of tools) {
    if (isRich(t)) {
      cur = null;
      // Coalesce consecutive plan blocks: the newer CLI emits one TaskCreate /
      // TaskUpdate per item, so a 4-item plan would otherwise render 4 separate
      // plan cards. They all describe the same evolving plan, so keep ONE rich
      // seg (the latest block) — StreamTurn renders it from the live aggregate.
      const last = segs[segs.length - 1];
      if (t.kind === "plan" && last?.seg === "rich" && last.tool.kind === "plan") {
        last.tool = t;
        continue;
      }
      segs.push({ seg: "rich", tool: t });
      continue;
    }
    const grp = t.kind === "edit" || t.kind === "create" ? "edit" : "other";
    if (!cur || cur.seg !== grp) { cur = { seg: grp, tools: [] }; segs.push(cur); }
    cur.tools.push(t);
  }
  return segs;
}

// Lower-case verb phrase per kind for a mixed-kind breakdown ("read 3 · searched
// 2 · ran 1"). Keeps the same vocabulary as VERB_PAST but count-friendly.
const KIND_VERB: Record<TKind, string> = {
  read: "read", grep: "searched", edit: "edited", create: "created", shell: "ran",
  agent: "delegated", web: "searched the web", fetch: "fetched", test: "tested",
  lint: "checked", mcp: "called", plan: "planned", ask: "asked", exitplan: "proposed a plan",
};

function groupSummary(tools: StreamTool[]): string {
  const kinds = new Set(tools.map((t) => t.kind));
  const n = tools.length;
  if (kinds.size === 1) {
    const k = [...kinds][0];
    if (k === "shell") return `Ran ${n} command${n > 1 ? "s" : ""}`;
    if (k === "read") return `Read ${n} file${n > 1 ? "s" : ""}`;
    if (k === "grep") return n > 1 ? `Searched ${n} times` : "Searched the repo";
    if (k === "mcp") return `Ran ${n} tool${n > 1 ? "s" : ""}`;
  }
  // Mixed kinds: name what actually ran, dominant-first ("read 4 · searched 2 ·
  // ran 1"), instead of a flat "Ran N steps". Cap at 3 segments + a tail count.
  const counts = new Map<TKind, number>();
  for (const t of tools) counts.set(t.kind, (counts.get(t.kind) ?? 0) + 1);
  const ordered = [...counts].sort((a, b) => b[1] - a[1]);
  const segs = ordered.slice(0, 3).map(([k, c]) => `${KIND_VERB[k]} ${c}`);
  const restKinds = ordered.slice(3);
  const restCount = restKinds.reduce((s, [, c]) => s + c, 0);
  const tail = restCount > 0 ? ` · +${restCount} more` : "";
  const body = segs.join(" · ");
  return body.charAt(0).toUpperCase() + body.slice(1) + tail;
}

// Names-on-rows summary: instead of "Read 2 files", show "Read layout.ts,
// rest.ts" so the collapsed work line names its targets. Targets are each
// tool's basename caption, de-duped, capped so the row stays one line.
//
// Kinds whose caption IS a target name (file/dir/pattern) get name-listed; the
// rest (shell commands, mcp calls) only count. A MIXED group is now segmented
// per-kind, dominant-first, so "Read 2 files · searched 1" becomes the far more
// useful "Read layout.ts, rest.ts · searched \"apply_slot\"" — the filenames
// were always in the data, the old flat-count summary just threw them away.
const NAMABLE = (k: TKind) => k === "read" || k === "grep" || k === "edit" || k === "create";

// One verb-led segment for a single kind's tools: names the targets if namable,
// else defers to a bare count via KIND_VERB ("ran 2").
function nameSeg(k: TKind, tools: StreamTool[], nameBudget: number): string {
  const c = tools.length;
  if (!NAMABLE(k)) return `${KIND_VERB[k]} ${c}`;
  const names: string[] = [];
  for (const t of tools) {
    const nm = (t.cap ?? "").trim();
    if (nm && !names.includes(nm)) names.push(nm);
  }
  if (names.length === 0) return `${KIND_VERB[k]} ${c}`;
  const shown = names.slice(0, nameBudget).join(", ");
  const more = names.length > nameBudget ? ` +${names.length - nameBudget} more` : "";
  return `${VERB_PAST[k]} ${shown}${more}`;
}

export function groupNames(tools: StreamTool[]): string {
  const kinds = new Set(tools.map((t) => t.kind));

  // Single kind: name up to 3 targets (the common, readable case).
  if (kinds.size === 1) {
    const k = [...kinds][0] as TKind;
    if (!NAMABLE(k)) return groupSummary(tools);
    return nameSeg(k, tools, 3);
  }

  // Mixed kinds: one segment per kind, dominant-first, joined by " · ". Give
  // the lead kind a wider name budget so the primary target list stays useful;
  // trailing kinds get 1 name so the whole row stays a single line. Cap at 3
  // segments + a "+N more" tail (mirrors groupSummary's shape).
  const byKind = new Map<TKind, StreamTool[]>();
  for (const t of tools) {
    const arr = byKind.get(t.kind) ?? [];
    arr.push(t);
    byKind.set(t.kind, arr);
  }
  const ordered = [...byKind].sort((a, b) => b[1].length - a[1].length);
  const segs = ordered
    .slice(0, 3)
    .map(([k, ts], i) => nameSeg(k, ts, i === 0 ? 2 : 1));
  const restCount = ordered.slice(3).reduce((s, [, ts]) => s + ts.length, 0);
  const tail = restCount > 0 ? ` · +${restCount} more` : "";
  const body = segs.join(" · ");
  return body.charAt(0).toUpperCase() + body.slice(1) + tail;
}

// ── Tool-detail density ─────────────────────────────────────────────────────
// Map the `toolDetail` pref tier onto a WorkLine render mode. Pure so it's unit-
// testable + shared, mirroring classifySay/groupNames.
//  - "collapsed": minimal — one named outcome line, chevron still expands.
//  - "rows":      balanced — named header, click to expand the per-tool list.
//  - "expanded":  detailed — rows auto-open with full paths (+full shell output,
//                 handled in StreamShell).
export type ToolDetailTier = "minimal" | "balanced" | "detailed";
export function workLineMode(tier: ToolDetailTier): "collapsed" | "rows" | "expanded" {
  if (tier === "minimal") return "collapsed";
  if (tier === "detailed") return "expanded";
  return "rows";
}

// ── Narration classification ────────────────────────────────────────────────
// The model narrates between tool calls — "Now I'll build:", "Compiled clean —
// copy to bin and launch:", "The exe is still locked, kill it harder:". In the
// CLI that reads as dim connective tissue hugging the work stream; rendered as a
// full prose block it reads as *chat between tools* and makes a working turn feel
// chatty. We classify each `say` block into one of three weights, and the
// narration-density pref decides what each weight does on screen.
//
//  - "filler"  : a short single-sentence lead-in ("Let me check the routes.") —
//                pure announcement, the work row says the same thing.
//  - "connective": a short between-tools beat that DOES carry a fact ("Compiled
//                clean — now the release build:", "exe still locked, kill harder:").
//                Trailing colon, or "<did X>. <now Y>" shape, ≤2 sentences, short.
//  - "prose"   : everything else — real answers, explanations, anything long /
//                multi-line / code-bearing. NEVER demoted or hidden.
export type SayWeight = "filler" | "connective" | "prose";

const FILLER_LEAD = /^(?:let me|let's|now (?:i'?ll|let)|i'?ll|i'?m going to|first,?\s|next,?\s|then,?\s|okay,?\s|alright,?\s|sure,?\s)/i;
// A between-tools beat typically ends by pointing at the next action — a trailing
// colon ("…launch-test:") — or pairs a result with the next step ("Compiled
// clean. Now …"). Verbs that report a just-finished step lead these often.
const STEP_REPORT_LEAD = /^(?:compiled|built|done|ok|good|great|confirmed|fixed|that|the |both |running|installed|copied|removed|deleted|created|added|now |all )/i;

export function classifySay(text: string): SayWeight {
  const t = text.trim();
  if (t.length === 0) return "prose";
  if (/\n/.test(t)) return "prose";   // multi-line → real content
  if (/```/.test(t)) return "prose";  // has code → keep
  if (/`[^`]+`/.test(t) && t.length > 90) return "prose"; // code-heavy explanation

  const sentences = t.split(/[.!?]+\s/).filter(Boolean);

  // Pure filler: one short sentence opening with a throwaway lead-in.
  if (t.length <= 120 && sentences.length <= 1 && FILLER_LEAD.test(t)) return "filler";

  // Connective beat: short (≤200), ≤2 sentences, AND it either points forward
  // (ends with ":") or reports-then-pivots (a step-report lead). These are the
  // "Now kill the instance and build:" / "Compiled clean. Copy to bin:" lines.
  if (t.length <= 200 && sentences.length <= 2) {
    if (/:$/.test(t)) return "connective";
    if (STEP_REPORT_LEAD.test(t) && /\b(now|next|then|let|copy|launch|build|run|kill|update|commit|check)\b/i.test(t)) {
      return "connective";
    }
  }
  return "prose";
}

// Back-compat for the existing unit test + any caller wanting the old boolean.
export function isFillerSay(text: string): boolean {
  return classifySay(text) === "filler";
}

export const VERB_PAST: Record<TKind, string> = {
  read: "Read", grep: "Searched", edit: "Edited", create: "Created", shell: "Ran",
  agent: "Delegated", web: "Searched the web", fetch: "Fetched", test: "Tested",
  lint: "Checked", mcp: "Called", plan: "Planned", ask: "Asked", exitplan: "Proposed a plan",
};
export const VERB_ING: Record<TKind, string> = {
  read: "Reading", grep: "Searching", edit: "Editing", create: "Creating", shell: "Running",
  agent: "Delegating", web: "Searching the web", fetch: "Fetching", test: "Running tests",
  lint: "Type-checking", mcp: "Calling", plan: "Planning", ask: "Waiting for your answer",
  exitplan: "Proposing a plan",
};

export type AnsweredPair = { question: string; answers: string[] };

// The backend (mcp_server.rs::format_ask_user_result) hands back the answered
// ask_user tool_result as plain text Claude reads — blocks of "Q: <question>\n
// A: <label>[, <label>…]" joined by newlines, OR a dismissal sentence. The card
// rendered that raw in a <pre> (the "looks like shit" complaint). Parse it back
// into structured pairs so the answered state can render clean chips. Multi-
// select answers are comma-joined by the backend, so split A: on ", ".
// Returns [] for the dismissal sentence or any unparseable text (caller falls
// back to a neutral state). Kept pure + exported for unit tests.
export function parseAskUserResult(result: string | null | undefined): AnsweredPair[] {
  if (!result || /^User dismissed the question/i.test(result)) return [];
  const out: AnsweredPair[] = [];
  // Each block starts with "Q: "; subsequent blocks are delimited by "\nQ: ".
  const blocks = result.split(/\nQ: /).map((b, i) => (i === 0 ? b.replace(/^Q: /, "") : b));
  for (const block of blocks) {
    // Greedy question group so a model-supplied "\nA: " inside the question
    // body can't truncate it + forge the answer — the real answer is the LAST
    // "\nA: " segment (backend appends it last).
    const m = block.match(/^([\s\S]*)\nA: ([\s\S]*)$/);
    if (!m) continue;
    const question = m[1].trim();
    // Backend joins multi-select labels with US (\x1F) so a label containing
    // ", " can't fracture into phantom answers (A1). Fall back to ", " for any
    // legacy/cached result that predates the US delimiter.
    const raw = m[2];
    const answers = (raw.includes("\u{1f}") ? raw.split("\u{1f}") : raw.split(", "))
      .map((s) => s.trim())
      .filter(Boolean);
    if (question) out.push({ question, answers });
  }
  return out;
}
