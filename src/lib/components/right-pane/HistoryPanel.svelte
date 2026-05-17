<script lang="ts">
  import type { PanelIcon } from "./index";
  import HistoryDrawer from "../assistant/HistoryDrawer.svelte";

  // PanelShell passes title + icon for the registry contract; HistoryDrawer
  // sheds its overlay + close button under uiPrefs.useV03Shell.
  let { }: { title: string; icon: PanelIcon } = $props();
</script>

<div class="wrap">
  <HistoryDrawer />
</div>

<style>
  .wrap {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    height: 100%;
  }
</style>
