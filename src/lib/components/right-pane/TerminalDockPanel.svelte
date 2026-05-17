<script lang="ts">
  import type { PanelIcon } from "./index";
  import TerminalPanel from "../terminal/TerminalPanel.svelte";

  // PanelShell passes title + icon for the registry contract; the dock-mode
  // TerminalPanel branch renders its own tab strip + tool buttons, so these
  // are accepted but unused.
  let { }: { title: string; icon: PanelIcon } = $props();
</script>

<div class="wrap">
  <TerminalPanel />
</div>

<style>
  /* xterm has no natural height — without a floor, the body collapses to 0px
     when ps.height is null. 280px matches the v0.2 overlay default. */
  .wrap {
    display: flex; flex-direction: column;
    flex: 1; min-height: 280px;
    height: 100%;
  }
</style>
