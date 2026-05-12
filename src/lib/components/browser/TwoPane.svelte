<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, onMount } from "svelte";
  import { X } from "lucide-svelte";
  import { connection } from "../../state/connection.svelte";
  import { browserTabs } from "../../state/browser-tabs.svelte";
  import { syncModal } from "../../state/sync-modal.svelte";
  import LocalPane from "./LocalPane.svelte";
  import RemotePane from "./RemotePane.svelte";
  import OpRail from "./OpRail.svelte";
  import FlashToast from "../FlashToast.svelte";
  import SyncModal from "../sync/SyncModal.svelte";

  type LocalEntry = {
    name: string; path: string; is_dir: boolean; size: number; mtime: number;
  };
  type RemoteEntry = {
    full_path: string; name: string; is_dir: boolean; size: number; last_modified: string;
  };

  let toast = $state<{ msg: string; kind: "ok" | "err" | "info" } | null>(null);
  let localSel = $state<string[]>([]);
  let remoteSel = $state<string[]>([]);

  onMount(() => {
    const s = connection.selected;
    browserTabs.hydrate(s?.localRoot ?? "", s?.remoteRoot ?? "/");
  });

  $effect(() => {
    const s = connection.selected;
    if (!s) return;
    if (browserTabs.tabs.length === 0) {
      browserTabs.hydrate(s.localRoot, s.remoteRoot);
    } else if (browserTabs.tabs.length === 1) {
      const t = browserTabs.tabs[0];
      if (t.name === "default" && (!t.localPath || !t.remotePath)) {
        browserTabs.updateLocalPath(0, s.localRoot);
        browserTabs.updateRemotePath(0, s.remoteRoot);
      }
    }
  });

  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  function flash(msg: string, kind: "ok" | "err" | "info" = "info") {
    toast = { msg, kind };
    if (toastTimer !== null) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { toast = null; toastTimer = null; }, 4500);
  }
  onDestroy(() => {
    if (toastTimer !== null) { clearTimeout(toastTimer); toastTimer = null; }
  });

  function setLocalPath(idx: number, p: string) { browserTabs.updateLocalPath(idx, p); }
  function setRemotePath(idx: number, p: string) { browserTabs.updateRemotePath(idx, p); }

  function openLocalNewTab(entry: LocalEntry) {
    const t = browserTabs.active;
    if (!t) return;
    browserTabs.open(entry.name, entry.path, t.remotePath);
  }

  function openRemoteNewTab(entry: RemoteEntry) {
    const t = browserTabs.active;
    if (!t) return;
    browserTabs.open(entry.name, t.localPath, entry.full_path);
  }

  function joinRemote(dir: string, name: string): string {
    const trimmed = dir.replace(/\/+$/, "");
    return trimmed === "" ? "/" + name : trimmed + "/" + name;
  }
  function joinLocal(dir: string, name: string): string {
    const sep = dir.includes("\\") || /^[A-Za-z]:/.test(dir) ? "\\" : "/";
    const trimmed = dir.replace(/[\\/]+$/, "");
    return trimmed + sep + name;
  }
  function basename(p: string): string {
    const norm = p.replaceAll("\\", "/").replace(/\/+$/, "");
    const idx = norm.lastIndexOf("/");
    return idx === -1 ? norm : norm.slice(idx + 1);
  }

  async function uploadLocalsToRemoteDir(localPaths: string[], targetRemoteDir: string) {
    const s = connection.selected;
    if (!s || localPaths.length === 0) return;
    const jobs: [string, string][] = localPaths
      .filter((lp) => !!basename(lp))
      .map((lp) => [lp, joinRemote(targetRemoteDir, basename(lp))]);
    if (jobs.length === 0) return;
    flash(`Uploading ${jobs.length}…`, "info");
    try {
      const res = await invoke<boolean[]>("upload_paths", { serverKey: s.key, jobs });
      const ok = res.filter(Boolean).length;
      flash(`Uploaded ${ok}/${jobs.length}`, ok === jobs.length ? "ok" : "err");
    } catch (e) {
      flash(`Upload failed: ${e}`, "err");
    }
  }

  async function uploadLocalsToRemote(localPaths: string[]) {
    const t = browserTabs.active;
    if (!t) return;
    await uploadLocalsToRemoteDir(localPaths, t.remotePath);
  }

  async function downloadRemotesToLocalDir(remotePaths: string[], targetLocalDir: string) {
    const s = connection.selected;
    if (!s || remotePaths.length === 0) return;
    const jobs: [string, string][] = remotePaths
      .filter((rp) => !!basename(rp))
      .map((rp) => [rp, joinLocal(targetLocalDir, basename(rp))]);
    if (jobs.length === 0) return;
    flash(`Downloading ${jobs.length}…`, "info");
    try {
      const res = await invoke<boolean[]>("download_paths", { serverKey: s.key, jobs });
      const ok = res.filter(Boolean).length;
      flash(`Downloaded ${ok}/${jobs.length}`, ok === jobs.length ? "ok" : "err");
    } catch (e) {
      flash(`Download failed: ${e}`, "err");
    }
  }

  async function downloadRemotesToLocal(remotePaths: string[]) {
    const t = browserTabs.active;
    if (!t) return;
    await downloadRemotesToLocalDir(remotePaths, t.localPath);
  }

  // Op rail actions
  function onUpload() { uploadLocalsToRemote(localSel); }
  function onDownload() { downloadRemotesToLocal(remoteSel); }

  // Sync button: open the modal, fire backend scan. SyncModal handles diag
  // events + activity feed + watchdog itself — no hardcoded timeout here.
  // v0.2.19's 30s frontend timeout false-alarmed on >30s scans (894-entry
  // reconcile took ~45s). Modal stays patient as long as events keep arriving.
  const syncing = $derived(syncModal.open && syncModal.phase === "scanning");
  async function onSync() {
    if (syncing) return;
    syncModal.start();
    try {
      const fired = await invoke<boolean>("diag_force_drift_scan");
      if (!fired) {
        syncModal.fail("Not connected — start auto-sync first.");
      }
    } catch (e) {
      syncModal.fail(String(e));
    }
  }
