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
  } from "lucide-svelte";
  import type { ToolBlock } from "../../state/assistant.svelte";

  let { tool }: { tool: ToolBlock } = $props();
  let expanded = $state(false);

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
</script>

<div class="chip" data-status={tool.status}>
  <button class="chip-head" type="button" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
    <span class="chip-chev" class:open={expanded}><ChevronRight size={11} /></span>
    <span class="chip-icon"><Icon size={12} /></span>
    <span class="chip-tool">{toolLabel}</span>
    <span class="chip-sep">·</span>
    <span class="chip-sum mono">{summary}</span>
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

  {#if expanded}
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
    animation: chip-in 200ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes chip-in {
    from { opacity: 0; transform: translateY(3px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .chip[data-status="done"] { opacity: 0.85; }
  .chip[data-status="done"]:hover { opacity: 1; }
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
  .chip[data-status="error"] .chip-icon { color: var(--danger); opacity: 1; }
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
</style>
