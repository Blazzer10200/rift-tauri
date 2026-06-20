<script lang="ts">
  import { PanelLeftClose, Folder, GitBranch, ChevronsUpDown } from "lucide-svelte";
  import { workspace, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { WORKSPACES } from "../workspaces";
  import RiftLogo from "./RiftLogo.svelte";
  import ConversationList from "./ConversationList.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  // Sidebar nav: every workspace except Settings (pinned to the foot). Order
  // follows the user's persisted workspace order, same as the old titlebar nav.
  const navItems = $derived(workspace.order.filter((id) => id !== "settings"));

  // Per-icon hover micro-motion hook (CSS targets .snav-ic-<key>).
  const ICON_KEY: Record<WorkspaceId, string> = {
    home: "home", chat: "chat", "local-llm": "local", settings: "settings",
  };

  const repoName = $derived(
    (assistant.activeRoot ?? "").replace(/[/\\]+$/, "").split(/[/\\]/).pop() || "No folder",
  );

  function goto(id: WorkspaceId) { workspace.setActive(id); }

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
  <aside class="sidebar" class:home={workspace.activeId === "home"}>
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

    <button class="ws-switch" type="button" use:tooltip={assistant.activeRoot ?? "Pick a workspace folder"} aria-label="Workspace folder">
      <span class="ws-switch-ic"><Folder size={15} /></span>
      <span class="ws-switch-text">
        <span class="ws-switch-repo">{repoName}</span>
        {#if assistant.workspaceBranch}
          <span class="ws-switch-branch"><GitBranch size={11} />{assistant.workspaceBranch}</span>
        {/if}
      </span>
      <ChevronsUpDown class="ws-switch-chev" size={14} />
    </button>

    <nav class="side-nav" aria-label="Workspaces">
      {#each navItems as id (id)}
        {@const def = WORKSPACES[id]}
        <button
          class="snav-item"
          class:on={workspace.activeId === id}
          type="button"
          onclick={() => goto(id)}
          aria-current={workspace.activeId === id ? "page" : undefined}
        >
          <def.icon class="snav-ic snav-ic-{ICON_KEY[id]}" size={16} />
          <span class="lbl">{def.title}</span>
          {#if id === "local-llm"}<span class="exp-dot" aria-hidden="true"></span>{/if}
        </button>
      {/each}
    </nav>

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

  /* workspace switcher */
  .ws-switch { display: flex; align-items: center; gap: 9px; width: 100%; height: 46px; padding: 0 9px 0 8px; margin: 2px 0 8px; flex: none;
    border-radius: 11px; border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); text-align: left;
    transition: background var(--dur-fast), border-color var(--dur-fast); }
  .ws-switch:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .ws-switch-ic { width: 30px; height: 30px; flex: none; display: grid; place-items: center; border-radius: 8px;
    background: var(--bg-elev-2); border: 1px solid var(--border); color: var(--fg-muted); }
  .ws-switch-text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; line-height: 1.25; }
  .ws-switch-repo { font-size: 12.5px; font-weight: 600; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ws-switch-branch { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ws-switch-branch :global(svg) { color: var(--fg-faint); flex: none; }
  .ws-switch :global(.ws-switch-chev) { color: var(--fg-faint); flex: none; transition: color var(--dur-fast); }
  .ws-switch:hover :global(.ws-switch-chev) { color: var(--fg-muted); }

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
  .snav-item:hover :global(.snav-ic-local)    { transform: rotate(90deg) scale(1.04); }
  .snav-item:hover :global(.snav-ic-settings) { transform: rotate(140deg); }
  .exp-dot { width: 6px; height: 6px; border-radius: 50%; flex: none; background: var(--warn);
    box-shadow: 0 0 6px color-mix(in oklab, var(--warn) 55%, transparent); }

  /* conversation-list section wrapper */
  .side-sec { display: flex; flex-direction: column; flex: 1; min-height: 0; margin-top: 10px; }

  .side-foot { display: flex; flex-direction: column; gap: 2px; flex: none; padding-top: 8px; }

  @keyframes barPop { from { transform: translateY(-50%) scaleY(0.25); } to { transform: translateY(-50%) scaleY(1); } }
</style>
