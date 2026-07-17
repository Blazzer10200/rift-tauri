<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { X, PanelLeftOpen } from "lucide-svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  const win = getCurrentWindow();

  // Reflect real window state on the maximize/restore control (icon + label).
  let maximized = $state(false);
  onMount(() => {
    win.isMaximized().then((m) => { maximized = m; }).catch(console.error);
    const unlisten = win.onResized(() => {
      win.isMaximized().then((m) => { maximized = m; }).catch(console.error);
    });
    return () => { void unlisten.then((fn) => fn()); };
  });

</script>

<div class="topbar" class:rail-hidden={shell.collapsed} data-tauri-drag-region>
  {#if shell.collapsed}
    <!-- Collapsed-rail cluster: hovering the panel glyph peeks the island over
         the content; clicking pins it open. -->
    <div class="tb-left">
      <button
        class="tb-ic"
        type="button"
        onmouseenter={() => shell.beginPeek()}
        onmouseleave={() => shell.schedulePeekClose()}
        onclick={() => shell.toggleCollapsed()}
        use:tooltip={"Open sidebar — hover to peek"}
        aria-label="Open sidebar"
      >
        <PanelLeftOpen size={15} />
      </button>
    </div>
  {/if}

  <div class="topbar-r">
    <!-- Calm chrome: window controls only. Other app actions live in the
         command palette + shortcuts; notifications moved to the sidebar
         footer bell. -->
    <div class="winctl">
      <button class="wc" type="button" onclick={() => win.minimize().catch(console.error)} use:tooltip={"Minimize"} aria-label="Minimize"><span class="wc-min"></span></button>
      <button class="wc" type="button" onclick={() => win.toggleMaximize().catch(console.error)} use:tooltip={maximized ? "Restore" : "Maximize"} aria-label={maximized ? "Restore" : "Maximize"}><span class={maximized ? "wc-restore" : "wc-max"}></span></button>
      <button class="wc wc-x" type="button" onclick={() => win.close().catch(console.error)} use:tooltip={"Close"} aria-label="Close"><X size={12} /></button>
    </div>
  </div>
</div>

<style>
  .topbar { display: flex; align-items: center; gap: 10px; height: 40px; flex: none; padding: 0 6px 0 20px; }
  .topbar.rail-hidden { padding-left: 8px; }
  .tb-left { display: flex; align-items: center; gap: 2px; flex: none;
    animation: enter var(--dur-base) var(--ease-page); }
  .tb-left button { -webkit-app-region: no-drag; }
  @media (prefers-reduced-motion: reduce) { .tb-left { animation: none; } }
  .topbar-r { margin-left: auto; display: flex; align-items: center; gap: 2px; flex: none; }
  .topbar-r .winctl { margin-left: 6px; }
  .tb-ic { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 8px;
    color: var(--fg-subtle); transition: background var(--dur-fast), color var(--dur-fast); }
  .tb-ic:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .tb-ic:disabled { opacity: 0.4; }
  .winctl { display: flex; gap: 2px; }
  .wc { width: 36px; height: 30px; display: grid; place-items: center; color: var(--fg-muted); border-radius: 7px; transition: background var(--dur-fast); }
  .wc:hover { background: var(--surface-hover); }
  .wc-x:hover { background: var(--danger); color: var(--danger-fg); }
  .wc-min { width: 10px; height: 1.5px; background: currentColor; }
  .wc-max { width: 9px; height: 9px; border: 1.5px solid currentColor; border-radius: 2px; }
  /* Restore glyph: front square + a second square peeking out top-right. */
  .wc-restore { position: relative; width: 9px; height: 9px; }
  .wc-restore::before { content: ""; position: absolute; left: 0; bottom: 0; width: 7px; height: 7px; border: 1.5px solid currentColor; border-radius: 1.5px; background: var(--bg); }
  .wc-restore::after { content: ""; position: absolute; right: 0; top: 0; width: 7px; height: 7px; border: 1.5px solid currentColor; border-radius: 1.5px; }
</style>
