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
import { notify } from "../toast.svelte";
import { prettyPath } from "../../components/shell/tabsbar/helpers";

/** Shape of the bits of AssistantStore the workspace fns mutate. Structural
 *  type (no class import) — matches AssistantStore's declared $state fields. */
type WorkspaceHost = {
  workspace: WorkspaceState;
  workspaceFiles: string[];
  workspaceFilesLoadingFor: string | null;
  workspaceBranch: string | null;
  localScratchPath: string | null;
  lastError: string | null;
  applyWorkspacePrefs: () => void;
  // Per-tab root surface (split-pane): the @-mention walk + branch probe scope
  // to the focused tab's effective root, and the per-pane picker writes the
  // chosen folder onto the tab rather than the global default.
  activeRoot: string | null;
  tabFor: (id: string | null) => { workspaceRoot: string | null } | null;
};

export async function refreshWorkspace(host: WorkspaceHost): Promise<void> {
  try {
    host.workspace = await invoke<WorkspaceState>("assistant_get_workspace");
    host.applyWorkspacePrefs();
  } catch (e) {
    console.warn("assistant_get_workspace failed", e);
  }
  // Resolve the persistent scratch path once for the "Local" badge. Best-effort:
  // null leaves no-folder turns in the legacy no-tools mode (badge just hides).
  if (host.localScratchPath === null) {
    try {
      host.localScratchPath = await invoke<string | null>("assistant_local_scratch_path");
    } catch (e) {
      console.warn("assistant_local_scratch_path failed", e);
    }
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

/** Per-pane folder picker → set THIS tab's root without touching the global
 *  default (so the choice can't leak into other panes). Returns false if
 *  cancelled. */
export async function pickTabFolder(host: WorkspaceHost, tabId: string | null): Promise<boolean> {
  try {
    const result = await openDialog({ directory: true, multiple: false });
    const path = typeof result === "string" ? result : null;
    if (!path) return false;
    await setTabRoot(host, tabId, path);
    return true;
  } catch (e) {
    host.lastError = `Open folder failed: ${String(e)}`;
    return false;
  }
}

/** Set a single tab's project folder. Canonicalizes + records the recent MRU
 *  backend-side (no global `current_root` mutation), stores the canonical path
 *  on the tab, then refreshes the recents list + the focused-pane file/branch
 *  caches. */
export async function setTabRoot(host: WorkspaceHost, tabId: string | null, path: string): Promise<void> {
  const tab = host.tabFor(tabId);
  if (!tab) return;
  try {
    const canonical = await invoke<string>("assistant_set_tab_root", { path });
    tab.workspaceRoot = canonical;
    // Pull the updated recent-roots MRU into the global workspace state so the
    // picker still offers it; current_root is intentionally left unchanged.
    await refreshWorkspace(host);
    host.workspaceFiles = [];
    host.workspaceBranch = null;
    notify.info("Pane folder set", { detail: prettyPath(canonical), mono: true });
  } catch (e) {
    const msg = String(e);
    if (/not a directory/i.test(msg) && host.workspace.recent.includes(path)) {
      await removeRecentRoot(host, path);
      notify.warn("Folder no longer exists", { detail: `Removed from recents: ${prettyPath(path)}`, mono: true });
    } else {
      host.lastError = `Set pane folder failed: ${msg}`;
    }
  }
}

export async function setRoot(host: WorkspaceHost, path: string): Promise<void> {
  try {
    host.workspace = await invoke<WorkspaceState>("assistant_set_root", { path });
    host.workspaceFiles = [];
    host.workspaceBranch = null;
    host.applyWorkspacePrefs();
    notify.info("Workspace changed", { detail: prettyPath(path), mono: true });
  } catch (e) {
    const msg = String(e);
    // #35: a recent folder that was moved/deleted fails here ("not a directory").
    // Self-heal — drop the dead entry so the MRU stops offering a path that 404s.
    if (/not a directory/i.test(msg) && host.workspace.recent.includes(path)) {
      await removeRecentRoot(host, path);
      notify.warn("Folder no longer exists", { detail: `Removed from recents: ${prettyPath(path)}`, mono: true });
    } else {
      host.lastError = `Set workspace failed: ${msg}`;
    }
  }
}

export async function clearRoot(host: WorkspaceHost): Promise<void> {
  try {
    host.workspace = await invoke<WorkspaceState>("assistant_clear_root");
    host.workspaceFiles = [];
    host.workspaceBranch = null;
    host.applyWorkspacePrefs();
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
  const root = host.activeRoot;
  if (!root) { host.workspaceFiles = []; return; }
  if (host.workspaceFilesLoadingFor === root) return;
  host.workspaceFilesLoadingFor = root;
  try {
    const files = await invoke<string[]>("assistant_list_workspace_files", { root });
    if (host.activeRoot === root) host.workspaceFiles = files;
  } catch (e) {
    console.warn("assistant_list_workspace_files failed", e);
  } finally {
    // Only release the guard if it's still ours — a root-switch mid-await may
    // have armed a newer load; clearing unconditionally would mask it.
    if (host.workspaceFilesLoadingFor === root) host.workspaceFilesLoadingFor = null;
  }
}

/** Lazy-load the active workspace's git branch (or null if not a git repo).
 *  Cheap (`git rev-parse`); surfaced in the Welcome context strip. */
export async function loadWorkspaceBranch(host: WorkspaceHost): Promise<void> {
  const root = host.activeRoot;
  if (!root) { host.workspaceBranch = null; return; }
  try {
    const branch = await invoke<string | null>("assistant_workspace_branch", { root });
    // #190: discard a stale result if the root changed during the await.
    if (host.activeRoot === root) host.workspaceBranch = branch;
  } catch (e) {
    // #191: surface the failure instead of silently blanking the branch.
    if (host.activeRoot === root) {
      host.workspaceBranch = null;
      host.lastError = `Branch read failed: ${String(e)}`;
    }
  }
}
