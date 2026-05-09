<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type KeyPaths = {
    privatePath: string;
    publicPath: string;
    publicKeyText: string;
  };

  type Props = {
    open: boolean;
    onClose: (keyPath: string | null) => void;
  };

  let { open, onClose }: Props = $props();

  let exists = $state(false);
  let pubKey = $state("");
  let keyPath = $state("");
  let intro = $state("");
  let error = $state("");
  let busy = $state(false);
  let copied = $state(false);

  $effect(() => {
    if (open) refresh();
  });

  async function refresh() {
    error = "";
    copied = false;
    try {
      exists = await invoke<boolean>("default_ssh_key_exists");
      if (exists) {
        const pub = await invoke<string | null>("read_default_ssh_pub_key");
        pubKey = pub ?? "";
        intro = "An ed25519 key already exists at ~/.ssh/id_ed25519. Send the public key below to your server admin so they can add it to the server's authorized_keys.";
        keyPath = "~/.ssh/id_ed25519";
      } else {
        pubKey = "";
        intro = "No SSH key found at ~/.ssh/id_ed25519. Click Generate to create an ed25519 keypair (no passphrase). The public key will appear below — send it to your server admin to add to authorized_keys.";
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function generate() {
    busy = true;
    error = "";
    try {
      const result = await invoke<KeyPaths>("generate_default_ssh_key", { comment: null });
      pubKey = result.publicKeyText;
      keyPath = result.privatePath;
      exists = true;
      intro = `Generated key at ${result.privatePath}. Send the public key to your admin.`;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function copy() {
    if (!pubKey) return;
    try {
      await navigator.clipboard.writeText(pubKey);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      error = "Could not access clipboard.";
    }
  }

  function done() {
    onClose(keyPath || null);
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose(null);
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") onClose(null);
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="backdrop" onclick={onBackdrop} role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" aria-label="SSH key setup">
      <header>
        <h2>SSH key setup</h2>
      </header>

      <p class="intro">{intro}</p>

      <div class="pubkey-box">
        <div class="label">{exists ? "Your public key — send this to your admin" : "Your public key (will appear after generation)"}</div>
        <textarea readonly value={pubKey} placeholder="ssh-ed25519 …" rows="5"></textarea>
      </div>

      {#if error}<p class="error">{error}</p>{/if}

      <footer>
        <button class="generate" onclick={generate} disabled={exists || busy} type="button">
          {exists ? "Already have a key" : busy ? "Generating…" : "Generate ed25519 key"}
        </button>
        <button class="copy" onclick={copy} disabled={!pubKey} type="button">
          {copied ? "Copied ✓" : "Copy pubkey"}
        </button>
        <button class="done" onclick={done} type="button">Done</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 200;
  }
  .dialog {
    background: #17171C;
    border: 1px solid #3A2A66;
    border-radius: 6px;
    width: 560px; max-width: 92vw;
    box-shadow: 0 18px 50px rgba(0,0,0,0.6);
    padding: 18px 20px;
    color: #E8E8EE;
    display: flex; flex-direction: column; gap: 12px;
  }
  h2 { margin: 0; font-size: 14px; font-weight: 600; }
  .intro { font-size: 12px; color: #7A7A85; line-height: 1.5; margin: 0; }
  .pubkey-box {
    background: #0F0F12;
    border: 1px solid #26262E;
    border-radius: 4px;
    padding: 10px;
  }
  .label { font-size: 11px; color: #7A7A85; margin-bottom: 6px; }
  textarea {
    width: 100%;
    background: transparent;
    border: 0;
    color: #E8E8EE;
    font-family: Consolas, monospace;
    font-size: 11px;
    resize: vertical;
    outline: none;
    padding: 0;
  }
  .error { color: #FF5C6B; font-size: 11px; margin: 0; }
  footer {
    display: flex; justify-content: flex-end; gap: 6px;
    margin-top: 4px;
  }
  button {
    background: #1F1F26;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  button:hover:not(:disabled) { background: #26262E; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
  button.done { color: #8B6BE6; border-color: #3A2A66; }
</style>
