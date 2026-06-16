// Git working-tree state for the Environment panel. Thin runes-class singleton
// (UsageStore pattern) over the typed `git_*` Tauri commands in
// commands/git.rs. No polling — the panel triggers refresh() on mount, on a
// Refresh click, on workspace change, and after a commit.

import { invoke } from "@tauri-apps/api/core";

export type GitFile = {
  path: string;
  staged: string;   // porcelain X column (one char)
  unstaged: string; // porcelain Y column
  adds: number;
  dels: number;
};

export type GitStatus = {
  branch: string;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: GitFile[];
  total_adds: number;
  total_dels: number;
  remote: string | null;
  root: string;
};

class GitStore {
  status = $state<GitStatus | null>(null);
  error = $state<string | null>(null);
  loading = $state(false);
  committing = $state(false);
  commitError = $state<string | null>(null);

  async refresh(root: string | null): Promise<void> {
    if (!root) {
      this.status = null;
      this.error = "No workspace folder open.";
      return;
    }
    this.loading = true;
    this.error = null;
    try {
      this.status = await invoke<GitStatus>("git_working_status", { root });
    } catch (e) {
      this.status = null;
      this.error = String(e);
    } finally {
      this.loading = false;
    }
  }

  async fileDiff(root: string | null, path: string, cached: boolean): Promise<string> {
    if (!root) throw new Error("No workspace folder open.");
    return invoke<string>("git_file_diff", { root, path, cached });
  }

  async commit(root: string | null, message: string, push: boolean): Promise<boolean> {
    if (!root) {
      this.commitError = "No workspace folder open.";
      return false;
    }
    this.committing = true;
    this.commitError = null;
    try {
      await invoke<string>("git_commit_and_push", { root, message, push });
      await this.refresh(root);
      return true;
    } catch (e) {
      this.commitError = String(e);
      return false;
    } finally {
      this.committing = false;
    }
  }
}

export const git = new GitStore();
