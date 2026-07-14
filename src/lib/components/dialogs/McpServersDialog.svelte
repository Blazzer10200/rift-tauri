<script lang="ts">
  import { tick } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { Plug, RefreshCw } from "lucide-svelte";
  import { mcpPanel } from "../../state/mcp-panel.svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { mcpHint, statusMeta } from "../../state/assistant/mcpStatus";

  let panelEl: HTMLDivElement | undefined = $state();

  // Pull focus into the panel on open — the composer textarea otherwise keeps
  // focus and its own Escape handling eats the close key before it bubbles.
  $effect(() => {
    if (mcpPanel.open) {
      void (async () => {
        await tick();
        panelEl?.focus();
      })();
    }
  });

  const rows = $derived(mcpPanel.rows ?? []);
  const hint = $derived(mcpPanel.rows ? mcpHint(mcpPanel.rows) : null);
  const allOk = $derived(rows.length > 0 && rows.every((r) => r.status === "connected"));

  function recheck() {
    void mcpPanel.refresh(
      assistant.workspace.current,
      assistant.activeTab?.mcpServers ?? null,
    );
  }

  function onKeydown(e: KeyboardEvent) {
    if (!mcpPanel.open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      mcpPanel.hide();
    }
  }

  function backdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) mcpPanel.hide();
  }
</script>

<!-- Capture phase: fires before the focused composer's own key handlers. -->
<svelte:window onkeydowncapture={onKeydown} />

