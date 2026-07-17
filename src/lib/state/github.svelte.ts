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
  /** Monotonic fetch id: a newer refresh supersedes any in-flight result. */
  #epoch = 0;

  async refresh(root: string | null, opts: { force?: boolean } = {}): Promise<void> {
    if (!root) {
      this.status = null;
      this.loadedFor = null;
      return;
    }
    const now = Date.now();
    const sameRoot = this.loadedFor === root;
    if (!opts.force && sameRoot && now - this.#lastFetched < MIN_GAP_MS) return;
    // Redundant same-root call while one is already in flight → drop. A
    // DIFFERENT root or a forced call supersedes the in-flight fetch instead:
    // the old blanket `if (loading) return` silently dropped root switches and
    // popover force-refreshes, leaving the previous repo's status on screen.
    if (this.loading && sameRoot && !opts.force) return;
    const epoch = ++this.#epoch;
    this.loading = true;
    // Root switch: drop the old repo's snapshot instead of flashing it.
    if (!sameRoot) this.status = null;
    try {
      const s = await invoke<GhStatus>("gh_branch_status", { root });
      if (epoch !== this.#epoch) return; // superseded — a newer refresh owns the state
      this.status = s;
    } catch (e) {
      if (epoch !== this.#epoch) return;
      // Keep branch/repo from the prior same-root snapshot so the popover
      // header doesn't blank to "…" on a transient failure.
      const prev = sameRoot ? this.status : null;
      this.status = {
        state: "error",
        branch: prev?.branch,
        repo: prev?.repo,
        url: prev?.url,
        detail: String(e),
      };
    } finally {
      if (epoch === this.#epoch) {
        this.loadedFor = root;
        this.#lastFetched = Date.now();
        this.loading = false;
      }
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
