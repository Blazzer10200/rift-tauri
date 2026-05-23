<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { Key, ChevronRight, ChevronLeft, CheckCircle2, AlertCircle, Loader2, Copy, FolderOpen } from "lucide-svelte";

  type Props = {
    onNext: () => void;
    onBack?: () => void;
  };
  const { onNext, onBack }: Props = $props();

  type Mode = "choose" | "generate" | "import" | "ready";
  let mode = $state<Mode>("choose");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let keyPath = $state<string | null>(null);
  let pubKey = $state<string | null>(null);
  let copied = $state(false);

  onMount(async () => {
    try {
      const exists = await invoke<boolean>("default_ssh_key_exists");
      if (exists) {
        keyPath = await invoke<string>("default_ssh_key_path");
        pubKey = await invoke<string>("read_default_ssh_pub_key");
        mode = "ready";
      }
    } catch (e) {
      // Probe failure is non-fatal — user can still generate or import.
      console.debug("ssh key probe failed:", e);
    }
  });

  async function onGenerate() {
    busy = true;
    error = null;
    try {
      await invoke("generate_default_ssh_key");
      keyPath = await invoke<string>("default_ssh_key_path");
      pubKey = await invoke<string>("read_default_ssh_pub_key");
      mode = "ready";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function onImport() {
    error = null;
    const picked = await openDialog({
      title: "Pick existing SSH private key",
      multiple: false,
      directory: false,
    });
    if (typeof picked !== "string") return;
    keyPath = picked;
    mode = "ready";
    // We don't read the pub key for imported paths — the user already has it
    // wherever they normally manage it.
    pubKey = null;
  }

  async function onCopyPub() {
    if (!pubKey) return;
    try {
      await navigator.clipboard.writeText(pubKey);
      copied = true;
      setTimeout(() => { copied = false; }, 1800);
    } catch (e) {
      error = `clipboard: ${e}`;
    }
  }
</script>

<div class="onboarding-step">
  <header class="onboarding-head">
    <div class="onboarding-icon"><Key size={28} /></div>
    <h2 class="onboarding-title">SSH key</h2>
    <p class="onboarding-sub">
      Rift uses SSH key authentication for SFTP — no passwords stored.
    </p>
  </header>

  {#if mode === "choose"}
    <div class="choice-grid">
      <button class="choice" type="button" onclick={() => (mode = "generate")} disabled={busy}>
        <strong>Generate a new key</strong>
        <span class="hint">Creates <code>~/.ssh/id_rift_ed25519</code>. You'll copy the public half into your server's <code>authorized_keys</code>.</span>
      </button>
      <button class="choice" type="button" onclick={onImport} disabled={busy}>
        <strong>Import existing key</strong>
        <span class="hint">Pick a private-key file you already use. Rift just remembers the path.</span>
      </button>
    </div>
  {/if}

  {#if mode === "generate"}
    <div class="confirm-block">
      <p>
        About to run <code>ssh-keygen -t ed25519 -f ~/.ssh/id_rift_ed25519</code>.
        Existing key at this path will not be overwritten.
      </p>
      <div class="row">
        <button class="btn ghost" type="button" onclick={() => (mode = "choose")} disabled={busy}>Back</button>
        <button class="btn primary" type="button" onclick={onGenerate} disabled={busy}>
          {#if busy}<Loader2 size={14} class="spin" />{/if}
          {busy ? "Generating…" : "Generate now"}
        </button>
      </div>
    </div>
  {/if}

  {#if mode === "ready"}
    <div class="status-block">
      <p class="status-line">
        <CheckCircle2 size={16} />
        Key ready
      </p>
      {#if keyPath}
        <div class="path-row">
          <FolderOpen size={14} />
          <code>{keyPath}</code>
        </div>
      {/if}
      {#if pubKey}
        <p class="next-step">
          Copy the public half and add it to <code>~/.ssh/authorized_keys</code> on
          your game server (one line, no quotes).
        </p>
        <textarea class="pub-key" readonly rows="3" value={pubKey}></textarea>
        <button class="btn sm" type="button" onclick={onCopyPub}>
          <Copy size={12} />
          {copied ? "Copied" : "Copy public key"}
        </button>
      {/if}
    </div>
  {/if}

  {#if error}
    <p class="error-line"><AlertCircle size={14} /> {error}</p>
  {/if}

  <footer class="onboarding-foot">
    <button class="btn ghost sm" type="button" onclick={() => onBack?.()}>
      <ChevronLeft size={14} /> Back
    </button>
    <button class="btn primary" type="button" onclick={onNext} disabled={mode !== "ready"}>
      Next
      <ChevronRight size={14} />
    </button>
  </footer>
</div>

<style>
  .onboarding-step {
    display: flex; flex-direction: column; gap: 20px;
    max-width: 560px; margin: 56px auto; padding: 28px;
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
  .choice-grid { display: grid; gap: 10px; }
  .choice {
    display: flex; flex-direction: column; gap: 4px;
    padding: 14px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    transition: border-color 100ms ease, background 100ms ease;
  }
  .choice:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--bg-elev-3);
  }
  .choice strong { font-size: 13.5px; }
  .choice .hint { font-size: 12px; color: var(--fg-muted); line-height: 1.5; }
  .confirm-block { display: flex; flex-direction: column; gap: 12px; }
  .confirm-block p { margin: 0; font-size: 13px; color: var(--fg-2); line-height: 1.5; }
  .row { display: flex; gap: 8px; justify-content: flex-end; }
  .status-block { display: flex; flex-direction: column; gap: 10px; }
  .status-line { display: flex; align-items: center; gap: 6px; color: var(--success, oklch(0.78 0.16 145)); font-size: 13px; margin: 0; }
  .path-row { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--fg-muted); }
  .path-row code { font-family: var(--font-mono); }
  .next-step { font-size: 12.5px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .pub-key {
    width: 100%; resize: vertical;
    font-family: var(--font-mono); font-size: 11.5px;
    padding: 8px;
    background: var(--bg-elev-2);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .error-line { display: flex; align-items: center; gap: 6px; color: var(--danger); font-size: 12.5px; margin: 0; }
  code {
    font-family: var(--font-mono); font-size: 12px;
    padding: 1px 5px;
    background: var(--bg-elev-2);
    border-radius: var(--radius-xs);
  }
  .onboarding-foot {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px; padding-top: 16px;
    border-top: 1px solid var(--border);
  }
  :global(.spin) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
