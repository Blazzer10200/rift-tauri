<script lang="ts">
  import { Brain, ChevronDown } from "lucide-svelte";
  import { fmtDur } from "./streamModel";
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
    <p class="sthink-text">{text}</p>
  {/if}
</div>
