<script lang="ts">
  import { Upload, X } from "lucide-svelte";

  export type ReuploadChoice = "skip" | "reupload" | "always";

  type Props = {
    open: boolean;
    fileName: string;
    detail?: string;
    onClose: (choice: ReuploadChoice | null) => void;
  };

  let { open, fileName, detail = "", onClose }: Props = $props();

  let dontAsk = $state(false);

  function pick(choice: ReuploadChoice) {
    onClose(choice);
    dontAsk = false;
  }
  function dismiss() { onClose(null); dontAsk = false; }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) dismiss();
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") dismiss();
    if (e.key === "Enter") pick(dontAsk ? "always" : "reupload");
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation">
    <div class="dialog-shell" style="width: 460px;" role="dialog" aria-modal="true" aria-label="Re-upload after edit?">
      <div class="dialog-head">
        <div class="dialog-icon warn"><Upload size={14}/></div>
        <div>
          <div class="dialog-title">Re-upload after edit?</div>
          <div class="dialog-sub">External editor saved <span class="mono">{fileName}</span>.</div>
        </div>
        <button class="dialog-close" type="button" onclick={dismiss} aria-label="Close">
          <X size={14}/>
        </button>
      </div>
      <div class="dialog-body">
        <p class="lead">Rift downloaded this file when you opened it. The local copy is now newer.</p>
        {#if detail}<p class="detail mono">{detail}</p>{/if}
        <label class="dialog-checkbox">
          <input type="checkbox" bind:checked={dontAsk}/>
          <span>Always re-upload for this server</span>
        </label>
      </div>
      <div class="dialog-foot">
        <button class="btn ghost" type="button" onclick={() => pick("skip")}>Skip</button>
        <div class="dialog-foot-spacer"></div>
        <button class="btn primary" type="button" onclick={() => pick(dontAsk ? "always" : "reupload")}>
          <Upload size={11}/> Re-upload
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .lead { color: var(--fg-2); margin: 0 0 8px; font-size: var(--fs-sm); line-height: 1.5; }
  .detail {
    color: var(--fg-subtle); font-size: var(--fs-xs);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 8px;
    word-break: break-all;
    margin: 0 0 12px;
  }
</style>
