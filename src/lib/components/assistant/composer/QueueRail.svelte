<script lang="ts">
  // Pending queued-message rail; see docs/ARCHITECTURE.md#frontend-map.
  // One ordered disclosure above the composer. A message typed while
  // a turn is streaming is queued (send.ts) and fires automatically when the turn
  // finishes — the chips here let you edit, reorder, or drop a queued message
  // before it sends.
  import { X, Clock, Paperclip, Pencil, ChevronDown } from "@lucide/svelte";
  import { fly, slide } from "svelte/transition";
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
  let open = $state(true);
  let hadQueue = $state(false);
  $effect(() => {
    const hasQueue = queue.length > 0;
    if (hasQueue && !hadQueue) open = true;
    hadQueue = hasQueue;
    if (editingId) open = true;
  });
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
  <div class="pending-rail" transition:fly={{ y: 7, duration: 170, easing: quintOut }}>
    <div class="rail-head">
      <button class="rail-toggle" type="button" aria-expanded={open} onclick={() => (open = !open)}>
        <Clock size={12} class="rail-lead" />
        <span class="rail-title">Next up</span>
        <span class="rail-count">{queue.length}</span>
        <span class="rail-hint">sends automatically</span>
        <span class="rail-chev" class:open><ChevronDown size={12} /></span>
      </button>
      {#if queue.length >= 2 && open}
        <button class="rail-clear" type="button" aria-label="Clear all queued messages" onclick={() => { if (tab) tab.queue = []; }}>
          Clear all
        </button>
      {/if}
    </div>
    {#if open}
      <div class="queue-list" transition:slide={{ duration: 160 }}>
        {#each queue as q, i (q.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="qrow"
            class:dragging={dragId === q.id}
            draggable={queue.length > 1 && editingId !== q.id}
            ondragstart={(e) => onChipDragStart(e, q.id)}
            ondragover={(e) => onChipDragOver(e, q.id)}
            ondragend={onChipDragEnd}
            in:fly={{ y: 5, duration: 150, delay: 20 * i, easing: quintOut }}
            out:fly={{ y: 5, duration: 120, easing: quintOut }}
          >
            <span class="qnum" aria-hidden="true">{i + 1}</span>
            {#if q.images?.[0]}
              {@const im = q.images[0]}
              {@const safeMime = /^image\/(png|jpeg|gif|webp|avif|bmp)$/.test(im.mime ?? "") ? im.mime : "image/png"}
              <img class="qthumb" src={`data:${safeMime};base64,${im.dataBase64}`} alt="" />
            {/if}
            {#if editingId === q.id}
              <input class="qedit" data-qedit={q.id} bind:value={editText} onkeydown={onEditKey} onblur={commitEditQueued} aria-label="Edit queued message" />
            {:else}
              <span class="qtext" class:grab={queue.length > 1} use:tooltip={queueChipLabel(q)}>{queueChipLabel(q)}</span>
              {#if q.text.trim().length > 0 && nAttach(q) > 0}
                <span class="qattach" use:tooltip={attachTip(q)}><Paperclip size={10} />{nAttach(q)}</span>
              {/if}
              <span class="qactions">
                <button class="qbtn" type="button" onclick={() => startEditQueued(q)} aria-label="Edit queued message" use:tooltip={"Edit"}><Pencil size={11} /></button>
                <button class="qbtn" type="button" onclick={() => removeQueued(q.id)} aria-label="Remove from queue" use:tooltip={"Remove"}><X size={11} /></button>
              </span>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  /* One queue owner. Ordered rows replace the chip cloud, and actions reveal
     only on hover/focus so message content stays primary. */
  .pending-rail {
    position: relative; z-index: 0; margin: 0 2px 7px;
    overflow: hidden; border: 1px solid color-mix(in oklch, var(--border) 78%, transparent);
    border-radius: 10px; background: color-mix(in oklch, var(--surface) 54%, transparent);
    font-size: var(--fs-xs); box-shadow: inset 0 1px 0 color-mix(in oklch, white 3%, transparent);
  }
  .rail-head { display: flex; align-items: center; min-width: 0; }
  .rail-toggle { display: flex; align-items: center; gap: 7px; flex: 1; min-width: 0;
    height: 30px; padding: 0 9px; border: 0; background: transparent; color: var(--fg-muted);
    font: inherit; cursor: pointer; text-align: left; }
  .rail-toggle:hover { background: color-mix(in oklab, var(--fg) 3.5%, transparent); color: var(--fg-2); }
  .rail-toggle:focus-visible, .rail-clear:focus-visible, .qbtn:focus-visible { outline: 0; box-shadow: inset 0 0 0 2px var(--ring); }
  :global(.rail-lead) { color: var(--fg-faint); flex: none; }
  .rail-title { font-weight: 650; color: var(--fg-2); }
  .rail-count { min-width: 17px; height: 17px; padding: 0 5px; display: inline-grid; place-items: center;
    border-radius: 999px; background: color-mix(in oklab, var(--fg) 7%, transparent);
    color: var(--fg-muted); font-family: var(--font-mono); font-size: 9.5px; font-weight: 700; }
  .rail-hint { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--fg-faint); }
  .rail-chev { display: inline-flex; margin-left: auto; color: var(--fg-faint); transition: transform var(--dur-fast); }
  .rail-chev.open { transform: rotate(180deg); }
  .rail-clear { flex: none; height: 24px; margin-right: 5px; padding: 0 7px;
    border: 0; border-radius: 6px; background: transparent; color: var(--fg-faint);
    font: inherit; font-size: 10px; cursor: pointer; }
  .rail-clear:hover { background: var(--surface-hover); color: var(--danger); }
  .queue-list { padding: 2px 6px 6px; border-top: 1px solid color-mix(in oklch, var(--border) 55%, transparent); }
  .qrow { display: flex; align-items: center; gap: 7px; min-width: 0; min-height: 30px;
    padding: 2px 4px; border-radius: 7px; color: var(--fg-muted); }
  .qrow + .qrow { border-top: 1px solid color-mix(in oklch, var(--border) 38%, transparent); border-radius: 0; }
  .qrow:hover, .qrow:focus-within { background: color-mix(in oklab, var(--fg) 3%, transparent); }
  .qrow.dragging { opacity: 0.45; }
  .qnum { width: 17px; flex: none; text-align: center; font-family: var(--font-mono); font-size: 9.5px; color: var(--fg-faint); }
  .qthumb { width: 24px; height: 24px; flex: none; object-fit: cover; border-radius: 6px; border: 1px solid var(--border); }
  .qtext { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--fg-muted); }
  .qtext.grab { cursor: grab; }
  .qrow.dragging .qtext.grab { cursor: grabbing; }
  .qattach { display: inline-flex; align-items: center; gap: 3px; flex: none; color: var(--fg-faint); font-size: 9.5px; }
  .qactions { display: inline-flex; align-items: center; gap: 2px; flex: none; opacity: 0;
    transition: opacity var(--dur-fast); }
  .qrow:hover .qactions, .qrow:focus-within .qactions { opacity: 1; }
  .qbtn { display: inline-grid; place-items: center; width: 22px; height: 22px; padding: 0;
    border: 0; border-radius: 6px; background: transparent; color: var(--fg-faint); cursor: pointer; }
  .qbtn:hover { background: var(--surface-hover); color: var(--fg-2); }
  .qedit { min-width: 0; flex: 1; height: 24px; padding: 0 7px; background: var(--bg-inset);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, var(--border)); border-radius: 6px;
    color: var(--fg); font: inherit; font-size: var(--fs-xs); outline: none; }
  .qedit:focus { box-shadow: 0 0 0 2px var(--ring); }
  @media (max-width: 620px) {
    .rail-hint { display: none; }
    .qactions { opacity: 1; }
  }
</style>
