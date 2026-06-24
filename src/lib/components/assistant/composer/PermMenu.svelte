<script lang="ts">
  // C7 (per docs/design/composer-split.md) — the permission-mode popover,
  // lifted verbatim from Composer.svelte 2026-06-10. Portals to <body> (like
  // the hint pop) so it escapes the composer's `overflow: hidden` +
  // backdrop-filter containing block; positions itself against the anchor
  // pill and closes on outside-mousedown via onRequestClose. Keyboard nav
  // (permIdx, ⇧Tab cycle) stays parent-owned. Visuals = spec `.pop` rich-item
  // popover (docs/design/rift-redesign.html) — icon tiles, tone tinting,
  // accent-soft selected slab + checkmark.
  import { Check } from "lucide-svelte";
  import { tick } from "svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { MODE_OPTIONS, type ModeOpt, type PermTone, permToneFor } from "./modelMatrix";

  let {
    permIdx,
    anchor,
    onPick,
    onRequestClose,
  }: {
    permIdx: number;
    anchor: HTMLElement | null;
    onPick: (m: ModeOpt) => void;
    onRequestClose: () => void;
  } = $props();

  let permPop = $state<HTMLDivElement | null>(null);
  let permPos = $state<{ top: number; left: number }>({ top: 0, left: 0 });
  function positionPerm() {
    if (!anchor || !permPop) return;
    const a = anchor.getBoundingClientRect();
    const ph = permPop.offsetHeight || 220;
    const pw = permPop.offsetWidth || 264;
    let top = a.top - ph - 9;
    if (top < 8) top = a.bottom + 9;
    let left = a.left;
    const maxLeft = window.innerWidth - pw - 8;
    if (left > maxLeft) left = maxLeft;
    if (left < 8) left = 8;
    permPos = { top, left };
  }
  function onDocPermMousedown(ev: MouseEvent) {
    if (anchor && ev.target instanceof Node && anchor.contains(ev.target)) return;
    if (permPop && ev.target instanceof Node && permPop.contains(ev.target)) return;
    onRequestClose();
  }
  $effect(() => {
    window.addEventListener("mousedown", onDocPermMousedown);
    return () => window.removeEventListener("mousedown", onDocPermMousedown);
  });
  $effect(() => {
    void tick().then(positionPerm);
    const onResize = () => positionPerm();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });
  const toneClass = (t: PermTone) => (t ? `tone-${t}` : "");
</script>

<div
  class="perm-menu pop"
  role="menu"
  bind:this={permPop}
  use:portal
  style="top: {permPos.top}px; left: {permPos.left}px;"
