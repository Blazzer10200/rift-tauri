<script lang="ts">
  // Onboarding-only window chrome: brand + window controls, nothing that could
  // navigate away from / over the first-run flow. (The old full titlebar —
  // workspace topnav, ⌘K pill, settings gear — was dead code here since the
  // Topbar/Sidebar redesign took over normal-mode chrome; trimmed 2026-07-01.)
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Minus, Square, Copy, X } from "lucide-svelte";
  import RiftLogo from "./RiftLogo.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  const win = getCurrentWindow();
  let maximized = $state(false);

  onMount(() => {
    win.isMaximized().then((m) => { maximized = m; }).catch(console.error);
    const unlisten = win.onResized(() => {
      win.isMaximized().then((m) => { maximized = m; }).catch(console.error);
    });
    return () => { void unlisten.then((fn) => fn()); };
  });
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <RiftLogo size={18} class="riftmark" />
    <span class="app">Rift</span>
  </div>

  <div class="winctl">
    <button class="wb" onclick={() => win.minimize().catch(console.error)} use:tooltip={"Minimize"} type="button" aria-label="Minimize">
      <Minus size={10}/>
    </button>
    <button class="wb" onclick={() => win.toggleMaximize().catch(console.error)} use:tooltip={maximized ? "Restore" : "Maximize"} type="button" aria-label={maximized ? "Restore" : "Maximize"}>
      {#if maximized}<Copy size={9}/>{:else}<Square size={9}/>{/if}
    </button>
    <button class="wb close" onclick={() => win.close().catch(console.error)} use:tooltip={"Close"} type="button" aria-label="Close">
      <X size={10}/>
    </button>
  </div>
</div>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
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

  .brand {
    display: inline-flex; align-items: center; gap: 8px;
    height: 100%;
    padding-right: 4px;
  }
  .app { font-weight: 600; letter-spacing: -0.01em; color: var(--fg); }

  .winctl { display: flex; height: 100%; margin-left: 4px; }
  .wb {
    width: 38px; height: 100%;
    background: transparent; border: none;
    color: var(--fg-muted); cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    transition: background var(--dur-fast) var(--ease-soft), color var(--dur-fast) var(--ease-soft);
  }
  .wb:hover { background: var(--bg-elev-2); color: var(--fg); }
  .wb.close:hover { background: var(--danger); color: oklch(0.99 0 0); }
</style>
