<script lang="ts">
  import { onMount } from "svelte";
  import { assistant } from "../../state/assistant.svelte";
  import AssistantPane from "./AssistantPane.svelte";
  import TasksDock from "./TasksDock.svelte";

  onMount(() => {
    void assistant.init().then(() => {
      if (assistant.openTabs.length === 0) void assistant.newTab();
    });
  });
</script>

<div class="assistant">
  <div class="layout">
    {#if assistant.splitActive}
      <div class="split">
        {#each assistant.panes as p, i (i)}
          <AssistantPane
            tabId={p.tabId}
            focused={assistant.focusedPaneIdx === i}
            paneIdx={i}
          />
          {#if i < assistant.panes.length - 1}
            <div class="divider" aria-hidden="true"></div>
          {/if}
        {/each}
      </div>
    {:else}
      <AssistantPane
        tabId={assistant.currentConvoId}
        focused={true}
        paneIdx={0}
      />
    {/if}

    <div class="dock-slot" class:open={assistant.ui.dockOpen && assistant.tasks.length > 0}>
      <TasksDock />
    </div>
  </div>
</div>

<style>
  .assistant {
    flex: 1;
    display: flex; flex-direction: column;
    min-height: 0;
    background: var(--bg);
    color: var(--fg);
  }
  .layout {
    flex: 1; min-height: 0;
    display: flex;
    overflow: hidden;
    position: relative;
  }
  .split {
    flex: 1; min-width: 0; min-height: 0;
    display: flex;
    flex-direction: row;
    overflow: hidden;
  }
  .divider {
    width: 1px;
    flex-shrink: 0;
    background: var(--border);
    align-self: stretch;
  }

  .dock-slot {
    width: 0;
    overflow: hidden;
    transition: width 220ms cubic-bezier(0.22, 1, 0.36, 1), opacity 180ms ease-out;
    display: flex;
    opacity: 0;
  }
  .dock-slot.open { width: 280px; opacity: 1; }
  .dock-slot :global(.dock) { flex: 1; min-width: 280px; }
</style>
