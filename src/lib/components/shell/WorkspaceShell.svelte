<script lang="ts">
  import { WORKSPACES } from "../workspaces";
  import { workspace, WORKSPACE_IDS, type WorkspaceId } from "$lib/state/workspace.svelte";
</script>

<div class="ws-shell">
  {#each WORKSPACE_IDS as id (id)}
    {#if workspace.everOpened.has(id as WorkspaceId)}
      {@const def = WORKSPACES[id]}
      {@const Comp = def.component}
      <div class="ws-page" data-workspace={id} hidden={workspace.activeId !== id}>
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
  .ws-shell {
    flex: 1;
    min-height: 0; min-width: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }
  .ws-page {
    flex: 1;
    min-height: 0; min-width: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .ws-page[hidden] { display: none; }
</style>
