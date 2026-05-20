<script lang="ts">
  import { ListChecks, FolderOpen, Folder, X, TerminalSquare, Layers } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";

  function leafName(p: string): string {
    const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
    const parts = norm.split("/");
    return parts[parts.length - 1] || norm;
  }

  function shortAgo(sinceMs: number): string {
    const sec = Math.max(0, Math.floor((Date.now() - sinceMs) / 1000));
    if (sec < 60) return `${sec}s`;
    if (sec < 3600) return `${Math.floor(sec / 60)}m`;
    return `${Math.floor(sec / 3600)}h`;
  }

  const foreignShell = $derived(assistant.remoteShellLockedByOther);

  const authWarn = $derived.by(() => {
    const a = assistant.auth;
    if (!a) return null;
    if (a.pill === "yellow") return { tone: "yellow", text: "API key" };
    if (a.pill === "red") return { tone: "red", text: "Not connected" };
    return null;
  });

  const taskCount = $derived(assistant.tasks.length);
  const taskDone = $derived(assistant.tasks.filter((t) => t.status === "completed").length);

  let pulse = $state(false);
  let lastSeenUpdate = 0;
  $effect(() => {
    const t = assistant.ui.tasksUpdatedAt;
    if (t > lastSeenUpdate) {
      lastSeenUpdate = t;
      if (!assistant.ui.dockOpen && taskCount > 0) {
        pulse = true;
        setTimeout(() => (pulse = false), 700);
      }
    }
  });

  const tasksOpen = $derived(assistant.ui.dockOpen);
  function toggleTasks() {
    assistant.ui.dockOpen = !assistant.ui.dockOpen;
  }

  function shortK(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 10_000) return `${Math.round(n / 1000)}K`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return String(n);
  }

  const newTokens = $derived(
    assistant.lastTurnUsage
      ? assistant.lastTurnUsage.input + assistant.lastTurnUsage.cacheCreate
      : 0,
  );
  const ctxTokens = $derived(assistant.ctxTokens);
  const ctxWindow = $derived(assistant.ctxWindow);
  const ctxPct = $derived(assistant.ctxPct);
  const ctxTone = $derived(ctxPct >= 90 ? "red" : ctxPct >= 70 ? "yellow" : "ok");
  const compactWarning = $derived(assistant.compactWarning);
  const activeAgents = $derived(
    (assistant.activeTab?.agentSpawns ?? []).filter((a) => a.completedAt === null),
  );
  const activeAgentTitle = $derived.by(() => {
    if (activeAgents.length === 0) return "";
    return activeAgents
      .map((a) => `${a.subagentType}: ${a.description}`)
      .join("\n");
  });
  const ctxTitle = $derived.by(() => {
    const u = assistant.lastTurnUsage;
    if (!u) return "Context — send a message to populate";
    const s = assistant.sessionUsage;
    const cost =
      assistant.totalCostUsd !== null && assistant.totalCostUsd !== undefined
        ? ` · $${assistant.totalCostUsd.toFixed(4)}`
        : "";
    const hint =
      ctxPct >= 85
        ? "\n\nWindow nearly full — start a new chat (Ctrl+T) to drop the accumulated tool-result history."
        : ctxPct >= 70
          ? "\n\nCache is growing — Ctrl+T for a fresh chat if responses start lagging."
          : "";
    return (
      `Context this turn: ${ctxTokens.toLocaleString()} / ${ctxWindow.toLocaleString()} (${ctxPct.toFixed(1)}%)\n` +
      `  • new this turn: ${newTokens.toLocaleString()} (input ${u.input.toLocaleString()} + cache-create ${u.cacheCreate.toLocaleString()})\n` +
      `  • cache read: ${u.cacheRead.toLocaleString()} (replayed from prior turns)\n` +
      `  • output: ${u.output.toLocaleString()}\n` +
      `Session totals (${s.turns} turn${s.turns === 1 ? "" : "s"}):\n` +
      `  • input: ${s.totalInput.toLocaleString()} · output: ${s.totalOutput.toLocaleString()}\n` +
      `  • cache read: ${s.totalCacheRead.toLocaleString()} · cache create: ${s.totalCacheCreate.toLocaleString()}${cost}\n` +
      `Model: ${assistant.lastModelId ?? "?"}${hint}`
    );
  });
