<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Key, X, Copy, RefreshCw, Check, AlertTriangle } from "lucide-svelte";

  import { tooltip } from "$lib/actions/tooltip";
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
        intro = "An ed25519 key already exists. Send the public key below to your server admin so they can add it to authorized_keys.";
        keyPath = "~/.ssh/id_ed25519";
      } else {
        pubKey = "";
        intro = "No SSH key found at ~/.ssh/id_ed25519. Generate creates an ed25519 keypair (no passphrase) — the public key shows below to send to your admin.";
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
      intro = "Generated. Send the public key to your admin.";
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
    } catch {
      error = "Could not access clipboard.";
    }
  }

  function done() { onClose(keyPath || null); }
  function onBackdrop(e: MouseEvent) { if (e.target === e.currentTarget) onClose(null); }
  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") onClose(null);
  }

  // Compute fingerprint preview (first/last chunks of pubkey)
  const fingerprint = $derived.by(() => {
    if (!pubKey) return "";
    const parts = pubKey.trim().split(/\s+/);
    return parts.length >= 2 ? parts[1].slice(0, 12) + "…" + parts[1].slice(-12) : "";
  });
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation">
    <div class="dialog-shell" style="width: 560px;" role="dialog" aria-modal="true" aria-label="SSH key setup">
      <div class="dialog-head">
        <div class="dialog-icon"><Key size={14}/></div>
        <div>
          <div class="dialog-title">SSH keypair</div>
          <div class="dialog-sub">ed25519 · 256-bit{exists ? " · exists" : " · not yet generated"}</div>
        </div>
        <button class="dialog-close" type="button" onclick={() => onClose(null)} aria-label="Close">
          <X size={14}/>
        </button>
      </div>

      <div class="dialog-body">
        <p class="intro">{intro}</p>

        <div class="field">
          <label class="field-label" for="kg-pub">
            Public key <span class="field-sub">paste this into <span class="mono">~/.ssh/authorized_keys</span> on the remote</span>
          </label>
          <textarea
            id="kg-pub"
            class="key-blob mono"
            readonly
            value={pubKey}
            placeholder="ssh-ed25519 …"
            rows="4"
          ></textarea>
        </div>

        {#if fingerprint}
          <div class="field">
            <label class="field-label" for="kg-fp">Fingerprint</label>
            <div id="kg-fp" class="mono fp">{fingerprint}</div>
          </div>
        {/if}

        <div class="field">
          <label class="field-label" for="kg-loc">Private key location</label>
          <div id="kg-loc" class="mono dim">{keyPath || "~/.ssh/id_ed25519"}</div>
        </div>

        {#if error}
          <div class="error mono">
            <AlertTriangle size={11}/> {error}
          </div>
        {/if}
      </div>

      <div class="dialog-foot">
        <button class="btn ghost" type="button" onclick={done}>Close</button>
        <div class="dialog-foot-spacer"></div>
        <button
          class="btn"
          type="button"
          onclick={generate}
          disabled={exists || busy}
          use:tooltip={exists ? "Key already exists" : "Generate ed25519 keypair"}
        >
          <RefreshCw size={11} class={busy ? "spin" : ""}/>
          {busy ? "Generating…" : exists ? "Exists" : "Generate"}
        </button>
        <button
          class="btn primary"
          type="button"
          onclick={copy}
          disabled={!pubKey}
        >
          {#if copied}<Check size={11}/> Copied{:else}<Copy size={11}/> Copy public key{/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .intro { color: var(--fg-2); font-size: var(--fs-sm); line-height: 1.5; margin: 0 0 12px; }
  .key-blob {
    width: 100%;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    color: var(--fg);
    font-size: var(--fs-xs);
    resize: vertical;
    outline: none;
  }
  .key-blob:focus { border-color: var(--accent); }
  .fp { color: var(--fg-2); font-size: var(--fs-sm); }
  .error {
    display: inline-flex; align-items: center; gap: 6px;
    color: var(--danger);
    font-size: var(--fs-xs);
    margin-top: 8px;
  }
</style>
