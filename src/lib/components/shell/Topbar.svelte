<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";
  import { X, SplitSquareHorizontal, AppWindow } from "lucide-svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { WORKSPACES } from "../workspaces";
  import { tooltip } from "$lib/actions/tooltip";
  import NotificationCenter from "./NotificationCenter.svelte";

  const win = getCurrentWindow();
  const isChat = $derived(workspace.activeId === "chat");

  // Chat surface titles consistently as "Chat" (empty or not) — a real
  // conversation shows its own title once it has one. (Previously an empty chat
  // read "Home", which collided with the Workspace page now owning the home
  // destination — two surfaces, one word. The "home is a verb" NAV behavior
  // lives in goHome(); this is only the visible label.)
  const title = $derived(
    workspace.activeId === "chat"
      ? assistant.activeTab?.convoTitle || "Chat"
      : WORKSPACES[workspace.activeId].title,
  );
</script>

<div class="topbar" data-tauri-drag-region>
  <!-- Expand affordance lives on the collapsed mini-rail's brand button now —
       no duplicate open-sidebar control up here. -->
  <span class="topbar-title" data-tauri-drag-region>{title}</span>

  <div class="topbar-r">
    <!-- Window utilities as direct one-click icons (the old ⋮ dropdown buried
         notifications behind two clicks and duplicated the sidebar's search). -->
    {#if isChat}
      <button class="tb-ic" type="button" disabled={!assistant.canAddPane}
        onclick={() => assistant.addPane()}
        use:tooltip={assistant.canAddPane ? "Split editor · Ctrl+\\" : "Maximum panes open"}
        aria-label="Split editor">
        <SplitSquareHorizontal size={15} />
      </button>
    {/if}
    <button class="tb-ic" type="button"
      onclick={() => invoke("open_new_window").catch(console.error)}
      use:tooltip={"New window"} aria-label="New window">
      <AppWindow size={15} />
    </button>
    <NotificationCenter />

    <div class="winctl">
      <button class="wc" type="button" onclick={() => win.minimize().catch(console.error)} use:tooltip={"Minimize"} aria-label="Minimize"><span class="wc-min"></span></button>
      <button class="wc" type="button" onclick={() => win.toggleMaximize().catch(console.error)} use:tooltip={"Maximize"} aria-label="Maximize"><span class="wc-max"></span></button>
      <button class="wc wc-x" type="button" onclick={() => win.close().catch(console.error)} use:tooltip={"Close"} aria-label="Close"><X size={12} /></button>
    </div>
  </div>
</div>

<style>
  .topbar { display: flex; align-items: center; gap: 10px; height: 40px; flex: none; padding: 0 6px 0 20px; }
  .topbar-title { font-size: 13px; font-weight: 600; color: var(--fg-2); letter-spacing: -0.01em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .topbar-r { margin-left: auto; display: flex; align-items: center; gap: 2px; flex: none; }
  .topbar-r .winctl { margin-left: 6px; }
  .tb-ic { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 8px;
    color: var(--fg-subtle); transition: background var(--dur-fast), color var(--dur-fast); }
  .tb-ic:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .tb-ic:disabled { opacity: 0.4; }
  .winctl { display: flex; gap: 2px; }
  .wc { width: 36px; height: 30px; display: grid; place-items: center; color: var(--fg-muted); border-radius: 7px; transition: background var(--dur-fast); }
  .wc:hover { background: var(--surface-hover); }
  .wc-x:hover { background: var(--danger); color: white; }
  .wc-min { width: 10px; height: 1.5px; background: currentColor; }
  .wc-max { width: 9px; height: 9px; border: 1.5px solid currentColor; border-radius: 2px; }
</style>
