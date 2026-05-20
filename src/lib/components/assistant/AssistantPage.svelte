<script lang="ts">
  import { onMount, tick } from "svelte";
  import { ChevronDown } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Composer from "./Composer.svelte";
  import StatusHub from "./StatusHub.svelte";
  import TasksDock from "./TasksDock.svelte";

  let scrollEl = $state<HTMLDivElement | undefined>();
  let messagesEl = $state<HTMLDivElement | undefined>();
  // Stick-to-bottom flag — flips off if the user scrolls up away from the
  // tail, flips back on when they scroll back within ~80 px of the bottom.
  // Without this, an auto-scroll effect yanks the viewport down every time a
  // stream-event delta lands, making it impossible to scroll up to read
  // earlier turns while streaming.
  let stickToBottom = $state(true);
  // Tracks the convo whose scrollTop was last persisted, so on a tab switch
  // we know which slot to fill on scroll events.
  let lastActiveConvo: string | null = null;

  function onScroll() {
    if (!scrollEl) return;
    const gap = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
    stickToBottom = gap < 80;
    // Persist per-tab scroll position so switching away + back lands where we
    // were, not unconditionally pinned to bottom.
    if (assistant.currentConvoId) {
      assistant.setTabScroll(assistant.currentConvoId, scrollEl.scrollTop);
    }
  }

  onMount(() => {
    // Auto-open first tab once init resolves. Drops the redundant empty-tabs
    // gate — clicking "+ New chat" from a card just to land on the in-tab
    // empty state was a dead funnel.
    void assistant.init().then(() => {
      if (assistant.openTabs.length === 0) void assistant.newTab();
    });
  });

  // Observe the messages container for size changes (streaming text grows the
  // inner block w/o changing messages.length). $effect so it re-binds when
  // the scroll/messages elements remount (Phase C maximize toggle).
  $effect(() => {
    if (!messagesEl) return;
    const ro = new ResizeObserver(() => {
      if (scrollEl && stickToBottom) scrollEl.scrollTop = scrollEl.scrollHeight;
    });
    ro.observe(messagesEl);
    return () => ro.disconnect();
  });

  // Snap to bottom on user-initiated changes (new message, model swap, clear)
  // but skip stream-delta-driven scroll updates when the user is reading.
  $effect(() => {
    const _len = assistant.messages.length;
    const _streaming = assistant.streaming;
    void _len; void _streaming;
    void tick().then(() => {
      if (scrollEl && stickToBottom) scrollEl.scrollTop = scrollEl.scrollHeight;
    });
  });

  // Tab-switch scroll restore. Watches currentConvoId; on change, restore the
  // cached scrollTop for the incoming tab. Default to bottom if no cache
  // entry yet. Skips the very first run (no outgoing tab to remember).
  $effect(() => {
    const cur = assistant.currentConvoId;
    if (cur === lastActiveConvo) return;
    lastActiveConvo = cur;
    if (!cur || !scrollEl) return;
    const cached = assistant.getTabScroll(cur);
    void tick().then(() => {
      if (!scrollEl) return;
      if (cached != null) {
        // Smooth-scroll on tab restore so the jump reads as a transition,
        // not a snap. Safe here (one-shot); streaming-delta autoscroll
        // stays instant via the ResizeObserver path.
        scrollEl.scrollTo({ top: cached, behavior: "smooth" });
        const gap = scrollEl.scrollHeight - cached - scrollEl.clientHeight;
        stickToBottom = gap < 80;
      } else {
        scrollEl.scrollTop = scrollEl.scrollHeight;
        stickToBottom = true;
      }
    });
  });

  const needsAuth = $derived(assistant.auth?.pill === "red");
  const showEmpty = $derived(assistant.messages.length === 0);
  const showRemoteShellBanner = $derived(
    !assistant.remoteShellBannerSeen && assistant.remoteShellLastEvent !== null,
  );
  const hasTab = $derived(assistant.openTabs.length > 0);

  function jumpToLatest() {
    if (!scrollEl) return;
    scrollEl.scrollTo({ top: scrollEl.scrollHeight, behavior: "smooth" });
    stickToBottom = true;
  }
