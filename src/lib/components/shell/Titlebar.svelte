<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Search, Cog, Minus, Square, X, ChevronDown, Check, Plus, Pencil, Cable } from "lucide-svelte";
  import { connection, type ServerProfile } from "../../state/connection.svelte";

  let { onOpenPalette, onOpenSettings, onAddServer, onEditCurrent }: {
    onOpenPalette: () => void;
    onOpenSettings: () => void;
    onAddServer: () => void;
    onEditCurrent: (s: ServerProfile) => void;
  } = $props();

  const win = getCurrentWindow();

  let menuOpen = $state(false);
  let menuRef = $state<HTMLDivElement | null>(null);

  function onDocMouseDown(e: MouseEvent) {
    if (menuRef && !menuRef.contains(e.target as Node)) menuOpen = false;
  }
  $effect(() => {
    if (menuOpen) {
      document.addEventListener("mousedown", onDocMouseDown);
      return () => document.removeEventListener("mousedown", onDocMouseDown);
    }
  });

  async function pick(s: ServerProfile) {
    await connection.select(s.key);
    menuOpen = false;
  }

  type ConnState = "connected" | "connecting" | "error" | "offline";
  const connState = $derived<ConnState>(
    connection.connecting ? "connecting" :
    !connection.status ? "offline" :
    connection.status.state === "error" ? "error" :
    connection.status.state === "watching" || connection.status.state === "idle" || connection.status.state === "syncing" ? "connected" :
    "offline"
  );
  const connCfg = $derived.by<Record<ConnState, { cls: string; label: string; title: string }>>(() => ({
    connected:  { cls: "ok",     label: "Connected",  title: connection.selected?.fingerprint ? `ed25519 · ${connection.selected.fingerprint}` : "ed25519" },
    connecting: { cls: "info",   label: "Connecting", title: "handshake in progress" },
    error:      { cls: "danger", label: "Error",      title: connection.status?.detail ?? "auth failed" },
    offline:    { cls: "muted",  label: "Offline",    title: "not connected" },
  }));

  const sel = $derived(connection.selected);
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="left">
    <div class="brand" data-tauri-drag-region>
      <img class="logo" src="/favicon.png" alt="" aria-hidden="true" draggable="false"/>
      <span class="app">Rift</span>
    </div>

    <div class="svr-picker" bind:this={menuRef} data-open={menuOpen}>
      <button class="svr-btn" onclick={() => (menuOpen = !menuOpen)} type="button">
        <span class="svr-dot"></span>
        {#if sel}
          <span class="svr-name">{sel.name}</span>
          <span class="svr-host mono">{sel.user}@{sel.host}{sel.port !== 22 ? `:${sel.port}` : ""}</span>
        {:else}
          <span class="svr-name">No server</span>
          <span class="svr-host mono">— click to add</span>
        {/if}
        <ChevronDown size={12}/>
      </button>
      {#if menuOpen}
        <div class="svr-menu">
          <div class="menu-label">Servers</div>
          {#each connection.servers as s (s.key)}
            <button
              class="menu-item"
              data-active={sel?.key === s.key}
              onclick={() => pick(s)}
              type="button"
            >
              <span class="svr-dot"></span>
              <span class="mono name-cell">{s.name}</span>
              <span class="mono host-cell">{s.user}@{s.host}</span>
              {#if sel?.key === s.key}<Check size={12}/>{/if}
            </button>
          {/each}
          {#if connection.servers.length === 0}
            <div class="empty help">No servers yet.</div>
          {/if}
          <div class="divider"></div>
          <button class="menu-item" onclick={() => { onAddServer(); menuOpen = false; }} type="button">
            <Plus size={12}/><span>Add server…</span>
            <span class="kbd shortcut">⌘N</span>
          </button>
          {#if sel}
            <button class="menu-item" onclick={() => { onEditCurrent(sel); menuOpen = false; }} type="button">
              <Pencil size={12}/><span>Edit current…</span>
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <div class="pill {connCfg[connState].cls}" title={connCfg[connState].title}>
      <span class="dot"></span>
      <span>{connCfg[connState].label}</span>
    </div>
  </div>

  <div class="drag-fill" data-tauri-drag-region></div>

  <div class="right">
    {#if sel?.bridgePort}
      <div class="bridge" title={sel.txAdminUrl ?? `:${sel.bridgePort}`}>
        <span class="bridge-led" class:on={connState === "connected"}></span>
        <Cable size={12}/>
        <span>txAdmin</span>
        <span class="mono dim">:{sel.bridgePort}</span>
      </div>
    {/if}
    <button class="cmdk" onclick={onOpenPalette} type="button" title="Command palette (Ctrl+K)">
      <Search size={12}/>
      <span>Search or run a command</span>
      <span class="kbd">Ctrl</span><span class="kbd">K</span>
    </button>
    <button class="iconbtn" onclick={onOpenSettings} title="Settings" type="button" aria-label="Settings">
      <Cog size={14}/>
    </button>
    <div class="winctl">
      <button class="wb" onclick={() => win.minimize()} title="Minimize" type="button" aria-label="Minimize">
        <Minus size={10}/>
      </button>
      <button class="wb" onclick={() => win.toggleMaximize()} title="Maximize" type="button" aria-label="Maximize">
        <Square size={9}/>
      </button>
      <button class="wb close" onclick={() => win.close()} title="Close" type="button" aria-label="Close">
        <X size={10}/>
      </button>
    </div>
  </div>
</div>

<style>
  .titlebar {
    display: flex; align-items: center;
    height: 44px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-xs);
    user-select: none;
    padding-left: 12px;
  }
  .left  { display: flex; align-items: center; gap: 10px; flex-shrink: 0; height: 100%; }
  .right { display: flex; align-items: center; gap: 6px; flex-shrink: 0; height: 100%; }
  .drag-fill { flex: 1; height: 100%; }

  .brand {
    display: inline-flex; align-items: center; gap: 8px;
    height: 100%;
    padding-right: 4px;
  }
  .logo {
    width: 18px; height: 18px;
    object-fit: contain;
    flex-shrink: 0;
    -webkit-user-drag: none;
  }
  .app { font-weight: 600; letter-spacing: -0.01em; color: var(--fg); }

  .svr-picker { position: relative; }
  .svr-btn {
    display: inline-flex; align-items: center; gap: 8px;
    height: 28px; padding: 0 10px;
    background: var(--surface); color: var(--fg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font: inherit; font-size: var(--fs-xs);
    transition: background 100ms ease, border-color 100ms ease;
  }
  .svr-btn:hover { background: var(--surface-hover); border-color: color-mix(in oklch, var(--accent) 30%, var(--border-strong)); }
  .svr-dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 22%, transparent);
    flex-shrink: 0;
  }
  .svr-name { font-weight: 600; font-size: var(--fs-xs); }
  .svr-host { color: var(--fg-subtle); font-size: 11px; }

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
  .divider { height: 1px; background: var(--border); margin: 4px 0; }

  .pill { height: 22px; padding: 0 8px; font-size: var(--fs-xs); }

  .bridge {
    display: inline-flex; align-items: center; gap: 6px;
    height: 24px; padding: 0 10px;
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
  .dim { color: var(--fg-subtle); }

  .cmdk {
    display: inline-flex; align-items: center; gap: 8px;
    height: 26px; padding: 0 10px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    cursor: pointer; min-width: 220px;
    transition: background 100ms ease, border-color 100ms ease;
  }
  .cmdk:hover {
    background: var(--bg-elev-2);
    color: var(--fg-2);
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  .cmdk > span:nth-child(2) { flex: 1; text-align: left; }
  .cmdk :global(.kbd) { background: var(--bg-elev-3); }

  .iconbtn {
    width: 26px; height: 26px; border-radius: var(--radius-sm);
    background: transparent; border: 1px solid transparent;
    color: var(--fg-muted); cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    transition: background 100ms ease, color 100ms ease;
  }
  .iconbtn:hover { background: var(--bg-elev-2); color: var(--fg); }

  .winctl { display: flex; height: 100%; margin-left: 4px; }
  .wb {
    width: 38px; height: 100%;
    background: transparent; border: none;
    color: var(--fg-muted); cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    transition: background 100ms ease, color 100ms ease;
  }
  .wb:hover { background: var(--bg-elev-2); color: var(--fg); }
  .wb.close:hover { background: var(--danger); color: oklch(0.99 0 0); }
</style>
