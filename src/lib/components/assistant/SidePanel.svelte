<script lang="ts">
  // Right-side panel shell. The Session/Activity tab split was merged into one
  // scrolling surface (ActivityPanel) — live work, plan, outputs, sources, and
  // session review now stack top-to-bottom so nothing is hidden behind a tab.
  import ActivityPanel from "./ActivityPanel.svelte";

  let { tabId = null }: { tabId?: string | null } = $props();
</script>

<div class="side-panel">
  <ActivityPanel {tabId} />
</div>

<style>
  .side-panel { width: 100%; flex: 1; display: flex; flex-direction: column; min-height: 0; }
</style>
