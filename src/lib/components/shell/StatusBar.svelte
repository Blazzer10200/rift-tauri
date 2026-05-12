<script lang="ts">
  import { connection } from "../../state/connection.svelte";
  import { RefreshCw, Lock } from "lucide-svelte";
  import { fade } from "svelte/transition";

  const stateText = $derived(connection.status?.state ?? "offline");
  const ledClass = $derived(
    stateText === "watching" || stateText === "idle" ? "ok" :
    stateText === "syncing" ? "info" :
    stateText === "error" ? "danger" : "muted"
  );

  const watcherOn = $derived(stateText === "watching" || stateText === "idle" || stateText === "syncing");
  const queue = $derived(connection.status?.pending ?? 0);
  const failed = $derived(connection.status?.failed ?? 0);
  const locks = $derived(connection.lockCount);
  const conflicts = $derived(connection.conflictCount);

  async function toggleWatcher() {
    if (!connection.selected || connection.connecting) return;
    if (watcherOn) {
      await connection.disconnect();
    } else {
      try { await connection.connect(); } catch (e) { console.error(e); }
    }
  }
</script>

<div class="statusbar">
  <div class="grp">
    <span class="led" data-state={ledClass}></span>
    <span class="lbl">{stateText}</span>
  </div>
  <div class="sep"></div>
  <button
    class="grp watcher"
    type="button"
    onclick={toggleWatcher}
    disabled={!connection.selected || connection.connecting}
    title={connection.connecting ? "Connecting…" : watcherOn ? "Click to stop watching" : "Click to start watching"}
  >
    <RefreshCw size={11}/>
    <span class="lbl">watcher</span>
    <span class="mono val" class:ok={watcherOn}>{connection.connecting ? "…" : watcherOn ? "on" : "off"}</span>
  </button>
  {#if locks > 0}
    <div class="sep"></div>
    <div class="grp" transition:fade={{ duration: 100 }}>
      <Lock size={11}/>
      <span class="lbl">locks</span>
      <span class="mono val warn">{locks}</span>
    </div>
  {/if}
  {#if queue > 0}
    <div class="sep"></div>
    <div class="grp" transition:fade={{ duration: 100 }}>
      <span class="lbl">queued</span>
      <span class="mono val warn">{queue}</span>
    </div>
  {/if}
  {#if failed > 0}
    <div class="sep"></div>
    <div class="grp" transition:fade={{ duration: 100 }}>
      <span class="lbl">errors</span>
      <span class="mono val danger">{failed}</span>
    </div>
  {/if}

  <div class="flex-spacer"></div>

  {#if conflicts > 0}
    <div class="grp" transition:fade={{ duration: 100 }}>
      <span class="lbl">conflicts</span>
      <span class="mono val danger">{conflicts}</span>
    </div>
    <div class="sep"></div>
  {/if}
  <div class="grp" title="Press Ctrl+K">
    <span class="kbd">⌘</span><span class="kbd">K</span>
  </div>
</div>

<style>
  .statusbar {
    display: flex; align-items: center; gap: 10px;
    height: 22px; padding: 0 10px;
    background: var(--bg);
    border-top: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    user-select: none;
  }
  .grp { display: flex; align-items: center; gap: 6px; }
  .lbl { color: var(--fg-subtle); }
  .val { color: var(--fg-2); }
  .val.ok { color: var(--ok); }
  .val.warn { color: var(--warn); }
  .val.danger { color: var(--danger); }
  .sep { width: 1px; height: 12px; background: var(--border); }
  .flex-spacer { flex: 1; }
  .led {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--fg-faint);
    box-shadow: 0 0 0 2px color-mix(in oklch, currentColor 12%, transparent);
    transition: background 120ms ease;
  }
  .led[data-state="ok"]     { background: var(--ok); }
  .led[data-state="info"]   { background: var(--info); }
  .led[data-state="danger"] { background: var(--danger); }
  .led[data-state="muted"]  { background: var(--fg-faint); }

  .watcher {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    cursor: pointer;
    padding: 0 4px; margin: 0 -4px;
    border-radius: var(--radius-xs);
    transition: background 100ms ease;
  }
  .watcher:hover:not(:disabled) { background: var(--surface-hover); }
  .watcher:disabled { cursor: not-allowed; opacity: 0.6; }
  .watcher:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
</style>
