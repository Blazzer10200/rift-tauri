<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    X, Download, Upload, AlertTriangle, Check, Folder, RefreshCw,
  } from "lucide-svelte";

  type BootstrapState =
    | "synced" | "missingLocalRoot" | "empty"
    | "uninitialized" | "partial" | "badRemoteRoot";

  type BootstrapDetection = {
    state: BootstrapState;
    remoteResourceCount: number;
    localPresentCount: number;
    missingCount: number;
    remoteTopLevelDirs: string[];
  };

  type Props = {
    open: boolean;
    serverKey: string;
    initialLocalRoot: string;
    detection: BootstrapDetection | null;
    onClose: (result: { downloaded: number; failed: number; localRoot: string; cancelled: boolean } | null) => void;
  };

  let { open, serverKey, initialLocalRoot, detection, onClose }: Props = $props();

  let localRoot = $state("");
  let phase = $state<"idle" | "listing" | "downloading" | "done" | "error">("idle");
  let total = $state(0);
  let done = $state(0);
  let failed = $state(0);
  let progressText = $state("");
  let cancelled = $state(false);

  $effect(() => {
    if (open) {
      localRoot = initialLocalRoot;
      phase = "idle";
      total = 0; done = 0; failed = 0; cancelled = false;
      progressText = "";
    }
  });

  type Variant = "ok" | "warn" | "danger" | "info";
  type Cfg = { variant: Variant; title: string; sub: string; primary: string };

  const cfg = $derived<Cfg>(((): Cfg => {
    const d = detection;
    if (!d) return { variant: "info", title: "Bootstrap workspace", sub: "Detecting…", primary: "Continue" };
    switch (d.state) {
      case "synced":
        return { variant: "ok", title: "Already in sync", sub: "Local and remote roots match. Nothing to bootstrap.", primary: "Close" };
      case "missingLocalRoot":
        return { variant: "warn", title: "Local root is missing", sub: `The local folder doesn't exist yet. Pick a destination — Rift will pull ${d.remoteResourceCount} resource folders.`, primary: "Download all" };
      case "empty":
        return { variant: "info", title: "Local is empty", sub: `Server has ${d.remoteResourceCount} resource folders. Download them now to start working, or skip if you'll populate this folder another way.`, primary: "Download all" };
      case "uninitialized":
        return { variant: "info", title: "Fresh install", sub: `${d.localPresentCount} of ${d.remoteResourceCount} resource folders present locally — looks like a fresh install. Pull the missing ${d.missingCount}?`, primary: "Download missing" };
      case "partial":
        return { variant: "warn", title: "Partial sync", sub: `Missing ${d.missingCount} of ${d.remoteResourceCount} resource folders locally. Pull the rest to align with the server.`, primary: "Download missing" };
      case "badRemoteRoot":
        return { variant: "danger", title: "Remote path looks wrong", sub: `The remote path doesn't look like a FiveM resources directory (no [bracketed]/ subfolders). Most likely your Remote Root points at a parent — update it to end in /resources.`, primary: "Close" };
      default:
        return { variant: "info", title: "Bootstrap workspace", sub: "", primary: "Continue" };
    }
  })());

  const isBadRoot = $derived(detection?.state === "badRemoteRoot");
  const isSynced = $derived(detection?.state === "synced");
  const showPathInput = $derived(!isBadRoot && !isSynced && phase === "idle");
  const showProgress = $derived(phase === "listing" || phase === "downloading" || phase === "done" || phase === "error");
  const pct = $derived(total > 0 ? Math.round((done / total) * 100) : 0);

  function iconForVariant(v: Variant) {
    switch (v) {
      case "ok": return Check;
      case "warn": return AlertTriangle;
      case "danger": return AlertTriangle;
      case "info": return Download;
    }
  }

  async function download() {
    if (!localRoot.trim()) {
      progressText = "Pick a local folder first.";
      return;
    }
    phase = "listing";
    progressText = "Listing remote tree…";
    cancelled = false;
    try {
      const jobs = await invoke<[string, string][]>("bootstrap_list_files", {
        serverKey,
        localRoot,
      });
      if (jobs.length === 0) {
        total = 0; done = 0; failed = 0;
        phase = "done";
        progressText = "Nothing to download.";
        return;
      }
      total = jobs.length;
      done = 0; failed = 0;
      phase = "downloading";

      const chunkSize = 50;
      for (let i = 0; i < jobs.length && !cancelled; i += chunkSize) {
        const slice = jobs.slice(i, i + chunkSize);
        const result = await invoke<boolean[]>("download_paths", {
          serverKey,
          jobs: slice,
        });
        const sliceDone = result.filter(Boolean).length;
        const sliceFailed = result.length - sliceDone;
        done += sliceDone;
        failed += sliceFailed;
        progressText = `Downloading ${done} / ${total}${failed > 0 ? ` · ${failed} failed` : ""}`;
      }
      phase = "done";
      progressText = cancelled
        ? `Cancelled — ${done} of ${total} files downloaded.`
        : `Done — ${done} of ${total} files downloaded${failed > 0 ? ` (${failed} failed)` : ""}.`;
    } catch (e) {
      phase = "error";
      progressText = "Error: " + String(e);
    }
  }

  function cancel() { cancelled = true; }

  function close(skip: boolean) {
    if (skip || phase === "idle") {
      onClose(null);
    } else {
      onClose({ downloaded: done, failed, localRoot, cancelled });
    }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget && phase !== "downloading" && phase !== "listing") {
      close(true);
    }
  }
