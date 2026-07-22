<script lang="ts">
  // Sidebar project switcher — the C+ redesign's scope headline. Replaces
  // ProjectRail's Projects-block header with one Linear-style switcher row: the
  // active project's monogram + name + live git branch pill, opening a dropdown
  // of every project + "All projects". It preserves ProjectRail's real
  // affordances (open-focused, open-in-split via right-click, drag-a-project-to-
  // a-pane, new project) so nothing that shipped is lost — they just move behind
  // the dropdown + row context menu instead of a permanent chip list.
  import { ChevronsUpDown, Plus, SplitSquareHorizontal, FolderOpen, ArrowRight, Layers, GitBranch, Check } from "lucide-svelte";
  import { projects, projectRootKey } from "$lib/state/projects.svelte";
  import type { Project } from "$lib/state/assistant/types";
  import { assistant } from "$lib/state/assistant.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { goHome } from "$lib/state/nav";
  import { contextMenu } from "$lib/state/contextMenu.svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";

  import { projectHue } from "$lib/utils/projectHue";
  import Skeleton from "./Skeleton.svelte";
  import { bootLoad } from "$lib/state/bootLoad.svelte";

  const list = $derived(projects.sorted);
  const activeKey = $derived(projectRootKey(assistant.activeRoot));
  const isActive = (p: Project) => !!activeKey && projectRootKey(p.root) === activeKey;
  const monogram = (name: string) => (name.trim().match(/[a-z0-9]/i)?.[0] ?? "·").toUpperCase();
  const activeProject = $derived(list.find(isActive) ?? null);
  const activeHue = $derived(projectHue(activeProject?.name ?? "·"));
  // `assistant.workspaceBranch` is the FOCUSED-tab store field: it transiently
  // clears whenever a non-chat surface (Settings, AI Health) mounts and the
  // focused root shifts off the project, which made the branch pill flip to
  // "Local workspace" mid-navigation. Latch the last non-null branch per active
  // project so the pill stays stable while a project is still selected; only
  // reset when the active project actually changes.
  let latchedBranch = $state<string | null>(null);
  let latchedKey = $state<string | null>(null);
  $effect(() => {
    const key = activeKey;
    if (key !== latchedKey) { latchedKey = key; latchedBranch = null; }
    if (assistant.workspaceBranch) latchedBranch = assistant.workspaceBranch;
  });
  const branch = $derived(assistant.workspaceBranch ?? latchedBranch);

  // ── dropdown ───────────────────────────────────────────────────────────
  let menuOpen = $state(false);
  let menuPos = $state({ x: 0, y: 0, w: 0 });
  let triggerEl = $state<HTMLButtonElement>();

  function toggleMenu() {
    if (menuOpen) { menuOpen = false; return; }
    if (!triggerEl) return;
    const r = triggerEl.getBoundingClientRect();
    menuPos = { x: r.left, y: r.bottom + 6, w: r.width };
    menuOpen = true;
  }
  function closeMenu() { menuOpen = false; }

  $effect(() => {
    if (!menuOpen) return;
    const onDoc = () => closeMenu();
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") closeMenu(); };
    // #81: menuPos is captured once at open — a window resize while open would
    // leave the portaled menu floating at stale coords, so close it instead.
    window.addEventListener("click", onDoc);
    window.addEventListener("keydown", onKey);
    window.addEventListener("resize", onDoc);
    return () => {
      window.removeEventListener("click", onDoc);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", onDoc);
    };
  });

  // ── open actions (lifted verbatim from ProjectRail) ──────────────────────
  async function openFocused(p: Project) {
    closeMenu();
    shell.setAllProjects(false);
    await assistant.setRoot(p.root);
    goHome();
  }
  async function openInSplit(p: Project) {
    closeMenu();
    workspace.setActive("chat");
    await assistant.openProjectInPane(p.root, { splitNew: true });
  }
  function chooseAll() {
    closeMenu();
    shell.setAllProjects(true);
  }
  function newProject() {
    closeMenu();
    projects.requestNewProject();
    workspace.setActive("home");
  }

  // ── drag a project onto a pane (grows the split) ─────────────────────────
  function onItemDragStart(e: DragEvent, p: Project) {
    assistant.draggingProjectRoot = p.root;
    workspace.setActive("chat");
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "copy";
      e.dataTransfer.setData("text/plain", p.root);
    }
  }
  function onItemDragEnd() { assistant.draggingProjectRoot = null; }

  function onItemContext(e: MouseEvent, p: Project) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu.open(e.clientX, e.clientY, [
      { label: "Open project", icon: FolderOpen, action: () => void openFocused(p) },
      {
        label: "Open in split",
        icon: SplitSquareHorizontal,
        disabled: !assistant.canAddPane,
        action: () => void openInSplit(p),
      },
      { kind: "divider" },
      { label: "Go to Workspace", icon: ArrowRight, action: () => workspace.setActive("home") },
    ]);
  }
