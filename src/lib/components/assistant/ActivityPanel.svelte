<script lang="ts">
  // Merged side panel — one scrolling surface (no more Session/Activity tabs).
  // Top-to-bottom it answers "what's happening now" then "what it produced"
  // then "session review":
  //   Now strip   — live turn state + elapsed (while streaming)
  //   Running     — every in-flight unit (tools / shells / agents / thinking)
  //   Tasks       — TodoWrite plan + progress
  //   Outputs     — files written/edited this convo
  //   Sources     — URLs fetched + queries searched
  //   Tool mix    — per-tool histogram (session review)
  //   Insights    — slowest / failed / cancelled
  // The redundant "This session" stat card was dropped — tok/s · tools · cost
  // already live in the status bar. Everything here is per-tab reactive state.
  import { onMount, onDestroy } from "svelte";
  import {
    Loader2, Terminal, Bot, AlertCircle, Wrench, Activity, Sparkles,
    FileText, Globe, Search, ExternalLink, ChevronDown,
    ListChecks, Circle, CircleDot, CheckCircle2,
  } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import type { Block, ChatMessage } from "../../state/assistant.svelte";
  import { liveActivity, loadCollapsedSections, saveCollapsedSections } from "../../state/assistant/helpers";
  import { tooltip } from "$lib/actions/tooltip";
  import { scrubUser } from "$lib/util/redact";

  let { tabId = null }: { tabId?: string | null } = $props();

  const tab = $derived(tabId == null ? assistant.activeTab : assistant.tabFor(tabId));
  const messages = $derived<ChatMessage[]>(tab?.messages ?? []);
  const streaming = $derived(tab?.streaming ?? false);

  // 1s ticker — drives live elapsed readouts. Mounts only while the panel is
  // open, so it costs nothing when hidden.
  let now = $state(Date.now());
  let ticker: ReturnType<typeof setInterval> | null = null;
  onMount(() => { ticker = setInterval(() => { now = Date.now(); }, 1000); });
  onDestroy(() => { if (ticker) clearInterval(ticker); });

  // Mount-time stamp standing in for legacy blocks missing a startedAt — pinned
  // so `running` recomputes only when messages / agentSpawns change, not every
  // tick (see CR4 history).
  const mountTs = Date.now();

  // ── Now: canonical live turn state (tab.activity) ──────────────────────
  const turnStartedAt = $derived(tab?.activity.turnStartedAt ?? null);
  const liveItems = $derived(liveActivity(messages, tab?.agentSpawns ?? [], mountTs));
  // Thinking is a turn state (owned by the Now strip), not a discrete work
  // unit — the Running list shows only tool/shell/agent calls.
  const isThinking = $derived(liveItems.some((r) => r.kind === "thinking"));
  const running = $derived(liveItems.filter((r) => r.kind !== "thinking"));
  // Headline label: friendly caption matching the transcript rail.
  const nowLabel = $derived.by(() => {
    if (running.length === 1) return running[0].label;
    if (running.length > 1) return `Running ${running.length} actions`;
    if (isThinking) return "Thinking…";
    return "Responding…";
  });

  // ── Tool rollup — counts, errors, slowest — per-tab, reactive ──────────
  const toolStats = $derived.by(() => {
    const counts: Record<string, number> = {};
    let total = 0, errors = 0, cancelled = 0;
    let slowest: { name: string; ms: number; id: string } | null = null;
    let lastFail: string | null = null;
    for (const m of messages) {
      for (const b of m.blocks as Block[]) {
        if (b.type !== "tool") continue;
        counts[b.name] = (counts[b.name] ?? 0) + 1;
        total += 1;
        const isCancelled = b.result != null && b.result.includes("Cancelled:");
        if (isCancelled) cancelled += 1;
        else if (b.isError || b.status === "error") { errors += 1; lastFail = b.name; }
        if (b.durationMs != null && (!slowest || b.durationMs > slowest.ms)) slowest = { name: b.name, ms: b.durationMs, id: b.id };
      }
    }
    const histo = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    const max = histo.length ? histo[0][1] : 1;
    return { counts, total, errors, cancelled, slowest, lastFail, histo, max };
  });

  // ── Session context — outputs (files touched) + sources (web) ──────────
  const sessionContext = $derived.by(() => {
    const outMap = new Map<string, { path: string; tool: string }>();
    const srcMap = new Map<string, { kind: "url" | "query"; value: string }>();
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
    return {
      outputs: Array.from(outMap.values()).reverse(),
      sources: Array.from(srcMap.values()).reverse(),
    };
  });
  const outputs = $derived(sessionContext.outputs);
  const sources = $derived(sessionContext.sources);

  // ── Tasks (TodoWrite plan) ─────────────────────────────────────────────
  const tasks = $derived(tab?.tasks ?? []);
  const taskCounts = $derived.by(() => {
    const total = tasks.length;
    const done = tasks.filter((t) => t.status === "completed").length;
    const active = tasks.filter((t) => t.status === "in_progress").length;
    return { total, done, active };
  });
  const taskPct = $derived(taskCounts.total > 0 ? (taskCounts.done / taskCounts.total) * 100 : 0);

  const isEmpty = $derived(
    !streaming && running.length === 0 && toolStats.total === 0 &&
    tasks.length === 0 && outputs.length === 0 && sources.length === 0,
  );

  // ── Live / review split ────────────────────────────────────────────────
  // Running + Tasks are "happening now" (always shown). Outputs / Sources /
  // Tool mix / Insights are "session review" — grouped under one collapsible
  // header so a long session doesn't push the live state off-screen.
  const hasReview = $derived(
    outputs.length > 0 || sources.length > 0 || toolStats.histo.length > 0 ||
    !!toolStats.slowest || toolStats.errors > 0 || toolStats.cancelled > 0,
  );
  let collapsed = $state(loadCollapsedSections());
  const reviewCollapsed = $derived(collapsed.has("review"));
  function toggleReview() {
    const next = new Set(collapsed);
    if (next.has("review")) next.delete("review");
    else next.add("review");
    collapsed = next;
    saveCollapsedSections(next);
  }

  // Tool-mix is capped at 6 rows; the footer toggles the full list.
  let toolsExpanded = $state(false);
  const TOOL_CAP = 6;
  const shownTools = $derived(
    toolsExpanded ? toolStats.histo : toolStats.histo.slice(0, TOOL_CAP),
  );

  // Lazy opener for output/source clicks (same pattern as the old dock).
  let opener: typeof import("@tauri-apps/plugin-opener") | null = null;
  onMount(async () => {
    try { opener = await import("@tauri-apps/plugin-opener"); }
    catch { /* dev shim — no-op until tauri context is live */ }
  });

  // Scroll the transcript to a tool block + briefly flash it.
  function jumpTo(blockId: string) {
    const el = document.getElementById(`actnode-${blockId}`);
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    el.classList.add("act-flash");
    setTimeout(() => el.classList.remove("act-flash"), 1100);
  }

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
    catch (e) { console.warn("[ActivityPanel] openPath failed", scrubUser(path), e); }
  }
  async function openSource(item: { kind: "url" | "query"; value: string }) {
    if (!opener) return;
    try {
      if (item.kind === "url") await opener.openUrl(item.value);
      else if (item.kind === "query") await navigator.clipboard?.writeText(item.value);
    } catch (e) { console.warn("[ActivityPanel] open source failed", item, e); }
  }

  function fmtElapsed(ms: number): string {
    const s = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }
  function fmtDur(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const s = ms / 1000;
    return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m ${String(Math.floor(s % 60)).padStart(2, "0")}s`;
  }
</script>

<div class="activity">
  <!-- Now strip — live turn headline + elapsed ───────────────────────────── -->
  {#if streaming}
    <div class="now" class:think={isThinking && running.length === 0}>
      {#if isThinking && running.length === 0}
        <Sparkles size={14} class="mon-pulse" />
      {:else}
        <Loader2 size={14} class="mon-spin" />
      {/if}
      <span class="now-label">{nowLabel}</span>
      {#if turnStartedAt != null}
        <span class="now-el mono">{fmtElapsed(now - turnStartedAt)}</span>
      {/if}
    </div>
  {/if}

  {#if isEmpty}
    <div class="empty-note">
      <div class="empty-title">Activity</div>
      Live work, the plan, files Claude touches, and web sources collect here as
      the conversation runs.
    </div>
  {:else}
    <!-- Running — every in-flight unit ───────────────────────────────────── -->
    {#if running.length > 0}
      <section class="sect">
        <header class="sect-head">
          <Activity size={12} />
          <span class="sect-title">Running</span>
          <span class="badge live"><span class="live-dot"></span>{running.length}</span>
        </header>
        <ul class="rows">
          {#each running as r (r.id)}
            <li>
              <button
                type="button"
                class="run"
                onclick={() => jumpTo(r.id)}
                use:tooltip={"Jump to this call in the transcript"}
              >
                <span class="run-ic">
                  {#if r.kind === "agent"}<Bot size={13} class="mon-pulse" />
                  {:else if r.kind === "shell"}<Terminal size={13} class="mon-pulse" />
                  {:else}<Wrench size={13} class="mon-pulse" />{/if}
                </span>
                <span class="run-label" class:mono={r.kind === "shell"}>
                  {#if r.sub}<span class="agtype">{r.sub}</span>{/if}<span class="run-t">{r.label}</span>
                </span>
                <span class="run-el mono">{fmtElapsed(now - r.startedAt)}</span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- Tasks — TodoWrite plan ───────────────────────────────────────────── -->
    {#if tasks.length > 0}
      <section class="sect">
        <header class="sect-head">
          <ListChecks size={12} />
          <span class="sect-title">Tasks</span>
          <span class="counter mono">{taskCounts.done}<span class="counter-sep">/</span>{taskCounts.total}</span>
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
                {#if t.status === "completed"}<CheckCircle2 size={13} />
                {:else if t.status === "in_progress"}<CircleDot size={13} />
                {:else}<Circle size={13} />{/if}
              </span>
              <span class="row-text">{t.content}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- Session review zone — folds Outputs / Sources / Tool mix / Insights ── -->
    {#if hasReview}
      <button type="button" class="zone-head" onclick={toggleReview} aria-expanded={!reviewCollapsed}>
        <span class="zone-line" aria-hidden="true"></span>
        <span class="zone-label">Session review</span>
        <ChevronDown size={13} class="zone-chev {reviewCollapsed ? '' : 'open'}" />
      </button>
    {/if}
    {#if hasReview && !reviewCollapsed}
    <!-- Outputs — files touched ──────────────────────────────────────────── -->
    {#if outputs.length > 0}
      <section class="sect">
        <header class="sect-head">
          <FileText size={12} />
          <span class="sect-title">Outputs</span>
          <span class="counter mono">{outputs.length}</span>
        </header>
        <ul class="rows">
          {#each outputs as o (o.path)}
            <li class="row file">
              <button type="button" class="row-btn" onclick={() => openOutput(o.path)} use:tooltip={o.path}>
                <span class="row-icon"><FileText size={12} /></span>
                <span class="row-text">{basename(o.path)}</span>
                <span class="row-aft" aria-hidden="true"><ExternalLink size={10} /></span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- Sources — web refs ───────────────────────────────────────────────── -->
    {#if sources.length > 0}
      <section class="sect">
        <header class="sect-head">
          <Globe size={12} />
          <span class="sect-title">Sources</span>
          <span class="counter mono">{sources.length}</span>
        </header>
        <ul class="rows">
          {#each sources as s, i (s.kind + ':' + s.value + ':' + i)}
            <li class="row source">
              <button type="button" class="row-btn" onclick={() => openSource(s)}
                use:tooltip={s.kind === "url" ? s.value : `Search: ${s.value}\n(click to copy)`}>
                <span class="row-icon">
                  {#if s.kind === "url"}<Globe size={12} />{:else}<Search size={12} />{/if}
                </span>
                <span class="row-text">{s.kind === "url" ? hostnameOrSelf(s.value) : s.value}</span>
                <span class="row-aft" aria-hidden="true"><ExternalLink size={10} /></span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- Tool mix — histogram (session review) ─────────────────────────────── -->
    {#if toolStats.histo.length > 0}
      <section class="sect">
        <header class="sect-head"><Wrench size={12} /><span class="sect-title">Tool mix</span></header>
        <div class="histo">
          {#each shownTools as [name, count] (name)}
            <div class="hrow" use:tooltip={name}>
              <span class="hname">{name}</span>
              <span class="hbar"><i style="width: {(count / toolStats.max) * 100}%"></i></span>
              <span class="hn mono">{count}</span>
            </div>
          {/each}
          {#if toolStats.histo.length > TOOL_CAP}
            <button type="button" class="hmore" onclick={() => (toolsExpanded = !toolsExpanded)}>
              {#if toolsExpanded}Show less{:else}+{toolStats.histo.length - TOOL_CAP} more tool{toolStats.histo.length - TOOL_CAP === 1 ? "" : "s"}{/if}
            </button>
          {/if}
        </div>
      </section>
    {/if}

    <!-- Insights ─────────────────────────────────────────────────────────── -->
    {#if toolStats.slowest || toolStats.errors > 0 || toolStats.cancelled > 0}
      <section class="sect insights">
        {#if toolStats.slowest}
          <button type="button" class="insight warn jump"
            onclick={() => { const s = toolStats.slowest; if (s) jumpTo(s.id); }}
            use:tooltip={"Jump to this call in the transcript"}>
            <span class="ic"><AlertCircle size={13} /></span>
            Slowest tool · <b>{toolStats.slowest.name}</b> <span class="mono">{fmtDur(toolStats.slowest.ms)}</span>
          </button>
        {/if}
        {#if toolStats.errors > 0}
          <div class="insight err">
            <span class="ic"><AlertCircle size={13} /></span>
            {toolStats.errors} failed call{toolStats.errors === 1 ? "" : "s"}{#if toolStats.lastFail} · <b>{toolStats.lastFail}</b>{/if}
          </div>
        {/if}
        {#if toolStats.cancelled > 0}
          <div class="insight">
            <span class="ic"><AlertCircle size={13} /></span>
            {toolStats.cancelled} cancelled <span class="dim">· aborted parallel calls</span>
          </div>
        {/if}
      </section>
    {/if}
    {/if}
  {/if}
</div>

<style>
  .activity {
    width: 100%; flex: 1;
    display: flex; flex-direction: column;
    min-height: 0; overflow-x: hidden; overflow-y: auto;
    box-sizing: border-box;
  }
  .activity::-webkit-scrollbar { width: 8px; height: 0; }
  .activity::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 4px; }
  .activity::-webkit-scrollbar-thumb:hover { background: var(--fg-faint); }

  /* Now strip */
  .now {
    display: flex; align-items: center; gap: 9px;
    padding: 11px 14px; flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    background: var(--accent-soft);
  }
  .now :global(svg) { color: var(--accent); flex-shrink: 0; }
  .now-label {
    flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--fs-sm); font-weight: 600; color: var(--fg);
  }
  .now-el { flex-shrink: 0; font-size: 11px; color: var(--accent); font-variant-numeric: tabular-nums; }

  /* Session-review zone header — divider + collapse toggle */
  .zone-head {
    display: flex; align-items: center; gap: 8px;
    width: 100%; padding: 9px 14px;
    background: none; border: 0; border-top: 1px solid var(--border);
    text-align: left; font: inherit; cursor: pointer;
    color: var(--fg-faint);
    transition: color 120ms ease;
  }
  .zone-head:hover { color: var(--fg-2); }
  /* No stray divider line when the review zone is the panel's first content. */
  .zone-head:first-child { border-top: none; }
  .zone-label {
    font-size: 10px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase;
    flex-shrink: 0;
  }
  .zone-line { flex: 1; height: 1px; background: var(--border); }
  .zone-head :global(.zone-chev) {
    flex-shrink: 0; transition: transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
    transform: rotate(-90deg);
  }
  .zone-head :global(.zone-chev.open) { transform: rotate(0deg); }

  .sect { display: flex; flex-direction: column; border-bottom: 1px solid var(--border); }
  .sect:last-of-type { border-bottom: none; }
  .sect-head {
    display: flex; align-items: center; gap: 8px;
    padding: 11px 14px 8px;
    color: var(--fg); font-size: var(--fs-sm); font-weight: 600; flex-shrink: 0;
  }
  .sect-head :global(svg) { color: var(--accent); flex-shrink: 0; }
  .sect-title { color: var(--fg); }
  .badge {
    margin-left: auto; font-size: 10px; padding: 2px 7px;
    background: var(--accent-soft); color: var(--accent);
    border-radius: 999px; font-variant-numeric: tabular-nums; font-weight: 650;
  }
  .badge.live { display: inline-flex; align-items: center; gap: 5px; }
  .counter {
    margin-left: auto; font-size: 10px; padding: 2px 7px;
    background: var(--accent-soft); color: var(--accent);
    border-radius: 999px; font-variant-numeric: tabular-nums; font-weight: 600;
  }
  .counter-sep { opacity: 0.55; margin: 0 1px; }

  /* Running rows */
  .rows { list-style: none; margin: 0; padding: 4px 8px 12px; display: flex; flex-direction: column; gap: 2px; }
  .run {
    display: flex; align-items: center; gap: 9px;
    width: 100%; padding: 7px 8px; border-radius: 6px;
    background: none; border: 0; text-align: left; font: inherit; cursor: pointer;
    font-size: var(--fs-sm); color: var(--fg-2);
    transition: background 120ms ease;
  }
  .run:hover { background: var(--bg-elev-2); }
  .run:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .run-ic { display: flex; align-items: center; color: var(--accent); flex-shrink: 0; }
  .run-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .run-label.mono .run-t { font-family: var(--font-mono); font-size: 12px; color: var(--fg); }
  .agtype { color: var(--accent); font-family: var(--font-mono); font-size: var(--fs-xs); margin-right: 6px; }
  .run-el { flex-shrink: 0; font-size: 10px; color: var(--fg-faint); font-variant-numeric: tabular-nums; }

  /* Tasks progress bar */
  .progress { position: relative; height: 2px; background: var(--bg-elev-2); overflow: hidden; flex-shrink: 0; }
  .progress-fill { height: 100%; background: var(--accent); transition: width 280ms cubic-bezier(0.22, 1, 0.36, 1); }
  .progress-active {
    position: absolute; top: 0; bottom: 0;
    background: color-mix(in oklch, var(--accent) 60%, transparent);
    animation: progress-active-pulse 1.4s ease-in-out infinite;
  }
  @keyframes progress-active-pulse { 0%, 100% { opacity: 0.45; } 50% { opacity: 0.95; } }

  .row.task {
    display: flex; align-items: flex-start; gap: 8px;
    padding: 7px 8px; border-radius: 6px;
    font-size: var(--fs-sm); color: var(--fg-2);
    transition: background 120ms ease-out;
  }
  .row.task:hover { background: var(--surface-hover); }
  .row.task .row-icon { display: flex; align-items: center; color: var(--fg-subtle); margin-top: 1px; }
  .row.task[data-status="in_progress"] .row-icon,
  .row.task[data-status="completed"] .row-icon { color: var(--accent); }
  .row.task[data-status="completed"] .row-text { color: var(--fg-subtle); text-decoration: line-through; }
  .row-text { line-height: 1.4; word-break: break-word; flex: 1; min-width: 0; }

  /* Clickable rows — outputs + sources */
  .row.file, .row.source { padding: 0; }
  .row-btn {
    display: flex; align-items: center; gap: 8px;
    width: 100%; padding: 7px 8px;
    background: transparent; border: 1px solid transparent; border-radius: 6px;
    color: var(--fg-2); font: inherit; font-size: var(--fs-sm); text-align: left; cursor: pointer;
    transition: background 120ms ease-out, border-color 120ms ease-out, color 120ms ease-out;
  }
  .row-btn:hover { background: var(--surface-hover); border-color: var(--border); color: var(--fg); }
  .row-btn:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .row-btn .row-icon { display: flex; align-items: center; color: var(--fg-subtle); flex-shrink: 0; }
  .row-btn:hover .row-icon { color: var(--accent); }
  .row-btn .row-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-aft { display: inline-flex; align-items: center; color: var(--fg-faint); opacity: 0; transition: opacity 120ms ease-out; flex-shrink: 0; }
  .row-btn:hover .row-aft, .row-btn:focus-visible .row-aft { opacity: 0.7; }

  /* Histogram */
  .histo { padding: 4px 14px 14px; display: flex; flex-direction: column; gap: 7px; }
  .hrow { display: flex; align-items: center; gap: 9px; font-size: var(--fs-sm); }
  .hname { width: 62px; flex-shrink: 0; color: var(--fg-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hmore {
    align-self: flex-start; margin-top: 2px; padding: 3px 6px;
    background: none; border: 0; border-radius: 5px;
    font: inherit; font-size: var(--fs-xs); color: var(--fg-subtle);
    cursor: pointer; transition: background 120ms ease, color 120ms ease;
  }
  .hmore:hover { background: var(--bg-elev-2); color: var(--fg-2); }
  .hmore:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .hbar { flex: 1; height: 7px; background: var(--bg-elev-2); border-radius: 4px; overflow: hidden; }
  .hbar i { display: block; height: 100%; background: var(--accent); border-radius: 4px; transition: width 280ms cubic-bezier(0.22,1,0.36,1); }
  .hn { width: 18px; text-align: right; color: var(--fg-muted); font-variant-numeric: tabular-nums; }

  /* Insights */
  .insights { padding-bottom: 6px; }
  .insight { display: flex; align-items: center; gap: 8px; padding: 9px 14px; font-size: var(--fs-sm); color: var(--fg-muted); }
  .insight .ic { display: flex; align-items: center; flex-shrink: 0; }
  .insight b { color: var(--fg-2); font-weight: 600; }
  .insight.warn .ic { color: var(--warn); }
  .insight.err .ic { color: var(--danger); }
  .insight .dim { color: var(--fg-subtle); }
  .insight.jump { width: 100%; background: none; border: 0; font: inherit; text-align: left; cursor: pointer; transition: background 120ms ease; }
  .insight.jump:hover { background: var(--bg-elev-2); }
  .insight.jump:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  :global(.tl-node.act-flash) {
    animation: act-flash 1.1s cubic-bezier(0.22, 1, 0.36, 1) both;
    border-radius: 8px;
  }
  @keyframes act-flash {
    0%   { box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 70%, transparent); background: color-mix(in oklch, var(--accent) 14%, transparent); }
    100% { box-shadow: 0 0 0 2px transparent; background: transparent; }
  }

  .empty-note { color: var(--fg-subtle); font-size: var(--fs-xs); line-height: 1.55; padding: 14px 16px; }
  .empty-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); margin-bottom: 4px; }

  .mono { font-family: var(--font-mono, monospace); }
  .live-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); animation: mon-live-pulse 1.4s ease-in-out infinite; }
  .activity :global(.mon-spin) { animation: mon-spin 0.9s linear infinite; }
  .activity :global(.mon-pulse) { animation: mon-live-pulse 1.4s ease-in-out infinite; }
  @keyframes mon-spin { to { transform: rotate(360deg); } }
  @keyframes mon-live-pulse { 0%,100% { opacity: 0.4; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) {
    .live-dot, .activity :global(.mon-spin), .activity :global(.mon-pulse), .progress-active { animation: none; }
    :global(.tl-node.act-flash) { animation: none; }
  }
</style>
