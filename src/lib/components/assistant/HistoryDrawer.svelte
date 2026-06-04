<script lang="ts">
  import {
    History, Plus, Trash2, Pencil, Check, MessagesSquare, Search, X,
    Calendar, Clock, Bot, MessageSquare, GitBranch, Download, ExternalLink,
    FileText, Sparkles, ChevronRight, Maximize2,
  } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { assistant, type ConversationMeta } from "../../state/assistant.svelte";
  import type { ConversationRecord, Block, TextBlock } from "../../state/assistant/types";
  import PageHeader from "../shell/PageHeader.svelte";
  import OpenInPaneMenu from "./OpenInPaneMenu.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  // `compact` collapses the page-style PageHeader into a slim popover header
  // so this same component can serve both the workspace surface (removed
  // 2026-05-21) and the in-chat history popover. `onSelected` lets a popover
  // host close itself after the user picks/creates a conversation.
  let { compact = false, onSelected = () => {}, onExpand = undefined }: {
    compact?: boolean;
    onSelected?: () => void;
    onExpand?: () => void;
  } = $props();

  let renameId = $state<string | null>(null);
  let renameDraft = $state("");
  let confirmDeleteId = $state<string | null>(null);
  let searchQuery = $state("");
  let ctxMenu = $state<{ tabId: string; x: number; y: number } | null>(null);

  // Detail pane state (full mode only)
  let selectedId = $state<string | null>(null);
  let detailRecord = $state<ConversationRecord | null>(null);
  let detailLoading = $state(false);
  let detailError = $state<string | null>(null);
  let previewExpanded = $state(false);

  function onRowContext(e: MouseEvent, id: string) {
    e.preventDefault();
    ctxMenu = { tabId: id, x: e.clientX, y: e.clientY };
  }

  const filteredConversations = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    const base = assistant.conversations;
    if (!q) return base;
    return base.filter((c) => {
      if (c.title.toLowerCase().includes(q)) return true;
      const summaries = c.compactionSummaries;
      if (!summaries || summaries.length === 0) return false;
      return summaries.some((s) => s.toLowerCase().includes(q));
    });
  });

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

  // The ConversationMeta for the currently selected detail entry
  const selectedMeta = $derived(
    selectedId ? assistant.conversations.find((c) => c.id === selectedId) ?? null : null
  );

  async function selectConvo(id: string) {
    if (selectedId === id) return;
    selectedId = id;
    detailRecord = null;
    detailError = null;
    previewExpanded = false;
    detailLoading = true;
    try {
      detailRecord = await invoke<ConversationRecord>("assistant_load_conversation", { id });
    } catch (e) {
      detailRecord = null;
      detailError = e instanceof Error ? e.message : "Failed to load conversation.";
    } finally {
      detailLoading = false;
    }
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

  function fmtDate(ms: number): string {
    return new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  }

  function onKeyRename(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); void commitRename(); }
    else if (e.key === "Escape") { e.preventDefault(); renameId = null; }
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
    return { destroy() {} };
  }

  function modelLabel(model: string): string {
    return model.charAt(0).toUpperCase() + model.slice(1);
  }

  // Turn count: user messages only
  const detailTurnCount = $derived.by(() => {
    if (!detailRecord) return null;
    return detailRecord.messages.filter(m => m.role === "user").length;
  });

  // Transcript preview turns (first 6 messages, alternating user/assistant)
  const previewMessages = $derived.by(() => {
    if (!detailRecord) return [];
    return detailRecord.messages
      .filter(m => m.role === "user" || m.role === "assistant")
      .slice(0, 6);
  });

  // Compaction recap — most recent summary if present
  const latestRecap = $derived.by(() => {
    if (!detailRecord?.compactionHistory?.length) return null;
    const hist = detailRecord.compactionHistory;
    return hist[hist.length - 1].summary;
  });

  function msgTextPreview(blocks: Block[]): string {
    return blocks
      .filter((b): b is TextBlock => b.type === "text")
      .map(b => b.text)
      .join(" ")
      .trim()
      .replace(/\s+/g, " ")
      .slice(0, 200);
  }

  function openSelected() {
    if (!selectedId) return;
    void assistant.openTab(selectedId);
    onSelected();
  }
</script>

