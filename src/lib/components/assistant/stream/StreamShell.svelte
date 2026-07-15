<script lang="ts">
  import { slide } from "svelte/transition";
  import { fmtDur, outputPeek, stripAnsi, type StreamTool } from "./streamModel";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { highlightSync, whenReady } from "$lib/state/highlighter.svelte";
  import OutputBlock from "./OutputBlock.svelte";
  import BlockHeader from "./BlockHeader.svelte";

  let { tool, poll }: { tool: StreamTool; poll?: number } = $props();

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
  // Full untrimmed command (cap truncates at 70) — copy + tooltip want the real thing.
  const fullCmd = $derived(
    typeof tool.input?.command === "string" ? (tool.input.command as string) : (tool.cap ?? ""),
  );

  // Live elapsed — anchored to mount (the block mounts the instant the tool
  // starts streaming, so mount ≈ spawn). Ticks the pill once a second while
  // running; a static "running…" told the user nothing about a slow command.
  const t0 = Date.now();
  let now = $state(t0);
  $effect(() => {
    if (!running) return;
    now = Date.now();
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  const elapsed = $derived(Math.max(0, Math.round((now - t0) / 1000)));

  // Shiki-highlighted command line — same singleton Markdown uses. The full
  // <pre> wrapper is stripped down to the inner spans so it can sit inline in
  // the head. Falls back to plain text until the highlighter warms up.
  let hlReady = $state(false);
  void whenReady().then(() => { hlReady = true; });
  const cmdHtml = $derived.by(() => {
    if (!hlReady) return null;
    const lang = flavor === "pwsh" ? "powershell" : flavor === "cmd" ? "bat" : "bash";
    const html = highlightSync(tool.cap ?? "", lang);
    if (!html) return null;
    return html.replace(/^<pre[^>]*><code[^>]*>/, "").replace(/<\/code><\/pre>\s*$/, "");
  });

  // "full" auto-expands (live in-and-out); "peek"/"minimal" start collapsed.
  // While a command is still running in "full" mode, keep it open so the user
  // watches output land. User can override either way once it's done.
  let open = $state(false);
  let touched = $state(false); // user toggled manually → stop auto-driving `open`
  $effect(() => {
    if (touched) return;
    open = mode === "full";
  });

  // Peek renders dim trailing lines — strip ANSI there (the full body gets the
  // real colors via OutputBlock's SGR parser instead).
  const peek = $derived(
    mode === "peek" ? outputPeek(tool.result != null ? stripAnsi(tool.result) : null, 3) : { lines: [], more: 0 },
  );
  function toggle() { touched = true; open = !open; }

  // Exit-status footer data — honest: we only know ok/failed (no structured
  // exit code crosses the tool-result envelope), plus duration + line count.
  const outLineCount = $derived.by(() => {
    if (!hasOut || tool.result == null) return 0;
    return stripAnsi(tool.result).replace(/\s+$/, "").split("\n").length;
  });

  // Copy the command + its output as one shell-session snippet — what you'd
  // paste into an issue or a terminal to reproduce.
  const copyText = () => {
    const body = tool.result != null ? stripAnsi(tool.result).replace(/\s+$/, "") : "";
    return body ? `${promptGlyph} ${fullCmd}\n${body}` : `${promptGlyph} ${fullCmd}`;
  };
</script>

<div class="sshell" class:bad={failed} class:running>
  <!-- IN: prompt glyph + command (Shiki-tinted) + the shared meta cluster. -->
  <BlockHeader
    expandable={hasOut && mode !== "minimal"}
    expanded={open}
    onToggle={toggle}
    pill={running ? { text: fmtDur(elapsed), tone: "running" } : failed ? { text: "failed", tone: "bad" } : null}
    durationLabel={!running && tool.durSecs >= 1 ? fmtDur(tool.durSecs) : null}
    copyText={running ? null : copyText}
  >
    {#snippet lead()}
      <span class="ssh-prompt {flavor}" use:tooltip={flavor === "pwsh" ? "Ran in PowerShell" : flavor === "cmd" ? "Ran in cmd.exe" : "Ran in bash"} aria-hidden="true">{promptGlyph}</span>
    {/snippet}
    {#snippet title()}
      <code class="ssh-cmd" title={fullCmd}>{#if cmdHtml}{@html cmdHtml}{:else}{tool.cap}{/if}</code>
      {#if poll && poll > 1}<span class="ssh-poll" title="Ran {poll} times waiting on this — showing the latest run">{running ? "waiting" : "polled"} ×{poll}</span>{/if}
      {#if running}<span class="ssh-cursor" aria-hidden="true"></span>{/if}
    {/snippet}
  </BlockHeader>

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
        <OutputBlock text={tool.result ?? ""} start={mode === "full" ? "expanded" : "collapsed"} live={running} tone="shell" fold="head-tail" />
        {#if !running}
          <div class="ssh-exit" class:bad={failed}>
            <span class="ssh-exit-mark">{failed ? "✗" : "✓"}</span>
            <span>{failed ? "failed" : "ok"}</span>
            {#if tool.durSecs >= 1}<span class="ssh-exit-pip">·</span><span>{fmtDur(tool.durSecs)}</span>{/if}
            <span class="ssh-exit-pip">·</span><span>{outLineCount} line{outLineCount === 1 ? "" : "s"}</span>
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  /* A shell block reads as a mini terminal: an IN row (prompt glyph + command)
     on a slightly raised "chrome" fill, then the OUT below it under a labeled
     rule. The two-tone fill + the prompt glyph make input-vs-output obvious at a
     glance. Meta cluster (pill/duration/copy/chevron) comes from BlockHeader. */
  .sshell { display: flex; flex-direction: column; margin: var(--stream-gap, 12px) 0; border-radius: var(--radius-lg);
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 2.5%, transparent); overflow: hidden;
    animation: blockIn var(--dur-base) var(--ease-page) both; }
  .sshell.bad { border-color: color-mix(in oklab, var(--danger) 34%, var(--border)); }
  /* the prompt glyph breathes while the command is still running — a soft glow
     halo makes a live command read as genuinely in-flight (terminal cursor
     energy). The glow is keyed to the flavor color. */
  .sshell.running .ssh-prompt { animation: sshPromptPulse 1.4s ease-in-out infinite; }
  .sshell.running .ssh-prompt.bash { text-shadow: 0 0 9px color-mix(in oklab, var(--ok) 60%, transparent); }
  .sshell.running .ssh-prompt.pwsh { text-shadow: 0 0 9px color-mix(in oklab, var(--info) 60%, transparent); }
  .sshell.running .ssh-prompt.cmd { text-shadow: 0 0 9px color-mix(in oklab, var(--warn) 60%, transparent); }
  @keyframes sshPromptPulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.45; } }
  @media (prefers-reduced-motion: reduce) {
    .sshell.running .ssh-prompt { animation: none; }
    .ssh-cursor { animation: none; }
  }

  /* IN row container chrome — layout/cluster mechanics live in BlockHeader. */
  .sshell :global(.bh) { padding: 8px 11px; gap: 9px;
    background: color-mix(in oklab, var(--fg) 3.5%, transparent);
    transition: background var(--dur-fast); }
  .sshell :global(.bh:hover:not(:disabled)) { background: color-mix(in oklab, var(--fg) 6%, transparent); }

  /* Prompt glyph — the "IN" marker, tinted per shell (bash green · pwsh blue ·
     cmd amber) so the flavor reads without a separate badge. */
  .ssh-prompt { flex: none; font-family: var(--font-mono); font-size: var(--fs-sm); font-weight: 700;
    line-height: 1; letter-spacing: 0.02em; cursor: default; user-select: none; }
  .ssh-prompt.bash { color: var(--ok); }
  .ssh-prompt.pwsh { color: var(--info); }
  .ssh-prompt.cmd { color: var(--warn); }
  /* The command itself — Shiki spans supply the tint; plain fallback stays
     bright so "what ran" is unmistakable either way. */
  .ssh-cmd { min-width: 0; font-size: var(--fs-sm); font-weight: 560; font-family: var(--font-mono);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--fg); }
  .ssh-cmd :global(span) { background: transparent !important; }
  /* Blinking block cursor while the command runs — terminal energy. */
  .ssh-cursor { flex: none; width: 7px; height: 13px; margin-left: 1px; border-radius: 1.5px;
    background: color-mix(in oklab, var(--accent) 80%, transparent);
    animation: sshBlink 1.06s steps(1) infinite; }
  @keyframes sshBlink { 0%, 49% { opacity: 1; } 50%, 100% { opacity: 0; } }

  /* Poll count — a coalesced wait-loop (3+ identical runs) shows once with
     this quiet tally instead of stacking near-identical terminal cards. */
  .ssh-poll { flex: none; font-size: 10px; font-family: var(--font-mono);
    color: var(--fg-faint); padding: 1px 6px; border-radius: 999px;
    border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
    background: color-mix(in oklab, var(--fg) 3%, transparent); }

  /* OUT — the command's output, on the base (darker) fill so it recedes behind
     the raised IN row. The line-capping / fold tiers live in OutputBlock. */
  .ssh-outwrap { border-top: 1px solid var(--border); }
  /* "output" boundary label — a tiny uppercase rule so IN vs OUT is spelled out,
     not just implied by the fill shift. */
  .ssh-outlabel { display: flex; align-items: center; gap: 7px; padding: 5px 11px 2px;
    font-size: 8.5px; font-weight: 700; letter-spacing: 0.09em; text-transform: uppercase; color: var(--fg-faint); }
  .ssh-outlabel-bad { color: var(--danger); }

  /* Exit-status footer — a slim truthful receipt: ✓/✗ · duration · line count.
     (We deliberately don't invent an exit CODE — the envelope doesn't carry one.) */
  .ssh-exit { display: flex; align-items: center; gap: 6px; padding: 4px 11px 6px;
    border-top: 1px solid color-mix(in oklch, var(--border) 45%, transparent);
    font-family: var(--font-mono); font-size: 10px; color: var(--fg-faint);
    font-variant-numeric: tabular-nums; }
  .ssh-exit .ssh-exit-mark { color: var(--ok); font-weight: 700; }
  .ssh-exit.bad .ssh-exit-mark { color: var(--danger); }
  .ssh-exit-pip { opacity: 0.5; }

  .ssh-peekwrap { border-top: 1px solid var(--border); }
  .ssh-peek { padding: 2px 11px 7px;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; color: var(--fg-subtle); }
  .ssh-line { white-space: pre-wrap; word-break: break-word; }
  .ssh-more { margin-top: 2px; font-size: 10.5px; color: var(--fg-faint); }
</style>
