<script lang="ts">
  import { Check } from "lucide-svelte";
  import { PANEL_IDS, type PanelId } from "$lib/state/panel-types";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { PANELS } from "./panels";

  let { onClose }: { onClose: () => void } = $props();

  let rootEl: HTMLDivElement | undefined;

  function onWindowClick(ev: MouseEvent) {
    if (rootEl && !rootEl.contains(ev.target as Node)) onClose();
  }
  function onKeyDown(ev: KeyboardEvent) {
    if (ev.key === "Escape") onClose();
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKeyDown}/>

<div class="add-menu" role="menu" bind:this={rootEl}>
  <header class="add-head">
    <span>Add panel</span>
  </header>
  <ul class="add-list">
    {#each PANEL_IDS as id (id)}
      {@const def = PANELS[id]}
      {@const open = uiPrefs.panels[id]?.open ?? false}
      <li>
        <button
          class="add-item"
          type="button"
          role="menuitemcheckbox"
          aria-checked={open}
          onclick={() => uiPrefs.togglePanel(id as PanelId)}
        >
          <span class="add-check" data-on={open}>
            {#if open}<Check size={12}/>{/if}
          </span>
          <span class="add-icon"><def.icon size={13}/></span>
          <span class="add-label">{def.title}</span>
          <span class="add-kbd mono">⌃{def.kbd}</span>
        </button>
      </li>
    {/each}
  </ul>
  <footer class="add-foot dim">
    Snippets, Diagnostics → v0.4
  </footer>
</div>

<style>
  .add-menu {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 0; right: 0;
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    padding: 6px;
    z-index: 60;
    animation: add-in 120ms ease-out both;
  }
  .add-head {
    padding: 4px 8px 6px;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  .add-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex; flex-direction: column; gap: 1px;
  }
  .add-item {
    display: flex; align-items: center; gap: 8px;
    width: 100%; height: 28px;
    padding: 0 8px;
    background: transparent; border: 0;
    color: var(--fg);
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition: background 100ms ease;
  }
  .add-item:hover { background: var(--surface-hover); }
  .add-check {
    display: inline-flex; align-items: center; justify-content: center;
    width: 14px; height: 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xs);
    background: var(--bg);
    color: var(--accent);
    flex-shrink: 0;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .add-check[data-on="true"] {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-fg);
  }
  .add-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 16px;
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .add-label { flex: 1; }
  .add-kbd {
    font-size: 10px;
    color: var(--fg-faint);
    letter-spacing: 0.02em;
  }
  .add-foot {
    padding: 6px 8px 2px;
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    border-top: 1px solid var(--border);
    margin-top: 4px;
  }

  @keyframes add-in {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: none; }
  }
</style>