<aside class="drawer" class:compact aria-label="Conversation history">
  {#if compact}
    <!-- ── Compact popover header ─── -->
    <div class="cmp-head">
      <span class="cmp-icon"><History size={13}/></span>
      <span class="cmp-title">History</span>
      <span class="cmp-count">{assistant.conversations.length}</span>
      <button
        class="cmp-new"
        type="button"
        use:tooltip={"New conversation (Ctrl+T)"}
        onclick={() => { void assistant.newTab(); onSelected(); }}
      >
        <Plus size={12} /> New
      </button>
      {#if onExpand}
        <button
          class="cmp-expand"
          type="button"
          use:tooltip={"Open full history"}
          onclick={onExpand}
          aria-label="Open full history"
        >
          <Maximize2 size={13} />
        </button>
      {/if}
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
          use:tooltip={"New conversation"}
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
          <button class="search-clear" type="button" use:tooltip={"Clear"} onclick={() => (searchQuery = "")}>
            <X size={11} />
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ── Body: full mode = master/detail split; compact = list only ── -->
  <div class="body" class:master-detail={!compact}>

    <!-- Master list -->
    <div class="list" class:master-pane={!compact}>
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
            class:selected={!compact && c.id === selectedId}
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
              <button class="row-btn" type="button" use:tooltip={"Save"} onclick={() => void commitRename()}>
                <Check size={12} />
              </button>
            {:else}
              <button
                class="row-main"
                type="button"
                data-model={c.model?.toLowerCase().includes("opus") ? "opus"
                  : c.model?.toLowerCase().includes("haiku") ? "haiku"
                  : "sonnet"}
                onclick={() => {
                  if (compact) {
                    void assistant.openTab(c.id);
                    onSelected();
                  } else {
                    void selectConvo(c.id);
                  }
                }}
                use:tooltip={compact ? "Open" : "View details"}
              >
                <span class="row-title">{c.title}</span>
                <span class="row-meta">
                  <span class="row-model-dot" aria-hidden="true"></span>
                  <span class="meta-model">{modelLabel(c.model)}</span>
                  <span class="meta-dot">·</span>
                  <span>{c.messageCount} msg</span>
                  <span class="meta-time">{fmtTime(c.updatedAt)}</span>
                </span>
              </button>
              <div class="row-tools">
                <button class="row-btn" type="button" use:tooltip={"Rename"} onclick={() => startRename(c)}>
                  <Pencil size={11} />
                </button>
                {#if confirmDeleteId === c.id}
                  <button
                    class="row-btn danger confirm"
                    type="button"
                    use:tooltip={"Confirm delete"}
                    onclick={() => { void assistant.deleteConversation(c.id); confirmDeleteId = null; }}
                  >
                    Sure?
                  </button>
                {:else}
                  <button class="row-btn" type="button" use:tooltip={"Delete"} onclick={() => (confirmDeleteId = c.id)}>
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

    <!-- Detail pane (full mode only) -->
    {#if !compact}
      <div class="hp-detail" class:empty={!selectedMeta}>
        {#if !selectedMeta}
          <!-- No selection empty state -->
          <div class="hp-empty">
            <div class="hp-empty-glyph"><History size={22} /></div>
            <div class="hp-empty-t">Select a conversation</div>
            <p class="hp-empty-s">Pick any conversation on the left to see its details, recap, and transcript preview.</p>
          </div>
        {:else}
          <div class="hp-detail-scroll">
            <!-- Title + subtitle -->
            <div class="hp-d-head">
              <div class="hp-d-titlewrap">
                <h2 class="hp-d-title">{selectedMeta.title}</h2>
                <div class="hp-d-sub">
                  <span class="hp-proj">
                    <Bot size={13} />
                    {modelLabel(selectedMeta.model)}
                  </span>
                  <span class="hp-status mut">
                    <Clock size={10} />
                    {fmtTime(selectedMeta.updatedAt)}
                  </span>
                </div>
              </div>
            </div>

            <!-- Stat strip — only real fields from ConversationMeta -->
            <div class="hp-stats">
              <div class="hp-stat">
                <span class="hp-stat-ico"><MessageSquare size={14} /></span>
                <div>
                  <div class="hp-stat-v">{selectedMeta.messageCount}</div>
                  <div class="hp-stat-l">Messages</div>
                </div>
              </div>
              {#if detailTurnCount !== null}
                <div class="hp-stat">
                  <span class="hp-stat-ico"><ChevronRight size={14} /></span>
                  <div>
                    <div class="hp-stat-v">{detailTurnCount}</div>
                    <div class="hp-stat-l">Turns</div>
                  </div>
                </div>
              {/if}
              <div class="hp-stat">
                <span class="hp-stat-ico"><Calendar size={14} /></span>
                <div>
                  <div class="hp-stat-v">{fmtDate(selectedMeta.createdAt)}</div>
                  <div class="hp-stat-l">Created</div>
                </div>
              </div>
              <div class="hp-stat">
                <span class="hp-stat-ico"><Clock size={14} /></span>
                <div>
                  <div class="hp-stat-v">{fmtDate(selectedMeta.updatedAt)}</div>
                  <div class="hp-stat-l">Updated</div>
                </div>
              </div>
              <div class="hp-stat">
                <span class="hp-stat-ico"><Bot size={14} /></span>
                <div>
                  <div class="hp-stat-v">{modelLabel(selectedMeta.model)}</div>
                  <div class="hp-stat-l">Model</div>
                </div>
              </div>
            </div>

            <!-- AI Recap block -->
            <div class="hp-block">
              <div class="hp-block-label">
                <Sparkles size={12} />
                AI Recap
              </div>
              {#if detailLoading}
                <p class="hp-noedit">Loading…</p>
              {:else if detailError}
                <p class="hp-noedit">Error: {detailError}</p>
              {:else if latestRecap}
                <p class="hp-summary">{latestRecap}</p>
              {:else}
                <p class="hp-noedit">No recap available — recaps are generated when a conversation is compacted.</p>
              {/if}
            </div>

            <!-- Transcript preview -->
            <div class="hp-block hp-preview">
              <div class="hp-block-label">
                <FileText size={12} />
                Transcript preview
              </div>
              {#if detailLoading}
                <p class="hp-noedit">Loading…</p>
              {:else if detailError}
                <p class="hp-noedit">Error: {detailError}</p>
              {:else if previewMessages.length === 0}
                <p class="hp-noedit">No messages yet.</p>
              {:else}
                <div class="hpv" class:expanded={previewExpanded}>
                  {#each previewMessages as msg (msg.id)}
                    <div class="hpv-turn" class:claude={msg.role === "assistant"}>
                      <div class="hpv-av" class:you={msg.role === "user"} class:claude={msg.role === "assistant"}>
                        {msg.role === "user" ? "Y" : "A"}
                      </div>
                      <div class="hpv-body">
                        <div class="hpv-who">
                          {msg.role === "user" ? "You" : "Assistant"}
                          {#if msg.role === "assistant" && msg.model}
                            <span class="hpv-model">{msg.model}</span>
                          {/if}
                        </div>
                        <div class="hpv-text">{msgTextPreview(msg.blocks) || "(no text)"}</div>
                      </div>
                    </div>
                  {/each}
                  {#if !previewExpanded}
                    <div class="hpv-fade"></div>
                  {/if}
                </div>
                {#if previewMessages.length >= 3}
                  <button
                    class="hpv-expand-btn"
                    type="button"
                    onclick={() => (previewExpanded = !previewExpanded)}
                  >
                    {previewExpanded ? "Show less" : "Show more"}
                  </button>
                {/if}
              {/if}
            </div>
          </div>

          <!-- Footer CTAs -->
          <div class="hp-d-foot">
            <button
              class="hp-cta primary"
              type="button"
              onclick={openSelected}
            >
              <ExternalLink size={13} /> Open
            </button>
            <button
              class="hp-cta"
              type="button"
              disabled
              use:tooltip={"Coming soon"}
            >
              <GitBranch size={13} /> Branch
            </button>
            <button
              class="hp-cta"
              type="button"
              disabled
              use:tooltip={"Coming soon"}
            >
              <Download size={13} /> Export
            </button>
          </div>
        {/if}
      </div>
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

  /* ── Compact popover header ── */
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

  .cmp-expand {
    display: inline-flex; align-items: center; justify-content: center;
    width: 26px; height: 26px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-muted);
    cursor: pointer;
    transition: background 120ms, color 120ms, border-color 120ms;
    flex-shrink: 0;
  }
  .cmp-expand:hover {
    background: oklch(from var(--accent) l c h / 0.12);
    border-color: oklch(from var(--accent) l c h / 0.4);
    color: var(--accent);
  }

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
    flex-shrink: 0;
  }
  .group-label {
    display: flex; align-items: center; gap: 10px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    padding: 12px 12px 6px;
    position: sticky; top: 0;
    background: linear-gradient(180deg, var(--surface, var(--bg-elev-1)) 70%, transparent);
    z-index: 1;
  }
  .group-label::after {
    content: "";
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
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

  /* ── Body layout ── */
  .body {
    flex: 1; min-height: 0;
    display: flex;
    overflow: hidden;
  }
  .body.master-detail {
    flex-direction: row;
    gap: 0;
  }

  /* ── Master list ── */
  .list {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 6px;
    display: flex; flex-direction: column;
    gap: 2px;
  }
  .list.master-pane {
    flex: none;
    width: 300px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
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
    position: relative;
  }
  .row:hover { background: var(--surface-hover); }
  .row.active { background: color-mix(in oklab, var(--accent) 14%, var(--surface)); }
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
  /* selected highlight in full mode (detail pane open) */
  .row.selected {
    background: color-mix(in oklch, var(--accent-soft) 70%, var(--surface));
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
  }
  .row.selected::before {
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
  .meta-time {
    margin-left: auto;
    padding: 1px 6px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
  }

  .row-model-dot {
    width: 5px; height: 5px;
    border-radius: 999px;
    background: var(--row-model-color, var(--fg-muted));
    box-shadow:
      0 0 0 1.5px color-mix(in oklch, var(--row-model-color, var(--fg-muted)) 14%, transparent),
      0 0 4px color-mix(in oklch, var(--row-model-color, var(--fg-muted)) 40%, transparent);
    flex-shrink: 0;
  }
  .row-main[data-model="sonnet"] { --row-model-color: var(--accent); }
  .row-main[data-model="opus"]   { --row-model-color: var(--accent); }
  .row-main[data-model="haiku"]  { --row-model-color: var(--accent); }

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

  /* ── Detail pane ── */
  .hp-detail {
    flex: 1; min-width: 0; min-height: 0;
    display: flex; flex-direction: column;
    background: var(--bg-inset, var(--surface));
    border-left: 1px solid var(--border);
    overflow: hidden;
  }
  .hp-detail.empty {
    align-items: center;
    justify-content: center;
  }
  .hp-empty {
    text-align: center;
    max-width: 300px;
    padding: 24px;
  }
  .hp-empty-glyph {
    display: inline-grid; place-items: center;
    width: 52px; height: 52px;
    border-radius: 14px;
    color: var(--fg-subtle);
    background: var(--bg-inset, var(--surface));
    border: 1px solid var(--border);
    margin-bottom: 12px;
  }
  .hp-empty-t {
    font-size: var(--fs-lg);
    font-weight: 650;
    color: var(--fg-2, var(--fg-muted));
    margin-bottom: 6px;
  }
  .hp-empty-s {
    font-size: var(--fs-sm);
    color: var(--fg-subtle, var(--fg-faint));
    line-height: 1.5;
    margin: 0;
  }

  .hp-detail-scroll {
    flex: 1; min-height: 0;
    overflow-y: auto;
    display: flex; flex-direction: column;
    padding: 16px 18px 12px;
    gap: 0;
  }

  .hp-d-head {
    display: flex; align-items: flex-start; justify-content: space-between; gap: 12px;
    flex-shrink: 0;
  }
  .hp-d-titlewrap { min-width: 0; }
  .hp-d-title {
    margin: 0;
    font-size: 19px; font-weight: 680;
    letter-spacing: -0.02em; line-height: 1.2;
    color: var(--fg);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .hp-d-sub {
    display: flex; align-items: center; flex-wrap: wrap; gap: 8px;
    margin-top: 8px;
  }
  .hp-proj {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: var(--fs-sm); color: var(--fg-2, var(--fg-muted)); font-weight: 550;
  }
  .hp-status {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: 11px; font-weight: 600;
    padding: 2px 8px; border-radius: 999px;
  }
  .hp-status.mut {
    color: var(--fg-muted);
    background: var(--bg-inset, var(--bg-elev-2));
    border: 1px solid var(--border);
  }

  /* Stat strip */
  .hp-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
    gap: 8px;
    margin-top: 14px;
    flex-shrink: 0;
  }
  .hp-stat {
    display: flex; align-items: center; gap: 8px;
    padding: 9px 10px;
    border-radius: var(--radius, 8px);
    background: var(--bg-inset, var(--bg-elev-2));
    border: 1px solid var(--border);
  }
  .hp-stat-ico {
    display: grid; place-items: center;
    color: var(--fg-subtle, var(--fg-faint));
    flex: none;
  }
  .hp-stat-v {
    font-size: var(--fs-md, var(--fs-sm));
    font-weight: 640;
    color: var(--fg);
    letter-spacing: -0.01em;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hp-stat-l {
    font-size: 10px;
    color: var(--fg-subtle, var(--fg-faint));
    margin-top: 2px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Content blocks */
  .hp-block {
    margin-top: 14px;
    flex-shrink: 0;
  }
  .hp-block-label {
    display: flex; align-items: center; gap: 6px;
    font-size: 11px; font-weight: 650;
    letter-spacing: 0.05em; text-transform: uppercase;
    color: var(--fg-subtle, var(--fg-faint));
    margin-bottom: 8px;
  }
  .hp-summary {
    margin: 0;
    font-size: var(--fs-md, var(--fs-sm));
    line-height: 1.62;
    color: var(--fg-2, var(--fg-muted));
  }
  .hp-noedit {
    color: var(--fg-muted);
    font-size: var(--fs-sm);
    margin: 0;
    line-height: 1.5;
    font-style: italic;
  }
  .hp-preview {
    flex: 1;
    min-height: 64px;
  }

  /* Transcript preview */
  .hpv {
    position: relative;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 10px);
    background: var(--bg-inset, var(--bg-elev-2));
    padding: 4px 12px 2px;
    max-height: 220px;
    overflow: hidden;
    transition: max-height 300ms var(--ease-soft, ease);
  }
  .hpv.expanded {
    max-height: 600px;
  }
  .hpv-turn {
    position: relative;
    display: grid;
    grid-template-columns: 22px 1fr;
    gap: 10px;
    padding: 12px 0;
  }
  .hpv-turn + .hpv-turn {
    border-top: 1px solid var(--border);
  }
  .hpv-av {
    grid-row: 1;
    width: 22px; height: 22px;
    border-radius: 6px;
    display: grid; place-items: center;
    font-size: 10px; font-weight: 700;
    z-index: 1;
    flex-shrink: 0;
  }
  .hpv-av.you {
    background: var(--bg-elev-2);
    color: var(--fg-2, var(--fg-muted));
    border: 1px solid var(--border-strong);
  }
  .hpv-av.claude {
    background: var(--accent-soft);
    color: var(--accent);
    border: 1px solid color-mix(in oklab, var(--accent) 30%, transparent);
  }
  .hpv-body { min-width: 0; }
  .hpv-who {
    font-size: 11px; font-weight: 650;
    color: var(--fg-2, var(--fg-muted));
    display: flex; align-items: center; gap: 7px;
  }
  .hpv-model {
    font-weight: 550; font-size: 10.5px;
    color: var(--fg-subtle, var(--fg-faint));
  }
  .hpv-text {
    margin-top: 3px;
    font-size: var(--fs-sm);
    line-height: 1.55;
    color: var(--fg-muted);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .hpv-fade {
    position: absolute; left: 0; right: 0; bottom: 0;
    height: 46px;
    background: linear-gradient(180deg, transparent, var(--bg-inset, var(--bg-elev-2)));
    pointer-events: none;
  }
  .hpv-expand-btn {
    display: block;
    margin: 6px auto 0;
    padding: 3px 10px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--fg-muted);
    font: inherit;
    font-size: var(--fs-xs);
    cursor: pointer;
    transition: background 120ms, color 120ms;
  }
  .hpv-expand-btn:hover { background: var(--surface-hover); color: var(--fg); }

  /* Footer CTAs */
  .hp-d-foot {
    flex: none;
    display: flex; gap: 8px;
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    background: var(--surface, var(--bg-elev-1));
    flex-shrink: 0;
  }
  .hp-cta {
    display: inline-flex; align-items: center; gap: 7px;
    height: 34px; padding: 0 13px;
    border-radius: var(--radius, 8px);
    border: 1px solid var(--border);
    background: var(--bg-inset, var(--bg-elev-2));
    color: var(--fg-2, var(--fg-muted));
    font: inherit; font-size: var(--fs-sm); font-weight: 600;
    cursor: pointer;
    transition: all 130ms;
  }
  .hp-cta:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); background: var(--surface-hover); }
  .hp-cta.primary {
    background: var(--accent);
    color: var(--accent-fg);
    border-color: var(--accent);
    margin-right: auto;
  }
  .hp-cta.primary:hover:not(:disabled) { filter: brightness(1.08); }
  .hp-cta:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  /* Responsive: narrow detail */
  @media (max-width: 880px) {
    .list.master-pane { width: 260px; }
    .hp-stats { grid-template-columns: repeat(2, 1fr); }
  }
</style>
