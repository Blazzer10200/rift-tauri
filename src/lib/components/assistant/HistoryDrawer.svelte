<script lang="ts">
  import { History, Plus, Trash2, Pencil, Check, MessagesSquare, Search, X } from "lucide-svelte";
  import { assistant, type ConversationMeta } from "../../state/assistant.svelte";
  import PageHeader from "../shell/PageHeader.svelte";
  import OpenInPaneMenu from "./OpenInPaneMenu.svelte";

  // `compact` collapses the page-style PageHeader into a slim popover header
  // so this same component can serve both the workspace surface (removed
  // 2026-05-21) and the in-chat history popover. `onSelected` lets a popover
  // host close itself after the user picks/creates a conversation.
  let { compact = false, onSelected = () => {} }: {
    compact?: boolean;
    onSelected?: () => void;
  } = $props();

  let renameId = $state<string | null>(null);
  let renameDraft = $state("");
  let confirmDeleteId = $state<string | null>(null);
  let searchQuery = $state("");
  let hideTests = $state(localStorage.getItem("rift.history.hideTests") === "1");
  let ctxMenu = $state<{ tabId: string; x: number; y: number } | null>(null);

  // Heuristic test-title detector — catches the canary/ping pairs (Token-A,
  // ACK-ONE, Memorize) plus generic "hello"/"please respond" probe titles
  // that clutter History during dev sessions.
  const TEST_TITLE_RE = /^(token-?[a-z]?\.?|ack-?one|memorize this token|hello!?$|hello\s|please (do exactly|respond)|i want to audit)/i;
  function isTestTitle(t: string): boolean {
    return TEST_TITLE_RE.test(t.trim());
  }

  $effect(() => {
    localStorage.setItem("rift.history.hideTests", hideTests ? "1" : "0");
  });

  function onRowContext(e: MouseEvent, id: string) {
    e.preventDefault();
    // History entries that aren't yet in openTabs need to be opened first so
    // `dropTabIntoPane` (which gates on openTabs.includes) can target them.
    if (!assistant.openTabs.includes(id)) {
      void assistant.openTab(id).then(() => {
        ctxMenu = { tabId: id, x: e.clientX, y: e.clientY };
      });
    } else {
      ctxMenu = { tabId: id, x: e.clientX, y: e.clientY };
    }
  }

  const filteredConversations = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    let base = assistant.conversations;
    if (hideTests) base = base.filter((c) => !isTestTitle(c.title));
    if (!q) return base;
    return base.filter((c) => {
      if (c.title.toLowerCase().includes(q)) return true;
      // E5: fall through to compaction summaries so long-running compacted
      // convos remain searchable by topic, not just rename.
      const summaries = c.compactionSummaries;
      if (!summaries || summaries.length === 0) return false;
      return summaries.some((s) => s.toLowerCase().includes(q));
    });
  });

  // Bucket by recency for scannability — Today / Yesterday / This week / Older.
  // Day boundaries use local midnight, not 24h windows, so "yesterday at 11pm"
  // doesn't bleed into "today" the next morning.
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
  const groupedConversations = $derived.by(() => {
    const groups: Record<BucketKey, ConversationMeta[]> = {
      today: [], yesterday: [], week: [], older: [],
    };
    for (const c of filteredConversations) {
      groups[bucketOf(c.updatedAt)].push(c);
    }
    return (["today", "yesterday", "week", "older"] as BucketKey[])
      .filter((k) => groups[k].length > 0)
      .map((k) => ({ key: k, label: BUCKET_LABELS[k], items: groups[k] }));
  });

  const hiddenTestCount = $derived(
    hideTests ? assistant.conversations.filter((c) => isTestTitle(c.title)).length : 0,
  );

  function startRename(c: ConversationMeta) {
    renameId = c.id;
    renameDraft = c.title;
  }

  async function commitRename() {
    if (renameId && renameDraft.trim()) {
      await assistant.renameConversation(renameId, renameDraft);
    }
    renameId = null;
  }

  function fmtTime(ms: number): string {
    const diff = Date.now() - ms;
    const min = 60_000, hr = 60 * min, day = 24 * hr;
    if (diff < min) return "just now";
    if (diff < hr) return `${Math.floor(diff / min)}m ago`;
    if (diff < day) return `${Math.floor(diff / hr)}h ago`;
    if (diff < 7 * day) return `${Math.floor(diff / day)}d ago`;
    return new Date(ms).toLocaleDateString();
  }

  function onKeyRename(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); void commitRename(); }
    else if (e.key === "Escape") { e.preventDefault(); renameId = null; }
  }

  /** Svelte action — focus + select on mount. Replaces the `autofocus`
   *  attribute, which trips the a11y_autofocus lint and ships a global
   *  jank in some screen-readers. Scoped focus is fine here because the
   *  input only renders when the user clicks the rename button. */
  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
    return { destroy() {} };
  }
</script>

