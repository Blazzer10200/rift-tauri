import type { Component } from "svelte";
import type { WorkspaceId } from "$lib/state/workspace.svelte";
import {
  MessageSquare, FolderTree,
  Activity, Settings as SettingsIcon, HeartPulse,
} from "lucide-svelte";
import AssistantPage from "../assistant/AssistantPage.svelte";
import WorkspacePage from "../workspace/WorkspacePage.svelte";
import SettingsPage from "../settings/SettingsPage.svelte";
import AiHealthPage from "../ai-health/AiHealthPage.svelte";

// lucide-svelte 1.x ships icons typed as legacy components; `typeof Activity`
// matches what each icon export looks like and stays compatible w/ Svelte 5
// runes-mode without the explicit `Component<...>` constraint that rejects
// them. Same pattern right-pane/index.ts used (PanelIcon).
type WorkspaceIcon = typeof Activity;

// Registry components have heterogeneous prop shapes; widening to Component
// lets WorkspaceShell pick the right shape per-entry.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type WorkspaceComponent = Component<any, any, string>;

type WorkspaceDef = {
  component: WorkspaceComponent;
  title: string;
  icon: WorkspaceIcon;
  kbd: string;
  disabled?: boolean;
};

export const WORKSPACES: Record<WorkspaceId, WorkspaceDef> = {
  // Workspace page: merged Home + Projects destination. Mounted normally via workspace.setActive("home").
  home:        { component: WorkspacePage,     title: "Workspace",   icon: FolderTree,    kbd: "1" },
  chat:        { component: AssistantPage,     title: "Chat",        icon: MessageSquare, kbd: "2" },
  // Legacy "projects" id: aliases WorkspacePage to keep the Record exhaustive + handle persisted activeId (init() folds it → "home").
  projects:    { component: WorkspacePage,     title: "Workspace",   icon: FolderTree,    kbd: "3" },
  settings:    { component: SettingsPage,      title: "Settings",    icon: SettingsIcon,  kbd: "4" },
  "ai-health": { component: AiHealthPage,       title: "AI Health",   icon: HeartPulse,    kbd: "6" },
};
