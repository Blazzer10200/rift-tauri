<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Folder, FileCode, File, ChevronRight } from "lucide-svelte";
  import PathBreadcrumbs from "./PathBreadcrumbs.svelte";
  import LockBadge from "./LockBadge.svelte";
  import { connection } from "../../state/connection.svelte";

  type LocalEntry = {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
    mtime: number;
  };

  type Props = {
    path: string;
    onPathChange: (next: string) => void;
    onOpenInNewTab: (entry: LocalEntry) => void;
    onDropPaths?: (remoteOrLocalPaths: string[]) => void;
    onSelectionChange?: (paths: string[]) => void;
  };
  let { path, onPathChange, onOpenInNewTab, onDropPaths, onSelectionChange }: Props = $props();

  let entries = $state<LocalEntry[]>([]);
  let filter = $state("");
  let selected = $state<Set<string>>(new Set());
  let error = $state<string | null>(null);
  let loading = $state(false);
  let menuFor = $state<LocalEntry | null>(null);
  let menuPos = $state<{ x: number; y: number }>({ x: 0, y: 0 });

  const winSep: "\\" = "\\";

  const filtered = $derived(
    filter.trim() === ""
      ? entries
      : entries.filter((e) => e.name.toLowerCase().includes(filter.toLowerCase())),
  );

  $effect(() => {
    void path;
    void load();
  });

  $effect(() => {
    onSelectionChange?.([...selected]);
  });

  async function load() {
    if (!path) { entries = []; return; }
    loading = true;
    error = null;
    try {
      entries = await invoke<LocalEntry[]>("local_list_dir", { path });
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

  function fmtTime(secs: number): string {
    if (!secs) return "—";
    const d = new Date(secs * 1000);
    return d.toLocaleString();
  }

  function parentOf(p: string): string | null {
    const norm = p.replaceAll("\\", "/").replace(/\/+$/, "");
    const idx = norm.lastIndexOf("/");
    if (idx <= 0) return null;
    if (/^[A-Za-z]:$/.test(norm.slice(0, idx))) {
      return norm.slice(0, idx) + "\\";
    }
    return norm.slice(0, idx).replaceAll("/", "\\");
  }

  function rowStatus(e: LocalEntry): "conflict" | "synced" {
    if (e.is_dir) return "synced";
    const inConflict = connection.conflicts.some((c) => {
      const cBase = c.local_path.replace(/[\\/]+$/, "").split(/[\\/]/).pop();
      return cBase === e.name;
    });
    return inConflict ? "conflict" : "synced";
  }

  function pickIcon(e: LocalEntry) {
    if (e.is_dir) return Folder;
    if (/\.(lua|ts|js|py|rs|cs|go|rb|json|yml|yaml|toml)$/i.test(e.name)) return FileCode;
    return File;
  }

  function onRowClick(e: MouseEvent, entry: LocalEntry) {
    if (e.shiftKey || e.ctrlKey || e.metaKey) {
      const next = new Set(selected);
      if (next.has(entry.path)) next.delete(entry.path);
      else next.add(entry.path);
      selected = next;
    } else {
      selected = new Set([entry.path]);
    }
  }

  function onRowDblClick(entry: LocalEntry) {
    if (entry.is_dir) {
      onPathChange(entry.path);
    }
  }

  function onContextMenu(e: MouseEvent, entry: LocalEntry) {
    e.preventDefault();
    menuFor = entry;
    menuPos = { x: e.clientX, y: e.clientY };
  }

  function closeMenu() { menuFor = null; }

  function onDragStart(e: DragEvent, entry: LocalEntry) {
    if (!e.dataTransfer) return;
    let paths: string[];
    if (selected.has(entry.path) && selected.size > 1) {
      paths = entries.filter((x) => selected.has(x.path)).map((x) => x.path);
    } else {
      paths = [entry.path];
      selected = new Set([entry.path]);
    }
    e.dataTransfer.setData("application/x-rift-local", JSON.stringify(paths));
    e.dataTransfer.effectAllowed = "copy";
  }

  function onDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes("application/x-rift-remote")) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    }
  }

  function onDrop(e: DragEvent) {
    const data = e.dataTransfer?.getData("application/x-rift-remote");
    if (!data) return;
    e.preventDefault();
    try {
      const remotePaths = JSON.parse(data) as string[];
      onDropPaths?.(remotePaths);
    } catch {}
  }
</script>

<svelte:window onclick={closeMenu} />

