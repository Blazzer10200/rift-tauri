export type PanelId =
  | "tasks"
  | "sync"
  | "files"
  | "history"
  | "agents"
  | "terminal"
  | "attachments"
  | "activity";

export type DockSlot = "left" | "right";

export type PanelState = {
  open: boolean;
  collapsed: boolean;
  order: number;
  height: number | null;
  /** v0.4 — which side of the split dock the panel lives in. Older
   *  localStorage records without this field migrate to "left" so existing
   *  users see no visible change on upgrade. */
  slot: DockSlot;
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
