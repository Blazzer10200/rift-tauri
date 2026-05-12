<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, onMount } from "svelte";
  import { X, Plus } from "lucide-svelte";
  import { fly, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { connection } from "../../state/connection.svelte";
  import { browserTabs } from "../../state/browser-tabs.svelte";
  import LocalPane from "./LocalPane.svelte";
  import RemotePane from "./RemotePane.svelte";
  import FlashToast from "../FlashToast.svelte";
  import SyncModal from "../sync/SyncModal.svelte";

  type LocalEntry = {
    name: string; path: string; is_dir: boolean; size: number; mtime: number;
  };
  type RemoteEntry = {
    full_path: string; name: string; is_dir: boolean; size: number; last_modified: string;
  };

  let toast = $state<{ msg: string; kind: "ok" | "err" | "info" } | null>(null);

  const SPLIT_KEY = "rift.browser.splitFrac";
  const SPLIT_MIN = 0.15;
  const SPLIT_MAX = 0.85;
  let splitFrac = $state(0.5);
  let splitEl = $state<HTMLDivElement | undefined>();
  let dragging = $state(false);

  onMount(() => {
    const s = connection.selected;
    browserTabs.hydrate(s?.localRoot ?? "", s?.remoteRoot ?? "/");
    try {
      const saved = localStorage.getItem(SPLIT_KEY);
      if (saved) {
        const v = parseFloat(saved);
        if (!isNaN(v) && v >= SPLIT_MIN && v <= SPLIT_MAX) splitFrac = v;
      }
    } catch { /* localStorage unavailable */ }
  });

  function persistSplit() {
    try { localStorage.setItem(SPLIT_KEY, splitFrac.toFixed(4)); } catch { /* noop */ }
  }

  function onDividerPointerDown(e: PointerEvent) {
    e.preventDefault();
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function onDividerPointerMove(e: PointerEvent) {
    if (!dragging || !splitEl) return;
    const rect = splitEl.getBoundingClientRect();
    if (rect.width <= 0) return;
    const x = e.clientX - rect.left;
    let frac = x / rect.width;
    if (frac < SPLIT_MIN) frac = SPLIT_MIN;
    else if (frac > SPLIT_MAX) frac = SPLIT_MAX;
    splitFrac = frac;
  }
  function onDividerPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    try { (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId); } catch { /* noop */ }
    persistSplit();
  }
  function resetSplit() {
    splitFrac = 0.5;
    try { localStorage.removeItem(SPLIT_KEY); } catch { /* noop */ }
  }
  function onDividerKey(e: KeyboardEvent) {
    const step = e.shiftKey ? 0.05 : 0.02;
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      splitFrac = Math.max(SPLIT_MIN, splitFrac - step);
      persistSplit();
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      splitFrac = Math.min(SPLIT_MAX, splitFrac + step);
      persistSplit();
    } else if (e.key === "Home" || e.key === "Enter") {
      e.preventDefault();
      resetSplit();
    }
  }

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

  // Snap tab paths back to the profile root when the profile's root changes
  // (e.g. user moved their FiveM dir + edited Settings). Without this the
  // tab keeps its old navigation in localStorage and the pane errors out
  // with "not a directory" against a path that no longer exists.
  function normPath(p: string): string {
    return (p ?? "").replaceAll("\\", "/").toLowerCase().replace(/\/+$/, "");
  }
  $effect(() => {
    const s = connection.selected;
    if (!s) return;
    const lroot = normPath(s.localRoot);
    const rroot = normPath(s.remoteRoot);
    browserTabs.tabs.forEach((t, i) => {
      if (lroot && !normPath(t.localPath).startsWith(lroot)) {
        browserTabs.updateLocalPath(i, s.localRoot);
      }
      if (rroot && !normPath(t.remotePath).startsWith(rroot)) {
        browserTabs.updateRemotePath(i, s.remoteRoot);
      }
    });
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

  function tabDisplay(t: { name: string; localPath: string; remotePath: string }): string {
    const p = t.localPath || t.remotePath;
    if (p) {
      const b = basename(p);
      if (b) return b;
    }
    return t.name || "tab";
  }

  function newTab() {
    const s = connection.selected;
    if (!s) return;
    const t = browserTabs.active;
    const lroot = t?.localPath || s.localRoot;
    const rroot = t?.remotePath || s.remoteRoot;
    browserTabs.open(basename(lroot) || "tab", lroot, rroot);
  }

  function onTabKey(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey) return;
    const k = e.key.toLowerCase();
    if (k === "t") { e.preventDefault(); newTab(); return; }
    if (k === "w") {
      e.preventDefault();
      if (browserTabs.tabs.length > 1) browserTabs.close(browserTabs.activeIdx);
      return;
    }
    if (e.key === "Tab") {
      e.preventDefault();
      const n = browserTabs.tabs.length;
      if (n <= 1) return;
      const next = (browserTabs.activeIdx + 1) % n;
      browserTabs.setActive(next);
    }
  }

  $effect(() => {
    window.addEventListener("keydown", onTabKey);
    return () => window.removeEventListener("keydown", onTabKey);
  });

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

</script>

<div class="two-pane">
  <div class="tabstrip-wrap">
    <div class="tabstrip">
      {#each browserTabs.tabs as t, i (t.id)}
      <div
        class="tab"
        data-active={i === browserTabs.activeIdx}
        in:fly={{ x: -10, duration: 180, easing: quintOut }}
        out:scale={{ start: 0.85, duration: 140, easing: quintOut }}
      >
        <button
          type="button" class="tab-label"
          onclick={() => browserTabs.setActive(i)}
          title={t.localPath || t.remotePath}
        >
          {tabDisplay(t)}
        </button>
        {#if browserTabs.tabs.length > 1}
          <button
            type="button" class="tab-x"
            onclick={() => browserTabs.close(i)}
            aria-label="Close tab"
            title="Close tab (Ctrl+W)"
          ><X size={10}/></button>
        {/if}
      </div>
    {/each}
      <button
        type="button" class="tab-new"
        onclick={newTab}
        title="New tab (Ctrl+T)"
        aria-label="New tab"
      ><Plus size={14}/></button>
    </div>
  </div>

  {#if browserTabs.active}
    {@const t = browserTabs.active}
    {@const idx = browserTabs.activeIdx}
    <div
      class="split"
      bind:this={splitEl}
      data-dragging={dragging}
      style="grid-template-columns: {splitFrac}fr 6px {1 - splitFrac}fr;"
    >
      <LocalPane
        path={t.localPath}
        onPathChange={(p: string) => setLocalPath(idx, p)}
        onOpenInNewTab={(e: LocalEntry) => openLocalNewTab(e)}
        onDropPaths={(remotePaths: string[]) => downloadRemotesToLocal(remotePaths)}
        onDropPathsToFolder={(remotePaths, targetDir) => downloadRemotesToLocalDir(remotePaths, targetDir)}
        onUploadPaths={(localPaths) => uploadLocalsToRemote(localPaths)}
      />
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="divider"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize panes (drag, or use arrow keys; double-click or Home to reset)"
        aria-valuenow={Math.round(splitFrac * 100)}
        aria-valuemin={Math.round(SPLIT_MIN * 100)}
        aria-valuemax={Math.round(SPLIT_MAX * 100)}
        tabindex="0"
        title="Drag to resize · double-click to reset"
        onpointerdown={onDividerPointerDown}
        onpointermove={onDividerPointerMove}
        onpointerup={onDividerPointerUp}
        onpointercancel={onDividerPointerUp}
        ondblclick={resetSplit}
        onkeydown={onDividerKey}
      ><span class="divider-grip" aria-hidden="true"></span></div>
      <RemotePane
        serverKey={connection.selectedKey}
        path={t.remotePath}
        onPathChange={(p: string) => setRemotePath(idx, p)}
        onOpenInNewTab={(e: RemoteEntry) => openRemoteNewTab(e)}
        onDropPaths={(localPaths: string[]) => uploadLocalsToRemote(localPaths)}
        onDropPathsToFolder={(localPaths, targetDir) => uploadLocalsToRemoteDir(localPaths, targetDir)}
        onDownloadPaths={(remotePaths) => downloadRemotesToLocal(remotePaths)}
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
  .tabstrip-wrap {
    position: relative;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
    height: 38px;
    /* Mask fades both edges; transparent when tabs don't overflow. */
    mask-image: linear-gradient(
      to right,
      transparent 0,
      #000 20px,
      #000 calc(100% - 20px),
      transparent 100%
    );
  }
  .tabstrip {
    display: flex; align-items: flex-end; gap: 2px;
    padding: 6px 8px 0;
    height: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }
  .tabstrip::-webkit-scrollbar { display: none; }
  .tab { flex-shrink: 0; }
  .tab {
    display: flex; align-items: center;
    background: transparent;
    border: 1px solid transparent;
    border-bottom: 0;
    border-radius: 8px 8px 0 0;
    padding: 0 4px 0 0;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
    max-width: 240px;
    min-width: 120px;
    height: 32px;
    transition: background 100ms ease, color 100ms ease, border-color 100ms ease;
  }
  .tab:hover { background: var(--surface-hover); color: var(--fg); }
  .tab[data-active="true"] {
    background: var(--bg);
    color: var(--fg);
    border-color: var(--border);
    margin-bottom: -1px;
  }
  .tab-label {
    flex: 1;
    background: transparent; border: 0;
    color: inherit;
    padding: 0 12px;
    height: 100%;
    cursor: pointer;
    font: inherit;
    font-weight: 500;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    text-align: center;
    letter-spacing: 0.01em;
  }
  .tab[data-active="true"] .tab-label { font-weight: 600; }
  .tab-x {
    background: transparent; border: 0;
    color: var(--fg-muted); cursor: pointer;
    width: 18px; height: 18px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    flex-shrink: 0;
    transition: background 100ms ease, color 100ms ease;
  }
  .tab-x:hover { background: var(--danger); color: oklch(0.99 0 0); }
  .tab-new {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    width: 28px; height: 28px;
    margin: 0 0 0 4px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    flex-shrink: 0;
    transition: background 120ms ease, color 120ms ease, transform 120ms ease;
  }
  .tab-new:hover { background: var(--surface-hover); color: var(--fg); transform: scale(1.08); }
  .tab-new:active { transform: scale(0.94); }
  .tab-new:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }

  .split {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 6px 1fr;
    min-height: 0; min-width: 0;
  }
  .split[data-dragging="true"] {
    cursor: col-resize;
    user-select: none;
  }
  .split[data-dragging="true"] :global(*) {
    user-select: none !important;
    pointer-events: none;
  }
  .split[data-dragging="true"] > .divider {
    pointer-events: auto;
  }

  .divider {
    position: relative;
    background: var(--bg-elev-3);
    cursor: col-resize;
    display: flex; align-items: center; justify-content: center;
    align-self: stretch;
    height: 100%;
    transition: background 100ms ease;
    touch-action: none;
  }
  .divider::before {
    content: "";
    position: absolute;
    inset: 0 -3px;
  }
  .divider:hover,
  .split[data-dragging="true"] > .divider {
    background: var(--accent);
  }
  .divider:focus-visible {
    outline: none;
    background: var(--accent);
    box-shadow: 0 0 0 2px var(--ring);
  }
  .divider-grip {
    width: 2px; height: 28px;
    background: var(--fg-faint);
    border-radius: 1px;
    opacity: 0.6;
    transition: opacity 100ms ease, background 100ms ease;
  }
  .divider:hover .divider-grip,
  .split[data-dragging="true"] .divider-grip {
    opacity: 1;
    background: oklch(0.99 0 0);
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