</script>

{#if bootLoad.showSkeleton}
  <div class="sw-skel">
    <Skeleton w="28px" h="28px" radius="8px" />
    <div class="sw-skel-meta">
      <Skeleton w="62%" h="12px" radius="5px" />
      <Skeleton w="40%" h="9px" radius="4px" delay={120} />
    </div>
  </div>
{:else}
<button
  bind:this={triggerEl}
  class="switcher"
  class:open={menuOpen}
  type="button"
  onclick={(e) => { e.stopPropagation(); toggleMenu(); }}
  use:tooltip={"Switch project"}
  aria-haspopup="menu"
  aria-expanded={menuOpen}
>
  <span class="sw-mono" class:all={shell.allProjects} style="--ph:{activeHue}">
    {#if shell.allProjects}<Layers size={14} />{:else}{monogram(activeProject?.name ?? "·")}{/if}
  </span>
  <span class="sw-meta">
    <span class="sw-name">{shell.allProjects ? "All chats" : (activeProject?.name ?? "No project")}</span>
    <span class="sw-sub">
      {#if !shell.allProjects && branch}
        <span class="branch"><GitBranch size={9} />{branch}</span>
      {:else if shell.allProjects}
        <span class="sw-dim">Every project's chats</span>
      {:else}
        <span class="sw-dim">Local workspace</span>
      {/if}
    </span>
  </span>
  <ChevronsUpDown size={15} class="sw-ch" />
</button>
{/if}

{#if menuOpen}
  <div class="sw-menu" use:portal style="left:{menuPos.x}px; top:{menuPos.y}px; min-width:{menuPos.w}px" role="menu" tabindex="-1">
    <button
      class="sw-item all"
      class:on={shell.allProjects}
      type="button" role="menuitem"
      onclick={(e) => { e.stopPropagation(); chooseAll(); }}
    >
      <span class="sw-item-mk mk-all"><Layers size={13} /></span>
      <span class="sw-item-nm">All chats</span>
      {#if shell.allProjects}<Check size={14} class="sw-item-ck" />{/if}
    </button>

    {#if list.length}<div class="sw-div"></div>{/if}

    {#each list as p (p.id)}
      <button
        class="sw-item"
        class:on={!shell.allProjects && isActive(p)}
        type="button" role="menuitem"
        draggable="true"
        onclick={(e) => { e.stopPropagation(); void openFocused(p); }}
        ondragstart={(e) => onItemDragStart(e, p)}
        ondragend={onItemDragEnd}
        oncontextmenu={(e) => onItemContext(e, p)}
        use:tooltip={`Open ${p.name} · drag to a pane to split · right-click for options`}
      >
        <span class="sw-item-mk" style="--ph:{projectHue(p.name)}">{monogram(p.name)}</span>
        <span class="sw-item-nm">{p.name}</span>
        {#if !shell.allProjects && isActive(p)}<Check size={14} class="sw-item-ck" />{/if}
      </button>
    {/each}

    <div class="sw-div"></div>
    <button class="sw-item add" type="button" role="menuitem" onclick={(e) => { e.stopPropagation(); newProject(); }}>
      <span class="sw-item-mk mk-add"><Plus size={13} strokeWidth={2.4} /></span>
      <span class="sw-item-nm">New project</span>
    </button>
  </div>
{/if}

<style>
  /* switcher trigger — the C+ hero. Inset well (--bg-inset) w/ a monogram tile,
     name + branch pill stacked, a chevron. Same low-saturation language as the
     rest of the rail; accent lives only in the monogram + branch pill. */
  .switcher { display: flex; align-items: center; gap: 10px; height: 46px; margin: 2px 0; padding: 0 8px; flex: none;
    border-radius: 11px; border: 1px solid var(--border); background: var(--bg-inset);
    transition: border-color var(--dur-fast), background var(--dur-fast); }
  .switcher:hover, .switcher.open { border-color: var(--border-strong); background: var(--surface); }
  /* Boot skeleton — same footprint as the switcher trigger so nothing jumps. */
  .sw-skel { display: flex; align-items: center; gap: 10px; height: 46px; margin: 2px 0; padding: 0 8px; flex: none;
    border-radius: 11px; border: 1px solid var(--border); background: var(--bg-inset);
    animation: sw-skel-in 240ms var(--ease-page) both; }
  .sw-skel-meta { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 5px; }
  @keyframes sw-skel-in { from { opacity: 0; } to { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .sw-skel { animation: none; } }
  /* Monogram tiles wear the project's identity hue (--ph), not the accent. */
  .sw-mono { width: 28px; height: 28px; flex: none; display: grid; place-items: center; border-radius: 8px;
    font-size: 12px; font-weight: 700; letter-spacing: -0.01em; color: oklch(0.78 0.14 var(--ph)); background: oklch(0.72 0.14 var(--ph) / 0.13);
    box-shadow: inset 0 0 0 1px oklch(0.75 0.14 var(--ph) / 0.28); }
  .sw-mono.all { color: var(--fg-muted); background: color-mix(in oklab, var(--fg) 9%, transparent); box-shadow: none; }
  .sw-meta { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; text-align: left; }
  .sw-name { font-size: 13px; font-weight: 650; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sw-sub { display: flex; align-items: center; gap: 6px; font-size: 10.5px; color: var(--fg-faint); min-width: 0; }
  .sw-dim { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .switcher :global(.sw-ch) { color: var(--fg-faint); flex: none; transition: color var(--dur-fast); }
  .switcher:hover :global(.sw-ch), .switcher.open :global(.sw-ch) { color: var(--fg-2); }

  /* branch pill — accent-soft, matches the app's existing .branch-pill idiom.
     Branch name only (the app exposes no clean/dirty/synced signal). */
  .branch { display: inline-flex; align-items: center; gap: 3px; flex: none; max-width: 100%;
    height: 15px; padding: 0 6px; border-radius: 999px; background: var(--accent-soft); color: var(--accent);
    font-size: 9.5px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .branch :global(svg) { flex: none; }

  /* dropdown — portaled to <body> to escape the sidebar's backdrop-filter
     containing block (same reason ConversationList's menu portals). Rides the
     app's shared popover look. */
  :global(.sw-menu) { position: fixed; z-index: 52; max-width: 320px; max-height: 60vh; overflow-y: auto; padding: 5px;
    border-radius: 12px; background: color-mix(in oklab, var(--bg-elev-2) 90%, transparent);
    -webkit-backdrop-filter: blur(12px) saturate(1.35); backdrop-filter: blur(12px) saturate(1.35);
    border: 1px solid color-mix(in oklab, var(--fg) 12%, transparent);
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.08), 0 24px 56px -26px oklch(0 0 0 / 0.7), var(--shadow-lg);
    animation: swPopIn var(--dur-fast) var(--ease-page) both; transform-origin: top left; }
  :global(.sw-menu .sw-item) { display: flex; align-items: center; gap: 10px; width: 100%; height: 38px; padding: 0 9px; border-radius: 9px;
    color: var(--fg-muted); font-size: 12.5px; text-align: left; transition: background var(--dur-fast), color var(--dur-fast); }
  :global(.sw-menu .sw-item:hover) { background: var(--surface-hover); color: var(--fg-2); cursor: grab; }
  :global(.sw-menu .sw-item.add:hover) { cursor: pointer; }
  :global(.sw-menu .sw-item.on) { background: color-mix(in oklab, var(--fg) 10%, transparent); color: var(--fg); }
  :global(.sw-menu .sw-item-mk) { width: 22px; height: 22px; flex: none; display: grid; place-items: center; border-radius: 6px;
    font-size: 11px; font-weight: 700; color: oklch(0.78 0.14 var(--ph)); background: oklch(0.72 0.14 var(--ph) / 0.13);
    box-shadow: inset 0 0 0 1px oklch(0.75 0.14 var(--ph) / 0.28); }
  :global(.sw-menu .sw-item.on .sw-item-mk) { background: oklch(0.75 0.14 var(--ph)); color: var(--bg); box-shadow: none; }
  :global(.sw-menu .mk-all), :global(.sw-menu .mk-add) { color: var(--fg-muted); background: color-mix(in oklab, var(--fg) 9%, transparent); box-shadow: none; }
  :global(.sw-menu .sw-item.on .mk-all) { background: var(--accent); color: var(--bg); }
  :global(.sw-menu .sw-item-nm) { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  :global(.sw-menu .sw-item-ck) { color: var(--accent); flex: none; }
  :global(.sw-menu .sw-div) { height: 1px; margin: 4px 6px; background: color-mix(in oklab, var(--border) 60%, transparent); }
  @keyframes -global-swPopIn { from { opacity: 0; transform: scale(0.97) translateY(-4px); } to { opacity: 1; transform: none; } }
  @media (prefers-reduced-motion: reduce) { :global(.sw-menu) { animation: none; } }
</style>
