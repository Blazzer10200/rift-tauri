<script lang="ts">
  import { connection, type SyncPill } from "../state/connection.svelte";

  let { onPickServer, version }: { onPickServer: () => void; version: string } = $props();

  const pillClass: Record<SyncPill, string> = {
    Connected: "pill connected",
    Syncing: "pill syncing",
    Conflict: "pill conflict",
    "Lock-blocked": "pill lock",
    Disconnected: "pill disconnected",
  };

  const isConnected = $derived(connection.pill !== "Disconnected");

  async function toggle() {
    if (isConnected) {
      await connection.disconnect();
    } else {
      // Connecting requires folder spec — surfaced via Browse pane / autosync wiring.
      // Phase 2 just exposes disconnect; connect is invoked from devtools / Phase 3 UI.
    }
  }
</script>

<header class="topbar">
  <div class="brand">
    <span class="dot"></span>
    <span class="name">Rift</span>
    <span class="ver">v{version}</span>
  </div>

  <button class="picker" onclick={onPickServer} type="button">
    <span class="picker-icon"></span>
    {#if connection.selected}
      <span class="picker-name">{connection.selected.name}</span>
      <span class="picker-key">{connection.selected.key}</span>
    {:else}
      <span class="picker-name">Select server</span>
    {/if}
    <span class="chev">▾</span>
  </button>

  <div class="spacer"></div>

  <span class={pillClass[connection.pill]}>{connection.pill}</span>

  <button
    class="conn"
    class:connected={isConnected}
    onclick={toggle}
    type="button"
    disabled={!isConnected}
    title={isConnected ? "Disconnect" : "Connect via Browse tab (Phase 3)"}
  >
    {isConnected ? "Disconnect" : "Connect"}
  </button>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: #0F0F12;
    border-bottom: 1px solid #26262E;
    height: 44px;
    box-sizing: border-box;
  }
  .brand { display: flex; align-items: baseline; gap: 6px; }
  .dot { width: 10px; height: 10px; background: #8B6BE6; border-radius: 2px; display: inline-block; }
  .name { color: #E8E8EE; font-weight: 600; font-size: 15px; }
  .ver { color: #7A7A85; font-size: 11px; }

  .picker {
    background: #17171C;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 6px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    display: flex; align-items: center; gap: 8px;
    height: 30px;
  }
  .picker:hover { border-color: #3D2B6A; }
  .picker-icon { width: 6px; height: 6px; background: #8B6BE6; border-radius: 50%; }
  .picker-name { font-weight: 600; }
  .picker-key { color: #7A7A85; font-family: Consolas, monospace; }
  .chev { color: #7A7A85; font-size: 10px; }

  .spacer { flex: 1; }

  .pill {
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.3px;
    border: 1px solid transparent;
  }
  .pill.connected   { background: #0e2418; color: #4ADE80; border-color: #1d4a30; }
  .pill.syncing     { background: #1a1530; color: #8B6BE6; border-color: #3A2A66; }
  .pill.conflict    { background: #2a1418; color: #FF5C6B; border-color: #4a1d22; }
  .pill.lock        { background: #2a2210; color: #F0B95C; border-color: #4a3a1d; }
  .pill.disconnected{ background: #17171C; color: #7A7A85; border-color: #26262E; }

  .conn {
    background: transparent;
    color: #E8E8EE;
    border: 1px solid #26262E;
    padding: 6px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    height: 30px;
  }
  .conn.connected { background: #8B6BE6; color: white; border-color: #8B6BE6; }
  .conn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
