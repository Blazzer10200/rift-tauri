<script lang="ts">
  // v0.4 — Chat tabs bar. Browser-style tab strip + right-side chat status
  // chips (workspace, auth, ctx, tasks) + new-tab button. Mounted by AppShell
  // between the wire-error banner and the .body grid whenever the Chat
  // workspace is active.

  import { MessageSquare, Plus, X, ListChecks, FolderOpen, Folder, TerminalSquare, SplitSquareHorizontal } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";

  let dragFromIdx = $state<number | null>(null);
  let dragOverIdx = $state<number | null>(null);

  const tabs = $derived(assistant.openTabs);
  const activeId = $derived(assistant.currentConvoId);
  const splitActive = $derived(assistant.splitActive);
  const canAddPane = $derived(assistant.canAddPane);
  const paneCount = $derived(assistant.panes.length);

  /** Returns the 1-based pane index whose slot points at `id`, only when
   *  that pane is NOT the focused one (so we don't double-mark the active
   *  tab's pane label — the .active style already covers it). Else null. */
  function paneIndexFor(id: string): number | null {
    if (!splitActive) return null;
    const idx = assistant.panes.findIndex((p) => p.tabId === id);
    if (idx === -1) return null;
    if (idx === assistant.focusedPaneIdx) return null;
    return idx + 1;
  }

  const titleById = $derived.by(() => {
    const m = new Map<string, string>();
    for (const c of assistant.conversations) m.set(c.id, c.title);
    return m;
  });

  function titleFor(id: string): string {
    const t = titleById.get(id);
    if (t && t.trim().length > 0) return t.length > 40 ? t.slice(0, 40) + "…" : t;
    return "New chat";
  }

  function isStreamingTab(id: string): boolean {
    return assistant.tabFor(id)?.streaming ?? false;
  }

  function onTabClick(id: string) {
    void assistant.openTab(id);
  }

  function onClose(e: MouseEvent, id: string) {
    e.stopPropagation();
    void assistant.closeTab(id);
  }

  function onNewTab() {
    void assistant.newTab();
  }

  function onDragStart(e: DragEvent, idx: number) {
    dragFromIdx = idx;
    assistant.draggingTabId = tabs[idx] ?? null;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(idx));
    }
  }

  function onDragOver(e: DragEvent, idx: number) {
    if (dragFromIdx === null) return;
    e.preventDefault();
    dragOverIdx = idx;
  }

  function onDrop(e: DragEvent, idx: number) {
    e.preventDefault();
    if (dragFromIdx === null) return;
    const from = dragFromIdx;
    dragFromIdx = null;
    dragOverIdx = null;
    if (from === idx) return;
    assistant.reorderTabs(from, idx);
  }

  function onTailOver(e: DragEvent) {
    if (dragFromIdx === null) return;
    e.preventDefault();
    dragOverIdx = tabs.length;
  }

  function onTailDrop(e: DragEvent) {
    e.preventDefault();
    if (dragFromIdx === null) return;
    const from = dragFromIdx;
    dragFromIdx = null;
    dragOverIdx = null;
    assistant.reorderTabs(from, tabs.length);
  }

  function onDragEnd() {
    dragFromIdx = null;
    dragOverIdx = null;
    assistant.draggingTabId = null;
  }

  // -------- right-side chat status chips (absorbed from AssistantHeader) -----

  function leafName(p: string): string {
    const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
    const parts = norm.split("/");
    return parts[parts.length - 1] || norm;
  }

  function shortAgo(sinceMs: number): string {
    const sec = Math.max(0, Math.floor((Date.now() - sinceMs) / 1000));
    if (sec < 60) return `${sec}s`;
    if (sec < 3600) return `${Math.floor(sec / 60)}m`;
    return `${Math.floor(sec / 3600)}h`;
  }

  const foreignShell = $derived(assistant.remoteShellLockedByOther);

  const authWarn = $derived.by(() => {
    const a = assistant.auth;
    if (!a) return null;
    if (a.pill === "yellow") return { tone: "yellow", text: "API key" };
    if (a.pill === "red") return { tone: "red", text: "Not connected" };
    return null;
  });

  const taskCount = $derived(assistant.tasks.length);
  const taskDone = $derived(assistant.tasks.filter((t) => t.status === "completed").length);

  let pulse = $state(false);
  let lastSeenUpdate = 0;
  $effect(() => {
    const t = assistant.ui.tasksUpdatedAt;
    if (t > lastSeenUpdate) {
      lastSeenUpdate = t;
      if (!assistant.ui.dockOpen && taskCount > 0) {
        pulse = true;
        setTimeout(() => (pulse = false), 700);
      }
    }
  });

  const tasksOpen = $derived(assistant.ui.dockOpen);
  function toggleTasks() {
    assistant.ui.dockOpen = !assistant.ui.dockOpen;
  }

  function contextWindowFor(model: string | null): number {
    if (!model) return 200_000;
    if (/\[1m\]/i.test(model)) return 1_000_000;
    const id = model.toLowerCase();
    if (id.includes("haiku")) return 200_000;
    if (/sonnet-4-[56]/.test(id) || /opus-4-[67]/.test(id)) return 1_000_000;
    return 200_000;
  }

  function shortK(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 10_000) return `${Math.round(n / 1000)}K`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return String(n);
  }

  const newTokens = $derived(
    assistant.lastTurnUsage
      ? assistant.lastTurnUsage.input + assistant.lastTurnUsage.cacheCreate
      : 0,
  );
  const ctxTokens = $derived(
    assistant.lastTurnUsage
      ? assistant.lastTurnUsage.input + assistant.lastTurnUsage.cacheRead + assistant.lastTurnUsage.cacheCreate
      : 0,
  );
  const ctxWindow = $derived(contextWindowFor(assistant.lastModelId));
  const ctxPct = $derived(ctxWindow > 0 ? Math.min(100, (ctxTokens / ctxWindow) * 100) : 0);
  const ctxTone = $derived(ctxPct >= 90 ? "red" : ctxPct >= 70 ? "yellow" : "ok");
  const ctxTitle = $derived.by(() => {
    const u = assistant.lastTurnUsage;
    if (!u) return "Context — send a message to populate";
    const s = assistant.sessionUsage;
    const cost =
      assistant.totalCostUsd !== null && assistant.totalCostUsd !== undefined
        ? ` · $${assistant.totalCostUsd.toFixed(4)}`
        : "";
    const hint =
      ctxPct >= 85
        ? "\n\nWindow nearly full — start a new chat (Ctrl+T) to drop the accumulated tool-result history."
        : ctxPct >= 70
          ? "\n\nCache is growing — Ctrl+T for a fresh chat if responses start lagging."
          : "";
    return (
      `Context this turn: ${ctxTokens.toLocaleString()} / ${ctxWindow.toLocaleString()} (${ctxPct.toFixed(1)}%)\n` +
      `  • new this turn: ${newTokens.toLocaleString()} (input ${u.input.toLocaleString()} + cache-create ${u.cacheCreate.toLocaleString()})\n` +
      `  • cache read: ${u.cacheRead.toLocaleString()} (replayed from prior turns)\n` +
      `  • output: ${u.output.toLocaleString()}\n` +
      `Session totals (${s.turns} turn${s.turns === 1 ? "" : "s"}):\n` +
      `  • input: ${s.totalInput.toLocaleString()} · output: ${s.totalOutput.toLocaleString()}\n` +
      `  • cache read: ${s.totalCacheRead.toLocaleString()} · cache create: ${s.totalCacheCreate.toLocaleString()}${cost}\n` +
      `Model: ${assistant.lastModelId ?? "?"}${hint}`
    );
  });
