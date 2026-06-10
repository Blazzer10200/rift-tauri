<script lang="ts">
  import { commandPalette } from "../state/command-palette.svelte";
  import CommandPalette from "./dialogs/CommandPalette.svelte";
  import Titlebar from "./shell/Titlebar.svelte";
  import ToastHost from "./ToastHost.svelte";
  import UpdatePill from "./UpdatePill.svelte";
  import UpdateDialog from "./dialogs/UpdateDialog.svelte";
  import ChatTabsBar from "./shell/ChatTabsBar.svelte";
  import WorkspaceShell from "./shell/WorkspaceShell.svelte";
  import { WORKSPACES } from "./workspaces";
  import { workspace, type WorkspaceId } from "../state/workspace.svelte";
  import { browserDock } from "../state/browserDock.svelte";
  import { updates } from "../state/updates.svelte";
  import { assistant } from "../state/assistant.svelte";
  import { toast } from "../state/toast.svelte";
  import { onboarding } from "../state/onboarding.svelte";
  import { betaNotice } from "../state/betaNotice.svelte";
  import OnboardingFlow from "./onboarding/OnboardingFlow.svelte";

  // First-run gate: show the onboarding flow until it's been completed/skipped
  // once. Triggers when the assistant has no usable auth (no API key AND the
  // claude CLI isn't logged in) OR the beta notice hasn't been acknowledged —
  // the final onboarding step captures that ack, so every fresh tester sees it
  // before they start, regardless of auth. Gated on configLoaded so it never
  // flashes pre-probe. Dismissible + persisted.
  const showOnboarding = $derived(
    !onboarding.dismissed &&
    assistant.configLoaded &&
    ((!assistant.hasApiKey && !assistant.auth?.loggedIn) || !betaNotice.acknowledged),
  );

  // Finishing or skipping onboarding records both the dismissal and the beta
  // acknowledgment (the notice is the last step) so the gate stays closed.
  function finishOnboarding() {
    onboarding.dismiss();
    betaNotice.acknowledge();
  }

  function gotoSettings() {
    workspace.setActive("settings");
  }

  // ── lifecycle ──────────────────────────────────────────────────────
  $effect(() => {
    // Boot lands on Home, which reads workspace/conversations off the
    // assistant store — init here, not just on Chat/Settings mount, or the
    // dashboard renders the empty-state lie until another workspace runs it.
    void assistant.init();
    browserDock.init();
    void updates.checkOnLaunch();
    // Dev-only: expose the update store so CDP can drive its visual states
    // (toast + dialog) without a live feed. Stripped from prod builds.
    if (import.meta.env.DEV) {
      (window as unknown as { __updates?: typeof updates }).__updates = updates;
      (window as unknown as { __assistant?: typeof assistant }).__assistant = assistant;
    }
  });

  // HMR-safe global keydown — $effect cleanup runs on unmount AND when the
  // effect re-tracks during HMR replacement.
  $effect(() => {
    window.addEventListener("keydown", onGlobalKey);
    return () => window.removeEventListener("keydown", onGlobalKey);
  });

  function onGlobalKey(e: KeyboardEvent) {
    // First-run setup is modal — no palette / workspace-switch / tab shortcuts
    // may fire over (or behind) it. Its own Escape handler still works.
    if (showOnboarding) return;
    // Alt+1..9 → jump to chat tab N (1-indexed). Only fires inside the Chat
    // workspace so it doesn't hijack from a focused surface.
    if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey && /^[1-9]$/.test(e.key)
        && workspace.activeId === "chat") {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const target = assistant.openTabs[idx];
      if (target) assistant.openTab(target).catch(err => toast.push({ severity: "danger", title: String(err) }));
      return;
    }
    const meta = e.ctrlKey || e.metaKey;
    if (!meta) return;
    // Chat-tab keybinds — gated to the Chat workspace so tabs only respond when
    // the user is actually looking at the chat.
    if (!e.altKey && workspace.activeId === "chat") {
      if (!e.shiftKey && e.key.toLowerCase() === "t") {
        e.preventDefault();
        assistant.newTab().catch(err => toast.push({ severity: "danger", title: String(err) }));
        return;
      }
      if (!e.shiftKey && e.key.toLowerCase() === "w") {
        e.preventDefault();
        const cur = assistant.currentConvoId;
        if (cur) assistant.closeTab(cur).catch(err => toast.push({ severity: "danger", title: String(err) }));
        return;
      }
      if (e.key === "Tab") {
        e.preventDefault();
        assistant.cycleTab(e.shiftKey ? -1 : 1).catch(err => toast.push({ severity: "danger", title: String(err) }));
        return;
      }
      // Ctrl+\ → add a pane (right of focused). Ctrl+Shift+\ → close focused pane.
      if (e.key === "\\") {
        e.preventDefault();
        if (e.shiftKey) {
          assistant.closePane(assistant.focusedPaneIdx);
        } else {
          assistant.addPane();
        }
        return;
      }
    }
    const k = e.key.toLowerCase();
    // View toggles, mirrored in the ChatTabsBar View dropdown.
    if (e.shiftKey && k === "b") { e.preventDefault(); browserDock.toggle(); return; }
    if (e.shiftKey && k === "e") { e.preventDefault(); assistant.ui.dockOpen = !assistant.ui.dockOpen; return; }
    if (e.shiftKey && k === "d" && workspace.activeId === "chat") {
      e.preventDefault();
      if (assistant.ui.diffOpen) { assistant.ui.diffOpen = false; }
      else { assistant.ui.diffTarget = null; assistant.ui.diffOpen = true; }
      return;
    }
    // Ctrl+0 → return to Chat workspace.
    if (!e.shiftKey && e.key === "0") {
      e.preventDefault();
      workspace.setActive("chat");
      return;
    }
    // Ctrl+1..9 → workspace switch, mapped by activity-bar order.
    if (/^[1-9]$/.test(e.key)) {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const id = workspace.order[idx] as WorkspaceId | undefined;
      if (id && !WORKSPACES[id].disabled) workspace.setActive(id);
      return;
    }
    if (e.shiftKey) return;
    // Ctrl+L → focus the browser dock's address bar (open it if closed).
    if (k === "l" && workspace.activeId === "chat") { e.preventDefault(); browserDock.focusAddress(); return; }
    if (e.key === ",") { e.preventDefault(); gotoSettings(); return; }
    // Ctrl+P / Ctrl+K → global command palette.
    if (k === "p" || k === "k") { e.preventDefault(); commandPalette.show(); return; }
  }
