import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ServerProfile = {
  key: string;
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  remoteRoot: string;
  localRoot: string;
  fingerprint?: string | null;
  txAdminUrl?: string | null;
  bridgePort?: number | null;
};

export type AutoSyncState = "idle" | "syncing" | "error" | "disabled" | "watching";

export type AutoSyncStatus = {
  state: AutoSyncState;
  detail: string;
  pending: number;
  failed: number;
  ignored_total: number;
  conflicts: number;
  watches: number;
};

export type ActivityKind =
  | "sync" | "pull" | "delete" | "conflict" | "conflict_resolved"
  | "drift" | "bridge" | "block" | "error" | "system";

export type ActivityRow = {
  at: string;
  resource: string;
  file: string;
  action: string;
  kind: ActivityKind;
};

export type ConflictRecord = {
  local_path: string;
  remote_path: string;
  resource_name: string;
  local_size: number;
  local_mtime_utc: string;
  remote_size: number;
  remote_mtime_utc: string;
  last_known_size: number;
  last_known_mtime_utc: string;
};

export type LockEntry = {
  file_path: string;
  user: string;
  host: string;
  since: string;
};

export type WatchedEdit = {
  remotePath: string;
  tmpPath: string;
  displayName: string;
  lastSavedMtime: string;
  lastSavedSize: number;
};

export type EditChangedEvent = {
  remotePath: string;
  tmpPath: string;
  displayName: string;
  mtime: string;
  size: number;
};

export type SyncPill =
  | "Connected"
  | "Syncing"
  | "Conflict"
  | "Lock-blocked"
  | "Disconnected";

class ConnectionStore {
  servers = $state<ServerProfile[]>([]);
  selectedKey = $state<string | null>(null);
  status = $state<AutoSyncStatus | null>(null);
  locks = $state<LockEntry[]>([]);
  conflicts = $state<ConflictRecord[]>([]);
  lastActivity = $state<ActivityRow | null>(null);
  activityFeed = $state<ActivityRow[]>([]);
  watchedEdits = $state<WatchedEdit[]>([]);
  dirtyEdits = $state<Set<string>>(new Set());

  selected = $derived(
    this.selectedKey
      ? this.servers.find((s) => s.key === this.selectedKey) ?? null
      : null,
  );

  pill = $derived<SyncPill>(this.computePill());

  lockCount = $derived(this.locks.length);
  conflictCount = $derived(this.conflicts.length);

  private unlisteners: UnlistenFn[] = [];

  private computePill(): SyncPill {
    if (!this.status) return "Disconnected";
    if (this.conflictCount > 0) return "Conflict";
    if (this.locks.length > 0) return "Lock-blocked";
    if (this.status.state === "syncing") return "Syncing";
    if (this.status.state === "error") return "Disconnected";
    if (this.status.state === "watching" || this.status.state === "idle") {
      return "Connected";
    }
    return "Disconnected";
  }

  async loadServers() {
    this.servers = await invoke<ServerProfile[]>("list_servers");
    if (!this.selectedKey) {
      const last = await invoke<string | null>("get_last_selected");
      if (last && this.servers.some((s) => s.key === last)) {
        this.selectedKey = last;
      }
    }
  }

  async select(key: string) {
    this.selectedKey = key;
    try {
      await invoke("set_last_selected", { key });
    } catch (e) {
      console.error("set_last_selected failed", e);
    }
  }

  async deleteServer(key: string) {
    await invoke("delete_server", { key });
    if (this.selectedKey === key) {
      this.selectedKey = null;
    }
    await this.loadServers();
  }

  async wireEvents() {
    if (this.unlisteners.length) return;
    this.unlisteners.push(
      await listen<AutoSyncStatus>("autosync://status", (e) => {
        this.status = e.payload;
      }),
    );
    this.unlisteners.push(
      await listen<LockEntry[]>("autosync://locks", (e) => {
        this.locks = e.payload ?? [];
      }),
    );
    this.unlisteners.push(
      await listen<ConflictRecord>("autosync://conflict", (e) => {
        const cur = this.conflicts.filter(
          (c) => c.local_path !== e.payload.local_path,
        );
        cur.push(e.payload);
        this.conflicts = cur;
      }),
    );
    this.unlisteners.push(
      await listen<ActivityRow>("autosync://activity", (e) => {
        this.lastActivity = e.payload;
        const next = [e.payload, ...this.activityFeed];
        if (next.length > 200) next.length = 200;
        this.activityFeed = next;
      }),
    );
    this.unlisteners.push(
      await listen<EditChangedEvent>("edit://changed", (e) => {
        const next = new Set(this.dirtyEdits);
        next.add(e.payload.remotePath);
        this.dirtyEdits = next;
      }),
    );
    this.unlisteners.push(
      await listen<{ remotePath: string; success: boolean; error: string }>(
        "edit://reuploaded",
        (e) => {
          if (e.payload.success) {
            const next = new Set(this.dirtyEdits);
            next.delete(e.payload.remotePath);
            this.dirtyEdits = next;
          }
        },
      ),
    );
  }

  async refreshWatchedEdits() {
    try {
      this.watchedEdits = await invoke<WatchedEdit[]>("list_watched_edits");
    } catch (e) {
      console.error("list_watched_edits failed", e);
    }
  }

  lockForRemotePath(remotePath: string): LockEntry | null {
    if (!remotePath) return null;
    return this.locks.find((l) => l.file_path === remotePath) ?? null;
  }

  lockForBasename(name: string): LockEntry | null {
    if (!name) return null;
    return (
      this.locks.find((l) => {
        const i = l.file_path.lastIndexOf("/");
        const base = i === -1 ? l.file_path : l.file_path.slice(i + 1);
        return base === name;
      }) ?? null
    );
  }

  disposeEvents() {
    for (const u of this.unlisteners) u();
    this.unlisteners = [];
  }

  async refreshStatus() {
    try {
      const s = await invoke<AutoSyncStatus | null>("get_autosync_status");
      this.status = s;
    } catch {}
  }

  async disconnect() {
    try {
      await invoke("stop_autosync");
    } catch (e) {
      console.error(e);
    }
    this.status = null;
    this.locks = [];
    this.conflicts = [];
  }
}

export const connection = new ConnectionStore();
