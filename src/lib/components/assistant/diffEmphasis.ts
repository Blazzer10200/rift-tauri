// #11 (design pass): word-level intra-line emphasis for EditDiff "mod" pairs —
// the GitHub/VS Code convention where the changed FRAGMENT of a modified line
// gets a stronger tint than the line wash. Pure string logic (no DOM, no
// DOMPurify) so it unit-tests in node; safety comes by construction — the only
// HTML this emits is re-escaped text slices plus self-built `<span class="…">`
// wrappers around tokens that were already DOMPurify-sanitized upstream.

import { diffWordsWithSpace } from "diff";

export type CharInterval = [start: number, end: number];

export type UnifiedPatchPair =
  | { kind: "ctx"; left: string; right: string }
  | { kind: "del"; left: string; right: null }
  | { kind: "add"; left: null; right: string }
  | { kind: "mod"; left: string; right: string }
  | { kind: "meta"; text: string };

/** Convert an App Server unified patch into the same line-pair model used by
 * Claude's old_string/new_string edits. File headers stay out of the body;
 * hunk headers remain as quiet metadata so location context is not lost. */
export function parseUnifiedPatch(
  patch: string,
  kind: "add" | "update" | "delete" = "update",
): UnifiedPatchPair[] {
  const normalized = patch.replace(/\r\n/g, "\n");
  if (!normalized) return [];
  const hasPatchMarkers = /^(?:@@|diff --git|--- |\+\+\+ )/m.test(normalized);
  if (!hasPatchMarkers) {
    const lines = normalized.endsWith("\n")
      ? normalized.slice(0, -1).split("\n")
      : normalized.split("\n");
    return lines.map((line) =>
      kind === "delete"
        ? { kind: "del" as const, left: line, right: null }
        : { kind: "add" as const, left: null, right: line },
    );
  }

  const out: UnifiedPatchPair[] = [];
  let removed: string[] = [];
  let added: string[] = [];
  const flush = () => {
    const count = Math.max(removed.length, added.length);
    for (let i = 0; i < count; i++) {
      const left = removed[i];
      const right = added[i];
      if (left !== undefined && right !== undefined) out.push({ kind: "mod", left, right });
      else if (left !== undefined) out.push({ kind: "del", left, right: null });
      else if (right !== undefined) out.push({ kind: "add", left: null, right });
    }
    removed = [];
    added = [];
  };

  const lines = normalized.endsWith("\n")
    ? normalized.slice(0, -1).split("\n")
    : normalized.split("\n");
  for (const line of lines) {
    if (line.startsWith("diff --git ") || line.startsWith("index ") || line.startsWith("--- ") || line.startsWith("+++ ")) {
      flush();
      continue;
    }
    if (line.startsWith("@@")) {
      flush();
      out.push({ kind: "meta", text: line });
      continue;
    }
    if (line === "\\ No newline at end of file") continue;
    if (line.startsWith("-")) {
      if (added.length > 0) flush();
      removed.push(line.slice(1));
      continue;
    }
    if (line.startsWith("+")) {
      added.push(line.slice(1));
      continue;
    }
    flush();
    out.push({
      kind: "ctx",
      left: line.startsWith(" ") ? line.slice(1) : line,
      right: line.startsWith(" ") ? line.slice(1) : line,
    });
  }
  flush();
  return out;
}

export function unifiedPatchCounts(
  patch: string,
  kind: "add" | "update" | "delete" = "update",
): { add: number; del: number } {
  let add = 0;
  let del = 0;
  for (const pair of parseUnifiedPatch(patch, kind)) {
    if (pair.kind === "add") add++;
    else if (pair.kind === "del") del++;
    else if (pair.kind === "mod") { add++; del++; }
  }
  return { add, del };
}

/** Fraction of combined line length that may change before emphasis is more
 *  noise than signal — beyond it the pair reads as "rewritten", and full-line
 *  tint (already present) says that better than confetti. */
const MAX_CHANGED_FRACTION = 0.65;

/** Word-level change intervals for a paired del/add ("mod") line. Returns
 *  `null` when emphasis would be noise: blank lines, no shared content, or
 *  the pair is mostly a rewrite (see MAX_CHANGED_FRACTION). */
export function emphasisIntervals(
  left: string,
  right: string,
): { del: CharInterval[]; add: CharInterval[] } | null {
  if (left.length === 0 || right.length === 0) return null;
  const parts = diffWordsWithSpace(left, right);
  const del: CharInterval[] = [];
  const add: CharInterval[] = [];
  let posL = 0;
  let posR = 0;
  let changed = 0;
  const push = (list: CharInterval[], start: number, end: number) => {
    // Merge adjacent/overlapping intervals as we go (parts arrive in order).
    const last = list[list.length - 1];
    if (last && start <= last[1]) last[1] = Math.max(last[1], end);
    else list.push([start, end]);
  };
  for (const p of parts) {
    const len = p.value.length;
    if (p.removed) {
      push(del, posL, posL + len);
      changed += len;
      posL += len;
    } else if (p.added) {
      push(add, posR, posR + len);
      changed += len;
      posR += len;
    } else {
      posL += len;
      posR += len;
    }
  }
  if (del.length === 0 && add.length === 0) return null;
  if (changed / (left.length + right.length) > MAX_CHANGED_FRACTION) return null;
  return { del, add };
}

