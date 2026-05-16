<script lang="ts">
  import {
    ListChecks, Circle, CircleDot, CheckCircle2, Activity, Loader2, X,
    FileText, FolderTree, Search, ChevronRight, AlertCircle,
    FilePen, FilePlus, Terminal, Globe, Wrench,
  } from "lucide-svelte";
  import { assistant, type ToolBlock } from "../../state/assistant.svelte";

  const counts = $derived.by(() => {
    const total = assistant.tasks.length;
    const done = assistant.tasks.filter((t) => t.status === "completed").length;
    return { total, done };
  });

  const showCost = $derived(
    assistant.totalCostUsd !== null && assistant.auth?.apiKeyConfigured,
  );

  // Flatten every tool block across all assistant messages, oldest first.
  const allTools = $derived.by(() => {
    const out: ToolBlock[] = [];
    for (const m of assistant.messages) {
      if (m.role !== "assistant") continue;
      for (const b of m.blocks) if (b.type === "tool") out.push(b);
    }
    return out;
  });
  const totalOps = $derived(allTools.length);

  let expanded = $state<Record<string, boolean>>({});
  function toggleCard(id: string) {
    expanded = { ...expanded, [id]: !expanded[id] };
  }
  function shortName(name: string) { return name.replace(/^mcp__rift__/, ""); }
  function trim(s: string, n = 80): string {
    return s.length > n ? s.slice(0, n - 1) + "…" : s;
  }
  function hostOf(u: string): string {
    try { return new URL(u).host; } catch { return u; }
  }
  function summarize(t: ToolBlock): string {
    const n = shortName(t.name);
    const inp = t.input ?? {};
    // Claude Code built-ins (PascalCase) + Rift MCP variants (snake_case).
    if (n === "Read" || n === "read_file") return typeof inp.file_path === "string" ? (inp.file_path as string) : (typeof inp.path === "string" ? (inp.path as string) : "file");
    if (n === "Write") return typeof inp.file_path === "string" ? `${inp.file_path} (${(inp.content as string ?? "").length} chars)` : "file";
    if (n === "Edit") {
      const p = typeof inp.file_path === "string" ? inp.file_path as string : "file";
      const old = typeof inp.old_string === "string" ? (inp.old_string as string).split("\n")[0].slice(0, 32) : "";
      return old ? `${p} — ${old}` : p;
    }
    if (n === "Bash") return typeof inp.command === "string" ? trim(inp.command as string, 90) : "shell";
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
    if (n === "WebFetch") return typeof inp.url === "string" ? hostOf(inp.url as string) : "url";
    if (n === "WebSearch") return typeof inp.query === "string" ? `"${trim(inp.query as string, 60)}"` : "search";
    if (n === "list_dir") return typeof inp.path === "string" ? (inp.path as string) : "directory";
    if (n === "TodoWrite") {
      const todos = Array.isArray(inp.todos) ? (inp.todos as Array<{content?: string}>).length : 0;
      return `${todos} task${todos === 1 ? "" : "s"}`;
    }
    return n;
  }

  // Live elapsed timer — ticks every second while streaming.
  let nowTick = $state(Date.now());
  $effect(() => {
    if (!assistant.streaming) return;
    const id = setInterval(() => (nowTick = Date.now()), 500);
    return () => clearInterval(id);
  });
  const elapsed = $derived.by(() => {
    const started = assistant.activity.turnStartedAt;
    if (!started) return null;
    const ms = (assistant.streaming ? nowTick : Date.now()) - started;
    void nowTick;
    const s = Math.floor(ms / 1000);
    return s < 60 ? `${s}s` : `${Math.floor(s / 60)}m ${s % 60}s`;
  });
</script>

