<script lang="ts">
  // C2 (per docs/design/composer-split.md) — staged-attachment chips + the
  // attach-error pill, lifted verbatim from Composer.svelte 2026-06-09.
  // Paste/drop/file-pick handlers stay in the parent (they write tab state);
  // this child renders + reports clicks only.
  import { X } from "lucide-svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { fmtSize } from "./helpers";

  let {
    attachments,
    attachError,
    onRemove,
    onDismissError,
  }: {
    attachments: { id: string; mime: string; dataBase64: string; sizeBytes: number }[];
    attachError: string | null;
    onRemove: (id: string) => void;
    onDismissError: () => void;
  } = $props();
</script>

{#if attachments.length > 0 || attachError}
  <div class="attachments">
    {#each attachments as a (a.id)}
      <div class="attach-chip" use:tooltip={`${a.mime} · ${fmtSize(a.sizeBytes)}`}>
        <img class="attach-thumb" src={`data:${a.mime};base64,${a.dataBase64}`} alt="pasted attachment" />
        <span class="attach-meta">
          <span class="attach-name">image</span>
          <span class="attach-size">{fmtSize(a.sizeBytes)}</span>
        </span>
        <button
          class="attach-x"
          type="button"
          onclick={() => onRemove(a.id)}
          aria-label="Remove attachment"
        >
          <X size={11} />
        </button>
      </div>
    {/each}
    {#if attachError}
      <div class="attach-error" role="alert">
        {attachError}
        <button class="attach-error-x" type="button" onclick={onDismissError} aria-label="Dismiss">
          <X size={10} />
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .attachments {
    display: flex; flex-wrap: wrap; align-items: center; gap: 8px;
    margin-bottom: 8px;
  }
  .attach-chip {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 4px 8px 4px 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    transition: border-color 140ms ease-out, background 140ms ease-out;
  }
  .attach-chip:hover { border-color: var(--border-strong); }
  .attach-thumb {
    width: 40px; height: 40px;
    object-fit: cover;
    border-radius: 6px;
    background: var(--bg-elev-2);
  }
  .attach-meta {
    display: inline-flex; flex-direction: column; gap: 1px;
    line-height: 1.2;
  }
  .attach-name { font-size: var(--fs-xs); font-weight: 600; color: var(--fg); }
  .attach-size {
    font-size: 10px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }
  .attach-x {
    display: inline-flex; align-items: center; justify-content: center;
    width: 20px; height: 20px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--fg-faint);
    cursor: pointer;
    padding: 0;
    margin-left: 2px;
  }
  .attach-x:hover { background: var(--bg-elev-2); color: var(--fg); }
  .attach-error {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 4px 8px 4px 10px;
    background: var(--danger-soft, color-mix(in oklab, var(--danger) 12%, transparent));
    border: 1px solid color-mix(in oklab, var(--danger) 35%, var(--border));
    border-radius: 8px;
    font-size: var(--fs-xs);
    color: var(--danger);
  }
  .attach-error-x {
    display: inline-flex; align-items: center; justify-content: center;
    width: 16px; height: 16px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--danger);
    cursor: pointer;
    opacity: 0.7;
    padding: 0;
  }
  .attach-error-x:hover { opacity: 1; background: color-mix(in oklab, var(--danger) 18%, transparent); }
</style>
