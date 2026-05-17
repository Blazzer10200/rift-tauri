export type PanelId =
  | "sync"
  | "files"
  | "history"
  | "agents"
  | "terminal"
  | "attachments"
  | "activity";

export const PANEL_IDS: readonly PanelId[] = [
  "sync",
  "files",
  "history",
  "agents",
  "terminal",
  "attachments",
  "activity",
] as const;
