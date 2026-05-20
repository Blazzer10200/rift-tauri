// Sync page state — the unified drift-review + apply surface.
// Replaces the corner-rail Quick Actions + the transient scan modal.
// Pulls cached drift entries from the backend via `sync_get_drift_snapshot`
// and exposes selection + apply operations.

import { invoke } from "@tauri-apps/api/core";

export type DriftBucket = "synced" | "to_push" | "to_pull" | "to_delete" | "to_delete_remote" | "conflict";

export type DriftEntry = {
  resource_name: string;
  rel_path: string;
  local_path: string;
  remote_path: string;
  bucket: DriftBucket;
  local_exists: boolean;
  remote_exists: boolean;
  local_size: number;
  remote_size: number;
  local_mtime: string | null;
  remote_mtime: string | null;
  has_snapshot: boolean;
  reason: string;
};

export type AbortedShrunkFolder = {
  resource_name: string;
  remote_root: string;
  baseline_count: number;
  listing_count: number;
};

export type ResourceGroup = {
  resource: string;
  to_push: DriftEntry[];
  to_pull: DriftEntry[];
  to_delete: DriftEntry[];
  to_delete_remote: DriftEntry[];
  conflict: DriftEntry[];
  total_pending: number;
};

class SyncPageStore {
  loading = $state(false);
  busy = $state(false);
  errorMsg = $state<string | null>(null);
  scannedAt = $state<number | null>(null);
  entries = $state<DriftEntry[]>([]);
  abortedShrunk = $state<AbortedShrunkFolder[]>([]);
  dismissedShrunk = $state<Set<string>>(new Set());
  rebaselining = $state<Set<string>>(new Set());
  confirmingRebaseline = $state<string | null>(null);
  lastRebaselineResult = $state<{ resource: string; oldCount: number; newCount: number; localOnly: number } | null>(null);
  expanded = $state<Set<string>>(new Set());
  selected = $state<Set<string>>(new Set());
  // v0.2.53 Mirror mode: opt-in destructive remote-delete. Default false.
  // Session-scoped on the backend; synced via sync_set_mirror_mode.
  mirrorEnabled = $state(false);
  // Tracks whether a typed-confirm gate is open. Holds the bucket count
  // being mirrored so the modal can show "delete N remote files".
  mirrorConfirm = $state<{ count: number } | null>(null);
  // v0.2.55: which server-key we've already auto-scanned once for this
  // session. Cleared on disconnect so a re-select re-fires.
  firstScanFiredForServer = $state<string | null>(null);
  // v0.2.55: current phase of the combined syncNow() chain. Drives the
  // primary button label so users see "Pulling…" → "Pushing…" instead of
  // a generic spinner.
  syncPhase = $state<"idle" | "pulling" | "pushing">("idle");
  // v0.2.55: opt-in periodic rescan — catches remote-side drift (teammates
  // pushing, out-of-band edits) that the local watcher can't see. Persisted
  // to localStorage. Timer lives in AppShell so it survives tab changes.
  autoRescanEnabled = $state(false);
  autoRescanIntervalSec = $state(120);
  static AUTO_RESCAN_PRESETS = [30, 60, 120, 300, 600] as const;
  // v0.2.55: design-preview mode. Injects a synthetic fixture covering every
  // bucket + shrunk banner so the page renders in its fully-populated state
  // without needing real drift. Apply/Pull/Push buttons are gated while
  // active so fake paths don't reach the backend. Toggle off → refresh()
  // restores real data.
  previewMode = $state(false);
  private preReviewSnapshot: {
    entries: DriftEntry[];
    abortedShrunk: AbortedShrunkFolder[];
    scannedAt: number | null;
    expanded: Set<string>;
    mirrorEnabled: boolean;
    selected: Set<string>;
  } | null = null;

