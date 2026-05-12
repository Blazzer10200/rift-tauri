<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { revealItemInDir, openPath } from "@tauri-apps/plugin-opener";
  import { Folder, FileCode, File, ChevronRight, Upload, FolderOpen, Copy, ExternalLink, Pencil, Trash2 } from "lucide-svelte";
  import PathBreadcrumbs from "./PathBreadcrumbs.svelte";
  import LockBadge from "./LockBadge.svelte";
  import { fade } from "svelte/transition";
  import { connection } from "../../state/connection.svelte";
  import { fmtRelative, fmtAbsolute } from "../../utils/time";

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
    onDropPathsToFolder?: (remoteOrLocalPaths: string[], targetLocalDir: string) => void;
    onUploadPaths?: (localPaths: string[]) => void;
    onSelectionChange?: (paths: string[]) => void;
    refreshKey?: number;
  };
  let { path, onPathChange, onOpenInNewTab, onDropPaths, onDropPathsToFolder, onUploadPaths, onSelectionChange, refreshKey = 0 }: Props = $props();

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

  let loadToken = 0;

  $effect(() => {
    void path; void refreshKey;
    void load();
  });

  $effect(() => {
    onSelectionChange?.([...selected]);
  });

  async function load() {
    const token = ++loadToken;
    if (!path) { entries = []; return; }
    loading = true;
    error = null;
    try {
      const result = await invoke<LocalEntry[]>("local_list_dir", { path });
      if (token !== loadToken) return;
      entries = result;
      selected = new Set();
    } catch (e) {
      if (token !== loadToken) return;
      error = String(e);
      entries = [];
    } finally {
      if (token === loadToken) loading = false;
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
    return fmtRelative(new Date(secs * 1000));
  }
  function fmtTimeAbs(secs: number): string {
    if (!secs) return "";
    return fmtAbsolute(new Date(secs * 1000));
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
      // Normalize OS separators for cross-platform path equality.
      const cNorm = c.local_path.replaceAll("\\", "/");
      const eNorm = e.path.replaceAll("\\", "/");
      return cNorm === eNorm;
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
    e.stopPropagation();
    if (!selected.has(entry.path)) selected = new Set([entry.path]);
    menuFor = entry;
    menuPos = { x: e.clientX, y: e.clientY };
  }

  function onBodyContextMenu(e: MouseEvent) {
    e.preventDefault();
    menuFor = null;
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
    } catch (err) {
      console.warn("LocalPane: drop payload parse failed", err);
    }
  }

  let dropTarget = $state<string | null>(null);

  function onFolderDragOver(e: DragEvent, entry: LocalEntry) {
    if (!entry.is_dir) return;
    if (e.dataTransfer?.types.includes("application/x-rift-remote")) {
      e.preventDefault();
      e.stopPropagation();
      e.dataTransfer.dropEffect = "copy";
      dropTarget = entry.path;
    }
  }

  function onFolderDragLeave(entry: LocalEntry) {
    if (dropTarget === entry.path) dropTarget = null;
  }

  function onFolderDrop(e: DragEvent, entry: LocalEntry) {
    if (!entry.is_dir) return;
    const data = e.dataTransfer?.getData("application/x-rift-remote");
    if (!data) return;
    e.preventDefault();
    e.stopPropagation();
    dropTarget = null;
    try {
      const remotePaths = JSON.parse(data) as string[];
      onDropPathsToFolder?.(remotePaths, entry.path);
    } catch (err) {
      console.warn("LocalPane: folder drop parse failed", err);
    }
  }

  function onPaneKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      const tgt = e.target as HTMLElement | null;
      if (tgt && (tgt.tagName === "INPUT" || tgt.tagName === "TEXTAREA")) return;
      e.preventDefault();
      selected = new Set(filtered.map((x) => x.path));
    } else if (e.key === "Escape") {
      selected = new Set();
      menuFor = null;
    }
  }

  async function copyPath(p: string) {
    try { await navigator.clipboard.writeText(p); } catch (err) { console.warn("clipboard failed", err); }
  }

  async function revealInExplorer(p: string) {
    try { await revealItemInDir(p); } catch (err) { console.warn("reveal failed", err); }
  }

  async function openInDefault(p: string) {
    try { await openPath(p); } catch (err) {
      error = `open failed: ${err}`;
    }
  }

  async function renameEntry(entry: LocalEntry) {
    const serverKey = connection.selectedKey;
    if (!serverKey) { error = "no server selected"; return; }
    const next = window.prompt(`Rename "${entry.name}" to:`, entry.name)?.trim();
    if (!next || next === entry.name) return;
    if (next.includes("\\") || next.includes("/")) { error = "name cannot contain '\\' or '/'"; return; }
    const parent = entry.path.slice(0, entry.path.length - entry.name.length);
    const target = parent + next;
    try {
      await invoke("local_rename_path", { serverKey, from: entry.path, to: target });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function deleteEntries(entry: LocalEntry) {
    const serverKey = connection.selectedKey;
    if (!serverKey) { error = "no server selected"; return; }
    const paths = selected.has(entry.path) && selected.size > 1
      ? entries.filter((x) => selected.has(x.path)).map((x) => x.path)
      : [entry.path];
    const label = paths.length === 1 ? `"${entry.name}"` : `${paths.length} items`;
    if (!window.confirm(`Permanently delete ${label} on disk? This cannot be undone.`)) return;
    try {
      const results = await invoke<{ ok: boolean; error: string | null }[]>(
        "local_delete_paths",
        { serverKey, paths },
      );
      const failures = results.filter((r) => !r.ok);
      if (failures.length > 0) {
        const first = failures[0].error ?? "unknown error";
        error = failures.length === 1
          ? `delete failed: ${first}`
          : `delete failed for ${failures.length}/${paths.length} items — first: ${first}`;
      }
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  function uploadEntry(entry: LocalEntry) {
    const paths = selected.has(entry.path) && selected.size > 1
      ? entries.filter((x) => selected.has(x.path)).map((x) => x.path)
      : [entry.path];
    onUploadPaths?.(paths);
  }
</script>

<svelte:window onclick={closeMenu} />

<!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
<section
  class="pane"
  data-side="local"
  tabindex="0"
  aria-label="Local file browser"
  onkeydown={onPaneKeyDown}
>
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
    oncontextmenu={onBodyContextMenu}
  >
    {#if loading}
      <div class="empty">Loading…</div>
    {:else if !path}
      <div class="empty">No local root.</div>
    {:else}
      {#key path}
        <div in:fade={{ duration: 140 }}>
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
                data-droptarget={dropTarget === e.path}
                data-dir={e.is_dir}
                draggable="true"
                ondragstart={(ev) => onDragStart(ev, e)}
                ondragover={(ev) => onFolderDragOver(ev, e)}
                ondragleave={() => onFolderDragLeave(e)}
                ondrop={(ev) => onFolderDrop(ev, e)}
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
                <span class="row-mtime mono" title={fmtTimeAbs(e.mtime)}>{fmtTime(e.mtime)}</span>
              </div>
            {/each}
          {/if}
        </div>
      {/key}
    {/if}
  </div>
  <div class="foot mono dim">
    {filtered.length} {filtered.length === 1 ? "item" : "items"}{selected.size ? ` · ${selected.size} selected` : ""}
  </div>
</section>

{#if menuFor}
  {@const target = menuFor}
  {@const multi = selected.size > 1 && selected.has(target.path)}
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div
    class="ctxmenu"
    role="menu"
    tabindex="-1"
    style="left:{menuPos.x}px; top:{menuPos.y}px"
    onclick={(ev) => ev.stopPropagation()}
    onkeydown={(ev) => { if (ev.key === "Escape") { ev.preventDefault(); closeMenu(); } }}
  >
    {#if target.is_dir && !multi}
      <button type="button" class="ctx-item" onclick={() => { onOpenInNewTab(target); closeMenu(); }}>
        <FolderOpen size={13}/><span>Open in new tab</span>
      </button>
      <div class="ctx-sep"></div>
    {/if}
    <button type="button" class="ctx-item" onclick={() => { uploadEntry(target); closeMenu(); }}>
      <Upload size={13}/><span>Upload to remote{multi ? ` (${selected.size})` : ""}</span>
    </button>
    <div class="ctx-sep"></div>
    {#if !multi}
      {#if !target.is_dir}
        <button type="button" class="ctx-item" onclick={() => { openInDefault(target.path); closeMenu(); }}>
          <ExternalLink size={13}/><span>Open in default editor</span>
        </button>
      {/if}
      <button type="button" class="ctx-item" onclick={() => { revealInExplorer(target.path); closeMenu(); }}>
        <ExternalLink size={13}/><span>Reveal in Explorer</span>
      </button>
      <button type="button" class="ctx-item" onclick={() => { copyPath(target.path); closeMenu(); }}>
        <Copy size={13}/><span>Copy path</span>
      </button>
      <button type="button" class="ctx-item" onclick={() => { renameEntry(target); closeMenu(); }}>
        <Pencil size={13}/><span>Rename…</span>
      </button>
    {/if}
    <div class="ctx-sep"></div>
    <button type="button" class="ctx-item ctx-danger" onclick={() => { deleteEntries(target); closeMenu(); }}>
      <Trash2 size={13}/><span>Delete{multi ? ` (${selected.size})` : ""}</span>
    </button>
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
    transition: background 100ms ease, transform 80ms ease;
  }
  .row:hover { background: var(--surface-hover); }
  @media (prefers-reduced-motion: no-preference) {
    .row[data-dir="true"]:hover { transform: translateX(2px); }
  }
  .row[data-selected="true"] {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 var(--accent);
    border-left-color: transparent;
  }
  .row[data-droptarget="true"] {
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    outline: 1px dashed var(--accent);
    outline-offset: -2px;
  }
  .row[data-status="conflict"] .row-label { color: var(--danger); }
  .up-row {
    width: 100%;
    border: 0;
    font: inherit;
    transition: background 100ms ease;
  }
  .up-row:hover:not(:disabled) { background: var(--surface-hover); }
  .up-row:disabled { opacity: 0.4; cursor: not-allowed; }
  .row-name { display: inline-flex; align-items: center; gap: 6px; min-width: 0; overflow: hidden; }
  .row-label { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .twirl { display: inline-flex; width: 12px; color: var(--fg-faint); }
  .row-status { text-align: center; }
  .sym { font-size: 11px; }
  .sym.danger { color: var(--danger); }
  .sym.muted { color: var(--fg-faint); }
  .row-size, .row-mtime { color: var(--fg-subtle); font-size: var(--fs-xs); white-space: nowrap; }

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
    min-width: 200px; padding: 4px;
  }
  .ctx-item {
    display: flex; align-items: center; gap: 8px;
    width: 100%; text-align: left;
    background: transparent; border: 0;
    color: var(--fg); font-size: var(--fs-sm);
    padding: 6px 10px; border-radius: var(--radius-xs);
    cursor: pointer;
  }
  .ctx-item :global(svg) { color: var(--fg-subtle); flex-shrink: 0; }
  .ctx-item:hover { background: var(--surface-hover); color: var(--accent); }
  .ctx-item:hover :global(svg) { color: var(--accent); }
  .ctx-danger:hover { background: var(--danger-soft); color: var(--danger); }
  .ctx-danger:hover :global(svg) { color: var(--danger); }
  .ctx-sep {
    height: 1px;
    background: var(--border);
    margin: 4px 2px;
  }
</style>
