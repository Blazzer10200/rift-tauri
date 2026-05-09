<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile } from "../../state/connection.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import { Cog, Bolt, Server, Key, RefreshCw, Terminal, Info, Plus, Pencil, Trash2, Copy, Check } from "lucide-svelte";
  import { updates } from "../../state/updates.svelte";

  type Section = "appearance" | "tokens" | "servers" | "keys" | "sync" | "editor" | "about";

  let { initialSection = "appearance", onAddServer, onEditServer, onDeleteServer, onLaunchKeygen }: {
    initialSection?: Section;
    onAddServer: () => void;
    onEditServer: (s: ServerProfile) => void;
    onDeleteServer: (s: ServerProfile) => void;
    onLaunchKeygen: () => void;
  } = $props();

  let section = $state<Section>(initialSection);
  let appVersion = $state("?");
  let copied = $state(false);

  const sections: { id: Section; label: string; icon: typeof Cog }[] = [
    { id: "appearance", label: "Appearance",    icon: Cog },
    { id: "tokens",     label: "Design tokens", icon: Bolt },
    { id: "servers",    label: "Servers",       icon: Server },
    { id: "keys",       label: "SSH keys",      icon: Key },
    { id: "sync",       label: "Sync",          icon: RefreshCw },
    { id: "editor",     label: "Editor",        icon: Terminal },
    { id: "about",      label: "About",         icon: Info },
  ];

  const TOKENS_BLOB = `:root {
  /* Rift — Linear-precise · DARK */
  --bg:            oklch(0.142 0.008 270);
  --bg-elev-1:     oklch(0.178 0.010 270);
  --bg-elev-2:     oklch(0.212 0.012 270);
  --surface:       oklch(0.205 0.011 270);
  --fg:            oklch(0.972 0.004 270);
  --fg-2:          oklch(0.86  0.006 270);
  --fg-muted:      oklch(0.68  0.012 270);
  --accent:        oklch(0.66  0.18  275);
  --ok:            oklch(0.74  0.14  150);
  --warn:          oklch(0.80  0.14  78);
  --danger:        oklch(0.68  0.20  22);
  --info:          oklch(0.72  0.13  230);
  --border:        oklch(1 0 0 / 0.07);
  --border-strong: oklch(1 0 0 / 0.13);
  --ring:          oklch(0.66  0.18  275 / 0.45);
  --radius:        6px;
  --row-h:         28px;
  --font-ui:   "Inter Variable", "Inter", "Segoe UI", system-ui, sans-serif;
  --font-mono: "JetBrains Mono Variable", "JetBrains Mono", Consolas, monospace;
}`;

  onMount(async () => {
    try { appVersion = await invoke<string>("app_version"); } catch {}
    await connection.loadServers();
  });

  async function copyTokens() {
    try {
      await navigator.clipboard.writeText(TOKENS_BLOB);
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch (e) { console.error(e); }
  }

  async function pickServer(s: ServerProfile) {
    await connection.select(s.key);
  }
</script>

<div class="settings">
  <nav class="nav">
    {#each sections as s (s.id)}
      {@const Icon = s.icon}
      <button data-active={section === s.id} onclick={() => (section = s.id)} type="button">
        <Icon size={13}/><span>{s.label}</span>
      </button>
    {/each}
  </nav>

  <div class="body">
    {#if section === "appearance"}
      <div class="set-grid">
        <h3>Appearance</h3>
        <p class="help">Visual direction and density. Dark mode only.</p>
        <div class="set-row"><span>Direction</span><span class="mono">linear-precise</span></div>
        <div class="set-row"><span>Mode</span><span class="mono">dark</span></div>
        <div class="set-row"><span>Density</span>
          <div class="seg">
            {#each (["compact","regular","comfy"] as const) as d}
              <button data-active={uiPrefs.density === d}
                      onclick={() => uiPrefs.setDensity(d)} type="button">{d}</button>
            {/each}
          </div>
        </div>
        <div class="set-row"><span>Font (UI)</span><span class="mono">Inter Variable</span></div>
        <div class="set-row"><span>Font (mono)</span><span class="mono">JetBrains Mono Variable</span></div>
      </div>

    {:else if section === "tokens"}
      <div>
        <div class="set-head">
          <div>
            <h3>Design tokens</h3>
            <p class="help">OKLCH variable block ready to paste into <span class="mono">src/app.css</span>.</p>
          </div>
        </div>
        <div class="tokens-card">
          <div class="tokens-head">
            <span class="mono dim">src/app.css · @theme dark</span>
            <button class="btn sm" onclick={copyTokens} type="button">
              {#if copied}<Check size={11}/> Copied{:else}<Copy size={11}/> Copy{/if}
            </button>
          </div>
          <pre class="tokens-pre mono">{TOKENS_BLOB}</pre>
        </div>
      </div>

    {:else if section === "servers"}
      <div>
        <div class="set-head">
          <div>
            <h3>Servers</h3>
            <p class="help">{connection.servers.length} configured · profiles, keys, and bridge ports.</p>
          </div>
          <button class="btn primary sm" onclick={onAddServer} type="button">
            <Plus size={11}/> Add server
          </button>
        </div>
        {#if connection.servers.length === 0}
          <div class="empty card">
            <p>No servers yet. Click <strong>Add server</strong> to create one.</p>
          </div>
        {:else}
          <div class="srv-list">
            {#each connection.servers as s (s.key)}
              <div
                class="srv-card"
                data-active={connection.selectedKey === s.key}
                role="button"
                tabindex="0"
                onclick={() => pickServer(s)}
                onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); pickServer(s); } }}
              >
                <div class="srv-l">
                  <span class="srv-dot lg"></span>
                  <div>
                    <div class="mono srv-name">{s.name}</div>
                    <div class="mono dim">{s.user}@{s.host}{s.port !== 22 ? `:${s.port}` : ""}</div>
                  </div>
                </div>
                <div class="srv-meta mono dim">
                  {s.fingerprint ? `ed25519 · ${s.fingerprint.slice(0, 18)}…` : "no fingerprint pinned"}
                </div>
                <div class="srv-r">
                  <button class="btn ghost sm" onclick={(e) => { e.stopPropagation(); onEditServer(s); }} type="button" title="Edit" aria-label="Edit">
                    <Pencil size={11}/>
                  </button>
                  <button class="btn ghost sm" onclick={(e) => { e.stopPropagation(); onDeleteServer(s); }} type="button" title="Delete" aria-label="Delete">
                    <Trash2 size={11}/>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    {:else if section === "keys"}
      <div>
        <h3>SSH keys</h3>
        <p class="help">ed25519 keypair stored at <span class="mono">%APPDATA%/Rift/keys/</span>.</p>
        <div class="card key-card">
          <p class="help">Use the SSH key setup dialog to generate a new keypair or copy your public key.</p>
          <button class="btn primary sm" onclick={onLaunchKeygen} type="button">
            <Key size={11}/> Open key setup
          </button>
        </div>
      </div>

    {:else if section === "sync"}
      <div>
        <h3>Sync</h3>
        <p class="help">Real-time bidirectional sync rules — driven by the autosync engine.</p>
        <div class="set-row"><span>Engine state</span><span class="mono">{connection.status?.state ?? "offline"}</span></div>
        <div class="set-row"><span>Watches</span><span class="mono">{connection.status?.watches ?? 0}</span></div>
        <div class="set-row"><span>Pending</span><span class="mono">{connection.status?.pending ?? 0}</span></div>
        <div class="set-row"><span>Failed</span><span class="mono" class:danger={(connection.status?.failed ?? 0) > 0}>{connection.status?.failed ?? 0}</span></div>
        <div class="set-row"><span>Conflicts</span><span class="mono" class:danger={connection.conflictCount > 0}>{connection.conflictCount}</span></div>
        <div class="set-row"><span>Locks</span><span class="mono">{connection.lockCount}</span></div>
        <div class="set-row"><span>Ignored total</span><span class="mono dim">{connection.status?.ignored_total ?? 0}</span></div>
      </div>

    {:else if section === "editor"}
      <div>
        <h3>Editor</h3>
        <p class="help">Which application opens files when you press Enter on a remote file.</p>
        <div class="set-row"><span>Default editor</span><span class="mono dim">system default</span></div>
        <div class="set-row"><span>Wait flag</span><span class="mono dim">auto</span></div>
        <p class="help">Editor preferences are managed through your OS file associations.</p>
      </div>

    {:else if section === "about"}
      <div>
        <h3>About</h3>
        <div class="set-row"><span>Rift</span><span class="mono">{appVersion} · Tauri 2</span></div>
        <div class="set-row"><span>Engine</span><span class="mono">SvelteKit + Svelte 5 (runes)</span></div>
        <div class="set-row"><span>Style</span><span class="mono">Tailwind v4 · OKLCH tokens · Linear-precise</span></div>
        <div class="set-row"><span>SSH</span><span class="mono">russh + russh-sftp · ring backend</span></div>
        <div class="set-row"><span>License</span><span class="mono">MIT · github.com/Blazzer10200/rift</span></div>
        <div class="set-row">
          <span>Updates</span>
          <span class="upd-cell">
            <button type="button" onclick={() => updates.open()}>
              <RefreshCw size={11}/>
              Check for updates
            </button>
          </span>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings {
    display: grid;
    grid-template-columns: 200px 1fr;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .nav {
    display: flex; flex-direction: column; gap: 1px;
    padding: 12px 8px;
    background: var(--bg);
    border-right: 1px solid var(--border);
    overflow: auto;
  }
  .nav button {
    display: flex; align-items: center; gap: 8px;
    width: 100%; height: 28px; padding: 0 10px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .nav button:hover { background: var(--surface-hover); color: var(--fg); }
  .nav button[data-active="true"] {
    background: var(--surface); color: var(--fg);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .body {
    padding: 22px;
    overflow: auto;
    min-width: 0;
  }
  h3 { margin: 0 0 6px; font-size: var(--fs-lg); font-weight: 600; color: var(--fg); }

  .set-grid { display: flex; flex-direction: column; }
  .set-head { display: flex; align-items: flex-end; justify-content: space-between; margin-bottom: 14px; gap: 12px; }
  .set-row {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 4px;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
    gap: 12px;
  }
  .set-row:last-child { border-bottom: none; }
  .set-row .danger { color: var(--danger); }

  .seg {
    display: inline-flex; gap: 1px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px;
  }
  .seg button {
    background: transparent; border: 0;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    padding: 3px 9px;
    border-radius: var(--radius-xs);
    cursor: pointer;
  }
  .seg button[data-active="true"] { background: var(--surface); color: var(--fg); }

  .tokens-card {
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .tokens-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .tokens-pre {
    margin: 0;
    padding: 12px;
    font-size: var(--fs-sm);
    color: var(--fg-2);
    line-height: 1.6;
    overflow-x: auto;
  }

  .empty { padding: 24px; text-align: center; color: var(--fg-muted); }

  .srv-list { display: flex; flex-direction: column; gap: 6px; }
  .srv-card {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--fg);
    font: inherit;
    text-align: left;
  }
  .srv-card:hover { background: var(--surface-hover); }
  .srv-card[data-active="true"] {
    border-color: color-mix(in oklch, var(--accent) 50%, transparent);
    background: var(--accent-soft);
  }
  .srv-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .srv-dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 18%, transparent);
  }
  .srv-name { font-weight: 600; font-size: var(--fs-sm); }
  .srv-meta { font-size: var(--fs-xs); }
  .srv-r { display: flex; gap: 4px; }

  .key-card { padding: 18px; display: flex; flex-direction: column; gap: 12px; align-items: flex-start; }

  .upd-cell { display: inline-flex; align-items: center; gap: 8px; }
  .upd-cell button {
    display: inline-flex; align-items: center; gap: 6px;
    height: 22px; padding: 0 10px;
    background: var(--bg-elev-2); border: 1px solid var(--border);
    color: var(--fg); border-radius: var(--radius-sm);
    font: inherit; font-size: var(--fs-xs); cursor: pointer;
  }
  .upd-cell button:hover:not(:disabled) { background: var(--surface-hover); }
  .upd-cell button:disabled { opacity: 0.6; cursor: default; }
  :global(.spin) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
