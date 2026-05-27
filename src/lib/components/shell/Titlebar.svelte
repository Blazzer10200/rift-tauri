<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Minus, Square, X, ChevronDown, Check, Plus, Pencil, Cable } from "lucide-svelte";
  import { connection, type ServerProfile } from "../../state/connection.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  let { onAddServer, onEditCurrent }: {
    onAddServer: () => void;
    onEditCurrent: (s: ServerProfile) => void;
  } = $props();

  const win = getCurrentWindow();

  let menuOpen = $state(false);
  let menuRef = $state<HTMLDivElement | null>(null);
  let triggerRef = $state<HTMLButtonElement | null>(null);

  function onDocMouseDown(e: MouseEvent) {
    if (menuRef && !menuRef.contains(e.target as Node)) menuOpen = false;
  }
  function onMenuKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      menuOpen = false;
      triggerRef?.focus();
    }
  }
  $effect(() => {
    if (menuOpen) {
      document.addEventListener("mousedown", onDocMouseDown);
      document.addEventListener("keydown", onMenuKey);
      return () => {
        document.removeEventListener("mousedown", onDocMouseDown);
        document.removeEventListener("keydown", onMenuKey);
      };
    }
  });

  async function pick(s: ServerProfile) {
    await connection.select(s.key);
    menuOpen = false;
  }

  type ConnState = "connected" | "connecting" | "error" | "offline";
  const connState = $derived<ConnState>(
    connection.isHandshaking ? "connecting" :
    !connection.status ? "offline" :
    connection.status.state === "error" ? "error" :
    connection.status.state === "watching" || connection.status.state === "idle" || connection.status.state === "syncing" ? "connected" :
    "offline"
  );
  const connCfg = $derived.by<Record<ConnState, { cls: string; label: string; title: string }>>(() => ({
    connected:  { cls: "ok",     label: "Connected",  title: connection.selected?.fingerprint ? `ed25519 · ${connection.selected.fingerprint}` : "ed25519" },
    connecting: { cls: "info",   label: "Connecting", title: "handshake in progress" },
    error:      { cls: "danger", label: "Sync error", title: connection.status?.detail ? `${connection.status.detail} — connection still active` : "auth failed" },
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
      <button
        bind:this={triggerRef}
        class="svr-btn"
        onclick={() => (menuOpen = !menuOpen)}
        type="button"
        use:tooltip={`${connCfg[connState].label} — ${connCfg[connState].title}`}
        aria-haspopup="listbox"
        aria-expanded={menuOpen}
      >
        <span class="svr-dot" data-state={connCfg[connState].cls}></span>
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
        <div class="svr-menu" role="listbox" aria-label="Server profiles">
          <div class="menu-label">Servers</div>
          {#each connection.servers as s (s.key)}
            <button
              class="menu-item"
              role="option"
              aria-selected={sel?.key === s.key}
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
            <span class="kbd shortcut">Ctrl+N</span>
          </button>
          {#if sel}
            <button class="menu-item" onclick={() => { onEditCurrent(sel); menuOpen = false; }} type="button">
              <Pencil size={12}/><span>Edit current…</span>
            </button>
          {/if}
        </div>
      {/if}
    </div>

  </div>

  <div class="drag-fill" data-tauri-drag-region></div>

  <div class="right">
    {#if sel?.bridgePort}
      <div class="bridge" use:tooltip={sel.txAdminUrl ?? `:${sel.bridgePort}`}>
        <span class="bridge-led" class:on={connState === "connected"}></span>
        <Cable size={12}/>
        <span>txAdmin</span>
        <span class="mono dim">:{sel.bridgePort}</span>
      </div>
    {/if}
    <div class="winctl">
      <button class="wb" onclick={() => win.minimize()} use:tooltip={"Minimize"} type="button" aria-label="Minimize">
        <Minus size={10}/>
      </button>
      <button class="wb" onclick={() => win.toggleMaximize()} use:tooltip={"Maximize"} type="button" aria-label="Maximize">
        <Square size={9}/>
      </button>
      <button class="wb close" onclick={() => win.close()} use:tooltip={"Close"} type="button" aria-label="Close">
        <X size={10}/>
      </button>
    </div>
  </div>
</div>

<style>
  .titlebar {
    display: flex; align-items: center;
    height: 44px;
    width: 100%;
    min-width: 0;
    background: var(--bg-elev-1, var(--bg));
    border-bottom: 1px solid var(--border-strong);
    box-shadow: 0 1px 0 color-mix(in oklch, var(--bg) 60%, transparent);
    font-size: var(--fs-xs);
    user-select: none;
    padding-left: 12px;
  }
  /* Layout priority:
     - .right (window controls) stays flex-shrink:0 — always clickable.
     - .drag-fill takes ALL leftover space — that's the window-drag handle.
     - .left can shrink (server picker truncates) but doesn't grow past content. */
  .left      { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 0 1 auto; height: 100%; }
  .drag-fill { flex: 1 1 auto; min-width: 24px; height: 100%; }
  .right     { display: flex; align-items: center; gap: 6px; flex-shrink: 0; height: 100%; }

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
    background: color-mix(in oklch, var(--surface) 80%, transparent);
    color: var(--fg);
    border: 1px solid color-mix(in oklch, var(--border-strong) 90%, transparent);
    border-radius: 8px;
    cursor: pointer;
    font: inherit; font-size: var(--fs-xs);
    transition: background 140ms ease, border-color 140ms ease, box-shadow 140ms ease;
    min-width: 0; max-width: 100%;
  }
  .svr-btn:hover {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--accent) 35%, var(--border-strong));
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 10%, transparent);
  }
  .svr-picker[data-open="true"] .svr-btn {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border-strong));
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .svr-dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--fg-faint);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--fg-faint) 22%, transparent);
    flex-shrink: 0;
    transition: background 120ms ease, box-shadow 120ms ease;
  }
  .svr-dot[data-state="ok"]     { background: var(--ok);     box-shadow: 0 0 0 2px color-mix(in oklch, var(--ok)     22%, transparent); }
  .svr-dot[data-state="info"]   { background: var(--info);   box-shadow: 0 0 0 2px color-mix(in oklch, var(--info)   22%, transparent); }
  .svr-dot[data-state="danger"] { background: var(--danger); box-shadow: 0 0 0 2px color-mix(in oklch, var(--danger) 22%, transparent); }
  .svr-dot[data-state="muted"]  { background: var(--fg-faint); box-shadow: 0 0 0 2px color-mix(in oklch, var(--fg-faint) 22%, transparent); }
  @media (prefers-reduced-motion: no-preference) {
    .svr-dot[data-state="ok"]   { animation: dot-breathe 2.6s ease-in-out infinite; }
    .svr-dot[data-state="info"] { animation: dot-breathe 1.4s ease-in-out infinite; }
  }
  @keyframes dot-breathe {
    0%, 100% { box-shadow: 0 0 0 2px color-mix(in oklch, currentColor 22%, transparent); }
    50%      { box-shadow: 0 0 0 4px color-mix(in oklch, currentColor 14%, transparent); }
  }
  .svr-name { font-weight: 600; font-size: var(--fs-xs); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex-shrink: 1; min-width: 0; }
  .svr-host { color: var(--fg-subtle); font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex-shrink: 2; min-width: 0; }

  .svr-menu {
    position: absolute; top: calc(100% + 8px); left: 0;
    width: 380px; padding: 6px;
    background: color-mix(in oklch, var(--bg-elev-2) 86%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--border-strong) 80%, transparent);
    border-radius: 12px;
    box-shadow:
      0 20px 50px -10px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklch, var(--accent) 6%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    z-index: 1000;
    animation: svr-menu-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-origin: top left;
  }
  @keyframes svr-menu-in {
    from { opacity: 0; transform: translateY(-4px) scale(0.98); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }
  @media (prefers-reduced-motion: reduce) { .svr-menu { animation: none; } }
  .menu-label {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 10px 6px;
    color: var(--fg-faint);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .menu-label::after {
    content: "";
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
  }
  .menu-item {
    display: flex; align-items: center; gap: 10px; width: 100%;
    padding: 7px 10px; height: 30px;
    background: transparent; border: 0; color: var(--fg);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: 8px; cursor: pointer;
    transition: background 140ms ease-out;
  }
  .menu-item:hover { background: color-mix(in oklch, var(--accent) 10%, transparent); }
  .menu-item[data-active="true"] {
    background: linear-gradient(90deg,
      color-mix(in oklch, var(--accent) 14%, transparent),
      color-mix(in oklch, var(--accent) 4%, transparent) 70%);
  }
  .name-cell { flex: 1; }
  .host-cell { color: var(--fg-subtle); font-size: var(--fs-xs); }
  .shortcut { margin-left: auto; }
  .empty { padding: 8px; }
  .divider { height: 1px; background: var(--border); margin: 4px 0; }

  .bridge {
    display: inline-flex; align-items: center; gap: 6px;
    height: 24px; padding: 0 10px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 75%, transparent);
    border-radius: 999px;
    font-size: var(--fs-xs); color: var(--fg-2);
    transition: background 140ms ease, border-color 140ms ease;
  }
  .bridge:hover {
    background: var(--bg-elev-2);
    border-color: color-mix(in oklch, var(--ok) 30%, var(--border));
  }
  .bridge-led {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--fg-faint);
    box-shadow: 0 0 0 2px color-mix(in oklch, currentColor 22%, transparent);
    flex-shrink: 0;
  }
  .bridge-led.on {
    background: var(--ok);
    box-shadow:
      0 0 0 2px color-mix(in oklch, var(--ok) 24%, transparent),
      0 0 8px color-mix(in oklch, var(--ok) 50%, transparent);
    animation: bridge-breathe 2.6s ease-in-out infinite;
  }
  @keyframes bridge-breathe {
    0%, 100% { box-shadow: 0 0 0 2px color-mix(in oklch, var(--ok) 22%, transparent),
                          0 0 6px color-mix(in oklch, var(--ok) 45%, transparent); }
    50%      { box-shadow: 0 0 0 4px color-mix(in oklch, var(--ok) 14%, transparent),
                          0 0 10px color-mix(in oklch, var(--ok) 55%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) { .bridge-led.on { animation: none; } }
  .dim { color: var(--fg-subtle); }

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
