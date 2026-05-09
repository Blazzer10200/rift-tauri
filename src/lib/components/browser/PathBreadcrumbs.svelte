<script lang="ts">
  import { AppWindow, Server, RefreshCw, Filter } from "lucide-svelte";

  type Props = {
    side: "local" | "remote";
    path: string;
    sep: "/" | "\\";
    onNavigate: (newPath: string) => void;
    onRefresh?: () => void;
    filterValue?: string;
    onFilterChange?: (v: string) => void;
  };
  let { side, path, sep, onNavigate, onRefresh, filterValue = "", onFilterChange }: Props = $props();

  let showFilter = $state(false);

  type Crumb = { label: string; full: string };

  const crumbs = $derived.by<Crumb[]>(() => {
    if (!path) return [];
    const isWin = sep === "\\";
    const norm = path.replaceAll("\\", "/");
    const segments = norm.split("/").filter((s) => s.length > 0);
    const out: Crumb[] = [];
    if (isWin && /^[A-Za-z]:$/.test(segments[0] ?? "")) {
      const drive = segments.shift()!;
      out.push({ label: drive + "\\", full: drive + "\\" });
    } else if (norm.startsWith("/")) {
      out.push({ label: "/", full: "/" });
    }
    let cur = out.length > 0 ? out[out.length - 1].full : "";
    for (const seg of segments) {
      cur = cur.endsWith(sep) || cur === "" ? cur + seg : cur + sep + seg;
      out.push({ label: seg, full: cur });
    }
    return out;
  });
</script>

<div class="bcrumbs" data-side={side}>
  <div class="side-tag">
    {#if side === "local"}<AppWindow size={11}/>{:else}<Server size={11}/>{/if}
    <span>{side === "local" ? "LOCAL" : "REMOTE"}</span>
  </div>
  <nav class="path mono" aria-label="Path breadcrumbs">
    {#each crumbs as c, i (c.full)}
      {#if i > 0}<span class="sep">{sep}</span>{/if}
      <button class="crumb" type="button" onclick={() => onNavigate(c.full)}>{c.label}</button>
    {/each}
  </nav>
  <div class="actions">
    {#if onRefresh}
      <button class="btn ghost xs" onclick={onRefresh} type="button" title="Refresh" aria-label="Refresh">
        <RefreshCw size={11}/>
      </button>
    {/if}
    {#if onFilterChange}
      <button class="btn ghost xs" onclick={() => (showFilter = !showFilter)} type="button" title="Filter" aria-label="Filter" data-active={showFilter}>
        <Filter size={11}/>
      </button>
    {/if}
  </div>
  {#if showFilter && onFilterChange}
    <input
      class="input filter"
      placeholder="Filter…"
      value={filterValue}
      oninput={(e) => onFilterChange?.(e.currentTarget.value)}
      type="text"
    />
  {/if}
</div>

<style>
  .bcrumbs {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 10px;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
    min-height: 32px;
  }
  .side-tag {
    display: inline-flex; align-items: center; gap: 5px;
    color: var(--fg-subtle);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    padding-right: 8px;
    border-right: 1px solid var(--border);
  }
  .path {
    flex: 1;
    display: flex; align-items: center;
    flex-wrap: nowrap; overflow-x: auto;
    font-size: var(--fs-sm);
    color: var(--fg-muted);
    min-width: 0;
  }
  .crumb {
    background: transparent; border: 0;
    color: var(--fg); cursor: pointer;
    padding: 2px 6px; border-radius: var(--radius-xs);
    font: inherit;
  }
  .crumb:hover { background: var(--surface-hover); color: var(--accent); }
  .sep { color: var(--fg-faint); padding: 0 1px; }
  .actions { display: flex; gap: 2px; }
  .filter { width: 160px; height: 24px; }
</style>
