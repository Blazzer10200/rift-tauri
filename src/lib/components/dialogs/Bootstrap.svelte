<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

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
    onClose: (result: { downloaded: number; failed: number; localRoot: string } | null) => void;
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

  const titleFor = (s: BootstrapState | undefined): string => {
    switch (s) {
      case "badRemoteRoot": return "Remote path doesn't look right";
      default: return "Local folder needs the server files";
    }
  };

  const bodyFor = (d: BootstrapDetection | null): string => {
    if (!d) return "Detecting…";
    switch (d.state) {
      case "badRemoteRoot":
        return `The remote path you configured (${d.remoteResourceCount} folders found) doesn't look like a FiveM resources directory — FiveM wraps resource categories in brackets (e.g. [endure]/, [ox]/, [cfx-default]/). Most likely your Remote Root points at a parent folder. Update it to end in /resources.`;
      case "missingLocalRoot":
        return `The local folder for this server doesn't exist on your machine. Pick a destination below — Rift will download ${d.remoteResourceCount} resource folders from the server.`;
      case "empty":
        return `The local folder is empty. The server has ${d.remoteResourceCount} resource folders. Download them now to start working, or skip if you'll populate this folder another way.`;
      case "uninitialized":
        return `Only ${d.localPresentCount} of ${d.remoteResourceCount} resource folders are present locally — this looks like a fresh install. Download the missing ${d.missingCount} now?`;
      case "partial":
        return `You're missing ${d.missingCount} of ${d.remoteResourceCount} resource folders locally. Download them to bring your copy in line with the server.`;
      default:
        return "Local and remote look in sync — bootstrap not needed.";
    }
  };

  const isBadRoot = $derived(detection?.state === "badRemoteRoot");
  const showDownload = $derived(!isBadRoot && phase === "idle");
  const showProgress = $derived(phase === "listing" || phase === "downloading" || phase === "done" || phase === "error");

  // No native folder picker (plugin-dialog not wired in this scaffold). User
  // types the path; we validate on submit by attempting the bootstrap_list_files
  // call which will fail loudly if the local root can't be resolved.

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
      onClose({ downloaded: done, failed, localRoot });
    }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget && phase !== "downloading") close(true);
  }
</script>

{#if open}
  <div class="backdrop" onclick={onBackdrop} role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" aria-label={titleFor(detection?.state)}>
      <header><h2>{titleFor(detection?.state)}</h2></header>

      <p class="body">{bodyFor(detection)}</p>

      {#if !isBadRoot}
        <div class="path-row">
          <span class="label">Download to:</span>
          <input class="path" type="text" bind:value={localRoot} placeholder="C:\path\to\local\resources" disabled={phase === "downloading"} />
        </div>
      {/if}

      {#if showProgress}
        <div class="progress">
          <p class="progress-text">{progressText}</p>
          <div class="bar">
            <div
              class="fill"
              style:width={total > 0 ? `${Math.round((done / total) * 100)}%` : "0%"}
            ></div>
          </div>
        </div>
      {/if}

      <p class="footer-hint">Skips [disabled]/ folders. Downloads run in parallel; you can close Rift to cancel.</p>

      <footer>
        {#if isBadRoot}
          <button class="primary" onclick={() => close(true)} type="button">Close</button>
        {:else if phase === "idle"}
          <button class="cancel" onclick={() => close(true)} type="button">Skip for now</button>
          <button class="primary" onclick={download} type="button" disabled={!localRoot.trim()}>Download all</button>
        {:else if phase === "downloading" || phase === "listing"}
          <button class="cancel" onclick={cancel} type="button">Cancel download</button>
        {:else}
          <button class="primary" onclick={() => close(false)} type="button">Close</button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 200;
  }
  .dialog {
    background: #17171C;
    border: 1px solid #3A2A66;
    border-radius: 6px;
    width: 520px; max-width: 92vw;
    box-shadow: 0 18px 50px rgba(0,0,0,0.6);
    padding: 18px 20px;
    color: #E8E8EE;
    display: flex; flex-direction: column; gap: 14px;
  }
  h2 { margin: 0; font-size: 14px; font-weight: 600; }
  .body { font-size: 13px; line-height: 1.5; margin: 0; }
  .path-row { display: flex; align-items: center; gap: 8px; }
  .label { font-size: 11px; color: #7A7A85; }
  input.path {
    flex: 1;
    background: #0F0F12;
    border: 1px solid #26262E;
    border-radius: 4px;
    color: #E8E8EE;
    padding: 6px 8px;
    font-size: 12px;
    font-family: Consolas, monospace;
  }
  .progress { display: flex; flex-direction: column; gap: 8px; }
  .progress-text { font-size: 12px; color: #7A7A85; margin: 0; }
  .bar { height: 6px; background: #0F0F12; border-radius: 3px; overflow: hidden; }
  .fill { height: 100%; background: #8B6BE6; transition: width 0.2s; }
  .footer-hint { font-size: 11px; color: #7A7A85; margin: 0; }
  footer { display: flex; justify-content: flex-end; gap: 8px; }
  button {
    background: #1F1F26;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  button:hover:not(:disabled) { background: #26262E; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
  button.primary { background: #3A2A66; border-color: #3A2A66; color: white; }
  button.primary:hover:not(:disabled) { background: #4A3678; }
</style>
