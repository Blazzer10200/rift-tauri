<script lang="ts">
  import { onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    ChevronRight, Folder, FileCode, Upload, Download, X, Check,
    AppWindow, Server, RefreshCw, AlertTriangle, List, FolderTree,
  } from "lucide-svelte";
  import { connection } from "../../state/connection.svelte";
  import FlashToast from "../FlashToast.svelte";

  type DriftBucket = "Synced" | "ToPush" | "ToPull" | "Conflict";
  type DriftEntry = {
    resource_name: string;
    rel_path: string;
    local_path: string;
    remote_path: string;
    bucket: DriftBucket;
    local_exists: boolean;
    remote_exists: boolean;
    local_size: number;
    remote_size: number;
    local_mtime: string | null;
    remote_mtime: string | null;
    has_snapshot: boolean;
    reason: string;
  };
  type ScanResult = {
    entries: DriftEntry[];
    last_batch_listing_error: string | null;
    remote_folders_missing: string[];
  };

  type SideFilter = "all" | "local" | "remote";
  type Grouping = "dir" | "flat";

  let scanning = $state(false);
  let result = $state<ScanResult | null>(null);
  let error = $state<string | null>(null);
  let subpathsText = $state("");
  let selected = $state<Set<string>>(new Set());
  let expanded = $state<Set<string>>(new Set());
  let sideFilter = $state<SideFilter>("all");
  let grouping = $state<Grouping>("dir");
  let toolbarOpen = $state(true);

  const STORAGE_KEY = "rift.drift.subpaths.v1";

  $effect(() => {
    const k = connection.selectedKey;
    if (!k) return;
    const raw = localStorage.getItem(STORAGE_KEY + "." + k);
    subpathsText = raw ?? "";
  });

  function persist() {
    const k = connection.selectedKey;
    if (!k) return;
    localStorage.setItem(STORAGE_KEY + "." + k, subpathsText);
  }

  function parseSpecs() {
    return subpathsText
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean)
      .map((sub) => {
        const trimmed = sub.replace(/\/+$/, "");
        const idx = trimmed.lastIndexOf("/");
        const name = idx === -1 ? trimmed : trimmed.slice(idx + 1);
        return { resource_name: name || trimmed, remote_subpath: sub };
      });
  }

  async function scan() {
    const s = connection.selected;
    if (!s) { error = "Pick a server first."; return; }
    const folders = parseSpecs();
    if (folders.length === 0) {
      error = "Enter at least one resource subpath (one per line, e.g. [qbx]/qbx_core).";
      return;
    }
    persist();
    scanning = true;
    error = null;
    selected = new Set();
    expanded = new Set();
    try {
      result = await invoke<ScanResult>("scan_drift", {
        serverKey: s.key,
        folders,
      });
      toolbarOpen = false;
    } catch (e) {
      error = String(e);
      result = null;
    } finally {
      scanning = false;
    }
  }

  // Drift entries that need action (exclude Synced)
  const actionable = $derived<DriftEntry[]>(
    (result?.entries ?? []).filter((e) => e.bucket !== "Synced"),
  );

  function sideOf(e: DriftEntry): "local" | "remote" | "conflict" {
    if (e.bucket === "Conflict") return "conflict";
    if (e.bucket === "ToPush") return "local";
    return "remote";
  }

  const filtered = $derived<DriftEntry[]>(
    actionable.filter((e) => {
      if (sideFilter === "all") return true;
      const s = sideOf(e);
      if (sideFilter === "local") return s === "local";
      if (sideFilter === "remote") return s === "remote";
      return true;
    }),
  );

  type Group = { key: string; label: string | null; items: DriftEntry[] };

  const groups = $derived.by<Group[]>(() => {
    if (grouping === "flat") {
      return [{ key: "all", label: null, items: filtered }];
    }
    const map = new Map<string, DriftEntry[]>();
    for (const e of filtered) {
      const dir = e.rel_path.split("/").slice(0, -1).join("/") || "/";
      const arr = map.get(dir);
      if (arr) arr.push(e);
      else map.set(dir, [e]);
    }
    return [...map.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([dir, items]) => ({ key: dir, label: dir, items }));
  });

  const selectableIds = $derived(
    filtered.filter((e) => e.bucket !== "Conflict").map((e) => e.rel_path),
  );

  // Audit M21: prune `selected` to entries that are still selectable when
  // sideFilter / grouping / filtered list changes — otherwise applyPushPull
  // operates on entries the user can no longer see.
  //
  // INVARIANT: this effect writes `selected` based on `selectableIds`
  // ($derived). `selectableIds` MUST NOT depend (transitively) on `selected` —
  // any future dep closure between them creates an infinite reactive loop.
  // If you extend `selectableIds`, audit its chain.
  $effect(() => {
    const valid = new Set(selectableIds);
    let changed = false;
    const next = new Set<string>();
    for (const id of selected) {
      if (valid.has(id)) next.add(id);
      else changed = true;
    }
    if (changed) selected = next;
  });
  const allSelected = $derived(
    selectableIds.length > 0 && selectableIds.every((id) => selected.has(id)),
  );

  function toggleSel(rel: string) {
    const next = new Set(selected);
    if (next.has(rel)) next.delete(rel);
    else next.add(rel);
    selected = next;
  }
  function toggleAll() {
    if (allSelected) selected = new Set();
    else selected = new Set(selectableIds);
  }
  function clearSel() { selected = new Set(); }

  function toggleExpand(rel: string) {
    const next = new Set(expanded);
    if (next.has(rel)) next.delete(rel);
    else next.add(rel);
    expanded = next;
  }

  async function applyPushPull() {
    if (!result) return;
    const picks = result.entries.filter((e) => selected.has(e.rel_path));
    if (picks.length === 0) return;
    const s = connection.selected;
    if (!s) return;

    const toPush = picks
      .filter((p) => p.bucket === "ToPush" && p.local_exists)
      .map((p) => p.local_path);
    const toPull = picks
      .filter((p) => p.bucket === "ToPull" && p.remote_exists)
      .map((p) => [p.remote_path, p.local_path] as [string, string]);

    let pushed = 0, pulled = 0;
    const errs: string[] = [];

    if (toPush.length > 0) {
      try {
        pushed = await invoke<number>("enqueue_for_flush_batch", {
          paths: toPush, deleted: false, bypassPreflight: false,
        });
      } catch (e) { errs.push(`push: ${e}`); }
    }
    if (toPull.length > 0) {
      try {
        const res = await invoke<boolean[]>("download_paths", {
          serverKey: s.key, jobs: toPull,
        });
        pulled = res.filter(Boolean).length;
      } catch (e) { errs.push(`pull: ${e}`); }
    }
    error = errs.length > 0 ? errs.join("; ") : null;
    flash(`Pushed ${pushed} · Pulled ${pulled}${errs.length ? " · errors" : ""}`);
    selected = new Set();
    void scan();
  }

  async function autoResolveSafe() {
    if (!result) return;
    // safe = local-only (push) and remote-only (pull) — no real conflicts
    const safeIds = actionable
      .filter((e) =>
        (e.bucket === "ToPush" && !e.remote_exists) ||
        (e.bucket === "ToPull" && !e.local_exists),
      )
      .map((e) => e.rel_path);
    if (safeIds.length === 0) {
      flash("Nothing safe to auto-resolve.");
      return;
    }
    selected = new Set(safeIds);
    await applyPushPull();
  }

  let toast = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  function flash(msg: string) {
    toast = msg;
    if (toastTimer !== null) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { toast = null; toastTimer = null; }, 4500);
  }
  onDestroy(() => {
    if (toastTimer !== null) { clearTimeout(toastTimer); toastTimer = null; }
  });

  function fmtSize(n: number): string {
    if (n < 0) return "—";
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }
  function fmtMtime(s: string | null): string {
    if (!s) return "—";
    try { return new Date(s).toLocaleString(); } catch { return s; }
  }

  function basename(p: string): string {
    const i = p.lastIndexOf("/");
    return i === -1 ? p : p.slice(i + 1);
  }
