// Pure helpers for the GitHub branch-chip integration (vitest'd — keep free of
// runes/DOM). The shapes mirror `gh_remote::branch_status_sync` on the backend.

export type GhRunInfo = {
  databaseId?: number;
  workflowName?: string;
  displayTitle?: string;
  status?: string;
  conclusion?: string | null;
  event?: string;
  createdAt?: string;
  url?: string;
  /** Branch (or TAG, for release runs) the run actually ran on. */
  headBranch?: string;
  /** Present on red runs: first failing job + its first failing step. */
  failedJob?: string;
  failedStep?: string;
};

type GhPrInfo = {
  number?: number;
  title?: string;
  isDraft?: boolean;
  reviewDecision?: string | null;
  url?: string;
};

export type GhStatus = {
  state: "ok" | "no_root" | "no_repo" | "not_github" | "no_gh" | "no_auth" | "error";
  branch?: string;
  repo?: string;
  url?: string;
  ahead?: number | null;
  behind?: number | null;
  run?: GhRunInfo | null;
  pr?: GhPrInfo | null;
  detail?: string | null;
};

export type GhDotState = "none" | "idle" | "busy" | "ok" | "err";

/** Chip dot semantics (DESIGN §2: status, never accent):
 *  none = nothing to say (not GitHub / gh unavailable) → plain label ·
 *  idle = GitHub repo, no signal (no runs / cancelled) · busy = run in
 *  progress · ok = latest run passed · err = latest run failed. */
export function ghDot(s: GhStatus | null): GhDotState {
  if (!s || s.state !== "ok") return "none";
  const run = s.run;
  if (!run) return "idle";
  const status = run.status ?? "";
  if (["in_progress", "queued", "requested", "waiting", "pending"].includes(status)) return "busy";
  switch (run.conclusion ?? "") {
    case "success":
      return "ok";
    case "failure":
    case "startup_failure":
    case "timed_out":
      return "err";
    case "action_required":
      return "busy";
    default:
      return "idle"; // cancelled, skipped, neutral, stale
  }
}

/** Ahead/behind → one quiet phrase; null when upstream tracking is unknown. */
export function ghSyncLabel(ahead: number | null | undefined, behind: number | null | undefined): string | null {
  if (typeof ahead !== "number" || typeof behind !== "number") return null;
  if (ahead === 0 && behind === 0) return "In sync with origin";
  const parts: string[] = [];
  if (ahead > 0) parts.push(`${ahead} ahead`);
  if (behind > 0) parts.push(`${behind} behind`);
  return parts.join(" · ");
}

/** ISO timestamp → compact relative age ("3m ago"). Empty on bad input. */
export function ghRelTime(iso: string | undefined, now: number = Date.now()): string {
  if (!iso) return "";
  const t = new Date(iso).getTime();
  if (isNaN(t)) return "";
  const s = Math.max(0, Math.round((now - t) / 1000));
  if (s < 60) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.round(h / 24)}d ago`;
}

/** ISO start → compact elapsed span ("11m", "1h 5m") for live runs. */
export function ghElapsed(iso: string | undefined, now: number = Date.now()): string {
  if (!iso) return "";
  const t = new Date(iso).getTime();
  if (isNaN(t)) return "";
  const s = Math.max(0, Math.round((now - t) / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

/** Last-fetch ms timestamp → "checked just now" / "checked 3m ago"; empty before first fetch. */
export function ghCheckedLabel(fetchedAt: number, now: number = Date.now()): string {
  if (!fetchedAt) return "";
  const s = Math.max(0, Math.round((now - fetchedAt) / 1000));
  if (s < 60) return "checked just now";
  const m = Math.round(s / 60);
  if (m < 60) return `checked ${m}m ago`;
  return `checked ${Math.round(m / 60)}h ago`;
}

/** Composer prompt for "Ask Claude to fix" on a red run. */
export function ghFixPrompt(run: GhRunInfo): string {
  const id = run.databaseId ? ` (run ${run.databaseId})` : "";
  const what = [run.workflowName, run.displayTitle].filter(Boolean).join(" — ");
  const where = run.failedJob
    ? ` The failing job is "${run.failedJob}"${run.failedStep ? ` at step "${run.failedStep}"` : ""}.`
    : "";
  return (
    `The latest GitHub Actions run on this branch failed${id}: ${what || "see gh_checks"}.${where} ` +
    `Use gh_run_view with failed_logs: true to read the failure, find the root cause in the code, and fix it.`
  );
}

/** Composer prompt for "Pull latest" when the branch is behind origin. */
export function ghPullPrompt(behind: number): string {
  const n = behind === 1 ? "1 commit" : `${behind} commits`;
  return (
    `This branch is ${n} behind origin. Pull the latest changes with git_pull, ` +
    `then summarize what came in (git_log the new commits).`
  );
}

/** Composer prompt for "Push commits" when the branch is ahead of origin. */
export function ghPushPrompt(ahead: number): string {
  const n = ahead === 1 ? "1 unpushed commit" : `${ahead} unpushed commits`;
  return (
    `This branch has ${n}. Review them briefly with git_log, then push to origin ` +
    `with git_push and confirm the result.`
  );
}

/** Composer prompt for "Draft a PR" — writes flow through the assistant. */
export function ghPrPrompt(): string {
  return (
    "Draft a pull request for the current branch. Review the branch's changes first (git_log + git_diff " +
    "against the base branch), then propose a title and body and confirm them with me via ask_user " +
    "before calling gh_pr_create."
  );
}
