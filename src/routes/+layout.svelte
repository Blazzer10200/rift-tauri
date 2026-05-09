<script lang="ts">
  import "@fontsource-variable/inter";
  import "@fontsource-variable/jetbrains-mono";
  import "../app.css";
  import { onMount } from "svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";

  let { children } = $props();

  onMount(() => {
    uiPrefs.init();
  });
</script>

{@render children()}
