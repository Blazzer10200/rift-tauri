<script lang="ts">
  import { History, Plus, Search, X, ChevronLeft, MessagesSquare, Trash2 } from "lucide-svelte";
  import { assistant, type ConversationMeta } from "../../state/assistant.svelte";
  import { uiPrefs, CHAT_RAIL_MIN, CHAT_RAIL_MAX } from "../../state/ui-prefs.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  let searchQuery = $state("");
  // Per-row delete needs an inline confirm step so a stray click can't nuke a
  // chat; `confirmClearAll` is the equivalent guard for the wipe-everything path.
  let confirmDeleteId = $state<string | null>(null);
  let confirmClearAll = $state(false);

  function requestDelete(e: MouseEvent, id: string) {
    e.stopPropagation();
    confirmDeleteId = confirmDeleteId === id ? null : id;
  }
  function confirmDelete(e: MouseEvent, id: string) {
    e.stopPropagation();
    confirmDeleteId = null;
    void assistant.deleteConversation(id);
  }
  function clearAll() {
    confirmClearAll = false;
    void assistant.deleteAllConversations();
  }

  // Drag-to-resize the rail's right edge. Dragging right widens (rail is
  // left-anchored). Width is persisted (uiPrefs.chatRailWidth), clamped on move.
  let resizing = $state(false);
  function startResize(e: PointerEvent) {
    e.preventDefault();
    resizing = true;
    const startX = e.clientX;
    const startW = uiPrefs.chatRailWidth;
    const onMove = (ev: PointerEvent) => {
      const next = startW + (ev.clientX - startX);
      uiPrefs.chatRailWidth = Math.min(CHAT_RAIL_MAX, Math.max(CHAT_RAIL_MIN, next));
    };
    const onUp = () => {
      resizing = false;
      uiPrefs.setChatRailWidth(uiPrefs.chatRailWidth);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }
  function resetWidth() { uiPrefs.setChatRailWidth(220); }

  function fmtTime(ms: number): string {
    const diff = Date.now() - ms;
    const min = 60_000, hr = 60 * min, day = 24 * hr;
    if (diff < min) return "now";
    if (diff < hr) return `${Math.floor(diff / min)}m`;
    if (diff < day) return `${Math.floor(diff / hr)}h`;
    if (diff < 7 * day) return `${Math.floor(diff / day)}d`;
    return new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  type BucketKey = "today" | "yesterday" | "week" | "older";
  const BUCKET_LABELS: Record<BucketKey, string> = {
    today: "Today",
    yesterday: "Yesterday",
    week: "This week",
    older: "Older",
  };
  function bucketOf(ms: number): BucketKey {
    const now = new Date();
    const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const yesterdayStart = todayStart - 86_400_000;
    const weekStart = todayStart - 6 * 86_400_000;
    if (ms >= todayStart) return "today";
    if (ms >= yesterdayStart) return "yesterday";
    if (ms >= weekStart) return "week";
    return "older";
  }

  const filtered = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return assistant.conversations;
    return assistant.conversations.filter((c) => {
      if (c.title.toLowerCase().includes(q)) return true;
      const sums = c.compactionSummaries;
      return sums?.some((s) => s.toLowerCase().includes(q)) ?? false;
    });
  });

  const grouped = $derived.by(() => {
    const buckets: Record<BucketKey, ConversationMeta[]> = {
      today: [], yesterday: [], week: [], older: [],
    };
    for (const c of filtered) buckets[bucketOf(c.updatedAt)].push(c);
    return (["today", "yesterday", "week", "older"] as BucketKey[])
      .filter((k) => buckets[k].length > 0)
      .map((k) => ({ key: k, label: BUCKET_LABELS[k], items: buckets[k] }));
  });

  function openConvo(id: string) {
    void assistant.openTab(id);
  }

  // Collapse state + persistence now centralised in uiPrefs; the titlebar
  // PanelLeft button is the primary toggle. In-rail ChevronLeft still calls it.
  function toggleCollapsed() {
    uiPrefs.toggleChatRail();
  }
</script>

<aside
  class="cr"
  class:cr-collapsed={uiPrefs.chatRailCollapsed}
  class:resizing
  style={uiPrefs.chatRailCollapsed ? "" : `width:${uiPrefs.chatRailWidth}px`}
  aria-label="Conversation rail"
