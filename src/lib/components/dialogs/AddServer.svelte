<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    X, Server, ChevronLeft, ChevronRight, Check, Key, Terminal,
    AlertTriangle, ExternalLink,
  } from "lucide-svelte";
  import type { ServerProfile } from "../../state/connection.svelte";

  type Props = {
    open: boolean;
    editing: ServerProfile | null;
    onClose: (saved: ServerProfile | null) => void;
    onLaunchKeygen?: () => void;
  };

  let { open, editing, onClose, onLaunchKeygen }: Props = $props();

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

  let step = $state(0);
  const lastStep = 2;
  const stepLabels = ["Connection", "Workspace", "Bridge & save"];

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
      return;
    }
    name = ""; host = ""; port = "22"; user = "";
    keyPath = ""; remoteRoot = ""; localRoot = "";
    txAdminUrl = ""; bridgeToken = ""; bridgePort = "30121";
    // Async probe — cancel via the effect cleanup if the dialog closes
    // before the invokes resolve, so we don't stomp on a fresh dialog.
    let cancelled = false;
    void (async () => {
      try {
        const exists = await invoke<boolean>("default_ssh_key_exists");
        if (cancelled || !exists || keyPath) return;
        const path = await invoke<string | null>("default_ssh_key_path");
        if (cancelled) return;
        if (path) keyPath = path;
      } catch (e) { console.error("default ssh key path lookup failed", e); }
    })();
    return () => { cancelled = true; };
  });

  $effect(() => {
    if (!editing && !nameTouched && host.trim() && !name.trim()) {
      name = `Server (${host.trim()})`;
    }
  });

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
    const clamped = Math.max(0, Math.min(lastStep, target));
    if (clamped > step) {
      for (let i = step; i < clamped; i++) {
        const err = validateStep(i);
        if (err) { error = err; step = i; return; }
      }
    }
    error = "";
    step = clamped;
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
      const payload = {
        ...profile,
        bridgeToken: bridgeToken.trim() || editing?.bridgeToken || null,
        fingerprint: editing?.fingerprint || null,
        addedAt: editing?.addedAt || null,
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
    try { await openUrl(url); }
    catch (e) { error = "Could not open URL: " + String(e); }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget && !saving) onClose(null);
  }
  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape" && !saving) onClose(null);
  }

  const sshPreview = $derived.by(() => {
    const u = user.trim() || "user";
    const h = host.trim() || "host";
    const trimmedPort = port.trim();
    if (trimmedPort === "" || trimmedPort === "22") return `ssh ${u}@${h}`;
    const p = parseInt(trimmedPort, 10);
    if (!Number.isFinite(p) || p < 1 || p > 65535) return `ssh ${u}@${h} (invalid port)`;
    return `ssh ${u}@${h} -p ${p}`;
  });
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation">
    <div class="dialog-shell as-shell" role="dialog" aria-modal="true" aria-label={editing ? "Edit server" : "Add server"}>
      <div class="dialog-head">
        <div class="dialog-icon"><Server size={14}/></div>
        <div>
          <div class="dialog-title">{editing ? "Edit server" : "Add server"}</div>
          <div class="dialog-sub">Connect Rift to a remote FiveM dev server over SSH.</div>
        </div>
        <button class="dialog-close" type="button" onclick={() => !saving && onClose(null)} aria-label="Close">
          <X size={14}/>
        </button>
      </div>

      <div class="stepper">
        {#each stepLabels as label, i (label)}
          <button
            class="step"
            type="button"
            data-active={i === step}
            data-done={i < step}
            onclick={() => gotoStep(i)}
            disabled={i > step && !stepValid}
          >
            <span class="step-dot">{#if i < step}<Check size={10}/>{:else}{i + 1}{/if}</span>
            <span>{label}</span>
          </button>
          {#if i < stepLabels.length - 1}<span class="step-line"></span>{/if}
        {/each}
      </div>

      <div class="dialog-body as-body">
        {#if step === 0}
          <div class="field">
            <label class="field-label" for="as-name">Profile name <span class="field-sub">a short, memorable handle</span></label>
            <input id="as-name" class="input" type="text" bind:value={name} oninput={() => (nameTouched = true)} placeholder="(auto-suggests from host)"/>
          </div>
          <div class="row3">
            <div class="field">
              <label class="field-label" for="as-host">Host</label>
              <input id="as-host" class="input mono" type="text" bind:value={host} placeholder="10.0.4.18"/>
            </div>
            <div class="field narrow">
              <label class="field-label" for="as-port">Port</label>
              <input id="as-port" class="input mono" type="text" bind:value={port}/>
            </div>
            <div class="field">
              <label class="field-label" for="as-user">User</label>
              <input id="as-user" class="input mono" type="text" bind:value={user} placeholder="fivem"/>
            </div>
          </div>
          {#if (host.trim() || user.trim())}
            <div class="conn-preview">
              <Terminal size={11}/>
              <span class="mono">{sshPreview}</span>
            </div>
          {/if}
          <div class="field">
            <label class="field-label" for="as-key">
              Identity file <span class="field-sub">private key · ed25519 recommended</span>
            </label>
            <div class="input-row">
              <input id="as-key" class="input mono" type="text" bind:value={keyPath} placeholder="C:\Users\you\.ssh\id_ed25519"/>
              {#if onLaunchKeygen}
                <button class="btn" type="button" onclick={onLaunchKeygen}>
                  <Key size={11}/> Generate
                </button>
              {/if}
            </div>
          </div>
        {:else if step === 1}
          <div class="field">
            <label class="field-label" for="as-rroot">
              Remote root <span class="field-sub">must end at the FiveM <span class="mono">resources/</span> folder</span>
            </label>
            <input id="as-rroot" class="input mono" type="text" bind:value={remoteRoot} placeholder="/opt/fxserver/server/txData/<base>/resources"/>
          </div>
          <div class="field">
            <label class="field-label" for="as-lroot">
              Local root <span class="field-sub">the folder Rift watches on this machine</span>
            </label>
            <input id="as-lroot" class="input mono" type="text" bind:value={localRoot} placeholder="C:\dev\fivem\resources"/>
          </div>
          <div class="info-card">
            <div class="info-head">What is the remote root?</div>
            <p>Path on the server that holds your <span class="mono">[bracketed]/</span> resource folders. Rift will sync into and out of this directory. If you point it at the parent (e.g. <span class="mono">/opt/fxserver/server</span> instead of <span class="mono">…/resources</span>), drift scans will see thousands of unrelated files.</p>
          </div>
        {:else if step === 2}
          <div class="field">
            <label class="field-label" for="as-tx">
              txAdmin URL <span class="field-sub">optional — opens in browser for restart fallback</span>
            </label>
            <div class="input-row">
              <input id="as-tx" class="input mono" type="text" bind:value={txAdminUrl} placeholder="http://server.example:40120"/>
              <button class="btn" type="button" onclick={testTxAdmin} disabled={!txAdminUrl.trim()}>
                <ExternalLink size={11}/> Test
              </button>
            </div>
          </div>
          <div class="info-card">
            <div class="info-head">rift_bridge — auto-sync companion (optional)</div>
            <p>Install the <span class="mono">rift_bridge</span> resource on your FXServer to enable auto-sync + restart-on-save. Leave the token empty to disable.</p>
            <div class="field tight">
              <label class="field-label" for="as-btoken">Bridge token</label>
              <input id="as-btoken" class="input mono" type="text" bind:value={bridgeToken} placeholder={editing ? "(leave empty to keep existing)" : ""}/>
            </div>
            <div class="field tight">
              <label class="field-label" for="as-bport">Bridge port <span class="field-sub">default 30121</span></label>
              <input id="as-bport" class="input mono" type="text" bind:value={bridgePort}/>
            </div>
          </div>
          <div class="summary">
            <div class="summary-head">Summary</div>
            <div class="summary-grid mono">
              <span class="dim">name</span><span>{name.trim() || "—"}</span>
              <span class="dim">ssh</span><span>{user.trim() || "—"}@{host.trim() || "—"}:{port.trim()}</span>
              <span class="dim">remote</span><span>{remoteRoot.trim() || "—"}</span>
              <span class="dim">local</span><span>{localRoot.trim() || "—"}</span>
              <span class="dim">txAdmin</span><span>{txAdminUrl.trim() || "(none)"}</span>
              <span class="dim">bridge</span><span>{bridgeToken.trim() ? `enabled · port ${bridgePort.trim()}` : "(disabled)"}</span>
            </div>
          </div>
        {/if}
      </div>

      {#if error}
        <div class="error mono">
          <AlertTriangle size={11}/> {error}
        </div>
      {/if}

      <div class="dialog-foot">
        {#if step > 0}
          <button class="btn ghost" type="button" onclick={back} disabled={saving}>
            <ChevronLeft size={11}/> Back
          </button>
        {:else}
          <button class="btn ghost" type="button" onclick={() => onClose(null)} disabled={saving}>Cancel</button>
        {/if}
        <div class="dialog-foot-spacer"></div>
        {#if step > 0 && step < lastStep}
          <button class="btn ghost" type="button" onclick={() => onClose(null)} disabled={saving}>Cancel</button>
        {/if}
        {#if step < lastStep}
          <button class="btn primary" type="button" onclick={next} disabled={!stepValid}>
            Continue <ChevronRight size={11}/>
          </button>
        {:else}
          <button class="btn primary" type="button" onclick={save} disabled={!allValid || saving}>
            <Check size={11}/> {saving ? "Saving…" : (editing ? "Save changes" : "Save server")}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .as-shell { width: 640px; max-width: 92vw; }
  .as-body { display: flex; flex-direction: column; gap: 12px; }

  .step { /* override default button */
    background: transparent; border: 0; cursor: pointer;
    font: inherit;
  }
  .step:disabled { cursor: not-allowed; opacity: 0.5; }

  .row3 {
    display: grid;
    grid-template-columns: 1fr 88px 1fr;
    gap: 10px;
  }
  .field.tight { margin-bottom: 8px; }
  .field.narrow .input { text-align: center; }

  .conn-preview {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 6px 10px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--fg-2); font-size: var(--fs-xs);
    align-self: start;
  }

  .info-card {
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
  }
  .info-head { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); margin-bottom: 4px; }
  .info-card p {
    margin: 0 0 8px;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    line-height: 1.5;
  }
  .info-card p:last-of-type { margin-bottom: 0; }

  .summary {
    background: var(--bg-elev-2);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 14%, transparent);
  }
  .summary-head {
    font-size: var(--fs-xs); color: var(--fg-faint);
    text-transform: uppercase; letter-spacing: 0.06em;
    margin-bottom: 6px;
  }
  .summary-grid {
    display: grid;
    grid-template-columns: 70px 1fr;
    gap: 3px 10px;
    font-size: var(--fs-xs);
    color: var(--fg);
  }
  .summary-grid .dim { color: var(--fg-faint); }

  .error {
    display: inline-flex; align-items: center; gap: 6px;
    color: var(--danger);
    font-size: var(--fs-xs);
    padding: 0 14px 4px;
  }
</style>
