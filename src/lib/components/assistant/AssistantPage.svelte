<script lang="ts">
  import { onMount, tick } from "svelte";
  import { Minimize2, MessagesSquare, Plus } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import AssistantHeader from "./AssistantHeader.svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Composer from "./Composer.svelte";
  import TasksDock from "./TasksDock.svelte";
  import HistoryDrawer from "./HistoryDrawer.svelte";
  import { PANELS } from "../dock/panels";
  import { rightPane } from "../../state/right-pane.svelte";

  let scrollEl = $state<HTMLDivElement | undefined>();
  let messagesEl = $state<HTMLDivElement | undefined>();
  // Stick-to-bottom flag — flips off if the user scrolls up away from the
  // tail, flips back on when they scroll back within ~80 px of the bottom.
  // Without this, an auto-scroll effect yanks the viewport down every time a
  // stream-event delta lands, making it impossible to scroll up to read
  // earlier turns while streaming.
  let stickToBottom = $state(true);

  function onScroll() {
    if (!scrollEl) return;
    const gap = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
    stickToBottom = gap < 80;
  }

  onMount(() => {
    void assistant.init();
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

  const needsAuth = $derived(assistant.auth?.pill === "red");
  const showEmpty = $derived(assistant.messages.length === 0);
  const showRemoteShellBanner = $derived(
    !assistant.remoteShellBannerSeen && assistant.remoteShellLastEvent !== null,
  );
  // Phase C: when v0.3 maximize target is set, the chat scroll area swaps for
  // the panel body. Composer + header stay mounted so the user can keep
  // asking Claude about whatever they just maximized.
  const maximizedId = $derived(uiPrefs.useV03Shell ? uiPrefs.maximized : null);
  const maximizedDef = $derived(maximizedId ? PANELS[maximizedId] : null);
  // v0.4 — empty-tabs CTA. Under v0.3 only; v0.2 path always has chat shown.
  // Hides composer + chat scroller; renders a centered card w/ + New chat
  // button and a hint to expand History panel.
  const showEmptyTabsCta = $derived(
    uiPrefs.useV03Shell && assistant.openTabs.length === 0 && !maximizedId,
  );

  function openHistoryPanel() {
    rightPane.setActive("history");
  }
</script>

<div class="assistant">
  <AssistantHeader />

  <div class="layout">
    {#if !uiPrefs.useV03Shell}
      <HistoryDrawer />
    {/if}
    <div class="chat">
      {#if showEmptyTabsCta}
        <div class="empty-tabs">
          <div class="empty-tabs-card">
            <MessagesSquare size={28} class="sparkle"/>
            <h2>Start a new chat</h2>
            <p>Open a fresh tab to talk to Claude, or pick a saved conversation from History.</p>
            <div class="empty-tabs-actions">
              <button
                class="empty-tabs-btn primary"
                type="button"
                onclick={() => void assistant.newTab()}
              >
                <Plus size={14}/>
                <span>New chat</span>
                <kbd class="kbd-hint mono">Ctrl+T</kbd>
              </button>
              <button class="empty-tabs-btn" type="button" onclick={openHistoryPanel}>
                History
              </button>
            </div>
          </div>
        </div>
      {:else if maximizedId && maximizedDef}
        {#key maximizedId}
          <div class="restore-strip" role="region" aria-label="Maximized panel controls">
            <span class="restore-label">
              <maximizedDef.icon size={13}/>
              Maximized: <strong>{maximizedDef.title}</strong>
            </span>
            <button
              class="restore-btn"
              type="button"
              onclick={() => uiPrefs.maximizePanel(null)}
              title="Restore chat (Esc)"
            >
              <Minimize2 size={12}/>
              <span>Restore to dock</span>
              <kbd class="kbd-hint mono">Esc</kbd>
            </button>
          </div>
          <div class="max-body">
            <maximizedDef.component title={maximizedDef.title} icon={maximizedDef.icon}/>
          </div>
        {/key}
      {:else}
        <div class="scroll" bind:this={scrollEl} onscroll={onScroll}>
          {#if showEmpty}
            <EmptyState {needsAuth} />
          {:else}
            <div class="messages" bind:this={messagesEl}>
              {#each assistant.messages as m (m.id)}
                <MessageBubble message={m} streaming={assistant.streaming} />
              {/each}
              {#if assistant.lastError}
                <div class="error">⚠ {assistant.lastError}</div>
              {/if}
              {#if assistant.lastNotice}
                <button class="notice" type="button" onclick={() => assistant.dismissNotice()} title="Click to dismiss">
                  <span class="notice-icon">ℹ</span>
                  <span class="notice-text">{assistant.lastNotice}</span>
                </button>
              {/if}
              {#if showRemoteShellBanner}
                <button class="notice notice-shell" type="button" onclick={() => assistant.ackRemoteShellBanner()} title="Got it — don't show again">
                  <span class="notice-icon">⚡</span>
                  <span class="notice-text">
                    Claude just ran a remote shell command on your server. It's gated by the
                    Settings → Assistant → Allow remote shell toggle and a workspace-scoped
                    lock so two users can't fire shell commands at once. Click to dismiss.
                  </span>
                </button>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      {#if !showEmptyTabsCta}
        <Composer onsubmit={(text) => assistant.send(text)} />
      {/if}
    </div>

    <div class="dock-slot" class:open={assistant.ui.dockOpen}>
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
  }
  .scroll {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 4px;
    display: flex; flex-direction: column;
  }
  .messages {
    display: flex; flex-direction: column;
    gap: 10px;
    max-width: 860px;
    width: 100%;
    margin: 0 auto;
  }
  .error {
    margin-top: 10px;
    padding: 9px 12px;
    background: var(--danger-soft);
    border: 1px solid color-mix(in oklch, var(--danger) 35%, transparent);
    border-radius: 8px;
    color: oklch(0.92 0.05 22);
    font-size: var(--fs-sm);
  }
  .notice {
    margin-top: 10px;
    display: flex; align-items: flex-start; gap: 9px;
    width: 100%;
    padding: 9px 12px;
    background: color-mix(in oklch, var(--accent) 10%, var(--surface));
    border: 1px solid color-mix(in oklch, var(--accent) 30%, var(--border));
    border-radius: 8px;
    color: var(--fg-2);
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
    transition: background 140ms ease-out, border-color 140ms ease-out;
    animation: notice-in 200ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .notice:hover {
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
  @keyframes notice-in {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
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

  /* Phase C — Maximize-to-center strip + body. Sits above the maximized
     panel content, gives the user a clear "Restore to dock" affordance + Esc
     hint. Strip styled like the dialog/notice surface so it reads as a
     temporary overlay, not a permanent header. */
  .restore-strip {
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    padding: 6px 12px;
    background: color-mix(in oklch, var(--accent) 8%, var(--surface));
    border-bottom: 1px solid color-mix(in oklch, var(--accent) 28%, var(--border));
    font-size: var(--fs-sm);
    color: var(--fg-2);
    flex-shrink: 0;
    animation: restore-in 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .restore-label { display: inline-flex; align-items: center; gap: 8px; min-width: 0; color: var(--fg-muted); }
  .restore-label strong { color: var(--fg); font-weight: 600; }
  .restore-btn {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 4px 10px;
    background: transparent;
    border: 1px solid color-mix(in oklch, var(--accent) 35%, var(--border));
    color: var(--fg);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font: inherit; font-size: var(--fs-sm);
    transition: background 120ms ease, border-color 120ms ease;
  }
  .restore-btn:hover {
    background: color-mix(in oklch, var(--accent) 14%, var(--surface));
    border-color: color-mix(in oklch, var(--accent) 55%, var(--border));
  }
  .restore-btn .kbd-hint {
    padding: 1px 5px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 10px;
    color: var(--fg-muted);
  }
  .max-body {
    flex: 1; min-height: 0; min-width: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }
  @keyframes restore-in {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  /* v0.4 — empty-tabs CTA. Centered card replacing chat scroller + composer
     when assistant.openTabs is empty under v0.3. */
  .empty-tabs {
    flex: 1; min-height: 0;
    display: flex; align-items: center; justify-content: center;
    padding: 24px;
  }
  .empty-tabs-card {
    max-width: 360px;
    padding: 28px 32px 24px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: 12px;
    text-align: center;
    color: var(--fg-muted);
    box-shadow: 0 6px 24px oklch(0 0 0 / 0.18);
    animation: empty-tabs-in 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .empty-tabs-card :global(.sparkle) {
    color: var(--accent);
    margin-bottom: 8px;
  }
  .empty-tabs-card h2 {
    margin: 6px 0 8px;
    font-size: 16px;
    font-weight: 600;
    color: var(--fg);
  }
  .empty-tabs-card p {
    margin: 0 0 18px;
    font-size: var(--fs-sm);
    line-height: 1.5;
    color: var(--fg-muted);
  }
  .empty-tabs-actions {
    display: flex; flex-direction: column;
    gap: 6px;
    align-items: stretch;
  }
  .empty-tabs-btn {
    display: inline-flex; align-items: center; justify-content: center;
    gap: 8px;
    padding: 8px 14px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--fg-muted);
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-sm);
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .empty-tabs-btn:hover {
    color: var(--fg);
    border-color: var(--border-strong);
    background: var(--surface-hover);
  }
  .empty-tabs-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  .empty-tabs-btn.primary:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
    color: var(--accent-fg);
  }
  .empty-tabs-btn .kbd-hint {
    padding: 1px 6px;
    background: color-mix(in oklch, var(--accent-fg) 18%, transparent);
    border: 1px solid color-mix(in oklch, var(--accent-fg) 28%, transparent);
    border-radius: 4px;
    font-size: 10px;
    color: var(--accent-fg);
    margin-left: 4px;
  }
  .empty-tabs-btn:not(.primary) .kbd-hint {
    background: var(--bg-elev-2);
    border-color: var(--border);
    color: var(--fg-muted);
  }
  @keyframes empty-tabs-in {
    from { opacity: 0; transform: translateY(4px) scale(0.98); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }
</style>
