<script lang="ts">
  import Settings from "./Settings.svelte";
  import { dialogs } from "../../state/dialogs.svelte";
</script>

<section class="page">
  <div class="page-body">
    <Settings
      onAddServer={() => dialogs.onAddServer()}
      onEditServer={(s) => dialogs.onEditServer(s)}
      onDeleteServer={(s) => dialogs.onDeleteServer(s)}
      onLaunchKeygen={() => dialogs.onLaunchKeygen()}
    />
  </div>
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    background: var(--bg);
    color: var(--fg);
  }
  .page-body {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
</style>
