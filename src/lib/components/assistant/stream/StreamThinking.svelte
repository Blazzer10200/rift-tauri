<script lang="ts">
  import { Brain, ChevronDown } from "lucide-svelte";
  let { active = false, durSecs = 0, text = "" }: { active?: boolean; durSecs?: number; text?: string } = $props();
  let open = $state(false);
  const label = $derived(active ? "Thinking…" : durSecs >= 1 ? `Thought for ${Math.round(durSecs)}s` : "Thought");
</script>

<div class="sthink">
  <button class="sthink-row" onclick={() => (open = !open)} type="button">
    <Brain size={13} strokeWidth={2} />
    <span>{label}</span>
    {#if text}<ChevronDown class="sthink-chev {open ? 'open' : ''}" size={13} strokeWidth={2} />{/if}
  </button>
  {#if open && text}
    <p class="sthink-text">{text}</p>
  {/if}
</div>
