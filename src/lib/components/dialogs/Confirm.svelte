<script lang="ts">
  import { Trash2, Check, X } from "lucide-svelte";

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
  let confirmBtn = $state<HTMLButtonElement>();

  $effect(() => { if (!open) dontAsk = false; });

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
    if (e.key === "Enter") {
      const active = document.activeElement;
      if (!active || active === document.body || active === confirmBtn) decide(true);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation">
    <div class="dialog-shell" style="width: 440px;" role="alertdialog" aria-modal="true" aria-label={title}>
      <div class="dialog-head">
        <div class="dialog-icon" class:danger={isDanger} class:info={!isDanger}>
          {#if isDanger}<Trash2 size={14}/>{:else}<Check size={14}/>{/if}
        </div>
        <div>
          <div class="dialog-title">{title}</div>
          <div class="dialog-sub">{body}</div>
        </div>
        <button class="dialog-close" type="button" onclick={() => decide(false)} aria-label="Close">
          <X size={14}/>
        </button>
      </div>
      {#if showDontAsk}
        <div class="dialog-body">
          <label class="dialog-checkbox">
            <input type="checkbox" bind:checked={dontAsk}/>
            <span>Don't ask again</span>
          </label>
        </div>
      {/if}
      <div class="dialog-foot">
        <button class="btn ghost" type="button" onclick={() => decide(false)}>
          {cancelLabel}
          <span class="kbd dim">Esc</span>
        </button>
        <div class="dialog-foot-spacer"></div>
        <button
          class="btn"
          class:primary={!isDanger}
          class:danger={isDanger}
          type="button"
          bind:this={confirmBtn}
          onclick={() => decide(true)}
        >
          {confirmLabel}
          <span class="kbd dim">↵</span>
        </button>
      </div>
    </div>
  </div>
{/if}
