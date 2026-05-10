<script lang="ts">
  import { Download, X, RefreshCw } from "lucide-svelte";
  import { updates } from "../state/updates.svelte";

  const visible = $derived(
    (updates.state === "available" || updates.state === "applying") &&
    !updates.bannerDismissed
  );
</script>

{#if visible}
  <div class="banner" role="status" aria-live="polite">
    <span class="icon"><Download size={13}/></span>
    <span class="text">
      <strong>Update available</strong>
      <span class="dim">
        Rift <span class="mono">{updates.info?.version ?? "?"}</span> is ready —
        you're on <span class="mono">{updates.currentVersion}</span>
      </span>
    </span>
    <span class="actions">
      <button class="btn-details" type="button" onclick={() => updates.open()}>
        Details
      </button>
      <button
        class="btn-install"
        type="button"
        onclick={() => updates.apply()}
        disabled={updates.state === "applying"}
      >
        {#if updates.state === "applying"}
          <RefreshCw size={11} class="spin"/> Installing…
        {:else}
          Install now
        {/if}
      </button>
      <button class="btn-dismiss" type="button" onclick={() => updates.dismissBanner()} aria-label="Dismiss">
        <X size={12}/>
      </button>
    </span>
  </div>
{/if}

<style>
  .banner {
    display: flex; align-items: center; gap: 10px;
    width: 100%; height: 100%;
    padding: 0 12px;
    background: var(--accent-bg, color-mix(in srgb, var(--accent) 14%, var(--bg)));
    border-bottom: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    color: var(--fg);
    font: inherit;
  }
  .icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 20px; height: 20px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--accent) 28%, transparent);
    color: var(--accent);
    flex: 0 0 auto;
  }
  .text {
    display: flex; align-items: baseline; gap: 8px;
    font-size: var(--fs-sm);
    flex: 1; min-width: 0;
  }
  .text strong { color: var(--fg); font-weight: 600; }
  .dim { color: var(--fg-2); }
  .mono { font-family: var(--font-mono, ui-monospace, monospace); font-size: var(--fs-xs); }

  .actions { display: inline-flex; align-items: center; gap: 6px; flex: 0 0 auto; }
  .btn-details {
    height: 22px; padding: 0 8px;
    background: transparent;
    color: var(--fg-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: var(--fs-xs);
    cursor: pointer;
  }
  .btn-details:hover { background: color-mix(in srgb, var(--fg) 8%, transparent); color: var(--fg); }
  .btn-install {
    height: 22px; padding: 0 10px;
    background: var(--accent);
    color: var(--accent-fg, var(--bg));
    border: 0; border-radius: 4px;
    font-size: var(--fs-xs); font-weight: 500;
    cursor: pointer;
    display: inline-flex; align-items: center; gap: 4px;
  }
  .btn-install:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .btn-install:disabled { opacity: 0.7; cursor: default; }

  .btn-dismiss {
    height: 22px; width: 22px;
    background: transparent;
    color: var(--fg-faint);
    border: 0; border-radius: 4px;
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .btn-dismiss:hover {
    background: color-mix(in srgb, var(--fg) 10%, transparent);
    color: var(--fg);
  }

  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