</script>

<div class="shell">
  <Titlebar setupMode={showOnboarding} />

  <div class="middle">
    {#if showOnboarding}
      <OnboardingFlow onDone={finishOnboarding} />
    {:else}
      <div class="body">
        <div class="content">
          <div class="tabs-rail" data-show={workspace.activeId === "chat"} aria-hidden={workspace.activeId !== "chat"}>
            <ChatTabsBar />
          </div>
          <main class="pane">
            <WorkspaceShell />
          </main>
        </div>
      </div>
    {/if}
  </div>

  <UpdateDialog />

  <UpdatePill />

  <ToastHost />

  <CommandPalette />

</div>

<style>
  /* S136: pin .shell to the viewport via position:fixed instead of a
     percentage height chain — the chain intermittently collapses in prod. */
  .shell {
    position: fixed;
    inset: 0;
    display: grid;
    grid-template-rows: var(--titlebar-h) 1fr;
    min-width: 0;
    overflow: hidden;
    background: var(--bg);
    color: var(--fg);
  }
  .middle {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow: visible;
    position: relative;
  }
  /* Chat tabs rail collapses to 0 height when non-chat workspace is active.
     Always mounted so tab state survives workspace hops without remount jitter. */
  .tabs-rail {
    overflow: hidden;
    max-height: 0;
    opacity: 0;
    transform: translateY(-4px);
    transition:
      max-height 180ms cubic-bezier(.2,.7,.2,1),
      opacity 140ms ease,
      transform 180ms cubic-bezier(.2,.7,.2,1);
  }
  .tabs-rail[data-show="true"] {
    max-height: 48px;
    opacity: 1;
    transform: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .tabs-rail { transition: none; transform: none; }
  }
  .body {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
    min-width: 0;
    overflow: visible;
    position: relative;
  }
  .content {
    display: flex;
    flex-direction: column;
    min-height: 0; min-width: 0;
    overflow: visible;
    position: relative;
  }
  .pane {
    flex: 1 1 0;
    min-height: 0; min-width: 0;
    overflow: hidden;
    display: flex; flex-direction: column;
  }
</style>