  get groups(): ResourceGroup[] {
    const map = new Map<string, ResourceGroup>();
    for (const e of this.entries) {
      let g = map.get(e.resource_name);
      if (!g) {
        g = {
          resource: e.resource_name,
          to_push: [],
          to_pull: [],
          to_delete: [],
          to_delete_remote: [],
          conflict: [],
          total_pending: 0,
        };
        map.set(e.resource_name, g);
      }
      if (e.bucket === "to_push") g.to_push.push(e);
      else if (e.bucket === "to_pull") g.to_pull.push(e);
      else if (e.bucket === "to_delete") g.to_delete.push(e);
      else if (e.bucket === "to_delete_remote") g.to_delete_remote.push(e);
      else if (e.bucket === "conflict") g.conflict.push(e);
    }
    for (const g of map.values()) {
      g.total_pending = g.to_push.length + g.to_pull.length + g.to_delete.length + g.to_delete_remote.length + g.conflict.length;
    }
    return Array.from(map.values())
      .filter((g) => g.total_pending > 0)
      .sort((a, b) => b.total_pending - a.total_pending);
  }

  get totals() {
    let push = 0, pull = 0, del = 0, delRemote = 0, conf = 0;
    for (const e of this.entries) {
      if (e.bucket === "to_push") push++;
      else if (e.bucket === "to_pull") pull++;
      else if (e.bucket === "to_delete") del++;
      else if (e.bucket === "to_delete_remote") delRemote++;
      else if (e.bucket === "conflict") conf++;
    }
    return { push, pull, del, delRemote, conf, total: push + pull + del + delRemote + conf };
  }

  async refresh() {
    this.loading = true;
    this.errorMsg = null;
    try {
      const [list, aborted] = await Promise.all([
        invoke<DriftEntry[]>("sync_get_drift_snapshot"),
        invoke<AbortedShrunkFolder[]>("sync_get_aborted_shrunk"),
      ]);
      this.entries = list;
      this.abortedShrunk = aborted;
      this.scannedAt = Date.now();
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.loading = false;
    }
  }

  /** Visible (non-dismissed) aborted folders. Frontend-only dismissal —
   *  dismissed folders re-appear on next reconcile if guard still trips. */
  get visibleAbortedShrunk(): AbortedShrunkFolder[] {
    return this.abortedShrunk.filter((a) => !this.dismissedShrunk.has(a.remote_root));
  }

  dismissShrunk(remoteRoot: string) {
    const next = new Set(this.dismissedShrunk);
    next.add(remoteRoot);
    this.dismissedShrunk = next;
  }

  requestRebaseline(remoteRoot: string) {
    this.confirmingRebaseline = remoteRoot;
  }

  cancelRebaseline() {
    this.confirmingRebaseline = null;
  }

  /** Resolve the remote_subpath (e.g. "[endure]") from a remote_root by stripping
   *  the profile remote_root prefix. Backend reconstructs the full path. */
  private subpathFromRoot(remoteRoot: string, target: AbortedShrunkFolder): string {
    // resource_name is the bracket label; remote_subpath in detect_bootstrap is
    // identical. Use resource_name as the subpath — backend joins to profile root.
    return target.resource_name;
  }

  async confirmRebaseline() {
    const remoteRoot = this.confirmingRebaseline;
    if (!remoteRoot) return;
    const target = this.abortedShrunk.find((a) => a.remote_root === remoteRoot);
    if (!target) {
      this.confirmingRebaseline = null;
      return;
    }
    const subpath = this.subpathFromRoot(remoteRoot, target);
    const next = new Set(this.rebaselining);
    next.add(remoteRoot);
    this.rebaselining = next;
    this.confirmingRebaseline = null;
    this.errorMsg = null;
    try {
      const result = await invoke<[number, number, number]>("sync_rebaseline_folder", {
        remoteSubpath: subpath,
      });
      this.lastRebaselineResult = {
        resource: target.resource_name,
        oldCount: result[0],
        newCount: result[1],
        localOnly: result[2],
      };
      setTimeout(() => { void this.refresh(); }, 800);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      const after = new Set(this.rebaselining);
      after.delete(remoteRoot);
      this.rebaselining = after;
    }
  }

  /**
   * v0.2.55: auto-scan on first connect per server. Called from AppShell
   * whenever the watcher transitions to a ready state (watching/idle/syncing).
   * Idempotent — only fires once per server-key per session. Cleared on
   * disconnect via `clearAutoScanMark()`.
   */
  maybeAutoScan(serverKey: string | null) {
    if (!serverKey) return;
    if (this.firstScanFiredForServer === serverKey) return;
    if (this.busy || this.loading) return;
    this.firstScanFiredForServer = serverKey;
    void this.rescan();
  }

