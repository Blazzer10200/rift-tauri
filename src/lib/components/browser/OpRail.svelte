<script lang="ts">
  import { ArrowRight, ArrowLeft, RefreshCw, FileEdit, FileCode, Trash2 } from "lucide-svelte";

  type Props = {
    canUpload: boolean;
    canDownload: boolean;
    canEdit: boolean;
    canDelete: boolean;
    onUpload: () => void;
    onDownload: () => void;
    onSync: () => void;
    onEdit: () => void;
    onDiff: () => void;
    onDelete: () => void;
  };
  let {
    canUpload, canDownload, canEdit, canDelete,
    onUpload, onDownload, onSync, onEdit, onDiff, onDelete,
  }: Props = $props();
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
    <button class="op accent" onclick={onSync} title="Sync both sides" aria-label="Sync" type="button">
      <RefreshCw size={14}/>
    </button>
  </div>
  <div class="grp">
    <button class="op" disabled={!canEdit} onclick={onEdit} title="Open in editor" aria-label="Edit" type="button">
      <FileEdit size={13}/>
    </button>
    <button class="op" disabled={!canEdit} onclick={onDiff} title="Diff" aria-label="Diff" type="button">
      <FileCode size={13}/>
    </button>
  </div>
  <div class="grp foot">
    <button class="op danger" disabled={!canDelete} onclick={onDelete} title="Delete" aria-label="Delete" type="button">
      <Trash2 size={13}/>
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
  .grp.foot { margin-top: auto; }

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
  .op.danger:hover:not(:disabled) {
    background: var(--danger-soft);
    color: var(--danger);
    border-color: color-mix(in oklch, var(--danger) 40%, transparent);
  }
  .op:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .op:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
</style>
