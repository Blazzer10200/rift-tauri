// Parse a raw `git diff` unified-diff string into render-ready rows. The
// existing EditDiff/SessionDiff path consumes Claude's old/new-string Edit
// payloads (diffArrays), NOT `@@` hunk text — so the Environment panel needs
// its own lightweight unified-diff parser. Header/index lines are dropped (the
// FileDiffCard renders its own header); hunk headers + +/-/context lines carry
// old/new line numbers for the gutter.

export type DiffLineKind = "add" | "del" | "ctx" | "hunk";

export type DiffLine = {
  kind: DiffLineKind;
  text: string;
  oldNo: number | null;
  newNo: number | null;
};

export type ParsedDiff = { lines: DiffLine[]; binary: boolean };

const HUNK_RE = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

export function parseUnifiedDiff(raw: string): ParsedDiff {
  const lines: DiffLine[] = [];
  let oldNo = 0;
  let newNo = 0;
  let binary = false;

  for (const line of raw.split("\n")) {
    if (
      line.startsWith("diff --git") ||
      line.startsWith("index ") ||
      line.startsWith("--- ") ||
      line.startsWith("+++ ") ||
      line.startsWith("new file") ||
      line.startsWith("deleted file") ||
      line.startsWith("similarity ") ||
      line.startsWith("rename ") ||
      line.startsWith("old mode") ||
      line.startsWith("new mode") ||
      line.startsWith("\\")
    ) {
      continue; // header / "\ No newline at end of file"
    }
    if (line.startsWith("Binary files")) {
      binary = true;
      continue;
    }
    const hunk = HUNK_RE.exec(line);
    if (hunk) {
      oldNo = parseInt(hunk[1], 10);
      newNo = parseInt(hunk[2], 10);
      lines.push({ kind: "hunk", text: line, oldNo: null, newNo: null });
      continue;
    }
    if (line.startsWith("+")) {
      lines.push({ kind: "add", text: line.slice(1), oldNo: null, newNo: newNo++ });
    } else if (line.startsWith("-")) {
      lines.push({ kind: "del", text: line.slice(1), oldNo: oldNo++, newNo: null });
    } else {
      const text = line.startsWith(" ") ? line.slice(1) : line;
      lines.push({ kind: "ctx", text, oldNo: oldNo++, newNo: newNo++ });
    }
  }
  // Trailing blank from the final newline split — drop a lone empty ctx tail.
  if (lines.length && lines[lines.length - 1].kind === "ctx" && lines[lines.length - 1].text === "") {
    lines.pop();
  }
  return { lines, binary };
}

// Collapse runs of unmodified context longer than `threshold` into a single
// foldable segment, à la the "N unmodified lines" rows in the reference UI.
export type DiffSegment =
  | { fold: false; line: DiffLine }
  | { fold: true; count: number; lines: DiffLine[] };

export function foldContext(lines: DiffLine[], threshold = 6, keep = 3): DiffSegment[] {
  const out: DiffSegment[] = [];
  let i = 0;
  while (i < lines.length) {
    if (lines[i].kind !== "ctx") {
      out.push({ fold: false, line: lines[i] });
      i++;
      continue;
    }
    // Gather the contiguous context run.
    let j = i;
    while (j < lines.length && lines[j].kind === "ctx") j++;
    const run = lines.slice(i, j);
    // Only fold interior runs that are long enough; show `keep` on each side.
    if (run.length > threshold && i > 0 && j < lines.length) {
      run.slice(0, keep).forEach((line) => out.push({ fold: false, line }));
      out.push({ fold: true, count: run.length - keep * 2, lines: run.slice(keep, run.length - keep) });
      run.slice(run.length - keep).forEach((line) => out.push({ fold: false, line }));
    } else {
      run.forEach((line) => out.push({ fold: false, line }));
    }
    i = j;
  }
  return out;
}
