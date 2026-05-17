<script lang="ts">
  import type { PanelIcon } from "./index";
  import ActivityFeed from "../activity/ActivityFeed.svelte";

  // PanelShell passes title + icon; the body owns its own toolbar so these
  // are accepted for the registry contract but not rendered.
  let { }: { title: string; icon: PanelIcon } = $props();
</script>

<div class="wrap">
  <ActivityFeed />
</div>

<style>
  .wrap {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    height: 100%;
  }
</style>