{#if mcpPanel.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="mcp-scrim" role="presentation" onclick={backdropClick} transition:fade={{ duration: 130 }}>
    <div
      class="mcp-panel"
      role="dialog"
      aria-modal="true"
      aria-label="MCP servers"
      tabindex="-1"
      bind:this={panelEl}
      transition:scale={{ start: 0.97, duration: 170, easing: quintOut }}
    >
      <header class="mcp-head">
        <span class="mcp-head-icon"><Plug size={15} /></span>
        <div class="mcp-head-t">
          <span class="mcp-title">MCP servers</span>
          <span class="mcp-sub">from your Claude Code setup</span>
        </div>
        <button
          type="button"
          class="mcp-recheck"
          onclick={recheck}
          disabled={mcpPanel.loading}
          title={mcpPanel.checkedAt
            ? `Last checked ${new Date(mcpPanel.checkedAt).toLocaleTimeString()}`
            : "Check now"}
        >
          <span class="mcp-recheck-icon" class:spin={mcpPanel.loading}><RefreshCw size={12} /></span>
          {mcpPanel.loading ? "Checking…" : "Re-check"}
        </button>
        <kbd class="mcp-kbd">Esc</kbd>
      </header>

      <div class="mcp-list" role="list" aria-label="MCP servers">
        {#if mcpPanel.rows == null && mcpPanel.loading}
          {#each [0, 1, 2] as i (i)}
            <div class="mcp-skel" style:animation-delay={`${i * 120}ms`}></div>
          {/each}
        {:else if rows.length === 0}
          <div class="mcp-empty">
            <span class="mcp-empty-t">No MCP servers configured</span>
            <span class="mcp-empty-s">
              Add one with <code>claude mcp add</code> in a terminal (or a project
              <code>.mcp.json</code>) — Rift picks it up automatically.
            </span>
          </div>
        {:else}
          {#each rows as r (r.name)}
            {@const meta = statusMeta(r.status)}
            <div class="mcp-row" role="listitem">
              <span class="mcp-dot {meta.tint}" aria-hidden="true"></span>
              <span class="mcp-name">{r.name}</span>
              {#if r.live}
                <span class="mcp-live" title="Status reported live by this chat's session">this chat</span>
              {/if}
              <span class="mcp-target" title={r.target ?? undefined}>{r.target ?? r.detail ?? ""}</span>
              <span class="mcp-status {meta.tint}" title={r.detail ?? undefined}>{meta.label}</span>
            </div>
          {/each}
        {/if}
      </div>

      {#if mcpPanel.error || hint || allOk}
        <footer class="mcp-foot" class:danger={!!mcpPanel.error}>
          {#if mcpPanel.error}
            <span>Live check failed: {mcpPanel.error}</span>
          {:else if hint}
            <span>{hint}</span>
          {:else}
            <span>All servers healthy. Statuses refresh at the start of each turn.</span>
          {/if}
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .mcp-scrim {
    position: fixed; inset: 0;
    z-index: 200;
    background: color-mix(in oklch, var(--bg) 55%, transparent);
    backdrop-filter: blur(8px) saturate(135%);
    -webkit-backdrop-filter: blur(8px) saturate(135%);
    display: flex; justify-content: center; align-items: center;
  }
  .mcp-panel {
    width: min(560px, calc(100vw - 32px));
    max-height: 64vh;
    margin-bottom: 5vh;
    display: flex; flex-direction: column;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-2xl);
    box-shadow:
      0 24px 64px -16px rgba(0, 0, 0, 0.6),
      0 4px 16px -8px rgba(0, 0, 0, 0.45),
      0 0 80px -32px color-mix(in oklab, var(--accent) 30%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
    overflow: hidden;
  }
  .mcp-panel:focus { outline: none; }

  .mcp-head {
    display: flex; align-items: center; gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }
  .mcp-head-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 27px; height: 27px;
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border-radius: 8px;
    flex-shrink: 0;
  }
  .mcp-head-t { display: flex; flex-direction: column; min-width: 0; margin-right: auto; }
  .mcp-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 620; letter-spacing: -0.01em; }
  .mcp-sub { color: var(--fg-faint); font-size: var(--fs-xs); }

  .mcp-recheck {
    display: inline-flex; align-items: center; gap: 6px;
    height: 26px; padding: 0 10px;
    background: var(--bg-elev-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    cursor: pointer;
    transition: color var(--dur-fast), border-color var(--dur-fast);
  }
  .mcp-recheck:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); }
  .mcp-recheck:disabled { opacity: 0.6; cursor: default; }
  .mcp-recheck-icon { display: inline-flex; }
  .mcp-recheck-icon.spin :global(svg) { animation: mcp-spin 900ms linear infinite; }
  @keyframes mcp-spin { to { transform: rotate(360deg); } }

  .mcp-kbd {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 18px; height: 18px;
    padding: 0 5px;
    font: inherit; font-size: 10px;
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: inset 0 -1px 0 var(--border);
  }

  .mcp-list {
    overflow-y: auto;
    padding: 6px;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }

  .mcp-row {
    display: flex; align-items: center; gap: 9px;
    height: 38px;
    padding: 0 10px;
    border-radius: 10px;
  }
  .mcp-row:hover { background: color-mix(in oklab, var(--fg) 4%, transparent); }

  .mcp-dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--fg-faint);
  }
  .mcp-dot.ok {
    background: var(--ok);
    box-shadow: 0 0 6px color-mix(in oklab, var(--ok) 55%, transparent);
  }
  .mcp-dot.warn { background: var(--warn); }
  .mcp-dot.danger {
    background: var(--danger);
    box-shadow: 0 0 6px color-mix(in oklab, var(--danger) 55%, transparent);
  }

  .mcp-name {
    color: var(--fg);
    font-size: var(--fs-sm);
    font-weight: 520;
    letter-spacing: -0.005em;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .mcp-live {
    flex-shrink: 0;
    padding: 1px 6px;
    font-size: 9px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--accent);
    background: var(--accent-soft);
    border-radius: 99px;
  }
  .mcp-target {
    flex: 1; min-width: 0;
    color: var(--fg-faint);
    font-size: var(--fs-xs);
    font-family: var(--font-mono, ui-monospace, monospace);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    text-align: right;
  }
  .mcp-status {
    flex-shrink: 0;
    min-width: 86px;
    text-align: right;
    font-size: var(--fs-xs);
    font-weight: 560;
  }
  .mcp-status.ok { color: var(--ok); }
  .mcp-status.warn { color: var(--warn); }
  .mcp-status.danger { color: var(--danger); }
  .mcp-status.muted { color: var(--fg-muted); }

  .mcp-skel {
    height: 38px;
    margin: 0 0 2px;
    border-radius: 10px;
    background: linear-gradient(
      100deg,
      color-mix(in oklab, var(--fg) 4%, transparent) 40%,
      color-mix(in oklab, var(--fg) 8%, transparent) 50%,
      color-mix(in oklab, var(--fg) 4%, transparent) 60%
    );
    background-size: 200% 100%;
    animation: mcp-shimmer 1.1s ease-in-out infinite;
  }
  @keyframes mcp-shimmer { from { background-position: 120% 0; } to { background-position: -80% 0; } }

  .mcp-empty {
    display: flex; flex-direction: column; align-items: center; gap: 5px;
    padding: 26px 22px;
    text-align: center;
  }
  .mcp-empty-t { color: var(--fg-2); font-size: var(--fs-md); font-weight: 600; }
  .mcp-empty-s { color: var(--fg-faint); font-size: var(--fs-xs); max-width: 40ch; }
  .mcp-empty-s code {
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    padding: 1px 5px;
    border-radius: 5px;
  }

  .mcp-foot {
    padding: 9px 14px;
    border-top: 1px solid var(--border);
    background: color-mix(in oklab, var(--fg) 2%, transparent);
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }
  .mcp-foot.danger { color: var(--danger); }
</style>