const ESCAPES: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};
function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ESCAPES[c]);
}

/** Minimal entity decoder for the escape set shiki/DOMPurify emit. Numeric
 *  refs included so offset math survives `&#39;`-style output. */
function decodeEntities(s: string): string {
  return s.replace(/&(?:amp|lt|gt|quot|#39|#x27|#(\d+));/g, (m, num) => {
    switch (m) {
      case "&amp;": return "&";
      case "&lt;": return "<";
      case "&gt;": return ">";
      case "&quot;": return '"';
      case "&#39;":
      case "&#x27;": return "'";
      default: return num ? String.fromCodePoint(Number(num)) : m;
    }
  });
}

/** Wrap the slices of `raw` covered by `intervals` in an emphasis span,
 *  escaping everything. Used when there's no syntax-highlighted base. */
function emphasizePlain(raw: string, intervals: CharInterval[], cls: string): string {
  let out = "";
  let pos = 0;
  for (const [start, end] of intervals) {
    if (start > pos) out += escapeHtml(raw.slice(pos, start));
    out += `<span class="${cls}">${escapeHtml(raw.slice(start, end))}</span>`;
    pos = end;
  }
  if (pos < raw.length) out += escapeHtml(raw.slice(pos));
  return out;
}

/** Layer emphasis spans onto a line. `baseHtml` is EditDiff's sanitized shiki
 *  token HTML (a FLAT sequence of `<span …>text</span>` / bare text) or null
 *  for monochrome lines. Splits tokens at interval boundaries so syntax color
 *  and change emphasis coexist. Bails back to `baseHtml` untouched on any
 *  shape surprise (nested spans, malformed tags) — degradation is "no
 *  emphasis", never a broken line. */
export function emphasizeHtml(
  raw: string,
  intervals: CharInterval[],
  cls: string,
  baseHtml: string | null,
): string | null {
  if (intervals.length === 0) return baseHtml;
  if (baseHtml === null) return emphasizePlain(raw, intervals, cls);

  // Tokenize the flat span sequence. depth>1 or stray tags → bail.
  type Token = { open: string | null; text: string };
  const tokens: Token[] = [];
  const re = /<span\b[^>]*>|<\/span>/g;
  let depth = 0;
  let cursor = 0;
  let openTag: string | null = null;
  let m: RegExpExecArray | null;
  while ((m = re.exec(baseHtml)) !== null) {
    const between = baseHtml.slice(cursor, m.index);
    if (between) tokens.push({ open: depth === 1 ? openTag : null, text: between });
    if (m[0] === "</span>") {
      depth--;
      if (depth < 0) return baseHtml;
      openTag = null;
    } else {
      depth++;
      if (depth > 1) return baseHtml;
      openTag = m[0];
    }
    cursor = m.index + m[0].length;
  }
  if (depth !== 0) return baseHtml;
  const tail = baseHtml.slice(cursor);
  if (tail) tokens.push({ open: null, text: tail });

  // Re-emit tokens, splitting each token's decoded text at interval bounds.
  let out = "";
  let pos = 0; // raw-char offset across the whole line
  let iv = 0;
  for (const t of tokens) {
    const rawText = decodeEntities(t.text);
    const tokenEnd = pos + rawText.length;
    let inner = "";
    let p = pos;
    while (p < tokenEnd) {
      while (iv < intervals.length && intervals[iv][1] <= p) iv++;
      const cur = intervals[iv];
      if (!cur || cur[0] >= tokenEnd) {
        inner += escapeHtml(rawText.slice(p - pos, tokenEnd - pos));
        p = tokenEnd;
      } else if (cur[0] > p) {
        inner += escapeHtml(rawText.slice(p - pos, cur[0] - pos));
        p = cur[0];
      } else {
        const sliceEnd = Math.min(cur[1], tokenEnd);
        inner += `<span class="${cls}">${escapeHtml(rawText.slice(p - pos, sliceEnd - pos))}</span>`;
        p = sliceEnd;
      }
    }
    out += t.open ? `${t.open}${inner}</span>` : inner;
    pos = tokenEnd;
  }
  // Offset drift (decoded length ≠ raw line) means the base wasn't what we
  // assumed — emit the untouched base rather than misplaced emphasis.
  if (pos !== raw.length) return baseHtml;
  return out;
}
