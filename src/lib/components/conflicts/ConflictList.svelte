<script lang="ts">
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
    <h3>Conflicts ({connection.conflicts.length})</h3>
  </header>
  {#if connection.conflicts.length === 0}
    <p class="empty">No conflicts.</p>
  {:else}
    <ul>
      {#each connection.conflicts as c (c.local_path)}
        <li>
          <button
            type="button"
            class:active={selected?.local_path === c.local_path}
            onclick={() => onSelect(c)}
          >
            <span class="resource">{c.resource_name}</span>
            <span class="file">{basename(c.local_path)}</span>
            <span class="path" title={c.local_path}>{c.local_path}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .list {
    width: 320px; flex-shrink: 0;
    background: #0F0F12; color: #E8E8EE;
    border-right: 1px solid #26262E;
    display: flex; flex-direction: column;
    min-height: 0;
  }
  header {
    padding: 10px 14px;
    border-bottom: 1px solid #26262E;
    background: #17171C;
  }
  h3 { margin: 0; font-size: 13px; }
  .empty { padding: 16px; color: #7A7A85; font-size: 12px; }
  ul { list-style: none; margin: 0; padding: 0; overflow: auto; flex: 1; }
  li { border-bottom: 1px solid #15101E; }
  button {
    display: grid;
    grid-template-columns: 1fr;
    text-align: left;
    width: 100%;
    background: transparent; border: 0;
    color: #E8E8EE;
    padding: 8px 14px;
    cursor: pointer;
    font: inherit;
  }
  button:hover { background: #17171C; }
  button.active { background: #15101E; border-left: 2px solid #FF5C6B; padding-left: 12px; }
  .resource { color: #8B6BE6; font-family: Consolas, monospace; font-size: 11px; }
  .file { color: #E8E8EE; font-size: 13px; font-weight: 600; }
  .path { color: #7A7A85; font-family: Consolas, monospace; font-size: 10px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
