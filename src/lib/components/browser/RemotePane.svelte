<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import PathBreadcrumbs from "./PathBreadcrumbs.svelte";
  import LockBadge from "./LockBadge.svelte";
  import { connection } from "../../state/connection.svelte";

  type RemoteEntry = {
    full_path: string;
    name: string;
    is_dir: boolean;
    size: number;
    last_modified: string;
  };

  type Props = {
    serverKey: string | null;
    path: string;
    onPathChange: (next: string) => void;
    onOpenInNewTab: (entry: RemoteEntry) => void;
    onDropPaths?: (localPaths: string[]) => void;
  };
  let { serverKey, path, onPathChange, onOpenInNewTab, onDropPaths }: Props = $props();

  let entries = $state<RemoteEntry[]>([]);
  let filter = $state("");
  let selected = $state<Set<string>>(new Set());
  let error = $state<string | null>(null);
  let loading = $state(false);
  let menuFor = $state<RemoteEntry | null>(null);
  let menuPos = $state<{ x: number; y: number }>({ x: 0, y: 0 });

  const posixSep: "/" = "/";

  const filtered = $derived(
    filter.trim() === ""
      ? entries
      : entries.filter((e) => e.name.toLowerCase().includes(filter.toLowerCase())),
  );

  $effect(() => {
    void path; void serverKey;
    void load();
  });

  async function load() {
    if (!serverKey || !path) { entries = []; return; }
    loading = true;
    error = null;
    try {
      entries = await invoke<RemoteEntry[]>("remote_list_dir", { serverKey, path });
      selected = new Set();
    } catch (e) {
      error = String(e);
      entries = [];
    } finally {
      loading = false;
    }
  }

  function fmtSize(n: number, isDir: boolean): string {
    if (isDir) return "—";
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function fmtTime(iso: string): string {
    if (!iso) return "—";
    try { return new Date(iso).toLocaleString(); } catch { return iso; }
  }

  function parentOf(p: string): string | null {
    const norm = p.replace(/\/+$/, "");
    if (!norm || norm === "/") return null;
    const idx = norm.lastIndexOf("/");
    if (idx <= 0) return "/";
    return norm.slice(0, idx);
  }

  function onRowClick(e: MouseEvent, entry: RemoteEntry) {
    if (e.shiftKey || e.ctrlKey || e.metaKey) {
      const next = new Set(selected);
      if (next.has(entry.full_path)) next.delete(entry.full_path);
      else next.add(entry.full_path);
      selected = next;
    } else {
      selected = new Set([entry.full_path]);
    }
  }

  function onRowDblClick(entry: RemoteEntry) {
    if (entry.is_dir) onPathChange(entry.full_path);
  }

  function onContextMenu(e: MouseEvent, entry: RemoteEntry) {
    e.preventDefault();
    menuFor = entry;
    menuPos = { x: e.clientX, y: e.clientY };
  }

  function closeMenu() { menuFor = null; }

  function onDragStart(e: DragEvent, entry: RemoteEntry) {
    if (!e.dataTransfer) return;
    let paths: string[];
    if (selected.has(entry.full_path) && selected.size > 1) {
      paths = entries.filter((x) => selected.has(x.full_path)).map((x) => x.full_path);
    } else {
      paths = [entry.full_path];
      selected = new Set([entry.full_path]);
    }
    e.dataTransfer.setData("application/x-rift-remote", JSON.stringify(paths));
    e.dataTransfer.effectAllowed = "copy";
  }

  function onDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes("application/x-rift-local")) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    }
  }

  function onDrop(e: DragEvent) {
    const data = e.dataTransfer?.getData("application/x-rift-local");
    if (!data) return;
    e.preventDefault();
    try {
      const localPaths = JSON.parse(data) as string[];
      onDropPaths?.(localPaths);
    } catch {}
  }
</script>

<svelte:window onclick={closeMenu} />

