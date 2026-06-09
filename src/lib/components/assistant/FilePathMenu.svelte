<script lang="ts">
  // Shared file-actions popover for any file path in the UI (edit diffs, the
  // Activity "Outputs" list, etc.). Reuses the .rift-menu chrome from app.css;
  // only positioning + the action set live here. Mount it conditionally and
  // feed it a viewport-anchored {x, y} (mirrors OpenInPaneMenu's contract).
  import { onMount } from "svelte";
  import { Code2, ExternalLink, FolderOpen, Copy, Type } from "lucide-svelte";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { invoke } from "@tauri-apps/api/core";
  import { portal } from "$lib/actions/portal";
  import { environment } from "$lib/state/environment.svelte";

  let { path, x, y, onClose }: { path: string; x: number; y: number; onClose: () => void } = $props();

  let menuEl = $state<HTMLDivElement | undefined>();
  // svelte-ignore state_referenced_locally
  let pos = $state({ x, y });

  const baseName = $derived(path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? path);

  onMount(() => {
    void environment.ensureLoaded(); // hide "Open in VS Code" if `code` isn't on PATH
    requestAnimationFrame(() => {
      if (!menuEl) return;
      const r = menuEl.getBoundingClientRect();
      let nx = x, ny = y;
      if (nx + r.width + 4 > window.innerWidth) nx = Math.max(4, window.innerWidth - r.width - 4);
      if (ny + r.height + 4 > window.innerHeight) ny = Math.max(4, window.innerHeight - r.height - 4);
      pos = { x: nx, y: ny };
    });
    const onDocClick = (e: MouseEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    setTimeout(() => {
      document.addEventListener("mousedown", onDocClick);
      document.addEventListener("keydown", onKey);
    }, 0);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  });

  async function run(fn: () => Promise<unknown>) {
    try { await fn(); onClose(); } catch (e) { console.warn("[FilePathMenu] action failed", e); }
  }

  const openInVscode = () => run(() => invoke("open_in_vscode", { path }));
  const openDefault = () => run(() => openPath(path));
  const reveal = () => run(() => revealItemInDir(path));
  const copyPath = () => run(async () => navigator.clipboard?.writeText(path));
  const copyName = () => run(async () => navigator.clipboard?.writeText(baseName));
</script>

<div bind:this={menuEl} use:portal class="rift-menu menu" role="menu" style="left: {pos.x}px; top: {pos.y}px;">
  {#if environment.code}
    <button type="button" class="rift-menu-row" role="menuitem" onclick={openInVscode}>
      <Code2 size={14} class="rift-menu-row-ic" />
      <span class="rift-menu-row-t">Open in VS Code</span>
    </button>
  {/if}
  <button type="button" class="rift-menu-row" role="menuitem" onclick={openDefault}>
    <ExternalLink size={14} class="rift-menu-row-ic" />
    <span class="rift-menu-row-t">Open in default app</span>
  </button>

  <div class="rift-menu-divider"></div>

  <button type="button" class="rift-menu-row" role="menuitem" onclick={reveal}>
    <FolderOpen size={14} class="rift-menu-row-ic" />
    <span class="rift-menu-row-t">Reveal in File Explorer</span>
  </button>

  <div class="rift-menu-divider"></div>

  <button type="button" class="rift-menu-row" role="menuitem" onclick={copyPath}>
    <Copy size={14} class="rift-menu-row-ic" />
    <span class="rift-menu-row-t">Copy full path</span>
  </button>
  <button type="button" class="rift-menu-row" role="menuitem" onclick={copyName}>
    <Type size={14} class="rift-menu-row-ic" />
    <span class="rift-menu-row-t">Copy file name</span>
  </button>
</div>

<style>
  /* Chrome inherited from .rift-menu / .rift-menu-row (app.css) — only layout here. */
  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 200px;
    display: flex; flex-direction: column; gap: 1px;
  }
  .menu :global(.rift-menu-row) { align-items: center; }
</style>
