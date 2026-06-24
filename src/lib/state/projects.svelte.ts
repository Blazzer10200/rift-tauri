// Project registry — reactive cache over the backend's project store
// (config.json via assistant_list_projects / _save_project / _delete_project).
// A project is a named alias over a workspace folder plus per-project
// file-pattern config (include/exclude globs). Switching to a project sets the
// active workspace root (the existing global folder mechanism) so the rest of
// the app — turns, MCP scoping, conversation filtering — needs no new plumbing.

import { invoke } from "@tauri-apps/api/core";
import type { Project } from "./assistant/types";

/** localStorage key for the id of the last-active project, so a relaunch can
 *  highlight it in the sidebar. The active ROOT itself is owned by the backend
 *  workspace state; this is purely the UI's "which project chip is lit". */
const ACTIVE_PROJECT_KEY = "rift.projects.activeId.v1";

/** Canonicalize a root for comparison (trailing-slash + case-insensitive, to
 *  match Windows paths and the ConversationList rootKey() convention). */
export function projectRootKey(r: string | null | undefined): string {
  return (r ?? "").replace(/[\\/]+$/, "").toLowerCase();
}

class ProjectRegistry {
  items = $state<Project[]>([]);
  /** Id of the project whose root is currently active, or null. Derived against
   *  the live active root rather than stored blindly, so it stays correct even
   *  if the root was changed by the plain folder picker. */
  activeId = $state<string | null>(null);
  loaded = $state(false);
  lastError = $state<string | null>(null);

  init() {
    if (typeof window === "undefined") return;
    this.activeId = localStorage.getItem(ACTIVE_PROJECT_KEY);
  }

  /** Sorted view for display — newest first, falling back to name. */
  get sorted(): Project[] {
    return [...this.items].sort(
      (a, b) => b.createdAt - a.createdAt || a.name.localeCompare(b.name),
    );
  }

  byId(id: string | null | undefined): Project | null {
    if (!id) return null;
    return this.items.find((p) => p.id === id) ?? null;
  }

  /** The project (if any) whose root matches the given active root. */
  byRoot(root: string | null | undefined): Project | null {
    const k = projectRootKey(root);
    if (!k) return null;
    return this.items.find((p) => projectRootKey(p.root) === k) ?? null;
  }

  async refresh(): Promise<void> {
    try {
      this.items = await invoke<Project[]>("assistant_list_projects");
      this.loaded = true;
    } catch (e) {
      this.lastError = `Load projects failed: ${String(e)}`;
      console.warn("assistant_list_projects failed", e);
    }
  }

  /** Create or update a project. Returns the saved project's id, or null on
   *  failure (error surfaced via lastError). */
  async save(p: {
    id?: string;
    name: string;
    root: string;
    include: string[];
    exclude: string[];
    createdAt?: number;
  }): Promise<string | null> {
    const id = p.id ?? crypto.randomUUID();
    const createdAt = p.createdAt ?? Date.now();
    try {
      this.items = await invoke<Project[]>("assistant_save_project", {
        id,
        name: p.name,
        root: p.root,
        include: p.include,
        exclude: p.exclude,
        createdAt,
      });
      this.lastError = null;
      return id;
    } catch (e) {
      this.lastError = `Save project failed: ${String(e)}`;
      return null;
    }
  }

  async remove(id: string): Promise<void> {
    try {
      this.items = await invoke<Project[]>("assistant_delete_project", { id });
      if (this.activeId === id) this.setActiveId(null);
    } catch (e) {
      this.lastError = `Delete project failed: ${String(e)}`;
    }
  }

  setActiveId(id: string | null) {
    this.activeId = id;
    if (typeof window === "undefined") return;
    if (id) localStorage.setItem(ACTIVE_PROJECT_KEY, id);
    else localStorage.removeItem(ACTIVE_PROJECT_KEY);
  }
}

export const projects = new ProjectRegistry();
