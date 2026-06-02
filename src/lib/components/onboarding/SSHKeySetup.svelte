<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { KeyRound, FolderUp, CheckCircle2, AlertCircle, Loader2, Copy, FolderOpen } from "lucide-svelte";
  import ObStage from "./ObStage.svelte";

  type Props = { onReady: (ready: boolean) => void };
  const { onReady }: Props = $props();

  type Mode = "choose" | "generate" | "import" | "ready";
  let mode = $state<Mode>("choose");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let keyPath = $state<string | null>(null);
  let pubKey = $state<string | null>(null);
  let copied = $state(false);

  // Gate the shell's Next on a usable key.
  $effect(() => { onReady(mode === "ready"); });

  onMount(async () => {
    try {
      const exists = await invoke<boolean>("default_ssh_key_exists");
      if (exists) {
        keyPath = await invoke<string>("default_ssh_key_path");
        pubKey = await invoke<string>("read_default_ssh_pub_key");
        mode = "ready";
      }
    } catch (e) {
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
    const picked = await openDialog({ title: "Pick existing SSH private key", multiple: false, directory: false });
    if (typeof picked !== "string") return;
    busy = true;
    try {
      await invoke("validate_ssh_key_file", { path: picked });
      keyPath = picked;
      pubKey = null;
      mode = "ready";
    } catch (e) {
      error = `Import failed: ${e}`;
    } finally {
      busy = false;
    }
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

<ObStage kind="ssh" caption="keypair → authorized_keys" />

<header class="ob-head">
  <span class="ob-eyebrow">Step 1 · Authentication</span>
  <h1 class="ob-title">Set up your SSH key</h1>
  <p class="ob-sub">Rift authenticates SFTP with an SSH key — no passwords stored anywhere.</p>
</header>

{#if mode === "choose"}
  <div class="ob-choices">
    <button class="ob-choice" type="button" onclick={() => (mode = "generate")} disabled={busy}>
      <span class="ob-choice-ic"><KeyRound size={19} /></span>
      <span class="ob-choice-t">Generate a new key</span>
      <span class="ob-choice-p">Creates <code class="ob-mono-i">~/.ssh/id_rift_ed25519</code>. You'll paste the public half into your server.</span>
    </button>
    <button class="ob-choice" type="button" onclick={onImport} disabled={busy}>
      <span class="ob-choice-ic"><FolderUp size={19} /></span>
      <span class="ob-choice-t">Import existing key</span>
      <span class="ob-choice-p">Pick a private key you already use — Rift just remembers the path.</span>
    </button>
  </div>
{/if}

{#if mode === "generate"}
  <div class="ob-confirm">
    <p class="ob-confirm-line">About to run a one-shot keygen — an existing key at this path is never overwritten.</p>
    <div class="ob-cmd">ssh-keygen -t ed25519 -f ~/.ssh/id_rift_ed25519</div>
    <div class="ob-input-row">
      <button class="ob-btn ghost sm" type="button" onclick={() => (mode = "choose")} disabled={busy}>Back to choices</button>
      <button class="ob-btn accent sm" type="button" onclick={onGenerate} disabled={busy}>
        {#if busy}<Loader2 size={14} class="spin" />{/if}
        {busy ? "Generating…" : "Generate now"}
      </button>
    </div>
  </div>
{/if}

{#if mode === "ready"}
  <div class="ob-ready">
    <span class="ob-status ok"><CheckCircle2 size={16} /> Key ready</span>
    {#if keyPath}
      <span class="ob-pathrow"><FolderOpen size={13} /> <code>{keyPath}</code></span>
    {/if}
    {#if pubKey}
      <p class="ob-hint"><span>Add the public half to <code class="ob-mono-i">~/.ssh/authorized_keys</code> on your game server (one line).</span></p>
      <div class="ob-keybox">{pubKey}
        <button class="ob-keycopy" class:copied type="button" onclick={onCopyPub}>
          <Copy size={11} /> {copied ? "Copied" : "Copy"}
        </button>
      </div>
    {/if}
  </div>
{/if}

{#if error}
  <span class="ob-error"><AlertCircle size={14} /> {error}</span>
{/if}
