<script lang="ts">
  import { ChevronDown, FileText, Search, Terminal, Wrench } from "lucide-svelte";
  import { groupSummary, VERB_PAST, VERB_ING, type StreamTool, type TKind } from "./streamModel";

  let { tools }: { tools: StreamTool[] } = $props();
  let open = $state(false);

  const ICONS: Partial<Record<TKind, typeof FileText>> = {
    read: FileText, grep: Search, shell: Terminal, mcp: Wrench,
  };
  const lead = $derived(tools[0]?.kind ?? "mcp");
  const Icon = $derived(ICONS[lead] ?? Wrench);
  const anyActive = $derived(tools.some((t) => t.status === "pending"));
  const summary = $derived(groupSummary(tools));
</script>

{#if anyActive && tools.length === 1}
  <div class="wline-active" title={tools[0].path ?? tools[0].cap}>
    <span class="wa-dot"></span>
    <span class="wa-text">{VERB_ING[tools[0].kind]}</span>
    <span class="wa-cap">{#if tools[0].dir}<span class="wa-dir">{tools[0].dir}</span>{/if}<b>{tools[0].cap}</b></span>
  </div>
{:else}
  <div class="wline">
    <button class="wline-head" onclick={() => (open = !open)} type="button">
      <span class="wline-ic"><Icon size={12} strokeWidth={2} /></span>
      <span class="wline-label">{summary}</span>
      <ChevronDown class="wline-chev {open ? 'open' : ''}" size={13} strokeWidth={2} />
    </button>
    {#if open}
      <div class="wline-list">
        {#each tools as t (t.id)}
          <div class="wline-row" title={t.path ?? t.cap}>
            <span class="wr-verb">{t.status === "pending" ? VERB_ING[t.kind] : VERB_PAST[t.kind]}</span>
            {#if t.dir}<span class="wr-dir">{t.dir}</span>{/if}<b>{t.cap}</b>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
