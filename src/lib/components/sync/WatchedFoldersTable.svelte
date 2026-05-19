<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Folder, Lock as LockIcon } from "lucide-svelte";
  import EmptyState from "../shell/EmptyState.svelte";
  import { connection } from "../../state/connection.svelte";
  import { workspace } from "../../state/workspace.svelte";

  type WatchedFolderInfo = {
    name: string;
    remote_root: string;
    file_count: number;
  };

  let folders = $state<WatchedFolderInfo[]>([]);
  let loading = $state(true);
  let unlistenDiag: UnlistenFn | null = null;

  async function refresh() {
    try {
      folders = await invoke<WatchedFolderInfo[]>("list_watched_folders");
    } catch (e) {
      console.error("list_watched_folders failed", e);
      folders = [];
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    await refresh();
    try {
      unlistenDiag = await listen<{ stage: string }>("diag://event", (e) => {
        if (e.payload.stage === "drift_scan_result") void refresh();
      });
    } catch (e) {
      console.error("WatchedFoldersTable: diag listener wire failed", e);
    }
  });

  onDestroy(() => { if (unlistenDiag) unlistenDiag(); });

  // Last activity timestamp per resource — newest match wins (feed is newest-first).
  function lastEventFor(name: string): string | null {
    for (const r of connection.activityFeed) {
      if (r.resource === name) return r.at;
    }
    return null;
  }

  // Lock count per resource. Lock paths look like "/opt/.../{resource}/..."; match on the
  // selected remoteRoot + resource name segment so cross-server name collisions stay scoped.
  function lockCountFor(remoteRoot: string): number {
    const root = remoteRoot.replace(/\/+$/, "") + "/";
    let n = 0;
    for (const l of connection.locks) {
      if (l.file_path.startsWith(root)) n++;
    }
    return n;
  }

  function fmtRel(iso: string | null): string {
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

  function deeplink(name: string) {
    connection.activityFilter = name;
    workspace.setActive("activity");
  }
</script>

<section class="card" aria-labelledby="watched-folders-title">
  <header class="card-head">
    <Folder size={13} />
    <h3 id="watched-folders-title">Watched folders</h3>
    {#if folders.length > 0}
      <span class="count mono">{folders.length}</span>
    {/if}
  </header>

  {#if loading}
    <div class="loading-row mono">Loading…</div>
  {:else if folders.length === 0}
    <EmptyState
      icon={Folder}
      tone="neutral"
      title="No folders watched"
      hint="Connect a server to start watching its top-level resources."
    />
  {:else}
    <div class="table" role="table">
      <div class="th" role="row">
        <div role="columnheader">Name</div>
        <div role="columnheader" class="right">Files</div>
        <div role="columnheader" class="right">Last</div>
        <div role="columnheader" class="right">Locks</div>
      </div>
      {#each folders as f, i (f.remote_root)}
        {@const last = lastEventFor(f.name)}
        {@const locks = lockCountFor(f.remote_root)}
        <button
          type="button"
          class="row"
          role="row"
          onclick={() => deeplink(f.name)}
          title="Open Activity filtered to {f.name}"
          in:fly={{
            y: 4,
            duration: 200,
            delay: Math.min(i * 8, 80),
            easing: quintOut,
          }}
        >
          <span class="name mono">{f.name}</span>
          <span class="right mono">{f.file_count.toLocaleString()}</span>
          <span class="right mono" title={last ?? "no events"}>{fmtRel(last)}</span>
          <span class="right">
            {#if locks > 0}
              <span class="lock-pip" in:fade={{ duration: 140 }}>
                <LockIcon size={10}/> {locks}
              </span>
            {:else}
              <span class="muted">—</span>
            {/if}
          </span>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .card {
    display: flex; flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elev-1);
    overflow: hidden;
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
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
  }
  .loading-row {
    padding: 14px;
    color: var(--fg-faint);
    font-size: var(--fs-sm);
  }

  .table {
    display: flex; flex-direction: column;
  }
  .th, .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 80px 60px 80px;
    align-items: center;
    column-gap: 12px;
    padding: 6px 14px;
    font-size: var(--fs-sm);
  }
  .th {
    color: var(--fg-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: var(--fs-xs);
    border-bottom: 1px solid var(--border);
  }
  .right { text-align: right; }

  .row {
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--border);
    text-align: left;
    color: inherit;
    font: inherit;
    cursor: pointer;
    transition: background 120ms ease, box-shadow 120ms ease;
  }
  .row:last-child { border-bottom: 0; }
  .row:hover {
    background: color-mix(in oklch, var(--accent) 6%, var(--bg-elev-1));
  }
  .row:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--ring);
  }
  .row:active {
    background: color-mix(in oklch, var(--accent) 12%, var(--bg-elev-1));
  }
  .name {
    color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .row .mono { font-variant-numeric: tabular-nums; }
  .muted { color: var(--fg-faint); }

  .lock-pip {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--warn-soft);
    color: var(--warn);
    font-size: var(--fs-xs);
    border: 1px solid color-mix(in oklch, var(--warn) 28%, transparent);
    font-variant-numeric: tabular-nums;
  }

  @media (prefers-reduced-motion: reduce) {
    .row { transition: none; }
  }
</style>
