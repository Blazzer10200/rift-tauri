// M3 (per docs/design/assistant-svelte-split.md) — workspace state IPC ops
// lifted out of `src/lib/state/assistant.svelte.ts` as free fns operating on
// a store ref. Per the M4 precedent (and the "no external import churn"
// invariant): Store fields (`workspace`, `workspaceFiles`, `workspaceFilesLoadingFor`,
// `lastError`, `lastNotice`) STAY declared as $state on AssistantStore so
// `assistant.workspace.current` etc. remain reactive. Only the IPC plumbing
// moves here. Store methods become thin thunks routing to these fns.

import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { WorkspaceState } from "./types";

/** Shape of the bits of AssistantStore the workspace fns mutate. Structural
 *  type (no class import) — matches AssistantStore's declared $state fields. */
type WorkspaceHost = {
  workspace: WorkspaceState;
  workspaceFiles: string[];
  workspaceFilesLoadingFor: string | null;
  workspaceBranch: string | null;
  lastError: string | null;
  lastNotice: string | null;
};

export async function refreshWorkspace(host: WorkspaceHost): Promise<void> {
  try {
    host.workspace = await invoke<WorkspaceState>("assistant_get_workspace");
  } catch (e) {
    console.warn("assistant_get_workspace failed", e);
  }
}

/** Native folder picker → set as active root. Returns false if user cancelled. */
export async function pickFolder(host: WorkspaceHost): Promise<boolean> {
  try {
    const result = await openDialog({ directory: true, multiple: false });
    const path = typeof result === "string" ? result : null;
    if (!path) return false;
    await setRoot(host, path);
    return true;
  } catch (e) {
    host.lastError = `Open folder failed: ${String(e)}`;
    return false;
  }
}

export async function setRoot(host: WorkspaceHost, path: string): Promise<void> {
  try {
    host.workspace = await invoke<WorkspaceState>("assistant_set_root", { path });
    host.workspaceFiles = [];
    host.workspaceBranch = null;
    host.lastNotice = `Workspace: ${path}`;
  } catch (e) {
    host.lastError = `Set workspace failed: ${String(e)}`;
  }
}

export async function clearRoot(host: WorkspaceHost): Promise<void> {
  try {
    host.workspace = await invoke<WorkspaceState>("assistant_clear_root");
    host.workspaceFiles = [];
    host.workspaceBranch = null;
  } catch (e) {
    console.warn("assistant_clear_root failed", e);
  }
}

export async function removeRecentRoot(host: WorkspaceHost, path: string): Promise<void> {
  try {
    host.workspace = await invoke<WorkspaceState>("assistant_remove_recent_root", { path });
  } catch (e) {
    console.warn("assistant_remove_recent_root failed", e);
  }
}

/** Lazy-load relative file paths under the current workspace root. Caches
 *  per-root in `workspaceFiles`; concurrent calls are de-duped via the
 *  `workspaceFilesLoadingFor` guard. */
export async function loadWorkspaceFiles(host: WorkspaceHost): Promise<void> {
  const root = host.workspace.current;
  if (!root) { host.workspaceFiles = []; return; }
  if (host.workspaceFilesLoadingFor === root) return;
  host.workspaceFilesLoadingFor = root;
  try {
    const files = await invoke<string[]>("assistant_list_workspace_files");
    if (host.workspace.current === root) host.workspaceFiles = files;
  } catch (e) {
    console.warn("assistant_list_workspace_files failed", e);
  } finally {
    host.workspaceFilesLoadingFor = null;
  }
}

/** Lazy-load the active workspace's git branch (or null if not a git repo).
 *  Cheap (`git rev-parse`); surfaced in the Welcome context strip. */
export async function loadWorkspaceBranch(host: WorkspaceHost): Promise<void> {
  if (!host.workspace.current) { host.workspaceBranch = null; return; }
  try {
    host.workspaceBranch = await invoke<string | null>("assistant_workspace_branch");
  } catch (e) {
    console.warn("assistant_workspace_branch failed", e);
    host.workspaceBranch = null;
  }
}
