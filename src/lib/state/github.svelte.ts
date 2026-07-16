// GitHub branch-chip status store. One snapshot per workspace root, fetched
// via `gh_branch_status` (aggregated backend call riding the user's own `gh`
// CLI — Rift stores no token). Refresh policy: lazy on chip mount, focus-
// regain with a 60s min-gap, forced after a remote-mutating tool
// (git_push / gh_pr_create) finishes a turn, and on popover open.

import { invoke } from "@tauri-apps/api/core";
import { ghDot, type GhStatus } from "./githubHelpers";

const MIN_GAP_MS = 60_000;

class GithubState {
  status = $state<GhStatus | null>(null);
  /** Root the current `status` belongs to (staleness + re-key guard). */
  loadedFor = $state<string | null>(null);
  loading = $state(false);
  dot = $derived(ghDot(this.status));

  #lastFetched = 0;
  /** Set when a turn used git_push / gh_pr_create — flushed at turn end. */
  #remoteMutated = false;

  async refresh(root: string | null, opts: { force?: boolean } = {}): Promise<void> {
    if (!root) {
      this.status = null;
      this.loadedFor = null;
      return;
    }
    const now = Date.now();
    if (!opts.force && this.loadedFor === root && now - this.#lastFetched < MIN_GAP_MS) return;
    if (this.loading) return;
    this.loading = true;
    // Root switch: drop the old repo's snapshot instead of flashing it.
    if (this.loadedFor !== root) this.status = null;
    try {
      const s = await invoke<GhStatus>("gh_branch_status", { root });
      this.status = s;
    } catch (e) {
      this.status = { state: "error", detail: String(e) };
    } finally {
      this.loadedFor = root;
      this.#lastFetched = Date.now();
      this.loading = false;
    }
  }

  /** Cheap call sites (chip mount, focus regain) — min-gap applies. */
  maybeRefresh(root: string | null): void {
    void this.refresh(root);
  }

  /** streaming.ts: a remote-mutating tool ran this turn. */
  noteRemoteMutation(): void {
    this.#remoteMutated = true;
  }

  /** streaming.ts turn end: force-refresh once if anything mutated the remote. */
  flushRemoteMutation(): void {
    if (!this.#remoteMutated) return;
    this.#remoteMutated = false;
    if (this.loadedFor) void this.refresh(this.loadedFor, { force: true });
  }
}

export const github = new GithubState();
