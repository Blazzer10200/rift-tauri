<script lang="ts">
  import { PanelLeftClose, Plus } from "lucide-svelte";
  import { workspace, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { WORKSPACES } from "../workspaces";
  import RiftLogo from "./RiftLogo.svelte";
  import ConversationList from "./ConversationList.svelte";
  import ProjectRail from "./ProjectRail.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { goHome } from "$lib/state/nav";

  // Sidebar nav: Workspace (home) + AI Health. Settings pinned to foot; Chat,
  // legacy Projects, the project switcher, and any disabled workspace excluded.
  const navItems = $derived(
    workspace.order.filter(
      (id) => id !== "settings" && id !== "chat" && id !== "projects" && !WORKSPACES[id].disabled,
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
  style="width:{shell.collapsed ? 0 : shell.width}px"
>
  <aside class="sidebar" class:home={isNavActive("home")}>
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

    <button class="new-chat" type="button" onclick={() => goHome()} use:tooltip={"New chat · Ctrl+N"}>
      <span class="nc-ic"><Plus size={16} strokeWidth={2.4} /></span>
      <span class="nc-lbl">New chat</span>
      <kbd class="nc-kbd">Ctrl N</kbd>
    </button>

    <nav class="side-nav" aria-label="Workspaces">
      {#each navItems as id (id)}
        {@const def = WORKSPACES[id]}
        <button
          class="snav-item"
          class:on={isNavActive(id)}
          type="button"
          onclick={() => goto(id)}
          aria-current={isNavActive(id) ? "page" : undefined}
        >
          <def.icon class="snav-ic snav-ic-{ICON_KEY[id]}" size={16} />
          <span class="lbl">{def.title}</span>
        </button>
      {/each}
    </nav>

    <ProjectRail />

    <div class="side-sec">
      <ConversationList />
    </div>

    <div class="side-foot">
      <button
        class="snav-item"
        class:on={workspace.activeId === "settings"}
        type="button"
        onclick={() => goto("settings")}
        aria-current={workspace.activeId === "settings" ? "page" : undefined}
      >
        <WORKSPACES.settings.icon class="snav-ic snav-ic-settings" size={16} />
        <span class="lbl">{WORKSPACES.settings.title}</span>
      </button>
    </div>
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
  .side-rail { position: relative; flex: none; min-width: 0; overflow: hidden;
    transition: width 0.36s var(--ease-page); }
  .side-rail.resizing, .side-rail.resizing .sidebar { transition: none; }
  .side-rail.collapsed .sidebar { transform: translateX(-18px); opacity: 0; pointer-events: none; }
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

  /* nav */
  .side-nav { display: flex; flex-direction: column; gap: 2px; flex: none; }
  .snav-item { position: relative; display: flex; align-items: center; gap: 10px; height: 36px; padding: 0 11px; border-radius: 9px;
    color: var(--fg-muted); font-size: 13px; font-weight: 500; transition: background var(--dur-fast), color var(--dur-fast); }
  .snav-item .lbl { flex: 1; text-align: left; }
  .snav-item:hover { background: var(--surface-hover); color: var(--fg-2); }
  .snav-item.on { background: color-mix(in oklab, var(--fg) 10%, transparent); color: var(--fg); }
  .snav-item.on :global(.snav-ic) { color: var(--accent); }
  .snav-item.on::before { content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 3px; height: 16px; border-radius: 0 3px 3px 0; background: var(--accent); animation: barPop 0.28s var(--ease-page) both; }
  .snav-item :global(.snav-ic) { transition: transform 0.34s var(--ease-page); transform-origin: 50% 50%; }
  .snav-item:hover :global(.snav-ic-home)     { transform: translateY(-2.5px) scale(1.06); }
  .snav-item:hover :global(.snav-ic-chat)     { transform: rotate(-10deg) scale(1.06); }
  .snav-item:hover :global(.snav-ic-projects) { transform: translateY(-2px) scale(1.06); }
  .snav-item:hover :global(.snav-ic-health)   { transform: scale(1.1); }
  .snav-item:hover :global(.snav-ic-settings) { transform: rotate(140deg); }

  /* primary action — New chat. Promoted to the top of the rail. Reads as primary
     via an accent-tinted surface + accent icon, not a saturated slab, so it stays
     in the rail's soft, low-saturation language. Kbd hint sits flush-right. */
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
  .new-chat :global(svg) { flex: none; color: var(--accent); transition: transform 0.34s var(--ease-page); }
  .new-chat:hover :global(svg) { transform: rotate(90deg); }
  .nc-kbd { flex: none; font-family: var(--font-mono); font-size: 9.5px; font-weight: 600; letter-spacing: 0.02em;
    padding: 2px 5px; border-radius: 5px; color: var(--fg-faint); background: color-mix(in oklab, var(--fg) 6%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 7%, transparent); transition: color var(--dur-fast); }
  .new-chat:hover .nc-kbd { color: var(--fg-subtle); }

  /* conversation-list section wrapper */
  .side-sec { display: flex; flex-direction: column; flex: 1; min-height: 0; margin-top: 10px; }

  .side-foot { display: flex; flex-direction: column; gap: 2px; flex: none; padding-top: 8px;
    margin-top: 4px; border-top: 1px solid color-mix(in oklab, var(--border) 60%, transparent); }

  @keyframes barPop { from { transform: translateY(-50%) scaleY(0.25); } to { transform: translateY(-50%) scaleY(1); } }
</style>
