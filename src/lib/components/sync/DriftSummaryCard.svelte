<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { GitBranch, CheckCircle2 } from "lucide-svelte";
  import { syncPage, type DriftEntry, type DriftBucket } from "../../state/sync-page.svelte";

  // Group entries by resource (top-level resource name from the engine).
  type DivergentGroup = {
    resource: string;
    push: number;
    pull: number;
    del: number;
    delRem: number;
    conflict: number;
    total: number;
  };

  const groups = $derived.by<DivergentGroup[]>(() => {
    const byResource = new Map<string, DivergentGroup>();
    for (const e of syncPage.entries as DriftEntry[]) {
      const g = byResource.get(e.resource_name) ?? {
        resource: e.resource_name,
        push: 0, pull: 0, del: 0, delRem: 0, conflict: 0, total: 0,
      };
      const b: DriftBucket = e.bucket;
      if (b === "to_push") g.push++;
      else if (b === "to_pull") g.pull++;
      else if (b === "to_delete") g.del++;
      else if (b === "to_delete_remote") g.delRem++;
      else if (b === "conflict") g.conflict++;
      else continue;
      g.total++;
      byResource.set(e.resource_name, g);
    }
    return [...byResource.values()].sort((a, b) => b.total - a.total);
  });

  const hasDrift = $derived(groups.length > 0);
</script>

<section class="card" aria-labelledby="drift-summary-title">
  <header class="card-head">
    <GitBranch size={13} />
    <h3 id="drift-summary-title">Drift summary</h3>
    {#if hasDrift}
      <span class="count mono">{groups.length}</span>
    {/if}
  </header>

  {#if !hasDrift}
    <div class="ok-chip" in:fade={{ duration: 160 }}>
      <span class="ok-glyph"><CheckCircle2 size={14}/></span>
      <span class="ok-text">Nothing diverged</span>
    </div>
  {:else}
    <ul class="rows">
      {#each groups as g, i (g.resource)}
        <li
          class="row"
          in:fly={{
            y: 4,
            duration: 200,
            delay: Math.min(i * 8, 80),
            easing: quintOut,
          }}
        >
          <span class="resource mono" title={g.resource}>{g.resource}</span>
          <span class="pips">
            {#if g.push > 0}<span class="pip" data-tone="push" title="{g.push} to push">{g.push}↑</span>{/if}
            {#if g.pull > 0}<span class="pip" data-tone="pull" title="{g.pull} to pull">{g.pull}↓</span>{/if}
            {#if g.del > 0}<span class="pip" data-tone="delete" title="{g.del} local delete">{g.del}×</span>{/if}
            {#if g.delRem > 0}<span class="pip" data-tone="delete" title="{g.delRem} remote delete">{g.delRem}⌫</span>{/if}
            {#if g.conflict > 0}<span class="pip" data-tone="conflict" title="{g.conflict} conflict">{g.conflict}!</span>{/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .card {
    display: flex; flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elev-1);
    overflow: hidden;
    min-width: 0;
  }
  .card-head {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--fg);
    font-size: var(--fs-sm);
  }
  .card-head h3 {
    margin: 0;
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  .count {
    margin-left: auto;
    font-size: var(--fs-xs);
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--warn-soft);
    color: var(--warn);
    font-variant-numeric: tabular-nums;
  }

  .ok-chip {
    display: inline-flex; align-items: center; gap: 8px;
    align-self: flex-start;
    margin: 12px 14px;
    padding: 4px 10px 4px 4px;
    border-radius: 999px;
    background: var(--ok-soft);
    color: var(--ok);
    font-size: var(--fs-sm);
    border: 1px solid color-mix(in oklch, var(--ok) 30%, transparent);
  }
  .ok-glyph {
    width: 22px; height: 22px;
    border-radius: 50%;
    background: var(--bg);
    display: inline-flex; align-items: center; justify-content: center;
  }

  .rows {
    list-style: none; margin: 0; padding: 0;
    display: flex; flex-direction: column;
  }
  .row {
    display: flex; align-items: center; gap: 10px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
  }
  .row:last-child { border-bottom: 0; }
  .resource {
    color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    flex: 1; min-width: 0;
  }
  .pips {
    display: inline-flex; gap: 4px;
    flex-shrink: 0;
  }
  .pip {
    display: inline-flex; align-items: center;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
    border: 1px solid transparent;
  }
  .pip[data-tone="push"]     { background: var(--accent-soft); color: var(--accent); border-color: color-mix(in oklch, var(--accent) 22%, transparent); }
  .pip[data-tone="pull"]     { background: var(--info-soft);   color: var(--info);   border-color: color-mix(in oklch, var(--info) 22%, transparent); }
  .pip[data-tone="delete"]   { background: var(--danger-soft); color: var(--danger); border-color: color-mix(in oklch, var(--danger) 22%, transparent); }
  .pip[data-tone="conflict"] { background: var(--warn-soft);   color: var(--warn);   border-color: color-mix(in oklch, var(--warn) 22%, transparent); }
</style>
