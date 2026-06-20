// "Home is a verb" (redesign §6): there is no standalone home page — the home
// surface IS an empty Chat tab. goHome() floats to Chat and only mints a new
// tab when the active one already holds a conversation, so repeated calls stay
// put (idempotent).
import { workspace } from "./workspace.svelte";
import { assistant } from "./assistant.svelte";

export function goHome() {
  workspace.setActive("chat");
  const tab = assistant.activeTab;
  if (!tab || tab.messages.length > 0) void assistant.newTab();
}
