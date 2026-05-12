<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { X, Loader2, AlertTriangle, CheckCircle2, Ban, Download } from "lucide-svelte";
  import { syncModal, type SyncActivityKind } from "../../state/sync-modal.svelte";
  import { connection } from "../../state/connection.svelte";

  type DiagEvent = { stage: string; fields: unknown; at?: string; resource?: string | null };
  type DriftProgressFields = { current?: number; total?: number; resource?: string };
  type DriftResultFields = {
    entries?: number;
    to_push?: number;
    to_pull?: number;
    conflicts?: number;
    listing_error?: string | null;
    cancelled?: boolean;
    pull_dispatched?: number;
  };
  type ActivityRow = {
    at: string;
    resource: string;
    file: string;
    action: string;
    kind: string;
  };

  let unlistenDiag: UnlistenFn | null = null;
  let unlistenActivity: UnlistenFn | null = null;
  let lastEventAt = 0;
  let watchdogTimer: ReturnType<typeof setInterval> | null = null;
  let cancelling = $state(false);
  let pulling = $state(false);

  function basename(p: string): string {
    const norm = p.replaceAll("\\", "/").replace(/\/+$/, "");
    const idx = norm.lastIndexOf("/");
    return idx === -1 ? norm : norm.slice(idx + 1);
  }

  function activityKindFrom(action: string, kind: string): SyncActivityKind {
    const k = kind.toLowerCase();
    if (k.includes("error") || k.includes("block")) return "error";
    if (k.includes("pull")) return "pull";
    if (k.includes("delete")) return "delete";
    if (k.includes("sync") || action.toLowerCase().includes("upload")) return "push";
    return "system";
  }

  function attachListeners() {
    listen<DiagEvent>("diag://event", (e) => {
      lastEventAt = Date.now();
      const stage = e.payload.stage;
      const fields = e.payload.fields;
      if (stage === "drift_scan_start") {
        syncModal.pushActivity({
          at: new Date().toISOString(),
          kind: "drift",
          text: "reconcile started",
        });
        // First-stage listing is one big SFTP batch call that can take 20-40s
        // on a deep tree before any per-folder progress event fires. Without
        // this hint the modal looks frozen for the duration.
        syncModal.pushActivity({
          at: new Date().toISOString(),
          kind: "drift",
          text: "listing remote files (this can take a moment)…",
        });
      } else if (stage === "drift_scan_progress") {
        const f = (fields as DriftProgressFields) ?? {};
        syncModal.progress(f.current ?? 0, f.total ?? 0, f.resource ?? "");
        if (f.resource) {
          syncModal.pushActivity({
            at: new Date().toISOString(),
            kind: "drift",
            text: `scanning ${f.resource} (${f.current}/${f.total})`,
          });
        }
      } else if (stage === "drift_scan_result") {
        const f = (fields as DriftResultFields) ?? {};
        const result = {
          entries: f.entries ?? 0,
          push: f.to_push ?? 0,
          pull: f.to_pull ?? 0,
          conflicts: f.conflicts ?? 0,
          listing_error: f.listing_error ?? null,
        };
        if (f.cancelled) {
          syncModal.cancelled(result);
        } else if (f.listing_error) {
          syncModal.fail(f.listing_error);
        } else {
          syncModal.complete(result);
        }
        cancelling = false;
      }
    }).then((u) => (unlistenDiag = u));

    listen<ActivityRow>("autosync://activity", (e) => {
      lastEventAt = Date.now();
      const r = e.payload;
      syncModal.pushActivity({
        at: r.at,
        kind: activityKindFrom(r.action, r.kind),
        text: `${r.action} — ${r.file}`,
      });
    }).then((u) => (unlistenActivity = u));
  }

  function detachListeners() {
    if (unlistenDiag) { unlistenDiag(); unlistenDiag = null; }
    if (unlistenActivity) { unlistenActivity(); unlistenActivity = null; }
    if (watchdogTimer !== null) { clearInterval(watchdogTimer); watchdogTimer = null; }
  }

  $effect(() => {
    if (syncModal.open) {
      lastEventAt = Date.now();
      attachListeners();
      watchdogTimer = setInterval(() => {
        if (syncModal.phase !== "scanning") return;
        const silent = Date.now() - lastEventAt;
        if (silent > 30000) syncModal.setStalled();
      }, 5000);
    } else {
      detachListeners();
    }
  });

  onDestroy(detachListeners);

  async function onCancel() {
    if (cancelling) return;
    cancelling = true;
    try {
      await invoke<boolean>("diag_cancel_drift_scan");
    } catch (err) {
      console.warn("cancel scan failed", err);
      cancelling = false;
    }
  }

  async function onPullNow() {
    if (pulling) return;
    pulling = true;
    syncModal.start();
    try {
      const fired = await invoke<boolean>("diag_force_pull_now");
      if (!fired) syncModal.fail("Not connected — start auto-sync first.");
    } catch (err) {
      syncModal.fail(String(err));
    } finally {
      pulling = false;
    }
  }

  function onDismiss() {
    syncModal.dismiss();
  }

  function onKeydown(e: KeyboardEvent) {
    if (!syncModal.open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      if (syncModal.phase === "scanning") {
        // Esc during scan = cancel intent
        onCancel();
      } else {
        onDismiss();
      }
    }
  }

  const profileName = $derived(connection.selected?.name ?? "");
  const pct = $derived(
    syncModal.totalFolders > 0
      ? Math.min(100, Math.round((syncModal.currentFolder / syncModal.totalFolders) * 100))
      : 0,
  );
