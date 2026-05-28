<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import {
    Activity as ActivityIcon, ArrowRight,
    RefreshCw, Download, Trash2, AlertTriangle, Check,
    GitBranch, Network, Lock as LockIcon, XCircle, Info,
  } from "lucide-svelte";
  import EmptyState from "../shell/EmptyState.svelte";
  import { connection, type ActivityKind } from "../../state/connection.svelte";
  import { workspace } from "../../state/workspace.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  const tail = $derived(connection.activityFeed.slice(0, 5));

  function kindIcon(k: ActivityKind) {
    switch (k) {
      case "sync": return RefreshCw;
      case "pull": return Download;
      case "delete": return Trash2;
      case "conflict": return AlertTriangle;
      case "conflict_resolved": return Check;
      case "drift": return GitBranch;
      case "bridge": return Network;
      case "block": return LockIcon;
      case "error": return XCircle;
      case "system": return Info;
    }
  }

  type Variant = "ok" | "warn" | "danger" | "info" | "muted";

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

  function fmtRel(iso: string | null | undefined): string {
    if (!iso) return "—";
    const t = new Date(iso).getTime();
    if (Number.isNaN(t)) return "—";
    const diff = Math.floor((Date.now() - t) / 1000);
    if (diff < 0) return "now";
    if (diff < 60) return `${diff}s`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
    return `${Math.floor(diff / 86400)}d`;
  }

  function openActivity() {
    workspace.setActive("activity");
  }
</script>

<section class="card" aria-labelledby="recent-activity-title">
  <header class="card-head">
    <ActivityIcon size={13} />
    <h3 id="recent-activity-title">Recent activity</h3>
    {#if connection.activityFeed.length > 0}
      <button type="button" class="open-link" onclick={openActivity}>
        Open Activity <ArrowRight size={11}/>
      </button>
    {/if}
  </header>

  {#if tail.length === 0}
    <EmptyState
      icon={ActivityIcon}
      tone="neutral"
      title="No recent activity"
      hint="Sync, pull, drift, and bridge events will land here."
    />
  {:else}
    <ul class="rows">
      {#each tail as r, i (r.at + "_" + r.resource + "_" + r.file + "_" + i)}
        {@const Icon = kindIcon(r.kind)}
        {@const v = kindVariant(r.kind)}
        <li
          class="row"
          in:fly={{
            y: 4,
            duration: 200,
            delay: Math.min(i * 8, 80),
            easing: quintOut,
          }}
        >
          <span class="time mono" use:tooltip={new Date(r.at).toLocaleString([], { hour12: true })}>{fmtRel(r.at)}</span>
          <span class="kchip" data-variant={v}><Icon size={10}/></span>
          <span class="resource mono">{r.resource ? `[${r.resource}]` : "—"}</span>
          <span class="path mono" use:tooltip={r.rel_path ?? r.file}>{r.rel_path ?? r.file ?? "—"}</span>
          <span class="action" use:tooltip={r.action}>{r.action}</span>
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
  .open-link {
    margin-left: auto;
    background: transparent;
    border: 0;
    color: var(--accent);
    font: inherit;
    font-size: var(--fs-xs);
    cursor: pointer;
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    transition: background 120ms ease;
  }
  .open-link:hover { background: var(--accent-soft); }
  .open-link:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--ring);
  }

  .rows {
    list-style: none; margin: 0; padding: 0;
    display: flex; flex-direction: column;
  }
  .row {
    display: grid;
    grid-template-columns: 38px 18px 90px minmax(0, 1fr) minmax(0, 1.2fr);
    align-items: center;
    column-gap: 10px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
    min-width: 0;
  }
  .row:last-child { border-bottom: 0; }
  .time { color: var(--fg-faint); font-size: var(--fs-xs); }
  .resource { color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .path, .action {
    color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    min-width: 0;
  }
  .action { color: var(--fg-muted); }

  .kchip {
    width: 18px; height: 18px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-2);
    color: var(--fg-muted);
  }
  .kchip[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kchip[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }
  .kchip[data-variant="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .kchip[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
</style>
