<script lang="ts">
  import { onMount } from "svelte";
  import { diagnostics } from "$lib/state/diagnostics.svelte";
  import DiagnosticsConsole from "./DiagnosticsConsole.svelte";
  import PageHero from "../shared/PageHero.svelte";

  onMount(() => {
    void diagnostics.init().catch((e) => console.warn("diagnostics.init failed", e));
  });
</script>

<div class="sb-main">
  <PageHero
    eyebrow="Debug"
    title="Diagnostics"
    desc="Live event stream from every subsystem — filter, search, and export. Paths and usernames are scrubbed at the source."
    maxWidth={1080}
  />
  <div class="dg-body">
    <DiagnosticsConsole page />
  </div>
</div>

<style>
  /* Transparent — keeps the app dot-field continuous across surfaces
     (AssistantPage doctrine); same shell as Settings/AI Health. */
  .sb-main { position: relative; overflow: hidden; display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; background: transparent; color: var(--fg); }
  .dg-body { flex: 1; min-height: 0; width: 100%; max-width: 1080px; margin: 0 auto; padding: 0 40px 30px; box-sizing: border-box; }
</style>
