// Phase 3b: Settings became a workspace, but its dialog callbacks
// (Add server / Edit server / Delete server / Keygen) are owned by AppShell.
// This tiny store lets the workspace component pull AppShell's handlers
// without prop-drilling through the WorkspaceShell registry. AppShell
// assigns these on mount; SettingsPage reads them on render.

import type { ServerProfile } from "./connection.svelte";

class DialogsState {
  onAddServer: () => void = () => {};
  onEditServer: (s: ServerProfile) => void = () => {};
  onDeleteServer: (s: ServerProfile) => void = () => {};
  onLaunchKeygen: () => void = () => {};
}

export const dialogs = new DialogsState();
