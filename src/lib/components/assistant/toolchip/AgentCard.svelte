<script lang="ts">
  import { Loader2, CheckCircle2, AlertCircle, Bot } from "@lucide/svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import Markdown from "../Markdown.svelte";

  let {
    agentSubtype,
    agentDescription,
    agentPrompt,
    agentResult,
    isError,
    status,
    durationLabel,
    workspaceRoot = null,
  }: {
    agentSubtype: string;
    agentDescription: string | null;
    agentPrompt: string | null;
    agentResult: { text: string; truncated: number } | null;
    isError: boolean;
    status: "pending" | "done" | "error";
    durationLabel: string | null;
    workspaceRoot?: string | null;
  } = $props();

  // Status glyph tone — `.tile-glyph` is neutral by kind, colored by STATUS
  // only (DESIGN §8: never tint an icon for "this is an Agent"). Mirrors
  // BlockHeader's PILL_TONE mapping onto the same app.css primitives.
  const glyphTone = $derived(status === "pending" ? "is-live" : status === "error" ? "is-bad" : "is-ok");
</script>

<!-- Agent card head — padding/font/hairline are the shared `.tile-head`
     recipe (app.css); the accent wash below is this card's own identity
     tint layered on top of it. -->
<div class="tile-head agent-head">
  <span class="agent-icon"><Bot size={14} /></span>
  <span class="tile-pill">{agentSubtype}</span>
  {#if agentDescription}
    <span class="agent-desc">{agentDescription}</span>
  {/if}
  {#if durationLabel}
    <span class="tile-dur" use:tooltip={"Wall-clock duration"}>{durationLabel}</span>
  {/if}
  <span class="tile-glyph {glyphTone}" aria-label={status === "pending" ? "Running" : status === "error" ? "Error" : "Done"}>
    {#if status === "pending"}<Loader2 size={12} class="chip-spin" />
    {:else if status === "error"}<AlertCircle size={12} />
    {:else}<CheckCircle2 size={12} />{/if}
  </span>
</div>

<!-- Agent card body -->
<div class="tile-body agent-body">
  {#if agentPrompt}
    <div class="agent-prompt-wrap">
      <span class="tile-label">prompt</span>
      <blockquote class="agent-prompt">{agentPrompt}</blockquote>
    </div>
  {/if}
  <div class="tile-label">{isError ? "error" : status === "pending" ? "working…" : "result"}</div>
  {#if status === "pending" && !agentResult}
    <div class="agent-pending">
      <span class="dots" aria-hidden="true"><span class="dot"></span><span class="dot"></span><span class="dot"></span></span>
      <span>Agent running…</span>
    </div>
  {:else if isError}
    <pre class="result error">{agentResult?.text ?? ""}</pre>
  {:else if agentResult}
    <div class="agent-result"><Markdown text={agentResult.text} {workspaceRoot} /></div>
    {#if agentResult.truncated > 0}
      <div class="tile-label">+{agentResult.truncated.toLocaleString()} more chars truncated</div>
    {/if}
  {/if}
</div>

<style>
  /* Head/pill/duration/glyph/label chrome now come from the global `.tile-*`
     system (app.css "stream tiles") — this file only owns the agent-specific
     bits: the accent identity wash on the head, and the prompt/result body. */
  .agent-head { background: color-mix(in oklab, var(--accent) 8%, transparent); }
  .agent-icon { display: inline-flex; color: var(--fg-muted); flex-shrink: 0; }
  .agent-desc {
    flex: 1; min-width: 0;
    color: var(--fg-2);
    font-size: var(--fs-sm);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .agent-body {
    padding: 10px 14px 12px;
    display: flex; flex-direction: column;
    gap: 8px;
  }
  .agent-prompt-wrap { display: flex; flex-direction: column; gap: 4px; }
  .agent-prompt {
    margin: 0;
    padding: 8px 12px;
    border-left: 2px solid color-mix(in oklab, var(--accent) 50%, transparent);
    background: color-mix(in oklch, var(--bg-elev-1) 80%, transparent);
    color: var(--fg-2);
    font-size: 11.5px;
    line-height: 1.5;
    font-style: italic;
    border-radius: 0 var(--radius-xs) var(--radius-xs) 0;
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 180px;
    overflow-y: auto;
  }
  .agent-result {
    padding: 4px 0;
    font-size: 12.5px;
    line-height: 1.55;
  }
  .agent-pending {
    display: inline-flex; align-items: center; gap: 8px;
    color: var(--fg-muted);
    font-size: 11.5px;
    font-style: italic;
  }
  .agent-pending .dots { display: inline-flex; gap: 3px; }
  .agent-pending .dot {
    width: 5px; height: 5px;
    border-radius: 50%;
    background: var(--accent);
    animation: agent-dot 1.1s ease-in-out infinite;
  }
  .agent-pending .dot:nth-child(2) { animation-delay: 0.15s; }
  .agent-pending .dot:nth-child(3) { animation-delay: 0.3s; }
  @keyframes agent-dot {
    0%, 60%, 100% { opacity: 0.3; transform: scale(0.85); }
    30% { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .agent-pending .dot { animation: none; }
  }

  .result {
    margin: 0;
    padding: 6px 8px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-xs);
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 280px;
    overflow: auto;
    color: var(--fg-2);
  }
  .result.error {
    border-color: color-mix(in oklab, var(--danger) 35%, var(--border));
    color: var(--danger);
    background: color-mix(in oklch, var(--danger-soft) 25%, var(--bg-elev-1));
  }

  .tile-glyph :global(.chip-spin) { animation: chip-spin 1s linear infinite; }
  @keyframes chip-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
</style>
