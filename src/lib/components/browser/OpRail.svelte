<script lang="ts">
  import { ArrowRight, ArrowLeft, RefreshCw } from "lucide-svelte";

  type Props = {
    canUpload: boolean;
    canDownload: boolean;
    syncing: boolean;
    onUpload: () => void;
    onDownload: () => void;
    onSync: () => void;
  };
  let { canUpload, canDownload, syncing, onUpload, onDownload, onSync }: Props = $props();
</script>

<div class="oprail">
  <div class="grp">
    <button class="op" disabled={!canUpload} onclick={onUpload} title="Upload local → remote" aria-label="Upload" type="button">
      <ArrowRight size={14}/>
    </button>
    <button class="op" disabled={!canDownload} onclick={onDownload} title="Download remote → local" aria-label="Download" type="button">
      <ArrowLeft size={14}/>
    </button>
  </div>
  <div class="grp">
    <button
      class="op accent"
      class:spinning={syncing}
      disabled={syncing}
      onclick={onSync}
      title="Reconcile both sides — compare local vs remote, find conflicts"
      aria-label="Reconcile"
      type="button"
    >
      <RefreshCw size={14}/>
    </button>
  </div>
</div>

<style>
  .oprail {
    display: flex; flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
    background: var(--bg);
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
    width: 36px;
  }
  .grp { display: flex; flex-direction: column; gap: 4px; align-items: center; }

  .op {
    width: 28px; height: 28px;
    display: inline-flex; align-items: center; justify-content: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    cursor: pointer;
    transition: background 80ms ease, color 80ms ease, border-color 80ms ease;
  }
  .op:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--fg);
    border-color: var(--border);
  }
  .op.accent { color: var(--accent); }
  .op.accent:hover:not(:disabled) {
    background: var(--accent-soft);
    border-color: color-mix(in oklch, var(--accent) 40%, transparent);
  }
  .op:disabled { opacity: 0.35; cursor: not-allowed; }
  .op:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }

  .op.spinning :global(svg) { animation: spin 900ms linear infinite; }
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
