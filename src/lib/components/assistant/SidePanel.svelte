<script lang="ts">
  // Right-side panel shell — two modes under one surface:
  //   Session  → TasksDock (tasks / outputs / sources the convo produced)
  //   Activity → ActivityPanel (live shells/agents + perf dashboard)
  // Active tab is app-level (assistant.ui.panelTab) so it persists across
  // pane remounts. The dock open/close flag stays assistant.ui.dockOpen.
  import { ListChecks, Activity } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import TasksDock from "./TasksDock.svelte";
  import ActivityPanel from "./ActivityPanel.svelte";

  let { tabId = null }: { tabId?: string | null } = $props();

  const tab = $derived(tabId == null ? assistant.activeTab : assistant.tabFor(tabId));
  const active = $derived(assistant.ui.panelTab);
  const live = $derived(tab?.streaming ?? false);

  function pick(t: "session" | "activity") { assistant.ui.panelTab = t; }
</script>

<div class="side-panel">
  <div class="ptabs" role="tablist" aria-label="Side panel">
    <button
      class="ptab" class:on={active === "session"}
      role="tab" aria-selected={active === "session"}
      onclick={() => pick("session")}
    >
      <ListChecks size={13} /> Session
    </button>
    <button
      class="ptab" class:on={active === "activity"}
      role="tab" aria-selected={active === "activity"}
      onclick={() => pick("activity")}
    >
      <Activity size={13} /> Activity
      {#if live}<span class="livedot" aria-label="live"></span>{/if}
    </button>
  </div>

  <div class="pbody">
    {#if active === "session"}
      <TasksDock {tabId} />
    {:else}
      <ActivityPanel {tabId} />
    {/if}
  </div>
</div>

<style>
  .side-panel { width: 100%; flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .ptabs {
    display: flex; gap: 2px;
    padding: 6px 8px 0;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .ptab {
    display: inline-flex; align-items: center; gap: 7px;
    padding: 7px 11px 9px;
    margin-bottom: -1px;
    background: transparent; border: 0; border-bottom: 2px solid transparent;
    color: var(--fg-subtle); font: inherit; font-size: var(--fs-sm); font-weight: 500;
    cursor: pointer;
    transition: color 120ms ease-out, border-color 120ms ease-out;
  }
  .ptab :global(svg) { color: currentColor; opacity: 0.85; }
  .ptab:hover { color: var(--fg-2); }
  .ptab.on { color: var(--fg); border-bottom-color: var(--accent); }
  .ptab.on :global(svg) { color: var(--accent); opacity: 1; }
  .livedot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent); animation: sp-pulse 1.4s ease-in-out infinite;
  }
  @keyframes sp-pulse { 0%, 100% { opacity: 0.4; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .livedot { animation: none; } }

  .pbody { flex: 1; min-height: 0; display: flex; flex-direction: column; }
</style>
