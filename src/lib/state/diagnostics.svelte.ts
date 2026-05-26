import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DiagStage =
  | "fs_event" | "ignored" | "debounced" | "queued" | "queue_dropped"
  | "upload_start" | "upload_done" | "upload_fail" | "atomic_rename"
  | "lock_acquired" | "lock_released" | "lock_held_by_other"
  | "drift_scan_start" | "drift_scan_progress" | "drift_scan_result"
  | "bridge_ping" | "bridge_ack"
  | "rescan_signal" | "sftp_connect" | "sftp_disconnect"
  | "remote_scan_start" | "remote_scan_result"
  | "remote_pull_start" | "remote_pull_done" | "remote_pull_fail"
  | "log" | "system";

export type DiagLevel = "trace" | "debug" | "info" | "warn" | "error";

export type DiagEvent = {
  at: string;
  seq: number;
  stage: DiagStage;
  level: DiagLevel;
  resource: string | null;
  file: string | null;
  message: string;
  fields: unknown;
};

export type DiagState = {
  at: string;
  autosync_state: string | null;
  autosync_detail: string;
  watcher_count: number;
  queue_pending: number;
  queue_failed: number;
  queue_dropped_total: number;
  ignored_total: number;
  conflicts: number;
  last_drift_scan_at: string | null;
  last_rescan_signal_at: string | null;
  bus_lag_total: number;
  events_emitted_total: number;
};

const RING_CAP = 1000;

const ALL_STAGES: DiagStage[] = [
  "fs_event", "ignored", "debounced", "queued", "queue_dropped",
  "upload_start", "upload_done", "upload_fail", "atomic_rename",
  "lock_acquired", "lock_released", "lock_held_by_other",
  "drift_scan_start", "drift_scan_progress", "drift_scan_result",
  "bridge_ping", "bridge_ack",
  "rescan_signal", "sftp_connect", "sftp_disconnect",
  "remote_scan_start", "remote_scan_result",
  "remote_pull_start", "remote_pull_done", "remote_pull_fail",
  "log", "system",
];

const LEVEL_ORDER: Record<DiagLevel, number> = {
  trace: 0, debug: 1, info: 2, warn: 3, error: 4,
};

class DiagnosticsStore {
  events = $state<DiagEvent[]>([]);
  state = $state<DiagState | null>(null);
  paused = $state(false);
  enabledStages = $state<Set<DiagStage>>(new Set(ALL_STAGES));
  minLevel = $state<DiagLevel>("debug");
  textFilter = $state("");

  filtered = $derived.by(() => {
    const q = this.textFilter.trim().toLowerCase();
    const minLv = LEVEL_ORDER[this.minLevel];
    return this.events.filter((e) => {
      if (!this.enabledStages.has(e.stage)) return false;
      if (LEVEL_ORDER[e.level] < minLv) return false;
      if (!q) return true;
      const hay =
        (e.resource ?? "") + " " +
        (e.file ?? "") + " " +
        e.message + " " + e.stage;
      return hay.toLowerCase().includes(q);
    });
  });

  countsByStage = $derived.by(() => {
    const out: Partial<Record<DiagStage, number>> = {};
    for (const e of this.events) {
      out[e.stage] = (out[e.stage] ?? 0) + 1;
    }
    return out;
  });

  private unlisteners: UnlistenFn[] = [];
  private wiring = false;
  // #249: tracks whether we've incremented the backend pump subscriber count,
  // so dispose() decrements exactly once per successful wire(). Component-level
  // mount/unmount can't be a 1:1 with wire/dispose because the store is a
  // singleton + wire() is re-entry-guarded.
  private subscribed = false;

  async wire() {
    if (this.wiring || this.unlisteners.length) return;
    this.wiring = true;
    try {
      this.unlisteners.push(
        await listen<DiagEvent>("diag://event", (e) => {
          if (this.paused) return;
          // Prepend + cap. New array each push so $derived updates.
          const next = [e.payload, ...this.events];
          if (next.length > RING_CAP) next.length = RING_CAP;
          this.events = next;
        }),
      );
      this.unlisteners.push(
        await listen<DiagState>("diag://state", (e) => {
          this.state = e.payload;
        }),
      );
      // Pull initial state immediately so the panel isn't empty for the
      // first 500ms after mount.
      try {
        this.state = await invoke<DiagState>("diag_get_state");
      } catch (err) {
        console.error("diag_get_state failed", err);
      }
      // #249: ask the backend pump to start its 500ms emit cadence. The pump
      // is always running but no-ops when subscriber count is 0.
      try {
        await invoke("diag_subscribe_state");
        this.subscribed = true;
      } catch (err) {
        console.warn("diag_subscribe_state failed (pump may stay idle)", err);
      }
    } catch (err) {
      console.error("diagnostics wire failed", err);
      this.dispose();
    } finally {
      this.wiring = false;
    }
  }

