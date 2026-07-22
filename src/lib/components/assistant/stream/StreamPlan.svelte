<script lang="ts">
  import { Check, ChevronDown, ListChecks } from "lucide-svelte";
  import type { PlanItem } from "./streamModel";
  let { items = [] }: { items?: PlanItem[] } = $props();
  const done = $derived(items.filter((i) => i.status === "done").length);
  const pct = $derived(items.length ? Math.round((done / items.length) * 100) : 0);
  // Long-session guard: the plan accretes all session (one TaskCreate per
  // item), so past this size the completed rows fold behind one summary line —
  // the card can't swallow the chat. Active + pending rows always stay visible.
  const FOLD_AT = 8;
  let expanded = $state(false);
  const foldable = $derived(items.length >= FOLD_AT && done > 2);
  const rows = $derived(
    foldable && !expanded
      ? items.map((it, i) => ({ it, i })).filter(({ it }) => it.status !== "done")
      : items.map((it, i) => ({ it, i })),
  );
</script>

<div class="splan">
  <div class="splan-head">
    <ListChecks size={14} strokeWidth={2} />
    <span class="splan-title">Plan</span>
    <span class="splan-track"><span class="splan-fill" style="width:{pct}%"></span></span>
    <span class="splan-count">{done}/{items.length}</span>
  </div>
  <ul class="splan-list">
    {#if foldable}
      <li>
        <button type="button" class="splan-fold" class:x={expanded} onclick={() => (expanded = !expanded)}>
          <span class="splan-mark"><Check size={13} strokeWidth={2.5} /></span>
          <span class="splan-fold-txt">{done} completed</span>
          <ChevronDown size={12} class="splan-fold-ch" />
        </button>
      </li>
    {/if}
    {#each rows as { it, i } (i)}
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
