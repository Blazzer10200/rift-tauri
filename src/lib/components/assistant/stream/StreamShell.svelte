<script lang="ts">
  import { ChevronRight, Check, Copy, X } from "lucide-svelte";
  import { slide } from "svelte/transition";
  import { fmtDur, outputPeek, type StreamTool } from "./streamModel";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import OutputBlock from "./OutputBlock.svelte";

  let { tool }: { tool: StreamTool } = $props();

  const running = $derived(tool.status === "pending");
  const failed = $derived(tool.status === "error");
  // Detailed tool-detail forces full shell output regardless of the separate
  // commandOutput pref (the Settings UI dims that control while Detailed is on).
  const mode = $derived(uiPrefs.toolDetail === "detailed" ? "full" : uiPrefs.commandOutput);
  const hasOut = $derived(typeof tool.result === "string" && tool.result.trim().length > 0);
  // Which shell ran this — the badge identity (bash · pwsh · cmd).
  const flavor = $derived(tool.flavor ?? "bash");
  // Terminal prompt glyph per shell — the "IN" marker. `$` reads as a POSIX
  // prompt (bash), `PS>` as PowerShell, `>` as cmd.exe.
  const promptGlyph = $derived(flavor === "pwsh" ? "PS>" : flavor === "cmd" ? ">" : "$");

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

  // Copy the command + its output as one shell-session snippet — what you'd
  // paste into an issue or a terminal to reproduce.
  let copied = $state(false);
  let copyFailed = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => () => { if (copyTimer) clearTimeout(copyTimer); });
  async function copyAll(e: MouseEvent) {
    e.stopPropagation();
    const cmd = tool.cap ?? "";
    const body = typeof tool.result === "string" ? tool.result.replace(/\s+$/, "") : "";
    if (copyTimer) clearTimeout(copyTimer);
    try {
      await navigator.clipboard.writeText(body ? `$ ${cmd}\n${body}` : `$ ${cmd}`);
      copied = true;
      copyFailed = false;
      copyTimer = setTimeout(() => { copied = false; copyTimer = null; }, 1200);
    } catch (err) {
      console.warn("shell copy failed", err);
      copied = false;
      copyFailed = true;
      copyTimer = setTimeout(() => { copyFailed = false; copyTimer = null; }, 1200);
    }
  }
</script>