<aside class="dock">
  <header class="dock-head">
    <div class="title">
      <ListChecks size={13} />
      <span>Tasks</span>
      {#if counts.total > 0}
        <span class="counter">{counts.done}/{counts.total}</span>
      {/if}
    </div>
    <button class="closebtn" type="button" onclick={() => (assistant.ui.dockOpen = false)} title="Close dock">
      <X size={13} />
    </button>
  </header>

  <div class="section">
    {#if assistant.tasks.length === 0}
      <div class="empty-note">
        Nothing yet — multi-step plans show up here as Claude works.
      </div>
    {:else}
      <ul class="tasklist">
        {#each assistant.tasks as t (t.id)}
          <li class="task" data-status={t.status}>
            <span class="status-icon">
              {#if t.status === "completed"}
                <CheckCircle2 size={13} />
              {:else if t.status === "in_progress"}
                <CircleDot size={13} />
              {:else}
                <Circle size={13} />
              {/if}
            </span>
            <span class="task-text">{t.content}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if totalOps > 0 || assistant.streaming}
    <div class="divider"></div>
    <div class="section activity">
      <div class="section-title">
        <Activity size={13} />
        <span>Activity</span>
        {#if totalOps > 0}<span class="counter">{totalOps}</span>{/if}
        {#if elapsed}<span class="elapsed">{elapsed}</span>{/if}
      </div>

      {#if assistant.activity.currentLabel}
        <div class="now-row">
          <Loader2 size={11} class="spin" />
          <span class="now-text">{assistant.activity.currentLabel}</span>
        </div>
      {/if}

      {#if allTools.length > 0}
        <div class="op-cards">
          {#each allTools as t (t.id)}
            {@const sn = shortName(t.name)}
            {@const isOpen = expanded[t.id] === true}
            <div class="op-card" data-status={t.status}>
              <button class="op-head" type="button" onclick={() => toggleCard(t.id)} aria-expanded={isOpen}>
                <span class="op-chev" class:open={isOpen}><ChevronRight size={11} /></span>
                <span class="op-icon">
                  {#if sn === "Read" || sn === "read_file"}<FileText size={11} />
                  {:else if sn === "list_dir"}<FolderTree size={11} />
                  {:else if sn === "Grep" || sn === "grep"}<Search size={11} />
                  {:else if sn === "Glob"}<FolderTree size={11} />
                  {:else if sn === "Write"}<FilePlus size={11} />
                  {:else if sn === "Edit"}<FilePen size={11} />
                  {:else if sn === "Bash"}<Terminal size={11} />
                  {:else if sn === "WebFetch" || sn === "WebSearch"}<Globe size={11} />
                  {:else if sn === "TodoWrite"}<ListChecks size={11} />
                  {:else}<Wrench size={11} />{/if}
                </span>
                <span class="op-name-row">
                  <span class="op-tool">{sn}</span>
                  <span class="op-sum">{summarize(t)}</span>
                </span>
                <span class="op-status">
                  {#if t.status === "pending"}<Loader2 size={10} class="spin" />
                  {:else if t.status === "error"}<AlertCircle size={10} />
                  {:else}<CheckCircle2 size={10} />{/if}
                </span>
              </button>
              {#if isOpen}
                <div class="op-body">
                  <div class="op-field-label">input</div>
                  <pre class="op-block">{JSON.stringify(t.input, null, 2)}</pre>
                  <div class="op-field-label">{t.isError ? "error" : "result"}</div>
                  <pre class="op-block" class:error={t.isError}>{t.result ?? "(running…)"}</pre>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if showCost}
        <div class="cost-row">
          <span class="cost-label">Last cost</span>
          <span class="cost-val mono">${assistant.totalCostUsd!.toFixed(4)}</span>
        </div>
      {/if}
    </div>
  {/if}
</aside>

<style>
  .dock {
    width: 280px;
    flex-shrink: 0;
    display: flex; flex-direction: column;
    background: var(--bg-elev-1);
    border-left: 1px solid var(--border);
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    box-sizing: border-box;
  }
  /* Custom scrollbar — match Rift's dark palette, hide horizontal artifacts. */
  .dock::-webkit-scrollbar { width: 8px; height: 0; }
  .dock::-webkit-scrollbar-track { background: transparent; }
  .dock::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 4px;
  }
  .dock::-webkit-scrollbar-thumb:hover { background: var(--fg-faint); }
  .dock-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .title {
    display: flex; align-items: center; gap: 6px;
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg);
  }
  .counter {
    font-size: 10px;
    padding: 1px 6px;
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: 999px;
    margin-left: 2px;
  }
  .closebtn {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
  }
  .closebtn:hover { color: var(--fg); background: var(--surface-hover); }

  .section { padding: 10px 12px; }
  .empty-note {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    line-height: 1.5;
    padding: 8px 4px;
  }

  .tasklist {
    list-style: none; margin: 0; padding: 0;
    display: flex; flex-direction: column; gap: 4px;
  }
  .task {
    display: flex; align-items: flex-start; gap: 8px;
    padding: 7px 8px;
    border-radius: 6px;
    font-size: var(--fs-sm);
    color: var(--fg-2);
    transition: background 120ms ease-out;
  }
  .task:hover { background: var(--surface-hover); }
  .status-icon {
    display: flex; align-items: center;
    color: var(--fg-subtle);
    margin-top: 1px;
  }
  .task[data-status="in_progress"] .status-icon { color: var(--accent); }
  .task[data-status="completed"] .status-icon { color: var(--accent); }
  .task[data-status="completed"] .task-text {
    color: var(--fg-subtle);
    text-decoration: line-through;
  }
  .task-text { line-height: 1.4; word-break: break-word; }

  .divider {
    height: 1px;
    margin: 4px 12px;
    background: var(--border);
  }

  .activity { padding-bottom: 14px; }
  .section-title {
    display: flex; align-items: center; gap: 6px;
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg);
    padding: 0 0 8px;
  }
  .elapsed {
    margin-left: auto;
    font-size: var(--fs-xs);
    font-weight: 500;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }

  .now-row {
    display: flex; align-items: center; gap: 7px;
    padding: 6px 8px;
    margin-bottom: 8px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklch, var(--accent) 22%, transparent);
    border-radius: 6px;
    color: var(--accent);
    font-size: var(--fs-xs);
  }
  .now-text {
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    font-family: var(--font-mono, monospace);
    color: var(--fg);
  }
  .now-row :global(.spin) { animation: dock-spin 1s linear infinite; flex-shrink: 0; }
  @keyframes dock-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }

  .op-cards {
    display: flex; flex-direction: column; gap: 4px;
  }
  .op-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    transition: border-color 140ms ease-out;
  }
  .op-card[data-status="pending"] {
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  .op-card[data-status="error"] {
    border-color: color-mix(in oklch, var(--danger) 40%, var(--border));
  }
  .op-head {
    display: flex; align-items: center; gap: 5px;
    width: 100%;
    padding: 5px 7px;
    background: transparent;
    border: 0;
    color: var(--fg);
    cursor: pointer;
    font: inherit;
    font-size: 11px;
    text-align: left;
    min-height: 26px;
  }
  .op-head:hover { background: var(--surface-hover); }
  .op-chev {
    display: inline-flex;
    color: var(--fg-faint);
    transition: transform 140ms ease-out;
    flex-shrink: 0;
  }
  .op-chev.open { transform: rotate(90deg); }
  .op-icon { display: inline-flex; color: var(--accent); flex-shrink: 0; }
  .op-name-row {
    flex: 1;
    min-width: 0;
    display: flex; flex-direction: column; gap: 1px;
  }
  .op-tool {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    color: var(--fg-muted);
    letter-spacing: 0.02em;
  }
  .op-sum {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    color: var(--fg-2);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .op-status { display: inline-flex; flex-shrink: 0; }
  .op-card[data-status="pending"] .op-status { color: var(--accent); }
  .op-card[data-status="error"] .op-status { color: var(--danger); }
  .op-card[data-status="done"] .op-status { color: var(--ok); }
  .op-status :global(.spin) { animation: dock-spin 1s linear infinite; }

  .op-body {
    padding: 6px 8px 8px;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }
  .op-field-label {
    font-size: 9px; text-transform: uppercase; letter-spacing: 0.08em;
    color: var(--fg-muted);
    margin: 4px 0 3px;
  }
  .op-field-label:first-child { margin-top: 0; }
  .op-block {
    margin: 0;
    padding: 5px 7px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    line-height: 1.4;
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 200px;
    overflow: auto;
    color: var(--fg-2);
  }
  .op-block.error {
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    color: oklch(0.88 0.07 22);
  }

  .cost-row {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px dashed var(--border);
    font-size: var(--fs-xs);
  }
  .cost-label { color: var(--fg-muted); }
  .cost-val { color: var(--fg-2); }
  .mono { font-family: var(--font-mono, monospace); }
</style>
