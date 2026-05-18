<script lang="ts">
  import { Sparkles, ListChecks, Plus, FolderOpen, Folder, X, TerminalSquare } from "lucide-svelte";
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

  // Auth status surfaces here ONLY when degraded — green/healthy state is
  // implicit (no clutter). The model name lives in the composer pill, not
  // up here: showing both was redundant and made the header feel busy.
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
      // Pulse only when TasksDock is closed — open surface already shows the
      // change. v0.4.1 collapses both paths to assistant.ui.dockOpen since
      // TasksDock now lives inside AssistantPage in both shells.
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

  /** Context-window cap for the model id reported by `system`/init.
   *  1M-context variants carry the `[1m]` suffix on the model name; everything
   *  else (sonnet/opus/haiku 4.x) is 200K by default. */
  function contextWindowFor(model: string | null): number {
    if (!model) return 200_000;
    if (/\[1m\]/i.test(model)) return 1_000_000;
    return 200_000;
  }

  function shortK(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 10_000) return `${Math.round(n / 1000)}K`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return String(n);
  }

  // Effective context the model is processing = uncached new input +
  // cache-create + cache-read. Cache hits don't shrink what the model "sees"
  // — they're billing optimization. Sum is what fills the window.
  const ctxTokens = $derived(
    assistant.lastTurnUsage
      ? assistant.lastTurnUsage.input + assistant.lastTurnUsage.cacheRead + assistant.lastTurnUsage.cacheCreate
      : 0,
  );
  const ctxWindow = $derived(contextWindowFor(assistant.lastModelId));
  const ctxPct = $derived(ctxWindow > 0 ? Math.min(100, (ctxTokens / ctxWindow) * 100) : 0);
  const ctxTone = $derived(ctxPct >= 90 ? "red" : ctxPct >= 70 ? "yellow" : "ok");
  const ctxTitle = $derived.by(() => {
    const u = assistant.lastTurnUsage;
    if (!u) return "Context — send a message to populate";
    const s = assistant.sessionUsage;
    const cost =
      assistant.totalCostUsd !== null && assistant.totalCostUsd !== undefined
        ? ` · $${assistant.totalCostUsd.toFixed(4)}`
        : "";
    return (
      `Context this turn: ${ctxTokens.toLocaleString()} / ${ctxWindow.toLocaleString()} (${ctxPct.toFixed(1)}%)\n` +
      `  • new input: ${u.input.toLocaleString()}\n` +
      `  • cache read: ${u.cacheRead.toLocaleString()}\n` +
      `  • cache create: ${u.cacheCreate.toLocaleString()}\n` +
      `  • output: ${u.output.toLocaleString()}\n` +
      `Session totals (${s.turns} turn${s.turns === 1 ? "" : "s"}):\n` +
      `  • input: ${s.totalInput.toLocaleString()} · output: ${s.totalOutput.toLocaleString()}\n` +
      `  • cache read: ${s.totalCacheRead.toLocaleString()} · cache create: ${s.totalCacheCreate.toLocaleString()}${cost}\n` +
      `Model: ${assistant.lastModelId ?? "?"}`
    );
  });
</script>

<header class="head">
  <div class="left">
    <div class="brand">
      <span class="brand-icon"><Sparkles size={14} /></span>
      <span class="brand-name">Assistant</span>
      <span class="beta" title="Beta — use at your own risk. Tool capabilities are evolving.">beta</span>
    </div>
  </div>

  <div class="right">
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

    {#if ctxTokens > 0}
      <span class="ctx-pill" data-tone={ctxTone} title={ctxTitle}>
        <span class="ctx-bar"><span class="ctx-fill" style="width: {ctxPct}%"></span></span>
        <span class="ctx-text">{shortK(ctxTokens)}<span class="ctx-sep">/</span>{shortK(ctxWindow)}</span>
        <span class="ctx-pct">{Math.round(ctxPct)}%</span>
      </span>
    {/if}

    <button
      class="hdr-btn"
      type="button"
      title="New conversation"
      onclick={() => void assistant.newTab()}
    >
      <Plus size={13} />
    </button>

    <button
      class="dock-toggle"
      class:open={tasksOpen}
      class:pulse
      type="button"
      onclick={toggleTasks}
      title="Tasks panel"
    >
      <ListChecks size={13} />
      {#if taskCount > 0}
        <span class="task-chip">{taskDone}/{taskCount}</span>
      {/if}
    </button>

  </div>
</header>

<style>
  .head {
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    padding: 9px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex-shrink: 0;
  }
  .left, .right { display: flex; align-items: center; gap: 8px; }
  .brand {
    display: flex; align-items: center; gap: 7px;
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--fg);
  }
  .brand-icon {
    display: flex; align-items: center; justify-content: center;
    color: var(--accent);
  }
  .beta {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    background: var(--warn-soft);
    color: var(--warn);
    border-radius: 4px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: help;
  }

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
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklch, var(--accent) 35%, var(--border));
    border-radius: 999px;
    color: var(--accent);
    font-size: var(--fs-xs);
    max-width: 200px;
  }
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
    color: var(--accent);
    cursor: pointer;
    display: inline-flex;
    opacity: 0.65;
    transition: opacity 120ms, background 120ms;
  }
  .ws-x:hover { opacity: 1; background: color-mix(in oklch, var(--accent) 18%, transparent); }

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
