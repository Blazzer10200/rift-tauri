<script lang="ts">
  import { Pin, PinOff, MoreHorizontal, Pencil, Trash2, ChevronDown } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { projects } from "$lib/state/projects.svelte";
  import type { ConversationMeta } from "$lib/state/assistant/types";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { leafName, rootKey } from "$lib/utils/path";

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
  // `rootKey` (utils/path.ts) canonicalizes for a case-insensitive match so a
  // convo's stamped root lines up with the open folder regardless of casing.
  /** Short project label for a root: the defined project's name if one owns the
   *  folder, else the folder basename. */
  function projLabel(r: string | null | undefined): string {
    const named = projects.byRoot(r);
    if (named) return named.name;
    return r ? leafName(r) : "Unfiled";
  }
  const activeKey = $derived(rootKey(assistant.activeRoot));

  const groups = $derived.by<Group[]>(() => {
    const now = Date.now();
    // Actively-streaming convos pin to the top of their bucket — clicking another
    // chat must NOT shove the live turn below it.
    const working = new Set(assistant.liveTabs.map((t) => t.convoId));
    // STABLE ORDER — the whole point of this comparator. The list must not
    // reshuffle just because you clicked between chats. Two rules make that hold:
    //  1. Rank by `lastActivityAt`, which advances ONLY on a real turn (send /
    //     result) — NEVER on open/switch/auto-save. We deliberately do NOT fall
    //     back to `updatedAt`: every open re-saves the tab and bumps updatedAt
    //     (often several tabs to the same millisecond), which is exactly what
    //     was causing the spam-click reshuffle. Legacy records with no
    //     lastActivityAt fall back to `createdAt` — fixed at creation, so their
    //     position is permanent.
    //  2. `createdAt` is the tiebreaker, so equal ranks have a deterministic,
    //     render-stable order (a plain `b-a` of equal values is unstable and
    //     visibly jitters on re-render).
    const act = (c: ConversationMeta) => c.lastActivityAt ?? c.createdAt;
    const byActivity = (a: ConversationMeta, b: ConversationMeta) => {
      const aw = working.has(a.id), bw = working.has(b.id);
      if (aw !== bw) return aw ? -1 : 1;
      const d = act(b) - act(a);
      return d !== 0 ? d : b.createdAt - a.createdAt;
    };
    const filtered = assistant.conversations.filter((c) => {
      // Scope to the open project unless the user flipped to All-projects, or
      // there's no open folder (activeKey === "" → show everything so chats are
      // never stranded). A convo whose root matches the open folder is in-scope.
      if (!shell.allProjects && activeKey && rootKey(c.workspaceRoot) !== activeKey) return false;
      return true;
    });
    const pinned: ConversationMeta[] = [];
    const buckets = new Map<string, ConversationMeta[]>();
    for (const c of filtered) {
      if (shell.isPinned(c.id)) { pinned.push(c); continue; }
      const b = timeBucket(act(c), now);
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

  // Default view = the most recent day's chats, flat, no header — everything
  // older tucks behind one "Show earlier" row. "Recent" = Pinned + the FIRST
  // non-pinned bucket (Today if it has items, else Yesterday, else whatever the
  // newest bucket is). This avoids the "list looks empty" trap when nothing was
  // touched today. Pinned always shows on top.
  const recentGroups = $derived.by(() => {
    const pinned = groups.filter((g) => g.label === "Pinned");
    const firstDay = groups.find((g) => g.label !== "Pinned");
    return firstDay ? [...pinned, firstDay] : pinned;
  });
  const recentLabels = $derived(new Set(recentGroups.map((g) => g.label)));
  const olderGroups = $derived(groups.filter((g) => !recentLabels.has(g.label)));
  const olderCount = $derived(olderGroups.reduce((n, g) => n + g.items.length, 0));

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
  // Live-turn elapsed timer — beginTurn stamps activity.turnStartedAt; a 1s
  // tick (running only while something streams) drives the readout.
  const liveStarts = $derived(new Map(assistant.liveTabs.map((t) => [t.convoId, t.tab.activity.turnStartedAt])));
  let nowTick = $state(Date.now());
  $effect(() => {
    if (workingIds.size === 0) return;
    nowTick = Date.now();
    const iv = setInterval(() => { nowTick = Date.now(); }, 1000);
    return () => clearInterval(iv);
  });
  function fmtElapsed(start: number | null | undefined): string {
    if (!start) return "";
    const s = Math.max(0, Math.floor((nowTick - start) / 1000));
    const m = Math.floor(s / 60);
    if (m >= 60) return `${Math.floor(m / 60)}h ${m % 60}m`;
    return `${m}:${String(s % 60).padStart(2, "0")}`;
  }
  function isActive(id: string) { return assistant.currentConvoId === id && workspace.activeId === "chat"; }

  function open(id: string) {
    if (renaming) return;
    workspace.setActive("chat");
    void assistant.openTab(id).catch(console.error);
  }

  // ── drag a conversation into a pane ──────────────────────────────────
  // Sets assistant.draggingTabId so AssistantPane's drop targets light up;
  // dropTabIntoPane opens the convo if it isn't already a tab. Switch to the
  // chat view on drop-start so the panes are actually visible to drop onto.
  function onRowDragStart(e: DragEvent, id: string) {
    if (renaming) { e.preventDefault(); return; }
    assistant.draggingTabId = id;
    workspace.setActive("chat");
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", id); // some engines need a payload to start a drag
    }
  }
  function onRowDragEnd() {
    assistant.draggingTabId = null;
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

{#snippet convRow(c: ConversationMeta, i: number)}
  <div
    class="crow"
    style="--i:{i}"
    class:on={isActive(c.id)}
    class:pinned={shell.isPinned(c.id)}
    class:menu-open={menuId === c.id}
    role="button"
    tabindex="0"
    draggable={renaming === c.id ? "false" : "true"}
    ondragstart={(e) => onRowDragStart(e, c.id)}
    ondragend={onRowDragEnd}
    onclick={() => open(c.id)}
    onkeydown={(e) => { if (e.key === "Enter") open(c.id); }}
  >
    {#if workingIds.has(c.id)}
      <span class="workdots on" aria-label="Working">
        <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
      </span>
    {:else if shell.isPinned(c.id)}
      <span class="crow-pin"><Pin size={11} /></span>
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
      {#if workingIds.has(c.id)}
        <span class="crow-time busy">{fmtElapsed(liveStarts.get(c.id))}</span>
      {:else}
        <span class="crow-time">{relTime(c.lastActivityAt ?? c.createdAt)}</span>
      {/if}
      <span class="crow-acts">
        <button
          class="crow-act"
          type="button"
          onclick={(e) => { e.stopPropagation(); shell.togglePin(c.id); }}
          use:tooltip={shell.isPinned(c.id) ? "Unpin" : "Pin"}
          aria-label={shell.isPinned(c.id) ? "Unpin conversation" : "Pin conversation"}
        >
          {#if shell.isPinned(c.id)}<PinOff size={13} />{:else}<Pin size={13} />{/if}
        </button>
        <button class="crow-act" type="button" onclick={(e) => openMenu(e, c.id)} aria-label="Conversation actions">
          <MoreHorizontal size={14} />
        </button>
      </span>
    {/if}
  </div>
{/snippet}

<div class="conv-list">
  {#if groups.length === 0}
    <div class="conv-empty">No conversations yet.{"\n"}Start one from Home or Chat.</div>
  {/if}

  <!-- Most-recent day (+ pinned) render FLAT, no date header — implied default.
       Keyed on the scope toggle so flipping This/All remounts the rows and
       replays the stagger-in cascade. -->
  {#key shell.allProjects}
    {#each recentGroups as g (g.label)}
      {#if g.label === "Pinned"}<div class="conv-group-label plain"><span class="cgl-txt">Pinned</span></div>{/if}
      {#each g.items as c, i (c.id)}{@render convRow(c, i)}{/each}
    {/each}
  {/key}

  <!-- Everything older tucks behind one quiet "Show earlier" toggle. -->
  {#if olderCount > 0}
    <button class="showmore conv-showmore" class:x={shell.historyExpanded} type="button" onclick={() => shell.toggleHistoryExpanded()}>
      <span class="sm-rule" aria-hidden="true"></span>
      <span class="sm-lbl">
        <ChevronDown size={12} class="sm-ch" />
        {shell.historyExpanded ? "Show less" : "Show earlier"}
        {#if !shell.historyExpanded}<span class="sm-ct">{olderCount}</span>{/if}
      </span>
      <span class="sm-rule" aria-hidden="true"></span>
    </button>
    {#if shell.historyExpanded}
      {#each olderGroups as g (g.label)}
        <div class="conv-group-label plain"><span class="cgl-txt">{g.label}</span><span class="cgl-ct">{g.items.length}</span></div>
        {#each g.items as c, i (c.id)}{@render convRow(c, i)}{/each}
      {/each}
    {/if}
  {/if}
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

<style>
  /* "show earlier/less" affordance — a fold line in the timeline: hairline on
     each side of a centered chevron + label + count. Reads as the seam where
     older history is tucked, not a dangling text row. */
  .showmore { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 6px 6px;
    font-size: 11px; font-weight: 550; color: var(--fg-subtle); transition: color var(--dur-fast); }
  .showmore .sm-rule { flex: 1; height: 1px; background: color-mix(in oklab, var(--border) 55%, transparent);
    transition: background var(--dur-fast); }
  .showmore .sm-lbl { display: inline-flex; align-items: center; gap: 5px; flex: none; }
  .showmore :global(.sm-ch) { color: var(--fg-faint); transition: transform 0.22s var(--ease-page), color var(--dur-fast); }
  .showmore.x :global(.sm-ch) { transform: rotate(180deg); }
  .showmore:hover { color: var(--fg-2); }
  .showmore:hover .sm-rule { background: color-mix(in oklab, var(--fg) 14%, transparent); }
  .showmore:hover :global(.sm-ch) { color: var(--fg-2); }
  .sm-ct { font-size: 9.5px; font-weight: 600; padding: 1px 6px; border-radius: 999px; color: var(--fg-faint);
    background: color-mix(in oklab, var(--fg) 7%, transparent); font-variant-numeric: tabular-nums;
    transition: color var(--dur-fast); }
  .showmore:hover .sm-ct { color: var(--fg-2); }
  @media (prefers-reduced-motion: reduce) { .showmore :global(.sm-ch) { transition: transform 0s; } }

  /* per-row project label (All-projects mode only) */
  .crow-proj { flex: none; max-width: 84px; padding: 1px 6px; border-radius: 5px;
    font-size: 10px; font-weight: 600; color: var(--fg-faint); background: color-mix(in oklab, var(--fg) 7%, transparent);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .crow:hover .crow-proj, .crow.menu-open .crow-proj { display: none; }

  .conv-list { display: flex; flex-direction: column; gap: 1px; overflow-y: auto; min-height: 0; flex: 1; padding: 8px 0;
    margin-top: 4px; border-top: 1px solid color-mix(in oklab, var(--border) 45%, transparent);
    scrollbar-width: thin; scrollbar-color: var(--border-strong) transparent; }
  .conv-list::-webkit-scrollbar { width: 8px; }
  .conv-list::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 8px; border: 2px solid transparent; background-clip: padding-box; }
  /* Sticky group headers — Pinned / Today / Older stay pinned to the top of the
     scroll container so the current bucket is always identifiable deep in a long
     list. Tinted+blurred backing so rows scrolling under them stay legible. */
  .conv-group-label { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; gap: 7px; width: 100%; padding: 10px 8px 4px 11px; flex: none;
    font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); text-align: left;
    background: linear-gradient(180deg, color-mix(in oklab, var(--fg) 3%, var(--bg)) 72%, transparent);
    -webkit-backdrop-filter: blur(3px); backdrop-filter: blur(3px); }
  .conv-group-label .cgl-txt { flex: 1; }
  .conv-group-label .cgl-ct { font-weight: 600; opacity: 0.55; font-variant-numeric: tabular-nums; }
  .conv-empty { padding: 22px 14px; text-align: center; font-size: 11.5px; line-height: 1.6; color: var(--fg-subtle); white-space: pre-line; }

  .crow { position: relative; display: flex; align-items: center; gap: 9px; height: 34px; padding: 0 6px 0 11px; flex: none;
    border-radius: 8px; color: var(--fg-muted); cursor: pointer; transition: background var(--dur-fast), color var(--dur-fast);
    animation: rowIn 0.22s var(--ease-page) both; animation-delay: calc(min(var(--i, 0), 12) * 14ms); }
  .crow:hover { background: var(--surface-hover); color: var(--fg-2); }
  .crow.on { background: color-mix(in oklab, var(--fg) 10%, transparent); color: var(--fg); }
  .crow.on::before { content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 3px; height: 15px; border-radius: 0 3px 3px 0; background: var(--accent); animation: barPop 0.28s var(--ease-page) both; }
  /* Pinned rows read as a different KIND of row than the time buckets: a faint
     accent wash + the pin glyph already rendered in the lead slot. Kept below the
     active-row treatment so an active pinned row still wins. */
  .crow.pinned:not(.on) { background: color-mix(in oklab, var(--accent) 5%, transparent); }
  .crow.pinned:not(.on):hover { background: color-mix(in oklab, var(--accent) 9%, transparent); }
  .crow-pin { display: inline-flex; flex: none; color: var(--accent); }
  .crow-title { flex: 1; min-width: 0; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: left; }
  .crow-time { font-size: 10.5px; color: var(--fg-faint); font-variant-numeric: tabular-nums; flex: none; opacity: 0.85; transition: opacity var(--dur-fast); }
  /* live-turn elapsed — activity color (§9: running = busy, not accent) */
  .crow-time.busy { color: var(--status-busy); opacity: 1; font-weight: 600; }
  /* Resting rows stay calm (title + relative time). On hover the time swaps for
     quick actions — pin + more (rename/delete live behind more). */
  .crow-acts { display: none; align-items: center; gap: 1px; flex: none; }
  .crow-act { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: var(--fg-faint); flex: none;
    transition: background var(--dur-fast), color var(--dur-fast); }
  .crow-act:hover { background: var(--surface-active); color: var(--fg); }
  .crow:hover .crow-time, .crow.menu-open .crow-time { display: none; }
  .crow:hover .crow-acts, .crow.menu-open .crow-acts { display: flex; animation: actsIn 0.14s var(--ease-page) both; }
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
  /* row entrance — a short cascade (capped at 12 rows so deep lists don't lag) */
  @keyframes rowIn { from { opacity: 0; transform: translateY(3px); } to { opacity: 1; transform: none; } }
  /* hover actions ease in instead of popping (display swap retriggers this) */
  @keyframes actsIn { from { opacity: 0; transform: translateX(5px); } to { opacity: 1; transform: none; } }
  @media (prefers-reduced-motion: reduce) { .crow, .crow-acts { animation: none !important; } }
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
</style>
