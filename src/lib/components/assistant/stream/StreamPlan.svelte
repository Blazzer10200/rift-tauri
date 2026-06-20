<script lang="ts">
  import { Check, ListChecks } from "lucide-svelte";
  import type { PlanItem } from "./streamModel";
  let { items = [] }: { items?: PlanItem[] } = $props();
  const done = $derived(items.filter((i) => i.status === "done").length);
  const pct = $derived(items.length ? Math.round((done / items.length) * 100) : 0);
</script>

<div class="splan">
  <div class="splan-head">
    <ListChecks size={14} strokeWidth={2} />
    <span class="splan-title">Plan</span>
    <span class="splan-track"><span class="splan-fill" style="width:{pct}%"></span></span>
    <span class="splan-count">{done}/{items.length}</span>
  </div>
  <ul class="splan-list">
    {#each items as it, i (i)}
      <li class="splan-item is-{it.status}">
        <span class="splan-mark">
          {#if it.status === "done"}<Check size={13} strokeWidth={2.5} />
          {:else if it.status === "active"}<span class="splan-ring"></span>
          {:else}<span style="width:7px;height:7px;border-radius:50%;border:1.5px solid currentColor;display:block"></span>{/if}
        </span>
        <span class="splan-text">{it.text}</span>
      </li>
    {/each}
  </ul>
</div>
