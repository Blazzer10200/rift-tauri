import type { Component } from "svelte";
import type { WorkspaceId } from "$lib/state/workspace.svelte";
import {
  MessageSquare, FolderTree,
  Activity, Settings as SettingsIcon, HeartPulse,
} from "@lucide/svelte";

// @lucide/svelte 1.x ships icons typed as legacy components; `typeof Activity`
// matches what each icon export looks like and stays compatible w/ Svelte 5
// runes-mode without the explicit `Component<...>` constraint that rejects
// them. Same pattern right-pane/index.ts used (PanelIcon).
type WorkspaceIcon = typeof Activity;

// Registry components have heterogeneous prop shapes; widening to Component
// lets WorkspaceShell pick the right shape per-entry.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WorkspaceComponent = Component<any, any, string>;

type WorkspaceModule = { default: WorkspaceComponent };

type WorkspaceDef = {
  load: () => Promise<WorkspaceModule>;
  title: string;
  icon: WorkspaceIcon;
  kbd: string;
  disabled?: boolean;
};

export const WORKSPACES: Record<WorkspaceId, WorkspaceDef> = {
  // Workspace page: merged Home + Projects destination. Mounted normally via workspace.setActive("home").
  home:        { load: () => import("../workspace/WorkspacePage.svelte"),     title: "Workspace",   icon: FolderTree,    kbd: "1" },
  chat:        { load: () => import("../assistant/AssistantPage.svelte"),     title: "Chat",        icon: MessageSquare, kbd: "2" },
  // Legacy "projects" id: aliases WorkspacePage to keep the Record exhaustive + handle persisted activeId (init() folds it → "home").
  projects:    { load: () => import("../workspace/WorkspacePage.svelte"),     title: "Workspace",   icon: FolderTree,    kbd: "3" },
  settings:    { load: () => import("../settings/SettingsPage.svelte"),      title: "Settings",    icon: SettingsIcon,  kbd: "4" },
  "ai-health": { load: () => import("../ai-health/AiHealthPage.svelte"),       title: "AI Health",   icon: HeartPulse,    kbd: "6" },
  diagnostics: { load: () => import("../diagnostics/DiagnosticsPage.svelte"), title: "Diagnostics", icon: Activity,      kbd: "5" },
};
