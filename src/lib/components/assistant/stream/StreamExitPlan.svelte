<script lang="ts">
  // ExitPlanMode rich block — plan mode's whole deliverable is the markdown in
  // `input.plan`, so it renders as a readable proposal card instead of the old
  // 48-char generic peek. Pending (the approval moment) = always open; settled
  // history = collapsed to a one-line summary with a toggle.
  import { ScrollText, ChevronDown } from "lucide-svelte";
  import Markdown from "../Markdown.svelte";
  import type { StreamTool } from "./streamModel";

  let { tool }: { tool: StreamTool } = $props();
  const plan = $derived(typeof tool.input?.plan === "string" ? (tool.input.plan as string) : "");
  const pending = $derived(tool.status === "pending");

  let userToggled = $state(false);
  let openPref = $state(false);
  // Open while the approval is live unless the user explicitly collapsed it.
  const open = $derived(userToggled ? openPref : pending);
  function toggle() {
    openPref = !open;
    userToggled = true;
  }
</script>

<div class="sxplan" class:pending>
  <button class="sxplan-head" type="button" onclick={toggle} aria-expanded={open}>
    <ScrollText size={14} strokeWidth={2} />
    <span class="sxplan-title">{pending ? "Plan proposed — review to continue" : "Proposed plan"}</span>
    <span class="sxplan-chev" class:open><ChevronDown size={13} strokeWidth={2.25} /></span>
  </button>
  {#if !plan}
    <div class="sxplan-empty">Drafting plan…</div>
  {:else if open}
    <div class="sxplan-body">
      <Markdown text={plan} />
    </div>
  {/if}
</div>
