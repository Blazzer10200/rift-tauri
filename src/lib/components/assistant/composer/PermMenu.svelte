<script lang="ts">
  // Permission-mode popover; see docs/ARCHITECTURE.md#frontend-map.
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
    // Only touch state on real movement — this runs every frame (follow loop).
    if (permPos.top !== top || permPos.left !== left) permPos = { top, left };
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
  // Anchor-follow loop (LOCKSTEP w/ SettingsMenu): keeps the panel glued to
  // the pill through the composer's hero↔docked FLIP instead of stranding it
  // at a mid-animation position. positionPerm() no-ops state when still.
  $effect(() => {
    void tick().then(positionPerm);
    let raf = requestAnimationFrame(function follow() {
      positionPerm();
      raf = requestAnimationFrame(follow);
    });
    return () => cancelAnimationFrame(raf);
  });
  const toneClass = (t: PermTone) => (t ? `tone-${t}` : "");
</script>

<!-- Root mousedown preventDefault (LOCKSTEP w/ SettingsMenu): padding
     misclicks must not blur the composer — blur disengages the hero posture
     and unmounts this menu mid-interaction. -->
<div
  class="perm-menu pop"
  role="menu"
  tabindex="-1"
  bind:this={permPop}
  use:portal
  style="top: {permPos.top}px; left: {permPos.left}px;"
  onmousedown={(e) => e.preventDefault()}
>
  <div class="pop-label">Permission mode</div>
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
      <span class="pi-ic"><Icon size={14} /></span>
      <span class="pi-name">{m.label}</span>
      {#if sel}<Check size={14} class="pop-ck" />{/if}
    </button>
  {/each}
</div>

<style>
  /* Portaled to <body> — rules ride :global (the portal re-parents the node, so
     scoped selectors can't be relied on). Visual spec = docs/design/rift-redesign.html
     `.pop` / `.pop-item.rich`. Namespaced under `.perm-menu` to avoid leaking. */
  /* pop-in keyframe → app.css (shared). */
  /* Panel chrome MIRRORS SettingsMenu (one popover idiom for the whole bar):
     flat professional surface — near-solid fill, mild blur, quick pop-in.
     Keep the two recipes in lockstep when either changes. */
  :global(.perm-menu.pop) {
    position: fixed; z-index: 9998; min-width: 236px; padding: 5px;
    border-radius: 12px; transform-origin: bottom left;
    background: color-mix(in oklab, var(--bg-elev-2) 94%, transparent);
    -webkit-backdrop-filter: blur(20px) saturate(1.4);
    backdrop-filter: blur(20px) saturate(1.4);
    border: 1px solid color-mix(in oklab, var(--fg) 10%, transparent);
    box-shadow:
      inset 0 1px 0 oklch(1 0 0 / 0.05),
      0 20px 48px -20px oklch(0 0 0 / 0.65),
      var(--shadow-lg);
    animation: pop-in var(--dur-base) var(--ease-page) both;
  }
  :global(.perm-menu .pop-label) {
    display: flex; align-items: center; gap: 7px;
    font-size: 9.5px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--fg-faint); padding: 9px 9px 6px;
  }
  /* Rows — dense one-liners (hint lives in the tooltip): tone glyph, label, ✓. */
  :global(.perm-menu .pop-item) {
    position: relative;
    display: flex; align-items: center; gap: 9px; width: 100%;
    min-height: 30px; padding: 0 9px; border-radius: 8px; border: 0; background: transparent;
    color: var(--fg-2); cursor: pointer; font: inherit; text-align: left;
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  :global(.perm-menu .pop-item:hover), :global(.perm-menu .pop-item.active) {
    background: var(--surface-hover); color: var(--fg);
  }
  /* Tone glyph — small inline icon, no tile box; carries the mode's semantic
     color (same signal the bar's perm button icon shows). */
  :global(.perm-menu .pi-ic) {
    flex: none; display: inline-flex; color: var(--fg-muted);
    transition: color var(--dur-fast);
  }
  :global(.perm-menu .pop-item.tone-ok .pi-ic)   { color: var(--ok); }
  :global(.perm-menu .pop-item.tone-warn .pi-ic) { color: var(--warn); }
  :global(.perm-menu .pop-item.tone-info .pi-ic) { color: var(--info); }
  :global(.perm-menu .pi-name) {
    flex: 1; min-width: 0; font-size: 12.5px; font-weight: 500; color: var(--fg-2);
    line-height: 1.2; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  :global(.perm-menu .pop-item:hover .pi-name) { color: var(--fg); }
  /* Selected — tone-soft wash + static tone rail + ✓ (matches SettingsMenu). */
  :global(.perm-menu .pop-item.sel)::before {
    content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 2px; height: 60%; border-radius: 0 2px 2px 0; background: var(--accent);
  }
  :global(.perm-menu .pop-item.tone-ok.sel)::before   { background: var(--ok); }
  :global(.perm-menu .pop-item.tone-warn.sel)::before { background: var(--warn); }
  :global(.perm-menu .pop-item.tone-info.sel)::before { background: var(--info); }

  /* permission tone tinting */
  :global(.perm-menu .pop-item.tone-ok .pi-ic)   { color: var(--ok); }
  :global(.perm-menu .pop-item.tone-warn .pi-ic) { color: var(--warn); }
  :global(.perm-menu .pop-item.tone-info .pi-ic) { color: var(--info); }
  :global(.perm-menu .pop-item.rich.sel) { background: var(--accent-soft); }
  :global(.perm-menu .pop-item.rich.sel .pi-name) { color: var(--fg); font-weight: 600; }
  :global(.perm-menu .pop-item.tone-ok.sel)   { background: var(--ok-soft); }
  :global(.perm-menu .pop-item.tone-warn.sel) { background: var(--warn-soft); }
  :global(.perm-menu .pop-item.tone-info.sel) { background: var(--info-soft); }
  :global(.perm-menu .pop-ck) { flex: none; margin-left: 2px; color: var(--accent); }
  :global(.perm-menu .pop-item.tone-ok.sel .pop-ck)   { color: var(--ok); }
  :global(.perm-menu .pop-item.tone-warn.sel .pop-ck) { color: var(--warn); }
  :global(.perm-menu .pop-item.tone-info.sel .pop-ck) { color: var(--info); }
  @media (prefers-reduced-motion: reduce) {
    :global(.perm-menu.pop) { animation: none; }
  }
</style>
