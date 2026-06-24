<script lang="ts">
  // Sidebar workspace switcher. Trigger shows the active folder/branch; clicking
  // opens a popover listing defined projects (one click to switch root), plus
  // "Open folder…" (ad-hoc, the old picker behavior) and "Manage projects"
  // (→ Projects page). Portals to <body> + anchors against the trigger, matching
  // the SettingsMenu / PermMenu popover pattern.
  import { tick } from "svelte";
  import { Folder, GitBranch, ChevronsUpDown, FolderTree, FolderOpen, Check, Settings2 } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { projects, projectRootKey } from "$lib/state/projects.svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { prettyPath } from "./tabsbar/helpers";

  let open = $state(false);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);
  let pos = $state<{ top: number; left: number; width: number }>({ top: 0, left: 0, width: 240 });

  const repoName = $derived(
    (assistant.activeRoot ?? "").replace(/[/\\]+$/, "").split(/[/\\]/).pop() || "No folder",
  );
  const activeKey = $derived(projectRootKey(assistant.activeRoot));
  const folderName = (root: string) => root.replace(/[/\\]+$/, "").split(/[/\\]/).pop() || root;

  function position() {
    if (!triggerEl || !menuEl) return;
    const a = triggerEl.getBoundingClientRect();
    const ph = menuEl.offsetHeight || 320;
    let top = a.bottom + 6;
    if (top + ph > window.innerHeight - 8) top = Math.max(8, a.top - ph - 6);
    pos = { top, left: a.left, width: a.width };
  }

  async function toggle() {
    open = !open;
    if (open) {
      await projects.refresh();
      await tick();
      position();
    }
  }
  function close() { open = false; }

  function onDocMousedown(ev: MouseEvent) {
    if (triggerEl && ev.target instanceof Node && triggerEl.contains(ev.target)) return;
    if (menuEl && ev.target instanceof Node && menuEl.contains(ev.target)) return;
    close();
  }
  $effect(() => {
    if (!open) return;
    window.addEventListener("mousedown", onDocMousedown);
    const onResize = () => position();
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("mousedown", onDocMousedown);
      window.removeEventListener("resize", onResize);
    };
  });

  async function chooseProject(root: string) {
    close();
    await assistant.setRoot(root);
    const p = projects.byRoot(root);
    projects.setActiveId(p?.id ?? null);
  }

  async function openFolder() {
    close();
    const ok = await assistant.pickFolder();
    if (ok) {
      const p = projects.byRoot(assistant.activeRoot);
      projects.setActiveId(p?.id ?? null);
    }
  }

  function manageProjects() {
    close();
    workspace.setActive("projects");
  }
</script>

<button
  class="ws-switch"
  class:open
  type="button"
  bind:this={triggerEl}
  onclick={toggle}
  use:tooltip={assistant.activeRoot ?? "Pick a workspace folder"}
  aria-haspopup="menu"
  aria-expanded={open}
  aria-label="Switch project or folder"
