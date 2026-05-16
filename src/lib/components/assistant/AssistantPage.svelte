<script lang="ts">
  import { onMount, tick } from "svelte";
  import { assistant } from "../../state/assistant.svelte";
  import AssistantHeader from "./AssistantHeader.svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import EmptyState from "./EmptyState.svelte";
  import Composer from "./Composer.svelte";
  import TasksDock from "./TasksDock.svelte";
  import HistoryDrawer from "./HistoryDrawer.svelte";

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
    // Observe the messages container for size changes (streaming text
    // grows the inner block w/o changing messages.length, so the $effect
    // below won't fire on every delta). When stuck to the bottom, snap.
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
</script>

<div class="assistant">
  <AssistantHeader />

  <div class="layout">
    <HistoryDrawer />
    <div class="chat">
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
          </div>
        {/if}
      </div>

      <Composer onsubmit={(text) => assistant.send(text)} />
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
</style>
