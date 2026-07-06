// Project registry — reactive cache over the backend's project store
// (config.json via assistant_list_projects / _save_project / _delete_project).
// A project is a named alias over a workspace folder plus per-project
// file-pattern config (include/exclude globs). Switching to a project sets the
// active workspace root (the existing global folder mechanism) so the rest of
// the app — turns, MCP scoping, conversation filtering — needs no new plumbing.

import { invoke } from "@tauri-apps/api/core";
import type { Project } from "./assistant/types";
import { rootKey } from "$lib/utils/path";

/** Canonicalize a root for comparison — the canonical `rootKey` (utils/path.ts),
 *  re-exported under the existing name so this module's callers stay unchanged. */
export const projectRootKey = rootKey;

// #81: the old stored `activeId` (+ its localStorage key "rift.projects.
// activeId.v1") was write-only state — every surface highlights the active
// project by ROOT match (`byRoot`/`projectRootKey` against the live root), so
// the stored id silently diverged (e.g. "open in split" set it to a project
// whose root never became global). Dropped; byRoot IS the active-project truth.

class ProjectRegistry {
  items = $state<Project[]>([]);
  loaded = $state(false);
  lastError = $state<string | null>(null);
  /** Transient one-shot: set by an "+ New project" affordance OUTSIDE the
   *  Workspace page (e.g. the sidebar ProjectSwitcher) to ask WorkspacePage to open
   *  its new-project editor on mount. WorkspacePage reads + clears it, so it
   *  fires exactly once and never re-opens on a later visit. */
  newProjectIntent = $state(false);
  requestNewProject() { this.newProjectIntent = true; }
  consumeNewProjectIntent(): boolean {
    if (!this.newProjectIntent) return false;
    this.newProjectIntent = false;
    return true;
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
      this.lastError = null;
    } catch (e) {
      this.lastError = String(e);
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
      this.lastError = String(e);
      return null;
    }
  }

  async remove(id: string): Promise<void> {
    try {
      this.items = await invoke<Project[]>("assistant_delete_project", { id });
      this.lastError = null;
    } catch (e) {
      this.lastError = String(e);
    }
  }
}

export const projects = new ProjectRegistry();
