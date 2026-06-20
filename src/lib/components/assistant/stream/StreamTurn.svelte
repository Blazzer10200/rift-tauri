<script lang="ts">
  import "$lib/styles/stream.css";
  import { Check, Send, Copy, RotateCcw } from "lucide-svelte";
  import Markdown from "../Markdown.svelte";
  import StreamThinking from "./StreamThinking.svelte";
  import WorkLine from "./WorkLine.svelte";
  import WriteBatch from "./WriteBatch.svelte";
  import StreamPlan from "./StreamPlan.svelte";
  import StreamWeb from "./StreamWeb.svelte";
  import StreamResult from "./StreamResult.svelte";
  import StreamAgent from "./StreamAgent.svelte";
  import { messageToTurn, groupBlocks, fmtDur, VERB_ING, type StreamTool } from "./streamModel";
  import { assistant, type ChatMessage } from "$lib/state/assistant.svelte";

  let { message, streaming = false, isLast = false }: { message: ChatMessage; streaming?: boolean; isLast?: boolean } = $props();

  const turn = $derived(messageToTurn(message));
  const groups = $derived(groupBlocks(turn.blocks));

  const plainText = $derived(
    message.blocks.map((b) => (b.type === "text" ? b.text : "")).join("").trim(),
  );

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  async function copy() {
    if (!plainText) return;
    try {
      await navigator.clipboard.writeText(plainText);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = false; copyTimer = null; }, 1200);
    } catch { /* clipboard denied — ignore */ }
  }

  // live status: the last pending tool drives the footer verb
  const liveTool = $derived.by((): StreamTool | null => {
    for (const g of groups) {
      if (g.type !== "work") continue;
      for (const seg of g.segs) {
        const tools = seg.seg === "rich" ? [seg.tool] : seg.tools;
        for (const t of tools) if (t.status === "pending") return t;
      }
    }
    return null;
  });
  const headLabel = $derived(
    streaming
      ? (turn.thinking?.active ? "Thinking…" : "Working…")
      : `Worked for ${fmtDur(turn.totalSecs)}`
  );
</script>

<div class="sturn">
  <div class="sturn-head {streaming ? 'live' : ''}">
    <span class="sh-dot"></span>
    <span class="sh-label">{headLabel}</span>
  </div>

  {#if turn.thinking}
    <StreamThinking active={turn.thinking.active} durSecs={turn.thinking.durSecs} text={turn.thinking.text} />
  {/if}

  {#each groups as g, gi (gi)}
    {#if g.type === "say"}
      <div class="snarr"><Markdown text={g.text} {streaming} /></div>
    {:else if g.type === "steer"}
      <div class="ssteer">
        <span class="ssteer-ic"><Send size={12} strokeWidth={2} /></span>
        <span><span class="ssteer-tag">Steered</span> {g.text}</span>
      </div>
    {:else}
      {#each g.segs as seg, si (si)}
        {#if seg.seg === "rich"}
          {#if seg.tool.kind === "plan"}
            <StreamPlan items={seg.tool.items ?? []} />
          {:else if seg.tool.kind === "web" || seg.tool.kind === "fetch"}
            <StreamWeb tool={seg.tool} />
          {:else if seg.tool.kind === "test" || seg.tool.kind === "lint"}
            <StreamResult tool={seg.tool} />
          {:else if seg.tool.kind === "agent"}
            <StreamAgent tool={seg.tool} />
          {/if}
        {:else if seg.seg === "edit"}
          <WriteBatch tools={seg.tools} />
        {:else}
          <WorkLine tools={seg.tools} />
        {/if}
      {/each}
    {/if}
  {/each}

  {#if streaming && liveTool}
    <div class="sfooter">
      <span class="sf-verb">{VERB_ING[liveTool.kind]}</span>
      <span class="sf-meta">{liveTool.cap}</span>
    </div>
  {:else if turn.applied}
    <div class="sapplied">
      <span class="ok"><Check size={13} strokeWidth={2.5} /> Applied</span>
      {#if turn.applied.add != null}<span class="add">+{turn.applied.add}</span>{/if}
      {#if turn.applied.del != null}<span class="del">−{turn.applied.del}</span>{/if}
      <span class="sapplied-meta">{turn.applied.time} · {turn.applied.cost}</span>
    </div>
  {/if}

  {#if !streaming && (plainText.length > 0 || isLast)}
    <div class="msg-acts">
      {#if plainText.length > 0}
        <button class="msg-act" class:copied type="button" onclick={copy}>
          {#if copied}<Check size={13} strokeWidth={2.5} />Copied{:else}<Copy size={13} />Copy{/if}
        </button>
      {/if}
      {#if isLast}
        <button class="msg-act" type="button" onclick={() => assistant.retryLast()}>
          <RotateCcw size={13} />Retry
        </button>
      {/if}
    </div>
  {/if}
</div>
