<script lang="ts">
  import { Globe } from "lucide-svelte";
  import type { StreamTool } from "./streamModel";
  let { tool }: { tool: StreamTool } = $props();
  const pending = $derived(tool.status === "pending");
  const sources = $derived(tool.sources ?? []);
  // Stable per-domain hue so each source dot reads as a distinct site, not a row
  // of identical grey squares — a lightweight favicon stand-in (no network fetch,
  // CSP-safe). Same domain always lands the same hue.
  function favHue(domain: string): string {
    let h = 0;
    for (let i = 0; i < domain.length; i++) h = (h * 31 + domain.charCodeAt(i)) % 360;
    return `oklch(0.72 0.13 ${h})`;
  }
</script>

<div class="sweb">
  <span class="sweb-ic"><Globe size={12} strokeWidth={2} /></span>
  <div class="sweb-main">
    <span class="sweb-q {pending ? 'sweb-pending' : ''}">
      {tool.kind === "fetch" ? "Fetched " : "Searched "}<b>{tool.query ?? tool.cap}</b>
    </span>
    {#if sources.length}
      <div class="sweb-src">
        {#each sources.slice(0, 5) as s (s)}
          <span class="sweb-chip"><span class="sweb-fav" style="--fav-h:{favHue(s)}"></span>{s}</span>
        {/each}
        {#if sources.length > 5}<span class="sweb-more">+{sources.length - 5} more</span>{/if}
      </div>
    {/if}
  </div>
</div>
