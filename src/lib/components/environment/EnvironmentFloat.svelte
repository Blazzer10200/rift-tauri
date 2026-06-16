<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { GitBranch, X } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { git } from "../../state/git.svelte";
  import { environmentDock } from "../../state/environmentDock.svelte";
  import EnvironmentPanel from "./EnvironmentPanel.svelte";

  // Canonical git refresher (mounted whenever the box is open, so the collapsed
  // pill has counts too). EnvironmentPanel just renders from git state.
  const root = $derived(assistant.activeTab?.workspaceRoot ?? null);
  onMount(() => { void git.refresh(root); });
  $effect(() => {
    const r = root;
    void git.refresh(r);
  });

  const status = $derived(git.status);
  const dirty = $derived((status?.total_adds ?? 0) > 0 || (status?.total_dels ?? 0) > 0);
</script>

{#if environmentDock.open && status}
  {#if environmentDock.expanded}
    <!-- Expanded: a real in-flow column. Reserves width at the workbench level so
         the chat shrinks beside it instead of being covered (no overlap). -->
    <div class="env-panel">
      <EnvironmentPanel />
    </div>
  {:else}
    <!-- Collapsed: a floating pill over the chat's top-right (no reserved space). -->
    <div class="env-pill" transition:fly={{ y: -6, duration: 140 }}>
      <button
        class="pill-main"
        type="button"
        title="Open source control"
        onclick={() => environmentDock.setExpanded(true)}
      >
        <GitBranch size={13} class="pill-ic" />
        <span class="pill-branch">{status.branch}</span>
        {#if dirty}
          <span class="pill-stat">
            <span class="adds">+{status.total_adds}</span>
            <span class="dels">−{status.total_dels}</span>
          </span>
        {:else}
          <span class="pill-clean">clean</span>
        {/if}
      </button>
      <button
        class="pill-x"
        type="button"
        title="Dismiss"
        aria-label="Dismiss environment"
        onclick={() => environmentDock.close()}
      >
        <X size={13} />
      </button>
    </div>
  {/if}
{/if}

<style>
  /* ── Expanded in-flow panel ─────────────────────────────────────────────── */
  .env-panel {
    flex: 0 0 400px;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--bg);
    overflow: hidden;
  }

  /* ── Collapsed floating pill ────────────────────────────────────────────── */
  .env-pill {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 30;
    display: inline-flex;
    align-items: center;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 16px -6px rgb(0 0 0 / 0.4);
    overflow: hidden;
    transition: border-color 120ms ease;
  }
  .env-pill:hover { border-color: color-mix(in oklab, var(--accent) 35%, var(--border)); }
  .pill-main {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 9px;
    border: 0;
    background: none;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-xs);
    cursor: pointer;
    max-width: 240px;
  }
  .pill-main:hover { background: color-mix(in oklab, var(--accent) 10%, transparent); }
  .pill-main :global(.pill-ic) { color: var(--fg-3); flex: none; }
  .pill-branch {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pill-stat { display: inline-flex; gap: 6px; font-variant-numeric: tabular-nums; flex: none; }
  .pill-clean { color: var(--fg-3); flex: none; }
  .adds { color: var(--ok); }
  .dels { color: var(--danger); }
  .pill-x {
    display: grid;
    place-items: center;
    width: 26px;
    align-self: stretch;
    border: 0;
    border-left: 1px solid var(--border);
    background: none;
    color: var(--fg-3);
    cursor: pointer;
  }
  .pill-x:hover { color: var(--fg); background: color-mix(in oklab, var(--accent) 10%, transparent); }
</style>
