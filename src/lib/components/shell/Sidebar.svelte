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
          (c) => !assistant.activeRoot || rk(c.workspaceRoot) === rk(assistant.activeRoot),
        ).length,
  );
  function rk(r: string | null | undefined): string {
    return (r ?? "").replace(/[/\\]+$/, "").replace(/[/\\]/g, "/").toLowerCase();
  }

  // Sliding thumb under the active scope button — measured (labels differ in
  // width), re-measured on every scope flip.
  let segEl = $state<HTMLElement>();
  let segThumb = $state({ x: 2, w: 0 });
  $effect(() => {
    void shell.allProjects;
    const on = segEl?.querySelector<HTMLElement>(".seg-btn.on");
    if (on) segThumb = { x: on.offsetLeft, w: on.offsetWidth };
  });

  // ── rail resize ──────────────────────────────────────────────────────
  function startResize(e: PointerEvent) {
    e.preventDefault();
    shell.resizing = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function moveResize(e: PointerEvent) {
    if (!shell.resizing) return;
    // Rail is pinned to the window's left edge, so clientX is the new width.
    shell.setWidth(e.clientX);
  }
  function endResize(e: PointerEvent) {
    if (!shell.resizing) return;
    shell.resizing = false;
    shell.commitWidth();
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div
  class="side-rail"
  class:collapsed={shell.collapsed}
  class:resizing={shell.resizing}
  style="width:{shell.collapsed ? 52 : shell.width}px"
>
  <aside class="sidebar" class:home={isNavActive("home")} inert={shell.collapsed}>
    <div class="side-head" data-tauri-drag-region>
      <span class="brand">
        <RiftLogo size={22} class="brand-mk" />
        <span class="brand-name">Rift</span>
      </span>
      <button
        class="side-collapse"
        type="button"
        onclick={() => shell.toggleCollapsed()}
        use:tooltip={"Collapse sidebar"}
        aria-label="Collapse sidebar"
      >
        <PanelLeftClose size={16} />
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

  <!-- Collapsed mini-rail: icon-only column so collapse never costs the core
       affordances (new chat / search / nav). Always mounted; opacity+inert
       gated so the width tween and the rail cross-fade read as one motion.
       The brand slot doubles as the expand control (logo ⇢ open-icon on hover). -->
  <div class="mini" inert={!shell.collapsed}>
    <button
      class="mini-brand"
      type="button"
      onclick={() => shell.toggleCollapsed()}
      use:tooltip={"Open sidebar"}
      aria-label="Open sidebar"
    >
      <span class="mb-logo"><RiftLogo size={22} /></span>
      <span class="mb-open"><PanelLeftOpen size={16} /></span>
    </button>
    <button class="mini-btn nc" type="button" onclick={() => goHome()} use:tooltip={"New chat · Ctrl+N"} aria-label="New chat">
      <Plus size={17} strokeWidth={2.4} />
    </button>
    <button class="mini-btn" type="button" onclick={() => commandPalette.show()} use:tooltip={"Search chats · Ctrl+K"} aria-label="Search chats">
      <Search size={16} />
    </button>
    <span class="mini-spacer" aria-hidden="true"></span>
    {#each footNav as id (id)}
      {@const def = WORKSPACES[id]}
      <button
        class="mini-btn nav"
        class:on={isNavActive(id)}
        type="button"
        onclick={() => goto(id)}
        use:tooltip={def.title}
        aria-label={def.title}
        aria-current={isNavActive(id) ? "page" : undefined}
      >
        <def.icon size={17} />
      </button>
    {/each}
    <button
      class="mini-btn nav"
      class:on={isNavActive("settings")}
      type="button"
      onclick={() => goto("settings")}
      use:tooltip={"Settings"}
      aria-label="Settings"
      aria-current={isNavActive("settings") ? "page" : undefined}
    >
      <WORKSPACES.settings.icon size={17} />
    </button>
  </div>

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
  .side-rail { position: relative; flex: none; min-width: 0; overflow: hidden;
    transition: width 0.36s var(--ease-page); }
  .side-rail.resizing, .side-rail.resizing .sidebar { transition: none; }
  .side-rail.collapsed .sidebar { transform: translateX(-18px); opacity: 0; pointer-events: none; }
  .side-rail.collapsed .side-resize { display: none; }

  /* ── collapsed mini-rail ── an icon column that fades in as the full rail
     fades out; slight delay so the two never overlap mid-tween. */
  .mini { position: absolute; inset: 0; z-index: 5; display: flex; flex-direction: column; align-items: center; gap: 5px;
    padding: 8px 0 10px; border-right: 1px solid var(--border); box-sizing: border-box;
    background: linear-gradient(180deg, color-mix(in oklab, var(--fg) 3.6%, var(--bg)), color-mix(in oklab, var(--fg) 1.6%, var(--bg)) 260px);
    opacity: 0; pointer-events: none; transform: translateX(-8px);
    transition: opacity 0.2s var(--ease-page), transform 0.26s var(--ease-page); }
  .side-rail.collapsed .mini { opacity: 1; pointer-events: auto; transform: none; transition-delay: 0.1s; }
  .mini button { -webkit-app-region: no-drag; }

  .mini-brand { position: relative; width: 36px; height: 36px; display: grid; place-items: center; border-radius: 9px; flex: none;
    transition: background var(--dur-fast); }
  .mini-brand .mb-logo, .mini-brand .mb-open { grid-area: 1 / 1; display: grid; place-items: center; transition: opacity var(--dur-fast); }
  .mini-brand :global(.mb-logo svg) { border-radius: 7px; display: block; }
  .mini-brand .mb-open { opacity: 0; color: var(--fg-2); }
  .mini-brand:hover { background: var(--surface-hover); }
  .mini-brand:hover .mb-logo { opacity: 0; }
  .mini-brand:hover .mb-open { opacity: 1; }

  .mini-btn { width: 36px; height: 36px; flex: none; display: grid; place-items: center; border-radius: 9px;
    color: var(--fg-muted); transition: background var(--dur-fast), color var(--dur-fast); }
  .mini-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .mini-btn:active { transform: translateY(1px); }
  .mini-btn.nc { color: var(--accent); background: color-mix(in oklab, var(--accent) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 24%, transparent); }
  .mini-btn.nc:hover { background: color-mix(in oklab, var(--accent) 18%, transparent); }
  .mini-btn.nav.on { color: var(--accent); background: color-mix(in oklab, var(--fg) 8%, transparent); }
  .mini-spacer { flex: 1; }
  @media (prefers-reduced-motion: reduce) { .mini { transition: none; } }

  .side-resize { position: absolute; top: 0; right: 0; width: 8px; height: 100%; z-index: 6; cursor: col-resize; -webkit-app-region: no-drag; }
  .side-resize::after { content: ""; position: absolute; top: 0; right: 0; width: 2px; height: 100%; background: transparent; transition: background var(--dur-fast); }
  .side-resize:hover::after, .side-rail.resizing .side-resize::after { background: var(--accent); }

  .sidebar { width: 100%; height: 100%; flex: none; display: flex; flex-direction: column; gap: 4px; min-height: 0;
    padding: 8px 10px 10px; border-right: 1px solid var(--border);
    background: linear-gradient(180deg, color-mix(in oklab, var(--fg) 3.6%, var(--bg)), color-mix(in oklab, var(--fg) 1.6%, var(--bg)) 260px);
    box-shadow: inset 0 1px 0 color-mix(in oklab, var(--fg) 4%, transparent);
    box-sizing: border-box; transition: transform 0.36s var(--ease-page), opacity 0.26s var(--ease-page); }
  .sidebar.home { background: color-mix(in oklab, var(--bg) 72%, transparent);
    backdrop-filter: blur(18px) saturate(1.1); -webkit-backdrop-filter: blur(18px) saturate(1.1);
    border-right-color: color-mix(in oklab, var(--border) 70%, transparent); }
  .sidebar button { -webkit-app-region: no-drag; }

  .side-head { display: flex; align-items: center; justify-content: space-between; height: 40px; padding: 0 6px 0 8px; flex: none; -webkit-app-region: drag; }
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
    transition: transform 0.18s var(--ease-page), width 0.18s var(--ease-page); }
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
