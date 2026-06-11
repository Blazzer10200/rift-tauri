<script lang="ts">
  // C3 (per docs/design/composer-split.md) — the pending rail: queued-message
  // chips + steer/clear actions docked to the composer's top lip, lifted
  // verbatim from Composer.svelte 2026-06-10. `steer()`/`steerFlash` stay in
  // the parent (they bind textarea focus + tab state) and arrive as props;
  // sendQueuedNow talks to the assistant store directly (brief allows — the
  // cluster already reads it pervasively).
  import { X, Check, Clock, Pencil, Navigation, CornerDownRight } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { tick } from "svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  type QueueItem = { id: string; text: string; mode?: "queue" | "steer" };

  let {
    tab,
    tabId = null,
    queue,
    streaming,
    steerFlash,
    draft,
    onSteer,
  }: {
    tab: { queue: QueueItem[] } | null;
    tabId?: string | null;
    queue: QueueItem[];
    streaming: boolean;
    steerFlash: boolean;
    draft: string;
    onSteer: () => void;
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
    if (!next) { tab.queue = tab.queue.filter((it) => it.id !== id); return; }
    tab.queue = tab.queue.map((it) => (it.id === id ? { ...it, text: next } : it));
  }
  function onEditKey(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); commitEditQueued(); }
    else if (e.key === "Escape") { e.preventDefault(); editingId = null; }
  }
  function removeQueued(id: string) {
    if (tab) tab.queue = tab.queue.filter((it) => it.id !== id);
  }
  // Promote a parked chip into the live turn: steer it now + drop it from the
  // queue. Only meaningful while streaming (assistant.steer re-queues on miss,
  // so no message is lost if the turn ends mid-click).
  function sendQueuedNow(q: { id: string; text: string }) {
    if (!tab || !streaming) return;
    removeQueued(q.id);
    pulseKey++;
    void assistant.steer(q.text, tabId);
  }
  // Rail-v2: per-chip fire mode. Steer chips don't start their own turn —
  // they inject into the NEXT turn at its first step (flushSteerChips in the
  // store). Toggle is a standing mark; "Send now" stays the immediate path.
  function toggleMode(q: QueueItem) {
    if (!tab) return;
    tab.queue = tab.queue.map((it) =>
      it.id === q.id ? { ...it, mode: it.mode === "steer" ? "queue" : "steer" } : it,
    );
  }
  // Pulse-on-inject: re-keying the sweep span replays the accent sweep. Fired
  // on "Send now" clicks and whenever the store flushes steer chips into a
  // turn (detected as a steer-count drop while streaming).
  let pulseKey = $state(0);
  let prevSteerCount = 0;
  $effect(() => {
    const n = queue.filter((q) => q.mode === "steer").length;
    if (n < prevSteerCount && streaming) pulseKey++;
    prevSteerCount = n;
  });
  const steerCount = $derived(queue.filter((q) => q.mode === "steer").length);
  const queueCount = $derived(queue.length - steerCount);
  const caption = $derived.by(() => {
    if (queue.length === 0) return "Working…";
    const parts: string[] = [];
    if (queueCount > 0) parts.push(queueCount === 1 ? "Sends when ready" : `${queueCount} queued`);
    if (steerCount > 0) parts.push(steerCount === 1 ? "Steers next turn" : `${steerCount} steer next turn`);
    return parts.join(" · ");
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

{#if queue.length > 0 || (streaming && (steerFlash || draft.trim().length > 0))}
  <div class="pending-rail" class:working={streaming && queue.length === 0} transition:fly={{ y: 14, duration: 260, easing: quintOut }}>
    {#key pulseKey}
      <span class="rail-sweep" aria-hidden="true"></span>
    {/key}
    <Clock size={12} class="rail-lead" />
    <span class="rail-caption">{caption}</span>
    {#each queue as q, i (q.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="pchip"
        class:steer={q.mode === "steer"}
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
          <span class="pchip-text" class:grab={queue.length > 1} use:tooltip={q.text}>{q.text}</span>
          {#if streaming}
            <button class="pchip-btn accent" type="button" onclick={() => sendQueuedNow(q)} aria-label="Send now into the running turn" use:tooltip={"Send now — inject into the running turn"}>
              <Navigation size={11} />
            </button>
          {/if}
          <button
            class="pchip-btn"
            class:accent={q.mode === "steer"}
            type="button"
            onclick={() => toggleMode(q)}
            aria-label={q.mode === "steer" ? "Switch to queue mode — fires as its own turn" : "Switch to steer mode — injects into the next turn"}
            use:tooltip={q.mode === "steer"
              ? "Steer mode — injects into the next turn's first step. Click for queue mode."
              : "Queue mode — fires as its own turn. Click to steer into the next turn instead."}
          >
            <CornerDownRight size={11} />
          </button>
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
      {#if streaming}
        <button
          class="rail-steer"
          class:flashed={steerFlash}
          type="button"
          onclick={onSteer}
          disabled={!steerFlash && !draft.trim()}
          aria-label="Steer the running turn"
          use:tooltip={draft.trim()
            ? "Redirect the running turn with your draft (Alt+Enter)"
            : "Type a message, then Steer to redirect the running turn"}
        >
          {#if steerFlash}
            <Check size={11} />
            Steered
          {:else}
            <Navigation size={11} />
            Steer
          {/if}
        </button>
      {/if}
      {#if queue.length >= 2}
        <button
          class="rail-clear"
          type="button"
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
  /* One-time accent sweep on activation — plays once when the rail mounts. */
  .rail-sweep {
    position: absolute; inset: 0;
    pointer-events: none;
    background: linear-gradient(
      100deg,
      transparent 0%,
      color-mix(in oklch, var(--accent) 0%, transparent) 35%,
      color-mix(in oklch, var(--accent) 28%, transparent) 50%,
      transparent 65%
    );
    background-size: 220% 100%;
    background-position: 120% 0;
    animation: rail-sweep 760ms ease-out 1;
  }
  @keyframes rail-sweep {
    from { background-position: 120% 0; }
    to   { background-position: -40% 0; }
  }
  :global(.rail-lead) { color: color-mix(in oklch, var(--accent) 85%, var(--fg-muted)); flex: none; }
  /* While "Working…" (no queue), breathe the lead + caption so the rail reads
     as a live turn-in-progress surface, not a static shelf. */
  .pending-rail.working :global(.rail-lead),
  .pending-rail.working .rail-caption { animation: rail-breathe 1.8s ease-in-out infinite; }
  @keyframes rail-breathe {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.55; }
  }
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
  /* Steer-mode chip — accent tint signals "rides the next turn" vs. a plain
     queued turn-starter. */
  .pchip.steer {
    border-color: color-mix(in oklab, var(--accent) 45%, var(--border));
    background: color-mix(in oklab, var(--accent) 10%, var(--field));
  }
  .pchip-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pchip-text.grab { cursor: grab; }
  .pchip.dragging .pchip-text.grab { cursor: grabbing; }
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
    transition: background 120ms ease-out, color 120ms ease-out;
  }
  .pchip-btn:hover { background: var(--bg-elev-2); color: var(--fg); }
  .pchip-btn.accent { color: color-mix(in oklab, var(--accent) 80%, var(--fg-faint)); }
  .pchip-btn.accent:hover { background: color-mix(in oklab, var(--accent) 20%, transparent); color: color-mix(in oklab, var(--accent) 95%, white); }
  .rail-actions {
    display: inline-flex; align-items: center; gap: 6px;
    margin-left: auto;
  }
  .rail-steer {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 10px;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 45%, var(--border));
    border-radius: 999px;
    color: color-mix(in oklab, var(--accent) 85%, var(--fg));
    cursor: pointer;
    font: inherit;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    transition: background 140ms ease-out, color 140ms ease-out, opacity 140ms ease-out;
  }
  .rail-steer:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 26%, transparent);
    color: color-mix(in oklab, var(--accent) 95%, white);
  }
  .rail-steer:disabled { opacity: 0.4; cursor: default; }
  .rail-steer.flashed {
    background: color-mix(in oklab, var(--accent) 30%, transparent);
    color: color-mix(in oklab, var(--accent) 95%, white);
    border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
    opacity: 1;
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
    transition: background 140ms ease-out, color 140ms ease-out;
  }
  .rail-clear:hover { background: var(--danger-soft); color: oklch(0.95 0.04 22); }
  @media (prefers-reduced-motion: reduce) {
    .rail-sweep { animation: none; }
    .pending-rail.working :global(.rail-lead),
    .pending-rail.working .rail-caption { animation: none; }
  }
</style>
