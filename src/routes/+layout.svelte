<script lang="ts">
  import "@fontsource-variable/inter";
  import "@fontsource-variable/jetbrains-mono";
  import "@fontsource-variable/lexend";
  import "../app.css";
  import { onMount } from "svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { accessibility } from "$lib/state/accessibility.svelte";

  let { children } = $props();

  onMount(() => {
    uiPrefs.init();
    workspace.init();
    accessibility.init();
  });
</script>

{@render children()}
