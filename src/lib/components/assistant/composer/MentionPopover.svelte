<script lang="ts">
  // The `@file` mention popover; see docs/ARCHITECTURE.md#frontend-map.
  // lifted verbatim from Composer.svelte 2026-06-10. Render + click only:
  // caret detection, the nav index, and the insert-into-draft logic stay
  // owned by the parent (they bind the textarea + tab state).
  let {
    results,
    activeIdx,
    fileCount,
    onPick,
  }: {
    results: string[];
    activeIdx: number;
    fileCount: number;
    onPick: (path: string) => void;
  } = $props();
</script>

<div class="rift-menu slash-menu mention-menu" role="menu">
  {#each results as path, i (path)}
    {@const slash = path.lastIndexOf("/")}
    {@const dir = slash > 0 ? path.slice(0, slash + 1) : ""}
    {@const base = slash >= 0 ? path.slice(slash + 1) : path}
    <button
      type="button"
      role="menuitem"
      class="rift-menu-row mention-item"
      class:active={i === activeIdx}
      style="--idx: {i}"
      onmousedown={(e) => { e.preventDefault(); onPick(path); }}
    >
      <span class="mention-base">{base}</span>
      <span class="mention-dir">{dir}</span>
    </button>
  {/each}
  <div class="slash-hint">
    {fileCount > 0
      ? `${fileCount} files · ↑↓ select · Tab/Enter pick · Esc cancel`
      : "loading workspace files…"}
  </div>
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
    animation: slash-in var(--dur-fast) cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  /* Autocomplete needs to feel immediate; rows enter together rather than
     cascading beneath the current match. */
  .mention-item {
    animation: slash-item-in var(--dur-fast) cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @keyframes slash-item-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .slash-menu, .mention-item { animation: none; }
  }
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
  .mention-menu { max-height: 280px; }
  .mention-item {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: baseline;
    gap: 10px;
    padding: 5px 10px;
  }
  .mention-base {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--fs-sm);
    color: var(--fg);
    font-weight: 500;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mention-dir {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
  }
  .mention-item.active .mention-base { color: var(--accent); }
</style>
