<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { connection, type ServerProfile } from "../state/connection.svelte";
  import { dialogs } from "../state/dialogs.svelte";
  import { commandPalette } from "../state/command-palette.svelte";
  import CommandPalette from "./dialogs/CommandPalette.svelte";
  import Titlebar from "./shell/Titlebar.svelte";
  import StatusBar from "./shell/StatusBar.svelte";
  import ToastHost from "./ToastHost.svelte";
  import AddServer from "./dialogs/AddServer.svelte";
  import Bootstrap from "./dialogs/Bootstrap.svelte";
  import Keygen from "./dialogs/Keygen.svelte";
  import Confirm from "./dialogs/Confirm.svelte";
  import Reupload, { type ReuploadChoice } from "./dialogs/Reupload.svelte";
  import UpdateDialog from "./dialogs/UpdateDialog.svelte";
  import ChatTabsBar from "./shell/ChatTabsBar.svelte";
  import WorkspaceShell from "./shell/WorkspaceShell.svelte";
  import ActivityBar from "./shell/ActivityBar.svelte";
  import { WORKSPACES } from "./workspaces";
  import { workspace, type WorkspaceId } from "../state/workspace.svelte";
  import { browserDock } from "../state/browserDock.svelte";
  import { updates } from "../state/updates.svelte";
  import { syncPage } from "../state/sync-page.svelte";
  import { assistant } from "../state/assistant.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  // Dialog state
  let addServerOpen = $state(false);
  let editingServer = $state<ServerProfile | null>(null);
  let keygenOpen = $state(false);
  let bootstrapOpen = $state(false);
  let bootstrapDetection = $state<Awaited<ReturnType<typeof detectBootstrapSafe>> | null>(null);
  let confirmState = $state<{
    open: boolean;
    title: string;
    body: string;
    isDanger: boolean;
    onResult: (ok: boolean) => void;
  }>({ open: false, title: "", body: "", isDanger: false, onResult: () => {} });

  // v0.2.55: auto-scan drift the moment a server is ready. Fires once
  // per server-key per session (latch in syncPage). Drops the
  // "open Sync page → click Rescan" first-launch ceremony — drift is
  // ready by the time the user gets there.
  $effect(() => {
    const st = connection.status?.state;
    const key = connection.selectedKey;
    if (!key) {
      syncPage.clearAutoScanMark();
      return;
    }
    if (st === "watching" || st === "idle" || st === "syncing") {
      syncPage.maybeAutoScan(key);
    }
  });

  // v0.2.55: periodic auto-rescan timer — catches remote-side drift the
  // local watcher can't see. Gated on enabled + watcher-ready + key.
  // Effect re-runs on toggle/interval change and tears down the prior
  // timer cleanly via the cleanup return.
  $effect(() => {
    const enabled = syncPage.autoRescanEnabled;
    const intervalMs = syncPage.autoRescanIntervalSec * 1000;
    const st = connection.status?.state;
    const watcherReady = st === "watching" || st === "idle" || st === "syncing";
    if (!enabled || !watcherReady || !connection.selectedKey) return;
    // setInterval first tick is at `intervalMs`, not immediately — set a
    // last-rescan stamp so we don't double-scan if the user just rescanned
    // manually. Past intervalMs since stamp → first tick will run.
    const id = setInterval(() => {
      if (syncPage.busy || syncPage.loading || syncPage.previewMode) return;
      void syncPage.rescan();
    }, intervalMs);
    return () => clearInterval(id);
  });

  function gotoSettings() {
    workspace.setActive("settings");
  }

  // Single alive flag — shared by the win-maximize effect AND the TOFU
  // fingerprint dialog awaiter. Past code had two parallel booleans
  // (alive + shellAlive) that could diverge silently. (#174 / #218)
  let alive = true;

  // ── lifecycle ──────────────────────────────────────────────────────
  // S131: SplashOverlay (layout-level) owns wireEvents + loadServers +
  // refreshStatus + first-run settings redirect. AppShell mounts behind the
  // blur and reacts off connection state as the overlay populates it.
  //
  // Dialog callback wiring lives in onMount/onDestroy (not at script-init):
  // route-level HMR remounts otherwise stack stale closures on the singleton
  // and dispatch into dead $state. (#169)
  onMount(() => {
    browserDock.init();
    updates.checkOnLaunch();
    dialogs.onAddServer = () => openAddServer(null);
    dialogs.onEditServer = (s) => openAddServer(s);
    dialogs.onDeleteServer = (s) => void deleteServer(s);
    dialogs.onLaunchKeygen = () => (keygenOpen = true);
  });

  // Borderless-window maximize compensation. Win32 draws maximized
  // `decorations: false` windows ~8px past each screen edge (invisible
  // resize frame), clipping the close button + StatusBar. Toggle a body
  // class so CSS can inset the shell by 8px while maximized.
  $effect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | null = null;

    async function apply() {
      try {
        const max = await win.isMaximized();
        if (alive) document.body.classList.toggle("win-maximized", max);
      } catch { /* window gone */ }
    }

    void apply();
    void win.onResized(() => { void apply(); }).then((fn) => {
      if (alive) unlisten = fn;
      else fn();
    });

    return () => {
      if (unlisten) unlisten();
      document.body.classList.remove("win-maximized");
    };
  });

  async function retryWire() {
    try {
      await connection.wireEvents();
      await connection.refreshStatus();
    } catch (e) {
      console.error("wireEvents retry failed", e);
    }
  }

  // HMR-safe global keydown — $effect cleanup runs on unmount AND when the
  // effect re-tracks during HMR replacement, vs onMount/onDestroy which can
  // double-bind during fast hot swaps.
  $effect(() => {
    window.addEventListener("keydown", onGlobalKey);
    return () => window.removeEventListener("keydown", onGlobalKey);
  });

  onDestroy(() => {
    alive = false;
    connection.disposeEvents();
    // Reset dialog callbacks to no-ops so a stale closure from this destroyed
    // shell doesn't fire into dead state on the next HMR-driven remount. (#169)
    dialogs.onAddServer = () => {};
    dialogs.onEditServer = () => {};
    dialogs.onDeleteServer = () => {};
    dialogs.onLaunchKeygen = () => {};
    // Audit H9: resolve any in-flight confirm promises so awaiters unblock.
    for (const resolve of pendingConfirms) resolve(false);
    pendingConfirms.clear();
    // If HMR/unmount fires mid-TOFU prompt, the .then() handler above bails on
    // `!alive` before resolving the handshake — leaving `connection.connecting`
    // stuck true until next manual reconnect. Drop the pending state explicitly.
    if (connection.pendingFingerprint) {
      connection.cancelFingerprint();
    }
  });

  function onGlobalKey(e: KeyboardEvent) {
    // Alt+1..9 → jump to chat tab N (1-indexed). Only fires inside the Chat
    // workspace so it doesn't hijack from a focused Files / Activity surface.
    if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey && /^[1-9]$/.test(e.key)
        && workspace.activeId === "chat") {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const target = assistant.openTabs[idx];
      if (target) void assistant.openTab(target);
      return;
    }
    const meta = e.ctrlKey || e.metaKey;
    if (!meta) return;
    // Chat-tab keybinds — same Chat-workspace gate so tabs only respond when
    // the user is actually looking at the chat. preventDefault keeps WebView2
    // from intercepting.
    if (!e.altKey && workspace.activeId === "chat") {
      if (!e.shiftKey && e.key.toLowerCase() === "t") {
        e.preventDefault();
        void assistant.newTab();
        return;
      }
      if (!e.shiftKey && e.key.toLowerCase() === "w") {
        e.preventDefault();
        const cur = assistant.currentConvoId;
        if (cur) void assistant.closeTab(cur);
        return;
      }
      if (e.key === "Tab") {
        e.preventDefault();
        void assistant.cycleTab(e.shiftKey ? -1 : 1);
        return;
      }
      // Ctrl+\ → add a new pane (right of focused). Ctrl+Shift+\ → close
      // focused pane. Both no-op at edges (cap reached / last pane).
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
    // Ctrl+0 → return to Chat workspace (was "close right pane" in v0.4.1;
    // closing has no meaning under the workspace shell).
    if (!e.shiftKey && e.key === "0") {
      e.preventDefault();
      workspace.setActive("chat");
      return;
    }
    // Ctrl+1..9 → workspace switch, mapped by activity-bar order so a user-
    // reordered bar matches its kbd shortcuts. Skips disabled-stub entries.
    if (/^[1-9]$/.test(e.key)) {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const id = workspace.order[idx] as WorkspaceId | undefined;
      if (id && !WORKSPACES[id].disabled) workspace.setActive(id);
      return;
    }
    if (e.shiftKey) return;
    if (e.key === ",") { e.preventDefault(); gotoSettings(); return; }
    // Ctrl+P / Ctrl+K → global command palette. Past Ctrl+P opened Settings;
    // Ctrl+, still covers that path.
    if (k === "p" || k === "k") { e.preventDefault(); commandPalette.show(); return; }
    if (k === "n") { e.preventDefault(); openAddServer(); return; }
  }

  // ── dialog launchers ───────────────────────────────────────────────
  function openAddServer(server: ServerProfile | null = null) {
    editingServer = server;
    addServerOpen = true;
  }

  async function detectBootstrapSafe(serverKey: string) {
    try {
      return await invoke<{
        state: "synced" | "missingLocalRoot" | "empty" | "uninitialized" | "partial" | "badRemoteRoot";
        remoteResourceCount: number;
        localPresentCount: number;
        missingCount: number;
        remoteTopLevelDirs: string[];
      }>("detect_bootstrap", { serverKey });
    } catch (e) {
      console.error("detect_bootstrap failed", e);
      return null;
    }
  }

  let openingBootstrap = $state(false);
  async function openBootstrap() {
    if (openingBootstrap || bootstrapOpen) return;
    const sel = connection.selected;
    if (!sel) {
      askConfirm({
        title: "No server selected",
        body: "Pick or add a server first, then run Bootstrap.",
        isDanger: false,
      });
      return;
    }
    openingBootstrap = true;
    try {
      bootstrapDetection = await detectBootstrapSafe(sel.key);
      bootstrapOpen = true;
    } finally {
      openingBootstrap = false;
    }
  }

  // Audit H9: track pending confirm resolvers so onDestroy can drain them
  // (resolve(false)) instead of leaving the awaiter hanging forever if the
  // shell unmounts mid-dialog.
  const pendingConfirms = new Set<(ok: boolean) => void>();
  function askConfirm(opts: { title: string; body: string; isDanger?: boolean }): Promise<boolean> {
    return new Promise((resolve) => {
      pendingConfirms.add(resolve);
      confirmState = {
        open: true,
        title: opts.title,
        body: opts.body,
        isDanger: opts.isDanger ?? false,
        onResult: (ok) => {
          pendingConfirms.delete(resolve);
          resolve(ok);
        },
      };
    });
  }

  // Audit C2 — TOFU prompt. When connection.connect() probes a fresh
  // fingerprint, show a confirmation dialog before we trust it. The `alive`
  // gate prevents the resolved promise from acting on a destroyed shell.
  let fingerprintHandled = $state<string | null>(null);
  $effect(() => {
    const fp = connection.pendingFingerprint;
    if (!fp || fp === fingerprintHandled) return;
    fingerprintHandled = fp;
    const name = connection.selected?.name ?? "this server";
    askConfirm({
      title: "Trust this server fingerprint?",
      body: `First connection to ${name}. Verify this matches what you expect from the server admin before accepting.\n\n${fp}`,
      isDanger: false,
    }).then((ok) => {
      if (!alive) {
        // #171: shell unmounted mid-dialog. Without this, `connecting` stays
        // stuck true on the persistent connection store and the StatusBar
        // reconnect button locks for the next remount. Treat unmount as cancel.
        connection.cancelFingerprint();
        return;
      }
      if (ok) connection.confirmFingerprint();
      else connection.cancelFingerprint();
      fingerprintHandled = null;
    }).catch(() => {
      // Promise rejection (rare: dialog drained on shutdown) — same recovery
      // as the unmount branch so `connecting` never sticks.
      connection.cancelFingerprint();
      fingerprintHandled = null;
    });
  });

  async function handleReupload(
    head: { remotePath: string; tmpPath: string; displayName: string; mtime: string; size: number },
    choice: ReuploadChoice | null,
  ) {
    connection.popReuploadPrompt();
    const key = connection.selectedKey;
    if (!key) return;

    if (choice === "always") {
      localStorage.setItem(`rift.reupload.always.${key}`, "1");
    }
    if (choice === "reupload" || choice === "always") {
      try {
        await invoke("save_edit_in_place", {
          serverKey: key,
          remotePath: head.remotePath,
        });
      } catch (e) {
        console.error("save_edit_in_place failed", e);
      }
    }
    // skip / null → leave file dirty; user can act later
  }

  async function deleteServer(s: ServerProfile) {
    const ok = await askConfirm({
      title: "Delete server?",
      body: `Remove "${s.name}" from rift.json? Local files won't be touched. This can't be undone.`,
      isDanger: true,
    });
    if (!ok) return;
    try {
      await connection.deleteServer(s.key);
    } catch (e) {
      console.error("deleteServer failed", e);
    }
  }
