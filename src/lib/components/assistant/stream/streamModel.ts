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
import { diffArrays } from "diff";

export type TKind =
  | "read" | "grep" | "edit" | "create" | "shell"
  | "agent" | "web" | "fetch" | "test" | "lint" | "mcp" | "plan" | "ask";

export type PlanItem = { text: string; status: "done" | "active" | "todo" };

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
  steps?: string[]; task?: string; result?: string | null; // agent
};

export type StreamBlock =
  | { type: "say"; text: string }
  | { type: "tool"; tool: StreamTool }
  | { type: "steer"; text: string };

// What the turn actually did, so the footer can be honest:
//  - "applied": at least one edit/create tool succeeded → green "Applied"
//  - "ran":     ran tools but changed no files (read/grep/shell/web/…) → muted "Done"
//  - "failed":  edits/creates were attempted but every one errored → "Changes failed"
//  - "text":    no tools at all (a plain answer) → no status badge
export type TurnOutcome = "applied" | "ran" | "failed" | "text";

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
  | { type: "say"; text: string }
  | { type: "steer"; text: string };

// ── helpers (mirror ToolChip.svelte) ────────────────────────────────────────
const shortName = (n: string) => n.replace(/^mcp__rift__/, "");
const basename = (p: string) => p.split(/[\\/]/).pop() || p;
const trim = (s: string, n = 60) => (s.length > n ? s.slice(0, n - 1) + "…" : s);
const hostOf = (u: string) => { try { return new URL(u).host; } catch { return u; } };

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
  if (n === "Agent" || n === "Task") return "agent";
  if (n === "WebSearch") return "web";
  if (n === "WebFetch") return "fetch";
  if (n === "TodoWrite" || n === "TaskCreate" || n === "TaskUpdate") return "plan";
  if (n === "ask_user") return "ask";
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
  if (n === "Bash" || n === "remote_bash") return typeof inp.command === "string" ? trim(inp.command, 70) : "shell";
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
  return n;
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
  if (kind === "edit" || kind === "create") {
    t.input = inp;
    const dc = diffCountsCached(tb.id, inp);
    if (dc) { t.add = dc.add; t.del = dc.del; }
  }
  if (kind === "plan") t.items = planItems(tb);
  if (kind === "web" || kind === "fetch") {
    t.query = t.cap; t.sources = []; t.count = null;
  }
  if (kind === "agent") {
    t.task = t.cap; t.steps = [];
    t.result = tb.result && !tb.isError ? trim(tb.result.trim().split("\n")[0] ?? "", 90) : null;
  }
  if (kind === "ask") {
    // Carry the raw questions input + full tool_result through so the
    // interactive ask card can render options + the answered transcript.
    t.input = inp;
    t.result = tb.result ?? null;
  }
  return t;
}

// Map one live assistant ChatMessage → the StreamTurn render model.
export function messageToTurn(m: ChatMessage): TurnModel {
  const blocks: StreamBlock[] = [];
  let thinkText: string[] = [];
  let thinkSecs = 0;
  let thinkActive = false;
  let totalSecs = 0;

  for (const b of m.blocks as Block[]) {
    if (b.type === "text") {
      if (b.text.trim()) blocks.push({ type: "say", text: b.text });
    } else if (b.type === "thinking") {
      if (b.text.trim()) thinkText.push(b.text.trim());
      if (typeof b.durationMs === "number") { thinkSecs += b.durationMs / 1000; totalSecs += b.durationMs / 1000; }
      if (b.status === "active") thinkActive = true;
    } else if (b.type === "tool") {
      const tool = adaptTool(b);
      totalSecs += tool.durSecs;
      blocks.push({ type: "tool", tool });
    } else if (b.type === "steer") {
      blocks.push({ type: "steer", text: b.text });
    }
    // boundary / image blocks are not part of a stream turn body
  }

  const thinking = (thinkText.length || thinkActive)
    ? { active: thinkActive, durSecs: thinkSecs, text: thinkText.join("\n\n") }
    : null;

  // Classify what the turn did from its tools (not from cost — every turn costs).
  const tools = blocks.filter((b): b is { type: "tool"; tool: StreamTool } => b.type === "tool").map((b) => b.tool);
  const mutators = tools.filter((t) => t.kind === "edit" || t.kind === "create");
  const okMutators = mutators.filter((t) => t.status !== "error");
  const changedFiles = new Set(okMutators.map((t) => t.path ?? t.cap)); // distinct by full path

  let outcome: TurnOutcome;
  if (okMutators.length > 0) outcome = "applied";
  else if (mutators.length > 0) outcome = "failed"; // attempted edits, all errored
  else if (tools.length > 0) outcome = "ran"; // tools, but nothing mutating
  else outcome = "text"; // a plain answer

  // Footer time·cost line — shown for any turn that did work (not pure text).
  const cost = typeof m.costUsd === "number" && m.costUsd > 0 ? `$${m.costUsd.toFixed(2)}` : null;
  const meta = outcome === "text" ? null : { time: fmtDur(totalSecs), cost };

  return { blocks, thinking, outcome, files: changedFiles.size, meta, totalSecs };
}

