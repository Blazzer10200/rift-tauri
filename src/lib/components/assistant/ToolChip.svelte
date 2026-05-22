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
    Circle,
  } from "lucide-svelte";
  import type { ToolBlock } from "../../state/assistant.svelte";
  import Markdown from "./Markdown.svelte";

  let { tool, variant = "card" }: { tool: ToolBlock; variant?: "card" | "timeline" } = $props();
  // Agent + TodoWrite are first-class card variants — default-expanded since
  // their body IS the message, not a debug detail. All other tools collapse.
  const isAgent = $derived(/^(mcp__rift__)?Agent$/.test(tool.name));
  const isTodoWrite = $derived(/^(mcp__rift__)?TodoWrite$/.test(tool.name));
  const isCard = $derived(isAgent || isTodoWrite);
  let expanded = $state(false);
  // Cards open by default; chips closed.
  $effect(() => { if (isCard) expanded = true; });

  function shortName(name: string): string { return name.replace(/^mcp__rift__/, ""); }
  function trim(s: string, n = 60): string {
    return s.length > n ? s.slice(0, n - 1) + "…" : s;
  }
  function hostOf(u: string): string {
    try { return new URL(u).host; } catch { return u; }
  }
  function basenameOf(p: string): string {
    const parts = p.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? p;
  }

  // Compact 1-line summary — file path, command, pattern, etc. Full BUILTINS
  // coverage so every tool name in `--allowed-tools` (assistant/mod.rs) has
  // a dedicated chip rendering instead of falling through to the generic
  // tool-name default.
  const summary = $derived.by<string>(() => {
    const n = shortName(tool.name);
    const inp = tool.input ?? {};
    const fp = typeof inp.file_path === "string" ? basenameOf(inp.file_path as string)
             : typeof inp.path === "string" ? basenameOf(inp.path as string)
             : typeof inp.notebook_path === "string" ? basenameOf(inp.notebook_path as string) : null;
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
    if (n === "TodoWrite" || n === "AskUserQuestion" || n === "ExitPlanMode") return "meta";
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
    // Agentic / planning / meta.
    if (sn === "Agent") return Bot;
    if (sn === "AskUserQuestion") return HelpCircle;
    if (sn === "ExitPlanMode") return FlagOff;
    if (sn === "SlashCommand") return Slash;
    if (sn === "Skill") return Sparkles;
    if (sn === "TodoWrite") return ListChecks;
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
      const cmd = s("command"); if (cmd) rows.push({ label: "command", value: cmd, mono: true });
      const desc = s("description"); if (desc) rows.push({ label: "purpose", value: desc });
      const to = num("timeout"); if (to) rows.push({ label: "timeout", value: `${to} ms`, mono: true });
      return rows;
    }
    if (n === "Read" || n === "read_file") {
      const fp = s("file_path") ?? s("path"); if (fp) rows.push({ label: "file", value: fp, mono: true });
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

<div class="chip" data-status={tool.status} data-category={category} data-variant={variant} class:as-card={isCard}>
  {#if isAgent}
    <!-- Agent card head — bigger, badge-led, no chevron toggle (always open). -->
    <div class="agent-head">
      <span class="agent-icon"><Bot size={14} /></span>
      <span class="agent-pill">{agentSubtype}</span>
      {#if agentDescription}
        <span class="agent-desc">{agentDescription}</span>
      {/if}
      {#if durationLabel}
        <span class="chip-duration mono" title="Wall-clock duration">{durationLabel}</span>
      {/if}
      <span class="chip-status">
        {#if tool.status === "pending"}<Loader2 size={12} class="chip-spin" />
        {:else if tool.status === "error"}<AlertCircle size={12} />
        {:else}<CheckCircle2 size={12} />{/if}
      </span>
    </div>
  {:else if isTodoWrite}
    <!-- TodoWrite card head — counts summary + status. -->
    <div class="todo-head">
      <span class="agent-icon"><ListChecks size={13} /></span>
      <span class="todo-title">Tasks</span>
      <span class="todo-counts mono">
        <span class="todo-count done" title="completed">{todoCounts.done}</span>
        <span class="todo-sep">/</span>
        <span class="todo-count total" title="total">{todoCounts.total}</span>
      </span>
      <span class="chip-status">
        {#if tool.status === "pending"}<Loader2 size={11} class="chip-spin" />
        {:else if tool.status === "error"}<AlertCircle size={11} />
        {:else}<CheckCircle2 size={11} />{/if}
      </span>
    </div>
  {:else}
    <button class="chip-head" type="button" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
      <span class="chip-chev" class:open={expanded}><ChevronRight size={11} /></span>
      <span class="chip-icon"><Icon size={12} /></span>
      <span class="chip-tool">{toolLabel}</span>
      <span class="chip-sep">·</span>
      <span class="chip-sum mono">{summary}</span>
      {#if !expanded && inlinePreview}
        <span class="chip-arrow" aria-hidden="true">→</span>
        <span class="chip-preview mono" title={tool.result ?? ""}>{inlinePreview}</span>
      {/if}
      {#if durationLabel}
        <span class="chip-duration mono" title="Wall-clock duration">{durationLabel}</span>
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

  {#if isAgent && expanded}
    <div class="agent-body">
      {#if agentPrompt}
        <div class="agent-prompt-wrap">
          <span class="agent-field-label">prompt</span>
          <blockquote class="agent-prompt">{agentPrompt}</blockquote>
        </div>
      {/if}
      <div class="agent-field-label">{tool.isError ? "error" : tool.status === "pending" ? "working…" : "result"}</div>
      {#if tool.status === "pending" && !tool.result}
        <div class="agent-pending">
          <span class="dots" aria-hidden="true"><span class="dot"></span><span class="dot"></span><span class="dot"></span></span>
          <span>Agent running…</span>
        </div>
      {:else if tool.isError}
        <pre class="result error">{tool.result ?? ""}</pre>
      {:else if tool.result}
        <div class="agent-result"><Markdown text={tool.result} /></div>
      {/if}
    </div>
  {:else if isTodoWrite && expanded}
    <div class="todo-body">
      <ul class="todo-list">
        {#each todoItems as item, i (i + item.content)}
          <li class="todo-item" data-status={item.status}>
            <span class="todo-box" aria-hidden="true">
              {#if item.status === "completed"}<CheckCircle2 size={12} />
              {:else if item.status === "in_progress"}<Loader2 size={12} class="todo-spin" />
              {:else}<Circle size={12} />{/if}
            </span>
            <span class="todo-content">{item.content}</span>
          </li>
        {/each}
      </ul>
    </div>
  {:else if expanded}
    <div class="chip-body">
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
        <div class="terminal">
          <pre class="terminal-out">{resultLines.shown.join("\n") || "(no output)"}</pre>
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
    align-self: flex-start;
    max-width: min(100%, 78ch);
    width: fit-content;
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
  .chip[data-variant="timeline"]:not(.as-card) .chip-head {
    padding: 2px 6px 2px 3px;
    min-height: 20px;
    border-radius: 4px;
    transition: background 140ms ease-out, transform 140ms ease-out;
  }
  .chip[data-variant="timeline"]:not(.as-card) .chip-head:hover {
    background: color-mix(in oklch, var(--surface-hover) 70%, transparent);
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
    border-radius: 5px;
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
    background: color-mix(in oklch, var(--accent-soft) 45%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--accent) 28%, var(--border));
    opacity: 1;
  }
  .chip[data-status="error"] {
    background: color-mix(in oklch, var(--danger-soft) 45%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--danger) 38%, var(--border));
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
    color: var(--fg-faint);
    transition: transform 140ms ease-out;
    flex-shrink: 0;
  }
  .chip-chev.open { transform: rotate(90deg); }
  .chip-icon {
    display: inline-flex;
    color: var(--accent);
    flex-shrink: 0;
    opacity: 0.85;
  }
  /* Category coloring — distinguishes read-only/mutation/side-effect/agentic
   * at a glance. Tones picked from the existing palette + slight hue shifts
   * so the chips read as a family rather than 5 unrelated colors. */
  .chip[data-category="read"]  .chip-icon { color: oklch(0.74 0.10 230); }
  .chip[data-category="write"] .chip-icon { color: var(--accent); }
  .chip[data-category="shell"] .chip-icon { color: oklch(0.78 0.12 60); }
  .chip[data-category="agent"] .chip-icon { color: oklch(0.74 0.13 310); }
  .chip[data-category="meta"]  .chip-icon { color: oklch(0.76 0.10 145); }
  /* Subtle left-edge accent so a wall of chips has visual scannability. */
  .chip { border-left-width: 2px; }
  .chip[data-category="read"]  { border-left-color: color-mix(in oklch, oklch(0.74 0.10 230) 45%, var(--border)); }
  .chip[data-category="write"] { border-left-color: color-mix(in oklch, var(--accent) 50%, var(--border)); }
  .chip[data-category="shell"] { border-left-color: color-mix(in oklch, oklch(0.78 0.12 60) 50%, var(--border)); }
  .chip[data-category="agent"] { border-left-color: color-mix(in oklch, oklch(0.74 0.13 310) 50%, var(--border)); }
  .chip[data-category="meta"]  { border-left-color: color-mix(in oklch, oklch(0.76 0.10 145) 45%, var(--border)); }
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
  .chip-sep { color: var(--fg-faint); flex-shrink: 0; font-size: 10px; }
  .chip-sum {
    flex: 1; min-width: 0;
    color: var(--fg-muted);
    font-size: 10.5px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .chip-arrow {
    color: var(--fg-faint);
    font-size: 10px;
    flex-shrink: 0;
    margin: 0 1px;
  }
  .chip-preview {
    color: var(--fg-2);
    font-size: 10.5px;
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 36ch;
    opacity: 0.85;
  }
  .chip-duration {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--warn-soft) 55%, var(--bg-elev-1));
    color: var(--warn);
    border: 1px solid color-mix(in oklch, var(--warn) 25%, var(--border));
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
  .field-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-muted);
    margin: 8px 0 4px;
    font-weight: 600;
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
    letter-spacing: 0.02em;
    font-size: 10px;
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
    padding: 6px 8px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: 4px;
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

  /* Result — terminal style (Bash, remote_bash, BashOutput). */
  .terminal {
    background: oklch(0.16 0.012 250);
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    border-radius: 5px;
    overflow: hidden;
  }
  .terminal-out {
    margin: 0;
    padding: 8px 10px;
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    line-height: 1.55;
    color: oklch(0.88 0.025 130);
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 320px;
    overflow: auto;
  }
  .terminal .more {
    padding: 4px 10px;
    font-size: 10px;
    color: var(--fg-muted);
    border-top: 1px solid color-mix(in oklch, var(--border) 40%, transparent);
    background: oklch(0.18 0.012 250);
    font-style: italic;
  }

  /* Result — code style (Read). */
  .result.code {
    background: var(--bg-elev-1);
    border-left: 2px solid color-mix(in oklch, var(--accent) 35%, transparent);
    padding: 6px 10px;
    max-height: 320px;
  }

  /* Result — list style (Grep, Glob, list_dir). */
  .list {
    list-style: none;
    margin: 0;
    padding: 4px 0;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: 4px;
    max-height: 320px;
    overflow: auto;
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    line-height: 1.55;
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
    padding: 6px 8px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: 4px;
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
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    color: oklch(0.88 0.07 22);
    background: color-mix(in oklch, var(--danger-soft) 25%, var(--bg-elev-1));
  }

  .mono { font-family: var(--font-mono, monospace); }

  /* ── Card variants (Agent + TodoWrite) ─────────────────────────────── */
  .chip.as-card {
    max-width: min(100%, 78ch);
    width: 100%;
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
    border-radius: 8px;
    border-width: 1px;
    border-left-width: 3px;
  }
  .chip.as-card[data-category="agent"] {
    border-left-color: color-mix(in oklch, oklch(0.74 0.13 310) 65%, var(--border));
  }
  .chip.as-card[data-category="meta"] {
    border-left-color: color-mix(in oklch, oklch(0.76 0.10 145) 55%, var(--border));
  }

  /* Agent head */
  .agent-head {
    display: flex; align-items: center; gap: 9px;
    padding: 8px 12px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    background: color-mix(in oklch, oklch(0.74 0.13 310) 8%, transparent);
  }
  .agent-icon {
    display: inline-flex;
    color: oklch(0.78 0.14 310);
    flex-shrink: 0;
  }
  .agent-pill {
    display: inline-flex; align-items: center;
    padding: 2px 9px;
    border-radius: 999px;
    background: color-mix(in oklch, oklch(0.74 0.13 310) 22%, transparent);
    border: 1px solid color-mix(in oklch, oklch(0.74 0.13 310) 40%, var(--border));
    color: oklch(0.84 0.10 310);
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    font-family: var(--font-mono, monospace);
    flex-shrink: 0;
  }
  .agent-desc {
    flex: 1; min-width: 0;
    color: var(--fg-2);
    font-size: 12px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .agent-body {
    padding: 10px 14px 12px;
    display: flex; flex-direction: column;
    gap: 8px;
  }
  .agent-field-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-muted);
    font-weight: 600;
  }
  .agent-prompt-wrap { display: flex; flex-direction: column; gap: 4px; }
  .agent-prompt {
    margin: 0;
    padding: 8px 12px;
    border-left: 2px solid color-mix(in oklch, oklch(0.74 0.13 310) 50%, transparent);
    background: color-mix(in oklch, var(--bg-elev-1) 80%, transparent);
    color: var(--fg-2);
    font-size: 11.5px;
    line-height: 1.5;
    font-style: italic;
    border-radius: 0 4px 4px 0;
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 180px;
    overflow-y: auto;
  }
  .agent-result {
    padding: 4px 0;
    font-size: 12.5px;
    line-height: 1.55;
  }
  .agent-pending {
    display: inline-flex; align-items: center; gap: 8px;
    color: var(--fg-muted);
    font-size: 11.5px;
    font-style: italic;
  }
  .agent-pending .dots { display: inline-flex; gap: 3px; }
  .agent-pending .dot {
    width: 5px; height: 5px;
    border-radius: 50%;
    background: oklch(0.74 0.13 310);
    animation: agent-dot 1.1s ease-in-out infinite;
  }
  .agent-pending .dot:nth-child(2) { animation-delay: 0.15s; }
  .agent-pending .dot:nth-child(3) { animation-delay: 0.3s; }
  @keyframes agent-dot {
    0%, 60%, 100% { opacity: 0.3; transform: scale(0.85); }
    30% { opacity: 1; transform: scale(1); }
  }

  /* Todo head + checklist */
  .todo-head {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 11px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    background: color-mix(in oklch, oklch(0.76 0.10 145) 6%, transparent);
  }
  .todo-head .agent-icon { color: oklch(0.76 0.13 145); }
  .todo-title {
    color: var(--fg-2);
    font-weight: 600;
    font-size: 11px;
    letter-spacing: 0.01em;
  }
  .todo-counts {
    display: inline-flex; align-items: center; gap: 3px;
    margin-left: auto;
    margin-right: 4px;
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }
  .todo-count.done { color: oklch(0.78 0.14 145); font-weight: 600; }
  .todo-count.total { color: var(--fg-muted); }
  .todo-sep { color: var(--fg-faint); }
  .todo-body { padding: 8px 12px 10px; }
  .todo-list {
    list-style: none;
    margin: 0; padding: 0;
    display: flex; flex-direction: column;
    gap: 3px;
  }
  .todo-item {
    display: grid;
    grid-template-columns: 18px 1fr;
    align-items: start;
    gap: 8px;
    padding: 3px 4px;
    border-radius: 4px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--fg-2);
    /* Transition (not animation) — safe with mutateStreaming rebuild. */
    transition: color 200ms ease-out, opacity 200ms ease-out;
  }
  .todo-box {
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--fg-faint);
    padding-top: 1px;
    transition: color 200ms ease-out;
  }
  .todo-content {
    word-wrap: break-word;
    transition: color 200ms ease-out, text-decoration-color 200ms ease-out;
  }
  .todo-item[data-status="completed"] .todo-box { color: oklch(0.78 0.14 145); }
  .todo-item[data-status="completed"] .todo-content {
    color: var(--fg-muted);
    text-decoration: line-through;
    text-decoration-color: color-mix(in oklch, var(--fg-muted) 60%, transparent);
  }
  .todo-item[data-status="in_progress"] .todo-box { color: var(--accent); }
  .todo-item[data-status="in_progress"] .todo-content { color: var(--fg); font-weight: 500; }
  .todo-item[data-status="pending"] .todo-content { color: var(--fg-2); }
  .todo-box :global(.todo-spin) {
    animation: chip-spin 1.1s linear infinite;
    color: var(--accent);
  }
</style>
