<script lang="ts">
  import { Search, X, Pin, PinOff, MoreHorizontal, Pencil, Trash2, Circle } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import type { ConversationMeta } from "$lib/state/assistant/types";
  import { portal } from "$lib/actions/portal";

  // ── grouping ─────────────────────────────────────────────────────────
  type Group = { label: string; items: ConversationMeta[] };

  function timeBucket(ts: number, now: number): string {
    const day = 86_400_000;
    const startToday = new Date(now); startToday.setHours(0, 0, 0, 0);
    const t0 = startToday.getTime();
    if (ts >= t0) return "Today";
    if (ts >= t0 - day) return "Yesterday";
    if (ts >= t0 - day * 7) return "Previous 7 Days";
    if (ts >= t0 - day * 30) return "Previous 30 Days";
    return "Older";
  }

  const BUCKET_ORDER = ["Today", "Yesterday", "Previous 7 Days", "Previous 30 Days", "Older"];

  // ── per-project scope ────────────────────────────────────────────────
  // Canonical roots from the backend are already normalized; lower-case for a
  // case-insensitive match (Windows paths) so a convo's stamped root lines up
  // with the open folder regardless of drive-letter casing.
  function rootKey(r: string | null | undefined): string {
    return (r ?? "").replace(/[\\/]+$/, "").toLowerCase();
  }
  /** Short project label (folder basename) for All-projects mode rows. */
  function projLabel(r: string | null | undefined): string {
    const k = (r ?? "").replace(/[\\/]+$/, "");
    if (!k) return "Unfiled";
    const seg = k.split(/[\\/]/).pop();
    return seg || k;
  }
  const activeKey = $derived(rootKey(assistant.activeRoot));

  const groups = $derived.by<Group[]>(() => {
    const now = Date.now();
    const q = shell.convQuery.trim().toLowerCase();
    // Actively-streaming convos pin to the top of their bucket — clicking another
    // chat bumps its updatedAt, which must NOT shove the live turn below it.
    const working = new Set(assistant.liveTabs.map((t) => t.convoId));
    const byActivity = (a: ConversationMeta, b: ConversationMeta) => {
      const aw = working.has(a.id), bw = working.has(b.id);
      if (aw !== bw) return aw ? -1 : 1;
      return b.updatedAt - a.updatedAt;
    };
    const filtered = assistant.conversations.filter((c) => {
      // Scope to the open project unless the user flipped to All-projects, or
      // there's no open folder (activeKey === "" → show everything so chats are
      // never stranded). A convo whose root matches the open folder is in-scope.
      if (!shell.allProjects && activeKey && rootKey(c.workspaceRoot) !== activeKey) return false;
      return !q || c.title.toLowerCase().includes(q) || (c.lastSnippet ?? "").toLowerCase().includes(q);
    });
    const pinned: ConversationMeta[] = [];
    const buckets = new Map<string, ConversationMeta[]>();
    for (const c of filtered) {
      if (shell.isPinned(c.id)) { pinned.push(c); continue; }
      const b = timeBucket(c.updatedAt, now);
      (buckets.get(b) ?? buckets.set(b, []).get(b)!).push(c);
    }
    const out: Group[] = [];
    if (pinned.length) out.push({ label: "Pinned", items: pinned.sort(byActivity) });
    for (const label of BUCKET_ORDER) {
      const items = buckets.get(label);
      if (items?.length) out.push({ label, items: items.sort(byActivity) });
    }
    return out;
  });

  function relTime(ts: number): string {
    const s = Math.max(0, (Date.now() - ts) / 1000);
    if (s < 60) return "now";
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    if (s < 86_400) return `${Math.floor(s / 3600)}h`;
    if (s < 604_800) return `${Math.floor(s / 86_400)}d`;
    return `${Math.floor(s / 604_800)}w`;
  }

  // ── row state ────────────────────────────────────────────────────────
  // Convos with an in-flight turn — drives the per-row workdots shimmer.
  const workingIds = $derived(new Set(assistant.liveTabs.map((t) => t.convoId)));
  function isActive(id: string) { return assistant.currentConvoId === id && workspace.activeId === "chat"; }
  function isOpen(id: string) { return assistant.openTabs.includes(id) && !isActive(id); }

  // ── delayed hover preview ──────────────────────────────────────────────
  let preview = $state<{ c: ConversationMeta; x: number; y: number } | null>(null);
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;

  const PREVIEW_W = 268; // keep in sync with .conv-preview width
  function onRowEnter(e: MouseEvent, c: ConversationMeta) {
    if (hoverTimer) clearTimeout(hoverTimer);
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    hoverTimer = setTimeout(() => {
      // Flip to the row's left if the right side would run past the viewport,
      // then hard-clamp into [8, vw-W-8] so it can never render clipped.
      let x = r.right + 8;
      if (x + PREVIEW_W + 8 > window.innerWidth) x = r.left - PREVIEW_W - 8;
      x = Math.max(8, Math.min(x, window.innerWidth - PREVIEW_W - 8));
      const y = Math.max(8, Math.min(r.top, window.innerHeight - 150));
      preview = { c, x, y };
    }, 460);
  }
  function onRowLeave() {
    if (hoverTimer) { clearTimeout(hoverTimer); hoverTimer = null; }
    preview = null;
  }

  function open(id: string) {
    if (renaming) return;
    workspace.setActive("chat");
    void assistant.openTab(id).catch(console.error);
  }

  // ── row menu ─────────────────────────────────────────────────────────
  let menuId = $state<string | null>(null);
  let menuPos = $state({ x: 0, y: 0 });

  function openMenu(e: MouseEvent, id: string) {
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menuPos = { x: Math.min(r.left, window.innerWidth - 170), y: r.bottom + 4 };
    menuId = menuId === id ? null : id;
  }
  function closeMenu() { menuId = null; }

  // ── rename ───────────────────────────────────────────────────────────
  let renaming = $state<string | null>(null);
  let renameValue = $state("");

  function startRename(id: string, current: string) {
    closeMenu();
    renaming = id;
    renameValue = current;
  }
  function commitRename() {
    if (!renaming) return;
    const id = renaming, title = renameValue.trim();
    renaming = null;
    if (title) void assistant.renameConversation(id, title).catch(console.error);
  }
  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); commitRename(); }
    else if (e.key === "Escape") { e.preventDefault(); renaming = null; }
  }

  function del(id: string) {
    closeMenu();
    void assistant.deleteConversation(id).catch(console.error);
  }

  $effect(() => {
    if (menuId === null) return;
    const onDoc = () => closeMenu();
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") closeMenu(); };
    window.addEventListener("click", onDoc);
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("click", onDoc); window.removeEventListener("keydown", onKey); };
  });
