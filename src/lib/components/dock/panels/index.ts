import { ListTodo, RefreshCcw, FolderOpen, History, Bot, Terminal, Paperclip, Activity } from "lucide-svelte";
import type { PanelId } from "$lib/state/panel-types";

import TasksStub from "./TasksStub.svelte";
import SyncPanel from "./SyncPanel.svelte";
import FilesStub from "./FilesStub.svelte";
import HistoryStub from "./HistoryStub.svelte";
import AgentsStub from "./AgentsStub.svelte";
import TerminalStub from "./TerminalStub.svelte";
import AttachmentsStub from "./AttachmentsStub.svelte";
import ActivityPanel from "./ActivityPanel.svelte";

// Shape matches lucide-svelte's component export — typed via `typeof Activity`
// so the registry stays compatible w/o explicit `Component<...>` constraint
// (lucide-svelte 0.x still ships legacy-style components; Svelte 5 runes-mode
// `Component<...>` typing rejects them). Same pattern as TabRail.svelte:15.
export type PanelIcon = typeof Activity;

export type PanelDef = {
  component: typeof TasksStub;
  title: string;
  icon: PanelIcon;
  kbd: string;
  defaultHeight?: number;
};

// Registry of every v1 panel. Phase B swaps stub components for real wrapped
// surfaces; this file is the only edit needed. PANEL_IDS + PRESETS live in
// panel-types.ts (single source of truth shared with ui-prefs).
export const PANELS: Record<PanelId, PanelDef> = {
  tasks:       { component: TasksStub,       title: "Tasks",       icon: ListTodo,   kbd: "1" },
  sync:        { component: SyncPanel,       title: "Sync",        icon: RefreshCcw, kbd: "2" },
  files:       { component: FilesStub,       title: "Files",       icon: FolderOpen, kbd: "3" },
  history:     { component: HistoryStub,     title: "History",     icon: History,    kbd: "4" },
  agents:      { component: AgentsStub,      title: "Agents",      icon: Bot,        kbd: "5" },
  terminal:    { component: TerminalStub,    title: "Terminal",    icon: Terminal,   kbd: "6" },
  attachments: { component: AttachmentsStub, title: "Attachments", icon: Paperclip,  kbd: "7" },
  activity:    { component: ActivityPanel,   title: "Activity",    icon: Activity,   kbd: "8" },
};