</script>

<section class="drift">
  <header class="hd">
    <div class="hd-l">
      <h3>Drift review</h3>
      <p class="help">
        {#if result}
          {filtered.length} {filtered.length === 1 ? "file" : "files"} · pick a side or resolve case-by-case
        {:else}
          Configure resources, then scan to compare local vs. remote
        {/if}
      </p>
    </div>
    <div class="hd-r">
      {#if result}
        <button
          class="btn ghost sm"
          type="button"
          onclick={() => (toolbarOpen = !toolbarOpen)}
          title="Edit subpaths"
        >
          <FolderTree size={11}/> Subpaths
        </button>
      {/if}
      <button
        class="btn primary sm"
        type="button"
        onclick={scan}
        disabled={scanning || !connection.selected}
      >
        <RefreshCw size={11}/>
        {scanning ? "Scanning…" : (result ? "Re-scan" : "Scan drift")}
      </button>
    </div>
  </header>

  {#if toolbarOpen}
    <div class="subpaths-row">
      <textarea
        class="subpaths"
        placeholder="Resource subpaths, one per line (e.g. [qbx]/qbx_core)"
        bind:value={subpathsText}
        rows="3"
      ></textarea>
    </div>
  {/if}

  {#if error}
    <div class="banner err">
      <AlertTriangle size={12}/> {error}
    </div>
  {/if}

  {#if result?.remote_folders_missing.length}
    <div class="banner warn">
      Missing remote folders: <span class="mono">{result.remote_folders_missing.join(", ")}</span>
    </div>
  {/if}
  {#if result?.last_batch_listing_error}
    <div class="banner warn mono">{result.last_batch_listing_error}</div>
  {/if}

  {#if result}
    <div class="subbar">
      <div class="segctl">
        <button data-active={sideFilter === "all"} type="button" onclick={() => (sideFilter = "all")}>
          All <span class="pip">{actionable.length}</span>
        </button>
        <button data-active={sideFilter === "local"} type="button" onclick={() => (sideFilter = "local")}>
          <AppWindow size={10}/> Local wins
          <span class="pip">{actionable.filter((e) => sideOf(e) === "local").length}</span>
        </button>
        <button data-active={sideFilter === "remote"} type="button" onclick={() => (sideFilter = "remote")}>
          <Server size={10}/> Remote wins
          <span class="pip">{actionable.filter((e) => sideOf(e) === "remote").length}</span>
        </button>
      </div>

      <div class="segctl sm">
        <button data-active={grouping === "dir"} type="button" onclick={() => (grouping = "dir")} title="Group by directory">
          <Folder size={11}/>
        </button>
        <button data-active={grouping === "flat"} type="button" onclick={() => (grouping = "flat")} title="Flat list">
          <List size={11}/>
        </button>
      </div>

      <button class="btn primary sm auto" type="button" onclick={autoResolveSafe} disabled={scanning}>
        <Check size={11}/> Auto-resolve safe
      </button>
    </div>

    {#if selected.size > 0}
      <div class="bulk">
        <div class="bulk-l">
          <button
            type="button"
            class="cb"
            data-on={allSelected}
            onclick={toggleAll}
            aria-label="Toggle all"
          >
            {#if allSelected}<Check size={10}/>{/if}
          </button>
          <span><strong>{selected.size}</strong> selected</span>
          <button class="btn ghost xs" type="button" onclick={clearSel}>Clear</button>
        </div>
        <div class="bulk-r">
          <button class="btn sm" type="button" onclick={applyPushPull}>
            <Upload size={11}/><Download size={11}/> Apply ({selected.size})
          </button>
        </div>
      </div>
    {/if}

    <div class="scroll">
      {#if filtered.length === 0}
        <div class="empty">
          {actionable.length === 0
            ? "Everything in sync — no drift detected."
            : "No matches in this filter."}
        </div>
      {:else}
        {#each groups as g (g.key)}
          <div class="group">
            {#if g.label !== null}
              <div class="group-hd">
                <Folder size={11}/>
                <span class="mono">{g.label}</span>
                <span class="dim">· {g.items.length}</span>
              </div>
            {/if}
            <div class="rows">
              {#each g.items as e (e.rel_path)}
                {@const side = sideOf(e)}
                {@const isOpen = expanded.has(e.rel_path)}
                {@const isSel = selected.has(e.rel_path)}
                {@const isConflict = e.bucket === "Conflict"}
                <div class="row" data-side={side} data-selected={isSel} data-open={isOpen}>
                  <div
                    class="row-main"
                    role="button"
                    tabindex="0"
                    onclick={() => toggleExpand(e.rel_path)}
                    onkeydown={(ev) => { if (ev.key === "Enter") toggleExpand(e.rel_path); }}
                  >
                    <button
                      type="button"
                      class="cb"
                      data-on={isSel}
                      disabled={isConflict}
                      onclick={(ev) => { ev.stopPropagation(); if (!isConflict) toggleSel(e.rel_path); }}
                      aria-label={isSel ? "Deselect" : "Select"}
                    >
                      {#if isSel}<Check size={10}/>{/if}
                    </button>
                    <span class="chev" data-open={isOpen}><ChevronRight size={11}/></span>
                    <FileCode size={12}/>
                    <div class="path-block">
                      <span class="path mono">{basename(e.rel_path)}</span>
                      <div class="sub">
                        <span class="pill" data-side={side}>
                          {#if side === "local"}<AppWindow size={9}/> local wins{:else if side === "remote"}<Server size={9}/> remote wins{:else}<AlertTriangle size={9}/> conflict{/if}
                        </span>
                        <span class="mono dim">{e.reason}</span>
                      </div>
                    </div>
                    <div class="stat mono dim">
                      {#if e.local_exists && e.remote_exists}
                        L {fmtSize(e.local_size)} · R {fmtSize(e.remote_size)}
                      {:else if e.local_exists}
                        L {fmtSize(e.local_size)}
                      {:else}
                        R {fmtSize(e.remote_size)}
                      {/if}
                    </div>
                  </div>
                  {#if isOpen}
                    <div class="peek">
                      <div class="peek-grid">
                        <span class="peek-k">Path</span>
                        <span class="peek-v mono">{e.rel_path}</span>

                        <span class="peek-k">Local</span>
                        <span class="peek-v mono">
                          {e.local_exists ? `${fmtSize(e.local_size)} · ${fmtMtime(e.local_mtime)}` : "—"}
                        </span>

                        <span class="peek-k">Remote</span>
                        <span class="peek-v mono">
                          {e.remote_exists ? `${fmtSize(e.remote_size)} · ${fmtMtime(e.remote_mtime)}` : "—"}
                        </span>

                        <span class="peek-k">Snapshot</span>
                        <span class="peek-v mono">{e.has_snapshot ? "yes" : "no"}</span>

                        <span class="peek-k">Reason</span>
                        <span class="peek-v">{e.reason}</span>
                      </div>
                      {#if isConflict}
                        <p class="conflict-note">
                          Both sides modified — resolve in the <strong>Conflicts</strong> tab.
                        </p>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {:else if !scanning}
    <div class="hint">
      Enter resource subpaths above and hit <strong>Scan drift</strong>. Subpaths are relative to the server's <span class="mono">remoteRoot</span>.
    </div>
  {/if}

  {#if toast}
    <div class="toast-anchor">
      <FlashToast message={toast} kind="ok" onDismiss={() => (toast = null)} />
    </div>
  {/if}
</section>

<style>
  .drift {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    padding: 10px 14px 14px;
    background: var(--bg);
    color: var(--fg);
    position: relative;
  }
  .hd {
    display: flex; justify-content: space-between; align-items: end;
    gap: 12px; margin-bottom: 8px;
  }
  .hd h3 { margin: 0; font-size: var(--fs-lg); font-weight: 600; }
  .hd .help { color: var(--fg-muted); font-size: var(--fs-xs); margin: 2px 0 0; }
  .hd-r { display: inline-flex; gap: 6px; align-items: center; }

  .subpaths-row { margin-bottom: 8px; }
  .subpaths {
    width: 100%;
    background: var(--bg-elev-1); color: var(--fg);
    border: 1px solid var(--border); border-radius: var(--radius-sm);
    padding: 6px 8px;
    font-family: var(--font-mono); font-size: var(--fs-xs);
    resize: vertical; min-height: 56px;
  }
  .subpaths:focus { outline: 0; border-color: var(--accent); background: var(--bg-elev-2); }

  .banner {
    padding: 6px 10px; margin-bottom: 6px;
    border-radius: var(--radius-sm);
    font-size: var(--fs-xs);
    display: inline-flex; align-items: center; gap: 6px;
  }
  .banner.err { background: var(--danger-soft); color: var(--danger); border: 1px solid color-mix(in oklch, var(--danger) 30%, transparent); }
  .banner.warn { background: var(--warn-soft); color: var(--warn); border: 1px solid color-mix(in oklch, var(--warn) 30%, transparent); }

  .subbar {
    display: flex; align-items: center; gap: 8px;
    margin-bottom: 8px; flex-wrap: wrap;
  }
  .segctl {
    display: inline-flex; padding: 2px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    gap: 1px;
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
  .segctl.sm button { padding: 3px 8px; height: 22px; }
  .segctl button:hover { color: var(--fg); }
  .segctl button[data-active="true"] {
    background: var(--surface); color: var(--fg);
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
  .auto { margin-left: auto; }

  .bulk {
    display: flex; align-items: center; justify-content: space-between;
    padding: 6px 10px; margin-bottom: 6px;
    background: var(--bg-elev-2);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 18%, transparent);
  }
  .bulk-l, .bulk-r { display: inline-flex; align-items: center; gap: 8px; font-size: var(--fs-sm); }

  .cb {
    width: 16px; height: 16px;
    border-radius: var(--radius-xs);
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    color: var(--accent-fg);
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .cb[data-on="true"] { background: var(--accent); border-color: var(--accent); }
  .cb:disabled { opacity: 0.35; cursor: not-allowed; }

  .scroll { flex: 1; min-height: 0; overflow: auto; }

  .group { margin-bottom: 10px; }
  .group-hd {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 6px 4px; color: var(--fg-muted);
    font-size: var(--fs-xs);
  }
  .group-hd .dim { color: var(--fg-faint); }
  .rows {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    overflow: hidden;
  }
  .row { border-bottom: 1px solid var(--border); }
  .row:last-child { border-bottom: 0; }
  .row[data-selected="true"] { background: var(--accent-soft); }
  .row[data-side="conflict"] { background: color-mix(in oklch, var(--danger-soft) 60%, transparent); }
  .row-main {
    display: grid;
    grid-template-columns: 16px 12px 14px 1fr auto;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    cursor: pointer;
    text-align: left;
  }
  .row-main:hover { background: var(--surface-hover); }
  .row-main:focus-visible { outline: 1px solid var(--accent); outline-offset: -1px; }
  .chev { color: var(--fg-faint); transition: transform 80ms; display: inline-flex; }
  .chev[data-open="true"] { transform: rotate(90deg); }

  .path-block { min-width: 0; }
  .path { color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; display: block; }
  .sub { display: flex; gap: 8px; align-items: center; margin-top: 1px; }
  .sub .dim { color: var(--fg-faint); font-size: var(--fs-xs); }

  .pill {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 10px;
    background: var(--bg-elev-2);
    color: var(--fg-muted);
  }
  .pill[data-side="local"] { background: var(--info-soft); color: var(--info); }
  .pill[data-side="remote"] { background: var(--warn-soft); color: var(--warn); }
  .pill[data-side="conflict"] { background: var(--danger-soft); color: var(--danger); }

  .stat { font-size: var(--fs-xs); white-space: nowrap; }

  .peek {
    padding: 8px 14px 10px 50px;
    background: var(--bg-elev-1);
    border-top: 1px dashed var(--border);
    font-size: var(--fs-xs);
  }
  .peek-grid {
    display: grid;
    grid-template-columns: 84px 1fr;
    gap: 4px 12px;
  }
  .peek-k { color: var(--fg-faint); }
  .peek-v { color: var(--fg-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .conflict-note { margin: 8px 0 0; color: var(--danger); font-size: var(--fs-xs); }

  .empty, .hint {
    padding: 24px; color: var(--fg-muted);
    font-size: var(--fs-sm); text-align: center;
  }

  .toast-anchor {
    position: absolute;
    bottom: 16px; left: 50%; transform: translateX(-50%);
    z-index: 50;
  }
</style>