>
  {#if !uiPrefs.chatRailCollapsed}
    <!-- ── Header ── -->
    <div class="cr-head">
      <span class="cr-head-ico"><History size={14} /></span>
      <span class="cr-head-t">Chats</span>
      <span class="cr-head-n">{assistant.conversations.length}</span>
      <button
        class="cr-hbtn"
        type="button"
        use:tooltip={"New conversation (Ctrl+T)"}
        onclick={() => void assistant.newTab()}
      >
        <Plus size={13} />
      </button>
      {#if assistant.conversations.length > 0}
        <button
          class="cr-hbtn cr-hbtn-danger"
          type="button"
          use:tooltip={"Delete all chats"}
          onclick={() => (confirmClearAll = !confirmClearAll)}
          aria-label="Delete all chats"
        >
          <Trash2 size={13} />
        </button>
      {/if}
      <button
        class="cr-hbtn"
        type="button"
        use:tooltip={"Collapse rail"}
        onclick={toggleCollapsed}
        aria-label="Collapse conversation rail"
      >
        <ChevronLeft size={13} />
      </button>
    </div>

    {#if confirmClearAll}
      <div class="cr-confirm" role="alertdialog" aria-label="Confirm delete all chats">
        <span class="cr-confirm-t">Delete all {assistant.conversations.length} chats?</span>
        <div class="cr-confirm-act">
          <button class="cr-confirm-btn" type="button" onclick={() => (confirmClearAll = false)}>Cancel</button>
          <button class="cr-confirm-btn danger" type="button" onclick={clearAll}>Delete all</button>
        </div>
      </div>
    {/if}

    <!-- ── Search ── -->
    <div class="cr-search">
      <Search size={11} />
      <input
        type="search"
        placeholder="Search chats…"
        bind:value={searchQuery}
        aria-label="Filter conversations"
      />
      {#if searchQuery}
        <button class="cr-search-x" type="button" onclick={() => (searchQuery = "")}>
          <X size={10} />
        </button>
      {/if}
    </div>

    <!-- ── List ── -->
    <div class="cr-list">
      {#if assistant.conversations.length === 0}
        <div class="cr-none">
          <MessagesSquare size={18} />
          <p>No conversations yet.</p>
        </div>
      {:else if filtered.length === 0}
        <div class="cr-none">No matches for "{searchQuery}".</div>
      {:else}
        {#each grouped as group (group.key)}
          <div class="cr-group">{group.label}</div>
          {#each group.items as c (c.id)}
            <div
              class="cr-row"
              class:active={c.id === assistant.currentConvoId}
              class:confirming={confirmDeleteId === c.id}
            >
              <span class="cr-bar" aria-hidden="true"></span>
              <button
                class="cr-row-main"
                type="button"
                use:tooltip={c.title}
                onclick={() => openConvo(c.id)}
              >
                <span class="cr-dot" aria-hidden="true"></span>
                <span class="cr-title">{c.title}</span>
              </button>
              {#if confirmDeleteId === c.id}
                <button
                  class="cr-del confirm"
                  type="button"
                  use:tooltip={"Confirm delete"}
                  onclick={(e) => confirmDelete(e, c.id)}
                >
                  Delete?
                </button>
              {:else}
                <span class="cr-time">{fmtTime(c.updatedAt)}</span>
                <button
                  class="cr-del"
                  type="button"
                  use:tooltip={"Delete chat"}
                  aria-label="Delete chat"
                  onclick={(e) => requestDelete(e, c.id)}
                >
                  <Trash2 size={11} />
                </button>
              {/if}
            </div>
          {/each}
        {/each}
      {/if}
    </div>

    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="cr-resize"
      class:active={resizing}
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize conversation rail (drag, or double-click to reset)"
      use:tooltip={"Drag to resize · double-click to reset"}
      onpointerdown={startResize}
      ondblclick={resetWidth}
    ></div>
  {/if}
</aside>

<style>
  /* ── Shell ── */
  .cr {
    position: relative;
    flex: none;
    min-height: 0;
    align-self: stretch;
    background: var(--bg);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    width: 220px;
    transition: width 0.28s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
  }
  /* No width animation while dragging — the inline width drives it per-frame. */
  .cr.resizing { transition: none; }
  .cr.cr-collapsed {
    width: 0;
    border-right: 0;
  }

  /* Resize grabber on the rail's right edge — a thin hit-area with a hover line
     that sits on the seam. Mirrors the Activity dock's .dock-resize. */
  .cr-resize {
    position: absolute;
    top: 0; bottom: 0; right: 0;
    width: 6px;
    cursor: col-resize;
    z-index: 2;
  }
  .cr-resize::after {
    content: "";
    position: absolute;
    top: 0; bottom: 0; right: 0;
    width: 2px;
    background: transparent;
    transition: background 0.12s;
  }
  .cr-resize:hover::after,
  .cr-resize.active::after { background: var(--accent); }

  /* ── Header ── */
  .cr-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 12px 8px 8px 12px;
    flex: none;
    white-space: nowrap;
  }
  .cr-head-ico {
    color: var(--accent);
    display: grid;
    place-items: center;
    flex: none;
  }
  .cr-head-t {
    font-size: var(--fs-sm);
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--fg);
  }
  .cr-head-n {
    font-size: 10.5px;
    color: var(--fg-subtle);
    margin-right: auto;
    font-variant-numeric: tabular-nums;
  }
  .cr-hbtn {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    border: 0;
    background: transparent;
    color: var(--fg-subtle);
    cursor: pointer;
    flex: none;
    transition: background 0.13s, color 0.13s;
  }
  .cr-hbtn:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .cr-hbtn-danger:hover {
    background: var(--danger-soft);
    color: var(--danger);
  }

  /* ── Delete-all confirm bar ── */
  .cr-confirm {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 8px 4px;
    padding: 7px 10px;
    flex: none;
    border-radius: var(--radius);
    background: var(--danger-soft);
    border: 1px solid color-mix(in oklab, var(--danger) 30%, transparent);
  }
  .cr-confirm-t {
    flex: 1;
    min-width: 0;
    font-size: var(--fs-xs);
    font-weight: 550;
    color: var(--fg);
  }
  .cr-confirm-act {
    display: flex;
    gap: 6px;
    flex: none;
  }
  .cr-confirm-btn {
    padding: 3px 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg-2);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s, color 0.12s, border-color 0.12s;
  }
  .cr-confirm-btn:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .cr-confirm-btn.danger {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
  .cr-confirm-btn.danger:hover {
    filter: brightness(1.08);
  }

  /* ── Search ── */
  .cr-search {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 4px 8px;
    padding: 0 9px;
    height: 30px;
    flex: none;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--fg-subtle);
    white-space: nowrap;
    transition: border-color 0.12s, box-shadow 0.12s;
  }
  .cr-search:focus-within {
    border-color: var(--border-focus);
    box-shadow: 0 0 0 3px var(--ring);
  }
  .cr-search input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 0;
    outline: 0;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
  }
  .cr-search input::placeholder {
    color: var(--fg-subtle);
  }
  .cr-search input::-webkit-search-cancel-button {
    appearance: none;
  }
  .cr-search-x {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border: 0;
    background: transparent;
    color: var(--fg-subtle);
    border-radius: 4px;
    cursor: pointer;
    flex: none;
  }
  .cr-search-x:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }

  /* ── List ── */
  .cr-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 2px 6px 8px;
    /* Dissolve rows into the rail bg as they scroll under the search box —
       kills the hard half-row sliver that otherwise clips at the top seam.
       Reveals .cr's solid var(--bg), so the fade is seamless. */
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, #000 10px);
    mask-image: linear-gradient(to bottom, transparent 0, #000 10px);
  }
  .cr-list::-webkit-scrollbar {
    width: 5px;
  }
  .cr-list::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 8px;
    border: 2px solid transparent;
    background-clip: padding-box;
  }

  /* ── Group label ── */
  .cr-group {
    display: flex;
    align-items: center;
    font-size: 10px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-faint);
    padding: 12px 6px 4px;
    position: sticky;
    top: 0;
    /* Solid behind the label band so rows scrolling under it don't bleed
       through; a short gradient tail (::after) softens the bottom seam. */
    background: var(--bg);
    z-index: 1;
    white-space: nowrap;
  }
  .cr-group::after {
    content: "";
    position: absolute;
    left: 0; right: 0;
    bottom: -6px;
    height: 6px;
    background: linear-gradient(180deg, var(--bg), transparent);
    pointer-events: none;
  }
  .cr-group:first-child {
    padding-top: 2px;
  }

  /* ── Row ── */
  .cr-row {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    border-radius: var(--radius-sm);
    padding-right: 4px;
    transition: background 0.12s;
    min-width: 0;
  }
  .cr-row:hover {
    background: var(--surface);
  }
  .cr-row.active {
    background: var(--accent-soft);
  }
  .cr-row.confirming {
    background: var(--danger-soft);
  }

  /* Main click target — opens the conversation. */
  .cr-row-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    background: transparent;
    border: 0;
    border-radius: var(--radius-sm);
    padding: 6px 4px 6px 10px;
    text-align: left;
    cursor: pointer;
    font: inherit;
  }

  /* Delete affordance — hidden until row hover, then a quiet danger glyph. */
  .cr-del {
    flex: none;
    display: grid;
    place-items: center;
    height: 20px;
    min-width: 20px;
    padding: 0 5px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--fg-faint);
    cursor: pointer;
    font: inherit;
    font-size: 10px;
    font-weight: 650;
    opacity: 0;
    transition: opacity 0.12s, background 0.12s, color 0.12s;
  }
  .cr-row:hover .cr-del,
  .cr-row.confirming .cr-del {
    opacity: 1;
  }
  .cr-del:hover {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .cr-del.confirm {
    opacity: 1;
    background: var(--danger-soft);
    color: var(--danger);
    white-space: nowrap;
  }

  /* accent bar — left edge indicator */
  .cr-bar {
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 2px;
    background: var(--accent);
    opacity: 0;
    transition: opacity 0.14s;
  }
  .cr-row.active .cr-bar {
    opacity: 1;
  }

  /* Quiet neutral dot; the active conversation's lights emerald. Emerald-only —
     no model tint here (model identity lives on the composer model-card). */
  .cr-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
    background: var(--fg-muted);
  }
  .cr-row.active .cr-dot { background: var(--accent); }

  .cr-title {
    flex: 1;
    min-width: 0;
    font-size: var(--fs-xs);
    font-weight: 500;
    color: var(--fg-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cr-row.active .cr-title {
    color: var(--fg);
    font-weight: 550;
  }

  .cr-time {
    flex: none;
    font-size: 10px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }

  /* ── Empty / no-results ── */
  .cr-none {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    padding: 20px 8px;
    text-align: center;
    line-height: 1.5;
  }
  .cr-none p {
    margin: 0;
  }
</style>
