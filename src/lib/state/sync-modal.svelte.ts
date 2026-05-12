// Center-stage Sync modal store. TwoPane.onSync() calls `open()`; the modal
// listens to `diag://event` + `autosync://activity` itself and walks the
// state machine. Watchdog (no progress event in 30s) surfaces a stalled
// banner but does NOT fail-out — the backend often takes 45s+ on a deep
// reconcile, and the old 30s hard timeout was the v0.2.19 false-alarm bug.

export type SyncPhase = "scanning" | "complete" | "cancelled" | "error";
export type SyncMode = "scan" | "pull" | "push";

export type SyncActivityKind =
  | "drift" | "sync" | "pull" | "push" | "delete" | "error" | "block" | "system";

export type SyncActivityRow = {
  at: string;
  kind: SyncActivityKind;
  text: string;
};

export type SyncResult = {
  push: number;
  pull: number;
  deletes: number;
  conflicts: number;
  entries: number;
  listing_error?: string | null;
};

const ACTIVITY_CAP = 100;

class SyncModalStore {
  open = $state(false);
  // `busy` tracks the backend op lifecycle. Set true on start(), cleared by
  // complete/cancelled/fail. Independent of `open` — user can dismiss the
  // modal via "Run in background" while the op continues. Drives the
  // TabRail Quick Actions disabled state and the modal's own listener gate.
  busy = $state(false);
  phase = $state<SyncPhase>("scanning");
  mode = $state<SyncMode>("scan");
  currentFolder = $state(0);
  totalFolders = $state(0);
  resource = $state("");
  stalled = $state(false);
  errorMsg = $state<string | null>(null);
  result = $state<SyncResult | null>(null);
  activity = $state<SyncActivityRow[]>([]);

  start(mode: SyncMode = "scan") {
    this.open = true;
    this.busy = true;
    this.phase = "scanning";
    this.mode = mode;
    this.currentFolder = 0;
    this.totalFolders = 0;
    this.resource = "";
    this.stalled = false;
    this.errorMsg = null;
    this.result = null;
    this.activity = [];
  }

  progress(current: number, total: number, resource: string) {
    this.currentFolder = current;
    this.totalFolders = total;
    this.resource = resource;
    this.stalled = false;
  }

  setStalled() { this.stalled = true; }

  complete(result: SyncResult) {
    this.phase = "complete";
    this.busy = false;
    this.result = result;
  }

  cancelled(result: SyncResult | null) {
    this.phase = "cancelled";
    this.busy = false;
    this.result = result;
  }

  fail(msg: string) {
    this.phase = "error";
    this.busy = false;
    this.errorMsg = msg;
  }

  pushActivity(row: SyncActivityRow) {
    const next = [...this.activity, row];
    if (next.length > ACTIVITY_CAP) next.splice(0, next.length - ACTIVITY_CAP);
    this.activity = next;
  }

  // Hide the modal but leave the op running. Backend events keep landing in
  // connection.activityFeed via its own long-lived listener — user sees
  // progress in the Activity tab.
  runInBackground() {
    this.open = false;
  }

  dismiss() {
    this.open = false;
  }
}

export const syncModal = new SyncModalStore();
