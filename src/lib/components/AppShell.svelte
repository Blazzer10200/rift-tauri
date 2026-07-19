<script lang="ts">
  import { untrack, onMount } from "svelte";
  import { commandPalette } from "../state/command-palette.svelte";
  import CommandPalette from "./dialogs/CommandPalette.svelte";
  import McpServersDialog from "./dialogs/McpServersDialog.svelte";
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
  import { github } from "../state/github.svelte";
  import { toast, notify } from "../state/toast.svelte";
  import { isFileDrag, attachImageFiles, summarizeAttach } from "./assistant/composer/helpers";
  import { goHome } from "../state/nav";
  import { onboarding } from "../state/onboarding.svelte";
  import { betaNotice } from "../state/betaNotice.svelte";
  import OnboardingFlow from "./onboarding/OnboardingFlow.svelte";
  import CloseConfirm from "./shell/CloseConfirm.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import Skeleton from "./shell/Skeleton.svelte";
  import { bootLoad } from "../state/bootLoad.svelte";

  // Verified close-out: the backend intercepts the main window's ✕
  // (lib.rs on_window_event) and emits this; the modal confirms + runs the
  // reap/verify checklist before exiting.
  let closeConfirmOpen = $state(false);

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
    const unlistenClose = listen("rift://close-requested", () => {
      closeConfirmOpen = true;
    });
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
    // Freeze watchdog heartbeat: proves this renderer's main thread is alive.
    // 3s cadence is lockstep with watchdog.rs::BEAT_EVERY_MS — the backend
    // reloads the webview if a FOCUSED window goes silent (v0.131.0 incident).
    const heartbeat = setInterval(() => {
      void invoke("ui_heartbeat").catch(() => {});
    }, 3_000);
    return () => {
      clearInterval(heartbeat);
      void unlistenClose.then((u) => u());
    };
  });

  // Keep the CLI-update store's install method in sync with the auth probe so
  // the top banner shows the correct upgrade command. `untrack` the write so the
  // effect depends ONLY on auth.installMethod, never on cliUpdate.method (which
  // setMethod touches) — that self-dependency was the effect_update_depth loop.
  $effect(() => {
    const m = assistant.auth?.installMethod ?? null;
    untrack(() => cliUpdate.setMethod(m));
  });

  // #42: a CLI updated in an external terminal (`npm i -g …` / `claude update`)
  // left auth.installs/cliVersion stale until the next discrete probe (send,
  // Settings visit). Re-probe when the window regains focus — the moment a user
  // returns from that terminal — throttled to one probe per 60s so alt-tabbing
  // can't spam the where.exe/version probes. Handler reads store state at event
  // time (not effect body), so this effect registers once and never re-tracks.
  $effect(() => {
    const PROBE_MIN_GAP_MS = 60_000;
    const onReturn = () => {
      if (document.visibilityState !== "visible") return;
      // GitHub chip: CI may have finished while the user was away — same
      // focus-regain moment, own 60s min-gap inside the store.
      github.maybeRefresh(assistant.activeRoot);
      if (assistant.authChecking) return;
      const last = assistant.authLastProbed;
      if (last !== null && Date.now() - last < PROBE_MIN_GAP_MS) return;
      void assistant.refreshAuth();
    };
    window.addEventListener("focus", onReturn);
    document.addEventListener("visibilitychange", onReturn);
    return () => {
      window.removeEventListener("focus", onReturn);
      document.removeEventListener("visibilitychange", onReturn);
    };
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

<div class="app">
  {#if showOnboarding}
    <Titlebar />
    <div class="ob-host">
      <OnboardingFlow onDone={finishOnboarding} />
    </div>
  {:else}
    <!-- Full-window strip ABOVE the body: keeps the sidebar head + topbar
         aligned as one continuous top band even while a banner shows. -->
    <UpdateBanner />
    <div class="app-body" class:rail-collapsed={shell.collapsed}>
      <Sidebar />
      <div class="main">
        <Topbar />
        <main class="workspace">
          <WorkspaceShell />
          {#if bootLoad.showSkeleton}
            <div class="ws-skel" aria-hidden="true">
              <Skeleton w="min(680px, 74%)" h="54px" radius="16px" />
              <Skeleton w="150px" h="10px" radius="5px" delay={140} />
            </div>
          {/if}
        </main>
        <!-- Footer INSIDE the island — one surface, one level; the bar inherits
             the island's fill and rounded bottom corners. -->
        <StatusBar />
      </div>
    </div>
  {/if}

  <UpdateDialog />
  <CloseConfirm bind:open={closeConfirmOpen} />

  <ToastHost />

  <CommandPalette />

  <McpServersDialog />
</div>

<style>
  /* One continuous surface across home, conversation, and settings — no hard
     seam between the reading column and the rest of the app. Pinned to the
     viewport via position:fixed (the percentage height chain collapses in prod). */
  .app {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    /* Plain flat canvas — matches the sidebar's calm (owner call 2026-07-16).
       All atmosphere lives INSIDE the main island; the window around it is
       just the base tone. */
    background: var(--bg);
    color: var(--fg);
    font-family: var(--font-ui);
    font-size: 13px;
    letter-spacing: -0.005em;
    overflow: hidden;
    transition: background 1s var(--ease-soft);
  }
  .app-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: row;
    position: relative;
    z-index: 1;
  }
  /* Main island — the content surface floats on the same canvas as the
     sidebar island: inset gutters, rounded hairline card, translucent lift so
     the ambient glow still reads through it. The topbar rides inside as the
     island's header. */
  .main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    margin: 8px 8px 8px 0;
    border-radius: var(--island-radius);
    border: 1px solid var(--island-border);
    /* Onyx slab — the island reads as a machined surface, not a lit stage
       (owner calls 2026-07-16: no grid, no brightness, no diagonal sheen).
       Fill is the SAME opaque gradient as the sidebar island (owner: "blended
       in with the sidebar") + a grounded foot and machined bevel. Strictly
       vertical gradients — angled washes read as glare. */
    background:
      linear-gradient(0deg, oklch(0 0 0 / 0.16) 0%, transparent 12%),
      linear-gradient(180deg, color-mix(in oklab, var(--fg) 4%, var(--bg)), color-mix(in oklab, var(--fg) 1.8%, var(--bg)) 280px);
    /* Machined bevel: a dark seam just inside the hairline border, plus the
       faintest accent-cooled light catching the top edge. */
    box-shadow:
      inset 0 1px 0 oklch(0.92 calc(var(--accent-c) * 0.25) var(--accent-h) / 0.07),
      inset 0 0 0 1px oklch(0 0 0 / 0.22);
    overflow: hidden;
    position: relative;
    isolation: isolate;
    transition: margin-left 0.36s var(--ease-page);
  }
  /* Film grain — static SVG noise at whisper opacity, scoped to the island
     like everything else. Kills banding on the island's light gradient.
     Desaturated in the SVG itself (feColorMatrix) so it never tints. */
  .main::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='160' height='160' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 160px 160px;
    opacity: 0.03;
  }
  /* Rail collapsed → rail width is 0, so the island keeps its own left gutter. */
  .app-body.rail-collapsed .main { margin-left: 8px; }
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

  /* Boot skeleton overlay — a subtle centered stand-in for the hero composer
     while the first load runs; crossfades in, gone once the surface is ready. */
  .ws-skel { position: absolute; inset: 0; z-index: 2; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 16px; pointer-events: none;
    animation: ws-skel-in 300ms var(--ease-page) both; }
  @keyframes ws-skel-in { from { opacity: 0; } to { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .ws-skel { animation: none; } }

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
