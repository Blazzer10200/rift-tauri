<script lang="ts">
  import { connection, type ServerProfile } from "../state/connection.svelte";

  type Props = {
    open: boolean;
    onClose: () => void;
    onAdd: () => void;
    onEdit: (s: ServerProfile) => void;
    onDelete: (s: ServerProfile) => void;
    onLaunchKeygen: () => void;
  };

  let { open, onClose, onAdd, onEdit, onDelete, onLaunchKeygen }: Props = $props();

  $effect(() => {
    if (open) {
      connection.loadServers().catch((e) => console.error("loadServers", e));
    }
  });

  async function pick(key: string) {
    await connection.select(key);
    onClose();
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function onKey(e: KeyboardEvent) {
    if (open && e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="backdrop" onclick={onBackdrop} role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" aria-label="Pick server">
      <header>
        <h2>Servers</h2>
        <div class="header-actions">
          <button class="aux" onclick={onLaunchKeygen} type="button" title="SSH key setup">Setup key</button>
          <button class="aux primary" onclick={onAdd} type="button">+ Add server</button>
          <button class="close" onclick={onClose} type="button" aria-label="Close">✕</button>
        </div>
      </header>

      {#if connection.servers.length === 0}
        <p class="empty">No servers yet. Click <strong>+ Add server</strong> to create one.</p>
      {:else}
        <ul>
          {#each connection.servers as s (s.key)}
            <li>
              <button
                class="row"
                class:selected={connection.selectedKey === s.key}
                onclick={() => pick(s.key)}
                type="button"
              >
                <div class="row-main">
                  <span class="row-name">{s.name}</span>
                  <span class="row-key">{s.key}</span>
                </div>
                <div class="row-sub">
                  <code>{s.user}@{s.host}:{s.port}</code>
                  <span class="row-root">{s.remoteRoot}</span>
                </div>
              </button>
              <div class="row-actions">
                <button class="ghost" onclick={() => onEdit(s)} type="button" title="Edit">Edit</button>
                <button class="ghost danger" onclick={() => onDelete(s)} type="button" title="Delete">Delete</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
  }
  .dialog {
    background: #17171C;
    border: 1px solid #26262E;
    border-radius: 6px;
    width: 600px; max-width: 92vw;
    max-height: 80vh;
    display: flex; flex-direction: column;
    box-shadow: 0 18px 50px rgba(0,0,0,0.6);
  }
  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 14px; border-bottom: 1px solid #26262E;
    gap: 10px;
  }
  h2 { margin: 0; color: #E8E8EE; font-size: 14px; font-weight: 600; }
  .header-actions { display: flex; align-items: center; gap: 6px; }
  .aux {
    background: #1F1F26;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 5px 10px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  .aux:hover { background: #26262E; }
  .aux.primary { background: #3A2A66; border-color: #3A2A66; color: white; font-weight: 600; }
  .aux.primary:hover { background: #4A3678; }
  .close {
    background: transparent; border: 0; color: #7A7A85;
    font-size: 16px; cursor: pointer; padding: 4px 8px;
  }
  .close:hover { color: #E8E8EE; }
  .empty { color: #7A7A85; padding: 24px; text-align: center; font-size: 13px; }
  .empty strong { color: #E8E8EE; }
  ul { list-style: none; margin: 0; padding: 8px; overflow: auto; }
  li {
    display: flex; align-items: stretch; gap: 6px;
    margin: 0 0 4px;
  }
  .row {
    flex: 1;
    background: transparent;
    border: 1px solid transparent;
    color: #E8E8EE;
    text-align: left;
    padding: 10px 12px;
    border-radius: 4px;
    cursor: pointer;
    display: flex; flex-direction: column; gap: 4px;
  }
  .row:hover { background: #1F1F26; }
  .row.selected { border-color: #3A2A66; background: #15101E; }
  .row-main { display: flex; align-items: baseline; gap: 10px; }
  .row-name { font-weight: 600; font-size: 13px; }
  .row-key { color: #8B6BE6; font-family: Consolas, monospace; font-size: 11px; }
  .row-sub { display: flex; gap: 12px; color: #7A7A85; font-size: 11px; }
  .row-sub code { font-family: Consolas, monospace; }
  .row-root { font-family: Consolas, monospace; }
  .row-actions { display: flex; flex-direction: column; gap: 2px; justify-content: center; }
  .ghost {
    background: transparent;
    border: 1px solid #26262E;
    color: #7A7A85;
    padding: 3px 8px;
    border-radius: 3px;
    font-size: 11px;
    cursor: pointer;
  }
  .ghost:hover { color: #E8E8EE; background: #1F1F26; }
  .ghost.danger:hover { color: #FF5C6B; border-color: #5A1A24; }
</style>
