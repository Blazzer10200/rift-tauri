<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { AlertCircle, CheckCircle2, Loader2, FolderOpen, Info } from "lucide-svelte";
  import ObStage from "./ObStage.svelte";

  type Props = { onSaved: (serverKey: string) => void };
  const { onSaved }: Props = $props();

  let name = $state("");
  let host = $state("");
  let port = $state(22);
  let user = $state("");
  let keyPath = $state("");
  let remoteRoot = $state("/opt/cfx-server");
  let localRoot = $state("");

  let busy = $state(false);
  let error = $state<string | null>(null);
  let savedKey = $state<string | null>(null);

  const locked = $derived(busy || !!savedKey);

  async function pickKey() {
    error = null;
    try {
      const def = await invoke<string | null>("default_ssh_key_path").catch(() => null);
      if (def) keyPath = def;
      const picked = await openDialog({ title: "Pick SSH private key", multiple: false, directory: false });
      if (typeof picked === "string") keyPath = picked;
    } catch (e) {
      error = String(e);
    }
  }

  async function pickFolder() {
    error = null;
    try {
      const picked = await openDialog({ title: "Pick the local workspace folder", multiple: false, directory: true });
      if (typeof picked === "string") localRoot = picked;
    } catch (e) {
      error = String(e);
    }
  }

  function validate(): string | null {
    if (!name.trim()) return "Name is required.";
    if (!host.trim()) return "Host is required.";
    if (!user.trim()) return "Username is required.";
    if (!keyPath.trim()) return "SSH private key path is required.";
    if (!remoteRoot.trim() || remoteRoot.trim() === "/") return "Remote root must be a specific path, not '/'.";
    if (!localRoot.trim()) return "Local workspace folder is required.";
    if (port <= 0 || port > 65535) return "Port must be between 1 and 65535.";
    return null;
  }

  async function onSave() {
    const v = validate();
    if (v) { error = v; return; }
    busy = true;
    error = null;
    try {
      const saved = await invoke<{ key: string }>("save_server", {
        profile: {
          key: "",
          name: name.trim(),
          host: host.trim(),
          port,
          user: user.trim(),
          keyPath: keyPath.trim(),
          remoteRoot: remoteRoot.trim(),
          localRoot: localRoot.trim(),
        },
        editKey: null,
      });
      savedKey = saved.key;
      onSaved(saved.key);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<ObStage kind="server" caption="local ↔ remote over sftp" />

<header class="ob-head">
  <span class="ob-eyebrow">Step 2 · Connection</span>
  <h1 class="ob-title">Add your first server</h1>
  <p class="ob-sub">Mirror the SFTP details your host gave you, and pick a local folder to sync into. You can edit this later in Settings → Server.</p>
</header>

<div class="ob-grid">
  <div class="ob-field">
    <span class="ob-label">Name</span>
    <input class="ob-input" type="text" placeholder="Production · my-fivem" bind:value={name} disabled={locked} />
  </div>
  <div class="ob-field">
    <span class="ob-label">Host</span>
    <input class="ob-input mono" type="text" placeholder="game.example.com" bind:value={host} disabled={locked} />
  </div>
  <div class="ob-field">
    <span class="ob-label">User</span>
    <input class="ob-input mono" type="text" placeholder="root" bind:value={user} disabled={locked} />
  </div>
  <div class="ob-field">
    <span class="ob-label">Port</span>
    <input class="ob-input mono" type="number" min="1" max="65535" bind:value={port} disabled={locked} />
  </div>
  <div class="ob-field full">
    <span class="ob-label">SSH private key</span>
    <div class="ob-input-row">
      <input class="ob-input mono" type="text" placeholder="C:\Users\you\.ssh\id_rift_ed25519" bind:value={keyPath} disabled={locked} />
      <button class="ob-btn sm" type="button" onclick={pickKey} disabled={locked}><FolderOpen size={13} /> Browse…</button>
    </div>
  </div>
  <div class="ob-field full">
    <span class="ob-label">Remote root (game-server directory)</span>
    <input class="ob-input mono" type="text" placeholder="/opt/cfx-server" bind:value={remoteRoot} disabled={locked} />
  </div>
  <div class="ob-field full">
    <span class="ob-label">Local workspace folder</span>
    <div class="ob-input-row">
      <input class="ob-input mono" type="text" placeholder="C:\Projects\my-fivem-server" bind:value={localRoot} disabled={locked} />
      <button class="ob-btn sm" type="button" onclick={pickFolder} disabled={locked}><FolderOpen size={13} /> Browse…</button>
    </div>
  </div>
</div>

<p class="ob-note">
  <Info size={13} />
  <span>Use a fast local SSD. Avoid OneDrive / Dropbox folders — they collide with Rift's atomic-rename pattern and produce <strong>phantom drift</strong>.</span>
</p>

{#if error}
  <span class="ob-error"><AlertCircle size={14} /> {error}</span>
{/if}

{#if savedKey}
  <span class="ob-status ok"><CheckCircle2 size={16} /> Server saved — continue to the next step.</span>
{:else}
  <div class="ob-input-row">
    <button class="ob-btn accent" type="button" onclick={onSave} disabled={busy}>
      {#if busy}<Loader2 size={14} class="spin" />{/if}
      {busy ? "Saving…" : "Save server"}
    </button>
  </div>
{/if}
