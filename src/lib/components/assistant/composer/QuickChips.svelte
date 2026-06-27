<script lang="ts">
  import type { QuickStart } from "./quickStarts";

  let {
    quickStarts,
    onPick,
  }: {
    quickStarts: QuickStart[];
    onPick: (prompt: string) => void;
  } = $props();
</script>

<div class="quick-chips">
  {#each quickStarts as s (s.title)}
    <button class="quick-chip" type="button" onclick={() => onPick(s.prompt)}>
      <span class="qc-ic"><s.icon size={14}/></span>{s.title}
    </button>
  {/each}
</div>

<style>
  .quick-chips {
    display: flex; flex-wrap: wrap; gap: 7px;
    padding: 0 2px 12px;
  }
  .quick-chip {
    display: inline-flex; align-items: center; gap: 7px;
    height: 28px; padding: 0 12px 0 10px;
    border-radius: 8px;
    font: inherit; font-size: 12px; color: var(--fg-muted);
    background: transparent; border: 1px solid var(--border);
    cursor: pointer;
    transition: background var(--dur-fast, 140ms), color var(--dur-fast, 140ms),
                border-color var(--dur-fast, 140ms),
                transform var(--dur-fast, 140ms) var(--ease-page, ease-out);
  }
  .qc-ic { display: inline-flex; color: var(--fg-faint); flex: none; transition: color var(--dur-fast, 140ms); }
  .quick-chip:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); transform: translateY(-1px); }
  .quick-chip:hover .qc-ic { color: var(--accent); }
  .quick-chip:active { transform: translateY(0) scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .quick-chip { transition: none; } }
</style>
