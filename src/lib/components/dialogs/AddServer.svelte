<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { ServerProfile } from "../../state/connection.svelte";

  type Props = {
    open: boolean;
    editing: ServerProfile | null;
    onClose: (saved: ServerProfile | null) => void;
    onLaunchKeygen?: () => void;
  };

  let { open, editing, onClose, onLaunchKeygen }: Props = $props();

  // ── form state ─────────────────────────────────────────────────────
  let name = $state("");
  let host = $state("");
  let port = $state("22");
  let user = $state("");
  let keyPath = $state("");
  let remoteRoot = $state("");
  let localRoot = $state("");
  let txAdminUrl = $state("");
  let bridgeToken = $state("");
  let bridgePort = $state("30121");
  let nameTouched = $state(false);

  // Stepper: 0=Connection, 1=Workspace, 2=Bridge & Save.
  let step = $state(0);
  const lastStep = 2;

  let error = $state("");
  let saving = $state(false);

  $effect(() => {
    if (!open) return;
    step = 0;
    error = "";
    nameTouched = false;
    if (editing) {
      name = editing.name ?? "";
      host = editing.host ?? "";
      port = String(editing.port ?? 22);
      user = editing.user ?? "";
      keyPath = editing.keyPath ?? "";
      remoteRoot = editing.remoteRoot ?? "";
      localRoot = editing.localRoot ?? "";
      txAdminUrl = editing.txAdminUrl ?? "";
      bridgeToken = "";
      bridgePort = String(editing.bridgePort ?? 30121);
      nameTouched = true;
    } else {
      name = ""; host = ""; port = "22"; user = "";
      keyPath = ""; remoteRoot = ""; localRoot = "";
      txAdminUrl = ""; bridgeToken = ""; bridgePort = "30121";
      // Auto-fill ~/.ssh/id_ed25519 if it exists
      void (async () => {
        try {
          const exists = await invoke<boolean>("default_ssh_key_exists");
          if (exists && !keyPath) {
            const home = (window as { __HOME__?: string }).__HOME__ ?? "~";
            keyPath = `${home}/.ssh/id_ed25519`;
          }
        } catch {}
      })();
    }
  });

  // Auto-suggest display name from host (Add flow only).
  $effect(() => {
    if (!editing && !nameTouched && host.trim() && !name.trim()) {
      name = `Server (${host.trim()})`;
    }
  });

  // ── validation per step ────────────────────────────────────────────
  function validateStep(s: number): string | null {
    if (s === 0) {
      if (!name.trim()) return "Display name is required.";
      if (!host.trim()) return "Host is required.";
      const p = parseInt(port.trim(), 10);
      if (!Number.isFinite(p) || p < 1 || p > 65535) return "Port must be 1-65535.";
      if (!user.trim()) return "User is required.";
      if (!keyPath.trim()) return "SSH key path is required.";
    } else if (s === 1) {
      if (!remoteRoot.trim()) return "Remote root is required.";
      if (!localRoot.trim()) return "Local root is required.";
    } else if (s === 2) {
      const bp = bridgePort.trim();
      if (bp) {
        const n = parseInt(bp, 10);
        if (!Number.isFinite(n) || n < 1 || n > 65535) return "Bridge port must be 1-65535.";
      }
    }
    return null;
  }

  const stepValid = $derived(validateStep(step) === null);
  const allValid = $derived(
    validateStep(0) === null && validateStep(1) === null && validateStep(2) === null,
  );

  function next() {
    const err = validateStep(step);
    if (err) { error = err; return; }
    error = "";
    if (step < lastStep) step += 1;
  }

  function back() {
    error = "";
    if (step > 0) step -= 1;
  }

  function gotoStep(target: number) {
    error = "";
    step = Math.max(0, Math.min(lastStep, target));
  }

  async function save() {
    for (let i = 0; i <= lastStep; i++) {
      const err = validateStep(i);
      if (err) { gotoStep(i); error = err; return; }
    }
    saving = true;
    error = "";
    try {
      const bp = parseInt(bridgePort.trim(), 10);
      const profile: ServerProfile = {
        key: editing?.key ?? "",
        name: name.trim(),
        host: host.trim(),
        port: parseInt(port.trim(), 10),
        user: user.trim(),
        keyPath: keyPath.trim(),
        remoteRoot: remoteRoot.trim(),
        localRoot: localRoot.trim(),
        txAdminUrl: txAdminUrl.trim() || null,
        bridgePort: Number.isFinite(bp) ? bp : null,
      };
      // Pass-through extra fields the backend cares about (not in ServerProfile type)
      const payload = {
        ...profile,
        bridgeToken: bridgeToken.trim() || (editing as unknown as { bridgeToken?: string })?.bridgeToken || null,
        fingerprint: (editing as unknown as { fingerprint?: string })?.fingerprint || null,
        addedAt: (editing as unknown as { addedAt?: string })?.addedAt || null,
      };
      const saved = await invoke<ServerProfile>("save_server", {
        profile: payload,
        editKey: editing?.key ?? null,
      });
      onClose(saved);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function testTxAdmin() {
    let url = txAdminUrl.trim();
    if (!url) { error = "Enter a txAdmin URL first."; return; }
    if (!/^https?:\/\//i.test(url)) {
      url = "http://" + url;
      txAdminUrl = url;
    }
    try {
      await openUrl(url);
    } catch (e) {
      error = "Could not open URL: " + String(e);
    }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget && !saving) onClose(null);
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape" && !saving) onClose(null);
  }

  const summary = $derived.by(() => {
    const tx = txAdminUrl.trim() || "(none)";
    const bridge = bridgeToken.trim() || (editing as unknown as { bridgeToken?: string })?.bridgeToken
      ? "enabled"
      : "(disabled)";
    return [
      `${name.trim() || "(unnamed)"}`,
      `  ssh://${user.trim()}@${host.trim()}:${port.trim()}`,
      `  remote → ${remoteRoot.trim()}`,
      `  local  → ${localRoot.trim()}`,
      `  txAdmin: ${tx}`,
      `  bridge:  ${bridge}`,
    ].join("\n");
  });
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="backdrop" onclick={onBackdrop} role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" aria-label={editing ? "Edit server" : "Add server"}>
      <header>
        <h2>{editing ? "Edit server" : "Add server"}</h2>
      </header>

      <!-- Stepper -->
      <div class="stepper">
        {#each [
          { label: "Connection", n: 1 },
          { label: "Workspace",  n: 2 },
          { label: "Bridge & Save", n: 3 },
        ] as s, i (s.n)}
          <div class="step" class:active={step === i} class:done={step > i}>
            <span class="pip">{s.n}</span>
            <span class="step-label">{s.label}</span>
          </div>
          {#if i < 2}<span class="step-bar"></span>{/if}
        {/each}
      </div>

      <div class="content">
        {#if step === 0}
          <div class="field">
            <label for="name">Display name *</label>
            <input id="name" type="text" bind:value={name} oninput={() => (nameTouched = true)} placeholder="(auto-suggests from host)" />
          </div>
          <div class="row-2">
            <div class="field grow">
              <label for="host">Host *</label>
              <input id="host" type="text" bind:value={host} />
            </div>
            <div class="field narrow">
              <label for="port">Port *</label>
              <input id="port" type="text" bind:value={port} />
            </div>
          </div>
          <div class="field">
            <label for="user">User *</label>
            <input id="user" type="text" bind:value={user} placeholder="ssh username (per-user account, not a shared service account)" />
          </div>
          <div class="field">
            <label for="keypath">SSH private key *</label>
            <div class="row-with-button">
              <input id="keypath" type="text" bind:value={keyPath} placeholder="C:\Users\you\.ssh\id_ed25519" />
              {#if onLaunchKeygen}
                <button type="button" class="aux" onclick={onLaunchKeygen}>Setup key</button>
              {/if}
            </div>
          </div>
        {:else if step === 1}
          <div class="field">
            <label for="rroot">Remote root * — must end at the FiveM resources folder</label>
            <input id="rroot" type="text" bind:value={remoteRoot} placeholder="/opt/fxserver/server/txData/<base>/resources" />
          </div>
          <div class="field">
            <label for="lroot">Local root *</label>
            <input id="lroot" type="text" bind:value={localRoot} placeholder="local FiveM server resources folder" />
          </div>
          <div class="info-card">
            <h3>What is the remote root?</h3>
            <p>Path on the server that holds your <code>[resources]/</code> folders. Rift will sync into and out of this directory. If you point it at the parent (e.g. <code>/opt/fxserver/server</code> instead of <code>…/resources</code>), drift scans will see thousands of unrelated files.</p>
          </div>
        {:else if step === 2}
          <div class="field">
            <label for="tx">txAdmin URL (optional, opens in browser for restart fallback)</label>
            <div class="row-with-button">
              <input id="tx" type="text" bind:value={txAdminUrl} placeholder="http://server.example:40120" />
              <button type="button" class="aux" onclick={testTxAdmin}>Test</button>
            </div>
          </div>
          <div class="info-card">
            <h3>rift_bridge — auto-sync companion (optional)</h3>
            <p>Install the rift_bridge resource on your FXServer to enable auto-sync + restart-on-save. Leave token empty to disable.</p>
            <div class="field">
              <label for="btoken">Bridge token</label>
              <input id="btoken" type="text" bind:value={bridgeToken} placeholder={editing ? "(leave empty to keep existing)" : ""} />
            </div>
            <div class="field">
              <label for="bport">Bridge port (default 30121)</label>
              <input id="bport" type="text" bind:value={bridgePort} />
            </div>
          </div>
          <div class="summary-card">
            <h3>Ready to save</h3>
            <pre>{summary}</pre>
          </div>
        {/if}
      </div>

      {#if error}<p class="error">{error}</p>{/if}

      <footer>
        <div>
          {#if step > 0}
            <button class="cancel" onclick={back} type="button" disabled={saving}>Back</button>
          {/if}
        </div>
        <div class="footer-right">
          <button class="cancel" onclick={() => onClose(null)} type="button" disabled={saving}>Cancel</button>
          {#if step < lastStep}
            <button class="primary" onclick={next} type="button" disabled={!stepValid}>Continue</button>
          {:else}
            <button class="primary" onclick={save} type="button" disabled={!allValid || saving}>
              {saving ? "Saving…" : (editing ? "Save changes" : "Save")}
            </button>
          {/if}
        </div>
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
    width: 640px; max-width: 92vw;
    max-height: 92vh;
    box-shadow: 0 18px 50px rgba(0,0,0,0.6);
    color: #E8E8EE;
    display: flex; flex-direction: column;
  }
  header { padding: 16px 20px; border-bottom: 1px solid #26262E; }
  h2 { margin: 0; font-size: 14px; font-weight: 600; }

  .stepper {
    display: flex; align-items: center; justify-content: center;
    padding: 14px 20px; gap: 8px;
    border-bottom: 1px solid #26262E;
  }
  .step { display: flex; align-items: center; gap: 8px; }
  .pip {
    width: 22px; height: 22px;
    border-radius: 11px;
    background: #1F1F26;
    border: 1px solid #26262E;
    display: flex; align-items: center; justify-content: center;
    font-size: 11px; font-weight: 600;
    color: #7A7A85;
  }
  .step.done .pip { border-color: #3A2A66; color: #8B6BE6; background: #1F1F26; }
  .step.active .pip { background: #3A2A66; border-color: #3A2A66; color: white; }
  .step-label { font-size: 12px; color: #7A7A85; }
  .step.active .step-label { color: #E8E8EE; font-weight: 600; }
  .step.done .step-label { color: #E8E8EE; }
  .step-bar { width: 36px; height: 1px; background: #26262E; }

  .content {
    padding: 20px;
    overflow: auto;
    flex: 1;
    display: flex; flex-direction: column; gap: 14px;
  }
  .field { display: flex; flex-direction: column; gap: 4px; }
  .field label { font-size: 11px; color: #7A7A85; }
  .field input {
    background: #0F0F12;
    border: 1px solid #26262E;
    border-radius: 4px;
    color: #E8E8EE;
    padding: 7px 9px;
    font-size: 13px;
    outline: none;
  }
  .field input:focus { border-color: #3A2A66; }
  .row-2 { display: flex; gap: 12px; }
  .grow { flex: 1; }
  .narrow { width: 100px; }
  .row-with-button { display: flex; gap: 8px; }
  .row-with-button input { flex: 1; }
  button.aux {
    background: #1F1F26;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  button.aux:hover { background: #26262E; }

  .info-card {
    background: #0F0F12;
    border: 1px solid #26262E;
    border-radius: 4px;
    padding: 12px;
    display: flex; flex-direction: column; gap: 8px;
  }
  .info-card h3 { margin: 0; font-size: 12px; font-weight: 600; }
  .info-card p { margin: 0; font-size: 11px; color: #7A7A85; line-height: 1.5; }
  .info-card code { font-family: Consolas, monospace; color: #8B6BE6; font-size: 11px; }
  .summary-card {
    background: #15101E;
    border: 1px solid #3A2A66;
    border-radius: 4px;
    padding: 12px;
  }
  .summary-card h3 { margin: 0 0 6px; font-size: 11px; font-weight: 600; }
  .summary-card pre {
    margin: 0;
    font-family: Consolas, monospace;
    font-size: 11px;
    color: #7A7A85;
    white-space: pre-wrap;
  }

  .error {
    color: #FF5C6B;
    font-size: 12px;
    margin: 0;
    padding: 0 20px 8px;
  }

  footer {
    display: flex; justify-content: space-between; align-items: center;
    padding: 14px 20px;
    border-top: 1px solid #26262E;
  }
  .footer-right { display: flex; gap: 8px; }
  button {
    background: #1F1F26;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 7px 16px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  button:hover:not(:disabled) { background: #26262E; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
  button.primary { background: #3A2A66; border-color: #3A2A66; color: white; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: #4A3678; }
</style>
