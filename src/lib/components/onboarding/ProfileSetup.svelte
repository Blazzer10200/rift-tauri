<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, ChevronRight, ChevronLeft, AlertCircle } from "lucide-svelte";

  type Props = {
    onNext: (localRoot: string) => void;
    onBack?: () => void;
  };
  const { onNext, onBack }: Props = $props();

  let localRoot = $state<string>("");
  let error = $state<string | null>(null);

  async function pickFolder() {
    error = null;
    try {
      const picked = await openDialog({
        title: "Pick the local workspace folder",
        multiple: false,
        directory: true,
      });
      if (typeof picked === "string") localRoot = picked;
    } catch (e) {
      error = String(e);
    }
  }

  function next() {
    if (!localRoot.trim()) {
      error = "Pick a folder before continuing.";
      return;
    }
    onNext(localRoot.trim());
  }
</script>

<div class="onboarding-step">
  <header class="onboarding-head">
    <h2 class="onboarding-title">Local workspace</h2>
    <p class="onboarding-sub">
      Pick a folder on this machine where Rift will mirror your remote
      resources. The folder doesn't need to be empty — Rift's first scan
      reconciles against the remote tree.
    </p>
  </header>

  <div class="path-row">
    <input
      class="input mono"
      type="text"
      placeholder="C:\\Projects\\my-fivem-server"
      bind:value={localRoot}
    />
    <button class="btn sm" type="button" onclick={pickFolder}>
      <FolderOpen size={12} />
      Browse…
    </button>
  </div>

  <p class="hint">
    Tip: put this on a fast local SSD. Avoid syncing it via OneDrive / Dropbox
    — those tools collide with Rift's atomic-rename pattern and produce
    phantom drift.
  </p>

  {#if error}
    <p class="error-line"><AlertCircle size={14} /> {error}</p>
  {/if}

  <footer class="onboarding-foot">
    <button class="btn ghost sm" type="button" onclick={() => onBack?.()}>
      <ChevronLeft size={14} /> Back
    </button>
    <button class="btn primary" type="button" onclick={next}>
      Next
      <ChevronRight size={14} />
    </button>
  </footer>
</div>

<style>
  .onboarding-step {
    display: flex; flex-direction: column; gap: 16px;
    max-width: 560px; margin: 56px auto; padding: 28px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .onboarding-head { display: flex; flex-direction: column; gap: 6px; }
  .onboarding-title { font-size: 18px; font-weight: 600; color: var(--fg); margin: 0; }
  .onboarding-sub { font-size: 13px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .path-row { display: flex; gap: 8px; align-items: center; }
  .path-row .input { flex: 1; }
  .hint { font-size: 12px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .error-line { display: flex; align-items: center; gap: 6px; color: var(--danger); font-size: 12.5px; margin: 0; }
  .onboarding-foot {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px; padding-top: 16px;
    border-top: 1px solid var(--border);
  }
</style>
