// Center-stage Sync modal store. TwoPane.onSync() calls `open()`; the modal
// listens to `diag://event` + `autosync://activity` itself and walks the
// state machine. Watchdog (no progress event in 30s) surfaces a stalled
// banner but does NOT fail-out — the backend often takes 45s+ on a deep
// reconcile, and the old 30s hard timeout was the v0.2.19 false-alarm bug.

export type SyncPhase = "scanning" | "complete" | "cancelled" | "error";

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
  conflicts: number;
  entries: number;
  listing_error?: string | null;
};

const ACTIVITY_CAP = 100;

class SyncModalStore {
  open = $state(false);
  phase = $state<SyncPhase>("scanning");
  currentFolder = $state(0);
  totalFolders = $state(0);
  resource = $state("");
  stalled = $state(false);
  errorMsg = $state<string | null>(null);
  result = $state<SyncResult | null>(null);
  activity = $state<SyncActivityRow[]>([]);

  start() {
    this.open = true;
    this.phase = "scanning";
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
    this.result = result;
  }

  cancelled(result: SyncResult | null) {
    this.phase = "cancelled";
    this.result = result;
  }

  fail(msg: string) {
    this.phase = "error";
    this.errorMsg = msg;
  }

  pushActivity(row: SyncActivityRow) {
    const next = [...this.activity, row];
    if (next.length > ACTIVITY_CAP) next.splice(0, next.length - ACTIVITY_CAP);
    this.activity = next;
  }

  dismiss() {
    this.open = false;
  }
}

export const syncModal = new SyncModalStore();