</script>

<div class="tabsbar" role="tablist" aria-label="Chat tabs">
  <div class="strip">
    {#each tabs as id, idx (id)}
      <div
        class="tab"
        class:in-pane={paneIndexFor(id) !== null}
        class:active={id === activeId}
        class:drop-target={dragOverIdx === idx && dragFromIdx !== null && dragFromIdx !== idx}
        role="tab"
        aria-selected={id === activeId}
        tabindex="0"
        data-tab-id={id}
        draggable={true}
        ondragstart={(e) => onDragStart(e, idx)}
        ondragover={(e) => onDragOver(e, idx)}
        ondrop={(e) => onDrop(e, idx)}
        ondragend={onDragEnd}
        onclick={() => onTabClick(id)}
        onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onTabClick(id); } }}
        title={titleFor(id)}
      >
        <span class="icon" aria-hidden="true">
          {#if isStreamingTab(id)}
            <span class="dot"></span>
          {:else}
            <MessageSquare size={12}/>
          {/if}
        </span>
        <span class="title">{titleFor(id)}</span>
        {#if paneIndexFor(id) !== null}
          <span class="pane-badge" title="Open in pane {paneIndexFor(id)}">{paneIndexFor(id)}</span>
        {/if}
        <button
          class="close"
          type="button"
          aria-label="Close tab"
          title="Close (Ctrl+W)"
          onclick={(e) => onClose(e, id)}
        >
          <X size={11}/>
        </button>
      </div>
    {/each}
    <div
      class="tail-zone"
      class:drop-target={dragOverIdx === tabs.length && dragFromIdx !== null}
      ondragover={onTailOver}
      ondrop={onTailDrop}
      role="presentation"
    ></div>
  </div>

  <div class="actions">
    {#if assistant.workspace.current}
      <span class="ws-chip" title={assistant.workspace.current}>
        <Folder size={11}/>
        <span class="ws-name">{leafName(assistant.workspace.current)}</span>
        <button
          class="ws-x"
          type="button"
          title="Close folder"
          onclick={() => void assistant.clearRoot()}
        ><X size={10}/></button>
      </span>
    {:else}
      <button
        class="hdr-btn"
        type="button"
        title="Open project folder"
        onclick={() => void assistant.pickFolder()}
      >
        <FolderOpen size={12}/>
        <span class="hdr-btn-label">Open folder</span>
      </button>
    {/if}

    {#if authWarn}
      <span
        class="auth-warn"
        data-tone={authWarn.tone}
        title={assistant.auth?.summary ?? authWarn.text}
      >
        <span class="auth-dot"></span>
        <span>{authWarn.text}</span>
      </span>
    {/if}

    {#if foreignShell}
      <span
        class="shell-lock"
        title={`${foreignShell.user}@${foreignShell.host} is running a remote command`}
      >
        <TerminalSquare size={11}/>
        <span>{foreignShell.user} ({shortAgo(foreignShell.sinceMs)})</span>
      </span>
    {/if}

    {#if ctxTokens > 0}
      <span class="ctx-pill" data-tone={ctxTone} title={ctxTitle}>
        <span class="ctx-bar"><span class="ctx-fill" style="width: {ctxPct}%"></span></span>
        <span class="ctx-text">{shortK(ctxTokens)}<span class="ctx-sep">/</span>{shortK(ctxWindow)}</span>
        <span class="ctx-pct">{Math.round(ctxPct)}%</span>
      </span>
    {/if}

    {#if taskCount > 0}
      <button
        class="dock-toggle"
        class:open={tasksOpen}
        class:pulse
        type="button"
        onclick={toggleTasks}
        title="Tasks panel"
      >
        <ListChecks size={12} />
        <span class="task-chip">{taskDone}/{taskCount}</span>
      </button>
    {/if}

    <button
      class="split-toggle"
      class:active={splitActive}
      type="button"
      onclick={() => assistant.addPane()}
      disabled={!canAddPane}
      title={canAddPane
        ? `Add pane (Ctrl+\\) — ${paneCount} of 4`
        : `Max panes reached (${paneCount}/4)`}
      aria-label="Add pane"
    >
      <SplitSquareHorizontal size={12} />
      {#if splitActive}
        <span class="split-count">{paneCount}</span>
      {/if}
    </button>
  </div>

  <button
    class="new-tab"
    type="button"
    title="New chat (Ctrl+T)"
    aria-label="New chat"
    onclick={onNewTab}
  >
    <Plus size={13}/>
  </button>
</div>

<style>
  .tabsbar {
    height: 34px;
    flex-shrink: 0;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: stretch;
    overflow: hidden;
  }
  .strip {
    flex: 1; min-width: 0;
    display: flex;
    align-items: stretch;
    gap: 0;
    padding: 0 4px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }
  .strip::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .strip::-webkit-scrollbar-button { display: none; }

  .tab {
    flex: 0 1 220px;
    min-width: 120px;
    max-width: 220px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    margin: 4px 1px 0;
    background: var(--bg);
    border: 1px solid transparent;
    border-bottom: 0;
    border-radius: 5px 5px 0 0;
    color: var(--fg-muted);
    cursor: pointer;
    font-size: var(--fs-sm);
    user-select: none;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
    position: relative;
    animation: tab-in 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes tab-in {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .tab { animation: none; }
  }
  .tab:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .tab.active {
    background: var(--bg-elev-1);
    color: var(--fg);
    font-weight: 600;
    border-color: var(--border);
    /* Top-edge accent — picks the active tab out of the row at a glance.
       Inset box-shadow vs ::before because ::before is reserved for the
       in-pane indicator. */
    box-shadow: inset 0 2px 0 0 var(--accent);
    z-index: 1;
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 0; right: 0; bottom: -1px;
    height: 1px;
    background: var(--bg-elev-1);
  }
  .tab.drop-target {
    box-shadow: -2px 0 0 var(--accent);
  }
  /* Split-pane indicator — when a non-focused pane owns this tab, mark it
     with a dim accent underline. The numbered .pane-badge tells the user
     WHICH pane (1-4). Focused-pane tabs use the normal .active style. */
  .tab.in-pane::before {
    content: "";
    position: absolute;
    left: 6px; right: 6px;
    bottom: 2px;
    height: 2px;
    border-radius: 2px;
    background: color-mix(in oklch, var(--accent) 45%, transparent);
    opacity: 0.7;
  }
  .pane-badge {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 14px; height: 14px;
    padding: 0 4px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--accent) 18%, var(--bg-elev-2));
    color: color-mix(in oklch, var(--accent) 80%, var(--fg));
    font-size: 9px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    flex-shrink: 0;
  }

  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--fg-faint);
  }
  .tab.active .icon { color: var(--accent); }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: tab-pulse 1.4s ease-in-out infinite;
  }
  @keyframes tab-pulse {
    0%, 100% { opacity: 0.35; transform: scale(0.85); }
    50%      { opacity: 1;    transform: scale(1); }
  }

  .title {
    flex: 1; min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.2;
  }

  .close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: var(--fg-faint);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
  }
  .tab:hover .close, .tab.active .close, .tab:focus-within .close { opacity: 1; }
  .close:hover { background: var(--surface-hover); color: var(--fg); }

  .tail-zone {
    flex: 1;
    min-width: 12px;
    align-self: stretch;
  }
  .tail-zone.drop-target {
    box-shadow: inset 2px 0 0 var(--accent);
  }

  /* ---- right-side action chips ---- */
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 6px 0 4px;
    flex-shrink: 0;
    align-self: center;
  }

  .hdr-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 3px 8px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--fg-muted);
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    line-height: 1;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .hdr-btn:hover { color: var(--fg); border-color: var(--border-strong); background: var(--surface-hover); }
  .hdr-btn-label { font-size: var(--fs-xs); }

  .ws-chip {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 4px 2px 8px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--fg-2);
    font-size: var(--fs-xs);
    line-height: 1;
    max-width: 220px;
  }
  .ws-chip :global(svg) { color: var(--fg-muted); }
  .ws-chip .ws-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ws-x {
    background: transparent;
    border: 0;
    border-radius: 999px;
    padding: 1px;
    color: var(--fg-muted);
    cursor: pointer;
    display: inline-flex;
    opacity: 0.65;
    transition: opacity 120ms, background 120ms, color 120ms;
  }
  .ws-x:hover { opacity: 1; color: var(--fg); background: var(--surface-hover); }

  .auth-warn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    line-height: 1;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
  }
  .auth-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--fg-muted); }
  .auth-warn[data-tone="yellow"] {
    color: var(--warn);
    border-color: color-mix(in oklch, var(--warn) 35%, var(--border));
    background: var(--warn-soft);
  }
  .auth-warn[data-tone="yellow"] .auth-dot { background: var(--warn); }
  .auth-warn[data-tone="red"] {
    color: var(--danger);
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    background: color-mix(in oklch, var(--danger) 10%, transparent);
  }
  .auth-warn[data-tone="red"] .auth-dot { background: var(--danger); }

  .shell-lock {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    line-height: 1;
    color: var(--warn);
    background: var(--warn-soft);
    border: 1px solid color-mix(in oklch, var(--warn) 35%, var(--border));
  }

  .ctx-pill {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    line-height: 1;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    color: var(--fg-muted);
    cursor: help;
    font-variant-numeric: tabular-nums;
  }
  .ctx-pill .ctx-bar {
    position: relative;
    width: 38px;
    height: 4px;
    background: var(--surface-hover);
    border-radius: 999px;
    overflow: hidden;
  }
  .ctx-pill .ctx-fill {
    display: block; height: 100%;
    background: var(--accent);
    transition: width 200ms ease-out, background 120ms;
    border-radius: 999px;
  }
  .ctx-pill .ctx-text { color: var(--fg); }
  .ctx-pill .ctx-sep { color: var(--fg-muted); margin: 0 2px; }
  .ctx-pill .ctx-pct { color: var(--fg-muted); font-size: 10px; }
  .ctx-pill[data-tone="yellow"] {
    border-color: color-mix(in oklch, var(--warn) 35%, var(--border));
    background: var(--warn-soft); color: var(--warn);
  }
  .ctx-pill[data-tone="yellow"] .ctx-text { color: var(--warn); }
  .ctx-pill[data-tone="yellow"] .ctx-fill { background: var(--warn); }
  .ctx-pill[data-tone="red"] {
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    background: color-mix(in oklch, var(--danger) 10%, transparent);
    color: var(--danger);
  }
  .ctx-pill[data-tone="red"] .ctx-text { color: var(--danger); }
  .ctx-pill[data-tone="red"] .ctx-fill { background: var(--danger); }

  .dock-toggle {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 3px 8px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--fg-muted);
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    line-height: 1;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .dock-toggle:hover { color: var(--fg); border-color: var(--border-strong); background: var(--surface-hover); }
  .dock-toggle.open {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  .dock-toggle.pulse { animation: dock-pulse 700ms ease-out; }

  .split-toggle {
    display: inline-flex; align-items: center; justify-content: center;
    gap: 3px;
    min-width: 26px; height: 22px;
    padding: 0 5px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--fg-muted);
    cursor: pointer;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .split-toggle:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); background: var(--surface-hover); }
  .split-toggle.active {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  .split-toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .split-count {
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  @keyframes dock-pulse {
    0%   { box-shadow: 0 0 0 0 var(--accent-soft); border-color: var(--accent); }
    60%  { box-shadow: 0 0 0 6px transparent; }
    100% { box-shadow: 0 0 0 0 transparent; }
  }
  .task-chip {
    font-size: 10px;
    padding: 1px 5px;
    background: var(--accent);
    color: var(--accent-fg);
    border-radius: 999px;
    font-weight: 600;
  }
  .dock-toggle:not(.open) .task-chip {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .new-tab {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 26px;
    margin: 4px 5px 0 4px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    color: var(--fg-muted);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
    align-self: center;
  }
  .new-tab:hover {
    background: var(--surface-hover);
    color: var(--fg);
    border-color: var(--border);
  }
</style>