<section class="pane">
  <PathBreadcrumbs {path} sep={posixSep} onNavigate={(p) => onPathChange(p)} />
  <div class="toolbar">
    <button
      type="button"
      class="up"
      disabled={!parentOf(path)}
      onclick={() => { const p = parentOf(path); if (p) onPathChange(p); }}
    >↑ Up</button>
    <input
      class="filter"
      placeholder="Filter…"
      bind:value={filter}
      type="text"
    />
  </div>
  {#if error}
    <div class="err">{error}</div>
  {/if}
  <div
    class="grid"
    role="presentation"
    ondragover={onDragOver}
    ondrop={onDrop}
  >
    <div class="header">
      <span class="col-name">Name</span>
      <span class="col-size">Size</span>
      <span class="col-mtime">Modified</span>
    </div>
    <div class="body">
      {#if !serverKey}
        <div class="empty">No server selected.</div>
      {:else if loading}
        <div class="empty">Loading…</div>
      {:else if filtered.length === 0}
        <div class="empty">{filter ? "No matches." : "Empty."}</div>
      {:else}
        {#each filtered as e (e.full_path)}
          <div
            class="row"
            class:dir={e.is_dir}
            class:sel={selected.has(e.full_path)}
            draggable="true"
            ondragstart={(ev) => onDragStart(ev, e)}
            onclick={(ev) => onRowClick(ev, e)}
            ondblclick={() => onRowDblClick(e)}
            oncontextmenu={(ev) => onContextMenu(ev, e)}
            role="row"
            tabindex="0"
            onkeydown={(ev) => { if (ev.key === "Enter") onRowDblClick(e); }}
          >
            <span class="col-name">
              <span class="icon">{e.is_dir ? "▸" : "·"}</span>
              {e.name}
              {#if !e.is_dir}
                {@const lk = connection.lockForRemotePath(e.full_path)}
                {#if lk}
                  <LockBadge holder={`${lk.user}@${lk.host}`} tooltip={`Locked by ${lk.user}@${lk.host} since ${new Date(lk.since).toLocaleString()}`} />
                {/if}
              {/if}
            </span>
            <span class="col-size">{fmtSize(e.size, e.is_dir)}</span>
            <span class="col-mtime">{fmtTime(e.last_modified)}</span>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</section>

{#if menuFor}
  <div class="ctxmenu" role="menu" style="left:{menuPos.x}px; top:{menuPos.y}px">
    {#if menuFor.is_dir}
      <button type="button" onclick={() => { if (menuFor) onOpenInNewTab(menuFor); closeMenu(); }}>
        Open in new tab
      </button>
    {/if}
  </div>
{/if}

<style>
  .pane {
    display: flex; flex-direction: column;
    flex: 1; min-width: 0; min-height: 0;
    background: #0F0F12;
  }
  .toolbar {
    display: flex; gap: 8px; padding: 6px 8px;
    border-bottom: 1px solid #26262E; background: #0F0F12;
  }
  .up {
    background: #17171C; color: #E8E8EE;
    border: 1px solid #26262E; border-radius: 3px;
    padding: 3px 10px; font-size: 12px; cursor: pointer;
  }
  .up:hover:not(:disabled) { border-color: #8B6BE6; }
  .up:disabled { opacity: 0.4; cursor: not-allowed; }
  .filter {
    flex: 1; background: #17171C; color: #E8E8EE;
    border: 1px solid #26262E; border-radius: 3px;
    padding: 3px 8px; font-size: 12px;
  }
  .filter:focus { outline: 0; border-color: #8B6BE6; }
  .err {
    color: #FF5C6B; padding: 8px 12px;
    background: #15101E; font-size: 12px; font-family: Consolas, monospace;
    border-bottom: 1px solid #26262E;
  }
  .grid {
    flex: 1; overflow: auto; min-height: 0;
    display: flex; flex-direction: column;
  }
  .header, .row {
    display: grid;
    grid-template-columns: 1fr 80px 140px;
    gap: 8px; padding: 4px 12px;
    align-items: center; font-size: 12px;
  }
  .header {
    color: #7A7A85; font-weight: 600;
    border-bottom: 1px solid #26262E; background: #17171C;
    position: sticky; top: 0;
  }
  .body { flex: 1; }
  .row { border-bottom: 1px solid #15101E; color: #E8E8EE; cursor: default; }
  .row:hover { background: #17171C; }
  .row.sel { background: #15101E; outline: 1px solid #8B6BE6; outline-offset: -1px; }
  .row.dir .col-name { color: #8B6BE6; }
  .icon { display: inline-block; width: 14px; color: #7A7A85; }
  .col-size, .col-mtime { color: #7A7A85; font-family: Consolas, monospace; }
  .empty { padding: 18px 12px; color: #7A7A85; font-size: 12px; text-align: center; }
  .ctxmenu {
    position: fixed; z-index: 100;
    background: #17171C; border: 1px solid #26262E; border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    min-width: 160px; padding: 4px;
  }
  .ctxmenu button {
    display: block; width: 100%; text-align: left;
    background: transparent; border: 0;
    color: #E8E8EE; font-size: 12px;
    padding: 6px 10px; border-radius: 3px; cursor: pointer;
  }
  .ctxmenu button:hover { background: #15101E; color: #8B6BE6; }
</style>
