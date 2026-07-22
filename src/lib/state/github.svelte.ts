// GitHub branch-chip status store. One snapshot per workspace root, fetched
// via `gh_branch_status` (aggregated backend call riding the user's own `gh`
// CLI — Rift stores no token). Refresh policy: lazy on chip mount, focus-
// regain with a 60s min-gap, forced after a remote-mutating tool
// (git_push / gh_pr_create) finishes a turn, and on popover open.

import { invoke } from "@tauri-apps/api/core";
import { ghDot, type GhStatus } from "./githubHelpers";
import { notify } from "./toast.svelte";

const MIN_GAP_MS = 60_000;
/** While the latest run is live (busy dot), poll so "in progress" can't go
 *  stale on screen — the min-gap only guards the passive triggers. */
const BUSY_POLL_MS = 30_000;

class GithubState {
  status = $state<GhStatus | null>(null);
  /** Root the current `status` belongs to (staleness + re-key guard). */
  loadedFor = $state<string | null>(null);
  loading = $state(false);
  /** When the current snapshot landed (ms) — drives the popover freshness hint. */
  fetchedAt = $state(0);
  dot = $derived(ghDot(this.status));

  #lastFetched = 0;
  /** Set when a turn used git_push / gh_pr_create — flushed at turn end. */
  #remoteMutated = false;
  /** Monotonic fetch id: a newer refresh supersedes any in-flight result. */
  #epoch = 0;
  #pollTimer: ReturnType<typeof setTimeout> | null = null;

  async refresh(root: string | null, opts: { force?: boolean } = {}): Promise<void> {
    if (!root) {
      this.status = null;
      this.loadedFor = null;
      this.#clearPoll();
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
    const prevSnap = sameRoot ? this.status : null;
    try {
      const s = await invoke<GhStatus>("gh_branch_status", { root });
      if (epoch !== this.#epoch) return; // superseded — a newer refresh owns the state
      this.#noteRunTransition(prevSnap, s);
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
        this.fetchedAt = this.#lastFetched;
        this.loading = false;
        this.#schedulePoll();
      }
    }
  }

  #clearPoll(): void {
    if (this.#pollTimer) {
      clearTimeout(this.#pollTimer);
      this.#pollTimer = null;
    }
  }

  /** Re-armed after every settled refresh; only ticks while the dot is busy. */
  #schedulePoll(): void {
    this.#clearPoll();
    if (this.dot !== "busy") return;
    const root = this.loadedFor;
    if (!root) return;
    this.#pollTimer = setTimeout(() => {
      this.#pollTimer = null;
      // Hidden window: skip the fetch, stay armed — the focus-regain refresh
      // catches up the moment the app is visible again.
      if (typeof document !== "undefined" && document.hidden) {
        this.#schedulePoll();
        return;
      }
      void this.refresh(root, { force: true });
    }, BUSY_POLL_MS);
  }

  /** The run we were watching just finished → say so (chip-adjacent toast). */
  #noteRunTransition(prev: GhStatus | null, next: GhStatus): void {
    const prevRun = prev?.state === "ok" ? prev.run : null;
    const nextRun = next.state === "ok" ? next.run : null;
    if (!prevRun || !nextRun || prevRun.databaseId !== nextRun.databaseId) return;
    if (ghDot(prev) !== "busy" || nextRun.status !== "completed") return;
    const name = nextRun.workflowName ?? "CI";
    const dot = ghDot(next);
    if (dot === "ok") {
      notify.ok(`${name} passed`, { detail: next.branch });
    } else if (dot === "err") {
      notify.danger(`${name} failed`, {
        detail: nextRun.failedJob ? `Failed in ${nextRun.failedJob}` : next.branch,
      });
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
