<script lang="ts">
  // Corner toast that surfaces an available update without yanking the user
  // into a full-screen modal. Auto-hides after AUTO_DISMISS_MS, paused on
  // hover. Click → opens the full dialog. The "×" snoozes the version so we
  // don't pop again next launch.
  import { Sparkles, X, ArrowRight } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { updates } from "../state/updates.svelte";

  const AUTO_DISMISS_MS = 12_000;

  let hovering = $state(false);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function arm() {
    if (timer) clearTimeout(timer);
    if (hovering) return;
    timer = setTimeout(() => { updates.dismissToast(); timer = null; }, AUTO_DISMISS_MS);
  }
  function disarm() {
    if (timer) { clearTimeout(timer); timer = null; }
    hovering = false;
  }

  $effect(() => {
    if (updates.toastVisible) arm();
    else disarm();
    return () => disarm();
  });

  function viewDetails(e: MouseEvent) {
    e.stopPropagation();
    updates.open();
  }
  function snooze(e: MouseEvent) {
    e.stopPropagation();
    updates.snooze();
  }
</script>

{#if updates.toastVisible && updates.info}
  <div
    class="toast"
    role="button"
    tabindex="0"
    onclick={viewDetails}
    onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); updates.open(); } }}
    onmouseenter={() => { hovering = true; disarm(); }}
    onmouseleave={() => { hovering = false; arm(); }}
    transition:fly={{ y: 12, duration: 220 }}
    aria-label="View update details"
  >
    <span class="glow"></span>
    <span class="icon">
      <Sparkles size={14}/>
    </span>
    <span class="body">
      <span class="title">Update available</span>
      <span class="meta">
        <span class="mono">v{updates.currentVersion}</span>
        <ArrowRight size={10} class="meta-arrow"/>
        <span class="mono tgt">v{updates.info.version}</span>
        {#if updates.sizeLabel}<span class="dim">· {updates.sizeLabel}</span>{/if}
      </span>
    </span>
    <span class="action">View</span>
    <button class="close" type="button" onclick={snooze} aria-label="Snooze this version">
      <X size={11}/>
    </button>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 36px;
    right: 18px;
    z-index: 60;
    display: grid;
    grid-template-columns: 32px 1fr auto 22px;
    align-items: center;
    gap: 10px;
    padding: 10px 12px 10px 10px;
    width: 360px;
    max-width: calc(100vw - 36px);
    background: linear-gradient(180deg,
      color-mix(in oklch, var(--accent) 14%, color-mix(in oklch, var(--bg-elev-1) 88%, transparent)) 0%,
      color-mix(in oklch, var(--bg-elev-1) 92%, transparent) 100%);
    backdrop-filter: blur(18px) saturate(140%);
    -webkit-backdrop-filter: blur(18px) saturate(140%);
    border: 1px solid color-mix(in oklch, var(--accent) 38%, var(--border-strong));
    border-radius: 12px;
    box-shadow:
      0 22px 56px -14px oklch(0 0 0 / 0.6),
      0 6px 18px -8px oklch(0 0 0 / 0.4),
      0 0 28px -6px color-mix(in oklch, var(--accent) 38%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
    color: var(--fg);
    cursor: pointer;
    text-align: left;
    font: inherit;
    overflow: hidden;
  }
  .toast:hover {
    border-color: color-mix(in oklch, var(--accent) 50%, var(--border-strong));
    box-shadow:
      0 22px 56px -16px rgba(0, 0, 0, 0.6),
      0 0 30px -6px color-mix(in oklch, var(--accent) 42%, transparent);
  }
  .glow {
    position: absolute;
    inset: -50% -10% auto -10%;
    height: 80%;
    background: radial-gradient(60% 100% at 50% 0%,
      color-mix(in oklch, var(--accent) 22%, transparent),
      transparent 70%);
    pointer-events: none;
  }
  .icon {
    position: relative;
    width: 32px; height: 32px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: 8px;
    background: color-mix(in oklch, var(--accent) 25%, var(--bg-elev-3));
    color: var(--accent);
    border: 1px solid color-mix(in oklch, var(--accent) 35%, transparent);
    box-shadow: 0 0 14px color-mix(in oklch, var(--accent) 30%, transparent);
  }
  .body { position: relative; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .title {
    color: var(--fg);
    font-size: var(--fs-sm);
    font-weight: 600;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .meta {
    display: inline-flex; align-items: center; gap: 4px;
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    white-space: nowrap;
  }
  .meta .tgt { color: var(--accent); }
  :global(.meta-arrow) { color: var(--fg-faint); }
  .dim { color: var(--fg-faint); }

  .action {
    position: relative;
    font-size: var(--fs-xs);
    color: var(--accent);
    font-weight: 600;
    padding: 4px 8px;
    border-radius: var(--radius-xs);
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklch, var(--accent) 28%, transparent);
  }
  .toast:hover .action {
    background: color-mix(in oklch, var(--accent) 22%, transparent);
  }

  .close {
    position: relative;
    width: 22px; height: 22px;
    display: inline-flex; align-items: center; justify-content: center;
    background: transparent;
    border: 0;
    border-radius: var(--radius-xs);
    color: var(--fg-faint);
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .close:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
</style>
