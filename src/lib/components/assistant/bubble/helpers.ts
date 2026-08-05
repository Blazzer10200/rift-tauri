// Pure helpers extracted from MessageBubble.svelte; see docs/ARCHITECTURE.md#frontend-map.
// Zero DOM/store deps beyond the args passed in; unit-tested in helpers.test.ts.
import type { Block, ThinkingBlock } from "../../../state/assistant.svelte";
import { diffArrays } from "diff";
import { captionForTool, captionForGroup } from "../toolCaption";

// Tool blocks that render inline as a full side-by-side diff (vs the
// compact ToolChip). Edit-family only — everything else gets a chip.
export function isInlineDiffTool(name: string): boolean {
  const sn = name.replace(/^mcp__rift__/, "");
  return sn === "Edit" || sn === "MultiEdit" || sn === "Write";
}
export function shortToolName(name: string): string { return name.replace(/^mcp__rift__/, ""); }
// Card-style tools render first-class chrome (their body IS the message) —
// never fold them into a collapsed tool group.
export function isCardTool(name: string): boolean {
  return /^(mcp__rift__)?(Agent|Task)$/.test(name)
    || /^(mcp__rift__)?TodoWrite$/.test(name)
    || /^mcp__rift__ask_user$/.test(name);
}
// Groupable = a plain status chip (Read/Grep/Bash/…): not an inline diff
// (Edit/MultiEdit) and not a first-class card. Runs of these collapse.
export function isGroupableChip(name: string): boolean {
  return !isInlineDiffTool(name) && !isCardTool(name);
}

// Detect every "Step N — title" header line in a text block. Returns an
// ordered list of prose segments interleaved with header markers so that a
// single text block containing multiple headers (e.g. "## Step 1 — A\n##
// Step 2 — B") splits into multiple step groups instead of collapsing the
// tail into the first group's body.
export type TextSegment =
  | { kind: "prose"; text: string }
  | { kind: "header"; stepNum: number; title: string };
