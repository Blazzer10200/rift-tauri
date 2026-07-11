<script lang="ts">
  import { FlaskConical, Shield } from "lucide-svelte";
  import { slide } from "svelte/transition";
  import { fmtDur, stripAnsi, type StreamTool } from "./streamModel";
  import BlockHeader from "./BlockHeader.svelte";
  import OutputBlock from "./OutputBlock.svelte";

  let { tool }: { tool: StreamTool } = $props();
  const running = $derived(tool.status === "pending");
  const failed = $derived(tool.status === "error" || (tool.fail != null && tool.fail > 0));
  const Icon = $derived(tool.kind === "lint" ? Shield : FlaskConical);
  const hasOut = $derived(typeof tool.result === "string" && tool.result.trim().length > 0);

  // Expandable failure detail — the old pill-only row was a dead end ("5
  // failed" with no way to see WHICH). Failures auto-open (that's the payload);
  // green runs stay folded. head-tail fold keeps the summary tail visible.
  let touched = $state(false);
  let open = $state(false);
  $effect(() => {
    if (!touched && failed && hasOut) open = true;
  });
  function toggle() { touched = true; open = !open; }

  const pill = $derived.by(() => {
    if (running) return { text: "running…", tone: "running" as const };
    if (failed) return { text: tool.fail != null ? `${tool.fail} failed` : "failed", tone: "bad" as const };
    return { text: tool.pass != null ? `${tool.pass} passed` : "passed", tone: "ok" as const };
  });
</script>

<div class="sresult" class:ok={!running && !failed} class:bad={failed}>
  <BlockHeader
    expandable={hasOut}
    expanded={open}
    onToggle={toggle}
    {pill}
    durationLabel={!running && tool.durSecs >= 1 ? fmtDur(tool.durSecs) : null}
    copyText={hasOut ? () => stripAnsi(tool.result ?? "") : null}
  >
    {#snippet lead()}
      <span class="sr-ic"><Icon size={12} strokeWidth={2} /></span>
    {/snippet}
    {#snippet title()}
      <span class="sr-label">{tool.cap}</span>
    {/snippet}
  </BlockHeader>
  {#if open && hasOut}
    <div class="sr-out" transition:slide={{ duration: 140 }}>
      <OutputBlock text={tool.result ?? ""} start="collapsed" tone="shell" fold="head-tail" live={running} />
    </div>
  {/if}
</div>