</script>

<div class="conv-search" class:active={shell.convQuery.length > 0}>
  <Search size={13} />
  <input
    type="text"
    placeholder="Search conversations…"
    bind:value={shell.convQuery}
    spellcheck="false"
    aria-label="Search conversations"
  />
  {#if shell.convQuery}
    <button class="cs-clear" type="button" onclick={() => (shell.convQuery = "")} aria-label="Clear search"><X size={12} /></button>
  {/if}
</div>

{#if assistant.activeRoot}
  <div class="conv-scope" role="group" aria-label="Conversation scope">
    <button
      class="cscope-btn"
      class:on={!shell.allProjects}
      type="button"
      onclick={() => { if (shell.allProjects) shell.toggleAllProjects(); }}
      title={projLabel(assistant.activeRoot)}
    >This project</button>
    <button
      class="cscope-btn"
      class:on={shell.allProjects}
      type="button"
      onclick={() => { if (!shell.allProjects) shell.toggleAllProjects(); }}
    >All projects</button>
  </div>
{/if}

<div class="conv-list">
  {#if groups.length === 0}
    <div class="conv-empty">{shell.convQuery ? "No conversations match." : "No conversations yet.\nStart one from Home or Chat."}</div>
  {/if}
  {#each groups as g (g.label)}
    <div class="conv-group-label">{g.label}<span class="cgl-ct">{g.items.length}</span></div>
    {#each g.items as c (c.id)}
      <div
        class="crow"
        class:on={isActive(c.id)}
        class:open={isOpen(c.id)}
        class:menu-open={menuId === c.id}
        role="button"
        tabindex="0"
        onclick={() => open(c.id)}
        onkeydown={(e) => { if (e.key === "Enter") open(c.id); }}
        onmouseenter={(e) => onRowEnter(e, c)}
        onmouseleave={onRowLeave}
      >
        {#if workingIds.has(c.id)}
          <span class="workdots on" aria-label="Working">
            <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
          </span>
        {:else if shell.isPinned(c.id)}
          <span class="crow-pin"><Pin size={11} /></span>
        {:else if isOpen(c.id)}
          <span class="crow-open"><Circle size={7} fill="currentColor" /></span>
        {/if}

        {#if renaming === c.id}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="crow-rename"
            bind:value={renameValue}
            onkeydown={onRenameKey}
            onblur={commitRename}
            onclick={(e) => e.stopPropagation()}
            autofocus
            aria-label="Rename conversation"
          />
        {:else}
          <span class="crow-title">{c.title || "Untitled"}</span>
          {#if shell.allProjects}
            <span class="crow-proj" title={c.workspaceRoot ?? "Unfiled"}>{projLabel(c.workspaceRoot)}</span>
          {/if}
          <span class="crow-time">{relTime(c.updatedAt)}</span>
          <button class="crow-menu-btn" type="button" onclick={(e) => openMenu(e, c.id)} aria-label="Conversation actions">
            <MoreHorizontal size={14} />
          </button>
        {/if}
      </div>
    {/each}
  {/each}
</div>

{#if menuId}
  {@const id = menuId}
  {@const isP = shell.isPinned(id)}
  <div class="conv-menu" use:portal style="left:{menuPos.x}px; top:{menuPos.y}px" role="menu" tabindex="-1">
    <button class="pop-item" type="button" role="menuitem" onclick={(e) => { e.stopPropagation(); shell.togglePin(id); closeMenu(); }}>
      {#if isP}<PinOff size={13} />Unpin{:else}<Pin size={13} />Pin{/if}
    </button>
    <button class="pop-item" type="button" role="menuitem" onclick={(e) => { e.stopPropagation(); startRename(id, assistant.conversations.find(c => c.id === id)?.title ?? ""); }}>
      <Pencil size={13} />Rename
    </button>
    <button class="pop-item danger" type="button" role="menuitem" onclick={(e) => { e.stopPropagation(); del(id); }}>
      <Trash2 size={13} />Delete
    </button>
  </div>
{/if}

{#if preview && !menuId && !renaming}
  <div class="conv-preview" use:portal style="left:{preview.x}px; top:{preview.y}px" aria-hidden="true">
    <div class="cp-title">{preview.c.title || "Untitled"}</div>
    {#if preview.c.lastSnippet}
      <div class="cp-snip">{preview.c.lastSnippet}</div>
    {/if}
    <div class="cp-meta">
      <span class="cp-when">{relTime(preview.c.updatedAt)} ago</span>
    </div>
  </div>
{/if}

<style>
  .conv-search { display: flex; align-items: center; gap: 8px; height: 32px; margin: 2px 2px 7px; padding: 0 10px; flex: none;
    border-radius: 9px; background: var(--bg-inset); border: 1px solid var(--border); color: var(--fg-subtle);
    transition: border-color var(--dur-fast), background var(--dur-fast); }
  .conv-search:focus-within { border-color: var(--border-focus); background: var(--surface); box-shadow: 0 0 0 3px var(--ring); }
  .conv-search :global(svg) { flex: none; }
  .conv-search input { flex: 1; min-width: 0; border: 0; outline: 0; background: none; font-size: 12.5px; color: var(--fg); }
  .conv-search input::placeholder { color: var(--fg-subtle); }
  .cs-clear { display: grid; place-items: center; width: 18px; height: 18px; border-radius: 5px; color: var(--fg-faint); flex: none; }
  .cs-clear:hover { background: var(--surface-active); color: var(--fg-2); }

  /* per-project scope toggle — segmented control under the search box */
  .conv-scope { display: flex; gap: 2px; margin: 0 2px 6px; padding: 2px; flex: none;
    border-radius: 8px; background: var(--bg-inset); border: 1px solid var(--border); }
  .cscope-btn { flex: 1; min-width: 0; height: 24px; padding: 0 8px; border-radius: 6px;
    font-size: 11px; font-weight: 600; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    transition: background var(--dur-fast), color var(--dur-fast); }
  .cscope-btn:hover { color: var(--fg-2); }
  .cscope-btn.on { background: var(--surface-active); color: var(--fg); }

  /* per-row project label (All-projects mode only) */
  .crow-proj { flex: none; max-width: 84px; padding: 1px 6px; border-radius: 5px;
    font-size: 10px; font-weight: 600; color: var(--fg-faint); background: color-mix(in oklab, var(--fg) 7%, transparent);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .crow:hover .crow-proj, .crow.menu-open .crow-proj { display: none; }

  .conv-list { display: flex; flex-direction: column; gap: 1px; overflow-y: auto; min-height: 0; flex: 1; padding-bottom: 8px;
    scrollbar-width: thin; scrollbar-color: var(--border-strong) transparent; }
  .conv-list::-webkit-scrollbar { width: 8px; }
  .conv-list::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 8px; border: 2px solid transparent; background-clip: padding-box; }
  .conv-group-label { display: flex; align-items: center; gap: 7px; padding: 12px 8px 5px 11px; flex: none;
    font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); }
  .conv-group-label .cgl-ct { font-weight: 600; opacity: 0.65; font-variant-numeric: tabular-nums; }
  .conv-empty { padding: 22px 14px; text-align: center; font-size: 11.5px; line-height: 1.6; color: var(--fg-subtle); white-space: pre-line; }

  .crow { position: relative; display: flex; align-items: center; gap: 9px; height: 34px; padding: 0 6px 0 11px; flex: none;
    border-radius: 8px; color: var(--fg-muted); cursor: pointer; transition: background var(--dur-fast), color var(--dur-fast); }
  .crow:hover { background: var(--surface-hover); color: var(--fg-2); }
  .crow.on { background: color-mix(in oklab, var(--fg) 10%, transparent); color: var(--fg); }
  .crow.on::before { content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 3px; height: 15px; border-radius: 0 3px 3px 0; background: var(--accent); animation: barPop 0.28s var(--ease-page) both; }
  .crow.open:not(.on) { color: var(--fg-2); }
  .crow.open:not(.on)::before { content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 3px; height: 11px; border-radius: 0 3px 3px 0; background: color-mix(in oklab, var(--accent) 45%, transparent); }
  .crow-open { display: inline-flex; flex: none; color: var(--accent); opacity: 0.85; }
  .crow:hover .crow-open, .crow.menu-open .crow-open { display: none; }
  .crow-pin { display: inline-flex; flex: none; color: var(--accent); }
  .crow-title { flex: 1; min-width: 0; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: left; }
  .crow-time { font-size: 10.5px; color: var(--fg-faint); font-variant-numeric: tabular-nums; flex: none; opacity: 0.85; transition: opacity var(--dur-fast); }
  .crow-menu-btn { display: none; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: var(--fg-faint); flex: none; }
  .crow:hover .crow-time, .crow.menu-open .crow-time { display: none; }
  .crow:hover .crow-menu-btn, .crow.menu-open .crow-menu-btn { display: grid; }
  .crow-menu-btn:hover { background: var(--surface-active); color: var(--fg); }
  .crow-rename { flex: 1; min-width: 0; height: 25px; padding: 0 8px; border-radius: 6px; border: 1px solid var(--border-focus);
    background: var(--bg-inset); color: var(--fg); font-size: 12.5px; outline: 0; box-shadow: 0 0 0 3px var(--ring); }

  /* conv-menu + conv-preview portal to <body> (escape the sidebar's
     filter/transform containing block that was clipping the fixed preview),
     so their rules ride :global. */
  :global(.conv-menu) { position: fixed; z-index: 51; min-width: 150px; padding: 5px; border-radius: 12px;
    background: color-mix(in oklab, var(--bg-elev-2) 60%, transparent); -webkit-backdrop-filter: blur(26px) saturate(1.6); backdrop-filter: blur(26px) saturate(1.6);
    border: 1px solid color-mix(in oklab, var(--fg) 12%, transparent);
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.08), 0 24px 56px -26px oklch(0 0 0 / 0.7), var(--shadow-lg);
    animation: convPopIn 0.16s var(--ease-page) both; transform-origin: top left; }
  :global(.conv-menu .pop-item) { display: flex; align-items: center; gap: 9px; width: 100%; height: 32px; padding: 0 10px; border-radius: 8px;
    color: var(--fg-2); font-size: 12.5px; text-align: left; transition: background var(--dur-fast), color var(--dur-fast); }
  :global(.conv-menu .pop-item:hover) { background: var(--surface-hover); color: var(--fg); }
  :global(.conv-menu .pop-item svg) { color: var(--fg-faint); flex: none; }
  :global(.conv-menu .pop-item.danger:hover) { background: var(--danger-soft, color-mix(in oklab, var(--danger) 18%, transparent)); color: var(--danger); }
  :global(.conv-menu .pop-item.danger:hover svg) { color: var(--danger); }

  @keyframes barPop { from { transform: translateY(-50%) scaleY(0.25); } to { transform: translateY(-50%) scaleY(1); } }
  /* global so the portaled (re-parented) menu/preview can reference it by name */
  @keyframes -global-convPopIn { from { opacity: 0; transform: scale(0.97) translateY(-2px); } to { opacity: 1; transform: none; } }

  /* per-chat working indicator — 3×3 dot-grid shimmer (diagonal wave) */
  .workdots { display: grid; grid-template-columns: repeat(3, 3px); grid-auto-rows: 3px; gap: 2px;
    width: 13px; height: 13px; place-content: center; flex: none; }
  .workdots i { width: 3px; height: 3px; border-radius: 1.5px; background: var(--accent);
    animation: workPulse 1.05s var(--ease-soft, ease) infinite; }
  .workdots i:nth-child(1) { animation-delay: 0s; }
  .workdots i:nth-child(2), .workdots i:nth-child(4) { animation-delay: 0.09s; }
  .workdots i:nth-child(3), .workdots i:nth-child(5), .workdots i:nth-child(7) { animation-delay: 0.18s; }
  .workdots i:nth-child(6), .workdots i:nth-child(8) { animation-delay: 0.27s; }
  .workdots i:nth-child(9) { animation-delay: 0.36s; }
  @keyframes workPulse { 0%, 100% { opacity: 0.16; transform: scale(0.5); } 45% { opacity: 1; transform: scale(1); } }
  @media (prefers-reduced-motion: reduce) { .workdots i { animation: none; opacity: 0.9; transform: scale(1); } }

  /* delayed hover preview (fixed, portaled, anchored beside the row) */
  :global(.conv-preview) { position: fixed; z-index: 50; width: 268px; padding: 12px 13px; border-radius: 13px; pointer-events: none;
    background: color-mix(in oklab, var(--bg-elev-2) 94%, var(--bg)); border: 1px solid var(--border-strong);
    box-shadow: 0 28px 64px -30px oklch(0 0 0 / 0.7), var(--shadow-lg); animation: convPopIn 0.16s var(--ease-page) both; }
  :global(.conv-preview .cp-title) { font-size: 12.5px; font-weight: 650; color: var(--fg); margin-bottom: 6px; line-height: 1.32; text-wrap: pretty; }
  :global(.conv-preview .cp-snip) { font-size: 11.5px; color: var(--fg-muted); line-height: 1.5; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  :global(.conv-preview .cp-meta) { display: flex; align-items: center; gap: 12px; margin-top: 10px; padding-top: 9px; border-top: 1px solid var(--border);
    font-size: 10.5px; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  :global(.conv-preview .cp-when) { display: inline-flex; align-items: center; gap: 5px; }
</style>
