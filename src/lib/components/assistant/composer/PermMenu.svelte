<script lang="ts">
  // C7 (per docs/design/composer-split.md) — the permission-mode popover,
  // lifted verbatim from Composer.svelte 2026-06-10. Portals to <body> (like
  // the hint pop) so it escapes the composer's `overflow: hidden` +
  // backdrop-filter containing block; positions itself against the anchor
  // pill and closes on outside-mousedown via onRequestClose. Keyboard nav
  // (permIdx, ⇧Tab cycle) stays parent-owned.
  import { Check } from "lucide-svelte";
  import { tick } from "svelte";
  import { assistant } from "../../../state/assistant.svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { MODE_OPTIONS, type ModeOpt } from "./modelMatrix";

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
    const pw = permPop.offsetWidth || 252;
    let top = a.top - ph - 8;
    if (top < 8) top = a.bottom + 8;
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
</script>

<div
  class="perm-menu"
  role="menu"
  bind:this={permPop}
  use:portal
  style="top: {permPos.top}px; left: {permPos.left}px;"
>
  <div class="mm-head">Permission mode <kbd class="perm-kbd">⇧Tab</kbd></div>
  {#each MODE_OPTIONS as m, i (m.id)}
    {@const Icon = m.icon}
    <button
      type="button"
      role="menuitemradio"
      aria-checked={m.id === assistant.permissionMode}
      class="perm-row"
      class:active={i === permIdx}
      class:current={m.id === assistant.permissionMode}
      data-mode={m.id}
      use:tooltip={m.hint}
      onmousedown={(ev) => { ev.preventDefault(); onPick(m); }}
    >
      <span class="perm-row-ic"><Icon size={13} /></span>
      <span class="perm-row-tt">
        <span class="perm-row-t">{m.label}</span>
        <span class="perm-row-d">{m.hint.split(" — ")[1] ?? m.hint}</span>
      </span>
      {#if m.id === assistant.permissionMode}<Check size={13} class="perm-row-chk" />{/if}
    </button>
  {/each}
</div>

<style>
  /* Portaled to <body> — rules ride :global exactly as they did in Composer
     (the portal re-parents the node, so scoped selectors can't be relied on). */
  @keyframes -global-hint-in {
    from { opacity: 0; transform: translateY(4px) scale(0.98); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }
  :global(.perm-menu) {
    position: fixed; width: 252px; padding: 5px;
    background: color-mix(in oklch, var(--surface) 86%, transparent);
    backdrop-filter: blur(16px) saturate(135%);
    -webkit-backdrop-filter: blur(16px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
    border-radius: 14px;
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklab, var(--accent) 6%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    z-index: 9998;
    animation: hint-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-origin: bottom left;
  }
  :global(.perm-menu .mm-head) {
    display: flex; align-items: center; gap: 7px;
    font-size: 9.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em;
    color: var(--fg-faint); padding: 4px 8px 6px;
  }
  :global(.perm-menu .perm-kbd) {
    font-family: var(--font-mono); font-size: 9.5px; color: var(--fg-muted);
    background: var(--bg-inset); border: 1px solid var(--border); border-radius: 4px;
    padding: 1px 5px; text-transform: none; letter-spacing: 0;
  }
  :global(.perm-menu .perm-row) {
    position: relative; display: flex; align-items: flex-start; gap: 9px; width: 100%;
    padding: 6px 8px; border-radius: 7px; border: 0; background: transparent;
    color: var(--fg-2); cursor: pointer; font: inherit; text-align: left;
    transition: background 120ms;
  }
  :global(.perm-menu .perm-row:hover), :global(.perm-menu .perm-row.active) { background: var(--surface-hover); }
  :global(.perm-menu .perm-row-ic) {
    width: 16px; flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center;
    color: var(--fg-subtle); margin-top: 1px; transition: color 130ms ease;
  }
  :global(.perm-menu .perm-row:hover .perm-row-ic), :global(.perm-menu .perm-row.active .perm-row-ic) { color: var(--fg-2); }
  :global(.perm-menu .perm-row-tt) { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  :global(.perm-menu .perm-row-t) { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); line-height: 1.25; }
  :global(.perm-menu .perm-row-d) { font-size: 10.5px; color: var(--fg-subtle); line-height: 1.3; }
  :global(.perm-menu .perm-row-chk) { color: var(--accent); flex-shrink: 0; margin-top: 1px; }
  /* current row: accent left-bar + accent icon, no slab (matches mock) */
  :global(.perm-menu .perm-row.current::before) {
    content: ""; position: absolute; left: 0; top: 6px; bottom: 6px; width: 2.5px;
    border-radius: 0 3px 3px 0; background: var(--accent); box-shadow: 0 0 8px var(--ring);
  }
  :global(.perm-menu .perm-row.current .perm-row-ic) { color: var(--accent); }
  :global(.perm-menu .perm-row[data-mode="bypassPermissions"].current::before) { background: var(--warn); box-shadow: 0 0 8px color-mix(in oklab, var(--warn) 55%, transparent); }
  :global(.perm-menu .perm-row[data-mode="bypassPermissions"].current .perm-row-ic),
  :global(.perm-menu .perm-row[data-mode="bypassPermissions"].current .perm-row-chk) { color: var(--warn); }
</style>