  /** Clear the auto-scan latch — call on disconnect so reconnect re-fires. */
  clearAutoScanMark() {
    this.firstScanFiredForServer = null;
  }

  /** v0.2.55: hydrate auto-rescan prefs from localStorage. Call once at mount. */
  loadAutoRescanPrefs() {
    if (typeof localStorage === "undefined") return;
    this.autoRescanEnabled = localStorage.getItem("rift.sync.autoRescan.enabled") === "1";
    const iv = parseInt(localStorage.getItem("rift.sync.autoRescan.intervalSec") ?? "", 10);
    if (!Number.isNaN(iv) && SyncPageStore.AUTO_RESCAN_PRESETS.includes(iv as 30 | 60 | 120 | 300 | 600)) {
      this.autoRescanIntervalSec = iv;
    }
  }

  /**
   * v0.2.55: cycle auto-rescan state through off → 30s → 1m → 2m → 5m → 10m → off.
   * Single click in the kebab walks through every preset, mirrors how
   * notification-sound cyclers work on most desktop apps.
   */
  cycleAutoRescan() {
    const presets = SyncPageStore.AUTO_RESCAN_PRESETS;
    if (!this.autoRescanEnabled) {
      this.autoRescanEnabled = true;
      this.autoRescanIntervalSec = presets[0];
    } else {
      const idx = presets.indexOf(this.autoRescanIntervalSec as 30 | 60 | 120 | 300 | 600);
      const next = idx + 1;
      if (next >= presets.length) {
        this.autoRescanEnabled = false;
      } else {
        this.autoRescanIntervalSec = presets[next];
      }
    }
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("rift.sync.autoRescan.enabled", this.autoRescanEnabled ? "1" : "0");
      localStorage.setItem("rift.sync.autoRescan.intervalSec", String(this.autoRescanIntervalSec));
    }
  }

  /** Pretty label for current interval — "30s", "1m", "2m", etc. */
  get autoRescanLabel(): string {
    const s = this.autoRescanIntervalSec;
    return s < 60 ? `${s}s` : `${Math.round(s / 60)}m`;
  }

  /**
   * v0.2.55: enter design-preview mode. Snapshots current state, then
   * injects a fixture covering every bucket + shrunk banner so the page
   * renders in its fully-populated form. exitPreview() restores reality.
   */
  enterPreview() {
    if (this.previewMode) return;
    this.preReviewSnapshot = {
      entries: this.entries,
      abortedShrunk: this.abortedShrunk,
      scannedAt: this.scannedAt,
      expanded: this.expanded,
      mirrorEnabled: this.mirrorEnabled,
      selected: this.selected,
    };
    this.selected = new Set();
    const now = new Date().toISOString();
    const mk = (resource: string, rel: string, bucket: DriftBucket, reason: string): DriftEntry => ({
      resource_name: resource,
      rel_path: rel,
      local_path: `C:/fxserver/resources/${resource}/${rel}`,
      remote_path: `/opt/fxserver/resources/${resource}/${rel}`,
      bucket,
      local_exists: bucket !== "to_pull" && bucket !== "to_delete",
      remote_exists: bucket !== "to_push" && bucket !== "to_delete_remote",
      local_size: 4096,
      remote_size: 4096,
      local_mtime: now,
      remote_mtime: now,
      has_snapshot: true,
      reason,
    });
    this.entries = [
      mk("[endure]", "client.lua",    "to_push",   "local edited, remote untouched"),
      mk("[endure]", "server.lua",    "to_push",   "size+mtime differ"),
      mk("[endure]", "config.json",   "to_pull",   "remote newer than baseline"),
      mk("[endure]", "old_module.lua","to_delete", "local missing, has baseline"),
      mk("qb-core",  "main.lua",      "to_push",   "size+mtime differ"),
      mk("qb-core",  "shared/jobs.lua","to_pull",  "remote-only, missing locally"),
      mk("qb-core",  "shared/items.lua","conflict","both sides edited since last sync"),
      mk("qb-banking","fxmanifest.lua","to_pull", "remote newer"),
      mk("qb-banking","html/index.html","to_delete_remote","local-removed, mirror mode"),
    ];
    this.abortedShrunk = [{
      resource_name: "[abandoned]",
      remote_root: "/opt/fxserver/resources/[abandoned]",
      baseline_count: 142,
      listing_count: 38,
    }];
    this.scannedAt = Date.now();
    this.expanded = new Set(["[endure]", "qb-core"]);
    this.mirrorEnabled = true;
    this.previewMode = true;
  }

  exitPreview() {
    if (!this.previewMode || !this.preReviewSnapshot) {
      this.previewMode = false;
      return;
    }
    const snap = this.preReviewSnapshot;
    this.entries = snap.entries;
    this.abortedShrunk = snap.abortedShrunk;
    this.scannedAt = snap.scannedAt;
    this.expanded = snap.expanded;
    this.mirrorEnabled = snap.mirrorEnabled;
    this.selected = snap.selected;
    this.preReviewSnapshot = null;
    this.previewMode = false;
    void this.refresh();
  }

  async rescan() {
    this.busy = true;
    this.errorMsg = null;
    try {
      const fired = await invoke<boolean>("sync_reconcile");
      if (!fired) {
        this.errorMsg = "Not connected — start watcher first.";
      }
      // Backend caches asynchronously; poll via refresh() after a delay.
      // The scan emits diag events; consumers can listen for completion if
      // they want precise timing. For now, refresh after a short delay so
      // the user sees fresh data without manual click.
      setTimeout(() => { void this.refresh(); }, 800);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }

  toggleExpanded(resource: string) {
    const next = new Set(this.expanded);
    if (next.has(resource)) next.delete(resource);
    else next.add(resource);
    this.expanded = next;
  }

  toggleSelected(path: string) {
    const next = new Set(this.selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    this.selected = next;
  }

  selectAllIn(group: ResourceGroup) {
    const next = new Set(this.selected);
    for (const e of [...group.to_push, ...group.to_pull, ...group.to_delete]) {
      next.add(e.local_path);
    }
    this.selected = next;
  }

  clearSelectionIn(group: ResourceGroup) {
    const paths = new Set([
      ...group.to_push.map((e) => e.local_path),
      ...group.to_pull.map((e) => e.local_path),
      ...group.to_delete.map((e) => e.local_path),
    ]);
    const next = new Set<string>();
    for (const p of this.selected) if (!paths.has(p)) next.add(p);
    this.selected = next;
  }

  clearAllSelection() {
    this.selected = new Set();
  }

  async applySelected() {
    if (this.selected.size === 0) return;
    this.busy = true;
    this.errorMsg = null;
    try {
      const paths = Array.from(this.selected);
      const fired = await invoke<boolean>("sync_apply_selected", { localPaths: paths });
      if (!fired) {
        this.errorMsg = "Not connected — start watcher first.";
      } else {
        this.clearAllSelection();
        // v0.2.55: rescan (not just refresh) so any non-selected drift
        // re-surfaces — applying pulls shouldn't hide pending pushes.
        setTimeout(() => { void this.rescan(); }, 1200);
      }
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }

  /**
   * v0.2.55: one-button sync — pull then push, in that order. Pull first so
   * local is rebased against remote before any push tries to dispatch
   * against a stale baseline. Conflicts stay in conflict bucket (not
   * auto-resolved); mirror remote-deletes stay gated behind typed-confirm.
   * Final rescan re-surfaces anything left behind (e.g. new drift detected
   * mid-pull). Phase between buckets surfaced via `syncPhase` state.
   */
  async syncNow() {
    this.busy = true;
    this.errorMsg = null;
    const hasPullWork = this.totals.pull + this.totals.del > 0;
    const hasPushWork = this.totals.push > 0;
    try {
      if (hasPullWork) {
        this.syncPhase = "pulling";
        const fired = await invoke<boolean>("sync_pull_pending");
        if (!fired) {
          this.errorMsg = "Not connected — start watcher first.";
          return;
        }
        // Give the pull queue room to drain before we push — backend
        // dispatches async and we don't have a hard completion signal yet.
        // Tune if this turns out tight in practice.
        await new Promise((r) => setTimeout(r, 2500));
      }
      if (hasPushWork) {
        this.syncPhase = "pushing";
        const fired = await invoke<boolean>("sync_push_pending");
        if (!fired) {
          this.errorMsg = "Not connected — start watcher first.";
          return;
        }
      }
      // Final rescan re-surfaces any leftover drift + new mid-sync changes.
      setTimeout(() => { void this.rescan(); }, 1200);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
      this.syncPhase = "idle";
    }
  }

  async pullAll() {
    this.busy = true;
    this.errorMsg = null;
    try {
      const fired = await invoke<boolean>("sync_pull_pending");
      if (!fired) this.errorMsg = "Not connected — start watcher first.";
      // v0.2.55: rescan after pull so pending pushes don't get hidden by
      // a stale-cleared snapshot. Plain refresh returned empty entries →
      // "Everything in sync" even when pushes still existed.
      setTimeout(() => { void this.rescan(); }, 1200);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }

  async pushAll() {
    this.busy = true;
    this.errorMsg = null;
    try {
      const fired = await invoke<boolean>("sync_push_pending");
      if (!fired) this.errorMsg = "Not connected — start watcher first.";
      // v0.2.55: rescan (see pullAll comment) — symmetry.
      setTimeout(() => { void this.rescan(); }, 1200);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }

  /**
   * v0.2.53 Mirror mode: toggle the backend's mirror flag. Next Rescan
   * will bucket `local-missing + remote-has + baseline-has` as
   * ToDeleteRemote (destructive) instead of ToPull (restore-from-remote).
   * Auto-refreshes after the toggle so the bucket counts redraw against
   * the cached scan. Session-scoped backend-side — survives nothing.
   */
  async toggleMirror(next: boolean) {
    this.busy = true;
    this.errorMsg = null;
    try {
      const enabled = await invoke<boolean>("sync_set_mirror_mode", { enabled: next });
      this.mirrorEnabled = enabled;
      // Re-scan to rebucket against the new mode.
      const fired = await invoke<boolean>("sync_reconcile");
      if (!fired) this.errorMsg = "Not connected — start watcher first.";
      setTimeout(() => { void this.refresh(); }, 800);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }

  /**
   * v0.2.53 Mirror apply: collect all `to_delete_remote` entries and stage
   * them for the typed-confirm gate. The gate is rendered in SyncPage.svelte
   * — this just opens it. Apply happens via `confirmMirrorApply` after the
   * user types "MIRROR" + clicks Confirm.
   */
  openMirrorConfirm() {
    const count = this.entries.filter((e) => e.bucket === "to_delete_remote").length;
    if (count === 0) {
      this.errorMsg = "No remote-delete entries to mirror. Enable Mirror + Rescan first.";
      return;
    }
    this.mirrorConfirm = { count };
  }

  cancelMirrorConfirm() {
    this.mirrorConfirm = null;
  }

  async confirmMirrorApply() {
    this.busy = true;
    this.errorMsg = null;
    // #140: re-filter at dispatch time. The open-time count from
    // openMirrorConfirm is stale if a rescan rebuckets entries between open
    // and apply — using the captured count or pre-collected paths can ship
    // entries with the wrong bucket / wrong intent. Read live `entries` and
    // assert the bucket inline.
    const paths = this.entries
      .filter((e) => e.bucket === "to_delete_remote")
      .map((e) => e.local_path);
    try {
      await invoke<boolean>("sync_apply_selected", { localPaths: paths });
      this.mirrorConfirm = null;
      // v0.2.55: rescan so any remaining non-mirror drift stays visible.
      setTimeout(() => { void this.rescan(); }, 1200);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }

  /**
   * v0.2.50 manual recovery: reclaim our own stale `.rift-lock` files
   * across every watched remote root. Stays disabled while busy/disconnected;
   * surfaces the swept count in errorMsg as info on success, or the backend
   * error string on failure.
   */
  async sweepStaleLocks() {
    this.busy = true;
    this.errorMsg = null;
    try {
      const n = await invoke<number>("sync_sweep_stale_locks");
      this.errorMsg = n > 0
        ? `Swept ${n} stale lock${n === 1 ? "" : "s"}.`
        : "No stale locks found.";
      setTimeout(() => { void this.refresh(); }, 400);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }
}

export const syncPage = new SyncPageStore();
