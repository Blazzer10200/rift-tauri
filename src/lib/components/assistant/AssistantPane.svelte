<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, ChevronUp, Plus, X, MessageSquarePlus, ChevronRight } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { workspace } from "../../state/workspace.svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import StreamTurn from "./stream/StreamTurn.svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import AssistantWelcome from "./AssistantWelcome.svelte";
  import Composer from "./Composer.svelte";
  import SessionDiff from "./SessionDiff.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  let {
    tabId,
    focused,
    paneIdx,
  }: { tabId: string | null; focused: boolean; paneIdx: number } = $props();

  // Park the jump-to-latest button just above the composer no matter how tall it
  // grows (queue rail / attachments / enhance bar). A fixed px offset hid the
  // button behind a tall composer.
  let composerSlotEl = $state<HTMLDivElement | undefined>();
  let composerH = $state(72);
  $effect(() => {
    const el = composerSlotEl;
    if (!el) return;
    const ro = new ResizeObserver(() => { composerH = el.offsetHeight; });
    ro.observe(el);
    return () => ro.disconnect();
  });

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
  // Auth-class errors become an *actionable* recovery banner instead of a dead
  // wall of text. A rejected API key routes to Settings (where it's cleared);
  // a rejected/expired `claude login` routes to in-app sign-in.
  const isKeyError = $derived(
    !!lastError && /api key was rejected|configured api key/i.test(lastError),
  );
  const isAuthError = $derived(
    !!lastError &&
      (isKeyError ||
        /\b401\b|authentication failed|not logged in|claude login|session was rejected|sign in there/i.test(
          lastError,
        )),
  );
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
  // Human-friendly label for the pane header so a split is identifiable by its
  // conversation, not just a number. Mirrors healthAlerts.tabTitle.
  const paneTitle = $derived.by(() => {
    if (!tabId || !tab) return "Empty pane";
    if (tab.convoTitle) return tab.convoTitle;
    const first = tab.messages.find((m) => m.role === "user");
    const text = first?.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim()
      .replace(/\s+/g, " ");
    if (!text) return "New chat";
    return text.length > 48 ? text.slice(0, 48) + "…" : text;
  });

  let scrollEl = $state<HTMLDivElement | undefined>();
  let messagesEl = $state<HTMLDivElement | undefined>();
  let stickToBottom = $state(true);
  let scrolledTop = $state(false);
  let lastTabId: string | null = null;

  // ── turn-rail: right-edge exchange scrubber (shows at 2+ exchanges) ────────
  // One tick per user message (the anchor of an exchange). The active tick is
  // the last user bubble whose top has scrolled past the viewport top.
  const exchangeCount = $derived(messages.filter((m) => m.role === "user").length);
  let activeXch = $state(0);

  function userBubbles(): HTMLElement[] {
    if (!scrollEl) return [];
    return [...scrollEl.querySelectorAll<HTMLElement>('.bubble[data-role="user"]')];
  }
  function updateActiveXch() {
    if (!scrollEl) return;
    const top = scrollEl.getBoundingClientRect().top;
    const bs = userBubbles();
    let idx = 0;
    for (let k = 0; k < bs.length; k++) {
      if (bs[k].getBoundingClientRect().top - top <= 48) idx = k;
      else break;
    }
    activeXch = idx;
  }
  function scrollToXch(i: number) {
    const bs = userBubbles();
    const el = bs[i];
    if (!el || !scrollEl) return;
    const offset =
      el.getBoundingClientRect().top - scrollEl.getBoundingClientRect().top + scrollEl.scrollTop - 16;
    scrollEl.scrollTo({ top: Math.max(0, offset), behavior: "smooth" });
  }
  function stepXch(dir: -1 | 1) {
    scrollToXch(Math.min(exchangeCount - 1, Math.max(0, activeXch + dir)));
  }

  function onScroll() {
    if (!scrollEl) return;
    const gap = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
    stickToBottom = gap < 80;
    scrolledTop = scrollEl.scrollTop > 8;
    updateActiveXch();
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

  // ── Composer FLIP: home (centered hero) → conversation (docked bottom) ────
  // The composer-host is a persistent node in `.csurf-col` (outside the
  // home/convo {#if}), so the SAME element survives the first send. We snapshot
  // its rect before the send mutates state, then invert+animate to the docked
  // position once the layout settles (mirrors the comp `chat.jsx` FLIP).
  let flipFirst: DOMRect | null = null;
  let prevEmpty: boolean | null = null;

  function handleSend(text: string) {
    if (showEmpty && composerSlotEl) flipFirst = composerSlotEl.getBoundingClientRect();
    assistant.send(text, tabId);
  }

  function runComposerFlip() {
    const host = composerSlotEl;
    const first = flipFirst;
    flipFirst = null;
    if (!host || !first) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    void tick().then(() => {
      if (!host) return;
      const last = host.getBoundingClientRect();
      const dy = Math.round(first.top - last.top);
      if (!dy) return;
      host.style.transition = "none";
      host.style.transform = `translateY(${dy}px)`;
      void host.offsetHeight;
      host.style.transition = "transform 380ms cubic-bezier(0.22, 1, 0.36, 1)";
      host.style.transform = "translateY(0)";
      const clear = () => { host.style.transition = ""; host.style.transform = ""; };
      host.addEventListener("transitionend", clear, { once: true });
      setTimeout(clear, 460);
    });
  }

  $effect(() => {
    const empty = showEmpty;
    const was = prevEmpty;
    prevEmpty = empty;
    if (was === null || was === empty) return;
    if (was && !empty) runComposerFlip();
    else flipFirst = null;
  });

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
    <div class="pane-head" class:focused>
      <span class="pane-label" use:tooltip={`Pane ${paneIdx + 1} of ${assistant.panes.length}`}>{paneIdx + 1}</span>
      <span class="pane-head-title" use:tooltip={paneTitle}>{paneTitle}</span>
      {#if streaming}
        <span class="pane-live" use:tooltip={"This pane is working"}><span class="pane-live-dot"></span>working</span>
      {/if}
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
        use:tooltip={"Close this pane"}
        aria-label="Close pane"
        onclick={onClosePane}
      >
        <X size={11}/>
      </button>
    </div>
  {/if}

  {#if !tabId}
    <div class="scroll">
      <div class="pane-empty">
        <div class="pane-empty-card">
          <div class="pane-empty-mark"><MessageSquarePlus size={20}/></div>
          <div class="pane-empty-title">Empty pane</div>
          <div class="pane-empty-hint">
            Start a fresh chat here, or drag a tab from the bar into this pane.
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
              <div class="pane-empty-recent-label">RESUME</div>
              {#each emptyRecents as c (c.id)}
                <button
                  class="pane-empty-recent-row"
                  type="button"
                  onclick={() => onEmptyOpenRecent(c.id)}
                  use:tooltip={c.title}
                >
                  <span class="pane-empty-recent-title">{c.title}</span>
                  <span class="pane-empty-recent-meta">{c.messageCount} msg</span>
                  <ChevronRight class="pane-empty-recent-chev" size={13}/>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <!-- Merged surface: HOME (centered hero) ⇄ CONVERSATION. The composer-host
         is a persistent node so it FLIPs from center → bottom on the first send
         (mirrors comp `chat.jsx`). Welcome/stream swap inside; alerts +
         composer stay below the content within the same column. -->
    <div class="csurf-col" class:is-home={showEmpty} class:is-convo={!showEmpty}>
      {#if showEmpty}
        <AssistantWelcome {needsAuth} {tabId} />
      {:else}
        <div class="stream" bind:this={scrollEl} onscroll={onScroll}>
          <div class="messages" bind:this={messagesEl}>
            {#each messages as m, mi (m.id)}
              {#if uiPrefs.streamMode && m.role === "assistant"}
                <StreamTurn
                  message={m}
                  isLast={mi === messages.length - 1}
                  streaming={streaming && mi === messages.length - 1}
                />
              {:else}
                <MessageBubble
                  message={m}
                  isLast={mi === messages.length - 1}
                  streaming={streaming
                    && mi === messages.length - 1
                    && m.role === "assistant"}
                />
              {/if}
            {/each}
          </div>
        </div>
      {/if}

      {#if showError || showNotice}
        <div class="alerts">
          {#if showNotice}
            <button class="alert notice" type="button" onclick={() => assistant.dismissNotice()} use:tooltip={"Click to dismiss"}>
              <span class="notice-icon">ℹ</span>
              <span class="notice-text">{assistant.lastNotice}</span>
            </button>
          {/if}
          {#if showError && isAuthError}
            <div class="alert error recovery">
              <span class="notice-icon">⚠</span>
              <div class="recovery-body">
                <span class="notice-text">{lastError}</span>
                <div class="recovery-actions">
                  {#if isKeyError}
                    <button class="recovery-btn primary" type="button" onclick={() => workspace.setActive("settings")}>
                      Open Settings
                    </button>
                  {:else}
                    <button
                      class="recovery-btn primary"
                      type="button"
                      disabled={assistant.loginInProgress}
                      onclick={() => assistant.startLogin()}
                    >
                      {assistant.loginInProgress ? "Signing in…" : "Sign in"}
                    </button>
                  {/if}
                  <button
                    class="recovery-btn"
                    type="button"
                    disabled={assistant.authChecking || assistant.loginInProgress}
                    onclick={() => assistant.recheckAuth()}
                  >
                    Re-check
                  </button>
                </div>
              </div>
            </div>
          {:else if showError}
            <div class="alert error">
              <span class="notice-icon">⚠</span>
              <span class="notice-text">{lastError}</span>
            </div>
          {/if}
        </div>
      {/if}

      <div class="composer-host" class:is-home={showEmpty} bind:this={composerSlotEl}>
        <Composer {tabId} hero={showEmpty} onsubmit={handleSend} />
      </div>
    </div>
  {/if}

  {#if tabId && !showEmpty}
    <div class="scroll-fade-top" class:visible={scrolledTop} aria-hidden="true"></div>
  {/if}

  {#if tabId && !showEmpty && !stickToBottom}
    <button class="jump-latest" type="button" style="bottom: {composerH + 12}px" onclick={jumpToLatest} aria-label="Jump to latest message">
      <span class="jl-ic" aria-hidden="true"><ChevronDown size={13}/></span>
      Jump to latest
    </button>
  {/if}

  {#if tabId && !showEmpty && exchangeCount >= 2}
    <div class="turnrail" aria-label="Jump between turns">
      <button class="tr-chev" type="button" disabled={activeXch <= 0} onclick={() => stepXch(-1)} aria-label="Previous turn">
        <ChevronUp size={14} />
      </button>
      <div class="tr-ticks">
        {#each Array(exchangeCount) as _, i (i)}
          <button
            class="tr-tick"
            class:on={i === activeXch}
            type="button"
            onclick={() => scrollToXch(i)}
            aria-label={`Turn ${i + 1}`}
            aria-current={i === activeXch ? "true" : undefined}
          ></button>
        {/each}
      </div>
      <button class="tr-chev" type="button" disabled={activeXch >= exchangeCount - 1} onclick={() => stepXch(1)} aria-label="Next turn">
        <ChevronDown size={14} />
      </button>
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
</div>

<style>
  .pane-shell {
    flex: 1 1 0;
    min-width: 320px;
    display: flex; flex-direction: row;
    min-height: 0;
  }
  .pane {
    flex: 1 1 0;
    min-width: 0;
    display: flex; flex-direction: column;
    min-height: 0;
    position: relative;
    border: 1px solid transparent;
    transition: border-color 140ms ease-out, background 140ms ease-out;
  }
  /* Pane header — a slim, always-legible strip atop each pane in split mode.
     Replaces the old floating low-opacity chrome: shows the pane index, its
     conversation title (so a split is identifiable at a glance), the ctx chip
     and a close button. Focused pane gets an accent wash + brighter title. */
  .pane-head {
    position: relative;
    z-index: 4;
    flex-shrink: 0;
    display: flex; align-items: center; gap: 6px;
    height: 28px;
    padding: 0 6px 0 8px;
    background: color-mix(in oklch, var(--bg-elev-1) 55%, transparent);
    border-bottom: 1px solid var(--border);
    transition: background 140ms ease-out, border-color 140ms ease-out;
  }
  .pane-head.focused {
    background: color-mix(in oklab, var(--accent) 9%, var(--bg-elev-1));
    border-bottom-color: color-mix(in oklab, var(--accent) 40%, var(--border));
  }
  .pane-head-title {
    flex: 1; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--fs-xs);
    font-weight: 500;
    color: var(--fg-muted);
    transition: color 140ms ease-out;
  }
  .pane-head.focused .pane-head-title { color: var(--fg); }
  .pane-label {
    flex-shrink: 0;
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
    flex-shrink: 0;
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
  /* Per-pane working indicator — the concurrent-streaming cue. Pulse via
     box-shadow (never opacity: a throttled/backgrounded pane could freeze on
     an opacity:0 frame and read as "dead"). */
  .pane-live {
    flex-shrink: 0;
    display: inline-flex; align-items: center; gap: 5px;
    height: 16px; padding: 0 7px 0 6px;
    border-radius: 999px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
    color: var(--accent);
    font-size: 10px; font-weight: 600; line-height: 1;
  }
  .pane-live-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    animation: pane-live-pulse var(--pulse-live, 1.6s) ease-out infinite;
  }
  @keyframes pane-live-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 45%, transparent); }
    70% { box-shadow: 0 0 0 4px transparent; }
  }
  @media (prefers-reduced-motion: reduce) {
    .pane-live-dot { animation: none; }
  }
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
    flex-shrink: 0;
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
  /* Ambient lift — a soft, hue-neutral pool of light behind the hero plus a
     faint floor vignette. No colored top band; the surface reads as gently
     lit rather than washed with accent, so it blends into --bg. The center
     pool borrows a whisper of accent (~3%) only so it isn't a grey smudge. */
  .atmos-glow {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 80% at 50% 34%,
        color-mix(in oklab, var(--accent) 3%, transparent) 0%,
        color-mix(in oklab, var(--fg) 1.5%, transparent) 34%,
        transparent 66%),
      radial-gradient(140% 60% at 50% 118%,
        color-mix(in oklab, #000 26%, transparent) 0%,
        transparent 60%);
    opacity: 0.85;
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
  /* ── Merged home ⇄ conversation column ──────────────────────────────────
     One flex column per pane. HOME centers its content (welcome + hero
     composer) as a unit and scrolls if it overflows; CONVERSATION lets the
     stream take the height with the composer docked at the bottom. The
     composer-host is shared across both so it FLIPs between the two. */
  .csurf-col {
    position: relative;
    z-index: 1;
    flex: 1; min-width: 0; min-height: 0;
    display: flex; flex-direction: column;
  }
  .csurf-col.is-home {
    justify-content: safe center;
    overflow-y: auto;
    padding: 30px 32px;
    scrollbar-width: none;
  }
  .csurf-col.is-home::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .csurf-col.is-home > :global(*) {
    width: 100%; max-width: 680px;
    margin-left: auto; margin-right: auto;
    flex: none;
  }
  .csurf-col.is-home .composer-host { margin-top: 16px; }

  .composer-host {
    position: relative;
    z-index: 1;
    width: 100%;
    will-change: transform;
  }

  /* Conversation scroll region — replaces the old single `.scroll` wrapper for
     the message timeline. Keeps the hidden-scrollbar + flex-column behavior. */
  .stream {
    position: relative;
    z-index: 1;
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 8px;
    scroll-padding-bottom: 28px;
    display: flex; flex-direction: column;
    scrollbar-width: none;
  }
  .stream::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .stream::-webkit-scrollbar-button { display: none; }

  .scroll {
    position: relative;
    z-index: 1;
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 28px;
    scroll-padding-bottom: 28px;
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
    margin-top: 18px;
  }

  .pane-empty {
    flex: 1;
    display: flex; align-items: center; justify-content: center;
    padding: 24px;
  }
  .pane-empty-card {
    display: flex; flex-direction: column; align-items: center;
    gap: 9px;
    padding: 22px 22px 18px;
    border: 1px solid var(--border);
    border-radius: var(--r-card, var(--radius-lg));
    background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent);
    box-shadow: 0 14px 38px -18px rgba(0, 0, 0, 0.55);
    max-width: 340px; width: 100%;
    animation: pane-empty-in 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @media (prefers-reduced-motion: reduce) {
    .pane-empty-card { animation: none; }
  }
  @keyframes pane-empty-in {
    from { opacity: 0; transform: translateY(6px) scale(0.985); }
    to   { opacity: 1; transform: none; }
  }
  .pane-empty-mark {
    width: 44px; height: 44px;
    border-radius: var(--radius-xl, 14px);
    display: grid; place-items: center;
    background: var(--accent-soft); color: var(--accent);
    margin-bottom: 1px;
  }
  .pane-empty-title {
    font-size: var(--fs-md);
    color: var(--fg);
    font-weight: 650;
    text-align: center;
    letter-spacing: -0.01em;
  }
  .pane-empty-hint {
    font-size: var(--fs-sm);
    color: var(--fg-muted);
    text-align: center;
    line-height: 1.45;
    max-width: 240px;
  }
  .pane-empty-actions {
    display: flex; gap: 8px;
    justify-content: center;
    margin-top: 6px;
  }
  .pane-empty-recent {
    display: flex; flex-direction: column;
    gap: 3px;
    margin-top: 12px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
    width: 100%;
  }
  .pane-empty-recent-label {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-faint);
    font-weight: 600;
    margin-bottom: 3px;
    padding: 0 4px;
  }
  .pane-empty-recent-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 9px;
    border-radius: 9px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
    text-align: left;
    transition: background 120ms ease, border-color 120ms ease;
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
    color: var(--fg-faint);
    font-family: var(--font-mono, monospace);
  }
  :global(.pane-empty-recent-chev) {
    flex-shrink: 0;
    color: var(--fg-faint);
    opacity: 0;
    transform: translateX(-3px);
    transition: opacity 120ms ease, transform 120ms ease, color 120ms ease;
  }
  .pane-empty-recent-row:hover :global(.pane-empty-recent-chev) {
    opacity: 1;
    transform: none;
    color: var(--accent);
  }

  /* Compact circular "scroll to latest" affordance, parked at the bottom-right
     of the message area just clear of the composer's top edge — the chat-app
     convention (Slack/Discord/ChatGPT). Icon-only; the chevron gently bobs to
     signal "new below". Replaced the centered text-pill, which read as bulky. */
  .jump-latest {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: inline-flex; align-items: center; gap: 7px;
    height: 32px; padding: 0 14px 0 12px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--surface) 82%, transparent);
    backdrop-filter: blur(16px) saturate(140%);
    -webkit-backdrop-filter: blur(16px) saturate(140%);
    border: 1px solid var(--border-strong);
    color: var(--fg-2);
    font-size: 12px; font-weight: 550;
    cursor: pointer;
    box-shadow: 0 12px 30px -14px oklch(0 0 0 / 0.6), 0 8px 22px -10px oklch(0 0 0 / 0.45);
    z-index: 3;
    animation: jump-in 200ms cubic-bezier(0.22, 1, 0.36, 1);
    transition: color 140ms ease, border-color 140ms ease;
  }
  .jump-latest:hover { color: var(--fg); border-color: var(--accent); }
  .jl-ic {
    display: grid; place-items: center;
    width: 18px; height: 18px; border-radius: 50%;
    background: var(--accent); color: var(--accent-fg);
  }
  @keyframes jump-in {
    from { opacity: 0; transform: translateX(-50%) translateY(8px); }
    to   { opacity: 1; transform: translateX(-50%) translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .jump-latest { animation: none; }
  }
  .turnrail {
    position: absolute; right: 9px; top: 50%; transform: translateY(-50%); z-index: 4;
    display: flex; flex-direction: column; align-items: flex-end; gap: 5px; padding: 5px 3px;
    opacity: 0.45; transition: opacity var(--dur-base);
  }
  .turnrail:hover { opacity: 1; }
  .tr-chev {
    width: 24px; height: 18px; display: grid; place-items: center;
    background: none; border: 0; cursor: pointer;
    color: var(--fg-faint); border-radius: 6px;
    transition: color var(--dur-fast), background var(--dur-fast);
  }
  .tr-chev:hover:not(:disabled) { color: var(--fg-2); background: var(--surface-hover); }
  .tr-chev:disabled { opacity: 0.3; cursor: default; }
  .tr-ticks { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; padding: 3px 6px; }
  .tr-tick {
    position: relative; height: 11px; display: flex; align-items: center; justify-content: flex-end;
    background: none; border: 0; padding: 0; cursor: pointer;
  }
  .tr-tick::after {
    content: ""; height: 2px; width: 12px; border-radius: 2px; background: var(--fg-faint); opacity: 0.55;
    transition: width var(--dur-fast), background var(--dur-fast), opacity var(--dur-fast);
  }
  .tr-tick:hover::after { width: 18px; opacity: 1; background: var(--fg-muted); }
  .tr-tick.on::after { width: 20px; background: var(--accent); opacity: 1; }

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
  /* Auth recovery: error text stacked over an action row. */
  .alert.error.recovery { align-items: flex-start; }
  .recovery-body { flex: 1; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .recovery-actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .recovery-btn {
    font: inherit;
    font-size: var(--fs-sm);
    font-weight: 600;
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid color-mix(in oklab, var(--danger) 40%, transparent);
    background: color-mix(in oklab, var(--danger) 12%, var(--surface));
    color: oklch(0.92 0.05 22);
    transition: background 140ms ease-out, border-color 140ms ease-out, opacity 140ms ease-out;
  }
  .recovery-btn:hover:not(:disabled) {
    background: color-mix(in oklab, var(--danger) 20%, var(--surface));
    border-color: color-mix(in oklab, var(--danger) 60%, transparent);
  }
  .recovery-btn.primary {
    background: var(--danger);
    border-color: var(--danger);
    color: oklch(0.99 0.01 22);
  }
  .recovery-btn.primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--danger) 88%, white);
  }
  .recovery-btn:disabled { opacity: 0.6; cursor: default; }
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

