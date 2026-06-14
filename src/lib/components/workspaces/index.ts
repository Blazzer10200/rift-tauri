import type { Component } from "svelte";
import type { WorkspaceId } from "$lib/state/workspace.svelte";
import {
  Home as HomeIcon, MessageSquare,
  Activity, Settings as SettingsIcon, Cpu,
} from "lucide-svelte";
import HomePage from "../home/HomePage.svelte";
import AssistantPage from "../assistant/AssistantPage.svelte";
import SettingsPage from "../settings/SettingsPage.svelte";
import LocalLlmPage from "../local-llm/LocalLlmPage.svelte";

// lucide-svelte 1.x ships icons typed as legacy components; `typeof Activity`
// matches what each icon export looks like and stays compatible w/ Svelte 5
// runes-mode without the explicit `Component<...>` constraint that rejects
// them. Same pattern right-pane/index.ts used (PanelIcon).
export type WorkspaceIcon = typeof Activity;

// Registry components have heterogeneous prop shapes; widening to Component
// lets WorkspaceShell pick the right shape per-entry.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WorkspaceComponent = Component<any, any, string>;

export type WorkspaceDef = {
  component: WorkspaceComponent;
  title: string;
  icon: WorkspaceIcon;
  kbd: string;
  disabled?: boolean;
  getCount?: () => number;
  getTone?: "warn" | "danger" | "info";
};

export const WORKSPACES: Record<WorkspaceId, WorkspaceDef> = {
  home:        { component: HomePage,          title: "Home",        icon: HomeIcon,      kbd: "1" },
  chat:        { component: AssistantPage,     title: "Chat",        icon: MessageSquare, kbd: "2" },
  settings:    { component: SettingsPage,      title: "Settings",    icon: SettingsIcon,  kbd: "3" },
  "local-llm": { component: LocalLlmPage,       title: "Local LLM",   icon: Cpu,           kbd: "4" },
};
