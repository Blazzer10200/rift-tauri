<script lang="ts">
  import { connection } from "../../state/connection.svelte";
  import { RefreshCw, Lock } from "lucide-svelte";

  const stateText = $derived(connection.status?.state ?? "offline");
  const ledClass = $derived(
    stateText === "watching" || stateText === "idle" ? "ok" :
    stateText === "syncing" ? "info" :
    stateText === "error" ? "danger" : "muted"
  );

  const userHost = $derived(
    connection.selected ? `${connection.selected.user}@${connection.selected.host}` : "—"
  );
  const watcherOn = $derived(stateText === "watching" || stateText === "idle" || stateText === "syncing");
  const queue = $derived(connection.status?.pending ?? 0);
  const failed = $derived(connection.status?.failed ?? 0);
</script>

<div class="statusbar">
  <div class="grp">
    <span class="led" data-state={ledClass}></span>
    <span class="lbl">{stateText}</span>
    <span class="mono val">{userHost}</span>
  </div>
  <div class="sep"></div>
  <div class="grp">
    <RefreshCw size={11}/>
    <span class="lbl">watcher</span>
    <span class="mono val" class:ok={watcherOn}>{watcherOn ? "on" : "off"}</span>
  </div>
  <div class="sep"></div>
  <div class="grp">
    <Lock size={11}/>
    <span class="lbl">locks</span>
    <span class="mono val">{connection.lockCount}</span>
  </div>
  <div class="sep"></div>
  <div class="grp">
    <span class="lbl">queue</span>
    <span class="mono val" class:warn={queue > 0}>{queue}</span>
  </div>
  {#if failed > 0}
    <div class="sep"></div>
    <div class="grp">
      <span class="lbl">failed</span>
      <span class="mono val danger">{failed}</span>
    </div>
  {/if}

  <div class="flex-spacer"></div>

  <div class="grp">
    <span class="lbl">conflicts</span>
    <span class="mono val" class:danger={connection.conflictCount > 0}>{connection.conflictCount}</span>
  </div>
  <div class="sep"></div>
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
  }
  .led[data-state="ok"]     { background: var(--ok); }
  .led[data-state="info"]   { background: var(--info); }
  .led[data-state="danger"] { background: var(--danger); }
  .led[data-state="muted"]  { background: var(--fg-faint); }
</style>
