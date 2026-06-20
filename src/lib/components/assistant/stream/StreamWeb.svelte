<script lang="ts">
  import { Globe } from "lucide-svelte";
  import type { StreamTool } from "./streamModel";
  let { tool }: { tool: StreamTool } = $props();
  const pending = $derived(tool.status === "pending");
  const sources = $derived(tool.sources ?? []);
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
          <span class="sweb-chip"><span class="sweb-fav" style="background:var(--accent)"></span>{s}</span>
        {/each}
        {#if sources.length > 5}<span class="sweb-more">+{sources.length - 5} more</span>{/if}
      </div>
    {/if}
  </div>
</div>