const STEP_HEADER_LINE = /^\s*(?:#{1,6}\s+)?(?:\*\*)?Step\s+(\d+)\s*[—–\-:→»]\s*(.*?)(?:\*\*)?\s*$/i;
export function parseTextBlock(text: string): TextSegment[] {
  const out: TextSegment[] = [];
  const lines = text.split("\n");
  let prose: string[] = [];
  const flushProse = () => {
    const joined = prose.join("\n").trim();
    if (joined) out.push({ kind: "prose", text: joined });
    prose = [];
  };
  for (const line of lines) {
    const m = line.match(STEP_HEADER_LINE);
    if (m) {
      flushProse();
      const stepNum = parseInt(m[1], 10);
      const title = (m[2] ?? "").replace(/[*_`]+$/, "").trim() || `Step ${stepNum}`;
      out.push({ kind: "header", stepNum, title });
    } else {
      prose.push(line);
    }
  }
  flushProse();
  return out;
}

// When the model interleaves a tool call mid-header (e.g. emits "## S",
// calls Bash, then continues "tep 1 — Long bash\n## Step 2 — …"), the text
// arrives in two separate blocks separated by tool blocks. marked.js parses
// the first chunk as a tear-shaped <h2>S</h2> and the second chunk as
// prose, killing the header. This pre-pass detects that pattern (a text
// block that's only a partial header prefix — `## ` plus a word fragment
// with no terminator) and surgically reconstructs the first complete header
// line, leaving any remaining tail (e.g. a *second* "## Step 2 …" header)
// as its own text block AFTER the intervening tool calls — so the tools
// attach to the recovered first step, not the second.
const PARTIAL_HEADER = /^\s*#{1,6}\s+(?:\*\*)?[A-Za-z]+\s*$/;
export function reconcileSplitHeaders(blocks: Block[]): Block[] {
  const out: Block[] = [];
  let i = 0;
  while (i < blocks.length) {
    const b = blocks[i];
    if (b.type !== "text" || !PARTIAL_HEADER.test(b.text)) {
      out.push(b);
      i++;
      continue;
    }
    // Lookahead — collect non-text blocks until the next text block.
    let j = i + 1;
    const interim: Block[] = [];
    while (j < blocks.length && blocks[j].type !== "text") {
      interim.push(blocks[j]);
      j++;
    }
    const candidate = j < blocks.length ? blocks[j] : null;
    const tail = candidate && candidate.type === "text" ? candidate : null;
    if (!tail) {
      out.push(b);
      i++;
      continue;
    }
    const merged = b.text + tail.text;
    // Confirm the merge actually produces a Step header — otherwise this
    // partial wasn't a header at all, leave both blocks alone.
    if (!/(?:^|\n)\s*(?:#{1,6}\s+)?(?:\*\*)?Step\s+\d+/i.test(merged)) {
      out.push(b);
      i++;
      continue;
    }
    // Split the merged text at the first newline — that's the end of the
    // recovered header line. Everything after stays as the tail block.
    const firstNl = merged.indexOf("\n");
    const firstLine = firstNl === -1 ? merged : merged.slice(0, firstNl);
    const remainder = firstNl === -1 ? "" : merged.slice(firstNl + 1);
    out.push({ type: "text", text: firstLine });
    out.push(...interim);
    if (remainder.trim()) out.push({ type: "text", text: remainder });
    i = j + 1;
  }
  return out;
}

// Local/open-weights models sometimes emit a tool_use ONE text-delta early:
// the model is mid-sentence ("…the current state of your projec"), the
// tool_use envelope lands, then the final fragment ("t.") arrives as a fresh
// text block AFTER the tool chip. The stream pump can't merge across the tool
// (it only coalesces with the immediately-preceding block), so the bubble
// renders the sentence split with an orphan fragment dangling below the chip.
// This pre-pass detects that signature — a head text block that does NOT end
// on a sentence terminator, separated from a tail text block ONLY by tool
// blocks, where the tail begins mid-sentence (lowercase / contraction) — and
// stitches the sentence back together, placing the whole line AFTER the tools
// (the model's intent: finish the thought, then the tool result follows).
// Narrowly gated so deliberate "Let me check X." → tool → "Found it." prose is
// left untouched: the head must look truncated and the tail must look like a
// continuation, not a new sentence.
const SENTENCE_END = /[.!?:)\]}"'`\n]\s*$/;
// Continuation punctuation only — NOT sentence terminators. A tail that opens
// with `.`/`!`/`?` is a new sentence, not a mid-construct split, so it must not
// trigger the stitch (else "real logic" + ".Done." fuse + reorder past the tool).
const TAIL_CONTINUES = /^\s*(?:[a-z]|['’](?:s|t|re|ve|ll|d|m)\b|[,;:)\]}])/;
export function mergeSplitProse(blocks: Block[]): Block[] {
  const out: Block[] = [];
  let i = 0;
  while (i < blocks.length) {
    const b = blocks[i];
    // Head must be a text block that looks cut off mid-sentence.
    if (b.type !== "text" || b.text.length === 0 || SENTENCE_END.test(b.text)) {
      out.push(b);
      i++;
      continue;
    }
    // Collect interim blocks until the next text block; bail if anything but
    // tools sits between (thinking/boundary = a real structural break).
    let j = i + 1;
    const interim: Block[] = [];
    while (j < blocks.length && blocks[j].type !== "text") {
      if (blocks[j].type !== "tool") break;
      interim.push(blocks[j]);
      j++;
    }
    const tail = j < blocks.length && blocks[j].type === "text" ? blocks[j] : null;
    // Only stitch when tools (≥1) actually intervened AND the tail reads as a
    // continuation of the same sentence — otherwise leave the blocks alone.
    if (interim.length === 0 || !tail || tail.type !== "text" || !TAIL_CONTINUES.test(tail.text)) {
      out.push(b);
      i++;
      continue;
    }
    // The split is mid-word/mid-sentence (the stream cut between deltas), so
    // the fragments rejoin with no separator — any real space was already in
    // one of the two chunks.
    out.push(...interim);
    out.push({ type: "text", text: b.text + tail.text });
    i = j + 1;
  }
  return out;
}

// Timeline-flat units. Step-N headers from prose become dividers (small
// inline labels on the rail) instead of numbered groups. Every other
// block becomes its own node on the chain.
export type NodeStatus = "neutral" | "pending" | "done" | "error";
export type TimelineUnit =
  | { kind: "block"; block: Block; key: string; status: NodeStatus; stepNum?: number; caption?: string }
  | { kind: "toolgroup"; blocks: Block[]; key: string; status: NodeStatus; stepNum?: number; caption?: string }
  | { kind: "divider"; stepNum: number; title: string; key: string };

export function statusOf(b: Block): NodeStatus {
  if (b.type === "tool") {
    if (b.status === "error" || b.isError) return "error";
    if (b.status === "pending") return "pending";
    return "done";
  }
  if (b.type === "thinking") return b.status === "active" ? "pending" : "done";
  return "neutral";
}
export function nodeKind(b: Block): "thinking" | "prose" | "tool" | "edit" | "image" {
  if (b.type === "thinking") return "thinking";
  if (b.type === "text") return "prose";
  if (b.type === "image") return "image";
  if (b.type === "tool") return isInlineDiffTool(b.name) ? "edit" : "tool";
  return "prose";
}

export function formatBoundaryAt(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour12: true });
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return "<1s";
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return rem === 0 ? `${m}m` : `${m}m ${rem}s`;
}

// Sub-second-aware: tool durations are often tens of ms, where formatDuration's
// "<1s" floor hides the signal. Show "Nms" below a second, else fall through.
export function formatDurationMs(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return formatDuration(ms);
}

// Wall-clock sum across a group's tool blocks (thinking/absorbed blocks ignored).
export function groupDurationMs(blocks: Block[]): number {
  let ms = 0;
  for (const b of blocks) {
    if (b.type === "tool" && b.durationMs != null) ms += b.durationMs;
  }
  return ms;
}

export function elapsedFor(b: ThinkingBlock, nowMs: number): string {
  // Done block → stored duration. Active block → live ms-from-start so the
  // role-row label ticks up as the model reasons (otherwise "Thinking …"
  // sits frozen for the full 17-40s on Opus tool-use turns).
  if (b.status === "done" && b.durationMs != null) return formatDuration(b.durationMs);
  if (b.status === "active") {
    const ms = Math.max(0, nowMs - b.startedAt);
    return formatDuration(ms);
  }
  return "…";
}

// Compact "Read ×3 · Grep · Bash" rollup for a collapsed tool-group head.
export function summarizeGroup(blocks: Block[]): string {
  const counts = new Map<string, number>();
  for (const b of blocks) {
    if (b.type !== "tool") continue;
    const n = shortToolName(b.name);
    counts.set(n, (counts.get(n) ?? 0) + 1);
  }
  const parts = [...counts].map(([n, c]) => (c > 1 ? `${n} ×${c}` : n));
  const shown = parts.slice(0, 4).join(" · ");
  return parts.length > 4 ? `${shown} +${parts.length - 4}` : shown;
}

// Tighten the resolved model id into a short human label. Handles both the
// dated/minor form and the dateless major-only form the 4.6 generation onward
// uses (a major release like Sonnet 5 omits the minor segment):
//   claude-sonnet-4-6-20251001  → Sonnet 4.6
//   claude-opus-4-7[1m]         → Opus 4.7
//   claude-haiku-4-5            → Haiku 4.5
//   claude-sonnet-5             → Sonnet 5    (dateless major-only)
//   claude-fable-5              → Fable 5
export function shortModel(id: string): string {
  // name, major, optional `-minor`. The minor group is non-greedy-optional so
  // `claude-sonnet-5` (no minor) still matches and renders as just "Sonnet 5".
  const m = /claude-(opus|sonnet|haiku|fable|mythos)-(\d+)(?:-(\d+))?/i.exec(id);
  if (!m) return id;
  const name = m[1][0].toUpperCase() + m[1].slice(1).toLowerCase();
  return m[3] ? `${name} ${m[2]}.${m[3]}` : `${name} ${m[2]}`;
}

export function lineDelta(oldSrc: unknown, newSrc: unknown): { adds: number; dels: number } {
  if (typeof oldSrc !== "string" || typeof newSrc !== "string") return { adds: 0, dels: 0 };
  // Normalize CRLF so a CRLF-old vs LF-new pair isn't counted as an all-lines-changed diff.
  const oldS = oldSrc.replace(/\r\n/g, "\n");
  const newS = newSrc.replace(/\r\n/g, "\n");
  // Skip exact diff for very large strings; return approx line counts instead.
  if (oldS.length + newS.length > 200_000) {
    return { adds: newS.split("\n").length, dels: oldS.split("\n").length };
  }
  let adds = 0, dels = 0;
  for (const c of diffArrays(oldS.split("\n"), newS.split("\n"))) {
    if (c.added) adds += c.count ?? c.value.length;
    else if (c.removed) dels += c.count ?? c.value.length;
  }
  return { adds, dels };
}

// Fold a run of GROUP_MIN+ consecutive plain tool chips into a single
// collapsible "N tools" node so a multi-tool turn stops reading as a wall of
// rows. Prose / thinking / edits / cards / images all break a run, so the
// narration↔tool ordering is preserved — only back-to-back status chips
// collapse. Runs shorter than GROUP_MIN stay inline as before.
const GROUP_MIN = 3;
// A short/empty completed thought. Claude emits one of these between most tool
// calls, which would otherwise break every run and prevent any grouping — so a
// quick thought mid-run is absorbed into the group (rendered inside its body)
// instead of splitting it. Substantial (>3s) or still-active thoughts stay
// visible on the spine and DO break the run.
function isQuickThinking(b: Block): boolean {
  return (
    b.type === "thinking" &&
    b.status === "done" &&
    (b.text.length === 0 || (b.durationMs != null && b.durationMs < 3000))
  );
}
export function coalesceToolGroups(units: TimelineUnit[]): TimelineUnit[] {
  const out: TimelineUnit[] = [];
  let run: Extract<TimelineUnit, { kind: "block" }>[] = [];
  const flush = () => {
    if (run.length === 0) return;
    // Threshold counts TOOLS, not units — interleaved quick-thoughts shouldn't
    // tip a 2-tool run over the grouping line.
    const toolCount = run.filter((u) => u.block.type === "tool").length;
    if (toolCount < GROUP_MIN) {
      out.push(...run);
    } else {
      const status: NodeStatus = run.some((u) => u.status === "error")
        ? "error"
        : run.some((u) => u.status === "pending")
          ? "pending"
          : "done";
      out.push({
        kind: "toolgroup",
        blocks: run.map((u) => u.block),
        key: `tg_${run[0].key}`,
        status,
      });
    }
    run = [];
  };
  for (const u of units) {
    const groupable =
      u.kind === "block" && u.block.type === "tool" && isGroupableChip(u.block.name);
    // Only absorb a quick thought when a run is already open (run starts on a
    // tool) so a lone thought never seeds a group.
    const absorbThought = u.kind === "block" && run.length > 0 && isQuickThinking(u.block);
    if (groupable || absorbThought) {
      run.push(u as Extract<TimelineUnit, { kind: "block" }>);
    } else {
      flush();
      out.push(u);
    }
  }
  flush();
  return out;
}

// Number each action unit (chip/group/edit) sequentially; caption = preceding
// "Step N" divider title if any, else synthesized. Orphan dividers kept.
export function numberActions(units: TimelineUnit[]): TimelineUnit[] {
  const out: TimelineUnit[] = [];
  let step = 0;
  let pending: { title: string; stepNum: number } | null = null;
  for (const u of units) {
    if (u.kind === "block" && u.block.type === "tool") {
      step++;
      const caption = pending?.title ?? captionForTool(u.block.name, u.block.input as Record<string, unknown>);
      pending = null;
      out.push({ ...u, stepNum: step, caption });
    } else if (u.kind === "toolgroup") {
      step++;
      const caption = pending?.title ?? captionForGroup(u.blocks);
      pending = null;
      out.push({ ...u, stepNum: step, caption });
    } else if (u.kind === "divider") {
      // Back-to-back headers: flush the earlier one as an orphan divider
      // instead of silently discarding it.
      if (pending) out.push({ kind: "divider", stepNum: pending.stepNum, title: pending.title, key: `od_${u.key}` });
      pending = { title: u.title, stepNum: u.stepNum };
    } else {
      if (pending) {
        out.push({ kind: "divider", stepNum: pending.stepNum, title: pending.title, key: `od_${u.key}` });
        pending = null;
      }
      out.push(u);
    }
  }
  if (pending) out.push({ kind: "divider", stepNum: pending.stepNum, title: pending.title, key: "od_tail" });
  return out;
}
