<script lang="ts">
  import { Terminal, ChevronRight } from "lucide-svelte";
  import { slide } from "svelte/transition";
  import { fmtDur, outputPeek, VERB_ING, VERB_PAST, type StreamTool } from "./streamModel";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";

  let { tool, streaming = false }: { tool: StreamTool; streaming?: boolean } = $props();

  const running = $derived(tool.status === "pending");
  const failed = $derived(tool.status === "error");
  const mode = $derived(uiPrefs.commandOutput);
  const hasOut = $derived(typeof tool.result === "string" && tool.result.trim().length > 0);

  // "full" auto-expands (live in-and-out); "peek"/"minimal" start collapsed.
  // While a command is still running in "full" mode, keep it open so the user
  // watches output land. User can override either way once it's done.
  let open = $state(false);
  let touched = $state(false); // user toggled manually → stop auto-driving `open`
  $effect(() => {
    if (touched) return;
    open = mode === "full";
  });

  const peek = $derived(mode === "peek" ? outputPeek(tool.result, 3) : { lines: [], more: 0 });
  function toggle() { touched = true; open = !open; }
</script>

<div class="sshell" class:bad={failed}>
  <button
    class="ssh-head"
    type="button"
    onclick={toggle}
    disabled={!hasOut && mode === "minimal"}
    aria-expanded={open}
  >
    <span class="ssh-ic"><Terminal size={12} strokeWidth={2} /></span>
    <span class="ssh-verb">{running ? VERB_ING[tool.kind] : VERB_PAST[tool.kind]}</span>
    <b class="ssh-cap">{tool.cap}</b>
    {#if running}
      <span class="ssh-pill running">running…</span>
    {:else if failed}
      <span class="ssh-pill bad">failed</span>
    {/if}
    {#if tool.durSecs >= 1 && !running}<span class="ssh-dur">{fmtDur(tool.durSecs)}</span>{/if}
    {#if hasOut && mode !== "minimal"}
      <ChevronRight size={13} class={"ssh-chev" + (open ? " open" : "")} />
    {/if}
  </button>

  {#if hasOut && mode !== "minimal"}
    {#if mode === "peek" && !open}
      <!-- Peek: a few trailing lines, click the head to expand the rest. -->
      <div class="ssh-peek">
        {#each peek.lines as ln (ln)}<div class="ssh-line">{ln}</div>{/each}
        {#if peek.more > 0}<div class="ssh-more">+{peek.more} more line{peek.more > 1 ? "s" : ""}</div>{/if}
      </div>
    {:else if open}
      <pre class="ssh-out" transition:slide={{ duration: 140 }}>{tool.result}</pre>
    {/if}
  {/if}
</div>

<style>
  .sshell { display: flex; flex-direction: column; border-radius: var(--radius-lg);
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 2%, transparent); overflow: hidden; }
  .sshell.bad { border-color: color-mix(in oklab, var(--danger, #f87171) 40%, var(--border)); }

  .ssh-head { display: flex; align-items: center; gap: 7px; width: 100%; padding: 6px 9px;
    text-align: left; font: inherit; cursor: pointer; min-width: 0; color: var(--fg-2);
    transition: background var(--dur-fast); }
  .ssh-head:hover:not(:disabled) { background: var(--surface-hover); }
  .ssh-head:disabled { cursor: default; }
  .ssh-ic { flex: none; display: grid; place-items: center; color: var(--fg-faint); }
  .ssh-verb { flex: none; font-size: var(--fs-xs); color: var(--fg-subtle); }
  .ssh-cap { flex: 1; min-width: 0; font-size: var(--fs-sm); font-weight: 560; font-family: var(--font-mono);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--fg); }
  .ssh-pill { flex: none; font-size: 10.5px; font-weight: 650; padding: 1px 7px; border-radius: 999px; }
  .ssh-pill.running { color: var(--accent); background: var(--accent-soft); }
  .ssh-pill.bad { color: var(--danger, #f87171); background: var(--danger-soft, color-mix(in oklab, red 14%, transparent)); }
  .ssh-dur { flex: none; font-size: var(--fs-xs); color: var(--fg-faint); }
  :global(.ssh-head .ssh-chev) { flex: none; color: var(--fg-faint); transition: transform var(--dur-fast); }
  :global(.ssh-head .ssh-chev.open) { transform: rotate(90deg); }

  /* Terminal-style output body — monospace, inset, hugging the command head. */
  .ssh-out { margin: 0; padding: 8px 11px; border-top: 1px solid var(--border);
    background: var(--bg-elev-1, color-mix(in oklab, var(--fg) 4%, transparent));
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; color: var(--fg-2);
    white-space: pre-wrap; word-break: break-word; max-height: 340px; overflow: auto; }

  .ssh-peek { padding: 5px 11px 7px; border-top: 1px solid var(--border);
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; color: var(--fg-subtle); }
  .ssh-line { white-space: pre-wrap; word-break: break-word; }
  .ssh-more { margin-top: 2px; font-size: 10.5px; color: var(--fg-faint); }
</style>
