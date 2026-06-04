<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, Plus, X } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import AssistantWelcome from "./AssistantWelcome.svelte";
  import Composer from "./Composer.svelte";
  import SidePanel from "./SidePanel.svelte";
  import SessionDiff from "./SessionDiff.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  import { DOCK_MIN, DOCK_MAX, saveDockWidth } from "../../state/assistant/helpers";
  let {
    tabId,
    focused,
    paneIdx,
  }: { tabId: string | null; focused: boolean; paneIdx: number } = $props();

  const tab = $derived(assistant.tabFor(tabId));
  const messages = $derived(tab?.messages ?? []);
  const streaming = $derived(tab?.streaming ?? false);
  const lastError = $derived(tab?.lastError ?? null);
  const showEmpty = $derived(messages.length === 0);
  const needsAuth = $derived(assistant.auth?.pill === "red");
  // Notices are session-global; only show on the focused pane to avoid dup banners.
  const showNotice = $derived(focused && !!assistant.lastNotice);
  // Per-tab error renders in whichever pane owns the erroring tab, focused or
  // not — otherwise a background-pane send-failure is silent until refocus.
  const showError = $derived(!!lastError);
  // Open whenever the user toggled the panel and a tab exists — the panel now
  // has an Activity tab that always has something to show (live + empty-state),
  // so the old "only if there are context signals" gate no longer applies.
  const dockOpen = $derived(assistant.ui.dockOpen && !!tab);

  // Per-pane status chip — own tab's ctx%, model, cost — independent of focus.
  const paneCtxPct = $derived(tab ? assistant.ctxPctFor(tab) : 0);
  const paneCtxTone = $derived(
    paneCtxPct >= 90 ? "red" : paneCtxPct >= 70 ? "yellow" : "ok",
  );
  const paneModel = $derived(tab?.lastModelId ?? null);
  const paneCost = $derived(tab?.totalCostUsd ?? null);
  const paneChipTitle = $derived.by(() => {
    if (!tab) return "";
    const w = assistant.ctxWindowFor(tab);
    const used = Math.round((paneCtxPct / 100) * w);
    const lines = [
      `Ctx: ${used.toLocaleString()} / ${w.toLocaleString()} (${paneCtxPct.toFixed(1)}%)`,
    ];
    if (paneModel) lines.push(`Model: ${paneModel}`);
    if (paneCost != null) lines.push(`Cost: ${paneCost.toFixed(4)}`);
    return lines.join("\n");
  });

  let scrollEl = $state<HTMLDivElement | undefined>();
  let messagesEl = $state<HTMLDivElement | undefined>();
  let stickToBottom = $state(true);
  let scrolledTop = $state(false);
  let lastTabId: string | null = null;

  // Dock resize — drag the handle on the dock's left edge. Dragging left widens
  // (the dock is right-anchored). Width is app-global (assistant.ui.dockWidth),
  // clamped + persisted on release.
  let resizing = $state(false);
  function startResize(e: PointerEvent) {
    e.preventDefault();
    resizing = true;
    const startX = e.clientX;
    const startW = assistant.ui.dockWidth;
    const onMove = (ev: PointerEvent) => {
      const next = startW + (startX - ev.clientX);
      assistant.ui.dockWidth = Math.min(DOCK_MAX, Math.max(DOCK_MIN, next));
    };
    const onUp = () => {
      resizing = false;
      saveDockWidth(assistant.ui.dockWidth);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }
  function resetDockWidth() {
    assistant.ui.dockWidth = 300;
    saveDockWidth(300);
  }

  function onScroll() {
    if (!scrollEl) return;
    const gap = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
    stickToBottom = gap < 80;
    scrolledTop = scrollEl.scrollTop > 8;
    if (tabId) assistant.setTabScroll(tabId, scrollEl.scrollTop);
  }

  $effect(() => {
    if (!messagesEl) return;
    const ro = new ResizeObserver(() => {
      // Background-pane streams: only force-stick when the user was already
      // at the bottom — don't yank the scroll on a pane they're reading
      // mid-history just because a sibling pane is streaming.
      if (scrollEl && stickToBottom) scrollEl.scrollTop = scrollEl.scrollHeight;
    });
    ro.observe(messagesEl);
    return () => ro.disconnect();
  });

  $effect(() => {
    const _len = messages.length;
    const _streaming = streaming;
    void _len; void _streaming;
    void tick().then(() => {
      if (scrollEl && stickToBottom) scrollEl.scrollTop = scrollEl.scrollHeight;
    });
  });

  $effect(() => {
    const cur = tabId;
    if (cur === lastTabId) return;
    lastTabId = cur;
    if (!cur || !scrollEl) return;
    const cached = assistant.getTabScroll(cur);
    void tick().then(() => {
      if (!scrollEl) return;
      if (cached != null) {
        scrollEl.scrollTo({ top: cached, behavior: "smooth" });
        const gap = scrollEl.scrollHeight - cached - scrollEl.clientHeight;
        stickToBottom = gap < 80;
      } else {
        scrollEl.scrollTop = scrollEl.scrollHeight;
        stickToBottom = true;
      }
    });
  });

  function jumpToLatest() {
    if (!scrollEl) return;
    scrollEl.scrollTo({ top: scrollEl.scrollHeight, behavior: "smooth" });
    stickToBottom = true;
  }

  function onPaneClick() {
    if (!focused) assistant.setFocusedPane(paneIdx);
  }

  // ── drag-from-tabsbar drop targets ───────────────────────────────────────
  // Pane is the sole drop target — overlays are purely visual. This avoids
  // the Chromium "not allowed" cursor that latches when dragover briefly
  // passes over an ancestor without a preventDefault chain (e.g. .layout
  // between .tabsbar and .pane). One container, continuous preventDefault.
  //
  // Half detection: in single-pane mode we compute which half of the pane
  // the cursor is over from `e.clientX` so drop assigns to pane 0 (left) or
  // pane 1 (right). In split mode the whole pane is one zone tied to
  // `paneIdx`.
  const dragging = $derived(assistant.draggingTabId);
  let hoverHalf = $state<"left" | "right" | "full" | null>(null);
  let paneEl = $state<HTMLDivElement | undefined>();

  function computeHalf(e: DragEvent): "left" | "right" | "full" {
    if (assistant.splitActive) return "full";
    const target = paneEl;
    if (!target) return "left";
    const rect = target.getBoundingClientRect();
    return e.clientX < rect.left + rect.width / 2 ? "left" : "right";
  }

  function onClosePane(e: MouseEvent) {
    e.stopPropagation();
    assistant.closePane(paneIdx);
  }

  // Empty-pane actions — surface New chat / Close pane / Recent picks so
  // an unassigned slot is actionable instead of just saying "No tab in this
  // pane". Each handler focuses this pane first so newTab / openTab assigns
  // here (both rely on focusedPaneIdx via assignFocusedPane).
  function onEmptyNew() {
    if (!focused) assistant.setFocusedPane(paneIdx);
    void assistant.newTab();
  }
  function onEmptyClosePane() {
    assistant.closePane(paneIdx);
  }
  function onEmptyOpenRecent(id: string) {
    if (!focused) assistant.setFocusedPane(paneIdx);
    void assistant.openTab(id);
  }
  // Skip convos already visible in another pane — moving a tab cross-pane
  // from this picker would yank it from a pane the user can see.
  const emptyRecents = $derived.by(() => {
    if (tabId) return [];
    const inPanes = new Set(
      assistant.panes.map((p) => p.tabId).filter((x): x is string => !!x),
    );
    return assistant.conversations
      .filter((c) => !inPanes.has(c.id))
      .slice(0, 3);
  });

  function onPaneDragOver(e: DragEvent) {
    // Always preventDefault when a tab is being dragged — without this on
    // BOTH dragenter and dragover, Chromium shows the "no-drop" cursor.
    if (!assistant.draggingTabId) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    const next = computeHalf(e);
    if (hoverHalf !== next) hoverHalf = next;
  }

  function onPaneDragLeave(e: DragEvent) {
    // Only clear if the cursor actually left the pane (not just crossed
    // into a child element). relatedTarget is null when leaving the
    // browser window or moving outside any tracked element.
    const to = e.relatedTarget as Node | null;
    if (to && paneEl && paneEl.contains(to)) return;
    hoverHalf = null;
  }

  function onPaneDrop(e: DragEvent) {
    if (!assistant.draggingTabId) return;
    e.preventDefault();
    const id = assistant.draggingTabId;
    const half = computeHalf(e);
    hoverHalf = null;
    if (!id) return;
    const targetPane: number = assistant.splitActive
      ? paneIdx
      : half === "left" ? 0 : 1;
    assistant.dropTabIntoPane(id, targetPane);
  }
</script>

<div class="pane-shell" class:split={assistant.splitActive}>
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  bind:this={paneEl}
  class="pane"
  class:focused
  class:split={assistant.splitActive}
  role="region"
  aria-label={focused ? "Focused chat pane" : "Inactive chat pane"}
  onclick={onPaneClick}
  onkeydown={(e) => { if (e.key === "Enter") onPaneClick(); }}
  ondragenter={onPaneDragOver}
  ondragover={onPaneDragOver}
  ondragleave={onPaneDragLeave}
  ondrop={onPaneDrop}
  tabindex={focused ? -1 : 0}
>
  <!-- Atmosphere layer — faint static accent top-glow + film grain. Calmed
       (no breathing) so the backdrop stays quiet behind an assistant terminal;
       accent-only vocabulary, no drift, no motion. -->
  <div class="atmos" aria-hidden="true">
    <span class="atmos-glow"></span>
    <span class="atmos-grain"></span>
  </div>
  {#if assistant.splitActive}
    <div class="pane-chrome" aria-hidden="true">
      <span class="pane-label" use:tooltip={"Pane {paneIdx + 1} of {assistant.panes.length}"}>{paneIdx + 1}</span>
      {#if tabId && tab}
        <span class="pane-ctx-chip" data-tone={paneCtxTone} use:tooltip={paneChipTitle}>
          <span class="pane-ctx-bar"><span class="pane-ctx-fill" style="width: {Math.min(100, paneCtxPct)}%"></span></span>
          <span class="pane-ctx-pct">{Math.round(paneCtxPct)}%</span>
          {#if paneCost != null}
            <span class="pane-cost">${paneCost.toFixed(2)}</span>
          {/if}
        </span>
      {/if}
      <button
        class="pane-close"
        type="button"
        use:tooltip={"Close this pane (Ctrl+Shift+\\)"}
        aria-label="Close pane"
        onclick={onClosePane}
      >
        <X size={11}/>
      </button>
    </div>
  {/if}

  <div class="scroll" bind:this={scrollEl} onscroll={onScroll}>
    {#if !tabId}
      <div class="pane-empty">
        <div class="pane-empty-card">
          <div class="pane-empty-title">Empty pane</div>
          <div class="pane-empty-hint">
            Start a chat, drag a tab from the bar, or close the pane.
          </div>
          <div class="pane-empty-actions">
            <button class="btn primary sm" type="button" onclick={onEmptyNew}>
              <Plus size={12}/> New chat
            </button>
            {#if assistant.panes.length > 1}
              <button class="btn ghost sm" type="button" onclick={onEmptyClosePane}>
                <X size={12}/> Close pane
              </button>
            {/if}
          </div>
          {#if emptyRecents.length > 0}
            <div class="pane-empty-recent">
              <div class="pane-empty-recent-label">RECENT</div>
              {#each emptyRecents as c (c.id)}
                <button
                  class="pane-empty-recent-row"
                  type="button"
                  onclick={() => onEmptyOpenRecent(c.id)}
                  use:tooltip={c.title}
                >
                  <span class="pane-empty-recent-title">{c.title}</span>
                  <span class="pane-empty-recent-meta">{c.messageCount} msg</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {:else if showEmpty}
      <AssistantWelcome {needsAuth} {tabId} />
    {:else}
      <div class="messages" bind:this={messagesEl}>
        {#each messages as m, mi (m.id)}
          <MessageBubble
            message={m}
            isLast={mi === messages.length - 1}
            streaming={streaming
              && mi === messages.length - 1
              && m.role === "assistant"}
          />
        {/each}
      </div>
    {/if}
  </div>

  {#if tabId && !showEmpty}
    <div class="scroll-fade-top" class:visible={scrolledTop} aria-hidden="true"></div>
  {/if}

  {#if tabId && !showEmpty && !stickToBottom}
    <button class="jump-latest" type="button" onclick={jumpToLatest} use:tooltip={"Jump to latest"}>
      <span class="jl-ic" aria-hidden="true"><ChevronDown size={13}/></span>
      <span class="jl-label">Latest</span>
    </button>
  {/if}

  {#if showError || showNotice}
    <div class="alerts">
      {#if showNotice}
        <button class="alert notice" type="button" onclick={() => assistant.dismissNotice()} use:tooltip={"Click to dismiss"}>
          <span class="notice-icon">ℹ</span>
          <span class="notice-text">{assistant.lastNotice}</span>
        </button>
      {/if}
      {#if showError}
        <div class="alert error">
          <span class="notice-icon">⚠</span>
          <span class="notice-text">{lastError}</span>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Composer renders for every non-empty pane so two panes can compose
       concurrently. Empty pane (no tab) still shows the EmptyState scroll
       region above and no composer. Send focuses this pane first so the
       store's activeTab-driven send() targets the right tab. -->
  {#if tabId}
    <div class="composer-slot">
      <Composer
        {tabId}
        onsubmit={(text) => {
          if (!focused) assistant.setFocusedPane(paneIdx);
          assistant.send(text);
        }}
      />
    </div>
  {/if}

  {#if dragging}
    {#if assistant.splitActive}
      <div class="drop-zone full" class:hover={hoverHalf === "full"} aria-hidden="true">
        <span class="drop-label">Drop in pane {paneIdx + 1}</span>
      </div>
    {:else}
      <div class="drop-zone half left" class:hover={hoverHalf === "left"} aria-hidden="true">
        <span class="drop-label">Split → left</span>
      </div>
      <div class="drop-zone half right" class:hover={hoverHalf === "right"} aria-hidden="true">
        <span class="drop-label">Split → right</span>
      </div>
    {/if}
  {/if}

  <!-- Session Diff — full-pane review of every edit. Focused pane only so it
       doesn't render twice in split mode. -->
  {#if focused && assistant.ui.diffOpen}
    <SessionDiff {tabId} onClose={() => (assistant.ui.diffOpen = false)} />
  {/if}
</div>

  {#if dockOpen}
    <div
      class="dock-resize"
      class:active={resizing}
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize panel"
      onpointerdown={startResize}
      ondblclick={resetDockWidth}
      use:tooltip={"Drag to resize · double-click to reset"}
    ></div>
  {/if}
  <aside
    class="pane-dock-slot"
    class:open={dockOpen}
    class:resizing
    aria-hidden={!dockOpen}
    style={dockOpen ? `width:${assistant.ui.dockWidth}px` : ""}
  >
    <SidePanel {tabId} />
  </aside>
</div>

<style>
  .pane-shell {
    flex: 1 1 0;
    min-width: 320px;
    display: flex; flex-direction: row;
    min-height: 0;
  }
  .pane-dock-slot {
    width: 0;
    overflow: hidden;
    transition: width 220ms cubic-bezier(0.22, 1, 0.36, 1), opacity 180ms ease-out;
    display: flex;
    opacity: 0;
    flex-shrink: 0;
    border-left: 1px solid var(--border);
  }
  .pane-dock-slot.open { width: 300px; opacity: 1; }
  /* While dragging, kill the width transition so the panel tracks the cursor. */
  .pane-dock-slot.resizing { transition: opacity 180ms ease-out; }
  .pane-dock-slot :global(.side-panel) { flex: 1; min-width: 0; }

  /* Resize grabber on the dock's left edge — a thin hit-area with a hover line. */
  .dock-resize {
    flex-shrink: 0;
    width: 6px;
    margin-right: -6px; /* overlap the dock border so it sits on the seam */
    z-index: 6; /* above the dock-head (z5) so the bar reaches the top, not cut at the header */
    cursor: col-resize;
    position: relative;
  }
  .dock-resize::after {
    content: "";
    position: absolute;
    inset: 0 2px;
    border-radius: 2px;
    background: transparent;
    transition: background 120ms ease;
  }
  .dock-resize:hover::after,
  .dock-resize.active::after { background: var(--accent); }

  .pane {
    flex: 1 1 0;
    min-width: 0;
    display: flex; flex-direction: column;
    min-height: 0;
    position: relative;
    border: 1px solid transparent;
    transition: border-color 140ms ease-out, background 140ms ease-out;
  }
  .pane-chrome {
    position: absolute;
    top: 4px; right: 6px;
    z-index: 4;
    display: inline-flex; align-items: center; gap: 4px;
    pointer-events: none;
    /* Always-visible at low opacity so the pane index is readable at a glance;
       hover/focus brightens it. Hidden-until-hover hid the affordance the
       multi-session UI depends on. */
    opacity: 0.5;
    transition: opacity 120ms ease-out;
  }
  .pane:hover .pane-chrome, .pane.focused .pane-chrome { opacity: 0.95; }
  .pane-label {
    pointer-events: auto;
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 16px; height: 16px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }
  .pane-ctx-chip {
    pointer-events: auto;
    display: inline-flex; align-items: center; gap: 5px;
    height: 16px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-muted);
    line-height: 1;
  }
  .pane-ctx-bar {
    width: 28px; height: 3px;
    background: var(--bg-elev-3, var(--border));
    border-radius: 2px;
    overflow: hidden;
    display: inline-block;
  }
  .pane-ctx-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease-out;
  }
  .pane-ctx-pct { color: var(--fg); font-weight: 600; }
  .pane-cost { color: var(--fg-muted); }
  .pane-ctx-chip[data-tone="yellow"] {
    border-color: color-mix(in oklab, var(--warn) 35%, var(--border));
  }
  .pane-ctx-chip[data-tone="yellow"] .pane-ctx-fill { background: var(--warn); }
  .pane-ctx-chip[data-tone="yellow"] .pane-ctx-pct { color: var(--warn); }
  .pane-ctx-chip[data-tone="red"] {
    border-color: color-mix(in oklab, var(--danger) 40%, var(--border));
  }
  .pane-ctx-chip[data-tone="red"] .pane-ctx-fill { background: var(--danger); }
  .pane-ctx-chip[data-tone="red"] .pane-ctx-pct { color: var(--danger); }

  .pane.focused .pane-label {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border));
    background: var(--accent-soft);
  }
  .pane-close {
    pointer-events: auto;
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 18px;
    padding: 0;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--fg-muted);
    cursor: pointer;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .pane-close:hover {
    color: var(--danger);
    border-color: color-mix(in oklab, var(--danger) 35%, var(--border));
    background: color-mix(in oklab, var(--danger) 10%, transparent);
  }
  /* Focused pane — visible accent rail along the top edge plus a stronger
     border. Subtle in single-pane mode (no split visible); pronounced in
     split mode where multiple panes compete for attention. */
  .pane.split.focused {
    border-color: color-mix(in oklab, var(--accent) 60%, transparent);
    box-shadow: inset 0 2px 0 0 var(--accent);
  }
  .pane.split:not(.focused) {
    cursor: pointer;
    background: color-mix(in oklch, var(--bg) 94%, transparent);
  }
  .pane.split:not(.focused):hover {
    background: var(--bg);
    border-color: color-mix(in oklab, var(--accent) 22%, transparent);
  }
  /* ── Atmosphere ─────────────────────────────────────────────────────
     Pure accent + grain. Matches the existing UpdateDialog head-glow +
     EmptyState glyph-halo vocabulary — same color family, same restraint.
     No info/ok colors, no garish drift. Sits behind chat content. */
  .atmos {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
  }
  /* Single soft accent glow at the top edge — same shape as the
     UpdateDialog .head-glow (60% 100% at 50% 0%). Gives the chat surface
     a subtle "lit-from-above" feel without competing w/ content. */
  .atmos-glow {
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 42%;
    background:
      radial-gradient(85% 100% at 50% 0%,
        color-mix(in oklab, var(--accent) 6%, transparent) 0%,
        color-mix(in oklab, var(--accent) 2%, transparent) 42%,
        transparent 78%);
    opacity: 0.7;
  }
  /* Tiny film grain via inline SVG turbulence — adds organic texture so
     the dark surface doesn't read as a flat void. ~3% opacity, no anim. */
  .atmos-grain {
    position: absolute;
    inset: 0;
    opacity: 0.04;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='200' height='200'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 0.5 0'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>");
    background-size: 200px 200px;
    mix-blend-mode: overlay;
  }
  .composer-slot {
    position: relative;
    z-index: 1;
  }
  .scroll {
    position: relative;
    z-index: 1;
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 4px;
    display: flex; flex-direction: column;
    scrollbar-width: none;
  }
  .scroll::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .scroll::-webkit-scrollbar-button { display: none; }
  /* Top fade — once the thread is scrolled down, content tucks under a soft
     shadow at the pane's top edge instead of hard-cutting at the scroll
     boundary. Fades in/out with scroll position (scrolledTop). */
  .scroll-fade-top {
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 44px;
    pointer-events: none;
    z-index: 2;
    background: linear-gradient(
      to bottom,
      var(--bg) 0%,
      color-mix(in oklch, var(--bg) 55%, transparent) 50%,
      transparent 100%
    );
    opacity: 0;
    transition: opacity 220ms ease-out;
  }
  .scroll-fade-top.visible { opacity: 1; }
  @media (prefers-reduced-motion: reduce) {
    .scroll-fade-top { transition: none; }
  }
  .messages {
    display: flex; flex-direction: column;
    gap: 18px;
    max-width: var(--chat-col-max);
    width: 100%;
    margin: 0 auto;
  }
  /* Asymmetric rhythm groups each exchange: a question sits tight against its
     answer (14px base gap), while the break BEFORE a new user question opens
     up (+16px) so separate exchanges read as distinct blocks. Replaces the old
     uniform 28px that made every turn float equally. Rail-spans-turn still
     carries assistant grouping; no divider line. */
  .messages > :global(.bubble[data-role="user"]:not(:first-child)) {
    margin-top: 12px;
  }

  .pane-empty {
    flex: 1;
    display: flex; align-items: center; justify-content: center;
    padding: 24px;
  }
  .pane-empty-card {
    display: flex; flex-direction: column;
    gap: 10px;
    padding: 18px 20px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    background: color-mix(in oklch, var(--bg-elev-1) 55%, transparent);
    max-width: 360px; width: 100%;
  }
  .pane-empty-title {
    font-size: var(--fs-md);
    color: var(--fg);
    font-weight: 600;
    text-align: center;
  }
  .pane-empty-hint {
    font-size: var(--fs-sm);
    color: var(--fg-muted);
    text-align: center;
    line-height: 1.45;
  }
  .pane-empty-actions {
    display: flex; gap: 8px;
    justify-content: center;
    margin-top: 4px;
  }
  .pane-empty-recent {
    display: flex; flex-direction: column;
    gap: 4px;
    margin-top: 6px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .pane-empty-recent-label {
    font-size: var(--fs-xs);
    letter-spacing: 0.08em;
    color: var(--fg-subtle);
    font-weight: 600;
    margin-bottom: 2px;
  }
  .pane-empty-recent-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
    text-align: left;
    transition: background 100ms ease, border-color 100ms ease;
  }
  .pane-empty-recent-row:hover {
    background: var(--surface-hover);
    border-color: var(--border);
  }
  .pane-empty-recent-title {
    flex: 1; min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pane-empty-recent-meta {
    flex-shrink: 0;
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
  }

  /* Glass affordance that floats above the composer — same emerald-tinted glass
     language as the chat menus, lifted clear of the composer's top edge. The
     chevron sits in an accent disc that gently bobs to signal "new below". */
  .jump-latest {
    position: absolute;
    left: 50%;
    bottom: 96px;
    transform: translateX(-50%);
    display: inline-flex; align-items: center; gap: 7px;
    padding: 4px 13px 4px 5px;
    background: color-mix(in oklch, var(--surface) 84%, transparent);
    backdrop-filter: blur(14px) saturate(140%);
    -webkit-backdrop-filter: blur(14px) saturate(140%);
    border: 1px solid color-mix(in oklab, var(--accent) 26%, var(--border));
    border-radius: 999px;
    color: var(--fg-2);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 550;
    letter-spacing: 0.01em;
    cursor: pointer;
    box-shadow:
      0 8px 22px -6px oklch(0 0 0 / 0.5),
      0 0 0 1px color-mix(in oklab, var(--accent) 9%, transparent),
      0 0 16px -2px color-mix(in oklab, var(--accent) 20%, transparent);
    z-index: 3;
    animation: jump-in 200ms cubic-bezier(0.22, 1, 0.36, 1);
    transition: color 140ms ease, border-color 140ms ease, box-shadow 160ms ease, transform 160ms ease;
  }
  .jump-latest:hover {
    color: var(--fg);
    border-color: color-mix(in oklab, var(--accent) 52%, var(--border));
    box-shadow:
      0 10px 26px -6px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklab, var(--accent) 15%, transparent),
      0 0 22px -2px color-mix(in oklab, var(--accent) 32%, transparent);
    transform: translateX(-50%) translateY(-1px);
  }
  .jl-ic {
    display: inline-flex; align-items: center; justify-content: center;
    width: 19px; height: 19px; border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    color: var(--accent);
    animation: jl-bob 1.9s ease-in-out infinite;
  }
  .jump-latest:hover .jl-ic { background: color-mix(in oklab, var(--accent) 28%, transparent); }
  .jl-label { padding-right: 1px; }
  @keyframes jump-in {
    from { opacity: 0; transform: translate(-50%, 8px) scale(0.96); }
    to   { opacity: 1; transform: translate(-50%, 0) scale(1); }
  }
  @keyframes jl-bob {
    0%, 100% { transform: translateY(0); }
    50%      { transform: translateY(1.5px); }
  }
  @media (prefers-reduced-motion: reduce) {
    .jump-latest, .jl-ic { animation: none; }
  }

  .alerts {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    display: flex; flex-direction: column;
    gap: 6px;
    padding: 6px 18px 0;
    max-width: var(--chat-col-max);
    width: 100%;
    margin: 0 auto;
  }
  .alert {
    display: flex; align-items: flex-start; gap: 9px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    line-height: 1.4;
    animation: enter 200ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .alert.error {
    background: var(--danger-soft);
    border: 1px solid color-mix(in oklab, var(--danger) 35%, transparent);
    color: oklch(0.92 0.05 22);
  }
  .alert.error .notice-icon { color: var(--danger); }
  .alert.notice {
    background: color-mix(in oklab, var(--accent) 10%, var(--surface));
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
    color: var(--fg-2);
    cursor: pointer;
    transition: background 140ms ease-out, border-color 140ms ease-out;
  }
  .alert.notice:hover {
    background: color-mix(in oklab, var(--accent) 14%, var(--surface));
    border-color: color-mix(in oklab, var(--accent) 50%, var(--border));
  }
  .notice-icon {
    color: var(--accent);
    font-weight: 700;
    line-height: 1.4;
    flex-shrink: 0;
  }
  .notice-text { flex: 1; line-height: 1.45; }
  @media (prefers-reduced-motion: reduce) {
    .alert { animation: none; }
  }

  /* Drag-drop overlay zones — fade in while a tab is being dragged from the
     ChatTabsBar. `.full` covers the whole pane (split mode); `.half left`/
     `.half right` carve the pane in two so the user can pick which side to
     split into (single-pane mode). */
  .drop-zone {
    position: absolute;
    top: 0; bottom: 0;
    display: flex; align-items: center; justify-content: center;
    z-index: 5;
    /* Visual only — drop handling lives on the .pane parent so dragover is
       continuously preventDefault'd across all child crossings. */
    pointer-events: none;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border: 1px dashed color-mix(in oklab, var(--accent) 40%, transparent);
    color: color-mix(in oklab, var(--accent) 80%, var(--fg));
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.02em;
    transition: background 120ms ease-out, border-color 120ms ease-out;
    animation: drop-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .drop-zone.full { left: 0; right: 0; }
  .drop-zone.half.left { left: 0; right: 50%; }
  .drop-zone.half.right { left: 50%; right: 0; }
  .drop-zone.hover {
    background: color-mix(in oklab, var(--accent) 20%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }
  .drop-label {
    padding: 6px 14px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    box-shadow: 0 6px 18px oklch(0 0 0 / 0.32);
    /* Critical: children must not capture pointer events or they'll fire
       dragleave on the parent zone every time the cursor crosses the pill,
       flickering hover state + losing the dragover preventDefault chain. */
    pointer-events: none;
  }
  @keyframes drop-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .drop-zone { animation: none; }
  }
</style>