</script>

<div class="shell">
  <Titlebar
    onAddServer={() => openAddServer(null)}
    onEditCurrent={(s) => openAddServer(s)}
  />

  <div class="middle">
    {#if connection.wireError}
      <div class="wire-error" role="alert">
        <span>Tauri event wiring failed — status, locks, and activity are frozen. {connection.wireError}</span>
        <button type="button" onclick={retryWire}>Retry</button>
      </div>
    {/if}

    <div class="tabs-rail" data-show={workspace.activeId === "chat"} aria-hidden={workspace.activeId !== "chat"}>
      <ChatTabsBar />
    </div>
    <div class="body">
      <ActivityBar />
      <main class="pane">
        <WorkspaceShell />
      </main>
    </div>
  </div>

  <StatusBar />

  <AddServer
    open={addServerOpen}
    editing={editingServer}
    onLaunchKeygen={() => (keygenOpen = true)}
    onClose={async (saved) => {
      addServerOpen = false;
      editingServer = null;
      if (saved) {
        await connection.loadServers();
        await connection.select(saved.key);
      }
    }}
  />

  <Keygen
    open={keygenOpen}
    onClose={(_keyPath) => (keygenOpen = false)}
  />

  <Bootstrap
    open={bootstrapOpen}
    serverKey={connection.selected?.key ?? ""}
    initialLocalRoot={connection.selected?.localRoot ?? ""}
    detection={bootstrapDetection}
    onClose={(_result) => {
      bootstrapOpen = false;
      bootstrapDetection = null;
    }}
  />

  <Confirm
    open={confirmState.open}
    title={confirmState.title}
    body={confirmState.body}
    isDanger={confirmState.isDanger}
    onClose={({ confirmed }) => {
      const cb = confirmState.onResult;
      confirmState = { ...confirmState, open: false };
      cb(confirmed);
    }}
  />

  {#if connection.pendingReuploads.length > 0}
    {@const head = connection.pendingReuploads[0]}
    <Reupload
      open={true}
      fileName={head.displayName}
      detail={head.remotePath}
      onClose={(choice: ReuploadChoice | null) => handleReupload(head, choice)}
    />
  {/if}

  <UpdateDialog />

  <ToastHost />

  <CommandPalette onAddServer={() => openAddServer(null)} />

</div>

<style>
  /* S136: pin .shell to the viewport via position:fixed instead of a
     percentage height chain (body 100% → wrapper display:contents → shell
     100%). The chain works in dev but intermittently collapses in prod
     builds — leaves the bottom of the window blank below the status bar.
     position:fixed + inset:0 sidesteps every parent-height-resolution edge
     case. SplashOverlay is also position:fixed so flow ordering is moot. */
  .shell {
    position: fixed;
    inset: 0;
    display: grid;
    grid-template-rows: 44px 1fr 26px;
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
  .wire-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 14px;
    background: var(--accent-danger-bg, #5a1d1d);
    color: var(--accent-danger-fg, #fecaca);
    font-size: var(--fs-sm, 12px);
    border-bottom: 1px solid var(--accent-danger-border, #7f2d2d);
  }
  .wire-error button {
    background: transparent;
    color: inherit;
    border: 1px solid currentColor;
    border-radius: 4px;
    padding: 3px 10px;
    cursor: pointer;
    font: inherit;
  }
  .wire-error button:hover { background: rgba(255,255,255,0.08); }
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
    /* will-change removed: perma-compositor layer caused text-blur on tab
       labels at fractional DPR (1.25/1.5x Win scaling). max-height can't be
       composited anyway, so it was a no-op for perf + a cost for clarity. */
  }
  .tabs-rail[data-show="true"] {
    max-height: 48px;
    opacity: 1;
    transform: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .tabs-rail { transition: none; transform: none; }
  }
  /* Workspace shell: [44px activity bar | main pane]. WorkspaceShell mounts
     each workspace once-and-keeps-it via the everOpened latch — no width to
     animate, so no grid-template-columns transition. */
  .body {
    flex: 1;
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    min-height: 0;
    min-width: 0;
    overflow: visible;
    position: relative;
  }
  .pane {
    min-height: 0; min-width: 0;
    overflow: hidden;
    display: flex; flex-direction: column;
  }
</style>
