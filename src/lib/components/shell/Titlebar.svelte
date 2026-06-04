<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Minus, Square, X, Search, Settings as SettingsIcon } from "lucide-svelte";
  import { commandPalette } from "../../state/command-palette.svelte";
  import { workspace, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { WORKSPACES } from "../workspaces";
  import RiftLogo from "./RiftLogo.svelte";

  import { tooltip } from "$lib/actions/tooltip";

  const win = getCurrentWindow();

  // Horizontal workspace nav lives in the titlebar (replaced the left activity
  // column). Settings is pinned to the right as a gear; the rest render inline.
  const navItems = $derived(
    workspace.order.filter((id) => id !== "settings"),
  );
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="left">
    <div class="brand" data-tauri-drag-region>
      <RiftLogo size={18} class="riftmark" />
      <span class="app">Rift</span>
    </div>

    <span class="nav-sep" aria-hidden="true"></span>

    <nav class="topnav" aria-label="Workspaces">
      {#each navItems as id (id)}
        {@const def = WORKSPACES[id]}
        {@const isActive = workspace.activeId === id}
        <button
          class="navitem"
          type="button"
          data-active={isActive}
          onclick={() => workspace.setActive(id)}
          use:tooltip={`${def.title} · Ctrl+${def.kbd}`}
          aria-label={def.title}
          aria-pressed={isActive}
        >
          <def.icon size={15} />
          <span class="navitem-label">{def.title}</span>
        </button>
      {/each}
    </nav>
  </div>

  <div class="center" data-tauri-drag-region>
    <button
      class="cmdk"
      type="button"
      onclick={() => commandPalette.show()}
      use:tooltip={"Search commands, servers & chats — Ctrl+K"}
      aria-label="Open command palette"
    >
      <Search size={13} aria-hidden="true" />
      <span class="cmdk-label">Search or run a command…</span>
      <span class="cmdk-kbd"><kbd>Ctrl</kbd><kbd>K</kbd></span>
    </button>
  </div>

  <div class="right">
    <button
      class="navtoggle settings-btn"
      type="button"
      data-active={workspace.activeId === "settings"}
      onclick={() => workspace.setActive("settings")}
      use:tooltip={`Settings · Ctrl+${WORKSPACES.settings.kbd} · Ctrl+,`}
      aria-label="Settings"
      aria-pressed={workspace.activeId === "settings"}
    >
      <SettingsIcon size={15}/>
    </button>
    <div class="winctl">
      <button class="wb" onclick={() => win.minimize().catch(console.error)} use:tooltip={"Minimize"} type="button" aria-label="Minimize">
        <Minus size={10}/>
      </button>
      <button class="wb" onclick={() => win.toggleMaximize().catch(console.error)} use:tooltip={"Maximize"} type="button" aria-label="Maximize">
        <Square size={9}/>
      </button>
      <button class="wb close" onclick={() => win.close().catch(console.error)} use:tooltip={"Close"} type="button" aria-label="Close">
        <X size={10}/>
      </button>
    </div>
  </div>
</div>

<style>
  .titlebar {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    height: var(--titlebar-h);
    width: 100%;
    min-width: 0;
    background: var(--bg-elev-1, var(--bg));
    border-bottom: 1px solid var(--border-strong);
    box-shadow: 0 1px 0 color-mix(in oklch, var(--bg) 60%, transparent);
    font-size: var(--fs-xs);
    user-select: none;
    padding-left: 12px;
  }
  /* Layout priority:
     - .left / .right are `auto` tracks — size to content.
     - .center is the `1fr` track — its empty margins are the window-drag handle; the
       centered ⌘K search pill inside is interactive (not draggable). */
  .left      { display: flex; align-items: center; gap: 8px; min-width: 0; height: 100%; }
  .center    { min-width: 24px; height: 100%; display: flex; align-items: center; justify-content: center; padding: 0 12px; }
  .right     { display: flex; align-items: center; gap: 6px; height: 100%; }

  /* Centered ⌘K command-palette search — mockup `.l2-cmd`. */
  .cmdk {
    display: inline-flex; align-items: center; gap: 8px;
    width: 100%; max-width: 300px; height: 26px;
    padding: 0 8px 0 10px;
    background: var(--field);
    border: 1px solid var(--field-border);
    border-radius: 8px;
    color: var(--fg-subtle);
    font: inherit; font-size: var(--fs-xs);
    cursor: text;
    transition: border-color 140ms var(--ease-soft), background 140ms var(--ease-soft), box-shadow 140ms var(--ease-soft);
  }
  .cmdk:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
  }
  .cmdk:focus-visible {
    outline: none;
    border-color: color-mix(in oklab, var(--accent) 45%, var(--border-strong));
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .cmdk :global(svg) { color: var(--fg-muted); flex-shrink: 0; }
  .cmdk-label {
    flex: 1; text-align: left;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .cmdk-kbd { margin-left: auto; display: inline-flex; gap: 3px; flex-shrink: 0; }
  .cmdk-kbd kbd {
    font-family: var(--font-mono); font-size: 10px; line-height: 1.4;
    color: var(--fg-muted);
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    padding: 1px 5px;
  }

  .brand {
    display: inline-flex; align-items: center; gap: 8px;
    height: 100%;
    padding-right: 4px;
  }
  .app { font-weight: 600; letter-spacing: -0.01em; color: var(--fg); }

  /* Sidebar collapse/expand toggle — mockup `.l2-navtoggle`. */
  .navtoggle {
    display: grid; place-items: center;
    width: 26px; height: 26px; flex: none;
    border-radius: 7px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg-subtle);
    cursor: pointer;
    transition: background 130ms var(--ease-soft), color 130ms var(--ease-soft), border-color 130ms var(--ease-soft);
  }
  .navtoggle:hover { background: var(--surface-hover); color: var(--fg); border-color: var(--border); }
  .navtoggle:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .navtoggle[data-active="true"] {
    background: var(--accent-soft); color: var(--accent); border-color: transparent;
  }
  .navtoggle:disabled {
    opacity: 0.3; cursor: default; background: transparent; border-color: transparent;
  }
  .settings-btn { margin-right: 2px; }

  /* Horizontal workspace nav — replaced the vertical activity column. */
  /* Hairline between the brand cluster and workspace nav — reads as two zones. */
  .nav-sep {
    width: 1px; height: 18px; flex-shrink: 0;
    background: var(--border-strong);
    margin: 0 4px;
  }
  .topnav { display: flex; align-items: center; gap: 2px; height: 100%; }
  .navitem {
    display: inline-flex; align-items: center; gap: 7px;
    height: 28px; padding: 0 10px;
    border-radius: 8px; border: 1px solid transparent;
    background: transparent; color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs); font-weight: 500;
    cursor: pointer;
    transition: background 130ms var(--ease-soft), color 130ms var(--ease-soft), border-color 130ms var(--ease-soft);
  }
  .navitem :global(svg) { flex-shrink: 0; }
  .navitem:hover { background: var(--surface-hover); color: var(--fg); }
  .navitem[data-active="true"] { background: var(--accent-soft); color: var(--accent); }
  .navitem:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .navitem-label { white-space: nowrap; }

  .winctl { display: flex; height: 100%; margin-left: 4px; }
  .wb {
    width: 38px; height: 100%;
    background: transparent; border: none;
    color: var(--fg-muted); cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    transition: background 100ms var(--ease-soft), color 100ms var(--ease-soft);
  }
  .wb:hover { background: var(--bg-elev-2); color: var(--fg); }
  .wb.close:hover { background: var(--danger); color: oklch(0.99 0 0); }
</style>
