export type PanelId =
  | "tasks"
  | "sync"
  | "files"
  | "history"
  | "agents"
  | "terminal"
  | "attachments"
  | "activity";

export type PanelState = {
  open: boolean;
  collapsed: boolean;
  order: number;
  height: number | null;
};

export type LayoutPreset = "minimal" | "standard" | "power";

export const PANEL_IDS: readonly PanelId[] = [
  "tasks",
  "sync",
  "files",
  "history",
  "agents",
  "terminal",
  "attachments",
  "activity",
] as const;

export const PRESETS: Record<LayoutPreset, readonly PanelId[]> = {
  minimal: ["tasks", "history"],
  standard: ["tasks", "sync", "files", "agents", "history"],
  power: ["tasks", "sync", "files", "agents", "history", "terminal", "activity", "attachments"],
} as const;
