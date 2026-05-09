<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Search, Cog, Minus, Square, X } from "lucide-svelte";
  import { connection } from "../../state/connection.svelte";

  let { onOpenPalette, onOpenSettings }: {
    onOpenPalette: () => void;
    onOpenSettings: () => void;
  } = $props();

  const win = getCurrentWindow();

  const docName = $derived(connection.selected?.name ?? "no server");
  const docPath = $derived(connection.selected?.remoteRoot ?? "—");
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    <div class="logo" aria-hidden="true">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
        <path d="M4 3h8a4.5 4.5 0 0 1 1.6 8.7L19 21"/>
        <path d="M4 12h8"/>
        <path d="M4 3v18"/>
      </svg>
    </div>
    <span class="app">Rift</span>
    <span class="sep">—</span>
    <span class="doc mono">{docName} · {docPath}</span>
  </div>

  <div class="right">
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
    display: flex; align-items: center; justify-content: space-between;
    height: 32px; padding: 0 0 0 10px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-xs);
    user-select: none;
  }
  .left { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0; height: 100%; }
  .logo {
    width: 18px; height: 18px;
    display: inline-flex; align-items: center; justify-content: center;
    background: linear-gradient(135deg, var(--accent), color-mix(in oklch, var(--accent) 55%, var(--bg)));
    color: var(--accent-fg);
    border-radius: var(--radius-xs);
  }
  .app { font-weight: 600; letter-spacing: -0.01em; color: var(--fg); }
  .sep { color: var(--fg-faint); }
  .doc { color: var(--fg-muted); font-size: var(--fs-xs); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .right { display: flex; align-items: center; gap: 4px; height: 100%; }
  .cmdk {
    display: inline-flex; align-items: center; gap: 8px;
    height: 22px; padding: 0 8px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    cursor: pointer; min-width: 240px;
    transition: background 80ms;
  }
  .cmdk:hover { background: var(--bg-elev-2); color: var(--fg-2); }
  .cmdk > span:nth-child(2) { flex: 1; text-align: left; }
  .cmdk :global(.kbd) { background: var(--bg-elev-3); }

  .iconbtn {
    width: 22px; height: 22px; border-radius: var(--radius-sm);
    background: transparent; border: 1px solid transparent;
    color: var(--fg-muted); cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .iconbtn:hover { background: var(--bg-elev-2); color: var(--fg); }

  .winctl { display: flex; height: 100%; margin-left: 6px; }
  .wb {
    width: 38px; height: 100%;
    background: transparent; border: none;
    color: var(--fg-muted); cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .wb:hover { background: var(--bg-elev-2); color: var(--fg); }
  .wb.close:hover { background: var(--danger); color: oklch(0.99 0 0); }
</style>
