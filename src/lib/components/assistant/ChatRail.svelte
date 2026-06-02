<script lang="ts">
  import { History, Plus, Search, X, ChevronLeft, ChevronRight, MessagesSquare } from "lucide-svelte";
  import { assistant, type ConversationMeta } from "../../state/assistant.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  const RAIL_KEY = "rift.ui.chat-rail-collapsed.v1";

  let searchQuery = $state("");

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

  function toggleCollapsed() {
    uiPrefs.chatRailCollapsed = !uiPrefs.chatRailCollapsed;
    try { localStorage.setItem(RAIL_KEY, uiPrefs.chatRailCollapsed ? "1" : "0"); } catch { /* noop */ }
  }
</script>

<aside
  class="cr"
  class:cr-collapsed={uiPrefs.chatRailCollapsed}
  aria-label="Conversation rail"
>
  {#if uiPrefs.chatRailCollapsed}
    <!-- Collapsed: just the toggle handle -->
    <button
      class="cr-expand-btn"
      type="button"
      use:tooltip={"Expand conversation rail"}
      onclick={toggleCollapsed}
      aria-label="Expand conversation rail"
    >
      <ChevronRight size={14} />
    </button>
  {:else}
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

    <!-- ── Search ── -->
    <div class="cr-search">
      <Search size={11} />
      <input
        type="search"
        placeholder="Search…"
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
            <button
              class="cr-row"
              class:active={c.id === assistant.currentConvoId}
              type="button"
              use:tooltip={c.title}
              onclick={() => openConvo(c.id)}
            >
              <span class="cr-bar" aria-hidden="true"></span>
              <span
                class="cr-dot"
                data-model={c.model?.toLowerCase().includes("opus") ? "opus"
                  : c.model?.toLowerCase().includes("haiku") ? "haiku"
                  : "sonnet"}
                aria-hidden="true"
              ></span>
              <span class="cr-title">{c.title}</span>
              <span class="cr-time">{fmtTime(c.updatedAt)}</span>
            </button>
          {/each}
        {/each}
      {/if}
    </div>
  {/if}
</aside>

<style>
  /* ── Shell ── */
  .cr {
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
  .cr.cr-collapsed {
    width: 32px;
  }

  /* ── Collapse toggle (collapsed state) ── */
  .cr-expand-btn {
    flex: none;
    display: grid;
    place-items: center;
    width: 100%;
    height: 40px;
    border: 0;
    background: transparent;
    color: var(--fg-subtle);
    cursor: pointer;
    transition: background 0.13s, color 0.13s;
  }
  .cr-expand-btn:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }

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
    background: linear-gradient(180deg, var(--bg) 78%, transparent);
    z-index: 1;
    white-space: nowrap;
  }
  .cr-group:first-child {
    padding-top: 2px;
  }

  /* ── Row ── */
  .cr-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: var(--radius-sm);
    padding: 6px 8px 6px 10px;
    cursor: pointer;
    font: inherit;
    transition: background 0.12s;
    min-width: 0;
  }
  .cr-row:hover {
    background: var(--surface);
  }
  .cr-row.active {
    background: var(--accent-soft);
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

  /* model-tinted dot */
  .cr-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
    background: var(--fg-muted);
  }
  .cr-dot[data-model="sonnet"] { background: oklch(0.74 0.13 230); }
  .cr-dot[data-model="opus"]   { background: oklch(0.70 0.18 295); }
  .cr-dot[data-model="haiku"]  { background: oklch(0.78 0.14 180); }

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
