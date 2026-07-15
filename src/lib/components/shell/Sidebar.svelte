<script lang="ts">
  import { PanelLeftClose, PanelLeftOpen, Plus, Search } from "lucide-svelte";
  import { workspace, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { commandPalette } from "$lib/state/command-palette.svelte";
  import { WORKSPACES } from "../workspaces";
  import RiftLogo from "./RiftLogo.svelte";
  import ConversationList from "./ConversationList.svelte";
  import ProjectSwitcher from "./ProjectSwitcher.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { goHome } from "$lib/state/nav";
  import { rootKey } from "$lib/utils/path";

  // Footer icon nav — the destinations that used to stack as full rows above the
  // history now collapse into ONE compact icon strip: Workspace · Chat · AI
  // Health left, Settings right. Model + connection state deliberately live
  // elsewhere (composer model pill / app status bar) — no duplicate readouts.
  // Excludes legacy/disabled ids.
  const footNav = $derived(
    workspace.order.filter(
      (id) => id !== "settings" && id !== "projects" && !WORKSPACES[id].disabled,
    ),
  );

  function isNavActive(id: WorkspaceId): boolean {
    return workspace.activeId === id;
  }

  // Per-icon hover micro-motion hook (CSS targets .snav-ic-<key>).
  const ICON_KEY: Record<WorkspaceId, string> = {
    home: "home", chat: "chat", projects: "projects", "local-llm": "local", settings: "settings", "ai-health": "health",
  };

  function goto(id: WorkspaceId) {
    workspace.setActive(id);
  }

  // ── scope segment data ────────────────────────────────────────────────
  // Live count of chats in the current scope — mirrors ConversationList's own
  // filter so the number the segment shows matches the list below it.
  const scopeCount = $derived(
    shell.allProjects
      ? assistant.conversations.length
      : assistant.conversations.filter(
          (c) => !assistant.activeRoot || rootKey(c.workspaceRoot) === rootKey(assistant.activeRoot),
        ).length,
  );

  // Sliding thumb under the active scope button — measured (labels differ in
  // width). A one-shot measure dislocates whenever layout shifts AFTER it ran
  // (font swap, project-switch re-render, rail resize), so a ResizeObserver on
  // the segment re-measures on any size change; zero-width reads (hidden /
  // mid-transition) are discarded rather than committed.
  let segEl = $state<HTMLElement>();
  let segThumb = $state({ x: 2, w: 0 });
  $effect(() => {
    void shell.allProjects;
    const seg = segEl;
    if (!seg) return;
    const measure = () => {
      const on = seg.querySelector<HTMLElement>(".seg-btn.on");
      if (on && on.offsetWidth > 0) segThumb = { x: on.offsetLeft, w: on.offsetWidth };
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(seg);
    return () => ro.disconnect();
  });

  // ── rail resize ──────────────────────────────────────────────────────
  function startResize(e: PointerEvent) {
    e.preventDefault();
    shell.resizing = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function moveResize(e: PointerEvent) {
    if (!shell.resizing) return;
    // The island's right edge sits 8px inside the rail's right edge, so the
    // rail width that puts the island edge under the cursor is clientX + 8.
    shell.setWidth(e.clientX + 8);
  }
  function endResize(e: PointerEvent) {
    if (!shell.resizing) return;
    shell.resizing = false;
    shell.commitWidth();
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<!-- Floating island: the rail is only a width spacer in the flex row; the
     island card is absolutely positioned inside it, inset from the window
     edges. Pinned → rail holds layout width and the island reads as furniture
     (lift + hairline). Collapsed → rail width 0, island slides out; hovering
     the topbar trigger peeks it back OVER the content as a true floating
     layer (shadow), and a click pins it. -->
<div
  class="side-rail"
  role="presentation"
  class:collapsed={shell.collapsed}
  class:peeking={shell.collapsed && shell.peek}
  class:resizing={shell.resizing}
  style="width:{shell.collapsed ? 0 : shell.width}px"
  onmouseenter={() => { if (shell.collapsed && shell.peek) shell.cancelPeekClose(); }}
  onmouseleave={() => { if (shell.collapsed && shell.peek) shell.schedulePeekClose(); }}
>
  <aside
    class="sidebar"
    class:home={isNavActive("home")}
    inert={shell.collapsed && !shell.peek}
    style="width:{shell.width - 16}px"
  >
    <div class="side-head" data-tauri-drag-region>
      <span class="brand">
        <RiftLogo size={22} class="brand-mk" />
        <span class="brand-name">Rift</span>
      </span>
      <button
        class="side-collapse"
        type="button"
        onclick={() => shell.toggleCollapsed()}
        use:tooltip={shell.collapsed ? "Pin sidebar open" : "Collapse sidebar"}
        aria-label={shell.collapsed ? "Pin sidebar open" : "Collapse sidebar"}
      >
        {#if shell.collapsed}
          <PanelLeftOpen size={16} />
        {:else}
          <PanelLeftClose size={16} />
        {/if}
      </button>
    </div>

    <ProjectSwitcher />

    <div class="action-row">
      <button class="new-chat" type="button" onclick={() => goHome()} use:tooltip={"New chat · Ctrl+N"}>
        <span class="nc-ic"><Plus size={16} strokeWidth={2.4} /></span>
        <span class="nc-lbl">New chat</span>
      </button>
      <button class="ic-btn" type="button" onclick={() => commandPalette.show()} use:tooltip={"Search chats · Ctrl+K"} aria-label="Search chats">
        <Search size={16} />
      </button>
    </div>

    <!-- Scope segment: This project / All (binds shell.allProjects) + live count. -->
    <div class="tool-row">
      <div class="seg" role="group" aria-label="Conversation scope" bind:this={segEl}>
        <span class="seg-thumb" aria-hidden="true" style="transform:translateX({segThumb.x}px); width:{segThumb.w}px"></span>
        <button class="seg-btn" class:on={!shell.allProjects} type="button" onclick={() => shell.setAllProjects(false)} aria-pressed={!shell.allProjects}>This project</button>
        <button class="seg-btn" class:on={shell.allProjects} type="button" onclick={() => shell.setAllProjects(true)} aria-pressed={shell.allProjects}>All</button>
      </div>
      <span class="count-note">{scopeCount} {scopeCount === 1 ? "chat" : "chats"}</span>
    </div>

    <div class="side-sec">
      <ConversationList />
    </div>

    <nav class="foot-nav" aria-label="Workspaces">
      {#each footNav as id (id)}
        {@const def = WORKSPACES[id]}
        <button
          class="fnav-item"
          class:on={isNavActive(id)}
          type="button"
          onclick={() => goto(id)}
          use:tooltip={def.title}
          aria-label={def.title}
          aria-current={isNavActive(id) ? "page" : undefined}
        >
          <def.icon class="snav-ic snav-ic-{ICON_KEY[id]}" size={17} />
        </button>
      {/each}
      <span class="fnav-spacer" aria-hidden="true"></span>
      <button
        class="fnav-item"
        class:on={isNavActive("settings")}
        type="button"
        onclick={() => goto("settings")}
        use:tooltip={"Settings"}
        aria-label="Settings"
        aria-current={isNavActive("settings") ? "page" : undefined}
      >
        <WORKSPACES.settings.icon class="snav-ic snav-ic-settings" size={17} />
      </button>
    </nav>
  </aside>

  <div
    class="side-resize"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize sidebar"
    onpointerdown={startResize}
    onpointermove={moveResize}
    onpointerup={endResize}
    onpointercancel={endResize}
  ></div>
</div>

<style>
  /* Rail = layout spacer + positioning context. overflow stays visible so the
     absolutely-positioned island can float over the content while peeking. */
  .side-rail { position: relative; z-index: 30; flex: none; min-width: 0;
    transition: width 0.36s var(--ease-page); }
  .side-rail.resizing, .side-rail.resizing .sidebar { transition: none; }
  .side-rail.collapsed .sidebar { transform: translateX(-16px); opacity: 0; pointer-events: none; }
  /* Peek: same island, now a genuinely floating layer → it earns a shadow.
     Starts BELOW the topbar so the trigger cluster stays visible and hoverable
     (island covering the trigger would flicker the hover state). */
  .side-rail.peeking .sidebar { transform: none; opacity: 1; pointer-events: auto;
    top: 44px;
    box-shadow: var(--shadow-float); }
  .side-rail.collapsed .side-resize { display: none; }

  /* Handle rides the island's right edge (8px inside the rail). */
  .side-resize { position: absolute; top: 48px; right: 4px; width: 8px; height: calc(100% - 64px); z-index: 6; cursor: col-resize; -webkit-app-region: no-drag; }
  .side-resize::after { content: ""; position: absolute; top: 0; right: 3px; width: 2px; height: 100%; border-radius: 2px; background: transparent; transition: background var(--dur-fast); }
  .side-resize:hover::after, .side-rail.resizing .side-resize::after { background: var(--accent); }

  /* The island card — inset from the window edges, rounded, hairline-bordered.
     Pinned it reads as furniture (lift, no shadow); .peeking above adds the
     float shadow when it truly hovers over content. */
  .sidebar { position: absolute; top: 8px; bottom: 8px; left: 8px;
    display: flex; flex-direction: column; gap: 4px; min-height: 0;
    padding: 0 10px 10px;
    border-radius: 14px;
    border: 1px solid color-mix(in oklab, var(--border) 92%, transparent);
    background: linear-gradient(180deg, color-mix(in oklab, var(--fg) 4%, var(--bg)), color-mix(in oklab, var(--fg) 1.8%, var(--bg)) 280px);
    overflow: hidden;
    box-sizing: border-box;
    transition: transform 0.36s var(--ease-page), top 0.36s var(--ease-page), opacity var(--dur-base) var(--ease-page), box-shadow var(--dur-base) var(--ease-page); }
  .sidebar.home { background: color-mix(in oklab, var(--bg) 72%, transparent);
    backdrop-filter: blur(18px) saturate(1.1); -webkit-backdrop-filter: blur(18px) saturate(1.1); }
  .sidebar button { -webkit-app-region: no-drag; }

  .side-head { display: flex; align-items: center; justify-content: space-between; height: 40px; margin-bottom: 8px; padding: 0 6px 0 8px; flex: none; -webkit-app-region: drag; }
  .brand { display: inline-flex; align-items: center; gap: 9px; }
  .brand :global(.brand-mk) { border-radius: 7px; display: block; }
  .brand-name { font-size: 14px; font-weight: 650; letter-spacing: -0.012em; color: var(--fg); }
  .side-collapse { width: 28px; height: 28px; display: grid; place-items: center; border-radius: 7px; flex: none;
    color: var(--fg-faint); transition: background var(--dur-fast), color var(--dur-fast); }
  .side-collapse:hover { background: var(--surface-hover); color: var(--fg-2); }

  /* shared icon micro-motion hooks (used by the footer icon nav + status gear) */
  :global(.snav-ic) { transition: transform 0.34s var(--ease-page); transform-origin: 50% 50%; }

  /* action row — New chat (primary, grows) + a search icon-button share one row
     to save vertical space (opens the command palette scoped to chats). */
  .action-row { display: flex; align-items: center; gap: 6px; flex: none; margin: 2px 0; }
  .action-row .new-chat { flex: 1; margin: 0; }
  .ic-btn { width: 38px; height: 38px; flex: none; display: grid; place-items: center; border-radius: 10px;
    color: var(--fg-muted); border: 1px solid var(--border); background: var(--bg-inset);
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .ic-btn:hover { background: var(--surface-hover); color: var(--fg); border-color: var(--border-strong); }

  /* scope toolrow — This project / All segment + live chat count. */
  .tool-row { display: flex; align-items: center; justify-content: space-between; padding: 4px 2px 2px; flex: none; }
  .seg { position: relative; display: inline-flex; padding: 2px; gap: 2px; border-radius: 8px; border: 1px solid var(--border); background: var(--bg-inset); }
  .seg-thumb { position: absolute; top: 2px; left: 0; height: 22px; border-radius: 6px; background: var(--surface-active);
    transition: transform var(--dur-fast) var(--ease-page), width var(--dur-fast) var(--ease-page); }
  .seg-btn { position: relative; z-index: 1; height: 22px; padding: 0 10px; border-radius: 6px; color: var(--fg-subtle); font-size: 11px; font-weight: 600;
    transition: color var(--dur-fast); }
  .seg-btn:hover { color: var(--fg-2); }
  .seg-btn.on { color: var(--fg); }
  @media (prefers-reduced-motion: reduce) { .seg-thumb { transition: none; } }
  .count-note { font-size: 10.5px; font-weight: 500; color: var(--fg-faint); font-variant-numeric: tabular-nums; padding-right: 2px; }

  /* footer icon nav — destinations collapse from full rows to a compact icon
     strip so the conversation list owns nearly the whole rail. */
  .foot-nav { display: flex; align-items: center; gap: 2px; flex: none; padding-top: 8px; margin-top: 4px;
    border-top: 1px solid color-mix(in oklab, var(--border) 60%, transparent); }
  .fnav-item { position: relative; width: 40px; height: 34px; display: grid; place-items: center; border-radius: 8px;
    color: var(--fg-muted); transition: background var(--dur-fast), color var(--dur-fast), transform var(--dur-fast); }
  .fnav-item:active { transform: scale(0.92); }
  .fnav-item:hover { background: var(--surface-hover); color: var(--fg-2); }
  .fnav-item.on { color: var(--accent); background: color-mix(in oklab, var(--fg) 8%, transparent); }
  .fnav-item:hover :global(.snav-ic-home)     { transform: translateY(-2.5px) scale(1.06); }
  .fnav-item:hover :global(.snav-ic-chat)     { transform: rotate(-10deg) scale(1.06); }
  .fnav-item:hover :global(.snav-ic-health)   { transform: scale(1.1); }
  .fnav-item:hover :global(.snav-ic-settings) { transform: rotate(140deg); }
  .fnav-spacer { flex: 1; }

  /* primary action — New chat. Reads as primary via an accent-tinted surface +
     accent icon, not a saturated slab, so it stays in the rail's soft language. */
  .new-chat { display: flex; align-items: center; gap: 10px; height: 38px; margin: 2px 0 8px; padding: 0 10px 0 11px; flex: none;
    border-radius: 10px; font-size: 13px; font-weight: 580; color: var(--fg);
    border: 1px solid color-mix(in oklab, var(--accent) 28%, var(--border));
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 12%, transparent), color-mix(in oklab, var(--accent) 5%, transparent));
    transition: background var(--dur-fast), border-color var(--dur-fast), transform var(--dur-fast); }
  .new-chat:hover { background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 18%, transparent), color-mix(in oklab, var(--accent) 9%, transparent));
    border-color: color-mix(in oklab, var(--accent) 44%, var(--border)); }
  .new-chat:active { transform: translateY(1px); }
  .new-chat .nc-ic { display: inline-flex; flex: none; }
  .new-chat .nc-lbl { flex: 1; text-align: left; }
  .new-chat :global(svg) { flex: none; color: var(--accent); }

  /* conversation-list section wrapper */
  .side-sec { display: flex; flex-direction: column; flex: 1; min-height: 0; margin-top: 2px; }

  @keyframes barPop { from { transform: translateY(-50%) scaleY(0.25); } to { transform: translateY(-50%) scaleY(1); } }
</style>
