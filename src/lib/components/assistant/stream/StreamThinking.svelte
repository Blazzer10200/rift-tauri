<script lang="ts">
  import { Brain, ChevronDown } from "@lucide/svelte";
  import { fmtDur } from "./streamModel";
  import Markdown from "../Markdown.svelte";
  let { active = false, durSecs = 0, text = "", workspaceRoot = null }:
    { active?: boolean; durSecs?: number; text?: string; workspaceRoot?: string | null } = $props();
  // Keep the row stable while reasoning streams. Automatically opening and
  // closing the body made provider state changes reflow the transcript.
  let open = $state(false);
  const label = $derived(active ? "Thinking…" : durSecs >= 1 ? `Thought for ${fmtDur(durSecs)}` : "Thought");
</script>

<div class="sthink">
  <!-- Disabled when there's no reasoning text: an empty thinking block (encrypted
       signature only, no plaintext) shouldn't look or act clickable (CC-UI ref §3). -->
  <button class="sthink-row" class:bare={!text} class:live={active} onclick={() => { if (text) open = !open; }} type="button" disabled={!text} aria-expanded={text ? open : undefined}>
    <Brain size={13} strokeWidth={2} />
    <span aria-live="polite">{label}</span>
    {#if text}<ChevronDown class="sthink-chev {open ? 'open' : ''}" size={13} strokeWidth={2} />{/if}
  </button>
  {#if open && text}
    <!-- Markdown, matching MessageBubble's thinking body — reasoning is often
         structured (lists, steps); a plain <p> flattened it. Streams the same
         word-reveal as prose while the pass is live. -->
    <div class="sthink-text"><Markdown {text} streaming={active} {workspaceRoot} /></div>
  {/if}
</div>
