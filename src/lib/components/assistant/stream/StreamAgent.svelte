<script lang="ts">
  import { ArrowRight, Bot, ChevronDown } from "lucide-svelte";
  import { fmtDur, type StreamTool } from "./streamModel";
  let { tool }: { tool: StreamTool } = $props();
  let open = $state(false);
  const running = $derived(tool.status === "pending");
  // The CLI does not stream a delegated sub-agent's intermediate tool steps —
  // we only see its spawn and its final result (see docs/ISSUES.md). So `steps`
  // is always empty; the card's detail IS the returned result line. Expandable
  // whenever a result landed; the live trail lives in the SubAgentDock.
  const steps = $derived(tool.steps ?? []);
  const expandable = $derived(steps.length > 0 || !!tool.result);
</script>

<div class="sagent {open ? 'open' : ''}">
  <button class="sagent-head" onclick={() => expandable && (open = !open)} type="button">
    <span class="sagent-ic {running ? 'spin' : ''}"><Bot size={13} strokeWidth={2} /></span>
    <span class="sagent-title">Delegated <b>{tool.task ?? tool.cap}</b></span>
    {#if running}
      <span class="sagent-meta sagent-run">working…</span>
    {:else if tool.durSecs >= 1}
      <span class="sagent-meta">{fmtDur(tool.durSecs)}</span>
    {/if}
    {#if expandable}<ChevronDown class="sagent-chev {open ? 'open' : ''}" size={13} strokeWidth={2} />{/if}
  </button>
  {#if running}
    <div class="sagent-hint">Sub-agent running in its own context — its result lands here when done.</div>
  {/if}
  {#if open && expandable}
    <div class="sagent-body">
      {#each steps as s, i (i)}
        <div class="sagent-step"><span class="sagent-bullet"></span>{s}</div>
      {/each}
      {#if tool.result}
        <div class="sagent-result"><ArrowRight size={13} strokeWidth={2} />{tool.result}</div>
      {/if}
    </div>
  {/if}
</div>