</script>

<div class="assistant">
  <div class="layout">
    <div class="chat">
      <div class="scroll" bind:this={scrollEl} onscroll={onScroll}>
        {#if showEmpty}
          <EmptyState {needsAuth} />
        {:else}
          <div class="messages" bind:this={messagesEl}>
            {#each assistant.messages as m, mi (m.id)}
              <MessageBubble
                message={m}
                streaming={assistant.streaming
                  && mi === assistant.messages.length - 1
                  && m.role === "assistant"}
              />
            {/each}
          </div>
        {/if}
      </div>

      {#if hasTab && !showEmpty && !stickToBottom}
        <button class="jump-latest" type="button" onclick={jumpToLatest} title="Jump to latest">
          <ChevronDown size={12}/>
          <span>Latest</span>
        </button>
      {/if}

      <StatusHub />

      <!-- Sticky alerts strip — errors + notices ride above the composer so
           they stay visible after replies push them up-scroll. Stacks bottom-up
           when multiple fire at once. -->
      {#if assistant.lastError || assistant.lastNotice || showRemoteShellBanner}
        <div class="alerts">
          {#if showRemoteShellBanner}
            <button class="alert notice notice-shell" type="button" onclick={() => assistant.ackRemoteShellBanner()} title="Got it — don't show again">
              <span class="notice-icon">⚡</span>
              <span class="notice-text">
                Claude just ran a remote shell command on your server. Gated by Settings → Assistant → Allow remote shell + a workspace-scoped lock. Click to dismiss.
              </span>
              <span class="alert-x" aria-hidden="true"><ChevronDown size={11} style="transform: rotate(-90deg)"/></span>
            </button>
          {/if}
          {#if assistant.lastNotice}
            <button class="alert notice" type="button" onclick={() => assistant.dismissNotice()} title="Click to dismiss">
              <span class="notice-icon">ℹ</span>
              <span class="notice-text">{assistant.lastNotice}</span>
            </button>
          {/if}
          {#if assistant.lastError}
            <div class="alert error">
              <span class="notice-icon">⚠</span>
              <span class="notice-text">{assistant.lastError}</span>
            </div>
          {/if}
        </div>
      {/if}

      <Composer onsubmit={(text) => assistant.send(text)} />
    </div>

    <div class="dock-slot" class:open={assistant.ui.dockOpen && assistant.tasks.length > 0}>
      <TasksDock />
    </div>
  </div>
</div>

<style>
  .assistant {
    flex: 1;
    display: flex; flex-direction: column;
    min-height: 0;
    background: var(--bg);
    color: var(--fg);
  }
  .layout {
    flex: 1; min-height: 0;
    display: flex;
    overflow: hidden;
    position: relative;
  }
  .chat {
    flex: 1; min-width: 0;
    display: flex; flex-direction: column;
    min-height: 0;
    position: relative;
  }
  .scroll {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 4px;
    display: flex; flex-direction: column;
    /* Hide scrollbar entirely — chat doesn't need a visible track, and the
       WebView2 native arrow-buttons leak through ::-webkit-scrollbar
       defaults. Scroll still works via wheel/keyboard/touch. */
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
  /* Faint top-border between adjacent bubbles — gives turns visual rhythm
     w/o forcing them into solid card boxes. First bubble stays flush. */
  .messages :global(.bubble + .bubble) {
    padding-top: 14px;
    border-top: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
  }

  /* Jump-to-latest pill — appears when user has scrolled away from tail.
     Centered above the composer; click smooth-scrolls back. */
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
  /* Sticky alerts strip — sits between the scroll area and the composer.
     Each alert is a row; multiple alerts stack w/ 6px gap. */
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

  .dock-slot {
    width: 0;
    overflow: hidden;
    transition: width 220ms cubic-bezier(0.22, 1, 0.36, 1), opacity 180ms ease-out;
    display: flex;
    opacity: 0;
  }
  .dock-slot.open { width: 280px; opacity: 1; }
  .dock-slot :global(.dock) { flex: 1; min-width: 280px; }
</style>
