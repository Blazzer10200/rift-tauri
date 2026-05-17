<script lang="ts">
  import { PANELS } from "../dock/panels";
  import { rightPane, type ActivityBarId } from "$lib/state/right-pane.svelte";

  // Drag-to-reorder: source index captured on dragstart, drop index resolved
  // from event.target's data-index. Same HTML5 DnD pattern as ChatTabsBar +
  // PanelShell. Visual hint via data-drop-target on the hover row.
  let dragSrc = $state<number | null>(null);
  let dropTarget = $state<number | null>(null);

  function onDragStart(idx: number, ev: DragEvent) {
    dragSrc = idx;
    if (ev.dataTransfer) {
      ev.dataTransfer.effectAllowed = "move";
      ev.dataTransfer.setData("text/x-activitybar-id", rightPane.order[idx]);
    }
  }
  function onDragOver(idx: number, ev: DragEvent) {
    if (dragSrc === null) return;
    ev.preventDefault();
    if (ev.dataTransfer) ev.dataTransfer.dropEffect = "move";
    dropTarget = idx;
  }
  function onDrop(idx: number, ev: DragEvent) {
    if (dragSrc === null) return;
    ev.preventDefault();
    rightPane.reorder(dragSrc, idx);
    dragSrc = null;
    dropTarget = null;
  }
  function onDragEnd() {
    dragSrc = null;
    dropTarget = null;
  }
</script>

<nav class="activitybar" aria-label="Right pane tools">
  {#each rightPane.order as id, idx (id)}
    {@const def = PANELS[id as ActivityBarId]}
    {@const isActive = rightPane.activeId === id}
    {@const count = def.getCount?.() ?? 0}
    {@const tone = def.getTone ?? "neutral"}
    <button
      class="ab-btn"
      type="button"
      data-active={isActive}
      data-drop-target={dropTarget === idx}
      title="{def.title} · Ctrl+{idx + 1}"
      aria-label="{def.title} {isActive ? '(open)' : ''}"
      aria-pressed={isActive}
      draggable="true"
      onclick={() => rightPane.toggle(id as ActivityBarId)}
      ondragstart={(e) => onDragStart(idx, e)}
      ondragover={(e) => onDragOver(idx, e)}
      ondrop={(e) => onDrop(idx, e)}
      ondragend={onDragEnd}
    >
      <span class="ab-icon"><def.icon size={16}/></span>
      {#if count > 0}
        <span class="ab-count count-pip" data-tone={tone}>{count > 99 ? "99+" : count}</span>
      {/if}
    </button>
  {/each}
</nav>

<style>
  .activitybar {
    width: 40px;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    background: var(--surface);
    border-left: 1px solid var(--border);
    overflow: hidden;
    user-select: none;
  }
  .ab-btn {
    position: relative;
    display: flex; align-items: center; justify-content: center;
    width: 40px; height: 40px;
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
    flex-shrink: 0;
  }
  .ab-btn:hover { color: var(--fg); background: var(--surface-hover); }
  .ab-btn:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--ring);
    color: var(--fg);
  }
  .ab-btn[data-active="true"] {
    color: var(--accent);
    background: color-mix(in oklch, var(--accent) 8%, transparent);
  }
  .ab-btn[data-active="true"]::before {
    content: "";
    position: absolute;
    left: 0; top: 6px; bottom: 6px;
    width: 2px;
    background: var(--accent);
    border-radius: 0 2px 2px 0;
  }
  .ab-btn[data-drop-target="true"] {
    background: var(--accent-soft);
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .ab-icon {
    display: inline-flex; align-items: center; justify-content: center;
  }
  .ab-count {
    position: absolute;
    top: 4px; right: 3px;
    min-width: 14px;
    height: 14px;
    padding: 0 4px;
    border-radius: 999px;
    font-size: 9px;
    font-weight: 600;
    line-height: 14px;
    text-align: center;
    color: var(--fg);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    pointer-events: none;
  }
  .ab-count[data-tone="warn"]   { background: var(--warn-soft);   color: var(--warn);   border-color: color-mix(in oklch, var(--warn) 35%, var(--border)); }
  .ab-count[data-tone="danger"] { background: var(--danger-soft); color: var(--danger); border-color: color-mix(in oklch, var(--danger) 35%, var(--border)); }
  .ab-count[data-tone="info"]   { background: var(--info-soft);   color: var(--info);   border-color: color-mix(in oklch, var(--info) 35%, var(--border)); }
</style>
