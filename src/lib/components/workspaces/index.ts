import type { Component } from "svelte";
import type { WorkspaceId } from "$lib/state/workspace.svelte";
import {
  MessageSquare, RefreshCcw, FolderOpen, AlertTriangle, Stethoscope,
  Terminal, Activity, Settings as SettingsIcon,
} from "lucide-svelte";
import AssistantPage from "../assistant/AssistantPage.svelte";
import SyncPage from "../sync/SyncPage.svelte";
import TwoPane from "../browser/TwoPane.svelte";
import ConflictsPage from "../conflicts/ConflictsPage.svelte";
import Diagnostics from "../diagnostics/Diagnostics.svelte";
import TerminalPanel from "../terminal/TerminalPanel.svelte";
import ActivityFeed from "../activity/ActivityFeed.svelte";
import SettingsPage from "../settings/SettingsPage.svelte";
import { connection } from "$lib/state/connection.svelte";

// lucide-svelte 1.x ships icons typed as legacy components; `typeof Activity`
// matches what each icon export looks like and stays compatible w/ Svelte 5
// runes-mode without the explicit `Component<...>` constraint that rejects
// them. Same pattern right-pane/index.ts used (PanelIcon).
export type WorkspaceIcon = typeof Activity;

// Components in the registry have heterogeneous prop shapes — most take no
// props, DisabledWorkspace takes { title, icon }. Widening to Component lets
// WorkspaceShell pick the right shape per-entry. The shell already branches
// on `disabled` to decide which props to pass.
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
  chat:        { component: AssistantPage,     title: "Chat",        icon: MessageSquare, kbd: "1" },
  sync:        { component: SyncPage,          title: "Sync",        icon: RefreshCcw,    kbd: "2" },
  files:       { component: TwoPane,           title: "Files",       icon: FolderOpen,    kbd: "3" },
  conflicts:   { component: ConflictsPage,     title: "Conflicts",   icon: AlertTriangle, kbd: "4",
                 getCount: () => connection.conflictCount, getTone: "danger" },
  diagnostics: { component: Diagnostics,       title: "Diagnostics", icon: Stethoscope,   kbd: "5" },
  terminal:    { component: TerminalPanel,     title: "Terminal",    icon: Terminal,      kbd: "6" },
  activity:    { component: ActivityFeed,      title: "Activity",    icon: Activity,      kbd: "7" },
  settings:    { component: SettingsPage,      title: "Settings",    icon: SettingsIcon,  kbd: "8" },
};