<div class="sshell" class:bad={failed} class:running>
  <!-- ── IN: the command line. A terminal prompt glyph (flavor-colored) + the
       command in monospace reads unmistakably as "what was run", distinct from
       the output below. The whole row toggles the output. ── -->
  <button
    class="ssh-head"
    type="button"
    onclick={toggle}
    disabled={!hasOut && mode === "minimal"}
    aria-expanded={open}
  >
    <span class="ssh-prompt {flavor}" use:tooltip={flavor === "pwsh" ? "Ran in PowerShell" : flavor === "cmd" ? "Ran in cmd.exe" : "Ran in bash"} aria-hidden="true">{promptGlyph}</span>
    <code class="ssh-cmd">{tool.cap}</code>
    <span class="ssh-meta">
      {#if running}
        <span class="ssh-pill running">running…</span>
      {:else if failed}
        <span class="ssh-pill bad">exit 1</span>
      {/if}
      {#if tool.durSecs >= 1 && !running}<span class="ssh-dur">{fmtDur(tool.durSecs)}</span>{/if}
      {#if !running}
        <span
          class="ssh-copy"
          class:copied
          class:failed={copyFailed}
          role="button"
          tabindex="0"
          aria-label={copyFailed ? "Copy failed" : "Copy command and output"}
          use:tooltip={copyFailed ? "Copy failed" : "Copy command + output"}
          onclick={copyAll}
          onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); copyAll(e as unknown as MouseEvent); } }}
        >{#if copied}<Check size={11} strokeWidth={2.5} />{:else if copyFailed}<X size={11} strokeWidth={2.5} />{:else}<Copy size={11} />{/if}</span>
      {/if}
      {#if hasOut && mode !== "minimal"}
        <ChevronRight size={13} class={"ssh-chev" + (open ? " open" : "")} />
      {/if}
    </span>
  </button>

  {#if hasOut && mode !== "minimal"}
    {#if mode === "peek" && !open}
      <!-- Peek (OUT, collapsed): a few trailing lines under the output rule. -->
      <div class="ssh-peekwrap">
        <div class="ssh-outlabel"><span>output</span>{#if failed}<span class="ssh-outlabel-bad">error</span>{/if}</div>
        <div class="ssh-peek">
          {#each peek.lines as ln (ln)}<div class="ssh-line">{ln}</div>{/each}
          {#if peek.more > 0}<div class="ssh-more">+{peek.more} more line{peek.more > 1 ? "s" : ""}</div>{/if}
        </div>
      </div>
    {:else if open}
      <!-- OUT: the command's output, under a labeled rule so IN vs OUT is clear. -->
      <div class="ssh-outwrap" transition:slide={{ duration: 140 }}>
        <div class="ssh-outlabel"><span>output</span>{#if failed}<span class="ssh-outlabel-bad">error</span>{/if}</div>
        <OutputBlock text={tool.result ?? ""} start={mode === "full" ? "expanded" : "collapsed"} live={running} />
      </div>
    {/if}
  {/if}
</div>

<style>
  /* A shell block reads as a mini terminal: an IN row (prompt glyph + command)
     on a slightly raised "chrome" fill, then the OUT below it under a labeled
     rule. The two-tone fill + the prompt glyph make input-vs-output obvious at a
     glance, which the old single-slab header didn't. */
  .sshell { display: flex; flex-direction: column; margin: var(--stream-gap, 12px) 0; border-radius: var(--radius-lg);
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 2.5%, transparent); overflow: hidden;
    animation: blockIn var(--dur-base) var(--ease-page) both; }
  .sshell.bad { border-color: color-mix(in oklab, var(--danger, #f87171) 34%, var(--border)); }
  /* the prompt glyph breathes while the command is still running — a soft glow
     halo makes a live command read as genuinely in-flight (terminal cursor
     energy), not just a dimming text. The glow is keyed to the flavor color so
     bash/pwsh/cmd keep their identity while running. */
  .sshell.running .ssh-prompt { animation: sshPromptPulse 1.4s ease-in-out infinite; }
  .sshell.running .ssh-prompt.bash { text-shadow: 0 0 9px color-mix(in oklab, var(--ok) 60%, transparent); }
  .sshell.running .ssh-prompt.pwsh { text-shadow: 0 0 9px color-mix(in oklab, var(--info) 60%, transparent); }
  .sshell.running .ssh-prompt.cmd { text-shadow: 0 0 9px color-mix(in oklab, var(--warn) 60%, transparent); }
  @keyframes sshPromptPulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.45; } }
  @media (prefers-reduced-motion: reduce) {
    .sshell.running .ssh-prompt { animation: none; }
  }

  /* IN row — the command line. */
  .ssh-head { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 11px;
    text-align: left; font: inherit; cursor: pointer; min-width: 0; color: var(--fg-2);
    background: color-mix(in oklab, var(--fg) 3.5%, transparent);
    transition: background var(--dur-fast); }
  .ssh-head:hover:not(:disabled) { background: color-mix(in oklab, var(--fg) 6%, transparent); }
  .ssh-head:disabled { cursor: default; }
  /* Prompt glyph — the "IN" marker, tinted per shell (bash green · pwsh blue ·
     cmd amber) so the flavor reads without a separate badge. */
  .ssh-prompt { flex: none; font-family: var(--font-mono); font-size: 12px; font-weight: 700;
    line-height: 1; letter-spacing: 0.02em; cursor: default; user-select: none; }
  .ssh-prompt.bash { color: var(--ok); }
  .ssh-prompt.pwsh { color: var(--info); }
  .ssh-prompt.cmd { color: var(--warn); }
  /* The command itself — the star of the IN row: brighter + heavier than the
     old caption so "what ran" is unmistakable. */
  .ssh-cmd { flex: 1; min-width: 0; font-size: var(--fs-sm); font-weight: 560; font-family: var(--font-mono);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--fg); }
  /* right-side status cluster — pill · duration · copy · expand */
  .ssh-meta { flex: none; display: inline-flex; align-items: center; gap: 7px; }
  .ssh-pill { flex: none; font-size: 10px; font-weight: 650; padding: 1px 7px; border-radius: 999px;
    font-family: var(--font-mono); letter-spacing: 0.02em; }
  .ssh-pill.running { color: var(--accent); background: var(--accent-soft); }
  .ssh-pill.bad { color: var(--danger, #f87171); background: var(--danger-soft, color-mix(in oklab, red 14%, transparent)); }
  .ssh-dur { flex: none; font-size: var(--fs-xs); color: var(--fg-faint); font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  /* Copy command+output — hidden until the row is hovered, like msg-acts. */
  .ssh-copy { flex: none; display: inline-grid; place-items: center; width: 20px; height: 20px;
    border-radius: 5px; color: var(--fg-faint); opacity: 0; transition: opacity var(--dur-fast), color var(--dur-fast), background var(--dur-fast); }
  .ssh-head:hover .ssh-copy, .ssh-copy:focus-visible { opacity: 1; }
  .ssh-copy:hover { color: var(--fg-2); background: var(--surface-hover); }
  .ssh-copy.copied { color: var(--ok); opacity: 1; }
  .ssh-copy.failed { color: var(--danger); opacity: 1; }
  :global(.ssh-head .ssh-chev) { flex: none; color: var(--fg-faint); transition: transform var(--dur-fast); }
  :global(.ssh-head .ssh-chev.open) { transform: rotate(90deg); }

  /* OUT — the command's output, on the base (darker) fill so it recedes behind
     the raised IN row. The line-capping / "Show more" tiers live in OutputBlock. */
  .ssh-outwrap { border-top: 1px solid var(--border); }
  /* "output" boundary label — a tiny uppercase rule so IN vs OUT is spelled out,
     not just implied by the fill shift. */
  .ssh-outlabel { display: flex; align-items: center; gap: 7px; padding: 5px 11px 2px;
    font-size: 8.5px; font-weight: 700; letter-spacing: 0.09em; text-transform: uppercase; color: var(--fg-faint); }
  .ssh-outlabel-bad { color: var(--danger); }

  .ssh-peekwrap { border-top: 1px solid var(--border); }
  .ssh-peek { padding: 2px 11px 7px;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; color: var(--fg-subtle); }
  .ssh-line { white-space: pre-wrap; word-break: break-word; }
  .ssh-more { margin-top: 2px; font-size: 10.5px; color: var(--fg-faint); }
</style>
