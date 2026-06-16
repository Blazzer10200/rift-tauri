<script lang="ts">
  // v0.4 — Chat tabs bar. Browser-style tab strip + right-side chat status
  // chips (workspace, auth, ctx, tasks) + new-tab button. Mounted by AppShell
  // between the wire-error banner and the .body grid whenever the Chat
  // workspace is active.

  import { MessageSquare, Plus, X, PanelRight, FolderOpen, Folder, FolderGit2, GitBranch, SplitSquareHorizontal, History, ChevronDown, Globe, Check, ArrowUpCircle, Copy, ExternalLink, FileDiff, Loader2, Bot } from "lucide-svelte";
  import { onDestroy, onMount } from "svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { cliUpdate } from "../../state/cliUpdate.svelte";
  import { modelFamily } from "../../state/assistant/helpers";
  import { browserDock } from "../../state/browserDock.svelte";
  import { activityDock } from "../../state/activityDock.svelte";
  import { environmentDock } from "../../state/environmentDock.svelte";
  import OpenInPaneMenu from "../assistant/OpenInPaneMenu.svelte";
  import HistoryDrawer from "../assistant/HistoryDrawer.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  import { portalFocus as portal } from "$lib/actions/portal";
  import { menuKeydown, leafName, prettyPath, shortK } from "./tabsbar/helpers";
  let ctxMenu = $state<{ tabId: string; x: number; y: number } | null>(null);
  let historyOpen = $state(false);
  let historyFull = $state(false);
  let historyPopover = $state<HTMLDivElement | undefined>();
  let historyPos = $state<{ top: number; right: number }>({ top: 0, right: 0 });

  // Context-detail popover — the ctx-pill's tooltip dumped a wall of text;
  // this turns it into a clickable panel with the same breakdown laid out.
  let ctxPanelOpen = $state(false);
  let ctxAnchor = $state<HTMLButtonElement | undefined>();
  let ctxPanel = $state<HTMLDivElement | undefined>();
  let ctxPos = $state<{ top: number; right: number }>({ top: 0, right: 0 });

  // `/history` slash command requests the drawer via the store's one-shot flag.
  $effect(() => {
    if (!assistant.ui.historyOpen) return;
    assistant.ui.historyOpen = false;
    openHistory();
  });

  // History opens from the Panels menu now → anchor its popover off the
  // Panels button (viewAnchor).
  function openHistory() {
    if (!viewAnchor) return;
    const r = viewAnchor.getBoundingClientRect();
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
      if (viewAnchor?.contains(t)) return;
      if (historyPopover?.contains(t)) return;
      historyOpen = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { historyOpen = false; viewAnchor?.focus(); }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  });

  // Close the full-screen history modal on Escape (backdrop click already handled).
  $effect(() => {
    if (!historyFull) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") historyFull = false;
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });

  function toggleCtxPanel() {
    if (ctxPanelOpen) { ctxPanelOpen = false; return; }
    if (!ctxAnchor) return;
    const r = ctxAnchor.getBoundingClientRect();
    ctxPos = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right) };
    ctxPanelOpen = true;
  }

  // ── Claude Code CLI update notice ──────────────────────────────────────
  // Glanceable badge in the right cluster, shown only when npm has a newer
  // `claude` than the one Rift is spawning (and the user hasn't dismissed it).
  // Click opens a small popover with the version delta + copy-command + dismiss.
  let cliBadgeAnchor = $state<HTMLButtonElement | undefined>();
  let cliPanel = $state<HTMLDivElement | undefined>();
  let cliPanelOpen = $state(false);
  let cliPos = $state<{ top: number; right: number }>({ top: 0, right: 0 });
  const cliInstalled = $derived(assistant.auth?.cliVersion ?? null);
  const cliUpdateReady = $derived(cliUpdate.availableAny(assistant.auth?.installs, cliInstalled));
  const cliSummary = $derived(cliUpdate.summary(assistant.auth?.installs));
  onMount(() => { void cliUpdate.maybeCheck(); });
  $effect(() => { cliUpdate.setMethod(assistant.auth?.installMethod ?? null); });
  async function runCliUpdate() {
    const ok = await cliUpdate.runUpdate();
    if (ok) { await assistant.refreshAuth(); cliPanelOpen = false; }
  }
  function toggleCliPanel() {
    if (cliPanelOpen) { cliPanelOpen = false; return; }
    if (!cliBadgeAnchor) return;
    const r = cliBadgeAnchor.getBoundingClientRect();
    cliPos = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right) };
    cliPanelOpen = true;
  }
  $effect(() => {
    if (!cliPanelOpen) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (cliBadgeAnchor?.contains(t)) return;
      if (cliPanel?.contains(t)) return;
      cliPanelOpen = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { cliPanelOpen = false; cliBadgeAnchor?.focus(); }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  });
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

  // View dropdown — consolidates the panel/layout toggles (browser, activity
  // dock, split pane) into one clean menu, à la Claude Code desktop's
  // top-right options popover. Anchored to its trigger, portaled to <body>
  // to escape the .tabs-rail overflow clip.
  let viewMenuOpen = $state(false);
  let viewAnchor = $state<HTMLButtonElement | undefined>();
  let viewMenu = $state<HTMLDivElement | undefined>();
  let viewPos = $state<{ top: number; right: number }>({ top: 0, right: 0 });

  function openViewMenu() {
    if (!viewAnchor) return;
    const r = viewAnchor.getBoundingClientRect();
    viewPos = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right) };
    viewMenuOpen = true;
  }
  $effect(() => {
    if (!viewMenuOpen) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (viewAnchor?.contains(t)) return;
      if (viewMenu?.contains(t)) return;
      viewMenuOpen = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { viewMenuOpen = false; viewAnchor?.focus(); }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  });

  // Project pill — replaces the old folder-chip / "Open folder" button. Shows
  // the active workspace folder; the dropdown switches between recent roots,
  // opens a new folder, or closes the current one. Portaled to escape the
  // .tabs-rail overflow clip, same as the other popovers.
  let projMenuOpen = $state(false);
  let projAnchor = $state<HTMLButtonElement | undefined>();
  let projMenu = $state<HTMLDivElement | undefined>();
  let projPos = $state<{ top: number; right: number }>({ top: 0, right: 0 });

  function openProjMenu() {
    if (!projAnchor) return;
    const r = projAnchor.getBoundingClientRect();
    projPos = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right) };
    projMenuOpen = true;
  }
  $effect(() => {
    if (!projMenuOpen) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (projAnchor?.contains(t)) return;
      if (projMenu?.contains(t)) return;
      projMenuOpen = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { projMenuOpen = false; projAnchor?.focus(); }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  });
  // recent roots, current first removed (it's shown as the active row inline).
  const recentRoots = $derived(assistant.workspace.recent ?? []);
  // The project-folder control reflects + sets the FOCUSED tab's folder so each
  // pane keeps its own project dir. `activeRoot` = focused tab's effective root.
  const activeRoot = $derived(assistant.activeRoot);
  const activeTabId = $derived(assistant.currentConvoId);
  // Resolve the active workspace's git branch for the tab-bar chip (null = not a repo).
  $effect(() => {
    if (assistant.activeRoot) void assistant.loadWorkspaceBranch();
  });

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

  // Window-level dragend safety net + Escape-cancel guard + unmount cleanup.
  // WebView2 can miss 'dragend' on Escape, leaving drag state stuck. (#208)
  $effect(() => {
    const onEsc = (e: KeyboardEvent) => { if (e.key === "Escape" && dragFromIdx !== null) onDragEnd(); };
    window.addEventListener("dragend", onDragEnd);
    window.addEventListener("keydown", onEsc);
    return () => {
      window.removeEventListener("dragend", onDragEnd);
      window.removeEventListener("keydown", onEsc);
    };
  });
  onDestroy(onDragEnd);

  // -------- right-side chat status chips (absorbed from AssistantHeader) -----

  const authWarn = $derived.by(() => {
    const a = assistant.auth;
    if (!a) return null;
    if (a.pill === "yellow") return { tone: "yellow", text: "API key" };
    if (a.pill === "red") return { tone: "red", text: "Sign in" };
    return null;
  });

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
  // #30: a resumed tab stays pinned to its original folder — when that differs
  // from the selected workspace, badge it so "chip says X, turns run in Y" is
  // visible. Comparison is separator/case-insensitive (Windows paths).
  const normPath = (p: string) => p.replace(/[\\/]+$/, "").replace(/\\/g, "/").toLowerCase();
  const cwdMismatch = $derived.by(() => {
    const pinned = assistant.activeTab?.sessionCwd;
    // Compare against the focused tab's OWN folder (the proj chip shows it now),
    // not the global default — else every per-tab root reads as a mismatch.
    const ws = assistant.activeRoot;
    if (!pinned || !ws) return null;
    return normPath(pinned) === normPath(ws) ? null : pinned;
  });
  const activeAgents = $derived(
    (assistant.activeTab?.agentSpawns ?? []).filter((a) => a.completedAt === null),
  );
  const activeAgentTitle = $derived.by(() =>
    activeAgents.map((a) => `${a.subagentType}: ${a.description}`).join("\n"),
  );
  const ctxHighNudge = $derived(
    ctxPct >= 70
      ? `Context ${ctxPct >= 85 ? "nearly full — start a fresh chat soon" : "filling up"} (Ctrl+T)`
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
        ? "\n\nWindow nearly full. Ctrl+T starts a fresh chat."
        : ctxPct >= 70
          ? "\n\nApproaching the window cap — Ctrl+T starts a fresh chat."
          : "";
    return (
      `Context: ${ctxTokens.toLocaleString()} / ${ctxWindow.toLocaleString()} (${ctxPct.toFixed(1)}%) — the model's hard input cap.\n\n` +
      `This turn:\n` +
      `  • new (input + cache-create): ${newTokens.toLocaleString()}\n` +
      `  • cache-read: ${u.cacheRead.toLocaleString()} — replay of prior turns, still occupies the window every turn\n` +
      `  • output: ${u.output.toLocaleString()}\n\n` +
      `The cache only saves billing — those tokens are in the window every send, same as fresh input.\n\n` +
      `Session (${s.turns} turn${s.turns === 1 ? "" : "s"}): in ${s.totalInput.toLocaleString()} · out ${s.totalOutput.toLocaleString()} · cache ${s.totalCacheRead.toLocaleString()} r / ${s.totalCacheCreate.toLocaleString()} w${cost}\n` +
      `Model: ${assistant.lastModelId ?? "?"}${action}`
    );
  });
