<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { connection } from "../../state/connection.svelte";
  import { browserTabs, type BrowserTab } from "../../state/browser-tabs.svelte";
  import LocalPane from "./LocalPane.svelte";
  import RemotePane from "./RemotePane.svelte";

  type LocalEntry = {
    name: string; path: string; is_dir: boolean; size: number; mtime: number;
  };
  type RemoteEntry = {
    full_path: string; name: string; is_dir: boolean; size: number; last_modified: string;
  };

  let leftPct = $state(50);
  let dragging = $state(false);
  let containerEl: HTMLDivElement | undefined = $state();
  let toast = $state<{ msg: string; kind: "ok" | "err" | "info" } | null>(null);

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

  function startDrag(e: MouseEvent) {
    e.preventDefault();
    dragging = true;
  }
  function onMove(e: MouseEvent) {
    if (!dragging || !containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    const pct = ((e.clientX - rect.left) / rect.width) * 100;
    leftPct = Math.max(15, Math.min(85, pct));
  }
  function endDrag() { dragging = false; }

  function flash(msg: string, kind: "ok" | "err" | "info" = "info") {
    toast = { msg, kind };
    setTimeout(() => { toast = null; }, 4500);
  }

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

  async function uploadLocalsToRemote(localPaths: string[]) {
    const s = connection.selected;
    const t = browserTabs.active;
    if (!s || !t) return;
    const jobs: [string, string][] = localPaths
      .filter((lp) => !!basename(lp))
      .map((lp) => [lp, joinRemote(t.remotePath, basename(lp))]);
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

  async function downloadRemotesToLocal(remotePaths: string[]) {
    const s = connection.selected;
    const t = browserTabs.active;
    if (!s || !t) return;
    const jobs: [string, string][] = remotePaths
      .filter((rp) => !!basename(rp))
      .map((rp) => [rp, joinLocal(t.localPath, basename(rp))]);
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
</script>

<svelte:window onmousemove={onMove} onmouseup={endDrag} />

<div class="two-pane">
  <div class="tabstrip">
    {#each browserTabs.tabs as t, i (t.id)}
      <div
        class="tab"
        class:active={i === browserTabs.activeIdx}
      >
        <button type="button" class="tab-label" onclick={() => browserTabs.setActive(i)}>
          {t.name}
        </button>
        {#if browserTabs.tabs.length > 1}
          <button
            type="button" class="tab-x"
            onclick={() => browserTabs.close(i)}
            aria-label="Close tab"
          >×</button>
        {/if}
      </div>
    {/each}
  </div>

  {#if browserTabs.active}
    {@const t = browserTabs.active}
    {@const idx = browserTabs.activeIdx}
    <div class="split" bind:this={containerEl}>
      <div class="side" style="width: {leftPct}%">
        <div class="side-label">Local</div>
        <LocalPane
          path={t.localPath}
          onPathChange={(p: string) => setLocalPath(idx, p)}
          onOpenInNewTab={(e: LocalEntry) => openLocalNewTab(e)}
          onDropPaths={(remotePaths: string[]) => downloadRemotesToLocal(remotePaths)}
        />
      </div>
      <button
        type="button"
        class="resizer"
        class:active={dragging}
        aria-label="Resize panes"
        onmousedown={startDrag}
      ></button>
      <div class="side" style="width: {100 - leftPct}%">
        <div class="side-label">Remote</div>
        <RemotePane
          serverKey={connection.selectedKey}
          path={t.remotePath}
          onPathChange={(p: string) => setRemotePath(idx, p)}
          onOpenInNewTab={(e: RemoteEntry) => openRemoteNewTab(e)}
          onDropPaths={(localPaths: string[]) => uploadLocalsToRemote(localPaths)}
        />
      </div>
    </div>
  {:else}
    <div class="placeholder">Pick a server to begin browsing.</div>
  {/if}

  {#if toast}
    <div class="toast" class:ok={toast.kind === "ok"} class:err={toast.kind === "err"}>
      {toast.msg}
    </div>
  {/if}
</div>

<style>
  .two-pane {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0; min-width: 0;
    background: #0F0F12;
    position: relative;
  }
  .tabstrip {
    display: flex; gap: 2px;
    padding: 4px 8px 0;
    background: #0F0F12;
    border-bottom: 1px solid #26262E;
    overflow-x: auto;
  }
  .tab {
    display: flex; align-items: center;
    background: #17171C;
    border: 1px solid #26262E; border-bottom: 0;
    border-radius: 4px 4px 0 0;
    padding: 0 4px 0 0;
    color: #7A7A85;
    font-size: 12px;
    max-width: 220px;
  }
  .tab.active {
    background: #15101E; color: #E8E8EE;
    border-color: #8B6BE6;
    border-bottom-color: #15101E;
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
    color: #7A7A85; cursor: pointer;
    width: 18px; height: 18px;
    border-radius: 3px;
    font-size: 14px; line-height: 1;
  }
  .tab-x:hover { background: #FF5C6B; color: #E8E8EE; }
  .split { flex: 1; display: flex; min-height: 0; min-width: 0; }
  .side {
    display: flex; flex-direction: column;
    min-width: 0; min-height: 0;
  }
  .side-label {
    padding: 4px 12px;
    background: #17171C;
    color: #7A7A85;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid #26262E;
  }
  .resizer {
    width: 4px; flex-shrink: 0;
    background: #26262E;
    cursor: col-resize;
    border: 0; padding: 0;
  }
  .resizer:hover, .resizer.active { background: #8B6BE6; }
  .resizer:focus-visible { outline: 0; background: #8B6BE6; }
  .placeholder {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: #7A7A85; font-size: 13px;
  }
  .toast {
    position: absolute;
    bottom: 16px; left: 50%; transform: translateX(-50%);
    background: #17171C;
    border: 1px solid #26262E;
    border-radius: 4px;
    padding: 8px 14px;
    color: #E8E8EE;
    font-size: 12px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    z-index: 50;
  }
  .toast.ok { border-color: #4ADE80; }
  .toast.err { border-color: #FF5C6B; color: #FF5C6B; }
</style>