<aside class="drawer" class:compact aria-label="Conversation history">
    {#if compact}
      <div class="cmp-head">
        <span class="cmp-icon"><History size={13}/></span>
        <span class="cmp-title">History</span>
        <span class="cmp-count">{assistant.conversations.length}</span>
        <button
          class="cmp-new"
          type="button"
          title="New conversation (Ctrl+T)"
          onclick={() => { void assistant.newTab(); onSelected(); }}
        >
          <Plus size={12} /> New
        </button>
      </div>
    {:else}
      <PageHeader
        icon={History}
        title="History"
        subtitle="{assistant.conversations.length} conversation{assistant.conversations.length === 1 ? '' : 's'}"
        tone="info"
      >
        {#snippet actions()}
          <button
            class="iconbtn primary"
            type="button"
            title="New conversation"
            onclick={() => void assistant.newTab()}
          >
            <Plus size={13} /> New
          </button>
        {/snippet}
      </PageHeader>
    {/if}

    {#if assistant.conversations.length > 0}
      <div class="filters">
        <div class="search">
          <Search size={12} />
          <input
            type="search"
            placeholder="Filter by title or summary…"
            bind:value={searchQuery}
            aria-label="Filter conversations"
          />
          {#if searchQuery}
            <button class="search-clear" type="button" title="Clear" onclick={() => (searchQuery = "")}>
              <X size={11} />
            </button>
          {/if}
        </div>
        <label class="hide-tests" title="Hide canary/ping test conversations (Token-A, ACK-ONE, hello, …)">
          <input type="checkbox" bind:checked={hideTests} />
          <span>Hide tests</span>
          {#if hiddenTestCount > 0}
            <span class="hide-tests-count">{hiddenTestCount}</span>
          {/if}
        </label>
      </div>
    {/if}

    <div class="list">
      {#if assistant.conversations.length === 0}
        <div class="empty">
          <MessagesSquare size={22} />
          <p>No saved conversations yet.</p>
          <span class="hint">Send a message — it'll appear here automatically.</span>
        </div>
      {:else if filteredConversations.length === 0}
        <div class="empty">
          <Search size={22} />
          <p>No matches for "{searchQuery}".</p>
          <span class="hint">Try a shorter query.</span>
        </div>
      {:else}
        {#each groupedConversations as group (group.key)}
          <div class="group-label">{group.label}</div>
          {#each group.items as c (c.id)}
          <div
            class="row"
            class:active={c.id === assistant.currentConvoId}
            class:editing={c.id === renameId}
            oncontextmenu={(e) => onRowContext(e, c.id)}
            role="presentation"
          >
            {#if c.id === renameId}
              <input
                class="rename-input"
                type="text"
                bind:value={renameDraft}
                onkeydown={onKeyRename}
                onblur={() => void commitRename()}
                use:focusOnMount
              />
              <button class="row-btn" type="button" title="Save" onclick={() => void commitRename()}>
                <Check size={12} />
              </button>
            {:else}
              <button
                class="row-main"
                type="button"
                onclick={() => { void assistant.openTab(c.id); onSelected(); }}
                title="Open"
              >
                <span class="row-title">{c.title}</span>
                <span class="row-meta">
                  <span class="meta-model">{c.model.charAt(0).toUpperCase() + c.model.slice(1)}</span>
                  <span class="meta-dot">·</span>
                  <span>{c.messageCount} msg</span>
                  <span class="meta-dot">·</span>
                  <span class="meta-time">{fmtTime(c.updatedAt)}</span>
                </span>
              </button>
              <div class="row-tools">
                <button class="row-btn" type="button" title="Rename" onclick={() => startRename(c)}>
                  <Pencil size={11} />
                </button>
                {#if confirmDeleteId === c.id}
                  <button
                    class="row-btn danger confirm"
                    type="button"
                    title="Confirm delete"
                    onclick={() => { void assistant.deleteConversation(c.id); confirmDeleteId = null; }}
                  >
                    Sure?
                  </button>
                {:else}
                  <button class="row-btn" type="button" title="Delete" onclick={() => (confirmDeleteId = c.id)}>
                    <Trash2 size={11} />
                  </button>
                {/if}
              </div>
            {/if}
          </div>
          {/each}
        {/each}
      {/if}
    </div>
  </aside>

{#if ctxMenu}
  <OpenInPaneMenu
    tabId={ctxMenu.tabId}
    x={ctxMenu.x}
    y={ctxMenu.y}
    onClose={() => (ctxMenu = null)}
  />
{/if}

<style>
  .drawer {
    position: static;
    width: 100%;
    height: 100%;
    flex: 1;
    display: flex; flex-direction: column;
    background: transparent;
  }
  /* Popover variant — slim header replaces the full PageHeader. Hosted by
     ChatTabsBar's history popover, which owns the outer surface + shadow. */
  .cmp-head {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .cmp-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px;
    border-radius: 6px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .cmp-title {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg);
  }
  .cmp-count {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-faint);
    padding: 1px 6px;
    background: var(--bg-elev-2);
    border-radius: 999px;
  }
  .cmp-new {
    margin-left: auto;
    display: inline-flex; align-items: center; gap: 4px;
    padding: 3px 9px;
    background: var(--accent);
    color: var(--accent-fg);
    border: 1px solid var(--accent);
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 600;
    transition: filter 120ms;
  }
  .cmp-new:hover { filter: brightness(1.08); }

  .iconbtn {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 4px 9px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-muted);
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .iconbtn:hover { color: var(--fg); border-color: var(--border-strong); }
  .iconbtn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  .iconbtn.primary:hover { background: var(--accent-hover); border-color: var(--accent-hover); color: var(--accent-fg); }

  .filters {
    display: flex; align-items: center; gap: 8px;
    margin: 8px 10px 4px;
  }
  .hide-tests {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 5px 9px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    transition: border-color 120ms, color 120ms, background 120ms;
  }
  .hide-tests:hover { color: var(--fg); border-color: var(--border-strong); }
  .hide-tests input { margin: 0; cursor: pointer; accent-color: var(--accent); }
  .hide-tests-count {
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
    font-size: 10px;
    padding: 1px 5px;
    background: var(--surface);
    border-radius: 999px;
  }
  .group-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-subtle);
    padding: 10px 12px 4px;
  }
  .group-label:first-child { padding-top: 4px; }
  .search {
    flex: 1; min-width: 0;
    display: flex; align-items: center; gap: 6px;
    padding: 5px 9px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--fg-faint);
    transition: border-color 120ms, color 120ms;
  }
  .search:focus-within {
    border-color: var(--accent);
    color: var(--fg-muted);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .search input {
    flex: 1; min-width: 0;
    background: transparent;
    border: 0; outline: none;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
  }
  .search input::placeholder { color: var(--fg-subtle); }
  .search input::-webkit-search-cancel-button { appearance: none; }
  .search-clear {
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 18px;
    background: transparent;
    border: 0;
    color: var(--fg-faint);
    cursor: pointer;
    border-radius: 50%;
    padding: 0;
  }
  .search-clear:hover { background: var(--surface-hover); color: var(--fg); }

  .list {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 6px;
    display: flex; flex-direction: column;
    gap: 2px;
  }
  .empty {
    margin: auto;
    text-align: center;
    color: var(--fg-faint);
    display: flex; flex-direction: column; align-items: center;
    gap: 6px;
    padding: 24px;
  }
  .empty p { margin: 0; font-size: var(--fs-sm); color: var(--fg-muted); }
  .empty .hint { font-size: 11px; }

  .row {
    display: flex; align-items: stretch;
    gap: 2px;
    border-radius: 8px;
    transition: background 120ms;
  }
  .row:hover { background: var(--surface-hover); }
  .row.active { background: color-mix(in oklch, var(--accent) 14%, var(--surface)); }
  .row.active::before {
    content: "";
    position: absolute;
    left: -1px;
    width: 3px;
    height: calc(100% - 12px);
    background: var(--accent);
    border-radius: 0 2px 2px 0;
    margin-top: 6px;
  }
  .row-main {
    flex: 1; min-width: 0;
    display: flex; flex-direction: column; gap: 3px;
    padding: 8px 10px;
    background: transparent;
    border: 0;
    border-radius: 8px;
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    font: inherit;
  }
  .row-title {
    font-size: var(--fs-sm);
    font-weight: 500;
    color: var(--fg);
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row-meta {
    display: flex; align-items: center; gap: 5px;
    font-size: 10.5px;
    color: var(--fg-faint);
  }
  .meta-model {
    text-transform: capitalize;
    font-weight: 600;
    color: var(--fg-muted);
  }
  .meta-dot { opacity: 0.6; }
  .meta-time { margin-left: auto; }

  .row-tools {
    display: flex; align-items: center; gap: 2px;
    padding: 0 4px;
    opacity: 0;
    transition: opacity 120ms;
  }
  .row:hover .row-tools, .row.editing .row-tools { opacity: 1; }
  .row-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px;
    background: transparent;
    border: 0; border-radius: 5px;
    color: var(--fg-faint);
    cursor: pointer;
    font: inherit;
    font-size: 10px;
    transition: background 120ms, color 120ms;
  }
  .row-btn:hover { background: var(--bg-elev-2); color: var(--fg); }
  .row-btn.danger:hover { background: var(--danger-soft); color: var(--danger); }
  .row-btn.confirm {
    width: auto;
    padding: 0 6px;
    background: var(--danger-soft);
    color: var(--danger);
    font-weight: 600;
  }

  .rename-input {
    flex: 1;
    padding: 8px 10px;
    background: var(--bg-elev-2);
    border: 1px solid var(--accent);
    border-radius: 8px;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    outline: none;
  }
</style>