</script>

<div class="tabsbar" data-model={modelFamily(assistant.effectiveModel)} role="tablist" aria-label="Chat tabs">
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
          <span class="pane-badge" use:tooltip={`Open in pane ${paneIndexFor(id)}`}>{paneIndexFor(id)}</span>
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
    <div class="top-pop">
      <button
        class="proj-pill"
        class:open={projMenuOpen}
        type="button"
        bind:this={projAnchor}
        onclick={() => { projMenuOpen ? (projMenuOpen = false) : openProjMenu(); }}
        aria-haspopup="menu"
        aria-expanded={projMenuOpen}
        use:tooltip={activeRoot ? prettyPath(activeRoot) : "Open a project folder"}
      >
        <FolderGit2 size={14} class="proj-ico"/>
        <span class="proj-name">{activeRoot ? leafName(activeRoot) : "Open project"}</span>
        <ChevronDown size={13} class={projMenuOpen ? "proj-chev chev-open" : "proj-chev"}/>
      </button>
    </div>

    {#if cwdMismatch}
      <span
        class="cwd-badge"
        use:tooltip={`This chat runs in ${prettyPath(cwdMismatch)} — its session was started there and stays pinned to it. New chats use the selected workspace.`}
      >
        <FolderGit2 size={12} />
        <span class="cwd-name">{leafName(cwdMismatch)}</span>
      </span>
    {/if}

    {#if assistant.workspaceBranch}
      <span class="branch-chip mono" use:tooltip={`On branch ${assistant.workspaceBranch}`}>
        <GitBranch size={12} />
        <span class="branch-name">{assistant.workspaceBranch}</span>
      </span>
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

    {#if ctxHighNudge}
      <span
        class="compact-warn"
        data-tone={ctxPct >= 85 ? "red" : "yellow"}
        use:tooltip={"Cache-read tokens count toward the window — at 95% there's no headroom for the next turn. Ctrl+T starts a fresh chat."}
        >{ctxHighNudge}</span>
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

    {#if cliUpdateReady}
      <button
        type="button"
        class="cli-badge"
        class:open={cliPanelOpen}
        bind:this={cliBadgeAnchor}
        onclick={toggleCliPanel}
        aria-haspopup="dialog"
        aria-expanded={cliPanelOpen}
        use:tooltip={"Claude Code CLI update available — click for details"}
      >
        <ArrowUpCircle size={12} />
        <span class="cli-badge-t">CLI update</span>
      </button>
    {/if}

    <span class="vdiv" aria-hidden="true"></span>

    <button
      class="hdr-btn panels-btn"
      class:open={viewMenuOpen}
      type="button"
      bind:this={viewAnchor}
      onclick={() => { viewMenuOpen ? (viewMenuOpen = false) : openViewMenu(); }}
      aria-haspopup="menu"
      aria-expanded={viewMenuOpen}
      use:tooltip={"Panels & layout"}
    >
      <PanelRight size={13} />
      {#if browserDock.open || activityDock.open || splitActive}
        <span class="view-dot" aria-hidden="true"></span>
      {/if}
      <ChevronDown size={10} class={viewMenuOpen ? "chev-open" : ""} />
    </button>
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
    aria-modal="true"
    aria-label="Conversation history"
    bind:this={historyPopover}
    use:portal
    style="top: {historyPos.top}px; right: {historyPos.right}px;"
  >
    <HistoryDrawer compact onSelected={() => (historyOpen = false)} onExpand={() => { historyOpen = false; historyFull = true; }} />
  </div>
{/if}

{#if historyFull}
  <div class="history-full-scrim" use:portal>
    <button class="history-full-backdrop" type="button" aria-label="Close history" onclick={() => (historyFull = false)}></button>
    <div class="history-full-panel">
      <HistoryDrawer onSelected={() => (historyFull = false)} />
    </div>
  </div>
{/if}

{#if ctxPanelOpen}
  <div
    class="ctx-panel"
    role="dialog"
    aria-modal="true"
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
      <span class="ctx-panel-meta">{lastModelId ?? "—"}</span>
    </div>
  </div>
{/if}

{#if cliPanelOpen}
  <div
    class="cli-panel"
    role="dialog"
    aria-modal="true"
    aria-label="Claude Code CLI update"
    bind:this={cliPanel}
    use:portal
    style="top: {cliPos.top}px; right: {cliPos.right}px;"
  >
    <div class="cli-panel-head" data-tone={cliSummary.tone}>
      <span class="cli-panel-ic"><ArrowUpCircle size={15} /></span>
      <span class="cli-panel-title">{cliSummary.headline}</span>
    </div>
    <div class="cli-panel-vers">
      <span class="cli-vchip old">{cliInstalled ?? "?"}</span>
      <span class="cli-varrow">→</span>
      <span class="cli-vchip new">{cliUpdate.latest ?? "?"}</span>
    </div>
    <div class="cli-panel-sub" data-tone={cliSummary.tone}>{cliSummary.detail}</div>
    <button type="button" class="cli-update-go" disabled={cliUpdate.updating} onclick={runCliUpdate}>
      {#if cliUpdate.updating}<Loader2 size={14} class="cli-spin" /> Updating…{:else}<ArrowUpCircle size={14} /> Update now{/if}
    </button>
    <div class="cli-cmd">
      <code>{cliUpdate.updateCommand}</code>
      <button
        type="button"
        class="cli-cmd-copy"
        class:done={cliUpdate.copied}
        onclick={() => void cliUpdate.copyCommand()}
        use:tooltip={"Copy update command"}
        aria-label="Copy update command"
      >
        {#if cliUpdate.copied}<Check size={13} />{:else}<Copy size={13} />{/if}
      </button>
    </div>
    <div class="cli-panel-foot">
      <a class="cli-panel-link" href={cliUpdate.changelogUrl} target="_blank" rel="noreferrer">
        <ExternalLink size={11} /> What's new
      </a>
      <button type="button" class="cli-panel-dismiss" onclick={() => { cliUpdate.dismiss(); cliPanelOpen = false; }}>
        Dismiss
      </button>
    </div>
  </div>
{/if}

{#if projMenuOpen}
  <div
    class="rift-menu proj-menu"
    role="menu"
    tabindex="-1"
    aria-label="Switch project folder"
    bind:this={projMenu}
    onkeydown={(e) => menuKeydown(e, projMenu)}
    use:portal
    style="top: {projPos.top}px; right: {projPos.right}px;"
  >
    <div class="rift-menu-head">Project folder</div>
    {#each recentRoots as path (path)}
      <button
        class="rift-menu-row"
        class:current={path === activeRoot}
        type="button"
        role="menuitem"
        use:tooltip={prettyPath(path)}
        onclick={() => { projMenuOpen = false; if (path !== activeRoot) void assistant.setTabRoot(activeTabId, path); }}
      >
        <Folder size={15} class="rift-menu-row-ic" />
        <span class="rift-menu-row-body">
          <span class="rift-menu-row-t">{leafName(path)}</span>
        </span>
        {#if path === activeRoot}<Check size={14} class="rift-menu-row-chk" />{/if}
      </button>
    {/each}
    {#if recentRoots.length > 0}<div class="rift-menu-divider" role="separator"></div>{/if}
    <button class="rift-menu-row" type="button" role="menuitem" onclick={() => { projMenuOpen = false; void assistant.pickTabFolder(activeTabId); }}>
      <FolderOpen size={15} class="rift-menu-row-ic" />
      <span class="rift-menu-row-body"><span class="rift-menu-row-t">Open folder…</span></span>
    </button>
    {#if assistant.activeTab?.workspaceRoot}
      <button class="rift-menu-row" type="button" role="menuitem" onclick={() => { projMenuOpen = false; if (assistant.activeTab) assistant.activeTab.workspaceRoot = null; assistant.workspaceFiles = []; assistant.workspaceBranch = null; }}>
        <X size={15} class="rift-menu-row-ic" />
        <span class="rift-menu-row-body"><span class="rift-menu-row-t">Use default folder</span></span>
      </button>
    {/if}
  </div>
{/if}

{#if viewMenuOpen}
  <div
    class="rift-menu view-menu"
    role="menu"
    tabindex="-1"
    aria-label="Panels &amp; layout"
    bind:this={viewMenu}
    onkeydown={(e) => menuKeydown(e, viewMenu)}
    use:portal
    style="top: {viewPos.top}px; right: {viewPos.right}px;"
  >
    <button
      class="vm-item"
      type="button"
      role="menuitem"
      onclick={() => { viewMenuOpen = false; openHistory(); }}
    >
      <History size={14} class="vm-icon" />
      <span class="vm-label">History</span>
      {#if assistant.conversations.length > 0}<span class="vm-count">{assistant.conversations.length}</span>{/if}
      <Check size={13} class="vm-check" aria-hidden="true" />
    </button>
    <div class="vm-sep" role="separator"></div>
    <button
      class="vm-item"
      class:on={browserDock.open}
      type="button"
      role="menuitemcheckbox"
      aria-checked={browserDock.open}
      onclick={() => { browserDock.toggle(); viewMenuOpen = false; }}
    >
      <Globe size={14} class="vm-icon" />
      <span class="vm-label">Web browser</span>
      <kbd class="vm-kbd">Ctrl&nbsp;⇧&nbsp;B</kbd>
      <Check size={13} class="vm-check" />
    </button>
    <button
      class="vm-item"
      class:on={activityDock.open}
      type="button"
      role="menuitemcheckbox"
      aria-checked={activityDock.open}
      onclick={() => { activityDock.toggle(); viewMenuOpen = false; }}
    >
      <Bot size={14} class="vm-icon" />
      <span class="vm-label">Sub-agents</span>
      {#if (assistant.activeTab?.agentSpawns.length ?? 0) > 0}<span class="vm-count">{assistant.activeTab?.agentSpawns.length}</span>{/if}
      <Check size={13} class="vm-check" />
    </button>
    <button
      class="vm-item"
      class:on={assistant.ui.diffOpen}
      type="button"
      role="menuitemcheckbox"
      aria-checked={assistant.ui.diffOpen}
      onclick={() => { assistant.ui.diffTarget = null; assistant.ui.diffOpen = !assistant.ui.diffOpen; viewMenuOpen = false; }}
    >
      <FileDiff size={14} class="vm-icon" />
      <span class="vm-label">Session diff</span>
      <kbd class="vm-kbd">Ctrl&nbsp;⇧&nbsp;D</kbd>
      <Check size={13} class="vm-check" />
    </button>
    <button
      class="vm-item"
      class:on={environmentDock.open}
      type="button"
      role="menuitemcheckbox"
      aria-checked={environmentDock.open}
      onclick={() => { environmentDock.toggle(); viewMenuOpen = false; }}
    >
      <GitBranch size={14} class="vm-icon" />
      <span class="vm-label">Environment</span>
      <Check size={13} class="vm-check" />
    </button>
    <div class="vm-sep" role="separator"></div>
    <button
      class="vm-item"
      type="button"
      role="menuitem"
      disabled={!canAddPane}
      onclick={() => { assistant.addPane(); viewMenuOpen = false; }}
    >
      <SplitSquareHorizontal size={14} class="vm-icon" />
      <span class="vm-label">{canAddPane ? "Split pane" : "Max panes"}</span>
      {#if splitActive}<span class="vm-count">{paneCount}/4</span>{/if}
      <kbd class="vm-kbd">Ctrl&nbsp;\</kbd>
      <Check size={13} class="vm-check" aria-hidden="true" />
    </button>
  </div>
{/if}

<style>
  .tabsbar {
    position: relative;
    height: 36px;
    flex-shrink: 0;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: stretch;
    overflow: hidden;
  }
  /* Emerald-only — the tabs bar no longer tints by model, matching the
     composer ring. Model identity lives on the picker model-card swatch. */
  .tabsbar                      { --model-color: var(--accent); }

  .strip {
    flex: 1; min-width: 0;
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 0 8px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }
  .strip::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .strip::-webkit-scrollbar-button { display: none; }

  .tab {
    flex: 0 1 210px;
    min-width: 130px;
    max-width: 220px;
    height: 26px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 8px;
    margin: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 7px;
    color: var(--fg-muted);
    cursor: pointer;
    font-size: var(--fs-sm);
    font-weight: 500;
    user-select: none;
    transition: background 120ms var(--ease-soft), color 120ms var(--ease-soft), border-color 120ms var(--ease-soft);
    position: relative;
    animation: tab-in 220ms var(--ease-page);
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
    z-index: 1;
  }
  .tab.active .icon { color: var(--model-color); }
  .tab.active .dot { background: var(--model-color); box-shadow: 0 0 8px color-mix(in oklch, var(--model-color) 60%, transparent); }
  .tab.drop-target {
    box-shadow: -2px 0 0 var(--accent);
  }
  /* Background-streaming tab (streaming + NOT active): subtle accent tint
     + animated underline pulse. Visible-but-quiet — the user should be able
     to scan the strip and see "Tab 2 is busy in background" without it
     stealing attention from whatever they're typing in the active tab. */
  .tab.bg-streaming {
    background: color-mix(in oklab, var(--accent) 8%, var(--bg));
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
    background: color-mix(in oklab, var(--accent) 45%, transparent);
    opacity: 0.7;
  }
  .pane-badge {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 14px; height: 14px;
    padding: 0 4px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 18%, var(--bg-elev-2));
    color: color-mix(in oklab, var(--accent) 80%, var(--fg));
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
    transition: opacity 120ms var(--ease-soft), background 120ms var(--ease-soft), color 120ms var(--ease-soft);
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
    /* Inter-zone gap. Items inside a .grp sit tighter (gap:5) so each zone
       reads as one unit; zones breathe apart at 9px. */
    gap: 9px;
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

  /* Hairline between the transient/nav zone and the persistent view-toggle
     segment — mirrors .actions::before so the cluster parses left-to-right
     as [context · status │ view]. */
  .vdiv {
    width: 1px;
    align-self: stretch;
    margin: 6px 0;
    background: linear-gradient(180deg,
      transparent,
      color-mix(in oklch, var(--border) 80%, transparent) 30%,
      color-mix(in oklch, var(--border) 80%, transparent) 70%,
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
    transition: background 120ms var(--ease-soft), color 120ms var(--ease-soft), border-color 120ms var(--ease-soft);
  }
  .hdr-btn:hover { color: var(--fg); border-color: var(--border-strong); background: var(--surface-hover); }

  /* ── Project pill — folder-git icon · name · chevron (mockup `.proj-pill`).
     Replaces the old folder-chip + Open-folder + History trio. The dropdown
     switches between recent roots. */
  .top-pop { position: relative; display: inline-flex; }
  .proj-pill {
    display: inline-flex; align-items: center; gap: 8px;
    height: 26px; padding: 0 9px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--fg);
    cursor: pointer;
    font: inherit; font-size: var(--fs-xs);
    max-width: 240px;
    transition: background 120ms var(--ease-soft), border-color 120ms var(--ease-soft);
  }
  .proj-pill:hover, .proj-pill.open {
    background: var(--surface-hover);
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border));
  }
  .proj-pill :global(.proj-ico) { color: var(--accent); flex-shrink: 0; }
  .proj-name {
    font-weight: 600;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .proj-pill :global(.proj-chev) { color: var(--fg-faint); flex-shrink: 0; transition: transform 180ms var(--ease-page), color 140ms ease; }
  .proj-pill :global(.chev-open) { transform: rotate(180deg); color: var(--fg-muted); }

  /* ── Branch chip — read-only git branch beside the project pill (mockup `⎇ main`). */
  .branch-chip {
    display: inline-flex; align-items: center; gap: 5px;
    height: 26px; padding: 0 8px;
    font-size: 11px; color: var(--fg-muted);
    max-width: 160px;
  }
  .branch-chip :global(svg) { color: var(--fg-faint); flex-shrink: 0; }
  .branch-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* #30: active tab pinned to a different folder than the workspace chip. */
  .cwd-badge {
    display: inline-flex; align-items: center; gap: 5px;
    height: 22px; padding: 0 8px;
    font-size: 11px; font-weight: 600;
    color: var(--warn);
    background: color-mix(in oklab, var(--warn) 10%, transparent);
    border: 1px solid color-mix(in oklab, var(--warn) 32%, transparent);
    border-radius: 999px;
    max-width: 180px;
  }
  .cwd-badge :global(svg) { flex-shrink: 0; }
  .cwd-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* ── Project dropdown — recent roots + open/close. Inherits .rift-menu chrome
     + .rift-menu-row rows (app.css); this only carries positioning + sizing. */
  .proj-menu {
    position: fixed;
    z-index: 50;
    min-width: 208px; max-width: 280px;
    display: flex; flex-direction: column; gap: 1px;
    animation: history-pop-in 150ms var(--ease-page);
    transform-origin: top right;
  }
  @media (prefers-reduced-motion: reduce) { .proj-menu { animation: none; } }

  .history-full-scrim {
    position: fixed;
    inset: 0;
    z-index: 300;
    display: grid;
    place-items: center;
    padding: 5vh 4vw;
    background: color-mix(in oklch, black 55%, transparent);
    animation: history-full-in var(--dur-page) var(--ease-page);
  }
  .history-full-backdrop {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
    cursor: default;
  }
  .history-full-panel {
    position: relative;
    z-index: 1;
    width: min(1100px, 100%);
    height: min(740px, 100%);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
  }
  .history-full-panel :global(.drawer) {
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  @keyframes history-full-in {
    from { opacity: 0; }
    to { opacity: 1; }
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
    animation: history-pop-in 160ms var(--ease-page);
    transform-origin: top right;
  }
  @keyframes history-pop-in {
    from { opacity: 0; transform: translateY(-4px) scale(0.98); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .history-popover { animation: none; }
  }

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
    border-color: color-mix(in oklab, var(--warn) 35%, var(--border));
    background: var(--warn-soft);
  }
  .auth-warn[data-tone="yellow"] .auth-dot { background: var(--warn); }
  .auth-warn[data-tone="red"] {
    color: var(--danger);
    border-color: color-mix(in oklab, var(--danger) 35%, var(--border));
    background: color-mix(in oklab, var(--danger) 10%, transparent);
  }
  .auth-warn[data-tone="red"] .auth-dot { background: var(--danger); }

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
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
    cursor: pointer;
    font-variant-numeric: tabular-nums;
    transition: background 120ms var(--ease-soft), border-color 120ms var(--ease-soft);
  }
  .agents-pill:hover {
    background: color-mix(in oklab, var(--accent) 22%, var(--surface));
    border-color: color-mix(in oklab, var(--accent) 55%, var(--border));
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
    border: 1px solid color-mix(in oklab, var(--warn) 35%, var(--border));
    cursor: help;
  }
  .compact-warn[data-tone="red"] {
    background: var(--danger-soft, color-mix(in oklab, var(--danger) 12%, transparent));
    color: var(--danger);
    border-color: color-mix(in oklab, var(--danger) 40%, var(--border));
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
    transition: border-color 120ms var(--ease-soft), background 120ms var(--ease-soft);
  }
  .ctx-pill:hover { border-color: var(--border-strong); }
  .ctx-pill.open { border-color: color-mix(in oklab, var(--accent) 55%, var(--border)); }
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
    border-color: color-mix(in oklab, var(--warn) 35%, var(--border));
    background: var(--warn-soft); color: var(--warn);
  }
  .ctx-pill[data-tone="yellow"] .ctx-text { color: var(--warn); }
  .ctx-pill[data-tone="yellow"] .ctx-fill { background: var(--warn); }
  .ctx-pill[data-tone="red"] {
    border-color: color-mix(in oklab, var(--danger) 35%, var(--border));
    background: color-mix(in oklab, var(--danger) 10%, transparent);
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
    animation: ctx-panel-in 140ms var(--ease-page);
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

  /* Claude Code CLI update — accent-tinted notice pill + detail popover. */
  .cli-badge {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 9px; border-radius: 999px;
    font-size: var(--fs-xs); font-weight: 600; line-height: 1;
    color: var(--accent); cursor: pointer; font-family: inherit;
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 38%, var(--border));
    transition: background 120ms var(--ease-soft), border-color 120ms var(--ease-soft);
    animation: cli-badge-in 220ms var(--ease-page);
  }
  .cli-badge:hover, .cli-badge.open {
    background: color-mix(in oklab, var(--accent) 20%, transparent);
    border-color: color-mix(in oklab, var(--accent) 60%, var(--border));
  }
  .cli-badge :global(svg) { flex-shrink: 0; }
  @keyframes cli-badge-in {
    from { opacity: 0; transform: scale(0.9); }
    to   { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) { .cli-badge { animation: none; } }

  .cli-panel {
    position: fixed;
    width: 300px;
    max-width: calc(100vw - 24px);
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.45), 0 4px 12px rgba(0, 0, 0, 0.25);
    z-index: 50;
    padding: 14px;
    display: flex; flex-direction: column; gap: 10px;
    font-size: var(--fs-xs);
    animation: ctx-panel-in 140ms var(--ease-page);
  }
  @media (prefers-reduced-motion: reduce) { .cli-panel { animation: none; } }
  .cli-panel-head { display: flex; align-items: center; gap: 8px; }
  .cli-panel-ic { display: inline-flex; color: var(--accent); }
  .cli-panel-head[data-tone="warn"] .cli-panel-ic { color: var(--warn); }
  .cli-panel-head[data-tone="danger"] .cli-panel-ic { color: var(--danger); }
  .cli-panel-title { font-weight: 600; color: var(--fg); font-size: var(--fs-sm); }
  .cli-panel-vers {
    display: flex; align-items: center; gap: 8px;
    font-variant-numeric: tabular-nums;
  }
  .cli-vchip {
    padding: 2px 8px; border-radius: 6px; font-family: var(--font-mono);
    font-size: 11px; border: 1px solid var(--border);
  }
  .cli-vchip.old { color: var(--fg-muted); background: var(--bg-inset); }
  .cli-vchip.new {
    color: var(--accent); font-weight: 600;
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border-color: color-mix(in oklab, var(--accent) 40%, var(--border));
  }
  .cli-varrow { color: var(--fg-faint); }
  .cli-panel-sub { color: var(--fg-muted); line-height: 1.45; }
  .cli-panel-sub[data-tone="warn"] { color: var(--warn); }
  .cli-panel-sub[data-tone="danger"] { color: var(--danger); word-break: break-word; }
  .cli-cmd {
    display: flex; align-items: center; gap: 8px;
    background: color-mix(in oklch, white 9%, var(--surface)); border: 1px solid var(--border-strong);
    border-radius: 8px; padding: 7px 8px 7px 10px;
  }
  .cli-cmd code {
    flex: 1; min-width: 0; font-family: var(--font-mono); font-size: 11px;
    color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cli-cmd-copy {
    flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center;
    width: 26px; height: 24px; border-radius: 6px;
    border: 1px solid var(--border); background: var(--surface);
    color: var(--fg-muted); cursor: pointer;
    transition: color 120ms var(--ease-soft), border-color 120ms var(--ease-soft), background 120ms var(--ease-soft);
  }
  .cli-cmd-copy:hover { color: var(--fg); border-color: var(--border-strong); }
  .cli-cmd-copy.done { color: var(--accent); border-color: color-mix(in oklab, var(--accent) 50%, var(--border)); }
  .cli-update-go {
    display: inline-flex; align-items: center; justify-content: center; gap: 6px;
    height: 32px; border-radius: 8px; border: 1px solid transparent; cursor: pointer;
    background: var(--accent); color: var(--accent-fg); font: inherit; font-size: 12px; font-weight: 650;
    transition: background 130ms var(--ease-soft), opacity 130ms var(--ease-soft);
  }
  .cli-update-go:hover:not(:disabled) { background: var(--accent-hover); }
  .cli-update-go:disabled { opacity: 0.7; cursor: default; }
  .cli-update-go :global(.cli-spin) { animation: cli-spin 0.8s linear infinite; }
  @keyframes cli-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .cli-update-go :global(.cli-spin) { animation: none; } }
  .cli-panel-foot {
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    padding-top: 10px;
  }
  .cli-panel-link {
    display: inline-flex; align-items: center; gap: 5px;
    color: var(--fg-muted); text-decoration: none; font-size: 11px;
    transition: color 120ms var(--ease-soft);
  }
  .cli-panel-link:hover { color: var(--accent); }
  .cli-panel-dismiss {
    padding: 4px 10px; border-radius: 8px;
    background: var(--bg-elev-2); border: 1px solid var(--border);
    color: var(--fg-2); cursor: pointer; font: inherit; font-size: var(--fs-xs);
    transition: border-color 120ms var(--ease-soft), color 120ms var(--ease-soft);
  }
  .cli-panel-dismiss:hover { border-color: var(--border-strong); color: var(--fg); }

  /* Segmented view control — Browser · Panel · Split grouped into one unit so
     the action cluster reads as a single block instead of three loose chips.
     Icon-only; tooltips carry the names. */
  /* View dropdown trigger — one panel button + chevron replaces the old
     3-icon segmented control. A small accent dot signals "some panel open"
     without opening the menu. */
  .panels-btn {
    gap: 4px;
    padding: 3px 7px;
    border-radius: 7px;
  }
  .panels-btn.open {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 40%, var(--border));
  }
  .panels-btn :global(svg) { transition: transform 140ms ease; }
  .panels-btn :global(.chev-open) { transform: rotate(180deg); }
  .view-dot {
    width: 5px; height: 5px;
    border-radius: 999px;
    background: var(--accent);
    box-shadow: 0 0 6px color-mix(in oklab, var(--accent) 60%, transparent);
  }

  /* View popover — Rift-styled take on the Claude Code desktop options menu:
     icon · label · right-aligned shortcut · trailing check for active toggles. */
  .view-menu {
    position: fixed;
    z-index: 50;
    min-width: 230px;
    display: flex; flex-direction: column; gap: 1px;
    animation: history-pop-in 150ms var(--ease-page);
    transform-origin: top right;
  }
  @media (prefers-reduced-motion: reduce) {
    .view-menu { animation: none; }
  }
  .vm-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 7px 9px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
    transition: background 110ms var(--ease-soft), color 110ms var(--ease-soft);
  }
  .vm-item:hover:not(:disabled) { background: var(--surface-hover); }
  .vm-item:disabled { color: var(--fg-subtle); cursor: not-allowed; }
  .vm-item.on { color: var(--accent); }
  /* Signature left emerald bar on the active/current row (design-system motif). */
  .vm-item.on::before {
    content: ""; position: absolute; left: 1px; top: 6px; bottom: 6px; width: 2.5px;
    border-radius: 0 3px 3px 0; background: var(--accent);
    box-shadow: 0 0 8px color-mix(in oklab, var(--accent) 45%, transparent);
  }
  .vm-item :global(.vm-icon) { color: var(--fg-muted); flex-shrink: 0; }
  .vm-item.on :global(.vm-icon) { color: var(--accent); }
  .vm-label { flex: 1; white-space: nowrap; }
  .vm-count {
    font-size: 10px; font-weight: 700; line-height: 1;
    font-variant-numeric: tabular-nums;
    padding: 1px 5px; border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
  }
  .vm-kbd {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-faint);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 5px;
    line-height: 1;
    white-space: nowrap;
  }
  /* The check column always reserves space (auto) so rows align; the glyph
     only paints for active toggles. menuitem rows (Split) have no check. */
  .vm-item :global(.vm-check) {
    color: var(--accent);
    opacity: 0;
    transition: opacity 110ms ease;
  }
  .vm-item.on :global(.vm-check) { opacity: 1; }
  .vm-sep {
    height: 1px;
    margin: 4px 6px;
    background: color-mix(in oklch, var(--border) 70%, transparent);
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
    height: 26px;
    margin: 0 2px 0 3px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-muted);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    align-self: center;
    transition: background 120ms var(--ease-soft), color 120ms var(--ease-soft), border-color 120ms var(--ease-soft), transform 120ms var(--ease-soft);
  }
  .new-tab:hover {
    background: color-mix(in oklab, var(--accent) 18%, var(--bg-elev-2));
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 40%, var(--border));
  }
  .new-tab:active { transform: scale(0.94); }
  .new-tab:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--ring);
  }
</style>
