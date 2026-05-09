<script lang="ts">
  import { connection, type ServerProfile } from "../state/connection.svelte";
  import { ChevronDown, Check, Plus, Pencil, Cable } from "lucide-svelte";

  let { onAddServer, onEditCurrent }: {
    onAddServer: () => void;
    onEditCurrent: (s: ServerProfile) => void;
  } = $props();

  let open = $state(false);
  let menuRef = $state<HTMLDivElement | null>(null);

  function onDocMouseDown(e: MouseEvent) {
    if (menuRef && !menuRef.contains(e.target as Node)) open = false;
  }

  $effect(() => {
    if (open) {
      document.addEventListener("mousedown", onDocMouseDown);
      return () => document.removeEventListener("mousedown", onDocMouseDown);
    }
  });

  async function pick(s: ServerProfile) {
    await connection.select(s.key);
    open = false;
  }

  type ConnState = "connected" | "connecting" | "error" | "offline";
  const connState = $derived<ConnState>(
    connection.connecting ? "connecting" :
    !connection.status ? "offline" :
    connection.status.state === "error" ? "error" :
    connection.status.state === "watching" || connection.status.state === "idle" || connection.status.state === "syncing" ? "connected" :
    "offline"
  );

  const connCfg: Record<ConnState, { cls: string; label: string; detail: string }> = {
    connected:  { cls: "ok",     label: "Connected",  detail: connection.selected?.fingerprint ? `ed25519 · ${connection.selected.fingerprint.slice(0, 18)}…` : "ed25519" },
    connecting: { cls: "info",   label: "Connecting", detail: "handshake" },
    error:      { cls: "danger", label: "Error",      detail: connection.status?.detail ?? "auth failed" },
    offline:    { cls: "muted",  label: "Offline",    detail: "not connected" },
  };

  const sel = $derived(connection.selected);
  const accentColor = "oklch(0.66 0.18 275)";
</script>

<div class="topbar">
  <div class="left">
    <div class="svr-picker" bind:this={menuRef} data-open={open}>
      <button class="svr-btn" onclick={() => (open = !open)} type="button">
        <span class="svr-dot" style="background: {accentColor}"></span>
        {#if sel}
          <span class="svr-name mono">{sel.name}</span>
          <span class="svr-host mono">{sel.user}@{sel.host}{sel.port !== 22 ? `:${sel.port}` : ""}</span>
        {:else}
          <span class="svr-name">No server</span>
          <span class="svr-host mono">— click to add</span>
        {/if}
        <ChevronDown size={12}/>
      </button>
      {#if open}
        <div class="svr-menu fade-in">
          <div class="menu-label">Servers</div>
          {#each connection.servers as s (s.key)}
            <button
              class="menu-item"
              data-active={sel?.key === s.key}
              onclick={() => pick(s)}
              type="button"
            >
              <span class="svr-dot" style="background: {accentColor}"></span>
              <span class="mono name-cell">{s.name}</span>
              <span class="mono host-cell">{s.user}@{s.host}</span>
              {#if sel?.key === s.key}<Check size={12}/>{/if}
            </button>
          {/each}
          {#if connection.servers.length === 0}
            <div class="empty help">No servers yet.</div>
          {/if}
          <div class="divider"></div>
          <button class="menu-item" onclick={() => { onAddServer(); open = false; }} type="button">
            <Plus size={12}/><span>Add server…</span>
            <span class="kbd shortcut">⌘N</span>
          </button>
          {#if sel}
            <button class="menu-item" onclick={() => { onEditCurrent(sel); open = false; }} type="button">
              <Pencil size={12}/><span>Edit current…</span>
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <div class="pill {connCfg[connState].cls}" title={connCfg[connState].detail}>
      <span class="dot"></span>
      <span>{connCfg[connState].label}</span>
      <span class="mono detail">{connCfg[connState].detail}</span>
    </div>
  </div>

  <div class="right">
    {#if sel?.bridgePort}
      <div class="bridge">
        <span class="bridge-led" class:on={connState === "connected"}></span>
        <Cable size={12}/>
        <span>txAdmin</span>
        <span class="mono dim">:{sel.bridgePort}{sel.txAdminUrl ? ` → ${sel.txAdminUrl.replace(/^https?:\/\//, "")}` : ""}</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .topbar {
    display: flex; align-items: center; justify-content: space-between;
    height: 44px; padding: 0 12px;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
  }
  .left, .right { display: flex; align-items: center; gap: 10px; }

  .svr-picker { position: relative; }
  .svr-btn {
    display: inline-flex; align-items: center; gap: 8px;
    height: 30px; padding: 0 10px;
    background: var(--surface); color: var(--fg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font: inherit; font-size: var(--fs-sm);
  }
  .svr-btn:hover { background: var(--surface-hover); }
  .svr-dot {
    width: 8px; height: 8px; border-radius: 50%;
    box-shadow: 0 0 0 2px color-mix(in oklch, currentColor 12%, transparent);
  }
  .svr-name { font-weight: 600; }
  .svr-host { color: var(--fg-subtle); font-size: var(--fs-xs); }

  .svr-menu {
    position: absolute; top: calc(100% + 6px); left: 0;
    width: 360px; padding: 6px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    z-index: 100;
  }
  .menu-label { padding: 6px 8px 4px; color: var(--fg-subtle); font-size: var(--fs-xs); letter-spacing: 0.05em; text-transform: uppercase; }
  .menu-item {
    display: flex; align-items: center; gap: 8px; width: 100%;
    padding: 6px 8px; height: 28px;
    background: transparent; border: 0; color: var(--fg);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-xs); cursor: pointer;
  }
  .menu-item:hover { background: var(--surface-hover); }
  .menu-item[data-active="true"] { background: var(--accent-soft); }
  .name-cell { flex: 1; }
  .host-cell { color: var(--fg-subtle); font-size: var(--fs-xs); }
  .shortcut { margin-left: auto; }
  .empty { padding: 8px; }

  .pill { height: 24px; }
  .pill .detail { color: currentColor; opacity: .68; margin-left: 4px; font-size: var(--fs-xs); }

  .bridge {
    display: inline-flex; align-items: center; gap: 6px;
    height: 26px; padding: 0 10px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--fs-xs); color: var(--fg-2);
  }
  .bridge-led {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--fg-faint);
    box-shadow: 0 0 0 2px color-mix(in oklch, currentColor 22%, transparent);
  }
  .bridge-led.on { background: var(--ok); }
</style>
