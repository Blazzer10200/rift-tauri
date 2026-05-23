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
  /** #9.1: token never crosses the IPC boundary. Backend signals presence
   *  via this flag so the UI can render "bridge configured" state. To set
   *  or rotate the token, send it on `save_server`; an empty value during
   *  edit means "keep existing". */
  hasBridgeToken?: boolean;
  addedAt?: string | null;
};

/** Payload for `save_server` — distinct from the read-side `ServerProfile`
 *  because the write path is the only place the renderer ever supplies a
 *  raw `bridgeToken`. */
export type ServerProfileWritable = Omit<ServerProfile, "hasBridgeToken"> & {
  bridgeToken?: string | null;
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
  rel_path?: string | null;
  local_path?: string | null;
  size_bytes?: number | null;
  latency_ms?: number | null;
  sha?: string | null;
  actor?: string | null;
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
  serverKey: string;
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
  | "Sync error"
  | "Disconnected";

export type FolderSpec = {
  resource_name: string;
  remote_subpath: string;
};

type BootstrapDetection = {
  state: "synced" | "missingLocalRoot" | "empty" | "uninitialized" | "partial" | "badRemoteRoot";
  remoteResourceCount: number;
  localPresentCount: number;
  missingCount: number;
  remoteTopLevelDirs: string[];
};

class ConnectionStore {
  servers = $state<ServerProfile[]>([]);
  selectedKey = $state<string | null>(null);
  status = $state<AutoSyncStatus | null>(null);
  connecting = $state<boolean>(false);
  lastConnectError = $state<string | null>(null);
  wireError = $state<string | null>(null);
  /** Audit C2: captured fingerprint awaiting user confirmation. UI listens
   *  for this and shows the trust-on-first-use modal. */
  pendingFingerprint = $state<string | null>(null);
  locks = $state<LockEntry[]>([]);
  conflicts = $state<ConflictRecord[]>([]);
  lastActivity = $state<ActivityRow | null>(null);
  activityFeed = $state<ActivityRow[]>([]);
  watchedEdits = $state<WatchedEdit[]>([]);
  /** Always replace (new Set(...)) — in-place .add()/.delete() is not reactive in runes mode. */
  dirtyEdits = $state<Set<string>>(new Set());
  pendingReuploads = $state<EditChangedEvent[]>([]);
  /** ms-epoch of the most recent successful scan/sync completion. Stamped on
   *  start_autosync resolve and on every `drift_scan_result` diag event. Null
   *  until the first scan lands. Surfaced by StatusBar as "last scan Xs ago". */
  lastScanAt = $state<number | null>(null);
  /** Phase 2 sync→activity deeplink. SyncPage WatchedFoldersTable row click sets
   *  this to the resource name + flips workspace to "activity"; ActivityFeed reads
   *  it once on mount and clears. Null between handoffs. */
  activityFilter = $state<string | null>(null);

  selected = $derived(
    this.selectedKey
      ? this.servers.find((s) => s.key === this.selectedKey) ?? null
      : null,
  );

  pill = $derived.by<SyncPill>(() => this.computePill());

  /** True only while a handshake is genuinely in flight (probing fingerprint or
   *  waiting for the first `autosync://status` event). Raw `connecting` stays
   *  true through the awaited `startAutosyncForSelected()` call AFTER status
   *  has already arrived via the listener, producing a brief but visible
   *  "Connecting" pill flash when the engine is actually watching. Consumers
   *  driving UI labels should read this; consumers gating actions (button
   *  disabled) should still read raw `connecting` to prevent double-fire. */
  isHandshaking = $derived(this.connecting && this.status === null);

  lockCount = $derived(this.locks.length);
  conflictCount = $derived(this.conflicts.length);

  private unlisteners: UnlistenFn[] = [];
  private wiring: boolean = false;

  private computePill(): SyncPill {
    if (!this.status) return "Disconnected";
    if (this.conflictCount > 0) return "Conflict";
    if (this.locks.length > 0) return "Lock-blocked";
    if (this.status.state === "syncing") return "Syncing";
    if (this.status.state === "error") return "Sync error";
    if (this.status.state === "watching" || this.status.state === "idle") {
      return "Connected";
    }
    return "Disconnected";
  }

  async loadServers() {
    // Audit M9: fetch both before assigning so `selected` doesn't briefly
    // resolve to null between the two awaits.
    const [servers, last] = await Promise.all([
      invoke<ServerProfile[]>("list_servers"),
      this.selectedKey ? Promise.resolve<string | null>(null) : invoke<string | null>("get_last_selected"),
    ]);
    this.servers = servers;
    if (!this.selectedKey && last && servers.some((s) => s.key === last)) {
      this.selectedKey = last;
      this.connect().catch((e) => console.error("auto-connect on load failed", e));
    }
  }

  async select(key: string) {
    const isNew = this.selectedKey !== key;
    this.selectedKey = key;
    try {
      await invoke("set_last_selected", { key });
    } catch (e) {
      console.error("set_last_selected failed", e);
    }
    if (isNew || !this.status) {
      this.connect().catch((e) => console.error("auto-connect failed", e));
    }
  }

  async connect() {
    if (!this.selectedKey) return;
    if (this.connecting) return;
    this.connecting = true;
    this.lastConnectError = null;
    this.pendingFingerprint = null;
    try {
      // Audit C2: if the profile has no pinned fingerprint, probe first and
      // hand the captured value to the UI for explicit user confirmation.
      // Avoids the prior silent TOFU persist.
      if (!this.selected?.fingerprint) {
        const fp = await invoke<string>("probe_server_fingerprint", {
          serverKey: this.selectedKey,
        });
        this.pendingFingerprint = fp;
        // connecting stays true; UI will call confirmFingerprint() or
        // cancelFingerprint() to resolve the flow.
        return;
      }
      await this.startAutosyncForSelected();
      this.connecting = false;
    } catch (e) {
      this.lastConnectError = String(e);
      console.error("connect failed", e);
      this.connecting = false;
      throw e;
    }
  }

  /** Audit C2: user accepted the probed fingerprint. Persist + resume connect. */
  async confirmFingerprint() {
    const key = this.selectedKey;
    const fp = this.pendingFingerprint;
    this.pendingFingerprint = null;
    if (!key || !fp) {
      this.connecting = false;
      return;
    }
    try {
      await invoke("set_server_fingerprint", { serverKey: key, fingerprint: fp });
      await this.loadServers();
      await this.startAutosyncForSelected();
    } catch (e) {
      this.lastConnectError = String(e);
      console.error("confirmFingerprint failed", e);
    } finally {
      this.connecting = false;
    }
  }

  /** Audit C2: user declined the probed fingerprint. Abort connect. */
  cancelFingerprint() {
    this.pendingFingerprint = null;
    this.connecting = false;
    this.lastConnectError = "User declined to trust the server fingerprint.";
  }

  private async startAutosyncForSelected() {
    if (!this.selectedKey) return;
    const det = await invoke<BootstrapDetection>("detect_bootstrap", {
      serverKey: this.selectedKey,
    });
    const folders: FolderSpec[] = (det.remoteTopLevelDirs ?? []).map((d) => ({
      resource_name: d,
      remote_subpath: d,
    }));
    const status = await invoke<AutoSyncStatus>("start_autosync", {
      serverKey: this.selectedKey,
      folders,
    });
    this.status = status;
    this.lastScanAt = Date.now();
  }

  async deleteServer(key: string) {
    await invoke("delete_server", { key });
    if (this.selectedKey === key) {
      this.selectedKey = null;
    }
    await this.loadServers();
  }

  async wireEvents() {
    if (this.wiring || this.unlisteners.length) return;
    this.wiring = true;
    this.wireError = null;
    try {
      this.unlisteners.push(
        await listen<AutoSyncStatus>("autosync://status", (e) => {
          this.status = e.payload;
        }),
      );
      // v0.2.53 auto-reconnect: watch for ConnectionWedged diag events and
      // trigger a soft restart when 3+ fire within 60s.
      this.unlisteners.push(
        await listen<{ stage: string }>("diag://event", (e) => {
          if (e.payload.stage === "connection_wedged") {
            this.handleConnectionWedged();
          } else if (e.payload.stage === "drift_scan_result") {
            this.lastScanAt = Date.now();
          }
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
          this.activityFeed = [e.payload, ...this.activityFeed].slice(0, 200);
        }),
      );
      this.unlisteners.push(
        await listen<EditChangedEvent>("edit://changed", (e) => {
          const key = this.selectedKey;
          if (!key || e.payload.serverKey !== key) {
            console.warn(
              `dropping edit://changed for ${e.payload.serverKey} (selected=${key})`,
            );
            return;
          }
          const next = new Set(this.dirtyEdits);
          next.add(e.payload.remotePath);
          this.dirtyEdits = next;

          const alwaysReupload =
            localStorage.getItem(`rift.reupload.always.${key}`) === "1";
          if (alwaysReupload) {
            void invoke("save_edit_in_place", {
              serverKey: key,
              remotePath: e.payload.remotePath,
            }).catch((err) => console.error("auto-reupload failed", err));
          } else {
            this.pendingReuploads = [...this.pendingReuploads, e.payload];
          }
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
    } catch (e) {
      console.error("wireEvents partial-init failure, rolling back", e);
      this.disposeEvents();
      this.wireError = String(e);
      throw e;
    } finally {
      this.wiring = false;
    }
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

  /**
   * Map a local absolute path to its remote-root-relative equivalent.
   * Returns null if the path isn't under the selected server's localRoot.
   * Used by LocalPane to look up locks by full remote path (not basename),
   * eliminating false-positive lock badges across same-named files.
   */
  remoteForLocalPath(localPath: string): string | null {
    const sel = this.selected;
    if (!sel || !localPath) return null;
    const norm = localPath.replaceAll("\\", "/");
    const root = sel.localRoot.replaceAll("\\", "/").replace(/\/+$/, "");
    if (!root) return null;
    if (!norm.toLowerCase().startsWith(root.toLowerCase() + "/")) return null;
    const rel = norm.slice(root.length + 1);
    if (!rel) return null;
    return sel.remoteRoot.replace(/\/+$/, "") + "/" + rel;
  }

  disposeEvents() {
    for (const u of this.unlisteners) u();
    this.unlisteners = [];
  }

  clearActivity() {
    this.activityFeed = [];
    this.lastActivity = null;
  }

  popReuploadPrompt() {
    if (this.pendingReuploads.length > 0) {
      this.pendingReuploads = this.pendingReuploads.slice(1);
    }
  }

  async cancelDownload() {
    await invoke("cancel_download");
  }

  async refreshStatus() {
    try {
      const s = await invoke<AutoSyncStatus | null>("get_autosync_status");
      this.status = s;
    } catch (e) {
      // Audit H2: surface IPC failures so a stale UI isn't masked.
      console.error("get_autosync_status failed", e);
    }
  }

  // v0.2.53 auto-reconnect: rolling buffer of ConnectionWedged event
  // timestamps. When 3+ fire within AUTO_RECONNECT_WINDOW_MS, trigger
  // a soft reconnect (stop_autosync → wait → start_autosync). Single
  // wedges don't reconnect — those usually self-resolve on the next op.
  // Lives entirely client-side so we don't have to refactor the engine's
  // owned SftpSession (which isn't behind a RwLock).
  private wedgeTimestamps: number[] = [];
  private reconnecting = false;
  // #214: cap retries + exponential backoff so a server that's truly down
  // doesn't burn IPC + log noise forever. Resets on a successful reconnect.
  private reconnectAttempts = 0;
  private static readonly AUTO_RECONNECT_THRESHOLD = 3;
  private static readonly AUTO_RECONNECT_WINDOW_MS = 60_000;
  private static readonly AUTO_RECONNECT_MAX_ATTEMPTS = 5;
  private static readonly AUTO_RECONNECT_BACKOFF_CAP_S = 30;

  private handleConnectionWedged() {
    const now = Date.now();
    this.wedgeTimestamps = this.wedgeTimestamps.filter(
      (t) => now - t < ConnectionStore.AUTO_RECONNECT_WINDOW_MS,
    );
    this.wedgeTimestamps.push(now);
    if (
      this.wedgeTimestamps.length >= ConnectionStore.AUTO_RECONNECT_THRESHOLD &&
      !this.reconnecting &&
      this.selectedKey
    ) {
      this.wedgeTimestamps = [];
      void this.autoReconnect();
    }
  }

  private async autoReconnect() {
    if (this.reconnecting) return;
    if (this.reconnectAttempts >= ConnectionStore.AUTO_RECONNECT_MAX_ATTEMPTS) {
      console.warn("[rift] auto-reconnect: max attempts reached, giving up");
      return;
    }
    this.reconnecting = true;
    // #170: also raise the manual `connecting` flag so concurrent connect()
    // attempts during auto-reconnect short-circuit instead of racing.
    this.connecting = true;
    const attempt = this.reconnectAttempts;
    this.reconnectAttempts++;
    const backoffMs = Math.min(2 ** attempt, ConnectionStore.AUTO_RECONNECT_BACKOFF_CAP_S) * 1000;
    console.warn(
      `[rift] auto-reconnect: attempt ${attempt + 1}/${ConnectionStore.AUTO_RECONNECT_MAX_ATTEMPTS}` +
        ` after ${backoffMs}ms backoff (3+ ConnectionWedged events in 60s)`,
    );
    try {
      await invoke("stop_autosync");
      await new Promise((r) => setTimeout(r, backoffMs));
      await this.startAutosyncForSelected();
      console.info("[rift] auto-reconnect: complete");
      this.reconnectAttempts = 0;
    } catch (e) {
      console.error("[rift] auto-reconnect failed", e);
    } finally {
      this.reconnecting = false;
      this.connecting = false;
    }
  }

  // L11: disconnect tears down the autosync session but deliberately leaves
  // tauri event listeners (`wireEvents`) in place. They're wired once at app
  // start and reused across reconnects — re-wiring on every connect would
  // race with in-flight emits.
  async disconnect() {
    try {
      await invoke("stop_autosync");
    } catch (e) {
      console.error(e);
    }
    this.status = null;
    this.locks = [];
    this.conflicts = [];
    this.lastScanAt = null;
  }
}

export const connection = new ConnectionStore();
