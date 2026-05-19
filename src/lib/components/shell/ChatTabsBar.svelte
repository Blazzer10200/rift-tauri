<script lang="ts">
  // v0.4 — Chat tabs bar. Browser-style tab strip mounted by AppShell between
  // the wire-error banner and the .body grid whenever the Chat workspace is
  // active. Each tab is an entry in assistant.openTabs (convo id). Active tab
  // = currentConvoId.
  // DnD reorder via HTML5 drag-and-drop, tail-zone for append. Close button
  // stopPropagation so clicking it doesn't switch to the tab first.

  import { MessageSquare, Plus, X } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";

  let dragFromIdx = $state<number | null>(null);
  let dragOverIdx = $state<number | null>(null);

  const tabs = $derived(assistant.openTabs);
  const activeId = $derived(assistant.currentConvoId);

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
    return assistant.streaming && assistant.currentConvoId === id;
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
    // HTML5 DnD splice semantics: removing from < to means target idx shifts
    // left by one. Pass the raw drop idx — reorderTabs clamps after splice.
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
  }
</script>

<div class="tabsbar" role="tablist" aria-label="Chat tabs">
  <div class="strip">
    {#each tabs as id, idx (id)}
      <div
        class="tab"
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
    scrollbar-width: thin;
  }
  .strip::-webkit-scrollbar { height: 4px; }
  .strip::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

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
  .tab.active::after {
    /* Cover the bottom border of the tabsbar so the tab merges into the body. */
    content: "";
    position: absolute;
    left: 0; right: 0; bottom: -1px;
    height: 1px;
    background: var(--bg-elev-1);
  }
  .tab.drop-target {
    box-shadow: -2px 0 0 var(--accent);
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

  .new-tab {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 26px;
    /* Phase 3c: sit at the right end of the tabsbar with a 5px gap from the
       activity-bar boundary. No longer scrolls with the tab strip. */
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
