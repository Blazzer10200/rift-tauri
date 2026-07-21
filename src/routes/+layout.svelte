<script lang="ts">
  import "@fontsource-variable/inter";
  import "@fontsource-variable/jetbrains-mono";
  import "@fontsource-variable/lexend";
  import "../app.css";
  import { onMount } from "svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { projects } from "$lib/state/projects.svelte";
  import { accessibility } from "$lib/state/accessibility.svelte";
  import SplashOverlay from "$lib/components/SplashOverlay.svelte";
  import ContextMenuHost from "$lib/components/shell/ContextMenuHost.svelte";
  import { handleGlobalContextMenu } from "$lib/state/contextMenu.svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";

  // Dev-only: Ctrl+Alt+G toggles the stream gallery (/dev/gallery) — a
  // showroom of every renderable block for design work. No-op in prod builds.
  function onDevKey(e: KeyboardEvent) {
    if (!import.meta.env.DEV) return;
    if (e.ctrlKey && e.altKey && (e.key === "g" || e.key === "G")) {
      e.preventDefault();
      void goto(page.url.pathname.startsWith("/dev/gallery") ? "/" : "/dev/gallery");
    }
  }

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
    void projects.refresh();
    accessibility.init();
  });
</script>

<svelte:document oncontextmenu={handleGlobalContextMenu} />
<svelte:window onkeydown={onDevKey} />

{@render children()}

<ContextMenuHost />

{#if !splashDone}
  <SplashOverlay onComplete={() => (splashDone = true)} />
{/if}
