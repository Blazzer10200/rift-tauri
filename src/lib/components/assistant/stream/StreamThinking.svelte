<script lang="ts">
  import { Brain, ChevronDown } from "lucide-svelte";
  import { fmtDur } from "./streamModel";
  import Markdown from "../Markdown.svelte";
  let { active = false, durSecs = 0, text = "" }: { active?: boolean; durSecs?: number; text?: string } = $props();
  let open = $state(false);
  const label = $derived(active ? "Thinking…" : durSecs >= 1 ? `Thought for ${fmtDur(durSecs)}` : "Thought");
</script>

<div class="sthink">
  <!-- Disabled when there's no reasoning text: an empty thinking block (encrypted
       signature only, no plaintext) shouldn't look or act clickable (CC-UI ref §3). -->
  <button class="sthink-row" class:bare={!text} onclick={() => text && (open = !open)} type="button" disabled={!text} aria-expanded={text ? open : undefined}>
    <Brain size={13} strokeWidth={2} />
    <span>{label}</span>
    {#if text}<ChevronDown class="sthink-chev {open ? 'open' : ''}" size={13} strokeWidth={2} />{/if}
  </button>
  {#if open && text}
    <!-- Markdown, matching MessageBubble's thinking body — reasoning is often
         structured (lists, steps); a plain <p> flattened it. -->
    <div class="sthink-text"><Markdown {text} /></div>
  {/if}
</div>
