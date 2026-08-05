<script lang="ts">
  // Pending queued-message rail; see docs/ARCHITECTURE.md#frontend-map.
  // chips + clear action docked to the composer's top lip. A message typed while
  // a turn is streaming is queued (send.ts) and fires automatically when the turn
  // finishes — the chips here let you edit, reorder, or drop a queued message
  // before it sends.
  import { X, Clock, Paperclip, Pencil } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { queueChipLabel } from "./helpers";
  import type { QueueItem } from "$lib/state/assistant/types";

  let {
    tab,
    queue,
  }: {
    tab: { queue: QueueItem[] } | null;
    queue: QueueItem[];
  } = $props();

  // ── Pending Rail (queue chips) ────────────────────────────────────────
  // Inline edit-before-fire for a queued message. editingId pins the chip in
  // edit mode; commit writes back into tab.queue, Esc/blur cancels.
  let editingId = $state<string | null>(null);
  let editText = $state("");
  function startEditQueued(q: { id: string; text: string }) {
    editingId = q.id;
    editText = q.text;
    void tick().then(() => {
      const el = document.querySelector<HTMLInputElement>(`[data-qedit="${q.id}"]`);
      el?.focus();
      el?.select();
    });
  }
  function commitEditQueued() {
    if (!editingId || !tab) { editingId = null; return; }
    const next = editText.trim();
    const id = editingId;
    editingId = null;
    if (!next) {
      // Blanking the text only deletes a text-only item — one that carries
      // attachments survives as an attachment-only chip (X removes it whole).
      const it = tab.queue.find((q) => q.id === id);
      if ((it?.images?.length ?? 0) + (it?.textFiles?.length ?? 0) === 0) {
        tab.queue = tab.queue.filter((q) => q.id !== id);
        return;
      }
      tab.queue = tab.queue.map((q) => (q.id === id ? { ...q, text: "" } : q));
      return;
    }
    tab.queue = tab.queue.map((it) => (it.id === id ? { ...it, text: next } : it));
  }
  function onEditKey(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); commitEditQueued(); }
    else if (e.key === "Escape") { e.preventDefault(); editingId = null; }
  }
  function removeQueued(id: string) {
    if (tab) tab.queue = tab.queue.filter((it) => it.id !== id);
  }
  const nAttach = (q: QueueItem) => (q.images?.length ?? 0) + (q.textFiles?.length ?? 0);
  function attachTip(q: QueueItem): string {
    const parts: string[] = [];
    if (q.images?.length) parts.push(`${q.images.length} image${q.images.length === 1 ? "" : "s"}`);
    if (q.textFiles?.length) parts.push(`${q.textFiles.length} file${q.textFiles.length === 1 ? "" : "s"}`);
    return `${parts.join(" · ")} attached`;
  }
  const caption = $derived.by(() => (queue.length === 1 ? "Sends when ready" : `${queue.length} queued`));
  // Drag-to-reorder queued chips — the queue order IS the send order. Reorder
  // live on dragover so the rail rearranges under the cursor.
  let dragId = $state<string | null>(null);
  function onChipDragStart(e: DragEvent, id: string) {
    dragId = id;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }
  function onChipDragOver(e: DragEvent, overId: string) {
    if (!dragId || dragId === overId || !tab) return;
    e.preventDefault();
    const from = tab.queue.findIndex((q) => q.id === dragId);
    const to = tab.queue.findIndex((q) => q.id === overId);
    if (from < 0 || to < 0) return;
    const next = tab.queue.slice();
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    tab.queue = next;
  }
  function onChipDragEnd() { dragId = null; }
</script>

