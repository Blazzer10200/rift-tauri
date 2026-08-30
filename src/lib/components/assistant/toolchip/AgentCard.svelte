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
</script>

<!-- Agent card head -->
<div class="agent-head" data-status={status}>
  <span class="agent-icon"><Bot size={14} /></span>
  <span class="agent-pill">{agentSubtype}</span>
  {#if agentDescription}
    <span class="agent-desc">{agentDescription}</span>
  {/if}
  {#if durationLabel}
    <span class="chip-duration mono" use:tooltip={"Wall-clock duration"}>{durationLabel}</span>
  {/if}
  <span class="chip-status" aria-label={status === "pending" ? "Running" : status === "error" ? "Error" : "Done"}>
    {#if status === "pending"}<Loader2 size={12} class="chip-spin" />
    {:else if status === "error"}<AlertCircle size={12} />
    {:else}<CheckCircle2 size={12} />{/if}
  </span>
</div>

<!-- Agent card body -->
<div class="agent-body">
  {#if agentPrompt}
    <div class="agent-prompt-wrap">
      <span class="agent-field-label">prompt</span>
      <blockquote class="agent-prompt">{agentPrompt}</blockquote>
    </div>
  {/if}
  <div class="agent-field-label">{isError ? "error" : status === "pending" ? "working…" : "result"}</div>
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
      <div class="agent-field-label">+{agentResult.truncated.toLocaleString()} more chars truncated</div>
    {/if}
  {/if}
</div>

<style>
  /* Agent head */
  .agent-head {
    display: flex; align-items: center; gap: 9px;
    padding: 8px 12px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .agent-icon {
    display: inline-flex;
    color: var(--accent-hover);
    flex-shrink: 0;
  }
  .agent-pill {
    display: inline-flex; align-items: center;
    padding: 2px 9px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 22%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 40%, var(--border));
    color: var(--accent-hover);
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    font-family: var(--font-mono, monospace);
    flex-shrink: 0;
  }
  .agent-desc {
    flex: 1; min-width: 0;
    color: var(--fg-2);
    font-size: 12px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .agent-body {
    padding: 10px 14px 12px;
    display: flex; flex-direction: column;
    gap: 8px;
  }
  .agent-field-label {
    font-size: 9.5px;
    text-transform: lowercase;
    letter-spacing: 0.04em;
    color: var(--fg-muted);
    font-weight: 600;
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
    border-radius: 0 4px 4px 0;
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

  .chip-duration {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    color: var(--fg-muted);
    border: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    font-weight: 600;
  }
  .chip-status {
    display: inline-flex;
    flex-shrink: 0;
  }
  /* Status lives in the glyph (CC-UI ref §9): activity green while running,
     outcome tokens once settled. */
  .agent-head[data-status="pending"] .chip-status { color: var(--status-busy); }
  .agent-head[data-status="done"] .chip-status { color: var(--ok); }
  .agent-head[data-status="error"] .chip-status { color: var(--danger); }
  .chip-status :global(.chip-spin) { animation: chip-spin 1s linear infinite; }
  @keyframes chip-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }

  .result {
    margin: 0;
    padding: 6px 8px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: 4px;
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
    color: oklch(0.88 0.07 22);
    background: color-mix(in oklch, var(--danger-soft) 25%, var(--bg-elev-1));
  }

  .mono { font-family: var(--font-mono, monospace); }
</style>