</script>

<svelte:window onkeydown={onKeydown} />

{#if syncModal.open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="overlay" role="dialog" aria-modal="true" aria-labelledby="sync-modal-title">
    <div
      class="backdrop"
      role="presentation"
      onclick={syncModal.phase === "scanning" ? undefined : onDismiss}
    ></div>
    <div class="card">
      <header class="card-head">
        <div class="title-row">
          {#if syncModal.phase === "scanning"}
            <Loader2 size={16} class="spin"/>
            <h2 id="sync-modal-title">Syncing {profileName}</h2>
          {:else if syncModal.phase === "complete"}
            <CheckCircle2 size={16}/>
            <h2 id="sync-modal-title">Sync complete</h2>
          {:else if syncModal.phase === "cancelled"}
            <Ban size={16}/>
            <h2 id="sync-modal-title">Sync cancelled</h2>
          {:else}
            <AlertTriangle size={16}/>
            <h2 id="sync-modal-title">Sync error</h2>
          {/if}
        </div>
        <button
          type="button"
          class="head-x"
          onclick={onDismiss}
          disabled={syncModal.phase === "scanning"}
          aria-label="Close"
          title={syncModal.phase === "scanning" ? "Cancel first" : "Close"}
        ><X size={14}/></button>
      </header>

      <section class="progress-section">
        <div class="bar-track">
          <div class="bar-fill" style="width: {pct}%"></div>
        </div>
        <div class="status-line mono">
          {#if syncModal.phase === "scanning"}
            {#if syncModal.totalFolders > 0}
              Scanning {syncModal.resource || "…"} — {syncModal.currentFolder} / {syncModal.totalFolders} folders
            {:else}
              Listing remote files… (this may take a moment on the first scan)
            {/if}
          {:else if syncModal.phase === "cancelled"}
            Cancelled after {syncModal.currentFolder} of {syncModal.totalFolders} folders
          {:else if syncModal.phase === "error"}
            {syncModal.errorMsg ?? "Unknown error"}
          {:else}
            Scanned {syncModal.result?.entries ?? 0} entries across {syncModal.totalFolders} folders
          {/if}
        </div>
        {#if syncModal.stalled && syncModal.phase === "scanning"}
          <div class="stalled-banner">
            No progress in 30s. Server may be slow — scan still running. Cancel if needed.
          </div>
        {/if}
      </section>

      <section class="counts">
        <div class="count-cell" data-tone="push">
          <div class="cell-num mono">{syncModal.result?.push ?? 0}</div>
          <div class="cell-label">To Push</div>
        </div>
        <div class="count-cell" data-tone="pull">
          <div class="cell-num mono">{syncModal.result?.pull ?? 0}</div>
          <div class="cell-label">To Pull</div>
        </div>
        <div class="count-cell" data-tone={(syncModal.result?.conflicts ?? 0) > 0 ? "danger" : "ok"}>
          <div class="cell-num mono">{syncModal.result?.conflicts ?? 0}</div>
          <div class="cell-label">Conflicts</div>
        </div>
      </section>

      <section class="activity">
        <div class="activity-head mono">Live activity</div>
        <div class="activity-feed">
          {#if syncModal.activity.length === 0}
            <div class="activity-empty">Waiting for events…</div>
          {:else}
            {#each syncModal.activity.slice().reverse() as row, i (i + row.at + row.text)}
              <div class="activity-row" data-kind={row.kind}>
                <span class="activity-dot"></span>
                <span class="activity-text mono">{row.text}</span>
              </div>
            {/each}
          {/if}
        </div>
      </section>

      <footer class="card-foot">
        {#if syncModal.phase === "scanning"}
          <button type="button" class="btn btn-danger" onclick={onCancel} disabled={cancelling}>
            {cancelling ? "Cancelling…" : "Cancel scan"}
          </button>
        {:else}
          {#if syncModal.phase === "complete" && (syncModal.result?.pull ?? 0) > 0}
            <button type="button" class="btn btn-accent" onclick={onPullNow} disabled={pulling}>
              <Download size={13}/>
              {pulling ? "Pulling…" : `Pull Now (${syncModal.result?.pull ?? 0})`}
            </button>
          {/if}
          <button type="button" class="btn btn-primary" onclick={onDismiss}>Dismiss</button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0; z-index: 200;
    display: flex; align-items: center; justify-content: center;
  }
  .backdrop {
    position: absolute; inset: 0;
    background: oklch(0% 0 0 / 0.55);
    backdrop-filter: blur(4px);
  }
  .card {
    position: relative;
    width: min(560px, 92vw);
    max-height: 86vh;
    display: flex; flex-direction: column;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg, 8px);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .card-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev-1);
  }
  .title-row { display: flex; align-items: center; gap: 8px; color: var(--fg); }
  .title-row h2 { margin: 0; font-size: var(--fs-md); font-weight: 600; }
  .head-x {
    background: transparent; border: 0; color: var(--fg-muted);
    width: 24px; height: 24px; cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-xs);
  }
  .head-x:hover:not(:disabled) { background: var(--danger); color: oklch(0.99 0 0); }
  .head-x:disabled { opacity: 0.35; cursor: not-allowed; }
  :global(.spin) { animation: spin 1.2s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .progress-section { padding: 14px 16px 10px; }
  .bar-track {
    height: 6px;
    background: var(--bg-elev-1);
    border-radius: 3px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 220ms ease-out;
  }
  .status-line {
    margin-top: 8px;
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
  }
  .stalled-banner {
    margin-top: 8px;
    padding: 6px 10px;
    background: color-mix(in oklch, var(--warning, orange) 18%, transparent);
    border: 1px solid color-mix(in oklch, var(--warning, orange) 40%, transparent);
    border-radius: var(--radius-xs);
    color: var(--fg);
    font-size: var(--fs-xs);
  }

  .counts {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    padding: 6px 16px 14px;
  }
  .count-cell {
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    text-align: center;
  }
  .count-cell[data-tone="danger"] {
    border-color: var(--danger);
    background: color-mix(in oklch, var(--danger) 12%, transparent);
  }
  .cell-num { font-size: var(--fs-lg); font-weight: 600; color: var(--fg); }
  .cell-label {
    margin-top: 2px;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .activity {
    flex: 1;
    min-height: 120px;
    display: flex; flex-direction: column;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }
  .activity-head {
    padding: 6px 16px;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
  }
  .activity-feed {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
    max-height: 220px;
  }
  .activity-empty {
    padding: 16px;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    text-align: center;
  }
  .activity-row {
    display: flex; align-items: center; gap: 8px;
    padding: 3px 16px;
    font-size: var(--fs-xs);
  }
  .activity-dot {
    flex-shrink: 0;
    width: 6px; height: 6px;
    border-radius: 50%;
    background: var(--fg-faint);
  }
  .activity-row[data-kind="push"] .activity-dot,
  .activity-row[data-kind="sync"] .activity-dot { background: var(--accent); }
  .activity-row[data-kind="pull"] .activity-dot { background: oklch(0.7 0.15 200); }
  .activity-row[data-kind="delete"] .activity-dot { background: var(--danger); }
  .activity-row[data-kind="error"] .activity-dot,
  .activity-row[data-kind="block"] .activity-dot { background: var(--danger); }
  .activity-row[data-kind="drift"] .activity-dot { background: var(--fg-subtle); }
  .activity-text { color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .activity-row[data-kind="error"] .activity-text { color: var(--danger); }

  .card-foot {
    display: flex; justify-content: flex-end; gap: 8px;
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-elev-1);
  }
  .btn {
    background: var(--bg-elev-2);
    color: var(--fg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xs);
    padding: 6px 14px;
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary { background: var(--accent); color: oklch(0.99 0 0); border-color: var(--accent); }
  .btn-danger { background: var(--danger); color: oklch(0.99 0 0); border-color: var(--danger); }
  .btn-accent {
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    color: var(--accent);
    border-color: var(--accent);
    display: inline-flex; align-items: center; gap: 6px;
  }
  .btn-accent:hover:not(:disabled) {
    background: color-mix(in oklch, var(--accent) 30%, transparent);
  }
</style>
