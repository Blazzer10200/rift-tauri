<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { X, RefreshCw, AlertTriangle, CheckCircle2, Ban, Download, Upload } from "lucide-svelte";
  import { syncModal, type SyncActivityKind } from "../../state/sync-modal.svelte";
  import { connection } from "../../state/connection.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  type DiagEvent = { stage: string; fields: unknown; at?: string; resource?: string | null };
  type DriftProgressFields = { current?: number; total?: number; resource?: string };
  type DriftResultFields = {
    entries?: number;
    to_push?: number;
    to_pull?: number;
    to_delete?: number;
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
  let pushing = $state(false);
  // After 3s of waiting for backend cancel-ack, surface a force-close
  // escape (down from 5s — user feedback). Backend keeps bailing in the
  // background; user gets the modal back via Run-in-background mode.
  let cancelStartedAt = $state<number | null>(null);
  let cancelTickMs = $state(0);
  let cancelTicker: ReturnType<typeof setInterval> | null = null;
  const canForceClose = $derived(cancelling && cancelTickMs > 3000);

  // Hard watchdog: if 60s pass between op start and the first result event,
  // the backend is hung/panicked/dead-socket. Force-fail the modal so the
  // user is never trapped staring at "Pushing pending local edits…" forever.
  // 60s aligns w/ russh keepalive (20s × 3) — any healthy server replies
  // way before this.
  const HARD_WATCHDOG_MS = 60_000;
  let opStartedAt = $state<number | null>(null);
  let opWatchdog: ReturnType<typeof setTimeout> | null = null;

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
        const startText = syncModal.mode === "pull"
          ? "pull-now started"
          : syncModal.mode === "push"
            ? "push-now started"
            : "reconcile started";
        syncModal.pushActivity({
          at: new Date().toISOString(),
          kind: "drift",
          text: startText,
        });
        // Scan + cold-cache Pull both do a slow SFTP batch listing before
        // per-folder progress events fire. Push has nothing to list — it
        // just drains the dirty queue.
        if (syncModal.mode !== "push") {
          syncModal.pushActivity({
            at: new Date().toISOString(),
            kind: "drift",
            text: "listing remote files (this can take a moment)…",
          });
        }
      } else if (stage === "drift_scan_progress") {
        // Push mode doesn't scan — any progress events arriving are stale.
        if (syncModal.mode === "push") return;
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
          deletes: f.to_delete ?? 0,
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
        cancelStartedAt = null;
        cancelTickMs = 0;
        if (cancelTicker !== null) { clearInterval(cancelTicker); cancelTicker = null; }
        // Result landed — disarm the hard watchdog.
        opStartedAt = null;
        if (opWatchdog !== null) { clearTimeout(opWatchdog); opWatchdog = null; }
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

  // Listeners stay attached while the backend op is `busy`, even after the
  // user dismisses the modal via "Run in background". This way the busy flag
  // clears when drift_scan_result fires, and the activity bar can re-enable.
  $effect(() => {
    if (syncModal.busy || syncModal.open) {
      if (!unlistenDiag && !unlistenActivity) {
        lastEventAt = Date.now();
        attachListeners();
      }
      if (watchdogTimer === null && syncModal.open) {
        watchdogTimer = setInterval(() => {
          if (syncModal.phase !== "scanning") return;
          const silent = Date.now() - lastEventAt;
          if (silent > 30000) syncModal.setStalled();
        }, 5000);
      }
      // Arm the hard watchdog the first time we transition into busy.
      if (opStartedAt === null && syncModal.busy) {
        opStartedAt = Date.now();
        if (opWatchdog !== null) clearTimeout(opWatchdog);
        opWatchdog = setTimeout(() => {
          if (syncModal.phase === "scanning") {
            console.error(
              `[rift] Sync hard-watchdog fired after ${HARD_WATCHDOG_MS}ms — backend never emitted drift_scan_result. Forcing modal to error state.`
            );
            syncModal.fail(
              "Backend stopped responding. Op may still be running; check the Activity tab. Reconnect if needed."
            );
          }
        }, HARD_WATCHDOG_MS);
      }
    } else {
      detachListeners();
      opStartedAt = null;
      if (opWatchdog !== null) { clearTimeout(opWatchdog); opWatchdog = null; }
    }
  });

  onDestroy(detachListeners);

  async function onCancel() {
    if (cancelling) return;
    cancelling = true;
    cancelStartedAt = Date.now();
    cancelTickMs = 0;
    if (cancelTicker === null) {
      cancelTicker = setInterval(() => {
        cancelTickMs = Date.now() - (cancelStartedAt ?? Date.now());
      }, 250);
    }
    try {
      await invoke<boolean>("sync_cancel");
    } catch (err) {
      console.warn("cancel failed", err);
      cancelling = false;
      cancelStartedAt = null;
      cancelTickMs = 0;
      if (cancelTicker !== null) { clearInterval(cancelTicker); cancelTicker = null; }
    }
  }

  function onForceClose() {
    // Demote to background: keep busy true so the StatusBar pill tracks the
    // op until the backend eventually emits drift_scan_result; just hide the
    // modal. User gets their UI back instantly.
    syncModal.runInBackground();
  }

  async function onPullNow() {
    if (pulling) return;
    pulling = true;
    syncModal.start("pull");
    try {
      const fired = await invoke<boolean>("sync_pull_pending");
      if (!fired) syncModal.fail("Not connected — start auto-sync first.");
    } catch (err) {
      syncModal.fail(String(err));
    } finally {
      pulling = false;
    }
  }

  async function onPushNow() {
    if (pushing) return;
    pushing = true;
    syncModal.start("push");
    try {
      const fired = await invoke<boolean>("sync_push_pending");
      if (!fired) syncModal.fail("Not connected — start auto-sync first.");
    } catch (err) {
      syncModal.fail(String(err));
    } finally {
      pushing = false;
    }
  }

  function onDismiss() {
    syncModal.dismiss();
  }

  function onRunInBackground() {
    syncModal.runInBackground();
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
    <div class="card" data-mode={syncModal.mode}>
      <header class="card-head">
        <div class="title-row">
          {#if syncModal.phase === "scanning"}
            {#if syncModal.mode === "pull"}
              <Download size={16} class="pulse-down"/>
              <h2 id="sync-modal-title">Pulling from {profileName}</h2>
            {:else if syncModal.mode === "push"}
              <Upload size={16} class="pulse-up"/>
              <h2 id="sync-modal-title">Pushing to {profileName}</h2>
            {:else}
              <RefreshCw size={16} class="pulse-spin"/>
              <h2 id="sync-modal-title">Scanning {profileName}</h2>
            {/if}
          {:else if syncModal.phase === "complete"}
            <CheckCircle2 size={16}/>
            <h2 id="sync-modal-title">{syncModal.mode === "pull" ? "Pull complete" : syncModal.mode === "push" ? "Push complete" : "Scan complete"}</h2>
          {:else if syncModal.phase === "cancelled"}
            <Ban size={16}/>
            <h2 id="sync-modal-title">{syncModal.mode === "pull" ? "Pull cancelled" : syncModal.mode === "push" ? "Push cancelled" : "Scan cancelled"}</h2>
          {:else}
            <AlertTriangle size={16}/>
            <h2 id="sync-modal-title">{syncModal.mode === "pull" ? "Pull error" : syncModal.mode === "push" ? "Push error" : "Scan error"}</h2>
          {/if}
        </div>
        <button
          type="button"
          class="head-x"
          onclick={() => (canForceClose ? onForceClose() : onDismiss())}
          disabled={syncModal.phase === "scanning" && !canForceClose}
          aria-label="Close"
          use:tooltip={syncModal.phase === "scanning"
            ? (canForceClose ? "Close anyway — op continues in background" : "Cancel first")
            : "Close"}
        ><X size={14}/></button>
      </header>

      <section class="progress-section">
        <div class="bar-track">
          <div class="bar-fill" style="width: {pct}%"></div>
        </div>
        <div class="status-line mono">
          {#if syncModal.phase === "scanning"}
            {#if syncModal.mode === "push"}
              Pushing pending local edits…
            {:else if syncModal.totalFolders > 0}
              {syncModal.mode === "pull" ? "Pulling" : "Scanning"} {syncModal.resource || "…"} — {syncModal.currentFolder} / {syncModal.totalFolders} folders
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

      <section class="counts" data-cols={syncModal.mode === "push" ? 1 : syncModal.mode === "pull" ? 3 : 4}>
        {#if syncModal.mode === "push"}
          <div class="count-cell" data-tone="push">
            <div class="cell-num mono">{syncModal.result?.push ?? 0}</div>
            <div class="cell-label">Pushed</div>
          </div>
        {:else}
          <div class="count-cell" data-tone="push">
            <div class="cell-num mono">{syncModal.result?.push ?? 0}</div>
            <div class="cell-label">To Push</div>
          </div>
          <div class="count-cell" data-tone="pull">
            <div class="cell-num mono">{syncModal.result?.pull ?? 0}</div>
            <div class="cell-label">{syncModal.mode === "pull" ? "Pulled" : "To Pull"}</div>
          </div>
          <div class="count-cell" data-tone="warn">
            <div class="cell-num mono">{syncModal.result?.deletes ?? 0}</div>
            <div class="cell-label">{syncModal.mode === "pull" ? "Removed" : "To Delete"}</div>
          </div>
          <div class="count-cell" data-tone={(syncModal.result?.conflicts ?? 0) > 0 ? "danger" : "ok"}>
            <div class="cell-num mono">{syncModal.result?.conflicts ?? 0}</div>
            <div class="cell-label">Conflicts</div>
          </div>
        {/if}
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
          {#if canForceClose}
            <button type="button" class="btn btn-ghost" onclick={onForceClose}
              use:tooltip={"Close the modal — backend will keep bailing in the background. Status bar will track it."}>
              Close anyway
            </button>
          {:else}
            <button type="button" class="btn btn-ghost" onclick={onRunInBackground}>
              Run in background
            </button>
          {/if}
          <button type="button" class="btn btn-danger" onclick={onCancel} disabled={cancelling}>
            {cancelling
              ? "Cancelling…"
              : (syncModal.mode === "pull" ? "Stop pull" : syncModal.mode === "push" ? "Stop push" : "Cancel scan")}
          </button>
        {:else}
          <!-- Pull Now / Push Now follow-ups only make sense after a SCAN —
               they're "I just scanned, now act on it" affordances. After a
               pull or push completes, the counts represent work that just
               happened, not pending work — so these would be lies. -->
          {#if syncModal.phase === "complete" && syncModal.mode === "scan" && (syncModal.result?.pull ?? 0) > 0}
            <button type="button" class="btn btn-accent" onclick={onPullNow} disabled={pulling}>
              <Download size={13}/>
              {pulling ? "Pulling…" : `Pull Now (${syncModal.result?.pull ?? 0})`}
            </button>
          {/if}
          {#if syncModal.phase === "complete" && syncModal.mode === "scan" && (syncModal.result?.push ?? 0) > 0}
            <button type="button" class="btn btn-accent" onclick={onPushNow} disabled={pushing}>
              <Upload size={13}/>
              {pushing ? "Pushing…" : `Push Now (${syncModal.result?.push ?? 0})`}
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
  /* Scan mode breathing rotation — 2.4s ease-in-out cycle, no jitter.
     Replaces the linear 1.2s spin that read as a frantic loader. */
  :global(.pulse-spin) {
    animation: pulse-spin 2.4s ease-in-out infinite;
    transform-origin: center;
  }
  @keyframes pulse-spin {
    0%   { transform: rotate(0deg);   opacity: 0.95; }
    50%  { transform: rotate(180deg); opacity: 0.65; }
    100% { transform: rotate(360deg); opacity: 0.95; }
  }

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
  /* Pull mode: lean into the app's purple accent (oklch hue 275). Was blue
     earlier — clashed w/ the rest of the chrome. The differentiation from
     the Scan modal comes from the title copy + icon + count layout, not a
     palette swap. */
  .card[data-mode="pull"] .bar-fill,
  .card[data-mode="push"] .bar-fill { background: var(--accent); }
  .card[data-mode="pull"] .card-head,
  .card[data-mode="push"] .card-head {
    background: color-mix(in oklch, var(--accent-soft) 80%, transparent);
    border-bottom-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  .card[data-mode="pull"] .title-row :global(svg),
  .card[data-mode="push"] .title-row :global(svg) { color: var(--accent); }
  /* Subtle downward bob on the download icon — reads as "files flowing in"
     without the spinning-arrow nonsense that read as a loader. ~1.6s cycle,
     2px travel, opacity dip to imply motion w/o being distracting. */
  :global(.pulse-down) {
    animation: pulse-down 1.6s ease-in-out infinite;
  }
  :global(.pulse-up) {
    animation: pulse-up 1.6s ease-in-out infinite;
  }
  @keyframes pulse-down {
    0%, 100% { transform: translateY(0); opacity: 0.95; }
    50%      { transform: translateY(2px); opacity: 0.65; }
  }
  @keyframes pulse-up {
    0%, 100% { transform: translateY(0); opacity: 0.95; }
    50%      { transform: translateY(-2px); opacity: 0.65; }
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
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    padding: 6px 16px 14px;
  }
  .counts[data-cols="3"] { grid-template-columns: repeat(3, 1fr); }
  .counts[data-cols="1"] { grid-template-columns: minmax(0, 220px); justify-content: center; }
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
  .btn-ghost {
    background: transparent;
    color: var(--fg-muted);
    border-color: var(--border);
  }
  .btn-ghost:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--fg);
  }
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
