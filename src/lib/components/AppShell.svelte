<script lang="ts">
  import { commandPalette } from "../state/command-palette.svelte";
  import CommandPalette from "./dialogs/CommandPalette.svelte";
  import Titlebar from "./shell/Titlebar.svelte";
  import Sidebar from "./shell/Sidebar.svelte";
  import Topbar from "./shell/Topbar.svelte";
  import ToastHost from "./ToastHost.svelte";
  import UpdatePill from "./UpdatePill.svelte";
  import UpdateDialog from "./dialogs/UpdateDialog.svelte";
  import WorkspaceShell from "./shell/WorkspaceShell.svelte";
  import { shell } from "../state/shell.svelte";
  import { WORKSPACES } from "./workspaces";
  import { workspace, type WorkspaceId } from "../state/workspace.svelte";
  import { browserDock } from "../state/browserDock.svelte";
  import { activityDock } from "../state/activityDock.svelte";
  import { environmentDock } from "../state/environmentDock.svelte";
  import { updates } from "../state/updates.svelte";
  import { assistant } from "../state/assistant.svelte";
  import { uiPrefs } from "../state/ui-prefs.svelte";
  import { toast, notify } from "../state/toast.svelte";
  import { isFileDrag, attachImageFiles, summarizeAttach } from "./assistant/composer/helpers";
  import { goHome } from "../state/nav";
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
    shell.init();
    browserDock.init();
    activityDock.init();
    environmentDock.init();
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

  // Window-level file-drop guard. Without it, a file dropped anywhere outside a
  // drop zone makes WebView2 navigate to its file:// URL — the app "breaks".
  // We swallow every file drag; a stray drop over the Chat workspace still
  // attaches its images to the active tab (the composer's own onDrop
  // stopPropagation()s, so this only catches drops that miss the composer).
  $effect(() => {
    const onWinDragOver = (e: DragEvent) => { if (isFileDrag(e)) e.preventDefault(); };
    const onWinDrop = async (e: DragEvent) => {
      if (!isFileDrag(e)) return;
      e.preventDefault();
      const files = e.dataTransfer?.files;
      if (workspace.activeId !== "chat" || !assistant.currentConvoId || !files?.length) return;
      const res = await attachImageFiles(files, (a) => assistant.addAttachment(a));
      if (res.attached > 0) {
        notify.ok(`${res.attached} image${res.attached === 1 ? "" : "s"} attached`, { detail: "Send to include them in your next message." });
      }
      const msg = summarizeAttach(res);
      if (msg) notify.warn("Some files weren't attached", { detail: msg });
    };
    window.addEventListener("dragover", onWinDragOver);
    window.addEventListener("drop", onWinDrop);
    return () => {
      window.removeEventListener("dragover", onWinDragOver);
      window.removeEventListener("drop", onWinDrop);
    };
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
    // View toggles (browser dock, diff drawer) — keyboard mirror of the topbar.
    if (e.shiftKey && k === "b") { e.preventDefault(); browserDock.toggle(); return; }
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
    // Ctrl+1..9 → workspace switch, mapped by activity-bar order. "home" is a
    // verb (redesign §6) — route it to the empty Chat surface, not the page.
    if (/^[1-9]$/.test(e.key)) {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const id = workspace.order[idx] as WorkspaceId | undefined;
      if (id === "home") { goHome(); return; }
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

<div class="app" data-dots={uiPrefs.dotField}>
  {#if showOnboarding}
    <Titlebar setupMode={true} />
    <div class="ob-host">
      <OnboardingFlow onDone={finishOnboarding} />
    </div>
  {:else}
    <div class="app-body">
      <Sidebar />
      <div class="main">
        <Topbar />
        <main class="workspace">
          <WorkspaceShell />
        </main>
      </div>
    </div>
  {/if}

  <UpdateDialog />

  <UpdatePill />

  <ToastHost />

  <CommandPalette />

  <!-- Screen-tint comfort filter (Settings → Appearance). Hue/strength driven
       by --tint-h / --tint-a on :root; opacity 0 when the filter is off. -->
  <div class="app-tint" aria-hidden="true"></div>
</div>

<style>
  /* One continuous surface across home, conversation, and settings: a single
     base tone + a static dotted field span the whole window, so there's no hard
     seam between the reading column and the rest of the app. Pinned to the
     viewport via position:fixed (the percentage height chain collapses in prod). */
  .app {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    background: var(--app-bg, var(--bg));
    color: var(--fg);
    font-family: var(--font-ui);
    font-size: 13px;
    letter-spacing: -0.005em;
    overflow: hidden;
    transition: background 1s var(--ease-soft);
  }
  .app::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    opacity: var(--dots-opacity, 1);
    background: radial-gradient(circle at center, color-mix(in oklab, var(--dots-ink, var(--fg)) 4.5%, transparent) 0.8px, transparent 1.5px);
    background-size: calc(30px * var(--dots-scale, 1)) calc(30px * var(--dots-scale, 1));
    -webkit-mask-image: radial-gradient(125% 105% at 50% 24%, #000 16%, transparent 80%);
    mask-image: radial-gradient(125% 105% at 50% 24%, #000 16%, transparent 80%);
  }
  /* Background texture variants (Settings → Appearance). "dots" uses the base
     .app::before above; the rest override it. All gradient-based, edge-faded. */
  :global(.app[data-dots="off"])::before { display: none; }
  :global(.app[data-dots="dense"])::before {
    background: radial-gradient(circle at center, color-mix(in oklab, var(--dots-ink, var(--fg)) 4%, transparent) 0.7px, transparent 1.3px);
    background-size: calc(18px * var(--dots-scale, 1)) calc(18px * var(--dots-scale, 1));
  }
  :global(.app[data-dots="margins"])::before {
    background: radial-gradient(circle at center, color-mix(in oklab, var(--dots-ink, var(--fg)) 6%, transparent) 0.8px, transparent 1.5px);
    background-size: calc(30px * var(--dots-scale, 1)) calc(30px * var(--dots-scale, 1));
    -webkit-mask-image: radial-gradient(130% 122% at 50% 46%, transparent 44%, #000 98%);
    mask-image: radial-gradient(130% 122% at 50% 46%, transparent 44%, #000 98%);
  }
  :global(.app[data-dots="grid"])::before {
    background-image:
      linear-gradient(to right, color-mix(in oklab, var(--dots-ink, var(--fg)) 4%, transparent) 1px, transparent 1px),
      linear-gradient(to bottom, color-mix(in oklab, var(--dots-ink, var(--fg)) 4%, transparent) 1px, transparent 1px);
    background-size: calc(34px * var(--dots-scale, 1)) calc(34px * var(--dots-scale, 1));
    -webkit-mask-image: radial-gradient(125% 105% at 50% 24%, #000 14%, transparent 80%);
    mask-image: radial-gradient(125% 105% at 50% 24%, #000 14%, transparent 80%);
  }
  :global(.app[data-dots="lines"])::before {
    background-image: linear-gradient(to bottom, color-mix(in oklab, var(--dots-ink, var(--fg)) 4%, transparent) 1px, transparent 1px);
    background-size: 100% calc(32px * var(--dots-scale, 1));
  }
  :global(.app[data-dots="diagonal"])::before {
    background-image: repeating-linear-gradient(45deg, color-mix(in oklab, var(--dots-ink, var(--fg)) 3.5%, transparent) 0 1px, transparent 1px calc(13px * var(--dots-scale, 1)));
    background-size: auto;
  }
  :global(.app[data-dots="crosshatch"])::before {
    background-image:
      repeating-linear-gradient(45deg, color-mix(in oklab, var(--dots-ink, var(--fg)) 5.5%, transparent) 0 1px, transparent 1px calc(15px * var(--dots-scale, 1))),
      repeating-linear-gradient(-45deg, color-mix(in oklab, var(--dots-ink, var(--fg)) 5.5%, transparent) 0 1px, transparent 1px calc(15px * var(--dots-scale, 1)));
    background-size: auto;
    -webkit-mask-image: radial-gradient(128% 108% at 50% 26%, #000 22%, transparent 86%);
    mask-image: radial-gradient(128% 108% at 50% 26%, #000 22%, transparent 86%);
  }
  :global(.app[data-dots="glow"])::before {
    background:
      radial-gradient(900px 520px at 50% -12%, color-mix(in oklab, var(--accent) 7%, transparent), transparent 60%),
      radial-gradient(720px 480px at 84% 4%, color-mix(in oklab, var(--accent) 8%, transparent), transparent 62%);
    background-size: auto;
    -webkit-mask-image: none; mask-image: none;
  }

  .app-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: row;
    position: relative;
    z-index: 1;
  }
  .main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .workspace {
    flex: 1;
    min-height: 0;
    min-width: 0;
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: wsIn var(--dur-page) var(--ease-page);
  }
  @keyframes wsIn { from { transform: translateY(7px); } to { transform: none; } }
  @media (prefers-reduced-motion: reduce) { .workspace { animation: none; } }

  /* Screen-tint comfort filter — full viewport, never intercepts input. */
  .app-tint {
    position: fixed;
    inset: 0;
    z-index: 90;
    pointer-events: none;
    background: oklch(0.70 0.16 var(--tint-h, 70));
    opacity: var(--tint-a, 0);
    transition: opacity 0.4s var(--ease-soft);
  }

  /* Onboarding takes the full surface below a minimal (brand + winctls) titlebar. */
  .ob-host {
    flex: 1;
    min-height: 0;
    min-width: 0;
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
  }
</style>