>
  <div class="pop-label">Permission mode <kbd class="perm-kbd">⇧Tab</kbd></div>
  {#each MODE_OPTIONS as m, i (m.id)}
    {@const Icon = m.icon}
    {@const sel = m.id === assistant.permissionMode}
    <button
      type="button"
      role="menuitemradio"
      aria-checked={sel}
      class="pop-item rich {toneClass(permToneFor(m.id))}"
      class:sel
      class:active={i === permIdx}
      data-mode={m.id}
      use:tooltip={m.hint}
      onmousedown={(ev) => { ev.preventDefault(); onPick(m); }}
    >
      <span class="pi-ic"><Icon size={15} /></span>
      <span class="pi-text">
        <span class="pi-name">{m.label}</span>
        <span class="pi-sub">{m.hint.split(" — ")[1] ?? m.hint}</span>
      </span>
      {#if sel}<Check size={15} class="pop-ck" />{/if}
    </button>
  {/each}
</div>

<style>
  /* Portaled to <body> — rules ride :global (the portal re-parents the node, so
     scoped selectors can't be relied on). Visual spec = docs/design/rift-redesign.html
     `.pop` / `.pop-item.rich`. Namespaced under `.perm-menu` to avoid leaking. */
  /* pop-in keyframe → app.css (shared). */
  :global(.perm-menu.pop) {
    position: fixed; z-index: 9998; min-width: 264px; padding: 7px;
    border-radius: 16px; transform-origin: bottom left;
    background: color-mix(in oklab, var(--bg-elev-2) 56%, transparent);
    -webkit-backdrop-filter: blur(26px) saturate(1.6);
    backdrop-filter: blur(26px) saturate(1.6);
    border: 1px solid color-mix(in oklab, var(--fg) 12%, transparent);
    box-shadow:
      inset 0 1px 0 oklch(1 0 0 / 0.08),
      0 28px 64px -28px oklch(0 0 0 / 0.7),
      var(--shadow-lg);
    animation: pop-in 0.26s var(--ease-page) both;
  }
  :global(.perm-menu .pop-label) {
    display: flex; align-items: center; gap: 7px;
    font-size: 10px; font-weight: 700; letter-spacing: 0.09em; text-transform: uppercase;
    color: var(--fg-faint); padding: 7px 9px 5px;
  }
  :global(.perm-menu .perm-kbd) {
    font-family: var(--font-mono); font-size: 9.5px; color: var(--fg-muted);
    background: var(--bg-inset); border: 1px solid var(--border); border-radius: 4px;
    padding: 1px 5px; text-transform: none; letter-spacing: 0;
  }
  :global(.perm-menu .pop-item) {
    display: flex; align-items: center; gap: 9px; width: 100%;
    padding: 0 8px; height: 32px; border-radius: 8px; border: 0; background: transparent;
    color: var(--fg-2); cursor: pointer; font: inherit; text-align: left;
  }
  :global(.perm-menu .pop-item:hover), :global(.perm-menu .pop-item.active) {
    background: var(--surface-hover); color: var(--fg);
  }
  :global(.perm-menu .pop-item.rich) {
    height: auto; gap: 11px; padding: 9px 10px; border-radius: 11px;
    transition: background var(--dur-fast);
  }
  :global(.perm-menu .pi-ic) {
    flex: none; width: 30px; height: 30px; display: grid; place-items: center;
    border-radius: 9px; background: var(--surface); color: var(--fg-muted);
    border: 1px solid var(--border);
    transition: transform 0.34s var(--ease-page), background var(--dur-fast),
                color var(--dur-fast), border-color var(--dur-fast);
  }
  :global(.perm-menu .pi-text) { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  :global(.perm-menu .pi-name) { font-size: 13px; font-weight: 600; color: var(--fg-2); line-height: 1.2; }
  :global(.perm-menu .pi-sub) {
    font-size: 11.5px; color: var(--fg-faint); line-height: 1.3;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  :global(.perm-menu .pop-item.rich:hover .pi-ic) {
    color: var(--fg-2); border-color: var(--border-strong); transform: scale(1.09) rotate(-3deg);
  }
  :global(.perm-menu .pop-item.rich:hover .pi-name) { color: var(--fg); }

  /* permission tone tinting */
  :global(.perm-menu .pop-item.tone-ok .pi-ic)   { color: var(--ok); }
  :global(.perm-menu .pop-item.tone-warn .pi-ic) { color: var(--warn); }
  :global(.perm-menu .pop-item.tone-info .pi-ic) { color: var(--info); }
  :global(.perm-menu .pop-item.rich.sel) { background: var(--accent-soft); }
  :global(.perm-menu .pop-item.rich.sel .pi-name) { color: var(--fg); }
  :global(.perm-menu .pop-item.tone-ok.sel)   { background: var(--ok-soft); }
  :global(.perm-menu .pop-item.tone-ok.sel .pi-ic)   { background: color-mix(in oklab, var(--ok) 16%, transparent);   border-color: transparent; }
  :global(.perm-menu .pop-item.tone-warn.sel) { background: var(--warn-soft); }
  :global(.perm-menu .pop-item.tone-warn.sel .pi-ic) { background: color-mix(in oklab, var(--warn) 16%, transparent); border-color: transparent; }
  :global(.perm-menu .pop-item.tone-info.sel) { background: var(--info-soft); }
  :global(.perm-menu .pop-item.tone-info.sel .pi-ic) { background: color-mix(in oklab, var(--info) 16%, transparent); border-color: transparent; }
  :global(.perm-menu .pop-ck) { flex: none; margin-left: 2px; color: var(--accent); }
  :global(.perm-menu .pop-item.tone-ok.sel .pop-ck)   { color: var(--ok); }
  :global(.perm-menu .pop-item.tone-warn.sel .pop-ck) { color: var(--warn); }
  :global(.perm-menu .pop-item.tone-info.sel .pop-ck) { color: var(--info); }
  @media (prefers-reduced-motion: reduce) {
    :global(.perm-menu.pop), :global(.perm-menu .pop-item.rich) { animation: none; }
  }
</style>
