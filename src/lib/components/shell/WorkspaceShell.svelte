<script lang="ts">
  import { WORKSPACES } from "../workspaces";
  import { workspace, WORKSPACE_IDS, type WorkspaceId } from "$lib/state/workspace.svelte";
</script>

<div class="ws-shell">
  {#each WORKSPACE_IDS as id (id)}
    {#if workspace.everOpened.has(id as WorkspaceId)}
      {@const def = WORKSPACES[id]}
      {@const Comp = def.component}
      {@const active = workspace.activeId === id}
      <div
        class="ws-page"
        data-workspace={id}
        data-active={active}
        aria-hidden={!active}
        inert={!active}
      >
        {#if def.disabled}
          <Comp title={def.title} icon={def.icon}/>
        {:else}
          <Comp />
        {/if}
      </div>
    {/if}
  {/each}
</div>

<style>
  /* Layered cross-fade: every once-opened workspace stays mounted (so state
     like scroll position, focus, in-flight requests survive), but only the
     active one is visible + interactive. inert keeps tab order and pointer
     events scoped to the active pane. */
  .ws-shell {
    flex: 1;
    min-height: 0; min-width: 0;
    position: relative;
    overflow: hidden;
    background: var(--bg);
  }
  .ws-page {
    position: absolute;
    inset: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    opacity: 0;
    pointer-events: none;
    transform: translateY(3px);
    transition: opacity 160ms ease, transform 200ms cubic-bezier(.2,.7,.2,1);
    will-change: opacity, transform;
  }
  .ws-page[data-active="true"] {
    opacity: 1;
    pointer-events: auto;
    transform: none;
    z-index: 1;
  }
  @media (prefers-reduced-motion: reduce) {
    .ws-page { transition: none; transform: none; }
  }
</style>
