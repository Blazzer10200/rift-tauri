<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { connection } from "../../state/connection.svelte";

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

  let scanning = $state(false);
  let result = $state<ScanResult | null>(null);
  let error = $state<string | null>(null);
  let subpathsText = $state("");
  let selected = $state<Set<string>>(new Set());

  const STORAGE_KEY = "rift.drift.subpaths.v1";

  $effect(() => {
    const k = connection.selectedKey;
    if (!k) return;
    const raw = localStorage.getItem(STORAGE_KEY + "." + k);
    if (raw !== null) subpathsText = raw;
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
    try {
      result = await invoke<ScanResult>("scan_drift", {
        serverKey: s.key,
        folders,
      });
    } catch (e) {
      error = String(e);
      result = null;
    } finally {
      scanning = false;
    }
  }

  const localOnly = $derived(
    result?.entries.filter((e) => e.bucket === "ToPush" && !e.remote_exists) ?? [],
  );
  const remoteOnly = $derived(
    result?.entries.filter((e) => e.bucket === "ToPull" && !e.local_exists) ?? [],
  );
  const modified = $derived(
    result?.entries.filter(
      (e) => e.bucket === "ToPush" || e.bucket === "ToPull" || e.bucket === "Conflict",
    ).filter((e) => e.local_exists && e.remote_exists) ?? [],
  );

  function toggle(rel: string) {
    const next = new Set(selected);
    if (next.has(rel)) next.delete(rel); else next.add(rel);
    selected = next;
  }
  function selectBucket(rows: DriftEntry[]) {
    const next = new Set(selected);
    for (const r of rows) next.add(r.rel_path);
    selected = next;
  }
  function clearSelection() { selected = new Set(); }

  async function applySelected() {
    if (!result) return;
    const picks = result.entries.filter((e) => selected.has(e.rel_path));
    if (picks.length === 0) return;
    // Translate drift entries → enqueue_for_flush_batch (push) or download_paths (pull).
    const s = connection.selected;
    if (!s) return;
    const toPush = picks.filter((p) => p.bucket === "ToPush" && p.local_exists).map((p) => p.local_path);
    const toPull = picks.filter((p) => p.bucket === "ToPull" && p.remote_exists)
      .map((p) => [p.remote_path, p.local_path] as [string, string]);
    let pushed = 0, pulled = 0, errs: string[] = [];
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
  }

  let toast = $state<string | null>(null);
  function flash(msg: string) {
    toast = msg;
    setTimeout(() => { toast = null; }, 4500);
  }

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
</script>

<section class="drift">
  <div class="toolbar">
    <textarea
      class="subpaths"
      placeholder="Resource subpaths, one per line (e.g. [qbx]/qbx_core)"
      bind:value={subpathsText}
      rows="3"
    ></textarea>
    <div class="actions">
      <button
        type="button"
        class="primary"
        onclick={scan}
        disabled={scanning || !connection.selected}
      >
        {scanning ? "Scanning…" : "Scan drift"}
      </button>
      <button type="button" onclick={clearSelection} disabled={selected.size === 0}>
        Clear ({selected.size})
      </button>
      <button
        type="button"
        class="apply"
        onclick={applySelected}
        disabled={selected.size === 0}
      >
        Apply selected
      </button>
    </div>
  </div>

  {#if error}
    <div class="err">{error}</div>
  {/if}

  {#if result}
    {#if result.remote_folders_missing.length > 0}
      <div class="warn">
        Missing remote folders: {result.remote_folders_missing.join(", ")}
      </div>
    {/if}
    {#if result.last_batch_listing_error}
      <div class="warn">Listing error: {result.last_batch_listing_error}</div>
    {/if}

    <div class="buckets">
      <div class="bucket b-push">
        <header>
          <h3>↑ Local Only ({localOnly.length})</h3>
          <button type="button" onclick={() => selectBucket(localOnly)} disabled={localOnly.length === 0}>Select all</button>
        </header>
        {#if localOnly.length === 0}
          <p class="empty">No local-only files.</p>
        {:else}
          <ul>
            {#each localOnly as e (e.rel_path)}
              <li>
                <label>
                  <input type="checkbox" checked={selected.has(e.rel_path)} onchange={() => toggle(e.rel_path)} />
                  <span class="path">{e.rel_path}</span>
                  <span class="meta">{fmtSize(e.local_size)} · {fmtMtime(e.local_mtime)}</span>
                </label>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="bucket b-pull">
        <header>
          <h3>↓ Remote Only ({remoteOnly.length})</h3>
          <button type="button" onclick={() => selectBucket(remoteOnly)} disabled={remoteOnly.length === 0}>Select all</button>
        </header>
        {#if remoteOnly.length === 0}
          <p class="empty">No remote-only files.</p>
        {:else}
          <ul>
            {#each remoteOnly as e (e.rel_path)}
              <li>
                <label>
                  <input type="checkbox" checked={selected.has(e.rel_path)} onchange={() => toggle(e.rel_path)} />
                  <span class="path">{e.rel_path}</span>
                  <span class="meta">{fmtSize(e.remote_size)} · {fmtMtime(e.remote_mtime)}</span>
                </label>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="bucket b-mod">
        <header>
          <h3>⚡ Modified ({modified.length})</h3>
          <button type="button" onclick={() => selectBucket(modified)} disabled={modified.length === 0}>Select all</button>
        </header>
        {#if modified.length === 0}
          <p class="empty">No modifications.</p>
        {:else}
          <ul>
            {#each modified as e (e.rel_path)}
              <li class:conflict={e.bucket === "Conflict"}>
                <label>
                  <input type="checkbox" checked={selected.has(e.rel_path)} onchange={() => toggle(e.rel_path)} />
                  <span class="path">{e.rel_path}</span>
                  <span class="bucket-tag {e.bucket.toLowerCase()}">{e.bucket}</span>
                  <span class="meta">L {fmtSize(e.local_size)} · R {fmtSize(e.remote_size)} · {e.reason}</span>
                </label>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  {:else if !scanning}
    <div class="hint">
      Enter resource subpaths above and click <strong>Scan drift</strong>. Subpaths are relative to the server's <code>remoteRoot</code>.
    </div>
  {/if}

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}
</section>

<style>
  .drift {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0; overflow: auto;
    background: #0F0F12; color: #E8E8EE;
    padding: 12px 14px;
    position: relative;
  }
  .toolbar {
    display: flex; gap: 12px; align-items: flex-start;
    margin-bottom: 8px;
  }
  .subpaths {
    flex: 1;
    background: #17171C; color: #E8E8EE;
    border: 1px solid #26262E; border-radius: 4px;
    padding: 6px 8px;
    font-family: Consolas, monospace; font-size: 12px;
    resize: vertical;
    min-height: 56px;
  }
  .subpaths:focus { outline: 0; border-color: #8B6BE6; }
  .actions { display: flex; flex-direction: column; gap: 6px; }
  .actions button {
    background: #17171C; color: #E8E8EE;
    border: 1px solid #26262E; border-radius: 3px;
    padding: 6px 12px; font-size: 12px; cursor: pointer;
    min-width: 130px;
  }
  .actions button:hover:not(:disabled) { border-color: #8B6BE6; }
  .actions button:disabled { opacity: 0.4; cursor: not-allowed; }
  .actions .primary { background: #15101E; border-color: #8B6BE6; color: #8B6BE6; font-weight: 600; }
  .actions .apply { background: #15101E; border-color: #4ADE80; color: #4ADE80; font-weight: 600; }
  .err {
    color: #FF5C6B; padding: 8px 12px;
    background: #15101E; font-size: 12px; font-family: Consolas, monospace;
    border-radius: 3px; margin: 4px 0;
  }
  .warn {
    color: #F0B95C; padding: 8px 12px;
    background: #15101E; font-size: 12px;
    border-radius: 3px; margin: 4px 0;
  }
  .buckets {
    display: grid; grid-template-columns: 1fr 1fr 1fr;
    gap: 10px; margin-top: 8px;
  }
  .bucket {
    background: #17171C; border: 1px solid #26262E;
    border-radius: 4px; padding: 8px; min-height: 200px;
    display: flex; flex-direction: column;
  }
  .b-push { border-color: #8B6BE6; }
  .b-pull { border-color: #F0B95C; }
  .b-mod  { border-color: #FF5C6B; }
  .bucket header {
    display: flex; justify-content: space-between; align-items: center;
    padding-bottom: 6px; margin-bottom: 6px;
    border-bottom: 1px solid #26262E;
  }
  .bucket h3 { margin: 0; font-size: 13px; color: #E8E8EE; }
  .bucket header button {
    background: transparent; border: 1px solid #26262E;
    color: #7A7A85; font-size: 11px; padding: 2px 8px; border-radius: 3px;
    cursor: pointer;
  }
  .bucket header button:hover:not(:disabled) { color: #8B6BE6; border-color: #8B6BE6; }
  .bucket ul { list-style: none; margin: 0; padding: 0; overflow: auto; flex: 1; }
  .bucket li { padding: 3px 0; border-bottom: 1px solid #15101E; font-size: 12px; }
  .bucket li.conflict .bucket-tag { color: #FF5C6B; }
  .bucket label {
    display: flex; gap: 8px; align-items: center;
    cursor: pointer;
  }
  .path { font-family: Consolas, monospace; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { color: #7A7A85; font-size: 11px; font-family: Consolas, monospace; }
  .bucket-tag { font-size: 10px; padding: 1px 5px; border-radius: 999px; background: #15101E; color: #F0B95C; }
  .bucket-tag.topush { color: #8B6BE6; }
  .bucket-tag.topull { color: #F0B95C; }
  .bucket-tag.conflict { color: #FF5C6B; }
  .empty { color: #7A7A85; font-size: 12px; margin: 8px 0; }
  .hint { color: #7A7A85; font-size: 12px; padding: 16px 4px; }
  .hint code { background: #17171C; padding: 1px 5px; border-radius: 3px; }
  .toast {
    position: absolute; bottom: 16px; left: 50%; transform: translateX(-50%);
    background: #17171C; border: 1px solid #4ADE80;
    border-radius: 4px; padding: 8px 14px;
    color: #E8E8EE; font-size: 12px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  }
</style>
