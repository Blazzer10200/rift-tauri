<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile } from "../../state/connection.svelte";
  import { Cog, Server, Key, Info, Plus, Pencil, Trash2, RefreshCw, Sparkles } from "lucide-svelte";
  import { updates } from "../../state/updates.svelte";

  type Section = "appearance" | "servers" | "keys" | "about";

  let { initialSection = "appearance", onAddServer, onEditServer, onDeleteServer, onLaunchKeygen }: {
    initialSection?: Section;
    onAddServer: () => void;
    onEditServer: (s: ServerProfile) => void;
    onDeleteServer: (s: ServerProfile) => void;
    onLaunchKeygen: () => void;
  } = $props();

  let section = $state<Section>(untrack(() => initialSection));
  let appVersion = $state("?");

  const sections: { id: Section; label: string; icon: typeof Cog }[] = [
    { id: "appearance", label: "Appearance", icon: Cog },
    { id: "servers",    label: "Servers",    icon: Server },
    { id: "keys",       label: "SSH keys",   icon: Key },
    { id: "about",      label: "About",      icon: Info },
  ];

  onMount(async () => {
    try { appVersion = await invoke<string>("app_version"); } catch {}
    await connection.loadServers();
  });

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
    {#key section}
      <div
        class="sub-shell"
        in:fly={{ y: 6, duration: 180, delay: 90, easing: quintOut }}
        out:fade={{ duration: 90 }}
      >
    {#if section === "appearance"}
      <div>
        <h3>Appearance</h3>
        <p class="help">Theme, density, and font controls.</p>
        <div class="soon card">
          <Sparkles size={18} class="soon-ico"/>
          <span class="soon-title">Coming soon</span>
          <span class="soon-hint">Density, font sizing, and accent-tint controls are planned for a later build. Dark mode stays the default.</span>
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
            <span class="empty-title">No servers yet</span>
            <span class="empty-hint">Click <strong>Add server</strong> to create one.</span>
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
                <div class="srv-meta mono dim" title={s.fingerprint ?? "no fingerprint pinned"}>
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
        <div class="set-head">
          <div>
            <h3>SSH keys</h3>
            <p class="help">ed25519 keypair stored at <span class="mono">%APPDATA%/Rift/keys/</span>. Generate or copy your public key from the setup dialog.</p>
          </div>
          <button class="btn primary sm" onclick={onLaunchKeygen} type="button">
            <Key size={11}/> Open key setup
          </button>
        </div>
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
          <button class="btn ghost sm" type="button" onclick={() => updates.open()}>
            <RefreshCw size={11}/> Check for updates
          </button>
        </div>
      </div>
    {/if}
      </div>
    {/key}
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
    transition: background 100ms, color 100ms;
  }
  .nav button:hover { background: var(--surface-hover); color: var(--fg); }
  .nav button[data-active="true"] {
    background: var(--surface); color: var(--fg);
    box-shadow: inset 2px 0 var(--accent);
  }

  .body {
    position: relative;
    min-width: 0;
    overflow: hidden;
  }
  .sub-shell {
    position: absolute;
    inset: 0;
    padding: 22px;
    overflow: auto;
  }
  h3 { margin: 0 0 6px; font-size: var(--fs-lg); font-weight: 600; color: var(--fg); }

  .set-head { display: flex; align-items: flex-end; justify-content: space-between; margin-bottom: 14px; gap: 12px; }

  .soon {
    margin-top: 14px;
    padding: 32px 24px;
    display: flex; flex-direction: column; gap: 8px;
    align-items: center; text-align: center;
    background: color-mix(in oklch, var(--accent) 6%, var(--surface));
    border: 1px dashed color-mix(in oklch, var(--accent) 35%, var(--border));
    border-radius: var(--radius);
  }
  .soon :global(.soon-ico) { color: var(--accent); }
  .soon-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .soon-hint { color: var(--fg-muted); font-size: var(--fs-xs); max-width: 360px; line-height: 1.5; }
  .set-row {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 4px;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
    gap: 12px;
  }
  .set-row:last-child { border-bottom: none; }

  .empty {
    padding: 28px 16px;
    display: flex; flex-direction: column; gap: 4px;
    align-items: center; text-align: center;
  }
  .empty-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .empty-hint { color: var(--fg-muted); font-size: var(--fs-xs); max-width: 280px; line-height: 1.45; }

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
    box-shadow: inset 2px 0 transparent;
    transition: background 100ms, border-color 100ms, box-shadow 100ms;
  }
  .srv-card:hover { background: var(--surface-hover); }
  .srv-card[data-active="true"] {
    background: color-mix(in oklch, var(--accent) 10%, var(--surface));
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border));
    box-shadow: inset 2px 0 var(--accent);
  }
  .srv-card[data-active="true"]:hover {
    background: color-mix(in oklch, var(--accent) 14%, var(--surface));
  }
  .srv-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .srv-dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 18%, transparent);
  }
  .srv-name { font-weight: 600; font-size: var(--fs-sm); }
  .srv-meta {
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    max-width: 220px;
  }
  .srv-r { display: flex; gap: 4px; }
</style>
