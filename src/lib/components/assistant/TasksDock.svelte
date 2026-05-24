<script lang="ts">
  // Session panel — multi-section right rail. Three sections derived from
  // the active tab's message stream:
  //   1. Tasks   — TodoWrite-emitted plan (existing behavior)
  //   2. Outputs — files Claude wrote/edited this convo (Edit/Write/MultiEdit/NotebookEdit)
  //   3. Sources — URLs fetched + queries searched (WebFetch/WebSearch)
  // Each section hides when empty. If all three empty, an "empty plan" hint
  // shows in place of all bodies. Filename stays TasksDock.svelte for import
  // stability — semantic name is now Session panel.
  import { onMount } from "svelte";
  import {
    Circle, CircleDot, CheckCircle2,
    ListChecks, FileText, Globe, Search, ExternalLink,
  } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import type { Block, ChatMessage } from "../../state/assistant.svelte";

  let { tabId = null }: { tabId?: string | null } = $props();

  const tab = $derived(tabId == null ? assistant.activeTab : assistant.tabFor(tabId));
  const tasks = $derived(tab?.tasks ?? []);
  const messages = $derived<ChatMessage[]>(tab?.messages ?? []);

  type OutputItem = { path: string; tool: string };
  type SourceItem = { kind: "url" | "query"; value: string };

  const sessionContext = $derived.by(() => {
    const outMap = new Map<string, OutputItem>();
    const srcMap = new Map<string, SourceItem>();
    for (const m of messages) {
      for (const b of m.blocks as Block[]) {
        if (b.type !== "tool") continue;
        const input = b.input ?? {};
        if (b.name === "Edit" || b.name === "Write" || b.name === "MultiEdit") {
          const fp = input.file_path;
          if (typeof fp === "string" && fp.length > 0) outMap.set(fp, { path: fp, tool: b.name });
        } else if (b.name === "NotebookEdit") {
          const fp = input.notebook_path;
          if (typeof fp === "string" && fp.length > 0) outMap.set(fp, { path: fp, tool: b.name });
        } else if (b.name === "WebFetch") {
          const url = input.url;
          if (typeof url === "string" && url.length > 0) srcMap.set(url, { kind: "url", value: url });
        } else if (b.name === "WebSearch") {
          const q = input.query;
          if (typeof q === "string" && q.length > 0) srcMap.set(`q:${q}`, { kind: "query", value: q });
        }
      }
    }
    // Insertion order is preserved by Map; last-touched stays at end (most recent first
    // is more useful — reverse so newest leads).
    return {
      outputs: Array.from(outMap.values()).reverse(),
      sources: Array.from(srcMap.values()).reverse(),
    };
  });

  const outputs = $derived(sessionContext.outputs);
  const sources = $derived(sessionContext.sources);

  const taskCounts = $derived.by(() => {
    const total = tasks.length;
    const done = tasks.filter((t) => t.status === "completed").length;
    const active = tasks.filter((t) => t.status === "in_progress").length;
    return { total, done, active };
  });
  const taskPct = $derived(taskCounts.total > 0 ? (taskCounts.done / taskCounts.total) * 100 : 0);

  const allEmpty = $derived(
    tasks.length === 0 && outputs.length === 0 && sources.length === 0,
  );

  // Lazy-loaded opener — same pattern as ConflictResolver / UpdateDialog so
  // we don't pay the import cost on initial dock mount.
  let opener: typeof import("@tauri-apps/plugin-opener") | null = null;
  onMount(async () => {
    try { opener = await import("@tauri-apps/plugin-opener"); }
    catch { /* dev shim — clicks silently no-op until tauri context is live */ }
  });

  function basename(p: string): string {
    const m = p.match(/[^\\/]+$/);
    return m ? m[0] : p;
  }
  function hostnameOrSelf(u: string): string {
    try { return new URL(u).hostname.replace(/^www\./, ""); }
    catch { return u; }
  }

  async function openOutput(path: string) {
    if (!opener) return;
    try { await opener.openPath(path); }
    catch (e) { console.warn("[SessionDock] openPath failed", path, e); }
  }
  async function openSource(item: SourceItem) {
    if (!opener) return;
    try {
      if (item.kind === "url") await opener.openUrl(item.value);
      // Queries have no canonical URL — clicking copies the query text instead.
      else if (item.kind === "query") await navigator.clipboard?.writeText(item.value);
    } catch (e) { console.warn("[SessionDock] open source failed", item, e); }
  }
