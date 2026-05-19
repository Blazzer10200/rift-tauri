<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { connection } from "../../state/connection.svelte";
  import { syncModal } from "../../state/sync-modal.svelte";
  import { Lock, RefreshCw, Download, Upload, AlertTriangle, Hourglass, Network } from "lucide-svelte";
  import { fade } from "svelte/transition";

  // Background-mode escape hatch: while a sync op is busy but the modal is
  // dismissed, surface a pill in the status bar so the user can re-open and
  // hit Stop. Without this, "Run in background" was a one-way trip.
  const bgSync = $derived(syncModal.busy && !syncModal.open);
  const bgIcon = $derived(
    syncModal.mode === "pull" ? Download :
    syncModal.mode === "push" ? Upload :
    RefreshCw
  );
  const bgLabel = $derived(
    syncModal.mode === "pull" ? "pulling" :
    syncModal.mode === "push" ? "pushing" :
    "scanning"
  );

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

  // Phase 1: bridge slot. The S107 #9.1 `hasBridgeToken` flag indicates the
  // profile holds a bridge token at all; we only surface the pill when the
  // server is also actively connected (otherwise it's misleading).
  const bridgeOn = $derived(!!connection.selected?.hasBridgeToken && watcherOn);

  // Phase 1: last-scan tick. Re-evaluate every 5s so "Xs ago" stays fresh
  // without re-render storms. Reactivity hinges on `now` being $state.
  let now = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => { now = Date.now(); }, 5000);
    return () => clearInterval(id);
  });
  const lastScanLabel = $derived.by(() => {
    const t = connection.lastScanAt;
    if (!t) return null;
    const sec = Math.max(0, Math.floor((now - t) / 1000));
    if (sec < 60) return `${sec}s ago`;
    if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
    return `${Math.floor(sec / 3600)}h ago`;
  });

  // Phase 1: app version pulled once at module init via the existing
  // `app_version` Tauri cmd (lib.rs:362). Renders on the right edge.
  let version = $state<string | null>(null);
  onMount(async () => {
    try { version = await invoke<string>("app_version"); }
    catch (e) { console.error("app_version invoke failed", e); }
  });

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
  <button
    class="grp state-toggle"
    type="button"
    onclick={toggleWatcher}
    disabled={!connection.selected || connection.connecting}
    title={connection.isHandshaking ? "Connecting…" : watcherOn ? "Click to stop watching" : "Click to start watching"}
  >
    <span class="led" data-state={ledClass}></span>
    <span class="lbl">{connection.isHandshaking ? "connecting" : stateText}</span>
  </button>

  {#if bgSync}
    {@const Icon = bgIcon}
    <div class="sep"></div>
    <button
      class="grp sync-pill"
      type="button"
      onclick={() => (syncModal.open = true)}
      title="Sync in progress — click to re-open the modal and Stop"
      transition:fade={{ duration: 100 }}
    >
      <span class="sync-dot"></span>
      <Icon size={11}/>
      <span class="lbl">{bgLabel}…</span>
    </button>
  {/if}

  {#if queue > 0}
    <div class="sep"></div>
    <div class="grp" title="Pending queue depth (debouncing + in-flight uploads)" transition:fade={{ duration: 100 }}>
      <Hourglass size={11}/>
      <span class="lbl">queue</span>
      <span class="mono val">{queue}</span>
    </div>
  {/if}

  {#if failed > 0}
    <div class="sep"></div>
    <div class="grp" title="Failed transfers awaiting retry" transition:fade={{ duration: 100 }}>
      <span class="lbl">failed</span>
      <span class="mono val warn">{failed}</span>
    </div>
  {/if}

  {#if conflicts > 0}
    <div class="sep"></div>
    <div class="grp" title="Unresolved conflicts — open the Conflicts workspace" transition:fade={{ duration: 100 }}>
      <AlertTriangle size={11}/>
      <span class="lbl">conflicts</span>
      <span class="mono val danger">{conflicts}</span>
    </div>
  {/if}

  <div class="flex-spacer"></div>

  {#if lastScanLabel}
    <div class="grp" title="Time since last drift scan / sync completion" transition:fade={{ duration: 100 }}>
      <span class="lbl">last scan</span>
      <span class="mono val">{lastScanLabel}</span>
    </div>
  {/if}

  {#if locks > 0}
    <div class="grp" title="Active lock files held by this client" transition:fade={{ duration: 100 }}>
      <Lock size={11}/>
      <span class="mono val warn">{locks}</span>
    </div>
  {/if}

  {#if bridgeOn}
    <div class="grp bridge" title="Bridge token configured — txAdmin/RCON enabled" transition:fade={{ duration: 100 }}>
      <Network size={11}/>
      <span class="lbl">bridge</span>
    </div>
  {/if}

  {#if version}
    <div class="grp version" title="Rift version">
      <span class="mono val faint">v{version}</span>
    </div>
  {/if}
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
  .val.warn { color: var(--warn); }
  .val.danger { color: var(--danger); }
  .val.faint { color: var(--fg-faint); }
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
  .sync-pill {
    background: transparent;
    border: 0;
    color: var(--info);
    cursor: pointer;
    padding: 0;
    font: inherit;
    font-size: var(--fs-xs);
    display: inline-flex; align-items: center; gap: 6px;
  }
  .sync-pill:hover { color: var(--fg); }
  .sync-pill .lbl { color: inherit; }
  .sync-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--info);
    animation: sync-pulse 1.4s ease-in-out infinite;
  }
  @keyframes sync-pulse {
    0%, 100% { opacity: 0.45; transform: scale(0.85); }
    50%      { opacity: 1;    transform: scale(1.15); }
  }
  @media (prefers-reduced-motion: reduce) {
    .sync-dot { animation: none; opacity: 0.85; }
  }

  .state-toggle {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    cursor: pointer;
    padding: 0 6px; margin: 0 -6px;
    border-radius: var(--radius-xs);
    text-transform: capitalize;
    transition: background 100ms ease, color 100ms ease;
  }
  .state-toggle:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .state-toggle:disabled { cursor: not-allowed; opacity: 0.6; }
  .state-toggle:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }

  .bridge { color: var(--info); }
  .bridge .lbl { color: inherit; }
</style>
