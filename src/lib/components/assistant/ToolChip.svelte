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
    Wrench, Loader2, CheckCircle2, AlertCircle, ChevronRight, ListChecks,
    Bot, HelpCircle, FlagOff, BookOpen, Sparkles, Slash, Square, SkipForward,
    GitBranch, GitCommitHorizontal, ExternalLink, Bell,
  } from "lucide-svelte";
  import { assistant, type ToolBlock } from "../../state/assistant.svelte";
  import { slide } from "svelte/transition";
  import { untrack } from "svelte";
  import Markdown from "./Markdown.svelte";
  import { basename } from "./toolCaption";
  import AgentCard from "./toolchip/AgentCard.svelte";
  import TodoCard from "./toolchip/TodoCard.svelte";
  import AskUserCard from "./toolchip/AskUserCard.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
  let { tool, variant = "card", caption = null }: { tool: ToolBlock; variant?: "card" | "timeline"; caption?: string | null } = $props();
  // Agent + TodoWrite + AskUser are first-class card variants — default-expanded
  // since their body IS the message, not a debug detail. All other tools collapse.
  const isAgent = $derived(/^(mcp__rift__)?Agent$/.test(tool.name));
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
    if (n === "Bash") return typeof inp.command === "string" ? trim(inp.command as string, 70) : "shell";
    if (n === "BashOutput") return typeof inp.bash_id === "string" ? `tail ${inp.bash_id}` : "tail bg shell";
    if (n === "KillBash" || n === "KillShell") return typeof inp.shell_id === "string" ? `kill ${inp.shell_id}` : "kill shell";
    if (n === "remote_bash") return typeof inp.command === "string" ? trim(inp.command as string, 70) : "remote shell";
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
    if (n === "SlashCommand") return typeof inp.command === "string" ? (inp.command as string) : "slash command";
    if (n === "Skill") return typeof inp.skill === "string" ? (inp.skill as string) : "skill";
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
    if (n === "Bash" || n === "remote_bash" || n === "BashOutput" || n === "KillBash" || n === "KillShell") return "shell";
    if (n === "Agent" || n === "Task" || n === "Skill" || n === "SlashCommand") return "agent";
    if (n === "TodoWrite" || n === "TaskCreate" || n === "TaskUpdate" || n === "AskUserQuestion" || n === "ask_user" || n === "ExitPlanMode") return "meta";
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
    return s < 10 ? `${s.toFixed(1)}s` : `${Math.round(s)}s`;
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
    if (sn === "Bash" || sn === "remote_bash") return Terminal;
    if (sn === "BashOutput") return SkipForward;
    if (sn === "KillBash" || sn === "KillShell") return Square;
    // Search / nav.
    if (sn === "list_dir" || sn === "Glob") return FolderTree;
    if (sn === "Grep" || sn === "grep") return Search;
    // Web.
    if (sn === "WebFetch" || sn === "WebSearch") return Globe;
    if (sn === "open_browser") return ExternalLink;
    if (sn === "notify") return Bell;
    // Local git (mcp__rift__git_*).
    if (sn === "git_commit") return GitCommitHorizontal;
    if (sn.startsWith("git_")) return GitBranch;
    // Agentic / planning / meta.
    if (sn === "Agent" || sn === "Task") return Bot;
    if (sn === "AskUserQuestion") return HelpCircle;
    if (sn === "ExitPlanMode") return FlagOff;
    if (sn === "SlashCommand") return Slash;
    if (sn === "Skill") return Sparkles;
    if (sn === "TodoWrite" || sn === "TaskCreate" || sn === "TaskUpdate") return ListChecks;
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
      const text = todos.map((t, i) => {
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

  const resultLines = $derived.by(() => {
    if (!tool.result) return null;
    const lines = tool.result.split("\n");
    if (lines.length <= 60) return { shown: lines, more: 0 };
    return { shown: lines.slice(0, 60), more: lines.length - 60 };
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
  // Conservative tone classifier — only strong, unambiguous signals get a
  // color; everything else stays neutral `out`. Runs only on terminal-style
  // results (Bash family), never on grep/list output.
  function classifyShellLine(line: string): "out" | "ok" | "err" | "warn" {
    if (/^\s*(✓|✔)/.test(line) || /\b0 (errors?|warnings?|issues?|problems?)\b/i.test(line)
        || /\b(passed|succeeded|success)\b/i.test(line)) return "ok";
    if (/^\s*(✗|✖|×)/.test(line) || /\b(error|errors|failed|fatal|panic|exception|traceback)\b/i.test(line)) return "err";
    if (/\b(warn|warning|warnings|deprecated)\b/i.test(line)) return "warn";
    return "out";
  }
  const termLines = $derived.by(() => {
    if (!resultLines) return [];
    return resultLines.shown.map((c, i) => ({ i, c, t: classifyShellLine(c) }));
  });

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
    <button class="chip-head" type="button" onclick={() => (expanded = !expanded)} aria-expanded={expanded} use:tooltip={expanded ? "Collapse" : "Expand details"}>
      <span class="chip-chev" class:open={expanded}><ChevronRight size={11} /></span>
      <span class="chip-icon"><Icon size={12} /></span>
      {#if variant === "timeline" && caption}
        <span class="chip-cap">{caption}</span>
      {:else}
        <span class="chip-tool">{toolLabel}</span>
        <span class="chip-sep">·</span>
        <span class="chip-sum mono">{summary}</span>
      {/if}
      {#if !expanded && inlinePreview}
        <span class="chip-preview mono" use:tooltip={tool.result ?? ""}>{inlinePreview}</span>
      {/if}
      {#if durationLabel}
        <span class="chip-duration mono" use:tooltip={"Wall-clock duration"}>{durationLabel}</span>
      {/if}
      <span class="chip-status">
        {#if tool.status === "pending"}
          <Loader2 size={11} class="chip-spin" />
        {:else if tool.status === "error"}
          <AlertCircle size={11} />
        {:else}
          <CheckCircle2 size={11} />
        {/if}
      </span>
    </button>
  {/if}

  {#if expanded}
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
      {#if !resultLines}
        <div class="result-pending">
          <Loader2 size={11} class="chip-spin" />
          <span>running…</span>
        </div>
      {:else if tool.isError}
        <pre class="result error">{resultLines.shown.join("\n")}{#if resultLines.more}
… +{resultLines.more} more lines{/if}</pre>
      {:else if resultStyle === "terminal"}
        <div class="terminal" data-shell={shellKind}>
          {#if shellCommand}
            <div class="terminal-cmd">
              <span class="term-badge" data-shell={shellKind}>{shellKind === "pwsh" ? "PowerShell" : "bash"}</span>
              <span class="term-cmd-text mono">{shellCommand}</span>
            </div>
          {/if}
          <div class="terminal-out">
            {#if termLines.length === 0}<div class="term-line out">(no output)</div>{/if}
            {#each termLines as tl (tl.i)}
              <div class="term-line {tl.t}">{tl.c}</div>
            {/each}
          </div>
          {#if resultLines.more}<div class="more">+{resultLines.more} more lines</div>{/if}
        </div>
      {:else if resultStyle === "code"}
        <pre class="result code">{resultLines.shown.join("\n")}{#if resultLines.more}
… +{resultLines.more} more lines{/if}</pre>
      {:else if resultStyle === "list"}
        <ul class="list">
          {#each resultLines.shown as line, i (i)}
            <li>{line}</li>
          {/each}
          {#if resultLines.more}<li class="more-line">+{resultLines.more} more</li>{/if}
        </ul>
      {:else}
        <pre class="result">{resultLines.shown.join("\n")}{#if resultLines.more}
… +{resultLines.more} more lines{/if}</pre>
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
    transition: border-color 140ms ease-out, background 140ms ease-out, opacity 140ms ease-out;
    animation: enter 200ms cubic-bezier(0.22, 1, 0.36, 1);
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
  .chip[data-variant="timeline"]:not(.as-card) .chip-head {
    padding: 3px 8px;
    min-height: 22px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--bg-elev-1) 45%, transparent);
    /* No resting left-bar — the node circle on the rail is the spine now, so
       the chip's own inset accent only competes with it. Hover still lifts the
       model-hue bar to signal interactivity. */
    transition: background 140ms ease-out, transform 140ms ease-out, box-shadow 140ms ease-out;
  }
  .chip[data-variant="timeline"]:not(.as-card) .chip-head:hover {
    background: color-mix(in oklch, var(--surface-hover) 80%, transparent);
    box-shadow: inset 2px 0 0 color-mix(in oklch, var(--border) 90%, transparent);
    transform: translateX(1px);
  }
  /* Rail-bullet already shows status — kill the redundant right-edge pill
     in timeline variant. Duration badge stays (slowness signal). */
  .chip[data-variant="timeline"]:not(.as-card) .chip-status { display: none; }
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
  .chip-head {
    display: flex; align-items: center; gap: 5px;
    width: 100%;
    padding: 3px 8px;
    background: transparent;
    border: 0;
    color: var(--fg-2);
    cursor: pointer;
    font: inherit;
    font-size: 11px;
    text-align: left;
    min-height: 22px;
  }
  .chip-head:hover { background: var(--surface-hover); }
  .chip-chev {
    display: inline-flex;
    color: var(--fg-muted);
    transition: transform 140ms ease-out, color 140ms ease-out;
    flex-shrink: 0;
  }
  .chip-chev.open { transform: rotate(90deg); }
  /* ui-audit #12: the chevron IS the expand affordance — let hover light it
     up so chips read as openable, not static log lines. */
  .chip-head:hover .chip-chev { color: var(--accent); }
  .chip-head:hover .chip-chev:not(.open) { transform: translateX(2px); }
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
    font-size: 10.5px;
    letter-spacing: 0.01em;
  }
  .chip-cap {
    flex: 1; min-width: 0;
    color: var(--fg);
    font-weight: 500;
    font-size: 11px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .chip[data-status="error"] .chip-cap { color: oklch(0.85 0.10 22); }
  .chip-sep { color: var(--fg-faint); flex-shrink: 0; font-size: 10px; }
  .chip-sum {
    flex: 1; min-width: 0;
    color: var(--fg-muted);
    font-size: 10.5px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .chip-preview {
    color: var(--fg-subtle);
    font-size: 10.5px;
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 28ch;
  }
  /* Neutral duration residue — informational, not a warning. Was warn-orange,
     which made a normal 1.2s Read look like an alert. Quiet muted pill now;
     genuine slowness reads from the number itself, not an alarm color. */
  .chip-duration {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    color: var(--fg-muted);
    border: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    font-weight: 600;
  }
  .chip-status {
    display: inline-flex;
    flex-shrink: 0;
  }
  .chip[data-status="pending"] .chip-status { color: var(--accent); }
  .chip[data-status="error"] .chip-status { color: var(--danger); }
  .chip[data-status="done"] .chip-status { color: var(--ok); }
  .chip-status :global(.chip-spin) { animation: chip-spin 1s linear infinite; }
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
    font-size: 11px;
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
    font-size: 10.5px;
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
    font-size: 10.5px;
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

  /* Result — terminal style (Bash, remote_bash, BashOutput). Neutral-gray
     surface shared with every other block (no accent tint), so terminals,
     reads, edits and the live stream blocks all read as one family. */
  .terminal {
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
  @media (prefers-reduced-motion: reduce) { .terminal { animation: none; } }
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
  .terminal[data-shell="bash"] .term-cmd-text::before { content: "$ "; color: var(--fg-faint); }
  .terminal[data-shell="pwsh"] .term-cmd-text::before { content: "PS> "; color: var(--info); }
  .terminal-out {
    margin: 0;
    padding: 8px 10px;
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    line-height: 1.6;
    color: oklch(0.88 0.025 130);
    max-height: 320px;
    overflow: auto;
  }
  .term-line {
    white-space: pre-wrap;
    word-break: break-word;
  }
  .term-line.out  { color: oklch(0.88 0.025 130); }
  .term-line.ok   { color: var(--ok); }
  .term-line.err  { color: var(--danger); }
  .term-line.warn { color: var(--warn); }
  .terminal .more {
    padding: 4px 11px;
    font-size: 10px;
    color: var(--fg-muted);
    border-top: 1px solid color-mix(in oklch, var(--border) 40%, transparent);
    background: color-mix(in oklab, var(--fg) 4%, transparent);
    font-style: italic;
  }

  /* Result — code style (Read). Neutral-gray surface, same family as the
     terminal + every other block (accent tint removed). */
  .result.code {
    background: color-mix(in oklab, var(--fg) 2%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    padding: 9px 13px;
    max-height: 320px;
    box-shadow: none;
    animation: tool-rise var(--dur-rise) var(--ease-page) both;
  }

  /* Result — list style (Grep, Glob, list_dir). Neutral-gray, same family. */
  .list {
    list-style: none;
    margin: 0;
    padding: 5px 0;
    background: color-mix(in oklab, var(--fg) 2%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    max-height: 320px;
    overflow: auto;
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    line-height: 1.55;
    box-shadow: none;
    animation: tool-rise var(--dur-rise) var(--ease-page) both;
  }
  @media (prefers-reduced-motion: reduce) {
    .result.code, .list { animation: none; }
  }
  .list li {
    padding: 1px 10px;
    color: var(--fg-2);
    white-space: pre-wrap;
    word-wrap: break-word;
  }
  .list li:hover { background: var(--surface-hover); color: var(--fg); }
  .list .more-line {
    margin-top: 3px;
    border-top: 1px solid color-mix(in oklch, var(--border) 40%, transparent);
    padding-top: 4px;
    color: var(--fg-muted);
    font-style: italic;
  }
  .list .more-line:hover { background: transparent; color: var(--fg-muted); }

  /* Result — plain text fallback. */
  .result {
    margin: 0;
    padding: 7px 10px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
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
