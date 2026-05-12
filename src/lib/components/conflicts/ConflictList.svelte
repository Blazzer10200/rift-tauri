<script lang="ts">
  import { AlertTriangle } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ConflictRecord } from "../../state/connection.svelte";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let mounted = $state(false);
  $effect(() => {
    mounted = true;
  });

  function rowIn(i: number) {
    if (reducedMotion) return { duration: 0 };
    return {
      x: -14,
      opacity: 0,
      duration: 260,
      delay: mounted ? 0 : Math.min(i, 7) * 40,
    };
  }

  type Props = {
    selected: ConflictRecord | null;
    onSelect: (c: ConflictRecord) => void;
  };
  let { selected, onSelect }: Props = $props();

  let busy = $state(false);
  let bulkError = $state<string | null>(null);

  function basename(p: string): string {
    const norm = p.replaceAll("\\", "/").replace(/\/+$/, "");
    const i = norm.lastIndexOf("/");
    return i === -1 ? norm : norm.slice(i + 1);
  }

  async function bulkResolve(resolution: "force_local" | "accept_remote") {
    const n = connection.conflicts.length;
    if (n === 0) return;
    const label = resolution === "force_local" ? "Use Local" : "Use Remote";
    if (!window.confirm(`Apply "${label}" to all ${n} conflicts? This cannot be undone.`)) return;
    busy = true; bulkError = null;
    const paths = connection.conflicts.map((c) => c.local_path);
    try {
      const res = await invoke<boolean[]>("resolve_conflicts_bulk", {
        localPaths: paths,
        resolution,
      });
      const ok = res.filter(Boolean).length;
      if (ok < paths.length) {
        bulkError = `Resolved ${ok}/${paths.length} — check Activity tab for failures.`;
      }
    } catch (e) {
      bulkError = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<aside class="list">
  <header>
    <h3>Conflicts</h3>
    {#if connection.conflicts.length > 0}
      <span class="count-pip danger">{connection.conflicts.length}</span>
    {/if}
  </header>
  {#if connection.conflicts.length === 0}
    <div class="empty">
      <span class="empty-title">No conflicts</span>
      <span class="empty-hint">Both sides agree — anything that diverges lands here.</span>
    </div>
  {:else}
    <div class="bulk-bar">
      <span class="bulk-label">Resolve all <span class="bulk-count">({connection.conflicts.length})</span></span>
      <div class="bulk-actions">
        <button type="button" class="btn ghost sm warn" disabled={busy} onclick={() => bulkResolve("accept_remote")}>
          Take Remote
        </button>
        <button type="button" class="btn ghost sm info" disabled={busy} onclick={() => bulkResolve("force_local")}>
          Take Local
        </button>
      </div>
    </div>
    {#if bulkError}
      <p class="bulk-error">{bulkError}</p>
    {/if}
    <div class="rows">
      {#each connection.conflicts as c, i (c.local_path)}
        <button
          type="button"
          class="row"
          data-active={selected?.local_path === c.local_path}
          onclick={() => onSelect(c)}
          in:fly={rowIn(i)}
          animate:flip={{ duration: reducedMotion ? 0 : 240 }}
        >
          <div class="row-head">
            <AlertTriangle size={11} class="ico"/>
            <span class="file mono">{basename(c.local_path)}</span>
          </div>
          <span class="resource mono">{c.resource_name}</span>
          <span class="path mono dim" title={c.local_path}>{c.local_path}</span>
        </button>
      {/each}
    </div>
  {/if}
  <p class="emptyhint help">New conflicts appear here in real time.</p>
</aside>

<style>
  .list {
    width: 320px; flex-shrink: 0;
    background: var(--bg);
    color: var(--fg);
    border-right: 1px solid var(--border);
    display: flex; flex-direction: column;
    min-height: 0;
  }
  header {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev-1);
  }
  header h3 { margin: 0; font-size: var(--fs-sm); font-weight: 600; }
  .empty {
    padding: 28px 16px;
    display: flex; flex-direction: column; gap: 4px;
    align-items: center; text-align: center;
  }
  .empty-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .empty-hint { color: var(--fg-muted); font-size: var(--fs-xs); max-width: 240px; line-height: 1.45; }
  .bulk-bar {
    display: flex; flex-direction: column; gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev-1);
  }
  .bulk-label {
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .bulk-count { color: var(--fg-faint); }
  .bulk-actions { display: flex; gap: 6px; }
  .bulk-actions .btn { flex: 1; }
  .bulk-error {
    margin: 0; padding: 6px 14px;
    background: var(--danger-soft);
    color: var(--danger);
    font-size: var(--fs-xs);
  }
  .rows { overflow: auto; flex: 1; padding: 6px 4px; display: flex; flex-direction: column; gap: 2px; }
  .row {
    display: flex; flex-direction: column; gap: 2px;
    width: 100%;
    background: transparent; border: 0;
    color: var(--fg);
    padding: 9px 12px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    font: inherit;
    box-shadow: inset 2px 0 transparent;
    transition: background 100ms, box-shadow 100ms;
  }
  .row:hover { background: var(--surface-hover); }
  .row[data-active="true"] {
    background: color-mix(in oklch, var(--danger) 10%, var(--surface));
    box-shadow: inset 2px 0 var(--danger);
  }
  .row[data-active="true"]:hover {
    background: color-mix(in oklch, var(--danger) 14%, var(--surface));
  }
  .row-head { display: inline-flex; align-items: center; gap: 6px; }
  .row-head :global(.ico) {
    color: var(--danger);
    transition: transform 120ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @media (prefers-reduced-motion: no-preference) {
    .row:hover .row-head :global(.ico) { transform: scale(1.15); }
  }
  .file { color: var(--fg); font-size: var(--fs-sm); font-weight: 500; }
  .resource { color: var(--accent); font-size: var(--fs-xs); }
  .path {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .emptyhint {
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    margin: 0;
  }
</style>