</script>

{#if open}
  {@const Ico = iconForVariant(cfg.variant)}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation">
    <div class="dialog-shell" style="width: 540px;" role="dialog" aria-modal="true" aria-label={cfg.title}>
      <div class="dialog-head">
        <div class="dialog-icon {cfg.variant}"><Ico size={14}/></div>
        <div>
          <div class="dialog-title">Bootstrap workspace</div>
          <div class="dialog-sub">{cfg.title}</div>
        </div>
        <button class="dialog-close" type="button" onclick={() => close(true)} aria-label="Close">
          <X size={14}/>
        </button>
      </div>

      <div class="dialog-body">
        <p class="lead">{cfg.sub}</p>

        {#if detection && detection.remoteTopLevelDirs.length > 0 && !isBadRoot}
          <div class="folders-preview">
            <div class="folders-head">Remote top-level</div>
            <div class="folders-list mono">
              {#each detection.remoteTopLevelDirs.slice(0, 6) as d}
                <span class="folder-chip"><Folder size={10}/> {d}</span>
              {/each}
              {#if detection.remoteTopLevelDirs.length > 6}
                <span class="folder-chip more">+{detection.remoteTopLevelDirs.length - 6} more</span>
              {/if}
            </div>
          </div>
        {/if}

        {#if showPathInput}
          <div class="field">
            <label class="field-label" for="bs-path">Local destination</label>
            <input
              id="bs-path"
              class="input mono"
              type="text"
              bind:value={localRoot}
              placeholder="C:\path\to\local\resources"
              disabled={phase === "downloading" || phase === "listing"}
            />
          </div>
        {/if}

        {#if showProgress}
          <div class="progress">
            <div class="progress-text mono">
              {#if phase === "downloading"}<RefreshCw size={11} class="spin"/>{/if}
              {progressText}
            </div>
            <div class="bar">
              <div class="fill" style:width="{pct}%"></div>
            </div>
            <div class="progress-stats mono dim">
              {#if total > 0}
                {done} / {total} files{failed > 0 ? ` · ${failed} failed` : ""} · {pct}%
              {/if}
            </div>
          </div>
        {/if}

        <p class="hint">Skips <span class="mono">[disabled]/</span> folders. Downloads run in parallel — click Cancel to stop after the current chunk.</p>
      </div>

      <div class="dialog-foot">
        {#if isBadRoot || isSynced}
          <div class="dialog-foot-spacer"></div>
          <button class="btn primary" type="button" onclick={() => close(true)}>Close</button>
        {:else if phase === "idle"}
          <button class="btn ghost" type="button" onclick={() => close(true)}>Skip for now</button>
          <div class="dialog-foot-spacer"></div>
          <button
            class="btn primary"
            type="button"
            onclick={download}
            disabled={!localRoot.trim()}
          >
            <Download size={11}/> {cfg.primary}
          </button>
        {:else if phase === "listing" || phase === "downloading"}
          <div class="dialog-foot-spacer"></div>
          <button class="btn danger" type="button" onclick={cancel}>Cancel</button>
        {:else}
          <div class="dialog-foot-spacer"></div>
          <button class="btn primary" type="button" onclick={() => close(false)}>Close</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .lead { color: var(--fg-2); font-size: var(--fs-sm); line-height: 1.5; margin: 0 0 12px; }
  .folders-preview {
    margin-bottom: 14px;
    padding: 8px 10px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .folders-head { font-size: var(--fs-xs); color: var(--fg-faint); margin-bottom: 6px; }
  .folders-list { display: flex; flex-wrap: wrap; gap: 4px; }
  .folder-chip {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 6px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-xs);
    font-size: var(--fs-xs);
    color: var(--fg-2);
  }
  .folder-chip.more { color: var(--fg-faint); }

  .progress { display: flex; flex-direction: column; gap: 6px; margin: 12px 0; }
  .progress-text {
    display: inline-flex; align-items: center; gap: 6px;
    font-size: var(--fs-xs);
    color: var(--fg-2);
  }
  .bar {
    height: 6px;
    background: var(--bg-elev-2);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease;
  }
  .progress-stats { font-size: var(--fs-xs); }

  .hint { font-size: var(--fs-xs); color: var(--fg-faint); margin: 8px 0 0; }
</style>