</script>

<aside class="dock">
  <!-- Tasks ──────────────────────────────────────────────────────────── -->
  {#if tasks.length > 0}
    <section class="sect">
      <header class="sect-head">
        <ListChecks size={12} />
        <span class="sect-title">Tasks</span>
        <span class="counter mono" title="{taskCounts.done} done · {taskCounts.active} in progress · {taskCounts.total} total">
          {taskCounts.done}<span class="counter-sep">/</span>{taskCounts.total}
        </span>
      </header>
      <div class="progress" role="progressbar" aria-valuenow={taskCounts.done} aria-valuemax={taskCounts.total}>
        <div class="progress-fill" style="width: {taskPct}%"></div>
        {#if taskCounts.active > 0}
          <div class="progress-active" style="left: {taskPct}%; width: {Math.max(4, 100 / taskCounts.total)}%"></div>
        {/if}
      </div>
      <ul class="rows">
        {#each tasks as t (t.id)}
          <li class="row task" data-status={t.status}>
            <span class="row-icon">
              {#if t.status === "completed"}
                <CheckCircle2 size={13} />
              {:else if t.status === "in_progress"}
                <CircleDot size={13} />
              {:else}
                <Circle size={13} />
              {/if}
            </span>
            <span class="row-text">{t.content}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <!-- Outputs ────────────────────────────────────────────────────────── -->
  {#if outputs.length > 0}
    <section class="sect">
      <header class="sect-head">
        <FileText size={12} />
        <span class="sect-title">Outputs</span>
        <span class="counter mono" title="{outputs.length} file{outputs.length === 1 ? '' : 's'} touched">{outputs.length}</span>
      </header>
      <ul class="rows">
        {#each outputs as o (o.path)}
          <li class="row file">
            <button
              type="button"
              class="row-btn"
              onclick={() => openOutput(o.path)}
              title={o.path}
            >
              <span class="row-icon"><FileText size={12} /></span>
              <span class="row-text">{basename(o.path)}</span>
              <span class="row-aft" aria-hidden="true"><ExternalLink size={10} /></span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <!-- Sources ────────────────────────────────────────────────────────── -->
  {#if sources.length > 0}
    <section class="sect">
      <header class="sect-head">
        <Globe size={12} />
        <span class="sect-title">Sources</span>
        <span class="counter mono" title="{sources.length} web reference{sources.length === 1 ? '' : 's'}">{sources.length}</span>
      </header>
      <ul class="rows">
        {#each sources as s, i (s.kind + ':' + s.value + ':' + i)}
          <li class="row source">
            <button
              type="button"
              class="row-btn"
              onclick={() => openSource(s)}
              title={s.kind === "url" ? s.value : `Search: ${s.value}\n(click to copy)`}
            >
              <span class="row-icon">
                {#if s.kind === "url"}<Globe size={12} />{:else}<Search size={12} />{/if}
              </span>
              <span class="row-text">
                {s.kind === "url" ? hostnameOrSelf(s.value) : s.value}
              </span>
              <span class="row-aft" aria-hidden="true"><ExternalLink size={10} /></span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <!-- Empty fallback ─────────────────────────────────────────────────── -->
  {#if allEmpty}
    <div class="empty-note">
      <div class="empty-title">Session panel</div>
      Plans, files Claude touches, and web sources will collect here as the
      conversation runs.
    </div>
  {/if}
</aside>

<style>
  .dock {
    width: 100%;
    flex: 1;
    display: flex; flex-direction: column;
    background: transparent;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    box-sizing: border-box;
  }
  .dock::-webkit-scrollbar { width: 8px; height: 0; }
  .dock::-webkit-scrollbar-track { background: transparent; }
  .dock::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 4px;
  }
  .dock::-webkit-scrollbar-thumb:hover { background: var(--fg-faint); }

  .sect {
    display: flex; flex-direction: column;
    border-bottom: 1px solid var(--border);
  }
  .sect:last-of-type { border-bottom: none; }

  .sect-head {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 14px 8px;
    color: var(--fg);
    font-size: var(--fs-sm);
    font-weight: 600;
    flex-shrink: 0;
  }
  .sect-head :global(svg) { color: var(--accent); flex-shrink: 0; }
  .sect-title { color: var(--fg); }

  .counter {
    margin-left: auto;
    font-size: 10px;
    padding: 2px 7px;
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: 999px;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .counter-sep { opacity: 0.55; margin: 0 1px; }

  /* Tasks-only progress bar — unchanged from v1 dock, scoped under .sect now. */
  .progress {
    position: relative;
    height: 2px;
    background: var(--bg-elev-2);
    overflow: hidden;
    flex-shrink: 0;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 280ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .progress-active {
    position: absolute;
    top: 0; bottom: 0;
    background: color-mix(in oklch, var(--accent) 60%, transparent);
    animation: progress-active-pulse 1.4s ease-in-out infinite;
  }
  @keyframes progress-active-pulse {
    0%, 100% { opacity: 0.45; }
    50%      { opacity: 0.95; }
  }
  @media (prefers-reduced-motion: reduce) {
    .progress-active { animation: none; opacity: 0.65; }
  }

  .rows {
    list-style: none; margin: 0;
    padding: 6px 8px 12px;
    display: flex; flex-direction: column; gap: 2px;
  }

  /* Static (no hover/click) rows — tasks. */
  .row.task {
    display: flex; align-items: flex-start; gap: 8px;
    padding: 7px 8px;
    border-radius: 6px;
    font-size: var(--fs-sm);
    color: var(--fg-2);
    transition: background 120ms ease-out;
  }
  .row.task:hover { background: var(--surface-hover); }
  .row.task .row-icon {
    display: flex; align-items: center;
    color: var(--fg-subtle);
    margin-top: 1px;
  }
  .row.task[data-status="in_progress"] .row-icon { color: var(--accent); }
  .row.task[data-status="completed"] .row-icon { color: var(--accent); }
  .row.task[data-status="completed"] .row-text {
    color: var(--fg-subtle);
    text-decoration: line-through;
  }
  .row-text { line-height: 1.4; word-break: break-word; flex: 1; min-width: 0; }

  /* Clickable rows — outputs + sources. Buttons reset to look like li. */
  .row.file, .row.source { padding: 0; }
  .row-btn {
    display: flex; align-items: center; gap: 8px;
    width: 100%;
    padding: 7px 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--fg-2);
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out;
  }
  .row-btn:hover {
    background: var(--surface-hover);
    border-color: var(--border);
    color: var(--fg);
  }
  .row-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .row-btn .row-icon {
    display: flex; align-items: center;
    color: var(--fg-subtle);
    flex-shrink: 0;
  }
  .row-btn:hover .row-icon { color: var(--accent); }
  .row-btn .row-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-aft {
    display: inline-flex; align-items: center;
    color: var(--fg-faint);
    opacity: 0;
    transition: opacity 120ms ease-out;
    flex-shrink: 0;
  }
  .row-btn:hover .row-aft, .row-btn:focus-visible .row-aft { opacity: 0.7; }

  .empty-note {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    line-height: 1.55;
    padding: 14px 16px;
  }
  .empty-title {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg-2);
    margin-bottom: 4px;
  }

  .mono { font-family: var(--font-mono, monospace); }
</style>
