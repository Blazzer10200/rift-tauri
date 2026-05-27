<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { Server, ChevronRight, ChevronLeft, AlertCircle, CheckCircle2, Loader2, FolderOpen } from "lucide-svelte";

  type Props = {
    localRoot: string;
    onNext: (serverKey: string) => void;
    onBack?: () => void;
  };
  const { localRoot, onNext, onBack }: Props = $props();

  let name = $state("");
  let host = $state("");
  let port = $state(22);
  let user = $state("");
  let keyPath = $state("");
  let remoteRoot = $state("/opt/cfx-server");

  let busy = $state(false);
  let error = $state<string | null>(null);
  let savedKey = $state<string | null>(null);

  async function pickKey() {
    error = null;
    try {
      const def = await invoke<string | null>("default_ssh_key_path").catch(() => null);
      if (def) keyPath = def;
      const picked = await openDialog({
        title: "Pick SSH private key",
        multiple: false,
        directory: false,
      });
      if (typeof picked === "string") keyPath = picked;
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
    if (!localRoot.trim()) return "Local root missing — go back and pick one.";
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
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function next() {
    if (savedKey) onNext(savedKey);
  }
</script>

<div class="onboarding-step">
  <header class="onboarding-head">
    <div class="onboarding-icon"><Server size={28} /></div>
    <h2 class="onboarding-title">Add your first server</h2>
    <p class="onboarding-sub">
      Mirror the SFTP credentials your game server admin gave you.
      You can edit or remove this server later from Settings → Network.
    </p>
  </header>

  <div class="grid">
    <label>
      <span>Name</span>
      <input class="input" type="text" placeholder="Production · my-fivem" bind:value={name} disabled={busy || !!savedKey} />
    </label>
    <label>
      <span>Host</span>
      <input class="input mono" type="text" placeholder="game.example.com" bind:value={host} disabled={busy || !!savedKey} />
    </label>
    <label class="port-row">
      <span>Port</span>
      <input class="input mono" type="number" min="1" max="65535" bind:value={port} disabled={busy || !!savedKey} />
    </label>
    <label>
      <span>User</span>
      <input class="input mono" type="text" placeholder="root" bind:value={user} disabled={busy || !!savedKey} />
    </label>
    <label class="full">
      <span>SSH private key</span>
      <div class="key-row">
        <input class="input mono" type="text" placeholder="C:\\Users\\you\\.ssh\\id_rift_ed25519" bind:value={keyPath} disabled={busy || !!savedKey} />
        <button class="btn sm" type="button" onclick={pickKey} disabled={busy || !!savedKey}>
          <FolderOpen size={12} /> Browse…
        </button>
      </div>
    </label>
    <label class="full">
      <span>Remote root (game-server directory)</span>
      <input class="input mono" type="text" placeholder="/opt/cfx-server" bind:value={remoteRoot} disabled={busy || !!savedKey} />
    </label>
    <label class="full">
      <span>Local root (from previous step)</span>
      <input class="input mono" type="text" value={localRoot} readonly disabled />
    </label>
  </div>

  {#if error}
    <p class="error-line"><AlertCircle size={14} /> {error}</p>
  {/if}

  {#if savedKey}
    <p class="status-line">
      <CheckCircle2 size={16} />
      Server saved · key <code>{savedKey}</code>
    </p>
  {/if}

  <footer class="onboarding-foot">
    <button class="btn ghost sm" type="button" onclick={() => onBack?.()} disabled={busy}>
      <ChevronLeft size={14} /> Back
    </button>
    {#if !savedKey}
      <button class="btn primary" type="button" onclick={onSave} disabled={busy}>
        {#if busy}<Loader2 size={14} class="spin" />{/if}
        {busy ? "Saving…" : "Save server"}
      </button>
    {:else}
      <button class="btn primary" type="button" onclick={next}>
        Next <ChevronRight size={14} />
      </button>
    {/if}
  </footer>
</div>

<style>
  .onboarding-step {
    display: flex; flex-direction: column; gap: 16px;
    max-width: 620px; margin: 56px auto; padding: 28px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .onboarding-head { display: flex; flex-direction: column; gap: 6px; }
  .onboarding-icon {
    width: 48px; height: 48px;
    display: flex; align-items: center; justify-content: center;
    border-radius: var(--radius-md);
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: var(--accent);
  }
  .onboarding-title { font-size: 18px; font-weight: 600; color: var(--fg); margin: 0; }
  .onboarding-sub { font-size: 13px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .grid label { display: flex; flex-direction: column; gap: 4px; font-size: 12.5px; color: var(--fg-muted); }
  .grid label.full { grid-column: 1 / -1; }
  .grid label span { font-size: 12px; }
  .port-row .input { font-variant-numeric: tabular-nums; }
  .key-row { display: flex; gap: 8px; align-items: center; }
  .key-row .input { flex: 1; }
  .error-line { display: flex; align-items: center; gap: 6px; color: var(--danger); font-size: 12.5px; margin: 0; }
  .status-line { display: flex; align-items: center; gap: 6px; color: var(--success, oklch(0.78 0.16 145)); font-size: 13px; margin: 0; }
  code { font-family: var(--font-mono); font-size: 12px; padding: 1px 5px; background: var(--bg-elev-2); border-radius: var(--radius-xs); }
  .onboarding-foot {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px; padding-top: 16px;
    border-top: 1px solid var(--border);
  }
</style>