// Group consecutive tool blocks into work runs (say/steer pass through).
export function groupBlocks(blocks: StreamBlock[]): Group[] {
  const out: Group[] = [];
  let work: StreamTool[] | null = null;
  for (const b of blocks) {
    if (b.type === "tool") {
      if (!work) work = [];
      work.push(b.tool);
    } else {
      if (work) { out.push({ type: "work", segs: segmentWork(work) }); work = null; }
      out.push(b.type === "say" ? { type: "say", text: b.text } : { type: "steer", text: b.text });
    }
  }
  if (work) out.push({ type: "work", segs: segmentWork(work) });
  return out;
}

const isRich = (k: TKind) => k === "plan" || k === "web" || k === "fetch" || k === "test" || k === "lint" || k === "agent" || k === "ask";

// Split a work run: rich tools each get their own block; edits batch; the rest
// collapse to one quiet WorkLine. Order preserved.
function segmentWork(tools: StreamTool[]): WorkSeg[] {
  const segs: WorkSeg[] = [];
  let cur: { seg: "edit" | "other"; tools: StreamTool[] } | null = null;
  for (const t of tools) {
    if (isRich(t.kind)) { cur = null; segs.push({ seg: "rich", tool: t }); continue; }
    const grp = t.kind === "edit" || t.kind === "create" ? "edit" : "other";
    if (!cur || cur.seg !== grp) { cur = { seg: grp, tools: [] }; segs.push(cur); }
    cur.tools.push(t);
  }
  return segs;
}

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
  return `Ran ${n} steps`;
}

// Names-on-rows summary: instead of "Read 2 files", show "Read layout.ts,
// rest.ts" so the collapsed work line names its targets. Verb comes from the
// lead tool's kind; targets are each tool's basename caption, de-duped, capped
// at 3 with a "+N more" tail. Falls back to groupSummary when the group is
// mixed-kind or its targets aren't file-ish names (shell commands, mcp calls).
export function groupNames(tools: StreamTool[]): string {
  const kinds = new Set(tools.map((t) => t.kind));
  // Only name-list the kinds whose caption IS a target name (file/dir/pattern).
  const namable = (k: TKind) => k === "read" || k === "grep" || k === "edit" || k === "create";
  if (kinds.size !== 1 || !namable([...kinds][0] as TKind)) return groupSummary(tools);
  const k = [...kinds][0] as TKind;
  const verb = VERB_PAST[k];
  const names: string[] = [];
  for (const t of tools) {
    const nm = (t.cap ?? "").trim();
    if (nm && !names.includes(nm)) names.push(nm);
  }
  if (names.length === 0) return groupSummary(tools);
  const shown = names.slice(0, 3).join(", ");
  const more = names.length > 3 ? ` +${names.length - 3} more` : "";
  return `${verb} ${shown}${more}`;
}

// Trivial transitional narration ("Let me look at the layout file.", "Now I'll
// check the routes.") reads as filler once the work rows name their targets.
// Drop a `say` block when it's a single short sentence opening with a known
// throwaway lead-in AND it sits adjacent to a work group (the work shows what
// the sentence was about to announce). Real prose — answers, multi-sentence
// explanation, anything long — is never touched.
const FILLER_LEAD = /^(?:let me|let's|now (?:i'?ll|let)|i'?ll|i'?m going to|first,?\s|next,?\s|then,?\s|okay,?\s|alright,?\s|sure,?\s)/i;
export function isFillerSay(text: string): boolean {
  const t = text.trim();
  if (t.length === 0 || t.length > 120) return false; // long → real content
  if (/\n/.test(t)) return false;                     // multi-line → real content
  if (/```/.test(t)) return false;                    // has code → keep
  // One sentence only (allow a trailing period/colon).
  const sentences = t.split(/[.!?]+\s/).filter(Boolean);
  if (sentences.length > 1) return false;
  return FILLER_LEAD.test(t);
}

export const VERB_PAST: Record<TKind, string> = {
  read: "Read", grep: "Searched", edit: "Edited", create: "Created", shell: "Ran",
  agent: "Delegated", web: "Searched the web", fetch: "Fetched", test: "Tested",
  lint: "Checked", mcp: "Called", plan: "Planned", ask: "Asked",
};
export const VERB_ING: Record<TKind, string> = {
  read: "Reading", grep: "Searching", edit: "Editing", create: "Creating", shell: "Running",
  agent: "Delegating", web: "Searching the web", fetch: "Fetching", test: "Running tests",
  lint: "Type-checking", mcp: "Calling", plan: "Planning", ask: "Waiting for your answer",
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
