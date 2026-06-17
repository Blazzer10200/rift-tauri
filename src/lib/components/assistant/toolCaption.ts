// Synthesized fallback captions for tool actions, used when the model didn't
// narrate a "Step N" line. MessageBubble folds in the model's own prose first.

type Input = Record<string, unknown>;

// Sibling of tabsbar/helpers.ts::leafName — same path-leaf extraction, kept
// local so the assistant family doesn't import across the shell boundary.
// Behavior must match leafName (its vitest + toolCaption.test.ts cover both).
export function basename(p: string): string {
  const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
  const parts = norm.split("/");
  return parts[parts.length - 1] || norm;
}
function shortName(name: string): string {
  return name.replace(/^mcp__rift__/, "");
}
function hostOf(u: string): string {
  try { return new URL(u).host; } catch { return u; }
}
function firstToken(cmd: string): string {
  const t = cmd.trim().split(/\s+/)[0] ?? cmd;
  return t.length > 24 ? t.slice(0, 23) + "…" : t;
}
function clip(s: string, n: number): string {
  return s.length > n ? s.slice(0, n - 1) + "…" : s;
}

/** Verb-led, sentence-case caption for a single tool call. */
export function captionForTool(name: string, input: Input = {}): string {
  const n = shortName(name);
  const s = (k: string) => (typeof input[k] === "string" ? (input[k] as string) : null);
  const fp = s("file_path") ?? s("path") ?? s("notebook_path");
  const base = fp ? basename(fp) : null;

  if (n === "Read" || n === "read_file") return base ? `Reading ${base}` : "Reading a file";
  if (n === "Write") return base ? `Creating ${base}` : "Writing a file";
  if (n === "Edit" || n === "MultiEdit" || n === "NotebookEdit")
    return base ? `Editing ${base}` : "Editing a file";
  if (n === "Bash" || n === "remote_bash") {
    const desc = s("description");
    if (desc) return clip(desc, 60);
    const cmd = s("command");
    const verb = n === "remote_bash" ? "Running remotely" : "Running";
    return cmd ? `${verb} \`${firstToken(cmd)}\`` : "Running a command";
  }
  if (n === "BashOutput") return "Checking shell output";
  if (n === "KillBash" || n === "KillShell") return "Stopping a shell";
  if (n === "Grep" || n === "grep") {
    const pat = s("pattern");
    const where = s("path");
    if (pat) return where ? `Searching for ${clip(pat, 32)} in ${basename(where)}` : `Searching for ${clip(pat, 40)}`;
    return "Searching files";
  }
  if (n === "Glob") {
    const pat = s("pattern");
    return pat ? `Finding ${clip(pat, 40)}` : "Finding files";
  }
  if (n === "list_dir") {
    const path = s("path");
    return path ? `Listing ${basename(path)}` : "Listing a directory";
  }
  if (n === "WebFetch") {
    const u = s("url");
    return u ? `Fetching ${hostOf(u)}` : "Fetching a page";
  }
  if (n === "WebSearch") {
    const q = s("query");
    return q ? `Searching the web for ${clip(q, 40)}` : "Searching the web";
  }
  if (n === "Agent") {
    const sa = s("subagent_type") ?? "general";
    const d = s("description");
    return d ? `Delegating: ${clip(d, 44)}` : `Delegating to ${sa}`;
  }
  if (n === "AskUserQuestion") return "Asking you a question";
  if (n === "ExitPlanMode") return "Presenting a plan";
  if (n === "SlashCommand") {
    const c = s("command");
    return c ? `Running ${clip(c, 40)}` : "Running a command";
  }
  if (n === "Skill") {
    const sk = s("skill");
    return sk ? `Using the ${sk} skill` : "Using a skill";
  }
  if (n === "TodoWrite") return "Updating the task list";
  if (n === "TaskCreate") {
    const subj = s("subject");
    return subj ? `Planning · ${clip(subj, 36)}` : "Adding a task";
  }
  if (n === "TaskUpdate") return "Updating the task list";
  return `Running ${n}`;
}

/** Caption for a coalesced tool group — one verb if homogeneous, else a count. */
export function captionForGroup(blocks: Array<{ type: string; name?: string }>): string {
  const names = blocks
    .filter((b) => b.type === "tool" && typeof b.name === "string")
    .map((b) => shortName(b.name as string));
  if (names.length === 0) return "Running tools";
  const distinct = new Set(names);
  const c = names.length;
  if (distinct.size === 1) {
    const n = names[0];
    if (n === "Read" || n === "read_file") return `Reading ${c} files`;
    if (n === "Grep" || n === "grep") return `Running ${c} searches`;
    if (n === "Glob") return `Running ${c} file searches`;
    if (n === "Bash" || n === "remote_bash") return `Running ${c} commands`;
    if (n === "WebFetch") return `Fetching ${c} pages`;
    if (n === "TaskCreate") return `Planning ${c} task${c === 1 ? "" : "s"}`;
    if (n === "TaskUpdate") return `Updating ${c} task${c === 1 ? "" : "s"}`;
    return `Running ${c} ${n} calls`;
  }
  // Heterogeneous. If one kind clearly dominates (≥2), lead with it so the
  // header stays specific ("Reading 3 files +1 more") instead of a flat count.
  // A flat all-distinct mix keeps the generic "Running N actions".
  const counts = new Map<string, number>();
  for (const n of names) counts.set(n, (counts.get(n) ?? 0) + 1);
  const [topName, topCount] = [...counts].sort((a, b) => b[1] - a[1])[0];
  if (topCount >= 2) {
    const lead = captionForGroup(
      Array.from({ length: topCount }, () => ({ type: "tool", name: topName })),
    );
    return `${lead} +${c - topCount} more`;
  }
  return `Running ${c} actions`;
}
