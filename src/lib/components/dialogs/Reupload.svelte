<script lang="ts">
  export type ReuploadChoice = "skip" | "reupload" | "always";

  type Props = {
    open: boolean;
    fileName: string;
    detail?: string;
    onClose: (choice: ReuploadChoice | null) => void;
  };

  let { open, fileName, detail = "", onClose }: Props = $props();

  function pick(choice: ReuploadChoice) { onClose(choice); }
  function dismiss() { onClose(null); }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) dismiss();
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") dismiss();
    if (e.key === "Enter") pick("reupload");
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="backdrop" onclick={onBackdrop} role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" aria-label="File saved — re-upload?">
      <header><h2>File saved — re-upload?</h2></header>
      <p class="body">'{fileName}' was modified. Push the changes back to the server?</p>
      {#if detail}<p class="detail">{detail}</p>{/if}
      <footer>
        <button class="skip" onclick={() => pick("skip")} type="button">Skip</button>
        <button class="always" onclick={() => pick("always")} type="button">Always</button>
        <button class="reupload" onclick={() => pick("reupload")} type="button">Re-upload</button>
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
    width: 440px; max-width: 92vw;
    box-shadow: 0 18px 50px rgba(0,0,0,0.6);
    padding: 18px 20px;
    color: #E8E8EE;
  }
  header { margin-bottom: 12px; }
  h2 { margin: 0; font-size: 14px; font-weight: 600; }
  .body { font-size: 13px; line-height: 1.5; margin: 0 0 8px; }
  .detail {
    font-family: Consolas, monospace; font-size: 11px;
    color: #7A7A85; margin: 0; word-break: break-all;
  }
  footer {
    display: flex; justify-content: flex-end; gap: 6px;
    margin-top: 16px;
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
  button:hover { background: #26262E; }
  button.reupload { color: #8B6BE6; border-color: #3A2A66; font-weight: 600; }
  button.reupload:hover { background: #15101E; }
</style>
