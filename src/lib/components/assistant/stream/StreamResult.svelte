<script lang="ts">
  import { FlaskConical, Shield } from "lucide-svelte";
  import { fmtDur, type StreamTool } from "./streamModel";
  let { tool }: { tool: StreamTool } = $props();
  const running = $derived(tool.status === "pending");
  const failed = $derived(tool.status === "error" || (tool.fail != null && tool.fail > 0));
  const Icon = $derived(tool.kind === "lint" ? Shield : FlaskConical);
</script>

<div class="sresult">
  <span class="sr-ic"><Icon size={12} strokeWidth={2} /></span>
  <span class="sr-label">{tool.cap}</span>
  {#if running}
    <span class="sr-pill running">running…</span>
  {:else if failed}
    <span class="sr-pill bad">{tool.fail != null ? `${tool.fail} failed` : "failed"}</span>
  {:else}
    <span class="sr-pill ok">{tool.pass != null ? `${tool.pass} passed` : "passed"}</span>
  {/if}
  {#if tool.durSecs >= 1}<span class="sr-dur">{fmtDur(tool.durSecs)}</span>{/if}
</div>
