<script lang="ts">
  // Inline tool block for the chat. Each non-Edit tool call lands here in
  // chronological position within the assistant message. Collapsed by
  // default — header shows icon + tool + 1-line summary + status. Click
  // to expand into input + result.
  //
  // Edits use the full side-by-side EditDiff component instead (handled by
  // the parent bubble); they never reach this component.

  import {
    FileText, FolderTree, Search, FilePen, FilePlus, Terminal, Globe,
    Wrench, Loader2, ListChecks,
    Bot, HelpCircle, FlagOff, Flag, BookOpen, Sparkles, Slash, Square, SkipForward,
    GitBranch, GitCommitHorizontal, AppWindow, Bell,
    Telescope, AlarmClock, Database, Radio, CalendarClock,
  } from "lucide-svelte";
  import type { ToolBlock } from "../../state/assistant.svelte";
  import { slide } from "svelte/transition";
  import { untrack } from "svelte";
  import { basename } from "./toolCaption";
  import AgentCard from "./toolchip/AgentCard.svelte";
  import TodoCard from "./toolchip/TodoCard.svelte";
  import AskUserCard from "./toolchip/AskUserCard.svelte";
  import BlockHeader from "./stream/BlockHeader.svelte";
  import OutputBlock from "./stream/OutputBlock.svelte";
  import ReadResult from "./stream/ReadResult.svelte";
  import GrepResult from "./stream/GrepResult.svelte";
  import { stripAnsi } from "./stream/streamModel";

  import { tooltip } from "$lib/actions/tooltip";
  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
  let { tool, variant = "card", caption = null }: { tool: ToolBlock; variant?: "card" | "timeline"; caption?: string | null } = $props();
  // Agent + TodoWrite + AskUser are first-class card variants — default-expanded
  // since their body IS the message, not a debug detail. All other tools collapse.
  const isAgent = $derived(/^(mcp__rift__)?(Agent|Task)$/.test(tool.name));
  const isTodoWrite = $derived(/^(mcp__rift__)?TodoWrite$/.test(tool.name));
  const isAskUser = $derived(/^mcp__rift__ask_user$/.test(tool.name));
  const isCard = $derived(isAgent || isTodoWrite || isAskUser);
  // Cards open by default; chips closed.
  let expanded = $state(untrack(() => isCard));
  // AskUser is fully owned by AskUserCard (state, store round-trip, card CSS) —
  // ToolChip just routes to it. See ./toolchip/AskUserCard.svelte.

  function shortName(name: string): string { return name.replace(/^mcp__rift__/, ""); }
  function trim(s: string, n = 60): string {
    return s.length > n ? s.slice(0, n - 1) + "…" : s;
  }
  function hostOf(u: string): string {
    try { return new URL(u).host; } catch { return u; }
  }
  // Compact 1-line summary — file path, command, pattern, etc. Full BUILTINS
  // coverage so every tool name in `--allowed-tools` (assistant/mod.rs) has
  // a dedicated chip rendering instead of falling through to the generic
  // tool-name default.
  const summary = $derived.by<string>(() => {
    const n = shortName(tool.name);
    const inp = tool.input ?? {};
    const fp = typeof inp.file_path === "string" ? basename(inp.file_path as string)
             : typeof inp.path === "string" ? basename(inp.path as string)
             : typeof inp.notebook_path === "string" ? basename(inp.notebook_path as string) : null;
    // File ops.
    if (n === "Read" || n === "read_file") return fp ?? "file";
    if (n === "Write") {
      const len = typeof inp.content === "string" ? (inp.content as string).length : 0;
      return fp ? `${fp} · ${len.toLocaleString()} chars` : "file";
    }
    if (n === "MultiEdit") {
      const count = Array.isArray(inp.edits) ? (inp.edits as unknown[]).length : 0;
      return fp ? `${fp} · ${count} edits` : `${count} edits`;
    }
    if (n === "NotebookEdit") return fp ?? "notebook";
    // Shell.
    if (n === "Bash" || n === "PowerShell") return typeof inp.command === "string" ? trim(inp.command as string, 70) : "shell";
    if (n === "BashOutput" || n === "TaskOutput") {
      const id = typeof inp.bash_id === "string" ? inp.bash_id : typeof inp.task_id === "string" ? inp.task_id : null;
      return id ? `tail ${id}` : "tail bg task";
    }
    if (n === "KillBash" || n === "KillShell" || n === "TaskStop") {
      const id = typeof inp.shell_id === "string" ? inp.shell_id : typeof inp.task_id === "string" ? inp.task_id : null;
      return id ? `stop ${id}` : "stop bg task";
    }
    if (n === "remote_bash") return typeof inp.command === "string" ? trim(inp.command as string, 70) : "remote shell";
    if (n === "LSP") {
      const op = typeof inp.operation === "string" ? (inp.operation as string) : "lsp";
      return typeof inp.filePath === "string" ? `${op} · ${basename(inp.filePath as string)}` : op;
    }
    // Search / nav.
    if (n === "Glob") {
      const pat = typeof inp.pattern === "string" ? (inp.pattern as string) : "?";
      const scope = typeof inp.path === "string" ? ` in ${inp.path}` : "";
      return `${pat}${scope}`;
    }
    if (n === "Grep" || n === "grep") {
      const pat = typeof inp.pattern === "string" ? `"${inp.pattern}"` : "?";
      const scope = typeof inp.path === "string" ? ` in ${inp.path}` : "";
      return `${pat}${scope}`;
    }
    if (n === "list_dir") return typeof inp.path === "string" ? (inp.path as string) : "directory";
    // Web.
    if (n === "WebFetch") return typeof inp.url === "string" ? hostOf(inp.url as string) : "url";
    if (n === "WebSearch") return typeof inp.query === "string" ? `"${trim(inp.query as string, 50)}"` : "search";
    // Agentic / planning / meta.
    if (n === "Agent") {
      const sa = typeof inp.subagent_type === "string" ? inp.subagent_type as string : "general";
      const desc = typeof inp.description === "string" ? ` · ${trim(inp.description as string, 40)}` : "";
      return `${sa}${desc}`;
    }
    if (n === "AskUserQuestion") {
      const qs = Array.isArray(inp.questions) ? (inp.questions as unknown[]).length : 0;
      return `${qs} question${qs === 1 ? "" : "s"}`;
    }
    if (n === "ExitPlanMode") return "exit plan mode";
    if (n === "EnterPlanMode") return "enter plan mode";
    if (n === "SlashCommand") return typeof inp.command === "string" ? (inp.command as string) : "slash command";
    if (n === "Skill") return typeof inp.skill === "string" ? (inp.skill as string) : "skill";
    if (n === "Workflow") return typeof inp.name === "string" ? (inp.name as string) : "workflow";
    if (n === "Monitor" || n === "REPL")
      return typeof inp.description === "string" ? trim(inp.description as string, 50) : n === "REPL" ? "javascript" : "watch";
    if (n === "Artifact") return typeof inp.file_path === "string" ? basename(inp.file_path as string) : "artifact";
    if (n === "ReportFindings") {
      const f = Array.isArray(inp.findings) ? (inp.findings as unknown[]).length : 0;
      return `${f} finding${f === 1 ? "" : "s"}`;
    }
    if (n === "TodoWrite") {
      const todos = Array.isArray(inp.todos) ? (inp.todos as Array<{content?: string}>).length : 0;
      return `${todos} task${todos === 1 ? "" : "s"}`;
    }
    if (n === "DesignSync") {
      const m = typeof inp.method === "string" ? inp.method as string : "sync";
      const files = Array.isArray(inp.files) ? (inp.files as unknown[]).length : 0;
      const paths = Array.isArray(inp.paths) ? (inp.paths as unknown[]).length : 0;
      const writes = Array.isArray(inp.writes) ? (inp.writes as unknown[]).length : 0;
      if (m === "write_files") return `write_files · ${files} file${files === 1 ? "" : "s"}`;
      if (m === "delete_files") return `delete_files · ${paths} path${paths === 1 ? "" : "s"}`;
      if (m === "finalize_plan") return `finalize_plan · ${writes} write${writes === 1 ? "" : "s"}`;
      if (m === "create_project" && typeof inp.name === "string") return `create_project · ${inp.name}`;
      if (m === "get_file" && typeof inp.path === "string") return `get_file · ${basename(inp.path as string)}`;
      if (m === "register_assets") {
        const a = Array.isArray(inp.assets) ? (inp.assets as unknown[]).length : 0;
        return `register_assets · ${a} card${a === 1 ? "" : "s"}`;
      }
      return m;
    }
    return n;
  });

  // Category drives the icon-color + border tint. Lets the eye distinguish
  // read-only (cheap, blue), mutation (accent), side-effect (warm),
  // agentic/meta (purple-ish) at a glance without parsing the tool name.
  type Category = "read" | "write" | "shell" | "agent" | "meta";
  const category = $derived.by<Category>(() => {
    const n = shortName(tool.name);
    if (n === "Edit" || n === "MultiEdit" || n === "Write" || n === "NotebookEdit") return "write";
    if (n === "Bash" || n === "PowerShell" || n === "remote_bash" || n === "BashOutput" || n === "KillBash" || n === "KillShell" || n === "TaskOutput" || n === "TaskStop" || n === "Monitor" || n === "REPL") return "shell";
    if (n === "Agent" || n === "Task" || n === "Skill" || n === "SlashCommand" || n === "Workflow" || n === "SendMessage") return "agent";
    if (n === "TodoWrite" || n === "TaskCreate" || n === "TaskUpdate" || n === "AskUserQuestion" || n === "ask_user" || n === "ExitPlanMode" || n === "EnterPlanMode" || n === "ReportFindings") return "meta";
    // Cloud publish + repo/worktree mutation read as consequential.
    if (n === "Artifact" || n === "EnterWorktree" || n === "ExitWorktree") return "write";
    // Local git: mutating ops read as consequential (write tint); read-only
    // status/diff/log stay cheap (read).
    if (n === "git_commit" || n === "git_push" || n === "git_pull") return "write";
    if (n === "git_status" || n === "git_diff" || n === "git_log") return "read";
    // Bridge side-effects — opening the dock / firing a toast.
    if (n === "open_browser" || n === "notify") return "shell";
    if (n === "DesignSync") {
      const m = typeof tool.input?.method === "string" ? tool.input.method as string : "";
      // Cloud-publishing methods read as consequential (write tint); reads stay cheap.
      return ["finalize_plan","write_files","delete_files","create_project","register_assets","unregister_assets"].includes(m) ? "write" : "read";
    }
    return "read";
  });

  // Format the result's first non-empty line as an inline preview when the
  // result is short and structurally a single line of useful text. Skips
  // when result is empty, multi-line w/ substance, or expansion would
  // confuse. Caps at 60 chars.
  const inlinePreview = $derived.by<string | null>(() => {
    if (!tool.result || tool.isError) return null;
    if (tool.status !== "done") return null;
    const raw = tool.result.trim();
    if (raw.length === 0) return null;
    const lines = raw.split("\n").map((l) => l.trim()).filter((l) => l.length > 0);
    if (lines.length === 0) return null;
    // Single-line short result → inline. Multi-line w/ 1-3 short lines → first line + "…".
    const first = lines[0];
    if (first.length > 60) return first.slice(0, 58) + "…";
    if (lines.length === 1) return first;
    if (lines.length <= 3 && raw.length <= 200) return first + " …";
    return null;
  });

  // Inline duration label for the chip head — only shown when the tool
  // actually took noticeable wall-clock time (>1s). For Bash/Agent calls
  // this surfaces slowness at a glance.
  function shortDuration(ms: number): string {
    if (ms < 1000) return "";
    const s = ms / 1000;
    if (s < 10) return `${s.toFixed(1)}s`;
    // Round the total ONCE before splitting (fmtDur's approach) — rounding
    // seconds and minutes independently emitted "60s"/"1m 60s" at :60 edges.
    const t = Math.round(s);
    if (t < 60) return `${t}s`;
    const m = Math.floor(t / 60), rem = t % 60;
    return rem ? `${m}m ${rem}s` : `${m}m`;
  }
  const durationLabel = $derived.by<string | null>(() => {
    if (tool.status !== "done") return null;
    if (typeof tool.durationMs !== "number" || tool.durationMs < 1000) return null;
    return shortDuration(tool.durationMs);
  });

  const Icon = $derived.by(() => {
    const sn = shortName(tool.name);
    // File ops.
    if (sn === "Read" || sn === "read_file") return FileText;
    if (sn === "Write") return FilePlus;
    if (sn === "Edit" || sn === "MultiEdit") return FilePen;
    if (sn === "NotebookEdit") return BookOpen;
    // Shell.
    if (sn === "Bash" || sn === "PowerShell" || sn === "remote_bash") return Terminal;
    if (sn === "BashOutput" || sn === "TaskOutput") return SkipForward;
    if (sn === "KillBash" || sn === "KillShell" || sn === "TaskStop") return Square;
    // Search / nav.
    if (sn === "list_dir" || sn === "Glob") return FolderTree;
    if (sn === "Grep" || sn === "grep") return Search;
    // Web.
    if (sn === "WebFetch" || sn === "WebSearch") return Globe;
    // In-app browser dock — not an external link (the page opens INSIDE Rift).
    if (sn === "open_browser") return AppWindow;
    if (sn === "notify") return Bell;
    // Local git (mcp__rift__git_*).
    if (sn === "git_commit") return GitCommitHorizontal;
    if (sn.startsWith("git_")) return GitBranch;
    // Agentic / planning / meta.
    if (sn === "Agent" || sn === "Task" || sn === "Workflow" || sn === "SendMessage") return Bot;
    if (sn === "AskUserQuestion") return HelpCircle;
    if (sn === "ExitPlanMode") return FlagOff;
    if (sn === "EnterPlanMode") return Flag;
    if (sn === "SlashCommand") return Slash;
    if (sn === "Skill") return Sparkles;
    if (sn === "Monitor" || sn === "REPL") return Terminal;
    if (sn === "Artifact") return Globe;
    if (sn === "TodoWrite" || sn === "TaskCreate" || sn === "TaskUpdate" || sn === "ReportFindings") return ListChecks;
    if (sn === "TaskGet" || sn === "TaskList") return ListChecks;
    // CLI built-ins that used to fall to the generic Wrench — give the ones a
    // user actually sees a real glyph (backwards-compat: any tool the CLI can
    // emit should read as itself, not an anonymous wrench).
    if (sn === "ToolSearch") return Telescope;
    if (sn === "ScheduleWakeup") return AlarmClock;
    if (sn === "PushNotification") return Bell;
    if (sn === "CronCreate" || sn === "CronDelete" || sn === "CronList") return CalendarClock;
    if (sn === "ListMcpResources" || sn === "ReadMcpResource" || sn === "ReadMcpResourceDir") return Database;
    if (sn === "RemoteTrigger") return Radio;
    return Wrench;
  });

  const toolLabel = $derived(shortName(tool.name));

  // Per-tool input projection — small key/value rows so the body stops
  // looking like a debug JSON dump. Returns null to fall back to the
  // raw JSON view for unknown tool shapes.
  type InputRow = { label: string; value: string; mono?: boolean };
  const inputRows = $derived.by<InputRow[] | null>(() => {
    const n = shortName(tool.name);
    const inp = tool.input ?? {};
    const s = (k: string) => typeof inp[k] === "string" ? (inp[k] as string) : null;
    const num = (k: string) => typeof inp[k] === "number" ? (inp[k] as number) : null;
    const rows: InputRow[] = [];
    if (n === "Bash" || n === "remote_bash") {
      // command moves to the terminal head (badge + $/PS> prefix) below.
      const desc = s("description"); if (desc) rows.push({ label: "purpose", value: desc });
      const to = num("timeout"); if (to) rows.push({ label: "timeout", value: `${to} ms`, mono: true });
      return rows;
    }
    if (n === "Read" || n === "read_file") {
      const fp = s("file_path") ?? s("path") ?? s("notebook_path"); if (fp) rows.push({ label: "file", value: fp, mono: true });
      const off = num("offset"); if (off != null) rows.push({ label: "offset", value: String(off), mono: true });
      const lim = num("limit"); if (lim != null) rows.push({ label: "limit", value: String(lim), mono: true });
      return rows;
    }
    if (n === "Write") {
      const fp = s("file_path"); if (fp) rows.push({ label: "file", value: fp, mono: true });
      const c = s("content"); if (c) rows.push({ label: "size", value: `${c.length.toLocaleString()} chars`, mono: true });
      return rows;
    }
    if (n === "Grep" || n === "grep") {
      const p = s("pattern"); if (p) rows.push({ label: "pattern", value: p, mono: true });
      const path = s("path"); if (path) rows.push({ label: "in", value: path, mono: true });
      const g = s("glob"); if (g) rows.push({ label: "glob", value: g, mono: true });
      const t = s("type"); if (t) rows.push({ label: "type", value: t, mono: true });
      const om = s("output_mode"); if (om) rows.push({ label: "mode", value: om });
      return rows;
    }
    if (n === "Glob") {
      const p = s("pattern"); if (p) rows.push({ label: "pattern", value: p, mono: true });
      const path = s("path"); if (path) rows.push({ label: "in", value: path, mono: true });
      return rows;
    }
    if (n === "list_dir") {
      const path = s("path"); if (path) rows.push({ label: "path", value: path, mono: true });
      return rows;
    }
    if (n === "WebFetch") {
      const u = s("url"); if (u) rows.push({ label: "url", value: u, mono: true });
      const pr = s("prompt"); if (pr) rows.push({ label: "ask", value: pr });
      return rows;
    }
    if (n === "WebSearch") {
      const q = s("query"); if (q) rows.push({ label: "query", value: q });
      return rows;
    }
    if (n === "Agent") {
      const sa = s("subagent_type"); if (sa) rows.push({ label: "agent", value: sa });
      const d = s("description"); if (d) rows.push({ label: "task", value: d });
      const pr = s("prompt"); if (pr) rows.push({ label: "prompt", value: pr });
      return rows;
    }
    if (n === "SlashCommand") {
      const c = s("command"); if (c) rows.push({ label: "command", value: c, mono: true });
      return rows;
    }
    if (n === "Skill") {
      const sk = s("skill"); if (sk) rows.push({ label: "skill", value: sk, mono: true });
      const a = s("args"); if (a) rows.push({ label: "args", value: a });
      return rows;
    }
    if (n === "TodoWrite") {
      const todos = Array.isArray(inp.todos) ? (inp.todos as Array<Record<string, unknown>>) : [];
      const text = todos.map((t) => {
        const mark = t.status === "completed" ? "✓" : t.status === "in_progress" ? "▸" : "·";
        const c = typeof t.content === "string" ? t.content : "";
        return `${mark} ${c}`;
      }).join("\n");
      if (text) rows.push({ label: "tasks", value: text, mono: true });
      return rows;
    }
    if (n === "BashOutput") {
      const id = s("bash_id"); if (id) rows.push({ label: "shell", value: id, mono: true });
      return rows;
    }
    if (n === "KillBash" || n === "KillShell") {
      const id = s("shell_id"); if (id) rows.push({ label: "shell", value: id, mono: true });
      return rows;
    }
    return null;
  });

  // Result rendering style. Drives the visual treatment of the result block.
  const resultStyle = $derived.by<"terminal" | "code" | "list" | "plain">(() => {
    const n = shortName(tool.name);
    if (n === "Bash" || n === "remote_bash" || n === "BashOutput") return "terminal";
    if (n === "Read" || n === "read_file") return "code";
    if (n === "Grep" || n === "grep" || n === "Glob" || n === "list_dir") return "list";
    return "plain";
  });

  // Error text — the one result shape still rendered by a local <pre> (the
  // shared OutputBlock/ReadResult/GrepResult bodies handle every success shape).
  const ERROR_CHAR_CAP = 20000;
  const errorText = $derived.by(() => {
    if (!tool.result) return "";
    const s = stripAnsi(tool.result);
    return s.length > ERROR_CHAR_CAP ? s.slice(0, ERROR_CHAR_CAP) + "\n… truncated" : s;
  });

  // Structured-body inputs (ReadResult gutter offset, GrepResult highlight).
  const readPath = $derived(
    typeof tool.input?.file_path === "string" ? (tool.input.file_path as string)
    : typeof tool.input?.path === "string" ? (tool.input.path as string) : null,
  );
  const readOffset = $derived(typeof tool.input?.offset === "number" ? (tool.input.offset as number) : null);
  const grepPattern = $derived(typeof tool.input?.pattern === "string" ? (tool.input.pattern as string) : null);

  // Copy affordance in the shared header — shell copies as a session snippet
  // ($ cmd + output); everything else copies the (ANSI-stripped) result.
  const copyText = $derived.by<(() => string) | null>(() => {
    if (tool.status === "pending" || !tool.result) return null;
    if (resultStyle === "terminal" && shellCommand) {
      return () => `$ ${shellCommand}\n${stripAnsi(tool.result ?? "")}`;
    }
    return () => stripAnsi(tool.result ?? "");
  });

  // ── Shell terminal — badge + prompt prefix + per-line tone ──────────────
  // Default bash; flip to pwsh only on strong PowerShell signals so a normal
  // bash line is never mislabeled. The prompt prefix ($ / PS>) is drawn in CSS.
  const shellCommand = $derived(typeof tool.input?.command === "string" ? (tool.input.command as string) : null);
  const shellKind = $derived.by<"bash" | "pwsh">(() => {
    const c = shellCommand ?? "";
    if (/(^|[\s|;(])(Get|Set|New|Start|Stop|Invoke|Remove|Add|Out|Where|ForEach|Select|Write|Import|Export)-[A-Z]/.test(c)
        || /\$env:|\$PSVersionTable|-ErrorAction|\bOut-Null\b|\bWrite-Host\b/.test(c)) return "pwsh";
    return "bash";
  });
  // (Line-tone classification + ANSI color live in the shared OutputBlock now —
  // streamModel.ts::classifyShellLine/ansiLines, one implementation for both trees.)

  // ── Agent card data ──────────────────────────────────────────────────
  const agentSubtype = $derived(
    typeof tool.input?.subagent_type === "string"
      ? (tool.input.subagent_type as string)
      : "general",
  );
  const agentDescription = $derived(
    typeof tool.input?.description === "string"
      ? (tool.input.description as string)
      : null,
  );
  const agentPrompt = $derived(
    typeof tool.input?.prompt === "string" ? (tool.input.prompt as string) : null,
  );

  // tool.result for an Agent block is model-controlled subagent output with no
  // size bound — a runaway/hostile agent returning megabytes would freeze the UI
  // on the synchronous marked.parse + DOMPurify pass. Cap before render.
  const AGENT_RESULT_CAP = 20000;
  const agentResult = $derived.by(() => {
    const r = tool.result;
    if (!r) return null;
    return r.length > AGENT_RESULT_CAP
      ? { text: r.slice(0, AGENT_RESULT_CAP), truncated: r.length - AGENT_RESULT_CAP }
      : { text: r, truncated: 0 };
  });

  // ── TodoWrite checklist data ─────────────────────────────────────────
  type TodoItem = { content: string; status: "pending" | "in_progress" | "completed" };
  const todoItems = $derived.by<TodoItem[]>(() => {
    const raw = tool.input?.todos;
    if (!Array.isArray(raw)) return [];
    return (raw as Array<Record<string, unknown>>)
      .map((t) => ({
        content: typeof t.content === "string" ? t.content : "",
        status:
          t.status === "completed" || t.status === "in_progress" || t.status === "pending"
            ? (t.status as TodoItem["status"])
            : "pending",
      }))
      .filter((t) => t.content.length > 0);
  });
  const todoCounts = $derived.by(() => {
    let done = 0, active = 0, pend = 0;
    for (const t of todoItems) {
      if (t.status === "completed") done++;
      else if (t.status === "in_progress") active++;
      else pend++;
    }
    return { done, active, pend, total: todoItems.length };
  });
</script>

<div class="chip" data-status={tool.status} data-category={category} data-variant={variant} class:as-card={isCard} class:is-ask={isAskUser}>
  {#if isAgent}
    <AgentCard
      {agentSubtype}
      {agentDescription}
      {agentPrompt}
      {agentResult}
      isError={tool.isError ?? false}
      status={tool.status}
      {durationLabel}
    />
  {:else if isTodoWrite}
    <TodoCard {todoItems} {todoCounts} status={tool.status} />
  {:else if isAskUser}
    <AskUserCard {tool} {expanded} />
  {:else}
    <BlockHeader
      expandable={true}
      {expanded}
      onToggle={() => (expanded = !expanded)}
      pill={tool.status === "pending" ? { text: "running…", tone: "running" } : tool.status === "error" ? { text: "failed", tone: "bad" } : null}
      {durationLabel}
      {copyText}
    >
      {#snippet lead()}
        <span class="chip-icon"><Icon size={12} /></span>
      {/snippet}
      {#snippet title()}
        {#if variant === "timeline" && caption}
          <span class="chip-cap">{caption}</span>
        {:else}
          <span class="chip-tool">{toolLabel}</span>
          <span class="chip-sep">·</span>
          <span class="chip-sum mono">{summary}</span>
        {/if}
      {/snippet}
      {#snippet meta()}
        {#if !expanded && inlinePreview}
          <span class="chip-preview mono" use:tooltip={tool.result ?? ""}>{inlinePreview}</span>
        {/if}
      {/snippet}
    </BlockHeader>
  {/if}

  {#if expanded && !isCard}
    <div class="chip-body" transition:slide={{ duration: reducedMotion ? 0 : 180 }}>
      {#if inputRows}
        {#if inputRows.length > 0}
          <dl class="kv">
            {#each inputRows as row (row.label)}
              <dt>{row.label}</dt>
              <dd class:mono={row.mono}>{row.value}</dd>
            {/each}
          </dl>
        {/if}
      {:else}
        <div class="field-label">input</div>
        <pre class="field-block">{JSON.stringify(tool.input, null, 2)}</pre>
      {/if}

      <div class="field-label">{tool.isError ? "error" : "result"}</div>
      {#if !tool.result}
        <div class="result-pending">
          <Loader2 size={11} class="chip-spin" />
          <span>running…</span>
        </div>
      {:else if tool.isError}
        <pre class="result error">{errorText}</pre>
      {:else if resultStyle === "terminal"}
        <div class="body-frame" data-shell={shellKind}>
          {#if shellCommand}
            <div class="terminal-cmd">
              <span class="term-badge" data-shell={shellKind}>{shellKind === "pwsh" ? "PowerShell" : "bash"}</span>
              <span class="term-cmd-text mono">{shellCommand}</span>
            </div>
          {/if}
          <OutputBlock text={tool.result} tone="shell" fold="head-tail" start="collapsed" />
        </div>
      {:else if resultStyle === "code"}
        <div class="body-frame"><ReadResult text={tool.result} path={readPath} offset={readOffset} /></div>
      {:else if resultStyle === "list" && shortName(tool.name) !== "list_dir"}
        <div class="body-frame"><GrepResult text={tool.result} pattern={grepPattern} bare={shortName(tool.name) === "Glob"} /></div>
      {:else}
        <div class="body-frame"><OutputBlock text={tool.result} tone="plain" start="collapsed" /></div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .chip {
    align-self: stretch;
    width: 100%;
    margin: 2px 0;
    background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    border-radius: 5px;
    overflow: hidden;
    transition: border-color var(--dur-fast) ease-out, background var(--dur-fast) ease-out, opacity var(--dur-fast) ease-out;
    animation: enter var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1);
  }
  .chip[data-status="done"] { opacity: 0.85; }
  .chip[data-status="done"]:hover { opacity: 1; }

  /* Timeline variant — naked head; the bullet on the rail (drawn by the
     parent .tl-node) carries the category + status signal. Card variants
     (Agent + TodoWrite) keep their chrome regardless. */
  .chip[data-variant="timeline"]:not(.as-card) {
    background: transparent;
    border: 0;
    border-left: 0;
    border-radius: 4px;
    margin: 0;
    animation: none;
  }
  /* Persistent faint surface + left-accent so each tool row reads as its own
     "action" lane, distinct from the plain narration prose above/below it.
     Previously transparent-until-hover, which left tool calls visually flat
     against the text. Hover brightens the lane + lifts the accent to the
     model hue. */
  .chip[data-variant="timeline"]:not(.as-card) :global(.bh) {
    padding: 3px 8px;
    min-height: 22px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--bg-elev-1) 45%, transparent);
    /* No resting left-bar — the node circle on the rail is the spine now, so
       the chip's own inset accent only competes with it. Hover still lifts the
       model-hue bar to signal interactivity. */
    transition: background var(--dur-fast) ease-out, transform var(--dur-fast) ease-out, box-shadow var(--dur-fast) ease-out;
  }
  .chip[data-variant="timeline"]:not(.as-card) :global(.bh:hover:not(:disabled)) {
    background: color-mix(in oklch, var(--surface-hover) 80%, transparent);
    box-shadow: inset 2px 0 0 color-mix(in oklch, var(--border) 90%, transparent);
    transform: translateX(1px);
  }
  /* Chip-tool name gets a bit more weight; summary stays muted for hierarchy. */
  .chip[data-variant="timeline"]:not(.as-card) .chip-tool {
    color: var(--fg);
    font-weight: 600;
  }
  .chip[data-variant="timeline"]:not(.as-card) .chip-sum { color: var(--fg-muted); }
  .chip[data-variant="timeline"]:not(.as-card)[data-status="error"] .chip-tool { color: oklch(0.85 0.13 22); }
  .chip[data-variant="timeline"]:not(.as-card) .chip-body {
    margin-top: 4px;
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    border-radius: 8px;
    background: color-mix(in oklch, var(--bg-elev-1) 80%, transparent);
  }
  .chip[data-variant="timeline"]:not(.as-card)[data-status="pending"] {
    background: transparent;
    animation: none;
  }
  .chip[data-variant="timeline"]:not(.as-card)[data-status="error"] {
    background: transparent;
  }
  .chip[data-status="pending"] {
    /* Neutral resting surface like every other block; a faint accent border is
       the only "running now" cue (status signal, not a card color). */
    background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent);
    border-color: color-mix(in oklab, var(--accent) 22%, var(--border));
    opacity: 1;
  }
  .chip[data-status="error"] {
    background: color-mix(in oklch, var(--danger-soft) 45%, var(--bg-elev-1));
    border-color: color-mix(in oklab, var(--danger) 38%, var(--border));
    opacity: 1;
  }
  /* Head chrome — layout + the meta cluster (pill/duration/copy/chevron) live
     in the shared BlockHeader; only the chip-specific container fill is here. */
  .chip :global(.bh) {
    gap: 5px;
    padding: 3px 8px;
    font-size: var(--fs-xs);
    min-height: 22px;
  }
  .chip :global(.bh:hover:not(:disabled)) { background: var(--surface-hover); }
  .chip-icon {
    display: inline-flex;
    color: var(--fg-2);
    flex-shrink: 0;
    opacity: 0.85;
  }
  /* Neutral left rule — calm/boxless, matching the stream blocks. Accent is
     reserved for the live (pending) and error states below, which are real
     signals; idle categories don't claim the model hue. */
  .chip { border-left-width: 2px; }
  .chip[data-category="read"],
  .chip[data-category="write"],
  .chip[data-category="shell"],
  .chip[data-category="agent"],
  .chip[data-category="meta"]  { border-left-color: color-mix(in oklch, var(--border) 80%, transparent); }
  .chip[data-status="error"] .chip-icon { color: var(--danger); opacity: 1; }
  .chip[data-status="error"] { border-left-color: var(--danger) !important; }
  .chip[data-status="pending"] {
    border-left-color: var(--accent) !important;
    animation: chip-pulse 1.8s ease-in-out infinite;
  }
  @keyframes chip-pulse {
    0%, 100% { background: color-mix(in oklch, var(--accent-soft) 45%, var(--bg-elev-1)); }
    50%      { background: color-mix(in oklch, var(--accent-soft) 25%, var(--bg-elev-1)); }
  }
  .chip-tool {
    font-weight: 600;
    color: var(--fg-2);
    flex-shrink: 0;
    font-size: var(--fs-xs);
    letter-spacing: 0.01em;
  }
  .chip-cap {
    flex: 1; min-width: 0;
    color: var(--fg);
    font-weight: 500;
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .chip[data-status="error"] .chip-cap { color: oklch(0.85 0.10 22); }
  .chip-sep { color: var(--fg-faint); flex-shrink: 0; font-size: 10px; }
  .chip-sum {
    flex: 1; min-width: 0;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .chip-preview {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 28ch;
  }
  /* (Duration pill + status affordance render in BlockHeader's meta cluster.) */
  @keyframes chip-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }

  .chip-body {
    padding: 8px 10px 10px;
    border-top: 1px solid var(--border);
    background: color-mix(in oklch, var(--surface) 96%, transparent);
  }
  /* Section labels (input / result) — lowercase to match the kv keys below
     them so the body reads as one consistent label family instead of mixing
     lowercase keys with UPPERCASE section heads. */
  .field-label {
    font-size: 9.5px;
    letter-spacing: 0.04em;
    color: var(--fg-muted);
    margin: 8px 0 4px;
    font-weight: 600;
    text-transform: lowercase;
  }
  .field-label:first-child { margin-top: 0; }

  /* Key-value input rows — replaces JSON dump for known tool shapes. */
  .kv {
    display: grid;
    grid-template-columns: minmax(46px, max-content) 1fr;
    column-gap: 10px;
    row-gap: 3px;
    margin: 0 0 2px;
    font-size: var(--fs-xs);
    line-height: 1.45;
  }
  .kv dt {
    color: var(--fg-muted);
    font-weight: 500;
    text-align: right;
    padding-top: 1px;
    text-transform: lowercase;
    letter-spacing: 0.04em;
    font-size: 9.5px;
  }
  .kv dd {
    margin: 0;
    color: var(--fg-2);
    word-wrap: break-word;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .kv dd.mono {
    font-family: var(--font-mono, monospace);
    font-size: var(--fs-xs);
    color: var(--fg);
  }

  /* JSON fallback (unknown tool shapes). */
  .field-block {
    margin: 0;
    padding: 7px 10px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    font-family: var(--font-mono, monospace);
    font-size: var(--fs-xs);
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 280px;
    overflow: auto;
    color: var(--fg-2);
  }

  /* Result — pending placeholder. */
  .result-pending {
    display: inline-flex; align-items: center; gap: 6px;
    color: var(--fg-muted);
    font-size: 11px;
    padding: 4px 2px;
  }
  .result-pending :global(.chip-spin) {
    color: var(--accent);
    animation: chip-spin 1s linear infinite;
  }

  /* Result body frame — ONE neutral-gray surface for every result shape
     (terminal via OutputBlock, code via ReadResult, matches via GrepResult,
     plain via OutputBlock), so history bodies and the live stream blocks all
     read as one family. */
  .body-frame {
    background: color-mix(in oklab, var(--fg) 2%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    overflow: hidden;
    position: relative;
    box-shadow: none;
    animation: tool-rise var(--dur-rise) var(--ease-page) both;
  }
  @keyframes tool-rise {
    from { opacity: 0; transform: translateY(5px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) { .body-frame { animation: none; } }
  /* Command head — shell badge + the $/PS>-prefixed command. */
  .terminal-cmd {
    display: flex; align-items: center; gap: 10px;
    padding: 9px 13px;
    background: color-mix(in oklch, var(--bg-elev-1) 28%, transparent);
    backdrop-filter: blur(8px) saturate(120%);
    -webkit-backdrop-filter: blur(8px) saturate(120%);
    border-bottom: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
  }
  .term-badge {
    display: inline-flex; align-items: center; gap: 6px;
    font-family: var(--font-mono, monospace);
    font-size: 9.5px; font-weight: 600;
    padding: 2px 9px; border-radius: 999px; flex-shrink: 0;
    border: 1px solid transparent;
  }
  .term-badge::before {
    content: ""; width: 5px; height: 5px; border-radius: 50%; background: currentColor;
    box-shadow: 0 0 6px currentColor; opacity: 0.9;
  }
  .term-badge[data-shell="bash"] { background: color-mix(in oklch, var(--fg) 6%, transparent); color: var(--fg-muted); border-color: color-mix(in oklch, var(--fg) 12%, transparent); }
  .term-badge[data-shell="pwsh"] { background: var(--info-soft); color: var(--info); border-color: color-mix(in oklch, var(--info) 30%, transparent); }
  .term-cmd-text {
    flex: 1; min-width: 0;
    font-size: 11px; color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .body-frame[data-shell="bash"] .term-cmd-text::before { content: "$ "; color: var(--fg-faint); }
  .body-frame[data-shell="pwsh"] .term-cmd-text::before { content: "PS> "; color: var(--info); }

  /* Result — error text (the one shape still rendered locally). */
  .result {
    margin: 0;
    padding: 7px 10px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    font-family: var(--font-mono, monospace);
    font-size: var(--fs-xs);
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 280px;
    overflow: auto;
    color: var(--fg-2);
  }
  .result.error {
    border-color: color-mix(in oklab, var(--danger) 35%, var(--border));
    color: oklch(0.88 0.07 22);
    background: color-mix(in oklch, var(--danger-soft) 25%, var(--bg-elev-1));
  }

  .mono { font-family: var(--font-mono, monospace); }

  /* ── Card variants (Agent + TodoWrite) ─────────────────────────────── */
  .chip.as-card {
    max-width: 100%;
    width: 100%;
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
    border-radius: 8px;
    border-width: 1px;
    border-left-width: 3px;
  }
  .chip.as-card[data-category="agent"] {
    border-left-color: color-mix(in oklab, var(--accent) 65%, var(--border));
  }
  .chip.as-card[data-category="meta"] {
    border-left-color: color-mix(in oklab, var(--accent) 55%, var(--border));
  }

  /* AskUser card OUTER FRAME — the head/body + .ask-* element styling live in
     ./toolchip/AskUserCard.svelte. These rules style the parent .chip wrapper
     (border / max-width / pending pulse); the crossings into the child head use
     :global() since .ask-head/.ask-status-text are now scoped to the child. */
  .chip.as-card.is-ask {
    --ask: var(--accent);
    max-width: min(100%, 560px);
    border-left-color: color-mix(in oklch, var(--ask) 55%, var(--border)) !important;
  }
  .chip.as-card.is-ask :global(.ask-head) {
    background: color-mix(in oklch, var(--ask) 8%, transparent);
  }

  /* Pending — gentle model-tinted outer glow. Signals "needs your input"
     using the assistant's own accent, not a semantic-green alarm. */
  .chip.as-card.is-ask[data-status="pending"] {
    animation: ask-card-pulse 2.4s ease-in-out infinite;
    background: color-mix(in oklch, var(--ask) 7%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--ask) 40%, var(--border));
    border-left-color: var(--ask) !important;
  }
  .chip.as-card.is-ask[data-status="pending"] :global(.ask-head) {
    background: color-mix(in oklch, var(--ask) 12%, transparent);
    border-bottom-color: color-mix(in oklch, var(--ask) 35%, var(--border));
  }
  .chip.as-card.is-ask[data-status="pending"] :global(.ask-status-text.awaiting) {
    color: color-mix(in oklch, var(--ask) 85%, white);
    font-weight: 600;
  }
  @keyframes ask-card-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklch, var(--ask) 0%, transparent); }
    50%      { box-shadow: 0 0 16px -2px color-mix(in oklch, var(--ask) 22%, transparent),
                          0 0 0 1px color-mix(in oklch, var(--ask) 16%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) {
    .chip.as-card.is-ask[data-status="pending"] { animation: none; }
  }
</style>
