<script lang="ts">
  // Sidebar project rail — compact monogram chips for each saved project, the
  // missing "get a project into a (split) pane fast" affordance. Click = open in
  // the focused pane. Drag a chip onto a pane half = open that project there
  // (grows the split). Right-click = "Open in split" one-click. The data layer
  // already supports per-pane projects (effectiveRoot falls through to the
  // tab's own workspaceRoot); this is purely the entry point.
  import { Plus, SplitSquareHorizontal, FolderOpen, ArrowRight, Layers } from "lucide-svelte";
  import { projects, projectRootKey } from "$lib/state/projects.svelte";
  import type { Project } from "$lib/state/assistant/types";
  import { assistant } from "$lib/state/assistant.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { goHome } from "$lib/state/nav";
  import { contextMenu } from "$lib/state/contextMenu.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  const list = $derived(projects.sorted);
  const activeKey = $derived(projectRootKey(assistant.activeRoot));
  const isActive = (p: Project) => !!activeKey && projectRootKey(p.root) === activeKey;
  const monogram = (name: string) => (name.trim().match(/[a-z0-9]/i)?.[0] ?? "·").toUpperCase();

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  // Collapsed view shows only the active project (so a 20-project rail stays 2
  // rows tall). Expanded shows all. When scope is All-projects or nothing is
  // active, collapsed falls back to showing nothing extra — the "All projects"
  // row alone represents the state.
  const activeProject = $derived(list.find(isActive) ?? null);
  const shownProjects = $derived(
    shell.projectsExpanded
      ? list
      : activeProject
        ? [activeProject]
        : [],
  );

  // Open in the focused pane (the classic single-surface open): set global root
  // so the welcome card + non-split surfaces reflect it, then float to chat.
  async function openFocused(p: Project) {
    projects.setActiveId(p.id);
    await assistant.setRoot(p.root);
    goHome();
  }

  // Open in a NEW split pane beside the current one (one click). Switches to the
  // chat surface first so the panes are visible.
  async function openInSplit(p: Project) {
    workspace.setActive("chat");
    projects.setActiveId(p.id);
    await assistant.openProjectInPane(p.root, { splitNew: true });
  }

  function onChipDragStart(e: DragEvent, p: Project) {
    assistant.draggingProjectRoot = p.root;
    // Make the pane drop zones visible by switching to the chat surface.
    workspace.setActive("chat");
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "copy";
      e.dataTransfer.setData("text/plain", p.root);
    }
  }
  function onChipDragEnd() {
    assistant.draggingProjectRoot = null;
  }

  function onChipContext(e: MouseEvent, p: Project) {
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

{#if list.length > 0}
  <div class="prail" aria-label="Projects">
    <div class="prail-h">
      <span class="prail-lbl">Projects</span>
      <button class="prail-add" type="button" use:tooltip={"New project"}
        onclick={() => { projects.requestNewProject(); workspace.setActive("home"); }} aria-label="New project">
        <Plus size={13} strokeWidth={2.4} />
      </button>
    </div>

    <!-- "All projects" row — folds the old This/All toggle into the project list
         itself. Active when the conversation scope is un-scoped. -->
    <button
      class="prow all"
      class:on={shell.allProjects}
      type="button"
      onclick={() => shell.setAllProjects(true)}
      aria-pressed={shell.allProjects}
    >
      <span class="prow-mk mk-all"><Layers size={13} /></span>
      <span class="prow-nm">All projects</span>
    </button>

    {#each shownProjects as p (p.id)}
      <button
        class="prow"
        class:on={!shell.allProjects && isActive(p)}
        type="button"
        draggable="true"
        use:tooltip={`Open ${p.name} · drag to a pane to split · right-click for options`}
        onclick={() => { shell.setAllProjects(false); void openFocused(p); }}
        ondragstart={(e) => onChipDragStart(e, p)}
        ondragend={onChipDragEnd}
        oncontextmenu={(e) => onChipContext(e, p)}
        aria-label={`Open ${p.name}`}
      >
        <span class="prow-mk">{monogram(p.name)}</span>
        <span class="prow-nm">{p.name}</span>
      </button>
    {/each}

    {#if list.length > 1 && (activeProject || shell.projectsExpanded)}
      <button class="showmore" type="button" onclick={() => shell.toggleProjectsExpanded()}>
        {shell.projectsExpanded ? "Show less" : `Show all (${list.length})`}
      </button>
    {/if}
  </div>
{/if}

<style>
  .prail { display: flex; flex-direction: column; gap: 2px; flex: none; margin-top: 4px; }
  .prail-h { display: flex; align-items: center; justify-content: space-between; padding: 0 4px 2px; }
  .prail-lbl { font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); }
  .prail-add { width: 20px; height: 20px; display: grid; place-items: center; border-radius: 6px; flex: none;
    color: var(--fg-faint); transition: background var(--dur-fast), color var(--dur-fast); }
  .prail-add:hover { background: var(--surface-hover); color: var(--fg-2); }

  /* Shared "show more/less" affordance — a quiet text link, indented to align
     with the row labels above it. Same pattern the conversation list uses, so
     projects + history rhyme instead of each having its own control. */
  .showmore { display: block; width: 100%; text-align: left; padding: 6px 11px 4px 11px; margin-top: 1px;
    font-size: 11.5px; font-weight: 500; color: var(--fg-subtle); transition: color var(--dur-fast); }
  .showmore:hover { color: var(--accent); }

  /* Project rows — same visual language as the nav rows above them (.snav-item:
     36px tall, pad 0 11px, gap 10px, radius 9px, --fg-muted). One repeating shape
     instead of the old icon-chip grid, so the whole top of the rail reads as one
     calm list. A small monogram stays as a quiet identifier, not the headline. */
  .prow { position: relative; display: flex; align-items: center; gap: 10px; height: 34px; padding: 0 11px; border-radius: 9px;
    color: var(--fg-muted); font-size: 12.5px; font-weight: 500; cursor: grab;
    transition: background var(--dur-fast), color var(--dur-fast); }
  .prow:hover { background: var(--surface-hover); color: var(--fg-2); }
  .prow:active { cursor: grabbing; }
  .prow.on { background: color-mix(in oklab, var(--fg) 10%, transparent); color: var(--fg); }
  .prow.on::before { content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 3px; height: 16px; border-radius: 0 3px 3px 0; background: var(--accent); animation: barPop 0.28s var(--ease-page) both; }
  .prow-mk { width: 20px; height: 20px; flex: none; display: grid; place-items: center; border-radius: 6px;
    font-size: 10.5px; font-weight: 700; letter-spacing: -0.01em; color: var(--accent); background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 24%, transparent); transition: background var(--dur-fast); }
  .prow.on .prow-mk { background: var(--accent); color: var(--bg); box-shadow: none; }
  .prow-mk.mk-all { color: var(--fg-muted); background: color-mix(in oklab, var(--fg) 9%, transparent); box-shadow: none; }
  .prow.all.on .prow-mk { background: var(--accent); color: var(--bg); }
  .prow.all.on .prow-mk :global(svg) { color: var(--bg); }
  .prow-nm { flex: 1; min-width: 0; text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  @keyframes barPop { from { transform: translateY(-50%) scaleY(0.25); } to { transform: translateY(-50%) scaleY(1); } }
</style>
