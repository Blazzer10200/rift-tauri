<script lang="ts">
  import type { PanelIcon } from "./index";
  import TasksDock from "../../assistant/TasksDock.svelte";

  // PanelShell passes title + icon for the registry contract; TasksDock
  // branches its own dock-head off under uiPrefs.useV03Shell.
  let { }: { title: string; icon: PanelIcon } = $props();
</script>

<div class="wrap">
  <TasksDock />
</div>

<style>
  .wrap {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    height: 100%;
  }
</style>
