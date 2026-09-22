<script lang="ts">
  import { tick } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { Plug, Plus, RefreshCw, Trash2 } from "@lucide/svelte";
  import {
    mcpPanel,
    MCP_SCOPES,
    MCP_TRANSPORTS,
    type McpScope,
    type McpTransport,
  } from "../../state/mcp-panel.svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { mcpHint, statusMeta } from "../../state/assistant/mcpStatus";

  let panelEl: HTMLDivElement | undefined = $state();

  // ── Add / remove (claude mcp add|remove through the backend) ───────────────
  let adding = $state(false);
  let addBusy = $state(false);
  let addError = $state<string | null>(null);
  let fName = $state("");
  let fTransport = $state<McpTransport>("stdio");
  let fTarget = $state("");
  let fArgs = $state("");
  let fScope = $state<McpScope>("user");
  // Name of the row whose inline remove-confirm strip is open.
  let removing = $state<string | null>(null);
  let removeScope = $state<McpScope>("user");
  let removeBusy = $state(false);
  let removeError = $state<string | null>(null);

  const hasRoot = $derived(!!assistant.workspace.current);
  const canAdd = $derived(fName.trim().length > 0 && fTarget.trim().length > 0 && !addBusy);

  function openAdd() {
    adding = true;
    addError = null;
    removing = null;
  }
  function closeAdd() {
    adding = false;
    addError = null;
    fName = ""; fTarget = ""; fArgs = ""; fTransport = "stdio"; fScope = "user";
  }

  async function submitAdd() {
    if (!canAdd) return;
    addBusy = true;
    addError = null;
    try {
      await mcpPanel.add(assistant.workspace.current, assistant.activeTab?.mcpServers ?? null, {
        name: fName.trim(),
        transport: fTransport,
        target: fTarget.trim(),
        // Whitespace-split; quoted args aren't supported — one token per arg.
        args: fTransport === "stdio" ? fArgs.split(/\s+/).map((a) => a.trim()).filter(Boolean) : [],
        scope: fScope,
      });
      closeAdd();
    } catch (e) {
      addError = String(e);
    } finally {
      addBusy = false;
    }
  }

  function openRemove(name: string) {
    removing = name;
    removeScope = "user";
    removeError = null;
    adding = false;
  }

  async function submitRemove() {
    if (!removing || removeBusy) return;
    removeBusy = true;
    removeError = null;
    try {
      await mcpPanel.remove(assistant.workspace.current, assistant.activeTab?.mcpServers ?? null, removing, removeScope);
      removing = null;
    } catch (e) {
      removeError = String(e);
    } finally {
      removeBusy = false;
    }
  }

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
      // Esc peels one layer: open form/confirm first, then the dialog.
      if (adding) closeAdd();
      else if (removing) removing = null;
      else mcpPanel.hide();
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
        <button type="button" class="mcp-recheck" onclick={adding ? closeAdd : openAdd} aria-expanded={adding} aria-controls="mcp-add-form">
          <span class="mcp-recheck-icon"><Plus size={12} /></span>
          {adding ? "Cancel" : "Add"}
        </button>
        <kbd class="mcp-kbd">Esc</kbd>
      </header>

      {#if adding}
        <form class="mcp-add" id="mcp-add-form" onsubmit={(e) => { e.preventDefault(); void submitAdd(); }}>
          <div class="mcp-add-grid">
            <label class="mcp-fld">
              <span>Name</span>
              <!-- svelte-ignore a11y_autofocus -->
              <input class="mcp-in mono" type="text" bind:value={fName} placeholder="my-server" spellcheck="false" autocomplete="off" autofocus />
            </label>
            <label class="mcp-fld">
              <span>Transport</span>
              <select class="mcp-in" bind:value={fTransport}>
                {#each MCP_TRANSPORTS as t (t)}<option value={t}>{t}</option>{/each}
              </select>
            </label>
            <label class="mcp-fld">
              <span>Scope</span>
              <select class="mcp-in" bind:value={fScope}>
                {#each MCP_SCOPES as s (s)}
                  <option value={s} disabled={s === "project" && !hasRoot}>{s}{s === "project" && !hasRoot ? " (open a folder)" : ""}</option>
                {/each}
              </select>
            </label>
            <label class="mcp-fld mcp-fld-wide">
              <span>{fTransport === "stdio" ? "Command" : "URL"}</span>
              <input class="mcp-in mono" type="text" bind:value={fTarget}
                placeholder={fTransport === "stdio" ? "npx" : "https://example.com/mcp"} spellcheck="false" autocomplete="off" />
            </label>
            {#if fTransport === "stdio"}
              <label class="mcp-fld mcp-fld-wide">
                <span>Arguments <em>space-separated</em></span>
                <input class="mcp-in mono" type="text" bind:value={fArgs} placeholder="-y @scope/server --port 3000" spellcheck="false" autocomplete="off" />
              </label>
            {/if}
          </div>
          {#if addError}<div class="mcp-add-err">{addError}</div>{/if}
          <div class="mcp-add-foot">
            <span class="mcp-add-hint">Runs <code>claude mcp add</code> — same config a terminal writes.</span>
            <button type="submit" class="mcp-btn primary" disabled={!canAdd}>{addBusy ? "Adding…" : "Add server"}</button>
          </div>
        </form>
      {/if}

      <div class="mcp-list" role="list" aria-label="MCP servers">
        {#if mcpPanel.rows == null && mcpPanel.loading}
          {#each [0, 1, 2] as i (i)}
            <div class="mcp-skel" style:animation-delay={`${i * 120}ms`}></div>
          {/each}
        {:else if rows.length === 0}
          <div class="mcp-empty">
            <span class="mcp-empty-t">No MCP servers configured</span>
            <span class="mcp-empty-s">
              Use <strong>Add</strong> above, or <code>claude mcp add</code> in a terminal (or a project
              <code>.mcp.json</code>) — Rift picks it up automatically.
            </span>
          </div>
        {:else}
          {#each rows as r (r.name)}
            {@const meta = statusMeta(r.status)}
            <div class="mcp-row" role="listitem" class:confirming={removing === r.name}>
              <span class="mcp-dot {meta.tint}" aria-hidden="true"></span>
              <span class="mcp-name">{r.name}</span>
              {#if r.live}
                <span class="mcp-live" title="Status reported live by this chat's session">this chat</span>
              {/if}
              <span class="mcp-target" title={r.target ?? undefined}>{r.target ?? r.detail ?? ""}</span>
              <span class="mcp-status {meta.tint}" title={r.detail ?? undefined}>{meta.label}</span>
              <button type="button" class="mcp-rm" title="Remove {r.name}" aria-label="Remove {r.name}"
                onclick={() => (removing === r.name ? (removing = null) : openRemove(r.name))}>
                <Trash2 size={13} />
              </button>
            </div>
            {#if removing === r.name}
              <div class="mcp-confirm">
                <span>Remove <strong>{r.name}</strong> from</span>
                <select class="mcp-in sm" bind:value={removeScope} aria-label="Scope to remove from">
                  {#each MCP_SCOPES as s (s)}<option value={s}>{s}</option>{/each}
                </select>
                <span>scope?</span>
                <button type="button" class="mcp-btn danger" disabled={removeBusy} onclick={() => void submitRemove()}>{removeBusy ? "Removing…" : "Remove"}</button>
                <button type="button" class="mcp-btn" disabled={removeBusy} onclick={() => (removing = null)}>Cancel</button>
                {#if removeError}<span class="mcp-add-err">{removeError}</span>{/if}
              </div>
            {/if}
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
    /* Opaque dim, no backdrop-filter (WebView2 fixed-overlay ban, app.css). */
    background: color-mix(in oklch, var(--bg) 40%, rgba(0, 0, 0, 0.6));
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
  .mcp-row:hover,
  .mcp-row.confirming { background: color-mix(in oklab, var(--fg) 4%, transparent); }

  .mcp-rm {
    display: inline-flex; align-items: center; justify-content: center;
    width: 24px; height: 24px;
    margin-left: 2px;
    flex-shrink: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--fg-faint);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast);
  }
  .mcp-row:hover .mcp-rm,
  .mcp-row:focus-within .mcp-rm,
  .mcp-row.confirming .mcp-rm { opacity: 1; }
  .mcp-rm:hover { color: var(--danger); border-color: color-mix(in oklab, var(--danger) 40%, transparent); }

  .mcp-confirm {
    display: flex; align-items: center; flex-wrap: wrap; gap: 8px;
    margin: 0 6px 6px;
    padding: 8px 10px;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    background: color-mix(in oklab, var(--danger) 7%, transparent);
    border: 1px solid color-mix(in oklab, var(--danger) 30%, transparent);
    border-radius: 10px;
  }
  .mcp-confirm strong { color: var(--fg); font-weight: 600; }

  .mcp-add {
    padding: 10px 14px 12px;
    border-bottom: 1px solid var(--border);
    background: color-mix(in oklab, var(--accent) 4%, transparent);
  }
  .mcp-add-grid {
    display: grid;
    grid-template-columns: 1.4fr 0.8fr 0.8fr;
    gap: 8px 10px;
  }
  .mcp-fld { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .mcp-fld > span { font-size: var(--fs-xs); font-weight: 560; color: var(--fg-muted); }
  .mcp-fld > span em { font-style: normal; font-weight: 400; color: var(--fg-faint); margin-left: 4px; }
  .mcp-fld-wide { grid-column: 1 / -1; }
  .mcp-in {
    height: 28px;
    padding: 0 9px;
    font: inherit; font-size: var(--fs-xs);
    color: var(--fg);
    background: var(--bg-elev-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    outline: none;
    min-width: 0;
  }
  .mcp-in.mono { font-family: var(--font-mono, ui-monospace, monospace); }
  .mcp-in.sm { height: 24px; padding: 0 6px; }
  .mcp-in:focus { border-color: var(--accent); }
  .mcp-add-err {
    margin-top: 8px;
    font-size: var(--fs-xs);
    color: var(--danger);
    word-break: break-word;
  }
  .mcp-add-foot { display: flex; align-items: center; gap: 10px; margin-top: 10px; }
  .mcp-add-hint { flex: 1; min-width: 0; font-size: var(--fs-xs); color: var(--fg-faint); }
  .mcp-add-hint code {
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    padding: 1px 5px;
    border-radius: 5px;
  }
  .mcp-btn {
    height: 26px; padding: 0 11px;
    font: inherit; font-size: var(--fs-xs); font-weight: 560;
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    cursor: pointer;
    transition: color var(--dur-fast), border-color var(--dur-fast), background var(--dur-fast);
  }
  .mcp-btn:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); }
  .mcp-btn:disabled { opacity: 0.55; cursor: default; }
  .mcp-btn.primary { color: var(--accent-fg, var(--bg)); background: var(--accent); border-color: var(--accent); }
  .mcp-btn.primary:hover:not(:disabled) { color: var(--accent-fg, var(--bg)); filter: brightness(1.08); }
  .mcp-btn.danger { color: var(--danger); border-color: color-mix(in oklab, var(--danger) 40%, transparent); }
  .mcp-btn.danger:hover:not(:disabled) { color: var(--danger); background: color-mix(in oklab, var(--danger) 12%, transparent); }

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
