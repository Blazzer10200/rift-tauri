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
  import { intro } from "$lib/state/intro.svelte";
  import SplashOverlay from "$lib/components/SplashOverlay.svelte";
  import ContextMenuHost from "$lib/components/shell/ContextMenuHost.svelte";
  import { handleGlobalContextMenu } from "$lib/state/contextMenu.svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";

  // Dev-only: Ctrl+Alt+G toggles the stream gallery (/dev/gallery), Ctrl+Alt+U
  // toggles the primitives catalog (/dev/ui) — showrooms for design work. No-op
  // in prod builds.
  function onDevKey(e: KeyboardEvent) {
    if (!import.meta.env.DEV) return;
    if (e.ctrlKey && e.altKey && (e.key === "g" || e.key === "G")) {
      e.preventDefault();
      void goto(page.url.pathname.startsWith("/dev/gallery") ? "/" : "/dev/gallery");
    }
    if (e.ctrlKey && e.altKey && (e.key === "u" || e.key === "U")) {
      e.preventDefault();
      void goto(page.url.pathname.startsWith("/dev/ui") ? "/" : "/dev/ui");
    }
  }

  let { children } = $props();
  // sessionStorage is per-window-instance (cleared on close), so prod cold-
  // launches always replay the splash. Dev HMR / page-refresh within the
  // same window skips — set inline so we never flash-mount the overlay.
  // To force-replay during iteration: `sessionStorage.removeItem("rift.splash.seen")`
  const seenSplash =
    typeof sessionStorage !== "undefined" &&
    !!sessionStorage.getItem("rift.splash.seen");
  let splashDone = $state(seenSplash);
  // Cold boot (splash will play) → arm the shell's island-assembly entrance.
  if (!seenSplash) intro.arm();

  onMount(() => {
    // app.html's pre-JS #first-paint mark is position:fixed, so it painted OVER
    // any page without an opaque positioned surface (/dev/gallery showed a dim
    // "R" across the stream). Children (incl. the opaque splash) are mounted
    // before this runs, so removing it here never flashes.
    document.getElementById("first-paint")?.remove();
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
  <SplashOverlay onComplete={() => { splashDone = true; }} />
{/if}
