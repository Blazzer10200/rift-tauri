<script lang="ts">
  import { connection, type ActivityRow, type ActivityKind } from "../../state/connection.svelte";

  let filter = $state("");
  let kindFilter = $state<ActivityKind | "all">("all");

  const kinds: { id: ActivityKind | "all"; label: string }[] = [
    { id: "all", label: "All" },
    { id: "sync", label: "Sync" },
    { id: "pull", label: "Pull" },
    { id: "delete", label: "Delete" },
    { id: "conflict", label: "Conflict" },
    { id: "drift", label: "Drift" },
    { id: "bridge", label: "Bridge" },
    { id: "block", label: "Block" },
    { id: "error", label: "Error" },
    { id: "system", label: "System" },
  ];

  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    return connection.activityFeed.filter((r) => {
      if (kindFilter !== "all" && r.kind !== kindFilter) return false;
      if (!q) return true;
      return (
        r.resource.toLowerCase().includes(q) ||
        r.file.toLowerCase().includes(q) ||
        r.action.toLowerCase().includes(q)
      );
    });
  });

  // Simple windowed virtualization — render only visible rows.
  const ROW_H = 26;
  const OVERSCAN = 8;
  let scroller: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewport = $state(600);

  $effect(() => {
    if (!scroller) return;
    const el = scroller;
    const onScroll = () => { scrollTop = el.scrollTop; };
    const ro = new ResizeObserver(() => { viewport = el.clientHeight; });
    el.addEventListener("scroll", onScroll, { passive: true });
    ro.observe(el);
    viewport = el.clientHeight;
    return () => {
      el.removeEventListener("scroll", onScroll);
      ro.disconnect();
    };
  });

  const startIdx = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const endIdx = $derived(
    Math.min(filtered.length, Math.ceil((scrollTop + viewport) / ROW_H) + OVERSCAN),
  );
  const slice = $derived(filtered.slice(startIdx, endIdx));
  const padTop = $derived(startIdx * ROW_H);
  const padBot = $derived(Math.max(0, (filtered.length - endIdx) * ROW_H));

  function fmtTime(iso: string): string {
    try { return new Date(iso).toLocaleTimeString(); } catch { return iso; }
  }
  function kindClass(k: ActivityKind): string { return `k-${k}`; }
</script>

<section class="feed">
  <div class="toolbar">
    <input
      class="filter"
      type="text"
      placeholder="Filter resource / file / action…"
      bind:value={filter}
    />
    <select class="kind" bind:value={kindFilter}>
      {#each kinds as k (k.id)}
        <option value={k.id}>{k.label}</option>
      {/each}
    </select>
    <span class="count">{filtered.length} / {connection.activityFeed.length}</span>
  </div>

  <div class="header">
    <span class="col-time">Time</span>
    <span class="col-kind">Kind</span>
    <span class="col-res">Resource</span>
    <span class="col-file">File</span>
    <span class="col-action">Action</span>
  </div>

  <div class="scroller" bind:this={scroller}>
    {#if filtered.length === 0}
      <div class="empty">
        {connection.activityFeed.length === 0
          ? "No activity yet — start auto-sync to see events."
          : "No matches."}
      </div>
    {:else}
      <div style="height:{padTop}px"></div>
      {#each slice as r, i (startIdx + "_" + r.at + "_" + i)}
        <div class="row {kindClass(r.kind)}" style="height:{ROW_H}px">
          <span class="col-time">{fmtTime(r.at)}</span>
          <span class="col-kind">{r.kind}</span>
          <span class="col-res">{r.resource}</span>
          <span class="col-file" title={r.file}>{r.file}</span>
          <span class="col-action">{r.action}</span>
        </div>
      {/each}
      <div style="height:{padBot}px"></div>
    {/if}
  </div>
</section>

<style>
  .feed {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    background: #0F0F12; color: #E8E8EE;
  }
  .toolbar {
    display: flex; gap: 8px; align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid #26262E;
    background: #0F0F12;
  }
  .filter {
    flex: 1;
    background: #17171C; color: #E8E8EE;
    border: 1px solid #26262E; border-radius: 3px;
    padding: 4px 8px; font-size: 12px;
  }
  .filter:focus { outline: 0; border-color: #8B6BE6; }
  .kind {
    background: #17171C; color: #E8E8EE;
    border: 1px solid #26262E; border-radius: 3px;
    padding: 4px 8px; font-size: 12px;
  }
  .count { color: #7A7A85; font-size: 11px; font-family: Consolas, monospace; }
  .header, .row {
    display: grid;
    grid-template-columns: 80px 80px 140px 1fr 200px;
    gap: 12px;
    padding: 4px 12px;
    align-items: center;
    font-size: 12px;
  }
  .header {
    color: #7A7A85; font-weight: 600;
    border-bottom: 1px solid #26262E;
    background: #17171C;
  }
  .scroller { flex: 1; overflow: auto; min-height: 0; }
  .row { border-bottom: 1px solid #15101E; }
  .row:hover { background: #17171C; }
  .col-time, .col-kind { color: #7A7A85; font-family: Consolas, monospace; }
  .col-res { color: #8B6BE6; font-family: Consolas, monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .col-file { color: #E8E8EE; font-family: Consolas, monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .col-action { color: #7A7A85; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row.k-sync .col-kind { color: #4ADE80; }
  .row.k-pull .col-kind { color: #8B6BE6; }
  .row.k-delete .col-kind { color: #FF5C6B; }
  .row.k-conflict .col-kind { color: #FF5C6B; }
  .row.k-conflict_resolved .col-kind { color: #4ADE80; }
  .row.k-drift .col-kind { color: #F0B95C; }
  .row.k-bridge .col-kind { color: #8B6BE6; }
  .row.k-block .col-kind { color: #F0B95C; }
  .row.k-error .col-kind { color: #FF5C6B; }
  .row.k-system .col-kind { color: #7A7A85; }
  .empty {
    padding: 24px; color: #7A7A85;
    text-align: center; font-size: 12px;
  }
</style>
