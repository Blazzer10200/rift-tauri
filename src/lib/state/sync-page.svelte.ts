// Sync page state — the unified drift-review + apply surface.
// Replaces the corner-rail Quick Actions + the transient scan modal.
// Pulls cached drift entries from the backend via `sync_get_drift_snapshot`
// and exposes selection + apply operations.

import { invoke } from "@tauri-apps/api/core";

export type DriftBucket = "synced" | "to_push" | "to_pull" | "to_delete" | "conflict";

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
          conflict: [],
          total_pending: 0,
        };
        map.set(e.resource_name, g);
      }
      if (e.bucket === "to_push") g.to_push.push(e);
      else if (e.bucket === "to_pull") g.to_pull.push(e);
      else if (e.bucket === "to_delete") g.to_delete.push(e);
      else if (e.bucket === "conflict") g.conflict.push(e);
    }
    for (const g of map.values()) {
      g.total_pending = g.to_push.length + g.to_pull.length + g.to_delete.length + g.conflict.length;
    }
    return Array.from(map.values())
      .filter((g) => g.total_pending > 0)
      .sort((a, b) => b.total_pending - a.total_pending);
  }

  get totals() {
    let push = 0, pull = 0, del = 0, conf = 0;
    for (const e of this.entries) {
      if (e.bucket === "to_push") push++;
      else if (e.bucket === "to_pull") pull++;
      else if (e.bucket === "to_delete") del++;
      else if (e.bucket === "conflict") conf++;
    }
    return { push, pull, del, conf, total: push + pull + del + conf };
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
        // Re-fetch after a short delay to reflect what the backend cleared.
        setTimeout(() => { void this.refresh(); }, 1200);
      }
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }

  async pullAll() {
    this.busy = true;
    this.errorMsg = null;
    try {
      const fired = await invoke<boolean>("sync_pull_pending");
      if (!fired) this.errorMsg = "Not connected — start watcher first.";
      setTimeout(() => { void this.refresh(); }, 1200);
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
      setTimeout(() => { void this.refresh(); }, 1200);
    } catch (e) {
      this.errorMsg = String(e);
    } finally {
      this.busy = false;
    }
  }
}

export const syncPage = new SyncPageStore();
