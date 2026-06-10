<script lang="ts">
  // C6 (per docs/design/composer-split.md) — the `/command` popover, lifted
  // verbatim from Composer.svelte 2026-06-10. Render + click only: the
  // keyboard nav index and open/filter state stay owned by the parent's onKey.
  let {
    commands,
    activeIdx,
    onPick,
  }: {
    commands: { name: string; desc: string }[];
    activeIdx: number;
    onPick: (c: { name: string; desc: string }) => void;
  } = $props();
</script>

<div class="rift-menu slash-menu" role="menu">
  {#each commands as c, i (c.name)}
    <button
      type="button"
      role="menuitem"
      class="rift-menu-row slash-row"
      class:active={i === activeIdx}
      style="--idx: {i}"
      onmousedown={(e) => { e.preventDefault(); onPick(c); }}
    >
      <span class="rift-menu-row-body">
        <span class="rift-menu-row-t slash-cmd">/{c.name}</span>
        <span class="rift-menu-row-d">{c.desc}</span>
      </span>
    </button>
  {/each}
  <div class="slash-hint">↑↓ select · Tab/Enter pick · Esc cancel</div>
</div>

<style>
  /* Shares the global .rift-menu chrome; this only carries positioning
     (full-width, anchored above the composer) + the entry tween. */
  .slash-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 100%;
    max-height: 280px;
    overflow-y: auto;
    z-index: 10;
    animation: slash-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  /* Per-row staggered entry — driven by inline style="--idx: {i}". */
  .slash-row {
    animation: slash-item-in 280ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--idx, 0) * 22ms);
  }
  @keyframes slash-item-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .slash-row { animation: none; }
  }
  .slash-cmd { font-family: var(--font-mono, ui-monospace, monospace); color: var(--accent); }
  .slash-hint {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px 4px;
    margin-top: 4px;
    font-size: 10px;
    color: var(--fg-faint);
    border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
  }
</style>
