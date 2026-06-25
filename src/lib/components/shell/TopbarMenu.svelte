<script lang="ts">
  // The four window-utility actions (split editor / search / notifications /
  // new window) folded into ONE dropdown so the topbar-right reads as a single
  // affordance next to the window controls instead of a row of loose icons.
  // Reuses the shared .rift-menu chrome (app.css) so it matches every other
  // Rift menu. Notifications still owns its own panel (NotificationCenter) — this
  // row just opens it; the trigger mirrors the unread badge so the count stays
  // visible without opening the menu.
  import { invoke } from "@tauri-apps/api/core";
  import { MoreVertical, SplitSquareHorizontal, Search, Bell, AppWindow } from "lucide-svelte";
  import { fly, fade } from "svelte/transition";
  import { assistant } from "$lib/state/assistant.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { commandPalette } from "$lib/state/command-palette.svelte";
  import { toast } from "$lib/state/toast.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let rootEl = $state<HTMLDivElement | null>(null);
  let open = $state(false);

  const isChat = $derived(workspace.activeId === "chat");

  function close() { open = false; }
  function toggle() { open = !open; }

  function splitEditor() {
    if (!assistant.canAddPane) return;
    assistant.addPane();
    close();
  }
  function search() { commandPalette.show(); close(); }
  function notifications() { toast.openCenter(); close(); }
  function newWindow() { invoke("open_new_window").catch(console.error); close(); }

  function onDocMousedown(ev: MouseEvent) {
    if (!open) return;
    if (rootEl && ev.target instanceof Node && rootEl.contains(ev.target)) return;
    close();
  }
  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape" && open) close();
  }
  $effect(() => {
    window.addEventListener("mousedown", onDocMousedown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDocMousedown);
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="tm" bind:this={rootEl}>
  <button
    class="tm-trigger"
    class:active={open}
    type="button"
    onclick={toggle}
    use:tooltip={"Menu"}
    aria-label="More actions"
    aria-haspopup="menu"
    aria-expanded={open}
  >
    <MoreVertical size={16} />
    {#if toast.unreadCount > 0}
      <span class="tm-badge" transition:fade={{ duration: reducedMotion ? 0 : 120 }}>
        {toast.unreadCount > 9 ? "9+" : toast.unreadCount}
      </span>
    {/if}
  </button>

  {#if open}
    <div
      class="rift-menu tm-panel"
      role="menu"
      transition:fly={{ y: reducedMotion ? 0 : -6, duration: reducedMotion ? 0 : 150 }}
    >
      {#if isChat}
        <button
          type="button"
          class="rift-menu-row"
          role="menuitem"
          disabled={!assistant.canAddPane}
          onclick={splitEditor}
        >
          <SplitSquareHorizontal size={15} class="rift-menu-row-ic" />
          <span class="rift-menu-row-body">
            <span class="rift-menu-row-t">Split editor</span>
            <span class="rift-menu-row-d">{assistant.canAddPane ? "Open a second pane" : "Maximum panes open"}</span>
          </span>
          <kbd class="rift-menu-kbd">Ctrl+\</kbd>
        </button>
        <div class="rift-menu-divider"></div>
      {/if}

      <button type="button" class="rift-menu-row" role="menuitem" onclick={search}>
        <Search size={15} class="rift-menu-row-ic" />
        <span class="rift-menu-row-body">
          <span class="rift-menu-row-t">Search</span>
          <span class="rift-menu-row-d">Commands &amp; chats</span>
        </span>
        <kbd class="rift-menu-kbd">Ctrl+K</kbd>
      </button>

      <button type="button" class="rift-menu-row" role="menuitem" onclick={notifications}>
        <Bell size={15} class="rift-menu-row-ic" />
        <span class="rift-menu-row-body">
          <span class="rift-menu-row-t">Notifications</span>
          <span class="rift-menu-row-d">{toast.unreadCount > 0 ? `${toast.unreadCount} unread` : "History & alerts"}</span>
        </span>
        {#if toast.unreadCount > 0}
          <span class="tm-row-count">{toast.unreadCount > 9 ? "9+" : toast.unreadCount}</span>
        {/if}
      </button>

      <div class="rift-menu-divider"></div>

      <button type="button" class="rift-menu-row" role="menuitem" onclick={newWindow}>
        <AppWindow size={15} class="rift-menu-row-ic" />
        <span class="rift-menu-row-body">
          <span class="rift-menu-row-t">New window</span>
          <span class="rift-menu-row-d">Open a separate Rift window</span>
        </span>
      </button>
    </div>
  {/if}
</div>

<style>
  .tm { position: relative; display: flex; }

  /* Topbar icon-button base — duplicated from Topbar (scoped styles don't cross
     into this child) so the trigger matches the window controls' rhythm. */
  .tm-trigger {
    position: relative;
    width: 30px; height: 30px;
    display: grid; place-items: center;
    border-radius: 8px;
    color: var(--fg-subtle);
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .tm-trigger:hover { background: var(--surface-hover); color: var(--fg-2); }
  .tm-trigger.active { background: var(--surface-hover); color: var(--fg-2); }

  .tm-badge {
    position: absolute;
    top: 2px; right: 2px;
    min-width: 14px; height: 14px;
    padding: 0 3px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--bg);
    font-size: 9px;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
    box-shadow: 0 0 0 2px var(--bg);
    pointer-events: none;
  }

  .tm-panel {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    min-width: 248px;
    display: flex; flex-direction: column; gap: 1px;
    z-index: 2100;
  }
  .tm-panel :global(.rift-menu-row) { align-items: center; }
  .tm-panel :global(.rift-menu-row:disabled) { opacity: 0.4; pointer-events: none; }

  /* Trailing unread pill on the Notifications row — accent chip, mirrors the bell badge. */
  .tm-row-count {
    flex: none;
    min-width: 16px; height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--bg);
    font-size: 9.5px; font-weight: 700;
    line-height: 16px; text-align: center;
  }

  @media (prefers-reduced-motion: reduce) {
    .tm-trigger, .tm-badge { transition: none; }
  }
</style>
