<script lang="ts">
  import type { PanelIcon } from "./index";
  import TwoPane from "../../browser/TwoPane.svelte";

  // PanelShell legacy contract — title + icon accepted but unused since
  // TwoPane owns its own toolbar / tabstrip. v0.4.1 dropped the dock summary
  // card; this is now a thin wrapper around the full Files surface.
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
