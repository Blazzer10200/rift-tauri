<script lang="ts">
  type Props = {
    open: boolean;
    title: string;
    body: string;
    confirmLabel?: string;
    cancelLabel?: string;
    isDanger?: boolean;
    showDontAsk?: boolean;
    onClose: (result: { confirmed: boolean; dontAsk: boolean }) => void;
  };

  let {
    open,
    title,
    body,
    confirmLabel = "Confirm",
    cancelLabel = "Cancel",
    isDanger = false,
    showDontAsk = false,
    onClose,
  }: Props = $props();

  let dontAsk = $state(false);

  function decide(confirmed: boolean) {
    onClose({ confirmed, dontAsk });
    dontAsk = false;
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) decide(false);
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") decide(false);
    if (e.key === "Enter") decide(true);
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="backdrop" onclick={onBackdrop} role="presentation">
    <div class="dialog" role="alertdialog" aria-modal="true" aria-label={title}>
      <header><h2 class:danger={isDanger}>{title}</h2></header>
      <p class="body">{body}</p>
      {#if showDontAsk}
        <label class="dont-ask">
          <input type="checkbox" bind:checked={dontAsk} />
          Don't ask again
        </label>
      {/if}
      <footer>
        <button class="cancel" onclick={() => decide(false)} type="button">{cancelLabel}</button>
        <button
          class="confirm"
          class:danger={isDanger}
          onclick={() => decide(true)}
          type="button"
        >{confirmLabel}</button>
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
    width: 420px; max-width: 92vw;
    box-shadow: 0 18px 50px rgba(0,0,0,0.6);
    padding: 18px 20px;
    color: #E8E8EE;
  }
  header { margin-bottom: 12px; }
  h2 { margin: 0; font-size: 14px; font-weight: 600; color: #E8E8EE; }
  h2.danger { color: #FF5C6B; }
  .body { font-size: 13px; color: #E8E8EE; line-height: 1.5; margin: 0; }
  .dont-ask {
    display: flex; align-items: center; gap: 6px;
    margin-top: 14px; font-size: 12px; color: #7A7A85;
  }
  footer {
    display: flex; justify-content: flex-end; gap: 8px;
    margin-top: 18px;
  }
  button {
    background: #1F1F26;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 6px 14px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  button:hover { background: #26262E; }
  button.confirm { background: #3A2A66; border-color: #3A2A66; color: white; }
  button.confirm:hover { background: #4A3678; }
  button.confirm.danger { background: #5A1A24; border-color: #5A1A24; }
  button.confirm.danger:hover { background: #732030; }
</style>
