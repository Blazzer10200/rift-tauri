<script lang="ts">
  import "$lib/styles/stream.css";
  import { Check, Send, Copy, RotateCcw, AlertTriangle } from "lucide-svelte";
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
  import { fmtTokens } from "$lib/state/assistant/helpers";
  import AnimatedCount from "./AnimatedCount.svelte";

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
    } catch (e) {
      console.warn("copy failed", e);
    }
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
  // The head is the TURN-level status only: "Working…" live, "Worked for Xs"
  // when done. It must NOT say "Thinking…" — and the StreamThinking block only
  // renders once thinking is DONE (the collapsible "Thought for Xs"). While
  // active, the live thinking state is already shown by the head + the footer's
  // rotating verb (which includes "Thinking…"); rendering the active block too
  // duplicated that label. Thinking-in-progress is the footer's job, not a block.
  const headLabel = $derived(
    streaming ? "Working…" : `Worked for ${fmtDur(turn.totalSecs)}`
  );

  // Live footer meta — spec's `Unfurling… 5s · 312 tokens`. 1s ticker drives
  // elapsed + tokens; both pull from the assistant store (turnStartedAt +
  // liveOutputTokens already maintained by streaming.ts, no backend work).
  let now = $state(0);
  $effect(() => {
    if (!streaming) return;
    now = Date.now();
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  const liveSecs = $derived.by(() => {
    const t = assistant.activity.turnStartedAt;
    return streaming && t != null && now > 0 ? Math.max(0, Math.round((now - t) / 1000)) : null;
  });
  const liveTokens = $derived.by(() => {
    void now;
    return streaming && assistant.liveOutputTokens > 0 ? assistant.liveOutputTokens : null;
  });

  // Live footer verb: a pending tool drives the real action word ("Reading X");
  // with no tool in flight (e.g. the model is thinking before any tool call) we
  // cycle a whimsical present-participle every ~2.4s so the turn reads as alive
  // instead of frozen. Mirrors app/stream.jsx StreamFooter (the DS reference).
  const WHIM_WORDS = [
    "Thinking", "Sussing", "Spelunking", "Pondering", "Brewing",
    "Reckoning", "Mulling", "Cogitating", "Hatching", "Conjuring",
    "Noodling", "Untangling",
  ];
  let whimTick = $state(0);
  $effect(() => {
    if (!streaming || liveTool) return;
    const id = setInterval(() => (whimTick = (whimTick + 1) % WHIM_WORDS.length), 2400);
    return () => clearInterval(id);
  });
  const footerVerb = $derived(liveTool ? VERB_ING[liveTool.kind] : `${WHIM_WORDS[whimTick]}…`);
</script>

<div class="sturn">
  <div class="sturn-head {streaming ? 'live' : ''}">
    <span class="sh-dot"></span>
    <span class="sh-label">{headLabel}</span>
  </div>

  {#if turn.thinking && !turn.thinking.active}
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

  {#if streaming}
    <div class="sfooter">
      {#key footerVerb}<span class="sf-verb">{footerVerb}</span>{/key}
      {#if liveTool?.cap}
        <span class="sf-meta">{liveTool.cap}</span>
      {/if}
      {#if liveSecs != null}
        <span class="sf-pip">·</span>
        <span class="sf-meta"><AnimatedCount value={liveSecs} durationMs={300} />s</span>
      {/if}
      {#if liveTokens != null}
        <span class="sf-pip">·</span>
        <span class="sf-meta"><AnimatedCount value={liveTokens} format={fmtTokens} /> tokens</span>
      {/if}
    </div>
  {:else if turn.outcome !== "text"}
    <div class="sapplied" data-outcome={turn.outcome}>
      {#if turn.outcome === "applied"}
        <span class="ok"><Check size={13} strokeWidth={2.5} /> Applied</span>
        <span class="files">{turn.files} file{turn.files === 1 ? "" : "s"}</span>
      {:else if turn.outcome === "failed"}
        <span class="bad"><AlertTriangle size={13} strokeWidth={2.5} /> Changes failed</span>
      {:else}
        <span class="ran"><Check size={13} strokeWidth={2.5} /> Done</span>
      {/if}
      {#if turn.meta}
        <span class="sapplied-meta">{turn.meta.time}{#if turn.meta.cost} · {turn.meta.cost}{/if}</span>
      {/if}
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
