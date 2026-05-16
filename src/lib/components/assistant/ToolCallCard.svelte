<script lang="ts">
  import { FileText, FolderTree, Search, ChevronRight, Loader2, CheckCircle2, AlertCircle } from "lucide-svelte";
  import type { ToolBlock } from "../../state/assistant.svelte";

  let { call }: { call: ToolBlock } = $props();

  let expanded = $state(false);

  const shortName = $derived(call.name.replace(/^mcp__rift__/, ""));

  // Derive a tight one-liner for the collapsed state.
  const summary = $derived.by(() => {
    const inp = call.input ?? {};
    switch (shortName) {
      case "read_file":
        return typeof inp.path === "string" ? inp.path : "file";
      case "list_dir":
        return typeof inp.path === "string" ? inp.path : "directory";
      case "grep": {
        const pat = typeof inp.pattern === "string" ? inp.pattern : "?";
        const scope = typeof inp.path === "string" ? ` in ${inp.path}` : "";
        const glob = typeof inp.glob === "string" ? ` (${inp.glob})` : "";
        return `\`${pat}\`${scope}${glob}`;
      }
      default:
        return shortName;
    }
  });

  const statusTone = $derived(
    call.status === "pending" ? "running" : call.status === "error" ? "error" : "ok",
  );

  function toggle() {
    expanded = !expanded;
  }
</script>

<div class="tool-card" data-tone={statusTone}>
  <button class="tool-head" type="button" onclick={toggle} aria-expanded={expanded}>
    <span class="chev" class:open={expanded}><ChevronRight size={12} /></span>
    <span class="icon">
      {#if shortName === "read_file"}
        <FileText size={13} />
      {:else if shortName === "list_dir"}
        <FolderTree size={13} />
      {:else if shortName === "grep"}
        <Search size={13} />
      {:else}
        <FileText size={13} />
      {/if}
    </span>
    <span class="name">{shortName}</span>
    <span class="summary">{summary}</span>
    <span class="status">
      {#if statusTone === "running"}
        <Loader2 size={12} class="spin" />
      {:else if statusTone === "error"}
        <AlertCircle size={12} />
      {:else}
        <CheckCircle2 size={12} />
      {/if}
    </span>
  </button>

  {#if expanded}
    <div class="tool-body">
      <div class="field-label">input</div>
      <pre class="block">{JSON.stringify(call.input, null, 2)}</pre>
      <div class="field-label">{call.isError ? "error" : "result"}</div>
      <pre class="block result" class:error={call.isError}>{call.result ?? "(running…)"}</pre>
    </div>
  {/if}
</div>

<style>
  .tool-card {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    font-size: var(--fs-sm);
    overflow: hidden;
    transition: border-color 200ms ease-out, background 200ms ease-out;
  }
  .tool-card:hover { border-color: var(--border-strong); }
  .tool-card[data-tone="error"] {
    border-color: color-mix(in oklch, var(--danger) 40%, var(--border));
    animation: tc-error-flash 600ms ease-out;
  }
  .tool-card[data-tone="running"] {
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  @keyframes tc-error-flash {
    0%   { background: color-mix(in oklch, var(--danger) 18%, var(--surface)); }
    100% { background: var(--surface); }
  }
  .status :global(svg) {
    animation: tc-status-in 280ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes tc-status-in {
    from { opacity: 0; transform: scale(0.6); }
    to   { opacity: 1; transform: scale(1); }
  }
  .tool-head {
    display: flex; align-items: center; gap: 8px;
    width: 100%;
    padding: 5px 10px;
    background: transparent;
    border: 0;
    color: var(--fg);
    cursor: pointer;
    text-align: left;
    font: inherit;
    min-height: 28px;
  }
  .tool-head:hover { background: var(--surface-hover); }
  .chev { display: inline-flex; transition: transform 120ms ease; color: var(--fg-muted); }
  .chev.open { transform: rotate(90deg); }
  .icon { display: inline-flex; color: var(--accent); }
  .name {
    font-family: var(--font-mono, monospace);
    font-size: var(--fs-xs);
    color: var(--fg-muted);
  }
  .summary {
    flex: 1;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-family: var(--font-mono, monospace);
    font-size: var(--fs-xs);
  }
  .status { display: inline-flex; color: var(--fg-muted); }
  .tool-card[data-tone="ok"] .status { color: oklch(0.76 0.18 152); }
  .tool-card[data-tone="error"] .status { color: var(--danger); }
  .tool-card[data-tone="running"] .status { color: var(--accent); }
  :global(.spin) { animation: tc-spin 1s linear infinite; }
  @keyframes tc-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }

  .tool-body {
    padding: 8px 10px 10px;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }
  .field-label {
    font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em;
    color: var(--fg-muted);
    margin: 6px 0 4px;
  }
  .field-label:first-child { margin-top: 0; }
  .block {
    margin: 0;
    padding: 6px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: var(--fs-xs);
    line-height: 1.45;
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 320px;
    overflow: auto;
  }
  .block.error {
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    color: oklch(0.88 0.07 22);
  }
</style>