{#if queue.length > 0}
  <div class="pending-rail" transition:fly={{ y: 14, duration: 260, easing: quintOut }}>
    <Clock size={12} class="rail-lead" />
    <span class="rail-caption">{caption}</span>
    {#each queue as q, i (q.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="pchip"
        class:dragging={dragId === q.id}
        style="--idx: {i}"
        draggable={queue.length > 1 && editingId !== q.id}
        ondragstart={(e) => onChipDragStart(e, q.id)}
        ondragover={(e) => onChipDragOver(e, q.id)}
        ondragend={onChipDragEnd}
        in:fly={{ y: 10, duration: 220, delay: 30 * i, easing: quintOut }}
        out:fly={{ y: 12, duration: 200, easing: quintOut }}
      >
        {#if editingId === q.id}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="pchip-edit"
            data-qedit={q.id}
            bind:value={editText}
            onkeydown={onEditKey}
            onblur={commitEditQueued}
            aria-label="Edit queued message"
          />
        {:else}
          <span class="pchip-text" class:grab={queue.length > 1} use:tooltip={queueChipLabel(q)}>{queueChipLabel(q)}</span>
          {#if nAttach(q) > 0}
            <span class="pchip-attach" use:tooltip={attachTip(q)}>
              <Paperclip size={10} />{nAttach(q)}
            </span>
          {/if}
          <button class="pchip-btn" type="button" onclick={() => startEditQueued(q)} aria-label="Edit queued message" use:tooltip={"Edit"}>
            <Pencil size={11} />
          </button>
        {/if}
        <button class="pchip-btn" type="button" onclick={() => removeQueued(q.id)} aria-label="Remove from queue" use:tooltip={"Remove"}>
          <X size={11} />
        </button>
      </div>
    {/each}
    <div class="rail-actions">
      {#if queue.length >= 2}
        <button
          class="rail-clear"
          type="button"
          aria-label="Clear all queued messages"
          use:tooltip={"Clear all queued messages"}
          onclick={() => { if (tab) tab.queue = []; }}
        >
          Clear
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* ── Pending Rail ────────────────────────────────────────────────────
     Queue chips docked to the composer's top lip. Rounded top corners +
     square bottom + a downward tuck (margin-bottom) make it read as a shelf
     the composer grew, not a floating box. The composer card (z-index 1)
     overlaps the rail's bottom edge so chips appear to rise out of it. */
  .pending-rail {
    position: relative;
    z-index: 0;
    display: flex; flex-wrap: wrap; align-items: center; gap: 6px;
    margin: 0 6px -10px;
    padding: 7px 12px 15px;
    overflow: hidden;
    background: color-mix(in oklch, var(--surface) 70%, transparent);
    backdrop-filter: blur(10px) saturate(130%);
    -webkit-backdrop-filter: blur(10px) saturate(130%);
    border: 1px solid color-mix(in oklch, var(--accent) 22%, var(--border));
    border-bottom: 0;
    border-radius: 14px 14px 0 0;
    box-shadow: inset 0 1px 0 color-mix(in oklch, white 4%, transparent);
    font-size: var(--fs-xs);
  }
  :global(.rail-lead) { color: color-mix(in oklch, var(--accent) 85%, var(--fg-muted)); flex: none; }
  .rail-caption {
    font-weight: 600;
    letter-spacing: 0.01em;
    color: color-mix(in oklch, var(--accent) 55%, var(--fg-muted));
    margin-right: 2px;
  }
  .pchip {
    display: inline-flex; align-items: center; gap: 3px;
    padding: 2px 4px 2px 9px;
    max-width: 260px;
    background: var(--field);
    border: 1px solid color-mix(in oklch, var(--border) 92%, transparent);
    border-radius: 999px;
    color: var(--fg);
    box-shadow: 0 1px 3px -1px oklch(0 0 0 / 0.3);
  }
  .pchip.dragging { opacity: 0.45; }
  .pchip-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pchip-text.grab { cursor: grab; }
  .pchip.dragging .pchip-text.grab { cursor: grabbing; }
  .pchip-attach {
    display: inline-flex; align-items: center; gap: 2px; flex: none;
    padding: 0 5px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: color-mix(in oklch, var(--accent) 80%, var(--fg-muted));
    font-size: 9.5px; font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .pchip-edit {
    min-width: 120px; max-width: 240px;
    padding: 1px 4px;
    background: var(--bg-inset);
    border: 1px solid color-mix(in oklch, var(--accent) 45%, var(--border));
    border-radius: 6px;
    color: var(--fg);
    font: inherit; font-size: var(--fs-xs);
    outline: none;
  }
  .pchip-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 17px; height: 17px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--fg-faint);
    cursor: pointer;
    padding: 0;
    transition: background var(--dur-fast) ease-out, color var(--dur-fast) ease-out;
  }
  .pchip-btn:hover { background: var(--bg-elev-2); color: var(--fg); }
  .rail-actions {
    display: inline-flex; align-items: center; gap: 6px;
    margin-left: auto;
  }
  .rail-clear {
    padding: 2px 10px;
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--danger) 30%, var(--border));
    border-radius: 999px;
    color: color-mix(in oklab, var(--danger) 80%, var(--fg-muted));
    cursor: pointer;
    font: inherit;
    font-size: 10px;
    letter-spacing: 0.04em;
    transition: background var(--dur-fast) ease-out, color var(--dur-fast) ease-out;
  }
  .rail-clear:hover { background: var(--danger-soft); color: oklch(0.95 0.04 22); }
</style>
