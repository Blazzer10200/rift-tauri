<script lang="ts">
  import { History, X, Plus, Trash2, Pencil, Check, MessagesSquare, Search } from "lucide-svelte";
  import { assistant, type ConversationMeta } from "../../state/assistant.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";

  let renameId = $state<string | null>(null);
  let renameDraft = $state("");
  let confirmDeleteId = $state<string | null>(null);
  let searchQuery = $state("");

  const filteredConversations = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return assistant.conversations;
    return assistant.conversations.filter((c) => c.title.toLowerCase().includes(q));
  });

  function close() {
    assistant.ui.historyOpen = false;
    renameId = null;
    confirmDeleteId = null;
    searchQuery = "";
  }

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
  }
</script>

{#if uiPrefs.useV03Shell || assistant.ui.historyOpen}
  {#if !uiPrefs.useV03Shell}
    <div class="overlay" onclick={close} role="presentation"></div>
  {/if}
  <aside class="drawer" class:v03={uiPrefs.useV03Shell} aria-label="Conversation history">
    <header class="head">
      <div class="title">
        <History size={14} />
        <span>History</span>
        <span class="count">{assistant.conversations.length}</span>
      </div>
      <div class="actions">
        <button
          class="iconbtn primary"
          type="button"
          title="New conversation"
          onclick={() => {
            if (uiPrefs.useV03Shell) void assistant.newTab();
            else { void assistant.newConversation(); close(); }
          }}
        >
          <Plus size={13} /> New
        </button>
        {#if !uiPrefs.useV03Shell}
          <button class="iconbtn" type="button" title="Close" onclick={close}>
            <X size={14} />
          </button>
        {/if}
      </div>
    </header>

    {#if assistant.conversations.length > 0}
      <div class="search">
        <Search size={12} />
        <input
          type="search"
          placeholder="Filter by title…"
          bind:value={searchQuery}
          aria-label="Filter conversations"
        />
        {#if searchQuery}
          <button class="search-clear" type="button" title="Clear" onclick={() => (searchQuery = "")}>
            <X size={11} />
          </button>
        {/if}
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
        {#each filteredConversations as c (c.id)}
          <div
            class="row"
            class:active={c.id === assistant.currentConvoId}
            class:editing={c.id === renameId}
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
                onclick={() => {
                  if (uiPrefs.useV03Shell) void assistant.openTab(c.id);
                  else void assistant.loadConversation(c.id);
                }}
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
      {/if}
    </div>
  </aside>
{/if}

<style>
  .overlay {
    position: absolute;
    inset: 0;
    background: oklch(0 0 0 / 0.35);
    z-index: 30;
    animation: fade-in 180ms ease-out;
  }
  .drawer {
    position: absolute;
    top: 0; bottom: 0; left: 0;
    width: 320px;
    background: var(--surface);
    border-right: 1px solid var(--border);
    box-shadow: 8px 0 32px oklch(0 0 0 / 0.4);
    z-index: 31;
    display: flex; flex-direction: column;
    animation: slide-in 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  /* Under v0.3, PanelShell owns frame + close affordance; the drawer is just
     a body that fills its dock panel. */
  .drawer.v03 {
    position: static;
    width: 100%;
    height: 100%;
    box-shadow: none;
    border-right: 0;
    background: transparent;
    animation: none;
    z-index: auto;
    flex: 1;
  }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slide-in { from { transform: translateX(-100%); } to { transform: translateX(0); } }

  .head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 11px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev-2);
  }
  .title {
    display: flex; align-items: center; gap: 7px;
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg);
  }
  .count {
    padding: 1px 7px;
    background: var(--surface-hover);
    color: var(--fg-muted);
    border-radius: 999px;
    font-size: 10px;
    font-weight: 700;
  }
  .actions { display: flex; gap: 6px; }
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

  .search {
    display: flex; align-items: center; gap: 6px;
    margin: 8px 10px 4px;
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
