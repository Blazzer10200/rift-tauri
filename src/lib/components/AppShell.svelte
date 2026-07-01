<script lang="ts">
  import { untrack, onMount } from "svelte";
  import { commandPalette } from "../state/command-palette.svelte";
  import CommandPalette from "./dialogs/CommandPalette.svelte";
  import Titlebar from "./shell/Titlebar.svelte";
  import Sidebar from "./shell/Sidebar.svelte";
  import Topbar from "./shell/Topbar.svelte";
  import StatusBar from "./shell/StatusBar.svelte";
  import ToastHost from "./ToastHost.svelte";
  import UpdateBanner from "./shell/UpdateBanner.svelte";
  // (UpdatePill retired — app updates now surface in the top UpdateBanner.)
  import UpdateDialog from "./dialogs/UpdateDialog.svelte";
  import WorkspaceShell from "./shell/WorkspaceShell.svelte";
  import { shell } from "../state/shell.svelte";
  import { WORKSPACES } from "./workspaces";
  import { workspace, type WorkspaceId } from "../state/workspace.svelte";
  import { browserDock } from "../state/browserDock.svelte";
  import { updates } from "../state/updates.svelte";
  import { cliUpdate } from "../state/cliUpdate.svelte";
  import { news } from "../state/news.svelte";
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
  // One-time boot init. MUST be onMount, NOT $effect: these init() calls write
  // $state the stores also read (shell.pinned, toast.history), so an $effect
  // would re-trigger itself → effect_update_depth_exceeded. onMount runs once,
  // non-reactively. (Boot lands on Home, which reads workspace/conversations off
  // the assistant store — init here, not just on Chat/Settings mount, or the
  // dashboard renders the empty-state lie until another workspace runs it.)
  onMount(() => {
    void assistant.init();
    shell.init();
    browserDock.init();
    toast.init();
    void updates.checkOnLaunch();
    // CLI update: check npm for the latest claude version once on launch so the
    // top banner can surface it app-wide (was Settings-only before). Throttled
    // to 6h internally; method syncs reactively below once auth lands.
    void cliUpdate.maybeCheck();
    // "What's new in AI" feed (Workspace page): fetch the Claude Code release feed
    // once on launch (free, deterministic). Throttled to 6h internally; the
    // Workspace page paints from cache instantly while this refreshes.
    void news.maybeFetch();
    // Dev-only: expose the update store so CDP can drive its visual states
    // (toast + dialog) without a live feed. Stripped from prod builds.
    if (import.meta.env.DEV) {
      (window as unknown as { __updates?: typeof updates }).__updates = updates;
      (window as unknown as { __assistant?: typeof assistant }).__assistant = assistant;
      (window as unknown as { __toast?: typeof toast }).__toast = toast;
    }
  });

  // Keep the CLI-update store's install method in sync with the auth probe so
  // the top banner shows the correct upgrade command. `untrack` the write so the
  // effect depends ONLY on auth.installMethod, never on cliUpdate.method (which
  // setMethod touches) — that self-dependency was the effect_update_depth loop.
  $effect(() => {
    const m = assistant.auth?.installMethod ?? null;
    untrack(() => cliUpdate.setMethod(m));
  });

  // HMR-safe global keydown — $effect cleanup runs on unmount AND when the
  // effect re-tracks during HMR replacement.
  $effect(() => {
    window.addEventListener("keydown", onGlobalKey);
    return () => window.removeEventListener("keydown", onGlobalKey);
  });

  // Responsive sidebar: auto-collapse the rail on narrow windows (laptops /
  // 150%-scaled displays) so the chat column keeps room, and restore the user's
  // own choice on widen. shell.syncToViewport carries the hysteresis + intent
  // tracking; this effect just feeds it the live width (rAF-coalesced).
  $effect(() => {
    let raf = 0;
    const onResize = () => {
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        shell.syncToViewport(window.innerWidth);
      });
    };
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      if (raf) cancelAnimationFrame(raf);
    };
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
      const targetConvoId = assistant.currentConvoId;
      if (workspace.activeId !== "chat" || !targetConvoId || !files?.length) return;
      // Capture the drop-time convo so a tab switch mid-await routes the
      // attachment to the tab the user actually dropped onto.
      const res = await attachImageFiles(files, (a) => assistant.addAttachment(a, targetConvoId));
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
    // Ctrl+N → new chat (global; mirrors the sidebar's primary action). "home"
    // is a verb — floats to the empty Chat surface and mints a fresh tab.
    if (!e.altKey && !e.shiftKey && k === "n") { e.preventDefault(); goHome(); return; }
    // View toggles (browser dock) — keyboard mirror of the topbar.
    if (e.shiftKey && k === "b") { e.preventDefault(); browserDock.toggle(); return; }
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
    <Titlebar />
    <div class="ob-host">
      <OnboardingFlow onDone={finishOnboarding} />
    </div>
  {:else}
    <div class="app-body">
      <Sidebar />
      <div class="main">
        <UpdateBanner />
        <Topbar />
        <main class="workspace">
          <WorkspaceShell />
        </main>
      </div>
    </div>
    <StatusBar />
  {/if}

  <UpdateDialog />

  <ToastHost />

  <CommandPalette />
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