</script>

<div class="two-pane">
  <div class="tabstrip">
    {#each browserTabs.tabs as t, i (t.id)}
      <div class="tab" data-active={i === browserTabs.activeIdx}>
        <button type="button" class="tab-label mono" onclick={() => browserTabs.setActive(i)}>
          {t.name}
        </button>
        {#if browserTabs.tabs.length > 1}
          <button
            type="button" class="tab-x"
            onclick={() => browserTabs.close(i)}
            aria-label="Close tab"
          ><X size={10}/></button>
        {/if}
      </div>
    {/each}
  </div>

  {#if browserTabs.active}
    {@const t = browserTabs.active}
    {@const idx = browserTabs.activeIdx}
    <div class="split">
      <LocalPane
        path={t.localPath}
        onPathChange={(p: string) => setLocalPath(idx, p)}
        onOpenInNewTab={(e: LocalEntry) => openLocalNewTab(e)}
        onDropPaths={(remotePaths: string[]) => downloadRemotesToLocal(remotePaths)}
        onDropPathsToFolder={(remotePaths, targetDir) => downloadRemotesToLocalDir(remotePaths, targetDir)}
        onUploadPaths={(localPaths) => uploadLocalsToRemote(localPaths)}
        onSelectionChange={(paths) => (localSel = paths)}
      />
      <OpRail
        canUpload={localSel.length > 0}
        canDownload={remoteSel.length > 0}
        {syncing}
        {onUpload}
        {onDownload}
        {onSync}
      />
      <RemotePane
        serverKey={connection.selectedKey}
        path={t.remotePath}
        onPathChange={(p: string) => setRemotePath(idx, p)}
        onOpenInNewTab={(e: RemoteEntry) => openRemoteNewTab(e)}
        onDropPaths={(localPaths: string[]) => uploadLocalsToRemote(localPaths)}
        onDropPathsToFolder={(localPaths, targetDir) => uploadLocalsToRemoteDir(localPaths, targetDir)}
        onDownloadPaths={(remotePaths) => downloadRemotesToLocal(remotePaths)}
        onSelectionChange={(paths) => (remoteSel = paths)}
      />
    </div>
  {:else}
    <div class="placeholder">Pick a server to begin browsing.</div>
  {/if}

  {#if toast}
    <div class="toast-anchor">
      <FlashToast message={toast.msg} kind={toast.kind} onDismiss={() => (toast = null)} />
    </div>
  {/if}

  <SyncModal />
</div>

<style>
  .two-pane {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0; min-width: 0;
    background: var(--bg);
    position: relative;
  }
  .tabstrip {
    display: flex; gap: 2px;
    padding: 4px 8px 0;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .tab {
    display: flex; align-items: center;
    background: var(--bg-elev-1);
    border: 1px solid var(--border); border-bottom: 0;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    padding: 0 4px 0 0;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    max-width: 220px;
  }
  .tab[data-active="true"] {
    background: var(--bg);
    color: var(--fg);
    border-color: var(--border-strong);
    border-bottom-color: var(--bg);
    margin-bottom: -1px;
  }
  .tab-label {
    background: transparent; border: 0;
    color: inherit;
    padding: 5px 10px;
    cursor: pointer;
    font: inherit;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .tab-x {
    background: transparent; border: 0;
    color: var(--fg-muted); cursor: pointer;
    width: 18px; height: 18px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
  }
  .tab-x:hover { background: var(--danger); color: oklch(0.99 0 0); }

  .split {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 36px 1fr;
    min-height: 0; min-width: 0;
  }

  .placeholder {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: var(--fg-muted); font-size: var(--fs-sm);
  }

  .toast-anchor {
    position: absolute;
    bottom: 16px; left: 50%; transform: translateX(-50%);
    z-index: 50;
  }
</style>