</script>

<div class="ah-bar">
  <div class="ah-left">
    {#if assistant.workspace.current}
      <span class="ws-chip" title={assistant.workspace.current}>
        <Folder size={11}/>
        <span class="ws-name">{leafName(assistant.workspace.current)}</span>
        <button
          class="ws-x"
          type="button"
          title="Close folder"
          onclick={() => void assistant.clearRoot()}
        ><X size={10}/></button>
      </span>
    {:else}
      <button
        class="hdr-btn"
        type="button"
        title="Open project folder"
        onclick={() => void assistant.pickFolder()}
      >
        <FolderOpen size={13}/>
        <span class="hdr-btn-label">Open folder</span>
      </button>
    {/if}
  </div>

  <div class="ah-right">
    {#if authWarn}
      <span
        class="auth-warn"
        data-tone={authWarn.tone}
        title={assistant.auth?.summary ?? authWarn.text}
      >
        <span class="dot"></span>
        <span>{authWarn.text}</span>
      </span>
    {/if}

    {#if foreignShell}
      <span
        class="shell-lock"
        title={`${foreignShell.user}@${foreignShell.host} is running a remote command`}
      >
        <TerminalSquare size={11}/>
        <span>{foreignShell.user} ({shortAgo(foreignShell.sinceMs)})</span>
      </span>
    {/if}

    {#if activeAgents.length > 0}
      <span class="agents-pill" title={activeAgentTitle}>
        <span class="agents-dot"></span>
        <span>{activeAgents.length} agent{activeAgents.length === 1 ? "" : "s"}</span>
      </span>
    {/if}

    {#if compactWarning}
      <span class="compact-warn" title="Compact early w/ /compact <focus> if you want fine control over the summary."
        >{compactWarning}</span>
    {/if}

    {#if ctxTokens > 0}
      <span class="ctx-pill" data-tone={ctxTone} title={ctxTitle}>
        <span class="ctx-bar"><span class="ctx-fill" style="width: {ctxPct}%"></span></span>
        <span class="ctx-text">{shortK(ctxTokens)}<span class="ctx-sep">/</span>{shortK(ctxWindow)}</span>
        <span class="ctx-pct">{Math.round(ctxPct)}%</span>
      </span>
    {/if}

    {#if ctxPct >= 50}
      <button
        class="compact-btn"
        data-tone={ctxTone}
        type="button"
        onclick={() => {
          const cost = ctxPct >= 70 ? "≈ $0.91" : "≈ $0.30";
          if (!confirm(`Compact conversation? ${cost} on Haiku · drops context to ~5-10% · next turn carries the summary forward.`)) return;
          void assistant.compactConversation();
        }}
        title="Summarize + remint the CLI session. Drops working context but preserves the summary on the next turn."
      >
        <Layers size={11} />
        <span>Compact</span>
      </button>
    {/if}

    {#if taskCount > 0}
      <button
        class="dock-toggle"
        class:open={tasksOpen}
        class:pulse
        type="button"
        onclick={toggleTasks}
        title="Tasks panel"
      >
        <ListChecks size={13} />
        <span class="task-chip">{taskDone}/{taskCount}</span>
      </button>
    {/if}
  </div>
</div>

<style>
  .ah-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 14px;
    min-height: 34px;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .ah-left, .ah-right {
    display: flex; align-items: center; gap: 8px;
    min-width: 0;
  }
  .ah-left { flex: 1; min-width: 0; }
  .ah-right { flex-shrink: 0; }

  .auth-warn {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 3px 10px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
  }
  .dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--fg-muted);
  }
  .auth-warn[data-tone="yellow"] {
    color: var(--warn);
    border-color: color-mix(in oklch, var(--warn) 35%, var(--border));
    background: var(--warn-soft);
  }
  .auth-warn[data-tone="yellow"] .dot { background: var(--warn); }
  .auth-warn[data-tone="red"] {
    color: var(--danger);
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    background: color-mix(in oklch, var(--danger) 10%, transparent);
  }
  .auth-warn[data-tone="red"] .dot { background: var(--danger); }

  .shell-lock {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 3px 10px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--warn);
    background: var(--warn-soft);
    border: 1px solid color-mix(in oklch, var(--warn) 35%, var(--border));
  }

  .agents-pill {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 3px 9px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    background: var(--accent-soft);
    color: var(--accent);
    border: 1px solid color-mix(in oklch, var(--accent) 30%, var(--border));
    cursor: help;
    font-variant-numeric: tabular-nums;
  }
  .agents-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    animation: agents-pulse 1.4s ease-in-out infinite;
  }
  @keyframes agents-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .compact-warn {
    display: inline-flex; align-items: center;
    padding: 3px 9px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    background: var(--warn-soft);
    color: var(--warn);
    border: 1px solid color-mix(in oklch, var(--warn) 35%, var(--border));
    cursor: help;
  }

  .ctx-pill {
    display: inline-flex; align-items: center; gap: 7px;
    padding: 3px 9px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    font-weight: 600;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    color: var(--fg-muted);
    cursor: help;
    font-variant-numeric: tabular-nums;
  }
  .ctx-pill .ctx-bar {
    position: relative;
    width: 44px;
    height: 4px;
    background: var(--surface-hover);
    border-radius: 999px;
    overflow: hidden;
  }
  .ctx-pill .ctx-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease-out, background 120ms;
    border-radius: 999px;
  }
  .ctx-pill .ctx-text { color: var(--fg); }
  .ctx-pill .ctx-sep { color: var(--fg-muted); margin: 0 2px; }
  .ctx-pill .ctx-pct { color: var(--fg-muted); font-size: 10px; }
  .ctx-pill[data-tone="yellow"] {
    border-color: color-mix(in oklch, var(--warn) 35%, var(--border));
    background: var(--warn-soft);
    color: var(--warn);
  }
  .ctx-pill[data-tone="yellow"] .ctx-text { color: var(--warn); }
  .ctx-pill[data-tone="yellow"] .ctx-fill { background: var(--warn); }
  .ctx-pill[data-tone="red"] {
    border-color: color-mix(in oklch, var(--danger) 35%, var(--border));
    background: color-mix(in oklch, var(--danger) 10%, transparent);
    color: var(--danger);
  }
  .ctx-pill[data-tone="red"] .ctx-text { color: var(--danger); }

  .compact-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg-muted);
    font-size: 11px;
    cursor: pointer;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .compact-btn:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .compact-btn[data-tone="yellow"] {
    border-color: color-mix(in oklch, var(--warn) 35%, var(--border));
    color: var(--warn);
  }
  .compact-btn[data-tone="red"] {
    border-color: color-mix(in oklch, var(--danger) 45%, var(--border));
    background: color-mix(in oklch, var(--danger) 8%, transparent);
    color: var(--danger);
  }
  .ctx-pill[data-tone="red"] .ctx-fill { background: var(--danger); }

  .hdr-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 5px 7px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-muted);
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .hdr-btn:hover { color: var(--fg); border-color: var(--border-strong); }
  .hdr-btn-label {
    font-size: var(--fs-xs);
  }

  .ws-chip {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 4px 6px 4px 10px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--fg-2);
    font-size: var(--fs-xs);
    max-width: 240px;
  }
  .ws-chip :global(svg) { color: var(--fg-muted); }
  .ws-chip .ws-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ws-x {
    background: transparent;
    border: 0;
    border-radius: 999px;
    padding: 1px;
    color: var(--fg-muted);
    cursor: pointer;
    display: inline-flex;
    opacity: 0.65;
    transition: opacity 120ms, background 120ms, color 120ms;
  }
  .ws-x:hover { opacity: 1; color: var(--fg); background: var(--surface-hover); }

  .dock-toggle {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 5px 9px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-muted);
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .dock-toggle:hover { color: var(--fg); border-color: var(--border-strong); }
  .dock-toggle.open {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  .dock-toggle.pulse { animation: dock-pulse 700ms ease-out; }
  @keyframes dock-pulse {
    0%   { box-shadow: 0 0 0 0 var(--accent-soft); border-color: var(--accent); }
    60%  { box-shadow: 0 0 0 6px transparent; }
    100% { box-shadow: 0 0 0 0 transparent; }
  }
  .task-chip {
    font-size: 10px;
    padding: 1px 5px;
    background: var(--accent);
    color: var(--accent-fg);
    border-radius: 999px;
    font-weight: 600;
  }
  .dock-toggle:not(.open) .task-chip {
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
