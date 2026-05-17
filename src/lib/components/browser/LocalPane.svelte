<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { revealItemInDir, openPath } from "@tauri-apps/plugin-opener";
  import { Folder, FileCode, File, Upload, FolderOpen, Copy, ExternalLink, Pencil, Trash2 } from "lucide-svelte";
  import PathBreadcrumbs from "./PathBreadcrumbs.svelte";
  import LockBadge from "./LockBadge.svelte";
  import { fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
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
    // On destroy / next re-run, bump the token so any in-flight `load()`
    // bails at its `token !== loadToken` guard. Defensive — pairs with the
    // existing stale-overwrite guard inside load().
    return () => { loadToken++; };
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
    if (isDir) return "";
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
    const MENU_W = 240;
    const MENU_H = 320;
    const PAD = 8;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let x = e.clientX;
    let y = e.clientY;
    if (x + MENU_W > vw) x = Math.max(PAD, vw - MENU_W - PAD);
    if (y + MENU_H > vh) y = Math.max(PAD, vh - MENU_H - PAD);
    menuPos = { x, y };
  }

  function onBodyContextMenu(e: MouseEvent) {
    e.preventDefault();
    menuFor = null;
  }

  function onBodyClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (target && target.closest(".row")) return;
    selected = new Set();
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

  function isEditableTarget(t: EventTarget | null): boolean {
    const el = t as HTMLElement | null;
    return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);
  }
  function onPaneKeyDown(e: KeyboardEvent) {
    if (isEditableTarget(e.target)) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      menuFor = null;
      selected = new Set(filtered.map((x) => x.path));
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "c") {
      if (selected.size === 0) return;
      e.preventDefault();
      menuFor = null;
      const text = [...selected].join("\n");
      try { void navigator.clipboard.writeText(text); } catch (err) { console.warn("clipboard failed", err); }
    } else if (e.key === "F2") {
      if (selected.size !== 1) return;
      e.preventDefault();
      menuFor = null;
      const [p] = selected;
      const ent = entries.find((x) => x.path === p);
      if (ent) void renameEntry(ent);
    } else if (e.key === "Delete") {
      if (selected.size === 0) return;
      e.preventDefault();
      menuFor = null;
      const firstPath = [...selected][0];
      const ent = entries.find((x) => x.path === firstPath);
      if (ent) void deleteEntries(ent);
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

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
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
    onclick={onBodyClick}
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
            <span class="row-name up-name">↑ ..</span>
            <span class="row-status"></span>
            <span class="row-size mono"></span>
            <span class="row-mtime mono"></span>
          </button>
          {#if filtered.length === 0}
            <div class="empty">
              <span class="empty-title">{filter ? "No matches" : "Empty folder"}</span>
              <span class="empty-hint">{filter ? `Nothing matches "${filter}".` : "Drop files here, or use ↑ to go up."}</span>
            </div>
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
                    <Folder size={14} class="row-icon row-icon-dir"/>
                  {:else}
                    <Icon size={13} class="row-icon"/>
                  {/if}
                  <span class="row-label mono">{e.name}</span>
                  {#if lk}
                    <LockBadge holder={`${lk.user}@${lk.host}`} tooltip={`Locked remotely by ${lk.user}@${lk.host}`} />
                  {/if}
                </span>
                <span class="row-status" title={status}>
                  {#if status === "conflict"}
                    <span class="sym danger">▲</span>
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
    transition:scale={{ start: 0.94, duration: 110, easing: quintOut }}
    onclick={(ev) => ev.stopPropagation()}
    onkeydown={(ev) => { if (ev.key === "Escape") { ev.preventDefault(); closeMenu(); } }}
  >
    {#if target.is_dir && !multi}
      <button type="button" class="ctx-item" data-tone="accent" onclick={() => { onOpenInNewTab(target); closeMenu(); }}>
        <FolderOpen size={13}/><span class="ctx-label">Open in new tab</span>
      </button>
      <div class="ctx-sep"></div>
    {/if}
    <button type="button" class="ctx-item" data-tone="warn" onclick={() => { uploadEntry(target); closeMenu(); }}>
      <Upload size={13}/><span class="ctx-label">Upload to remote{multi ? ` (${selected.size})` : ""}</span>
    </button>
    <div class="ctx-sep"></div>
    {#if !multi}
      {#if !target.is_dir}
        <button type="button" class="ctx-item" data-tone="info" onclick={() => { openInDefault(target.path); closeMenu(); }}>
          <ExternalLink size={13}/><span class="ctx-label">Open in default editor</span>
        </button>
      {/if}
      <button type="button" class="ctx-item" data-tone="neutral" onclick={() => { revealInExplorer(target.path); closeMenu(); }}>
        <ExternalLink size={13}/><span class="ctx-label">Reveal in Explorer</span>
      </button>
      <button type="button" class="ctx-item" data-tone="neutral" onclick={() => { copyPath(target.path); closeMenu(); }}>
        <Copy size={13}/><span class="ctx-label">Copy path</span><span class="ctx-kbd">Ctrl+C</span>
      </button>
      <button type="button" class="ctx-item" data-tone="neutral" onclick={() => { renameEntry(target); closeMenu(); }}>
        <Pencil size={13}/><span class="ctx-label">Rename…</span><span class="ctx-kbd">F2</span>
      </button>
    {/if}
    <div class="ctx-sep"></div>
    <button type="button" class="ctx-item" data-tone="danger" onclick={() => { deleteEntries(target); closeMenu(); }}>
      <Trash2 size={13}/><span class="ctx-label">Delete{multi ? ` (${selected.size})` : ""}</span><span class="ctx-kbd">Del</span>
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
  .row[data-dir="true"] {
    background: color-mix(in oklch, var(--accent) 4%, transparent);
  }
  .row[data-dir="true"] .row-label {
    font-weight: 600;
    color: var(--fg);
  }
  .row :global(.row-icon) { color: var(--fg-muted); flex-shrink: 0; }
  .row :global(.row-icon-dir) { color: var(--accent); opacity: 0.95; }
  .row:hover { background: var(--surface-hover); }
  .row[data-dir="true"]:hover {
    background: color-mix(in oklch, var(--accent) 11%, var(--surface-hover));
  }
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
  .row-name { display: inline-flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; }
  .row-label { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .up-name { color: var(--fg-muted); padding-left: 2px; letter-spacing: 0.04em; }
  .row-status { text-align: center; }
  .sym { font-size: 11px; }
  .sym.danger { color: var(--danger); }
  .row-size, .row-mtime { color: var(--fg-subtle); font-size: var(--fs-xs); white-space: nowrap; }

  .empty {
    padding: 28px 22px;
    display: flex; flex-direction: column; gap: 6px;
    color: var(--fg-muted); font-size: var(--fs-sm); text-align: center;
  }
  .empty-title { color: var(--fg); font-weight: 500; }
  .empty-hint { color: var(--fg-subtle); font-size: var(--fs-xs); }

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
    min-width: 224px; padding: 4px;
    transform-origin: top left;
  }
  .ctx-item {
    --ctx-tone: var(--fg-muted);
    position: relative;
    display: flex; align-items: center; gap: 10px;
    width: 100%; text-align: left;
    background: transparent; border: 0;
    color: var(--fg); font-size: var(--fs-sm);
    padding: 7px 10px; border-radius: var(--radius-xs);
    cursor: pointer;
    transition: background 100ms ease, color 100ms ease, box-shadow 100ms ease;
  }
  .ctx-item[data-tone="accent"] { --ctx-tone: var(--accent); }
  .ctx-item[data-tone="info"]   { --ctx-tone: var(--info); }
  .ctx-item[data-tone="warn"]   { --ctx-tone: var(--warn); }
  .ctx-item[data-tone="danger"] { --ctx-tone: var(--danger); }
  .ctx-item :global(svg) { color: var(--ctx-tone); opacity: 0.85; flex-shrink: 0; }
  .ctx-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ctx-kbd {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 10px;
    color: var(--fg-faint);
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 5px;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }
  .ctx-item:hover {
    background: color-mix(in oklch, var(--ctx-tone) 14%, var(--surface-hover));
    color: var(--ctx-tone);
    box-shadow: inset 2px 0 var(--ctx-tone);
  }
  .ctx-item:hover :global(svg) { opacity: 1; }
  .ctx-item:hover .ctx-kbd {
    color: var(--ctx-tone);
    border-color: color-mix(in oklch, var(--ctx-tone) 45%, var(--border));
    background: color-mix(in oklch, var(--ctx-tone) 8%, var(--bg-elev-1));
  }
  .ctx-sep {
    height: 1px;
    background: var(--border);
    margin: 4px 2px;
  }
</style>