>
  <span class="ws-switch-ic"><Folder size={15} /></span>
  <span class="ws-switch-text">
    <span class="ws-switch-repo">{repoName}</span>
    {#if assistant.workspaceBranch}
      <span class="ws-switch-branch"><GitBranch size={11} />{assistant.workspaceBranch}</span>
    {/if}
  </span>
  <ChevronsUpDown class="ws-switch-chev" size={14} />
</button>

{#if open}
  <div
    class="rift-menu proj-menu"
    role="menu"
    bind:this={menuEl}
    use:portal
    style="top: {pos.top}px; left: {pos.left}px; min-width: {Math.max(pos.width, 240)}px;"
  >
    {#if projects.items.length}
      <div class="pm-head">Projects</div>
      {#each projects.sorted as p (p.id)}
        {@const sel = !!activeKey && projectRootKey(p.root) === activeKey}
        <button class="pm-item" class:sel type="button" role="menuitem" onclick={() => chooseProject(p.root)}>
          <span class="pm-ic"><FolderTree size={15} /></span>
          <span class="pm-tx">
            <span class="pm-name">{p.name}</span>
            <span class="pm-sub mono">{prettyPath(p.root)}</span>
          </span>
          {#if sel}<Check size={14} class="pm-ck" />{/if}
        </button>
      {/each}
      <div class="rift-menu-divider"></div>
    {/if}

    <button class="pm-item simple" type="button" role="menuitem" onclick={openFolder}>
      <span class="pm-ic"><FolderOpen size={15} /></span>
      <span class="pm-name">Open folder…</span>
    </button>
    <button class="pm-item simple" type="button" role="menuitem" onclick={manageProjects}>
      <span class="pm-ic"><Settings2 size={15} /></span>
      <span class="pm-name">Manage projects</span>
    </button>
  </div>
{/if}

<style>
  /* trigger — same chrome as the old .ws-switch button it replaces */
  .ws-switch { display: flex; align-items: center; gap: 9px; width: 100%; height: 46px; padding: 0 9px 0 8px; margin: 2px 0 8px; flex: none;
    border-radius: 11px; border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); text-align: left;
    transition: background var(--dur-fast), border-color var(--dur-fast); }
  .ws-switch:hover, .ws-switch.open { background: var(--surface-hover); border-color: var(--border-strong); }
  .ws-switch-ic { width: 30px; height: 30px; flex: none; display: grid; place-items: center; border-radius: 8px;
    background: var(--bg-elev-2); border: 1px solid var(--border); color: var(--fg-muted); }
  .ws-switch-text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; line-height: 1.25; }
  .ws-switch-repo { font-size: 12.5px; font-weight: 600; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ws-switch-branch { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ws-switch-branch :global(svg) { color: var(--fg-faint); flex: none; }
  .ws-switch :global(.ws-switch-chev) { color: var(--fg-faint); flex: none; transition: color var(--dur-fast), transform var(--dur-fast); }
  .ws-switch:hover :global(.ws-switch-chev), .ws-switch.open :global(.ws-switch-chev) { color: var(--fg-muted); }
  .ws-switch.open :global(.ws-switch-chev) { transform: rotate(180deg); }

  /* popover */
  :global(.rift-menu.proj-menu) {
    position: fixed; z-index: 9998; max-width: 360px; max-height: min(70vh, 460px); overflow-y: auto;
    padding: 6px; border-radius: 14px;
    background: color-mix(in oklab, var(--bg-elev-2) 64%, transparent);
    -webkit-backdrop-filter: blur(26px) saturate(1.6); backdrop-filter: blur(26px) saturate(1.6);
    border: 1px solid color-mix(in oklab, var(--fg) 12%, transparent);
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.08), 0 24px 56px -26px oklch(0 0 0 / 0.7), var(--shadow-lg);
    animation: pm-pop 0.2s var(--ease-page) both;
  }
  @keyframes pm-pop { from { opacity: 0; transform: translateY(-6px) scale(0.97); } to { opacity: 1; transform: none; } }
  .pm-head { font-size: 10px; font-weight: 700; letter-spacing: 0.09em; text-transform: uppercase; color: var(--fg-faint); padding: 7px 9px 5px; }
  .pm-item { display: flex; align-items: center; gap: 10px; width: 100%; padding: 8px 9px; border-radius: 10px; text-align: left;
    color: var(--fg-2); transition: background var(--dur-fast), color var(--dur-fast); }
  .pm-item:hover { background: var(--surface-hover); color: var(--fg); }
  .pm-item.sel { background: var(--accent-soft); }
  .pm-ic { flex: none; width: 28px; height: 28px; display: grid; place-items: center; border-radius: 8px;
    background: var(--surface); border: 1px solid var(--border); color: var(--fg-muted); }
  .pm-item.sel .pm-ic { color: var(--accent); border-color: transparent; background: color-mix(in oklab, var(--accent) 14%, transparent); }
  .pm-tx { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .pm-name { font-size: 13px; font-weight: 560; line-height: 1.2; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pm-sub { font-size: 10.5px; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; direction: rtl; text-align: left; }
  .pm-item :global(.pm-ck) { flex: none; color: var(--accent); }
  .pm-item.simple .pm-name { font-weight: 520; color: var(--fg-muted); }
  .pm-item.simple:hover .pm-name { color: var(--fg); }
  .rift-menu-divider { height: 1px; margin: 5px 4px; background: var(--border); }
</style>
