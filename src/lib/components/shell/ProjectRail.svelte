<script lang="ts">
  // Sidebar project rail — compact monogram chips for each saved project, the
  // missing "get a project into a (split) pane fast" affordance. Click = open in
  // the focused pane. Drag a chip onto a pane half = open that project there
  // (grows the split). Right-click = "Open in split" one-click. The data layer
  // already supports per-pane projects (effectiveRoot falls through to the
  // tab's own workspaceRoot); this is purely the entry point.
  import { Plus, SplitSquareHorizontal, FolderOpen, ArrowRight } from "lucide-svelte";
  import { projects, projectRootKey } from "$lib/state/projects.svelte";
  import type { Project } from "$lib/state/assistant/types";
  import { assistant } from "$lib/state/assistant.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { goHome } from "$lib/state/nav";
  import { contextMenu } from "$lib/state/contextMenu.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { prettyPath } from "./tabsbar/helpers";

  const list = $derived(projects.sorted);
  const activeKey = $derived(projectRootKey(assistant.activeRoot));
  const isActive = (p: Project) => !!activeKey && projectRootKey(p.root) === activeKey;
  const monogram = (name: string) => (name.trim().match(/[a-z0-9]/i)?.[0] ?? "·").toUpperCase();

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
    <div class="prail-chips">
      {#each list as p (p.id)}
        <button
          class="pchip"
          class:on={isActive(p)}
          type="button"
          draggable="true"
          use:tooltip={`${p.name} · drag to a pane to split · right-click for options`}
          onclick={() => void openFocused(p)}
          ondragstart={(e) => onChipDragStart(e, p)}
          ondragend={onChipDragEnd}
          oncontextmenu={(e) => onChipContext(e, p)}
          aria-label={`Open ${p.name}`}
        >
          {monogram(p.name)}
          {#if isActive(p)}<span class="pchip-dot" aria-hidden="true"></span>{/if}
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .prail { display: flex; flex-direction: column; gap: 6px; flex: none; margin-top: 8px; }
  .prail-h { display: flex; align-items: center; justify-content: space-between; padding: 0 4px; }
  .prail-lbl { font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); }
  .prail-add { width: 20px; height: 20px; display: grid; place-items: center; border-radius: 6px; flex: none;
    color: var(--fg-faint); transition: background var(--dur-fast), color var(--dur-fast); }
  .prail-add:hover { background: var(--surface-hover); color: var(--fg-2); }

  .prail-chips { display: flex; flex-wrap: wrap; gap: 6px; padding: 0 2px; }
  .pchip { position: relative; width: 32px; height: 32px; flex: none; display: grid; place-items: center; border-radius: 9px;
    font-size: 13px; font-weight: 700; letter-spacing: -0.01em; cursor: grab;
    color: var(--accent); background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 26%, transparent);
    transition: transform var(--dur-fast), box-shadow var(--dur-fast), background var(--dur-fast); }
  .pchip:hover { transform: translateY(-2px); background: color-mix(in oklab, var(--accent) 18%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 44%, transparent), 0 6px 16px -10px color-mix(in oklab, var(--accent) 60%, transparent); }
  .pchip:active { cursor: grabbing; transform: translateY(0); }
  .pchip.on { box-shadow: inset 0 0 0 1.5px var(--accent); }
  .pchip-dot { position: absolute; top: -2px; right: -2px; width: 8px; height: 8px; border-radius: 50%;
    background: var(--accent); box-shadow: 0 0 0 2px var(--bg); }
</style>