<section class="pane" data-side="local">
  <PathBreadcrumbs
    side="local"
    {path}
    sep={winSep}
    onNavigate={(p) => onPathChange(p)}
    onRefresh={() => load()}
    filterValue={filter}
    onFilterChange={(v) => (filter = v)}
  />
  <div class="headcols">
    <div class="col col-name">name</div>
    <div class="col col-status" title="sync status">●</div>
    <div class="col col-size">size</div>
    <div class="col col-mtime">modified</div>
  </div>
  {#if error}
    <div class="err mono">{error}</div>
  {/if}
  <div
    class="body"
    role="presentation"
    ondragover={onDragOver}
    ondrop={onDrop}
  >
    {#if loading}
      <div class="empty">Loading…</div>
    {:else if !path}
      <div class="empty">No local root.</div>
    {:else}
      <button
        type="button"
        class="row up-row"
        disabled={!parentOf(path)}
        onclick={() => { const p = parentOf(path); if (p) onPathChange(p); }}
      >
        <span class="row-name"><span class="twirl"></span>↑ ..</span>
        <span class="row-status"></span>
        <span class="row-size mono">—</span>
        <span class="row-mtime mono">—</span>
      </button>
      {#if filtered.length === 0}
        <div class="empty">{filter ? "No matches." : "Empty."}</div>
      {:else}
        {#each filtered as e (e.path)}
          {@const status = rowStatus(e)}
          {@const lk = !e.is_dir ? (() => { const rp = connection.remoteForLocalPath(e.path); return rp ? connection.lockForRemotePath(rp) : null; })() : null}
          {@const Icon = pickIcon(e)}
          <div
            class="row"
            data-selected={selected.has(e.path)}
            data-status={status}
            draggable="true"
            ondragstart={(ev) => onDragStart(ev, e)}
            onclick={(ev) => onRowClick(ev, e)}
            ondblclick={() => onRowDblClick(e)}
            oncontextmenu={(ev) => onContextMenu(ev, e)}
            role="row"
            tabindex="0"
            onkeydown={(ev) => { if (ev.key === "Enter") onRowDblClick(e); }}
          >
            <span class="row-name">
              {#if e.is_dir}
                <span class="twirl"><ChevronRight size={10}/></span>
                <Folder size={13}/>
              {:else}
                <span class="twirl"></span>
                <Icon size={13}/>
              {/if}
              <span class="row-label mono">{e.name}</span>
              {#if lk}
                <LockBadge holder={`${lk.user}@${lk.host}`} tooltip={`Locked remotely by ${lk.user}@${lk.host}`} />
              {/if}
            </span>
            <span class="row-status" title={status}>
              {#if status === "conflict"}
                <span class="sym danger">▲</span>
              {:else}
                <span class="sym muted">·</span>
              {/if}
            </span>
            <span class="row-size mono">{fmtSize(e.size, e.is_dir)}</span>
            <span class="row-mtime mono">{fmtTime(e.mtime)}</span>
          </div>
        {/each}
      {/if}
    {/if}
  </div>
  <div class="foot mono dim">
    {filtered.length} {filtered.length === 1 ? "item" : "items"}{selected.size ? ` · ${selected.size} selected` : ""}
  </div>
</section>

{#if menuFor}
  <div
    class="ctxmenu"
    role="menu"
    style="left:{menuPos.x}px; top:{menuPos.y}px"
  >
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
    background: var(--bg);
  }
  .err {
    color: var(--danger); padding: 8px 12px;
    background: var(--danger-soft);
    font-size: var(--fs-xs);
    border-bottom: 1px solid color-mix(in oklch, var(--danger) 30%, transparent);
  }

  .headcols, .row {
    display: grid;
    grid-template-columns: 1fr 24px 80px 140px;
    gap: 8px;
    align-items: center;
    font-size: var(--fs-sm);
  }
  .headcols {
    padding: 4px 12px;
    color: var(--fg-subtle);
    font-weight: 600;
    font-size: var(--fs-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev-1);
    position: sticky; top: 0; z-index: 1;
  }
  .col-status { text-align: center; }
  .body { flex: 1; overflow: auto; min-height: 0; padding: 4px 0; }
  .row {
    padding: 0 12px;
    height: var(--row-h);
    color: var(--fg);
    cursor: default;
    border-left: 2px solid transparent;
    background: transparent;
    text-align: left;
  }
  .row:hover { background: var(--surface-hover); }
  .row[data-selected="true"] {
    background: var(--accent-soft);
    border-left-color: var(--accent);
  }
  .row[data-status="conflict"] .row-label { color: var(--danger); }
  .up-row {
    width: 100%;
    border: 0;
    font: inherit;
  }
  .up-row:disabled { opacity: 0.4; cursor: not-allowed; }
  .row-name { display: inline-flex; align-items: center; gap: 6px; min-width: 0; overflow: hidden; }
  .row-label { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .twirl { display: inline-flex; width: 12px; color: var(--fg-faint); }
  .row-status { text-align: center; }
  .sym { font-size: 11px; }
  .sym.danger { color: var(--danger); }
  .sym.muted { color: var(--fg-faint); }
  .row-size, .row-mtime { color: var(--fg-subtle); font-size: var(--fs-xs); }

  .empty { padding: 22px; color: var(--fg-muted); font-size: var(--fs-sm); text-align: center; }

  .foot {
    padding: 4px 12px;
    border-top: 1px solid var(--border);
    background: var(--bg-elev-1);
    font-size: var(--fs-xs);
  }

  .ctxmenu {
    position: fixed; z-index: 100;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    min-width: 160px; padding: 4px;
  }
  .ctxmenu button {
    display: block; width: 100%; text-align: left;
    background: transparent; border: 0;
    color: var(--fg); font-size: var(--fs-sm);
    padding: 6px 10px; border-radius: var(--radius-xs);
    cursor: pointer;
  }
  .ctxmenu button:hover { background: var(--surface-hover); color: var(--accent); }
</style>
