<script lang="ts">
  import {
    RefreshCw, Download, Trash2, AlertTriangle, Check,
    GitBranch, Network, Lock, XCircle, Info, Pause, Play,
  } from "lucide-svelte";
  import { connection, type ActivityRow, type ActivityKind } from "../../state/connection.svelte";

  type Group = "all" | "sync" | "pull" | "delete" | "drift" | "conflict" | "bridge" | "error" | "system";

  let filter = $state("");
  let group = $state<Group>("all");
  let paused = $state(false);
  let frozen = $state<ActivityRow[]>([]);

  const groups: { id: Group; label: string }[] = [
    { id: "all", label: "All" },
    { id: "sync", label: "Sync" },
    { id: "pull", label: "Pull" },
    { id: "delete", label: "Delete" },
    { id: "drift", label: "Drift" },
    { id: "conflict", label: "Conflicts" },
    { id: "bridge", label: "Bridge" },
    { id: "error", label: "Errors" },
    { id: "system", label: "System" },
  ];

  const source = $derived(paused ? frozen : connection.activityFeed);

  function inGroup(r: ActivityRow, g: Group): boolean {
    if (g === "all") return true;
    if (g === "conflict") return r.kind === "conflict" || r.kind === "conflict_resolved";
    return r.kind === g;
  }

  function countFor(g: Group): number {
    return connection.activityFeed.filter((r) => inGroup(r, g)).length;
  }

  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    return source.filter((r) => {
      if (!inGroup(r, group)) return false;
      if (!q) return true;
      return (
        r.resource.toLowerCase().includes(q) ||
        r.file.toLowerCase().includes(q) ||
        r.action.toLowerCase().includes(q)
      );
    });
  });

  // Windowed virtualization
  const ROW_H = 36;
  const OVERSCAN = 6;
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
    try {
      return new Date(iso).toLocaleTimeString([], { hour12: false });
    } catch { return iso; }
  }

  type Variant = "ok" | "warn" | "danger" | "info" | "muted";

  function kindIcon(k: ActivityKind) {
    switch (k) {
      case "sync": return RefreshCw;
      case "pull": return Download;
      case "delete": return Trash2;
      case "conflict": return AlertTriangle;
      case "conflict_resolved": return Check;
      case "drift": return GitBranch;
      case "bridge": return Network;
      case "block": return Lock;
      case "error": return XCircle;
      case "system": return Info;
    }
  }

  function kindVariant(k: ActivityKind): Variant {
    switch (k) {
      case "sync":
      case "conflict_resolved": return "ok";
      case "pull":
      case "bridge": return "info";
      case "drift":
      case "delete":
      case "block": return "warn";
      case "conflict":
      case "error": return "danger";
      default: return "muted";
    }
  }

  function togglePause() {
    if (paused) {
      paused = false;
      frozen = [];
    } else {
      frozen = [...connection.activityFeed];
      paused = true;
    }
  }
</script>

<section class="feed">
  <div class="toolbar">
    <div class="segctl">
      {#each groups as g (g.id)}
        {@const n = g.id === "all" ? connection.activityFeed.length : countFor(g.id)}
        <button
          type="button"
          data-active={group === g.id}
          onclick={() => (group = g.id)}
        >
          {g.label}
          {#if g.id !== "all"}
            <span class="pip" data-zero={n === 0}>{n}</span>
          {/if}
        </button>
      {/each}
    </div>

    <input
      class="filter"
      type="text"
      placeholder="Filter resource / file / action…"
      bind:value={filter}
    />

    <div class="actions">
      <button class="btn ghost sm" type="button" onclick={togglePause} title={paused ? "Resume feed" : "Pause feed"}>
        {#if paused}
          <Play size={11}/> Resume
        {:else}
          <Pause size={11}/> Pause
        {/if}
      </button>
      <button
        class="btn ghost sm"
        type="button"
        onclick={() => connection.clearActivity()}
        disabled={connection.activityFeed.length === 0}
        title="Clear feed"
      >
        <Trash2 size={11}/> Clear
      </button>
    </div>
  </div>

  <div class="list" bind:this={scroller}>
    {#if filtered.length === 0}
      <div class="empty">
        {connection.activityFeed.length === 0
          ? "No activity yet — start auto-sync to see events."
          : "No matches."}
      </div>
    {:else}
      <div style="height:{padTop}px"></div>
      {#each slice as r (r.at + "_" + r.resource + "_" + r.file + "_" + r.action)}
        {@const Icon = kindIcon(r.kind)}
        {@const v = kindVariant(r.kind)}
        <div class="row" style="height:{ROW_H}px">
          <span class="time mono">{fmtTime(r.at)}</span>
          <span class="kind" data-variant={v} title={r.kind}>
            <Icon size={12}/>
          </span>
          <div class="text">
            <span class="label">{r.action}</span>
            <span class="detail mono" title={r.file}>{r.resource}{r.file ? ` · ${r.file}` : ""}</span>
          </div>
        </div>
      {/each}
      <div style="height:{padBot}px"></div>
    {/if}
  </div>

  {#if paused}
    <div class="paused-banner mono">
      Feed paused — {Math.max(0, connection.activityFeed.length - frozen.length)} new since pause
    </div>
  {/if}
</section>

<style>
  .feed {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    padding: 10px 14px 14px;
    background: var(--bg);
    color: var(--fg);
    position: relative;
  }

  .toolbar {
    display: flex; align-items: center; gap: 8px;
    margin-bottom: 8px;
  }

  .segctl {
    display: inline-flex; padding: 2px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    gap: 1px;
    overflow-x: auto;
    max-width: 60%;
  }
  .segctl button {
    padding: 3px 9px; height: 22px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    border-radius: var(--radius-xs);
    cursor: pointer;
    display: inline-flex; align-items: center; gap: 6px;
    white-space: nowrap;
  }
  .segctl button:hover { color: var(--fg); }
  .segctl button[data-active="true"] {
    background: var(--surface);
    color: var(--fg);
    box-shadow: var(--shadow-sm);
  }
  .pip {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 16px; height: 14px; padding: 0 4px;
    border-radius: 7px;
    background: var(--bg-elev-3);
    color: var(--fg-subtle);
    font-size: 10px; line-height: 1;
    font-variant-numeric: tabular-nums;
  }
  .pip[data-zero="true"] { opacity: 0.45; }

  .filter {
    flex: 1; min-width: 0;
    background: var(--bg-elev-1);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 4px 8px;
    font: inherit; font-size: var(--fs-sm);
  }
  .filter:focus {
    outline: 0;
    border-color: var(--accent);
    background: var(--bg-elev-2);
  }

  .actions { display: inline-flex; gap: 6px; }

  .list {
    flex: 1; min-height: 0; overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .row {
    display: grid;
    grid-template-columns: 78px 28px 1fr;
    gap: 10px;
    align-items: center;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
  }
  .row:last-child { border-bottom: 0; }
  .row:hover { background: var(--surface-hover); }

  .time { color: var(--fg-subtle); font-size: var(--fs-xs); }
  .kind {
    width: 22px; height: 22px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-2); color: var(--fg-muted);
  }
  .kind[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kind[data-variant="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .kind[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
  .kind[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }

  .text { display: flex; flex-direction: column; min-width: 0; }
  .label {
    color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .detail {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  .empty {
    padding: 24px;
    color: var(--fg-muted);
    text-align: center;
    font-size: var(--fs-sm);
  }

  .paused-banner {
    position: absolute;
    bottom: 22px; left: 50%; transform: translateX(-50%);
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 4px 10px;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    box-shadow: var(--shadow);
  }
</style>
