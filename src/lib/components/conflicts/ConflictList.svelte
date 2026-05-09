<script lang="ts">
  import { AlertTriangle } from "lucide-svelte";
  import { connection, type ConflictRecord } from "../../state/connection.svelte";

  type Props = {
    selected: ConflictRecord | null;
    onSelect: (c: ConflictRecord) => void;
  };
  let { selected, onSelect }: Props = $props();

  function basename(p: string): string {
    const norm = p.replaceAll("\\", "/").replace(/\/+$/, "");
    const i = norm.lastIndexOf("/");
    return i === -1 ? norm : norm.slice(i + 1);
  }
</script>

<aside class="list">
  <header>
    <h3>Conflicts</h3>
    <span class="count-pip" class:danger={connection.conflicts.length > 0}>
      {connection.conflicts.length}
    </span>
  </header>
  {#if connection.conflicts.length === 0}
    <p class="empty">No conflicts.</p>
  {:else}
    <div class="rows">
      {#each connection.conflicts as c (c.local_path)}
        <button
          type="button"
          class="row"
          data-active={selected?.local_path === c.local_path}
          onclick={() => onSelect(c)}
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
    padding: 16px; color: var(--fg-muted);
    font-size: var(--fs-sm); text-align: center;
  }
  .rows { overflow: auto; flex: 1; padding: 4px; }
  .row {
    display: flex; flex-direction: column; gap: 2px;
    width: 100%;
    background: transparent; border: 0;
    color: var(--fg);
    padding: 8px 12px;
    margin-bottom: 2px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    font: inherit;
    border-left: 2px solid transparent;
  }
  .row:hover { background: var(--surface-hover); }
  .row[data-active="true"] {
    background: var(--danger-soft);
    border-left-color: var(--danger);
  }
  .row-head { display: inline-flex; align-items: center; gap: 6px; }
  .row-head :global(.ico) { color: var(--danger); }
  .file { color: var(--fg); font-size: var(--fs-sm); }
  .resource { color: var(--accent); font-size: var(--fs-xs); }
  .path {
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .emptyhint {
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    margin: 0;
  }
</style>
