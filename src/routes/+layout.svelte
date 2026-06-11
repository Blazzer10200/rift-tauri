<script lang="ts">
  import "@fontsource-variable/inter";
  import "@fontsource-variable/jetbrains-mono";
  import "@fontsource-variable/lexend";
  import "../app.css";
  import { onMount } from "svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { accessibility } from "$lib/state/accessibility.svelte";
  import SplashOverlay from "$lib/components/SplashOverlay.svelte";
  import ContextMenuHost from "$lib/components/shell/ContextMenuHost.svelte";
  import { handleGlobalContextMenu } from "$lib/state/contextMenu.svelte";

  let { children } = $props();
  // sessionStorage is per-window-instance (cleared on close), so prod cold-
  // launches always replay the splash. Dev HMR / page-refresh within the
  // same window skips — set inline so we never flash-mount the overlay.
  // To force-replay during iteration: `sessionStorage.removeItem("rift.splash.seen")`
  let splashDone = $state(
    typeof sessionStorage !== "undefined" &&
      !!sessionStorage.getItem("rift.splash.seen"),
  );

  onMount(() => {
    uiPrefs.init();
    workspace.init();
    accessibility.init();
  });
</script>

<svelte:document oncontextmenu={handleGlobalContextMenu} />

{@render children()}

<ContextMenuHost />

{#if !splashDone}
  <SplashOverlay onComplete={() => (splashDone = true)} />
{/if}
