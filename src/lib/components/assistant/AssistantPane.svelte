<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, X } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Composer from "./Composer.svelte";
  import StatusHub from "./StatusHub.svelte";

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
  const showRemoteShellBanner = $derived(
    !assistant.remoteShellBannerSeen && assistant.remoteShellLastEvent !== null,
  );
  // Notices are session-global; only show on the focused pane to avoid dup banners.
  const showNotice = $derived(focused && !!assistant.lastNotice);
  const showShellBanner = $derived(focused && showRemoteShellBanner);

  let scrollEl = $state<HTMLDivElement | undefined>();
  let messagesEl = $state<HTMLDivElement | undefined>();
  let stickToBottom = $state(true);
  let lastTabId: string | null = null;

  function onScroll() {
    if (!scrollEl) return;
    const gap = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
    stickToBottom = gap < 80;
    if (tabId) assistant.setTabScroll(tabId, scrollEl.scrollTop);
  }

  $effect(() => {
    if (!messagesEl) return;
    const ro = new ResizeObserver(() => {
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
  {#if assistant.splitActive}
    <div class="pane-chrome" aria-hidden="true">
      <span class="pane-label" title="Pane {paneIdx + 1} of {assistant.panes.length}">{paneIdx + 1}</span>
      <button
        class="pane-close"
        type="button"
        title="Close this pane (Ctrl+Shift+\)"
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
        <div class="pane-empty-inner">No tab in this pane</div>
      </div>
    {:else if showEmpty}
      <EmptyState {needsAuth} />
    {:else}
      <div class="messages" bind:this={messagesEl}>
        {#each messages as m, mi (m.id)}
          <MessageBubble
            message={m}
            streaming={streaming
              && mi === messages.length - 1
              && m.role === "assistant"}
          />
        {/each}
      </div>
    {/if}
  </div>

  {#if tabId && !showEmpty && !stickToBottom}
    <button class="jump-latest" type="button" onclick={jumpToLatest} title="Jump to latest">
      <ChevronDown size={12}/>
      <span>Latest</span>
    </button>
  {/if}

  <StatusHub {tabId} />

  {#if focused && (lastError || showNotice || showShellBanner)}
    <div class="alerts">
      {#if showShellBanner}
        <button class="alert notice notice-shell" type="button" onclick={() => assistant.ackRemoteShellBanner()} title="Got it — don't show again">
          <span class="notice-icon">⚡</span>
          <span class="notice-text">
            Claude just ran a remote shell command on your server. Gated by Settings → Assistant → Allow remote shell + a workspace-scoped lock. Click to dismiss.
          </span>
          <span class="alert-x" aria-hidden="true"><ChevronDown size={11} style="transform: rotate(-90deg)"/></span>
        </button>
      {/if}
      {#if showNotice}
        <button class="alert notice" type="button" onclick={() => assistant.dismissNotice()} title="Click to dismiss">
          <span class="notice-icon">ℹ</span>
          <span class="notice-text">{assistant.lastNotice}</span>
        </button>
      {/if}
      {#if lastError}
        <div class="alert error">
          <span class="notice-icon">⚠</span>
          <span class="notice-text">{lastError}</span>
        </div>
      {/if}
    </div>
  {/if}

  {#if focused}
    <Composer onsubmit={(text) => assistant.send(text)} />
  {:else}
    <div class="pane-focus-hint">Click to focus this pane</div>
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
</div>

<style>
  .pane {
    flex: 1 1 0;
    min-width: 320px;
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
    opacity: 0;
    transition: opacity 120ms ease-out;
  }
  .pane:hover .pane-chrome, .pane.focused .pane-chrome { opacity: 0.85; }
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
  .pane.focused .pane-label {
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 35%, var(--border));
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
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    background: color-mix(in oklch, var(--danger) 10%, transparent);
  }
  .pane.split.focused {
    border-color: color-mix(in oklch, var(--accent) 30%, transparent);
  }
  .pane.split:not(.focused) {
    cursor: pointer;
    background: color-mix(in oklch, var(--bg) 92%, transparent);
  }
  .pane.split:not(.focused):hover {
    background: var(--bg);
  }
  .scroll {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 4px;
    display: flex; flex-direction: column;
    scrollbar-width: none;
  }
  .scroll::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .scroll::-webkit-scrollbar-button { display: none; }
  .messages {
    display: flex; flex-direction: column;
    gap: 20px;
    max-width: min(960px, 88ch);
    width: 100%;
    margin: 0 auto;
  }
  .messages :global(.bubble + .bubble) {
    padding-top: 14px;
    border-top: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
  }

  .pane-empty {
    flex: 1;
    display: flex; align-items: center; justify-content: center;
    color: var(--fg-faint);
    font-size: var(--fs-sm);
  }
  .pane-empty-inner {
    padding: 14px 20px;
    border: 1px dashed var(--border-strong);
    border-radius: 10px;
  }

  .pane-focus-hint {
    flex-shrink: 0;
    margin: 10px auto 14px;
    padding: 8px 14px;
    max-width: min(960px, 88ch);
    width: calc(100% - 36px);
    text-align: center;
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    background: var(--bg-elev-1);
    border: 1px dashed var(--border-strong);
    border-radius: 10px;
    pointer-events: none;
  }

  .jump-latest {
    position: absolute;
    left: 50%;
    bottom: 84px;
    transform: translateX(-50%);
    display: inline-flex; align-items: center; gap: 6px;
    padding: 5px 12px 5px 10px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--fg-2);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 500;
    cursor: pointer;
    box-shadow: 0 6px 18px oklch(0 0 0 / 0.32);
    z-index: 3;
    animation: jump-in 180ms cubic-bezier(0.22, 1, 0.36, 1);
    transition: background 140ms ease, color 140ms ease, border-color 140ms ease;
  }
  .jump-latest:hover {
    background: var(--surface-hover);
    color: var(--fg);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border-strong));
  }
  @keyframes jump-in {
    from { opacity: 0; transform: translate(-50%, 6px); }
    to   { opacity: 1; transform: translate(-50%, 0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .jump-latest { animation: none; }
  }

  .alerts {
    flex-shrink: 0;
    display: flex; flex-direction: column;
    gap: 6px;
    padding: 6px 18px 0;
    max-width: min(960px, 88ch);
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
    animation: alert-in 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .alert.error {
    background: var(--danger-soft);
    border: 1px solid color-mix(in oklch, var(--danger) 35%, transparent);
    color: oklch(0.92 0.05 22);
  }
  .alert.error .notice-icon { color: var(--danger); }
  .alert.notice {
    background: color-mix(in oklch, var(--accent) 10%, var(--surface));
    border: 1px solid color-mix(in oklch, var(--accent) 30%, var(--border));
    color: var(--fg-2);
    cursor: pointer;
    transition: background 140ms ease-out, border-color 140ms ease-out;
  }
  .alert.notice:hover {
    background: color-mix(in oklch, var(--accent) 14%, var(--surface));
    border-color: color-mix(in oklch, var(--accent) 50%, var(--border));
  }
  .notice-icon {
    color: var(--accent);
    font-weight: 700;
    line-height: 1.4;
    flex-shrink: 0;
  }
  .notice-text { flex: 1; line-height: 1.45; }
  .alert-x { display: inline-flex; color: var(--fg-muted); flex-shrink: 0; opacity: 0.7; }
  @keyframes alert-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
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
    background: color-mix(in oklch, var(--accent) 8%, transparent);
    border: 1px dashed color-mix(in oklch, var(--accent) 40%, transparent);
    color: color-mix(in oklch, var(--accent) 80%, var(--fg));
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
    background: color-mix(in oklch, var(--accent) 20%, transparent);
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
