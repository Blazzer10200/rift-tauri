<script lang="ts">
  import { X } from "lucide-svelte";
  import { PRESETS, type LayoutPreset } from "$lib/state/panel-types";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { PANELS } from "./panels";

  // `onPick` lets the parent (AppShell) own the apply + side-effects.
  // If omitted, the picker applies the preset itself and calls `onClose`.
  // Both are optional so the picker is usable in either model.
  let { onClose, onPick, dismissible = false }: {
    onClose?: () => void;
    onPick?: (p: LayoutPreset) => void;
    dismissible?: boolean;
  } = $props();

  type PresetCard = {
    id: LayoutPreset;
    title: string;
    sub: string;
  };
  const cards: PresetCard[] = [
    { id: "minimal",  title: "Minimal",  sub: "Just the essentials. Two panels. Easy to learn." },
    { id: "standard", title: "Standard", sub: "Daily-driver loadout. Five panels — chat, sync, files, agents, history." },
    { id: "power",    title: "Power",    sub: "Everything on. Eight panels for power users with screen space to burn." },
  ];

  function pick(p: LayoutPreset) {
    if (onPick) {
      onPick(p);
    } else {
      uiPrefs.applyPreset(p);
      onClose?.();
    }
  }
  function onKeyDown(ev: KeyboardEvent) {
    if (dismissible && ev.key === "Escape") onClose?.();
  }
</script>

<svelte:window onkeydown={onKeyDown}/>

<div class="dialog-overlay" role="presentation">
  <div class="dialog-shell preset-shell" role="dialog" aria-modal="true" aria-labelledby="preset-title">
    <header class="preset-head">
      <div>
        <h2 id="preset-title">Pick a starting layout</h2>
        <p class="dim">You can change this any time in Settings → Appearance.</p>
      </div>
      {#if dismissible && onClose}
        <button class="dialog-close" type="button" aria-label="Close" onclick={onClose}>
          <X size={14}/>
        </button>
      {/if}
    </header>
    <div class="preset-grid">
      {#each cards as c (c.id)}
        {@const ids = PRESETS[c.id]}
        <button class="preset-card" type="button" onclick={() => pick(c.id)}>
          <div class="card-head">
            <span class="card-title">{c.title}</span>
            <span class="card-count">{ids.length} panel{ids.length === 1 ? "" : "s"}</span>
          </div>
          <p class="card-sub">{c.sub}</p>
          <ul class="card-panels">
            {#each ids as id (id)}
              {@const def = PANELS[id]}
              <li>
                <span class="card-panel-icon"><def.icon size={12}/></span>
                <span>{def.title}</span>
              </li>
            {/each}
          </ul>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .preset-shell {
    width: 720px;
    max-width: 92vw;
  }
  .preset-head {
    display: flex; align-items: flex-start; gap: 12px;
    padding: 16px 18px 14px;
    border-bottom: 1px solid var(--border);
  }
  .preset-head h2 {
    margin: 0;
    font-size: var(--fs-lg);
    font-weight: 600;
    color: var(--fg);
  }
  .preset-head .dim {
    margin: 4px 0 0;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
  }
  .preset-head > div { flex: 1; }

  .preset-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    padding: 16px 18px 18px;
  }

  .preset-card {
    display: flex; flex-direction: column; gap: 10px;
    padding: 14px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--fg);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 140ms ease, border-color 140ms ease, transform 80ms ease, box-shadow 140ms ease;
  }
  .preset-card:hover {
    background: var(--bg-elev-3);
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border-strong));
    box-shadow: 0 4px 18px color-mix(in oklch, var(--accent) 14%, transparent);
  }
  .preset-card:active { transform: translateY(1px); }
  .preset-card:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }

  .card-head {
    display: flex; align-items: center; justify-content: space-between;
  }
  .card-title {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--fg);
  }
  .card-count {
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    padding: 2px 6px;
    background: var(--bg-elev-1);
    border-radius: 999px;
  }
  .card-sub {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    line-height: 1.45;
  }
  .card-panels {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex; flex-direction: column; gap: 4px;
    border-top: 1px solid var(--border);
    padding-top: 10px;
  }
  .card-panels li {
    display: flex; align-items: center; gap: 8px;
    font-size: var(--fs-xs);
    color: var(--fg-2);
  }
  .card-panel-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 14px;
    color: var(--fg-muted);
  }
</style>
