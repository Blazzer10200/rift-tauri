<script lang="ts">
  // v0.4 — Chat tabs bar. Browser-style tab strip + right-side chat status
  // chips (workspace, auth, ctx, tasks) + new-tab button. Mounted by AppShell
  // between the wire-error banner and the .body grid whenever the Chat
  // workspace is active.

  import { MessageSquare, Plus, X, PanelRight, FolderOpen, Folder, TerminalSquare, SplitSquareHorizontal, Layers, History, ChevronDown, Globe } from "lucide-svelte";
  import { onDestroy } from "svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { modelFamily } from "../../state/assistant/helpers";
  import { browserDock } from "../../state/browserDock.svelte";
  import OpenInPaneMenu from "../assistant/OpenInPaneMenu.svelte";
  import HistoryDrawer from "../assistant/HistoryDrawer.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  let ctxMenu = $state<{ tabId: string; x: number; y: number } | null>(null);
  let historyOpen = $state(false);
  let historyAnchor = $state<HTMLButtonElement | undefined>();
  let historyPopover = $state<HTMLDivElement | undefined>();
  let historyPos = $state<{ top: number; right: number }>({ top: 0, right: 0 });

  // Context-detail popover — the ctx-pill's tooltip dumped a wall of text;
  // this turns it into a clickable panel with the same breakdown laid out.
  let ctxPanelOpen = $state(false);
  let ctxAnchor = $state<HTMLButtonElement | undefined>();
  let ctxPanel = $state<HTMLDivElement | undefined>();
  let ctxPos = $state<{ top: number; right: number }>({ top: 0, right: 0 });

  // Portal action — moves the node to <body> so the popover escapes the
  // `.tabs-rail` overflow:hidden clip (that clip exists to drive the
  // workspace-hop collapse animation, can't remove it).
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy() { node.remove(); } };
  }

  function openHistory() {
    if (!historyAnchor) return;
    const r = historyAnchor.getBoundingClientRect();
    historyPos = {
      top: r.bottom + 6,
      right: Math.max(8, window.innerWidth - r.right),
    };
    historyOpen = true;
  }

  // Close history popover on outside-click or Escape.
  $effect(() => {
    if (!historyOpen) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (historyAnchor?.contains(t)) return;
      if (historyPopover?.contains(t)) return;
      historyOpen = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { historyOpen = false; historyAnchor?.focus(); }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  });

  function toggleCtxPanel() {
    if (ctxPanelOpen) { ctxPanelOpen = false; return; }
    if (!ctxAnchor) return;
    const r = ctxAnchor.getBoundingClientRect();
    ctxPos = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right) };
    ctxPanelOpen = true;
  }
  $effect(() => {
    if (!ctxPanelOpen) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (ctxAnchor?.contains(t)) return;
      if (ctxPanel?.contains(t)) return;
      ctxPanelOpen = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { ctxPanelOpen = false; ctxAnchor?.focus(); }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  });

  function onTabContext(e: MouseEvent, id: string) {
    e.preventDefault();
    ctxMenu = { tabId: id, x: e.clientX, y: e.clientY };
  }

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

  // Window-level dragend safety net + unmount cleanup. Workspace-switch
  // collapses the rail mid-drag; some browsers swallow the per-element
  // dragend in that case. (#208)
  $effect(() => {
    window.addEventListener("dragend", onDragEnd);
    return () => window.removeEventListener("dragend", onDragEnd);
  });
  onDestroy(onDragEnd);

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
    if (a.pill === "red") return { tone: "red", text: "Sign in" };
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
        // Return cleanup so a workspace-switch / unmount within 700ms doesn't
        // leave the callback writing into a detached closure. (#159)
        const handle = setTimeout(() => (pulse = false), 700);
        return () => clearTimeout(handle);
      }
    }
  });

  const tasksOpen = $derived(assistant.ui.dockOpen);
  function toggleTasks() {
    assistant.ui.dockOpen = !assistant.ui.dockOpen;
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
  const ctxTokens = $derived(assistant.ctxTokens);
  const ctxWindow = $derived(assistant.ctxWindow);
  const ctxPct = $derived(assistant.ctxPct);
  const ctxTone = $derived(ctxPct >= 90 ? "red" : ctxPct >= 70 ? "yellow" : "ok");
  // Detail-panel sources (mirror the data the old tooltip text concatenated).
  const lastTurnUsage = $derived(assistant.lastTurnUsage);
  const sessionUsage = $derived(assistant.sessionUsage);
  const totalCostUsd = $derived(assistant.totalCostUsd);
  const lastModelId = $derived(assistant.lastModelId);
  const autoCompactThreshold = $derived(assistant.autoCompactThreshold);
  const compactWarning = $derived(assistant.compactWarning);
  const activeAgents = $derived(
    (assistant.activeTab?.agentSpawns ?? []).filter((a) => a.completedAt === null),
  );
  const activeAgentTitle = $derived.by(() =>
    activeAgents.map((a) => `${a.subagentType}: ${a.description}`).join("\n"),
  );
  const autoCompactDisabledNudge = $derived(
    assistant.autoCompactThreshold === null && ctxPct >= 70
      ? `Auto-compact off (${ctxPct >= 85 ? "compact soon" : "approaching window cap"})`
      : null,
  );

  const ctxTitle = $derived.by(() => {
    const u = assistant.lastTurnUsage;
    if (!u) return "Context — send a message to populate";
    const s = assistant.sessionUsage;
    const cost =
      assistant.totalCostUsd !== null && assistant.totalCostUsd !== undefined
        ? ` · ${assistant.totalCostUsd.toFixed(4)}`
        : "";
    const action =
      ctxPct >= 85
        ? "\n\nWindow nearly full. Type /compact to summarize and reset this chat, or Ctrl+T for a fresh one."
        : ctxPct >= 70
          ? "\n\nApproaching the window cap. /compact will summarize-and-reset; Ctrl+T starts fresh."
          : "";
    const autoNote =
      assistant.autoCompactThreshold === null && ctxPct >= 70
        ? "\nAuto-compact is off — enable it in Settings → Conversation compaction to fire automatically."
        : "";
    return (
      `Context: ${ctxTokens.toLocaleString()} / ${ctxWindow.toLocaleString()} (${ctxPct.toFixed(1)}%) — the model's hard input cap.\n\n` +
      `This turn:\n` +
      `  • new (input + cache-create): ${newTokens.toLocaleString()}\n` +
      `  • cache-read: ${u.cacheRead.toLocaleString()} — replay of prior turns, still occupies the window every turn\n` +
      `  • output: ${u.output.toLocaleString()}\n\n` +
      `The cache only saves billing — those tokens are in the window every send, same as fresh input.\n\n` +
      `Session (${s.turns} turn${s.turns === 1 ? "" : "s"}): in ${s.totalInput.toLocaleString()} · out ${s.totalOutput.toLocaleString()} · cache ${s.totalCacheRead.toLocaleString()} r / ${s.totalCacheCreate.toLocaleString()} w${cost}\n` +
      `Model: ${assistant.lastModelId ?? "?"}${action}${autoNote}`
    );
  });
</script>

<div class="tabsbar" data-model={modelFamily(assistant.model)} role="tablist" aria-label="Chat tabs">
  <div class="strip">
    {#each tabs as id, idx (id)}
      <div
        class="tab"
        class:in-pane={paneIndexFor(id) !== null}
        class:active={id === activeId}
        class:streaming={isStreamingTab(id)}
        class:bg-streaming={isStreamingTab(id) && id !== activeId}
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
        oncontextmenu={(e) => onTabContext(e, id)}
        onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onTabClick(id); } }}
        use:tooltip={isStreamingTab(id) && id !== activeId
          ? `${titleFor(id)} — streaming in background. Click to switch.`
          : titleFor(id)}
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
          <span class="pane-badge" use:tooltip={"Open in pane {paneIndexFor(id)}"}>{paneIndexFor(id)}</span>
        {/if}
        <button
          class="close"
          type="button"
          aria-label="Close tab"
          use:tooltip={"Close (Ctrl+W)"}
          onclick={(e) => onClose(e, id)}
        >
          <X size={11}/>
        </button>
      </div>
    {/each}
    <button
      class="new-tab"
      type="button"
      use:tooltip={"New chat (Ctrl+T)"}
      aria-label="New chat"
      onclick={onNewTab}
    >
      <Plus size={14}/>
    </button>
    <div
      class="tail-zone"
      class:drop-target={dragOverIdx === tabs.length && dragFromIdx !== null}
      ondragover={onTailOver}
      ondrop={onTailDrop}
      role="presentation"
    ></div>
  </div>

  <div class="actions">
    <button
      class="hdr-btn history-btn"
      class:open={historyOpen}
      type="button"
      use:tooltip={"Conversation history"}
      onclick={() => { historyOpen ? (historyOpen = false) : openHistory(); }}
      aria-haspopup="dialog"
      aria-expanded={historyOpen}
      bind:this={historyAnchor}
    >
      <History size={12}/>
      <span class="hdr-btn-label">History</span>
      {#if assistant.conversations.length > 0}
        <span class="history-count">{assistant.conversations.length}</span>
      {/if}
      <ChevronDown size={10} class={historyOpen ? "chev-open" : ""}/>
    </button>

    {#if assistant.workspace.current}
      <span class="ws-chip" use:tooltip={assistant.workspace.current}>
        <Folder size={11}/>
        <span class="ws-name">{leafName(assistant.workspace.current)}</span>
        <button
          class="ws-x"
          type="button"
          use:tooltip={"Close folder"}
          onclick={() => void assistant.clearRoot()}
        ><X size={10}/></button>
      </span>
    {:else}
      <button
        class="hdr-btn"
        type="button"
        use:tooltip={"Open project folder"}
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
        use:tooltip={assistant.auth?.summary ?? authWarn.text}
      >
        <span class="auth-dot"></span>
        <span>{authWarn.text}</span>
      </span>
    {/if}

    {#if foreignShell}
      <span
        class="shell-lock"
        use:tooltip={`${foreignShell.user}@${foreignShell.host} is running a remote command`}
      >
        <TerminalSquare size={11}/>
        <span>{foreignShell.user} ({shortAgo(foreignShell.sinceMs)})</span>
      </span>
    {/if}

    {#if activeAgents.length > 0}
      <button
        type="button"
        class="agents-pill"
        use:tooltip={`${activeAgentTitle}\n\nClick to open a fresh tab — this one keeps streaming in the background, you can chat in the new one.`}
        onclick={() => void assistant.newTab()}
      >
        <span class="agents-dot"></span>
        <span>{activeAgents.length} agent{activeAgents.length === 1 ? "" : "s"}</span>
        <Plus size={10} aria-hidden="true" />
      </button>
    {/if}

    {#if compactWarning}
      <span class="compact-warn" use:tooltip={"Compact early w/ /compact <focus> if you want fine control over the summary."}
        >{compactWarning}</span>
    {:else if autoCompactDisabledNudge}
      <span
        class="compact-warn"
        data-tone={ctxPct >= 85 ? "red" : "yellow"}
        use:tooltip={"Auto-compact is off in Settings → Conversation compaction. Cache-read tokens count toward the window — at 95% there's no headroom for the next turn. Enable a threshold to fire compaction automatically, or click the Compact button now."}
        >{autoCompactDisabledNudge}</span>
    {/if}

    {#if ctxTokens > 0}
      <button
        type="button"
        class="ctx-pill"
        class:open={ctxPanelOpen}
        data-tone={ctxTone}
        bind:this={ctxAnchor}
        onclick={toggleCtxPanel}
        aria-haspopup="dialog"
        aria-expanded={ctxPanelOpen}
        use:tooltip={"Context window — click for the full breakdown"}
      >
        <span class="ctx-bar"><span class="ctx-fill" style="width: {ctxPct}%"></span></span>
        <span class="ctx-text">{shortK(ctxTokens)}<span class="ctx-sep">/</span>{shortK(ctxWindow)}</span>
        <span class="ctx-pct">{Math.round(ctxPct)}%</span>
      </button>
    {/if}

    {#if ctxPct >= 50}
      <button
        class="compact-btn"
        data-tone={ctxTone}
        type="button"
        onclick={() => {
          const cost = ctxPct >= 70 ? "≈ $0.91" : "≈ $0.30";
          if (!confirm(`Compact conversation? ${cost} on Haiku · drops context to ~5-10% · next turn carries the summary forward.`)) return;
          void assistant.compactConversation();
        }}
        use:tooltip={"Summarize + remint the CLI session. Drops working context but preserves the summary on the next turn."}
      >
        <Layers size={11} />
        <span>Compact</span>
      </button>
    {/if}

    <div class="seg" role="group" aria-label="View">
      <button
        class="seg-btn"
        class:on={browserDock.open}
        type="button"
        use:tooltip={"Web browser panel"}
        onclick={() => browserDock.toggle()}
        aria-pressed={browserDock.open}
      >
        <Globe size={13}/>
      </button>
      <button
        class="seg-btn"
        class:on={tasksOpen}
        class:pulse
        type="button"
        onclick={toggleTasks}
        aria-pressed={tasksOpen}
        use:tooltip={taskCount > 0
          ? `Side panel — Session · Activity (${taskDone}/${taskCount} tasks done)`
          : "Side panel — Session · Activity"}
      >
        <PanelRight size={13} />
        {#if taskCount > 0}
          <span class="seg-chip">{taskDone}/{taskCount}</span>
        {/if}
      </button>
      <button
        class="seg-btn"
        class:on={splitActive}
        type="button"
        onclick={() => assistant.addPane()}
        disabled={!canAddPane}
        use:tooltip={canAddPane
          ? `Add pane (Ctrl+\\) — ${paneCount} of 4`
          : `Max panes reached (${paneCount}/4)`}
        aria-label="Add pane"
      >
        <SplitSquareHorizontal size={13} />
        {#if splitActive}
          <span class="seg-chip">{paneCount}</span>
        {/if}
      </button>
    </div>
  </div>
</div>

{#if ctxMenu}
  <OpenInPaneMenu
    tabId={ctxMenu.tabId}
    x={ctxMenu.x}
    y={ctxMenu.y}
    onClose={() => (ctxMenu = null)}
  />
{/if}

{#if historyOpen}
  <div
    class="history-popover"
    role="dialog"
    aria-label="Conversation history"
    bind:this={historyPopover}
    use:portal
    style="top: {historyPos.top}px; right: {historyPos.right}px;"
  >
    <HistoryDrawer compact onSelected={() => (historyOpen = false)} />
  </div>
{/if}

{#if ctxPanelOpen}
  <div
    class="ctx-panel"
    role="dialog"
    aria-label="Context window details"
    bind:this={ctxPanel}
    use:portal
    style="top: {ctxPos.top}px; right: {ctxPos.right}px;"
  >
    <div class="ctx-panel-head">
      <span class="ctx-panel-dot" data-tone={ctxTone}></span>
      <span class="ctx-panel-title">Context window</span>
      <span class="ctx-panel-pct" data-tone={ctxTone}>{ctxPct.toFixed(1)}%</span>
    </div>
    <div class="ctx-panel-bar">
      <span class="ctx-panel-fill" data-tone={ctxTone} style="width: {Math.min(100, ctxPct)}%"></span>
    </div>
    <div class="ctx-panel-sub">
      {ctxTokens.toLocaleString()} / {ctxWindow.toLocaleString()} tokens — the model's hard input cap
    </div>

    {#if lastTurnUsage}
      <div class="ctx-panel-section">
        <div class="ctx-panel-section-label">This turn</div>
        <div class="ctx-row"><span>New (input + cache-create)</span><span class="num">{newTokens.toLocaleString()}</span></div>
        <div class="ctx-row"><span>Cache-read (replayed)</span><span class="num">{lastTurnUsage.cacheRead.toLocaleString()}</span></div>
        <div class="ctx-row"><span>Output</span><span class="num">{lastTurnUsage.output.toLocaleString()}</span></div>
      </div>
      <div class="ctx-panel-section">
        <div class="ctx-panel-section-label">Session · {sessionUsage.turns} turn{sessionUsage.turns === 1 ? "" : "s"}</div>
        <div class="ctx-row"><span>Input</span><span class="num">{sessionUsage.totalInput.toLocaleString()}</span></div>
        <div class="ctx-row"><span>Output</span><span class="num">{sessionUsage.totalOutput.toLocaleString()}</span></div>
        <div class="ctx-row"><span>Cache read / write</span><span class="num">{sessionUsage.totalCacheRead.toLocaleString()} / {sessionUsage.totalCacheCreate.toLocaleString()}</span></div>
        {#if totalCostUsd != null}
          <div class="ctx-row"><span>Cost</span><span class="num">${totalCostUsd.toFixed(4)}</span></div>
        {/if}
      </div>
      <div class="ctx-panel-note">
        Cache only saves billing — those tokens occupy the window every send, same as fresh input.
      </div>
    {:else}
      <div class="ctx-panel-empty">Send a message to populate usage.</div>
    {/if}

    <div class="ctx-panel-foot">
      <span class="ctx-panel-meta">
        {lastModelId ?? "—"} · auto-compact {autoCompactThreshold === null ? "off" : `${autoCompactThreshold}%`}
      </span>
      <button
        type="button"
        class="ctx-panel-compact"
        data-tone={ctxTone}
        onclick={() => {
          const cost = ctxPct >= 70 ? "≈ $0.91" : "≈ $0.30";
          if (!confirm(`Compact conversation? ${cost} on Haiku · drops context to ~5-10% · next turn carries the summary forward.`)) return;
          ctxPanelOpen = false;
          void assistant.compactConversation();
        }}
        use:tooltip={"Summarize + remint the CLI session. Drops working context but preserves the summary on the next turn."}
      >
        <Layers size={11} /> Compact now
      </button>
    </div>
  </div>
{/if}

<style>
  .tabsbar {
    position: relative;
    height: 34px;
    flex-shrink: 0;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: stretch;
    overflow: hidden;
  }
  /* Aurora hue follows the active model, matching the composer ring. */
  .tabsbar[data-model="sonnet"] { --model-color: oklch(0.74 0.13 230); }
  .tabsbar[data-model="opus"]   { --model-color: oklch(0.70 0.18 295); }
  .tabsbar[data-model="haiku"]  { --model-color: oklch(0.78 0.14 180); }
  .tabsbar                      { --model-color: var(--accent); }
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
    background: color-mix(in oklch, var(--bg-elev-1) 30%, transparent);
    border: 1px solid transparent;
    border-bottom: 0;
    border-radius: 7px 7px 0 0;
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
       in-pane indicator. Tinted by current model + soft underglow that
       washes the lower edge of the tab in model color. */
    box-shadow:
      inset 0 2px 0 0 var(--model-color),
      inset 0 -28px 28px -24px color-mix(in oklch, var(--model-color) 28%, transparent);
    z-index: 1;
  }
  .tab.active .icon { color: var(--model-color); }
  .tab.active .dot { background: var(--model-color); box-shadow: 0 0 8px color-mix(in oklch, var(--model-color) 60%, transparent); }
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
  /* Background-streaming tab (streaming + NOT active): subtle accent tint
     + animated underline pulse. Visible-but-quiet — the user should be able
     to scan the strip and see "Tab 2 is busy in background" without it
     stealing attention from whatever they're typing in the active tab. */
  .tab.bg-streaming {
    background: color-mix(in oklch, var(--accent) 8%, var(--bg));
    color: var(--fg);
  }
  .tab.bg-streaming::after {
    content: "";
    position: absolute;
    left: 6px; right: 6px;
    bottom: 2px;
    height: 2px;
    border-radius: 2px;
    background: var(--accent);
    opacity: 0.65;
    animation: bg-stream-pulse 1.6s ease-in-out infinite;
  }
  @keyframes bg-stream-pulse {
    0%, 100% { opacity: 0.3; }
    50%      { opacity: 0.85; }
  }
  @media (prefers-reduced-motion: reduce) {
    .tab.bg-streaming::after { animation: none; opacity: 0.5; }
  }
  /* in-pane + bg-streaming share the ::before/::after slot — the in-pane
     ::before still renders (different pseudo); but its ::after would
     collide. Suppress the active-tab ::after override (which fills bg) by
     scoping that rule already to .active — bg-streaming is mutually
     exclusive with .active by render-time guard. */
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
    padding: 0 8px 0 10px;
    flex-shrink: 0;
    align-self: center;
    /* Visual separator from the tab strip — the actions cluster is a
       different domain (session-tools) and benefits from a hairline cut
       rather than blending into the last tab. Soft gradient so the cut
       reads on dark surfaces without feeling boxy. */
    position: relative;
  }
  .actions::before {
    content: "";
    position: absolute;
    left: 0;
    top: 6px; bottom: 6px;
    width: 1px;
    background: linear-gradient(180deg,
      transparent,
      color-mix(in oklch, var(--border) 90%, transparent) 30%,
      color-mix(in oklch, var(--border) 90%, transparent) 70%,
      transparent);
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

  .history-btn {
    position: relative;
  }
  .history-btn.open {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
  }
  .history-btn :global(svg) { transition: transform 140ms ease; }
  .history-btn :global(.chev-open) { transform: rotate(180deg); }
  .history-count {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    padding: 1px 5px;
    background: var(--bg-elev-2);
    color: var(--fg-faint);
    border-radius: 999px;
    line-height: 1;
  }
  .history-btn.open .history-count {
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    color: var(--accent);
  }

  .history-popover {
    position: fixed;
    width: 420px;
    max-width: calc(100vw - 24px);
    height: 540px;
    max-height: calc(100vh - 120px);
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow:
      0 24px 60px rgba(0, 0, 0, 0.45),
      0 4px 12px rgba(0, 0, 0, 0.25);
    z-index: 50;
    display: flex; flex-direction: column;
    overflow: hidden;
    animation: history-pop-in 160ms cubic-bezier(.2,.7,.2,1);
    transform-origin: top right;
  }
  @keyframes history-pop-in {
    from { opacity: 0; transform: translateY(-4px) scale(0.98); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .history-popover { animation: none; }
  }

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

  .agents-pill {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 2px 8px;
    border-radius: 999px;
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 600;
    line-height: 1;
    background: var(--accent-soft);
    color: var(--accent);
    border: 1px solid color-mix(in oklch, var(--accent) 30%, var(--border));
    cursor: pointer;
    font-variant-numeric: tabular-nums;
    transition: background 120ms, border-color 120ms;
  }
  .agents-pill:hover {
    background: color-mix(in oklch, var(--accent) 22%, var(--surface));
    border-color: color-mix(in oklch, var(--accent) 55%, var(--border));
  }
  .agents-pill :global(svg) {
    opacity: 0.7;
  }
  .agents-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    animation: agents-pulse 1.4s ease-in-out infinite;
  }
  @keyframes agents-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .compact-warn {
    display: inline-flex; align-items: center;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    line-height: 1;
    background: var(--warn-soft);
    color: var(--warn);
    border: 1px solid color-mix(in oklch, var(--warn) 35%, var(--border));
    cursor: help;
  }
  .compact-warn[data-tone="red"] {
    background: var(--danger-soft, color-mix(in oklch, var(--danger) 12%, transparent));
    color: var(--danger);
    border-color: color-mix(in oklch, var(--danger) 40%, var(--border));
  }

  .compact-btn {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    line-height: 1;
    cursor: pointer;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .compact-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .compact-btn[data-tone="yellow"] {
    border-color: color-mix(in oklch, var(--warn) 35%, var(--border));
    color: var(--warn);
  }
  .compact-btn[data-tone="red"] {
    border-color: color-mix(in oklch, var(--danger) 45%, var(--border));
    background: color-mix(in oklch, var(--danger) 8%, transparent);
    color: var(--danger);
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
    cursor: pointer;
    font-family: inherit;
    font-variant-numeric: tabular-nums;
    transition: border-color 120ms ease-out, background 120ms ease-out;
  }
  .ctx-pill:hover { border-color: var(--border-strong); }
  .ctx-pill.open { border-color: color-mix(in oklch, var(--accent) 55%, var(--border)); }
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

  /* Context-detail popover — replaces the old wall-of-text tooltip. */
  .ctx-panel {
    position: fixed;
    width: 320px;
    max-width: calc(100vw - 24px);
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.45), 0 4px 12px rgba(0, 0, 0, 0.25);
    z-index: 50;
    padding: 14px;
    display: flex; flex-direction: column; gap: 10px;
    font-size: var(--fs-xs);
    animation: ctx-panel-in 140ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes ctx-panel-in {
    from { opacity: 0; transform: translateY(-6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) { .ctx-panel { animation: none; } }
  .ctx-panel-head { display: flex; align-items: center; gap: 8px; }
  .ctx-panel-dot {
    width: 8px; height: 8px; border-radius: 999px;
    background: var(--accent);
  }
  .ctx-panel-dot[data-tone="yellow"] { background: var(--warn); }
  .ctx-panel-dot[data-tone="red"] { background: var(--danger); }
  .ctx-panel-title { font-weight: 600; color: var(--fg); font-size: var(--fs-sm); }
  .ctx-panel-pct {
    margin-left: auto; font-weight: 700; font-variant-numeric: tabular-nums;
    color: var(--fg-2);
  }
  .ctx-panel-pct[data-tone="yellow"] { color: var(--warn); }
  .ctx-panel-pct[data-tone="red"] { color: var(--danger); }
  .ctx-panel-bar {
    height: 6px; border-radius: 999px; overflow: hidden;
    background: var(--surface-hover);
  }
  .ctx-panel-fill {
    display: block; height: 100%; border-radius: 999px;
    background: var(--accent); transition: width 200ms ease-out;
  }
  .ctx-panel-fill[data-tone="yellow"] { background: var(--warn); }
  .ctx-panel-fill[data-tone="red"] { background: var(--danger); }
  .ctx-panel-sub { color: var(--fg-muted); line-height: 1.4; }
  .ctx-panel-section { display: flex; flex-direction: column; gap: 3px; }
  .ctx-panel-section-label {
    font-size: 10px; font-weight: 700; letter-spacing: 0.04em;
    text-transform: uppercase; color: var(--fg-faint);
    margin-bottom: 1px;
  }
  .ctx-row {
    display: flex; align-items: baseline; justify-content: space-between; gap: 12px;
    color: var(--fg-2);
  }
  .ctx-row .num { color: var(--fg); font-variant-numeric: tabular-nums; }
  .ctx-panel-note { color: var(--fg-faint); line-height: 1.4; font-size: 10.5px; }
  .ctx-panel-empty { color: var(--fg-muted); padding: 4px 0; }
  .ctx-panel-foot {
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    padding-top: 10px;
  }
  .ctx-panel-meta { color: var(--fg-faint); font-size: 10.5px; }
  .ctx-panel-compact {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 4px 10px; border-radius: 8px;
    background: var(--bg-elev-2); border: 1px solid var(--border);
    color: var(--fg-2); cursor: pointer; font: inherit; font-size: var(--fs-xs);
    white-space: nowrap;
    transition: border-color 120ms ease-out, color 120ms ease-out, background 120ms ease-out;
  }
  .ctx-panel-compact:hover { border-color: var(--border-strong); color: var(--fg); }
  .ctx-panel-compact[data-tone="yellow"] { color: var(--warn); border-color: color-mix(in oklch, var(--warn) 35%, var(--border)); }
  .ctx-panel-compact[data-tone="red"] { color: var(--danger); border-color: color-mix(in oklch, var(--danger) 40%, var(--border)); }

  /* Segmented view control — Browser · Panel · Split grouped into one unit so
     the action cluster reads as a single block instead of three loose chips.
     Icon-only; tooltips carry the names. */
  .seg {
    display: inline-flex; align-items: center;
    gap: 2px;
    padding: 2px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 7px;
  }
  .seg-btn {
    display: inline-flex; align-items: center; gap: 5px;
    height: 22px; padding: 0 7px;
    background: transparent; border: 0; border-radius: 5px;
    color: var(--fg-muted); cursor: pointer;
    font: inherit; font-size: var(--fs-xs); line-height: 1;
    transition: background 120ms ease, color 120ms ease;
  }
  .seg-btn:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg); }
  .seg-btn.on { background: var(--accent-soft); color: var(--accent); }
  .seg-btn:disabled { opacity: 0.45; cursor: not-allowed; }
  .seg-btn.pulse { animation: dock-pulse 700ms ease-out; }

  @keyframes dock-pulse {
    0%   { box-shadow: 0 0 0 0 var(--accent-soft); }
    60%  { box-shadow: 0 0 0 6px transparent; }
    100% { box-shadow: 0 0 0 0 transparent; }
  }
  .seg-chip {
    font-size: 10px; font-weight: 700; line-height: 1;
    font-variant-numeric: tabular-nums;
    padding: 1px 4px; border-radius: 999px;
    background: color-mix(in oklch, var(--accent) 20%, transparent);
    color: var(--accent);
  }

  /* New-chat affordance sits flush after the last tab — browser convention
     puts the + there, not buried after the right-side action chips. Idle
     state is a subtle elevated pill so the eye picks it out of the strip
     without competing with the active tab. Hover lifts the accent tint. */
  .new-tab {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    margin: 4px 4px 0 6px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-muted);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    align-self: center;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease, transform 120ms ease;
  }
  .new-tab:hover {
    background: color-mix(in oklch, var(--accent) 18%, var(--bg-elev-2));
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
  }
  .new-tab:active { transform: scale(0.94); }
  .new-tab:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--ring);
  }
</style>
