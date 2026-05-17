<script lang="ts">
  import type { PanelIcon } from "./index";
  import TwoPane from "../../browser/TwoPane.svelte";

  // PanelShell passes title + icon for the registry contract; TwoPane owns
  // its own toolbar / tabstrip and branches on uiPrefs.useV03Shell.
  let { }: { title: string; icon: PanelIcon } = $props();
</script>

<div class="wrap">
  <TwoPane />
</div>

<style>
  .wrap {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    height: 100%;
  }
</style>
