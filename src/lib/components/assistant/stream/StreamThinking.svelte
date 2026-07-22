<script lang="ts">
  import { Brain, ChevronDown } from "lucide-svelte";
  import { fmtDur } from "./streamModel";
  import Markdown from "../Markdown.svelte";
  let { active = false, durSecs = 0, text = "" }: { active?: boolean; durSecs?: number; text?: string } = $props();
  // Auto-driven while untouched: a LIVE pass with visible text streams open
  // (watching the model reason is the payload), then settles closed when the
  // pass ends. A manual toggle takes over for good.
  let touched = $state(false);
  let open = $state(false);
  $effect(() => { if (!touched) open = active && !!text; });
  const label = $derived(active ? "Thinking…" : durSecs >= 1 ? `Thought for ${fmtDur(durSecs)}` : "Thought");
</script>

<div class="sthink">
  <!-- Disabled when there's no reasoning text: an empty thinking block (encrypted
       signature only, no plaintext) shouldn't look or act clickable (CC-UI ref §3). -->
  <button class="sthink-row" class:bare={!text} class:live={active} onclick={() => { if (text) { touched = true; open = !open; } }} type="button" disabled={!text} aria-expanded={text ? open : undefined}>
    <Brain size={13} strokeWidth={2} />
    <span>{label}</span>
    {#if text}<ChevronDown class="sthink-chev {open ? 'open' : ''}" size={13} strokeWidth={2} />{/if}
  </button>
  {#if open && text}
    <!-- Markdown, matching MessageBubble's thinking body — reasoning is often
         structured (lists, steps); a plain <p> flattened it. Streams the same
         word-reveal as prose while the pass is live. -->
    <div class="sthink-text"><Markdown {text} streaming={active} /></div>
  {/if}
</div>