  dispose() {
    for (const u of this.unlisteners) u();
    this.unlisteners = [];
    // #249: decrement the backend pump refcount so it can go idle when no
    // Diagnostics panel is mounted. Fire-and-forget — IPC failure here just
    // means the pump emits for a few extra cycles, not a correctness issue.
    if (this.subscribed) {
      this.subscribed = false;
      void invoke("diag_unsubscribe_state").catch(() => {});
    }
  }

  clear() {
    this.events = [];
  }

  togglePause() {
    this.paused = !this.paused;
  }

  toggleStage(stage: DiagStage) {
    const next = new Set(this.enabledStages);
    if (next.has(stage)) next.delete(stage);
    else next.add(stage);
    this.enabledStages = next;
  }

  setAllStages(on: boolean) {
    this.enabledStages = new Set(on ? ALL_STAGES : []);
  }

  /**
   * One-shot diagnostic bundle — every signal Claude needs to triage a sync
   * issue, end-to-end. Fires a drift reconcile and waits up to 2s for the
   * result event to flow into the stream so the report includes fresh
   * reconcile data, not the stale prior snapshot.
   *
   * Sanitization: omits key_path + fingerprint from the profile (machine-
   * specific paths + fingerprints can be sensitive in shared transcripts).
   */
  async generateReport(): Promise<string> {
    const { connection } = await import("./connection.svelte");
    let appVersion = "unknown";
    try { appVersion = await invoke<string>("app_version"); } catch {}
    let snapshotPath: string | null = null;
    if (connection.selectedKey) {
      snapshotPath = await this.snapshotPath(connection.selectedKey);
    }
    // Trigger drift reconcile + wait briefly for the result event so the
    // report captures fresh entry counts. 2s timeout — most reconciles
    // finish in <500ms but a slow link shouldn't block the user forever.
    // One-shot event listener (registered BEFORE forceDriftScan so we don't
    // miss a fast response) instead of polling the ring buffer every 50ms.
    let unlistenDrift = null as UnlistenFn | null;
    const driftResult = new Promise<void>((resolve) => {
      const timeoutId = setTimeout(() => {
        unlistenDrift?.();
        unlistenDrift = null;
        resolve();
      }, 2000);
      void listen<DiagEvent>("diag://event", (e) => {
        if (e.payload.stage === "drift_scan_result") {
          clearTimeout(timeoutId);
          unlistenDrift?.();
          unlistenDrift = null;
          resolve();
        }
      }).then((u) => {
        if (unlistenDrift === null) { u(); return; } // already resolved
        unlistenDrift = u;
      });
    });
    const driftFired = await this.forceDriftScan();
    if (driftFired) {
      await driftResult;
    } else {
      // No scan fired — tear down the listener immediately rather than waiting 2s.
      unlistenDrift?.();
      unlistenDrift = null;
    }
    let ignoredBreakdown: Record<string, number> = {};
    try {
      ignoredBreakdown = await invoke<Record<string, number>>("diag_ignored_breakdown");
    } catch (e) {
      console.error("diag_ignored_breakdown failed", e);
    }
    const sel = connection.selected;
    const profile = sel
      ? {
          key: sel.key,
          name: sel.name,
          host: sel.host,
          port: sel.port,
          user: sel.user,
          remote_root: sel.remoteRoot,
          local_root: sel.localRoot,
          tx_admin_url: sel.txAdminUrl ?? null,
          bridge_port: sel.bridgePort ?? null,
        }
      : null;
    const bundle = {
      meta: {
        generated_at: new Date().toISOString(),
        rift_version: appVersion,
        platform: navigator.platform,
        user_agent: navigator.userAgent,
      },
      pipeline_state: this.state,
      profile,
      snapshot_path: snapshotPath,
      autosync_status: connection.status,
      locks: connection.locks,
      conflicts: connection.conflicts,
      watched_edits: connection.watchedEdits,
      dirty_edits: Array.from(connection.dirtyEdits),
      activity_feed: connection.activityFeed.slice(0, 100),
      diag_events: this.events.slice(0, 200),
      counts_by_stage: this.countsByStage,
      ignored_by_rule: ignoredBreakdown,
    };
    return JSON.stringify(bundle, null, 2);
  }

  async forceDriftScan(): Promise<boolean> {
    try {
      return await invoke<boolean>("sync_reconcile");
    } catch (e) {
      console.error("sync_reconcile failed", e);
      return false;
    }
  }

  async snapshotPath(serverKey: string): Promise<string | null> {
    try {
      return await invoke<string>("diag_snapshot_path", { serverKey });
    } catch (e) {
      console.error("diag_snapshot_path failed", e);
      return null;
    }
  }

}

export const diagnostics = new DiagnosticsStore();
