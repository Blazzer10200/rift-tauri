<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { X } from "lucide-svelte";
  import { connection, type ServerProfile } from "../state/connection.svelte";
  import Titlebar from "./shell/Titlebar.svelte";
  import TabRail from "./shell/TabRail.svelte";
  import StatusBar from "./shell/StatusBar.svelte";
  import Settings from "./settings/Settings.svelte";
  import ActivityToast from "./ActivityToast.svelte";
  import TwoPane from "./browser/TwoPane.svelte";
  import ActivityFeed from "./activity/ActivityFeed.svelte";
  import SyncPage from "./sync/SyncPage.svelte";
  import Diagnostics from "./diagnostics/Diagnostics.svelte";
  import ConflictsPage from "./conflicts/ConflictsPage.svelte";
  import AddServer from "./dialogs/AddServer.svelte";
  import Bootstrap from "./dialogs/Bootstrap.svelte";
  import Keygen from "./dialogs/Keygen.svelte";
  import Confirm from "./dialogs/Confirm.svelte";
  import Reupload, { type ReuploadChoice } from "./dialogs/Reupload.svelte";
  import CommandPalette, { type Command } from "./dialogs/CommandPalette.svelte";
  import UpdateDialog from "./dialogs/UpdateDialog.svelte";
  import TerminalPanel from "./terminal/TerminalPanel.svelte";
  import AssistantPage from "./assistant/AssistantPage.svelte";
  import ChatTabsBar from "./shell/ChatTabsBar.svelte";
  import RightPane from "./shell/RightPane.svelte";
  import ActivityBar from "./shell/ActivityBar.svelte";
  import { uiPrefs } from "../state/ui-prefs.svelte";
  import { rightPane } from "../state/right-pane.svelte";
  import { PANELS } from "./right-pane";
  import { updates } from "../state/updates.svelte";
  import { terminal } from "../state/terminal.svelte";
  import { syncPage } from "../state/sync-page.svelte";
  import { assistant } from "../state/assistant.svelte";

  type Tab = "browse" | "activity" | "sync" | "assistant" | "conflicts" | "settings" | "diagnostics";
  type SettingsSection = "appearance" | "terminal" | "assistant" | "servers" | "keys" | "about";

  let active = $state<Tab>("browse");
  let settingsSection = $state<SettingsSection>("appearance");
  // v0.3 shell: Settings becomes a slide-over modal. Both v0.2 and v0.3 paths
  // share the `settingsSection` cursor — same target component, different host.
  let settingsModalOpen = $state(false);
  // v0.2.55: lazy-mount + keep-alive — pages mount once on first visit
  // and stay mounted (hidden via `hidden` attr) on tab switch. Drops the
  // remount-flash where children re-ran onMount/transitions every switch.
  let visited = $state<Set<Tab>>(new Set(["browse"]));
  $effect(() => {
    if (!visited.has(active)) {
      visited = new Set([...visited, active]);
    }
  });

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
  let paletteOpen = $state(false);

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

  function gotoSettings(s: SettingsSection = "appearance") {
    settingsSection = s;
    if (uiPrefs.useV03Shell) {
      settingsModalOpen = true;
    } else {
      active = "settings";
    }
  }
  function closeSettingsModal() { settingsModalOpen = false; }

  // ── command registry ──────────────────────────────────────────────
  // Shared (both v0.2 and v0.3) — server + sync ops are surface-agnostic.
  const sharedCommands = $derived<Command[]>([
    { id: "switch-server", group: "Servers", title: "Manage servers…", subtitle: "Add, edit, or delete servers", shortcut: "Ctrl+P",
      run: () => gotoSettings("servers") },
    { id: "add-server", group: "Servers", title: "Add server…", subtitle: "Configure a new SSH/SFTP target",
      run: () => openAddServer() },
    { id: "setup-key", group: "Servers", title: "SSH key setup…", subtitle: "Generate or copy your default ed25519 key",
      run: () => (keygenOpen = true) },
    { id: "bootstrap", group: "Sync", title: "Bootstrap from remote…",
      subtitle: connection.selected ? `Pull missing files for ${connection.selected.name}` : "Connect a server first",
      run: () => openBootstrap() },
    { id: "connect",      group: "Sync",  title: "Connect",        subtitle: connection.selected ? `Start auto-sync for ${connection.selected.name}` : "Pick a server first",
      run: () => connection.connect().catch((e) => console.error(e)) },
    { id: "disconnect",   group: "Sync",  title: "Disconnect",     subtitle: "Stop auto-sync + tunnel",
      run: () => connection.disconnect() },
    { id: "reload",       group: "Servers", title: "Reload servers",
      run: () => connection.loadServers() },
  ]);

  const v02Commands = $derived<Command[]>([
    { id: "tab-browse",   group: "Go to", title: "Files",    shortcut: "Ctrl+1", run: () => (active = "browse")   },
    { id: "tab-sync",     group: "Go to", title: "Sync",     shortcut: "Ctrl+2", run: () => (active = "sync")     },
    { id: "tab-assistant",group: "Go to", title: "Assistant",shortcut: "Ctrl+3", run: () => (active = "assistant")},
    { id: "tab-conflicts",group: "Go to", title: "Conflicts",shortcut: "Ctrl+4", run: () => (active = "conflicts")},
    { id: "tab-activity", group: "Go to", title: "Activity", shortcut: "Ctrl+5", run: () => (active = "activity") },
    { id: "tab-settings", group: "Go to", title: "Settings", shortcut: "Ctrl+6", run: () => (active = "settings") },
    { id: "tab-diagnostics", group: "Go to", title: "Sync Inspector", subtitle: "Live diagnostics for the sync pipeline", shortcut: "Ctrl+Shift+D",
      run: () => (active = "diagnostics") },
  ]);

  const v03Commands = $derived<Command[]>([
    ...rightPane.order.map((id, idx) => {
      const def = PANELS[id];
      return {
        id: `right-pane-${id}`,
        group: "Right pane" as const,
        title: `Toggle ${def.title}`,
        shortcut: `Ctrl+${idx + 1}`,
        run: () => rightPane.toggle(id),
      } as Command;
    }),
    { id: "right-pane-close", group: "Right pane", title: "Close right pane", shortcut: "Ctrl+0", run: () => rightPane.close() },
    { id: "open-settings",    group: "App",        title: "Settings…",        shortcut: "Ctrl+,", run: () => gotoSettings("appearance") },
  ]);

  const commands = $derived<Command[]>(
    uiPrefs.useV03Shell ? [...sharedCommands, ...v03Commands] : [...sharedCommands, ...v02Commands],
  );

  // ── lifecycle ──────────────────────────────────────────────────────
  onMount(async () => {
    try {
      await connection.wireEvents();
    } catch (e) {
      console.error("wireEvents failed; banner will offer retry", e);
    }
    await connection.loadServers();
    await connection.refreshStatus();
    if (!connection.selectedKey && connection.servers.length === 0) {
      gotoSettings("servers");
    }
    updates.checkOnLaunch();
  });

  // Borderless-window maximize compensation. Win32 draws maximized
  // `decorations: false` windows ~8px past each screen edge (invisible
  // resize frame), clipping the close button + StatusBar. Toggle a body
  // class so CSS can inset the shell by 8px while maximized.
  $effect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | null = null;
    let alive = true;

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
      alive = false;
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
    connection.disposeEvents();
    // Audit H9: resolve any in-flight confirm promises so awaiters unblock.
    for (const resolve of pendingConfirms) resolve(false);
    pendingConfirms.clear();
  });

  function onGlobalKey(e: KeyboardEvent) {
    // Esc closes the Settings slide-over (v0.3 only — modal-shaped). v0.4.1
    // dropped the panel-maximize feature, so Esc has no other shell meaning.
    if (e.key === "Escape" && settingsModalOpen) {
      e.preventDefault();
      closeSettingsModal();
      return;
    }
    // v0.4 — Alt+1..9 jumps to chat tab N (1-indexed). Doesn't require meta;
    // checked before the meta-gate. Gated on v0.3 since tabs only exist there.
    if (uiPrefs.useV03Shell && e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const target = assistant.openTabs[idx];
      if (target) void assistant.openTab(target);
      return;
    }
    const meta = e.ctrlKey || e.metaKey;
    if (!meta) return;
    // v0.4 — chat-tab keybinds (gated on v0.3). Ctrl+T new, Ctrl+W close
    // active, Ctrl+Tab cycle. preventDefault keeps WebView2 from intercepting.
    if (uiPrefs.useV03Shell && !e.altKey) {
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
    }
    // Ctrl+` toggles the embedded terminal — global so it works on every tab.
    // Under v0.4.1 this routes to the Terminal right-pane page toggle.
    if (!e.shiftKey && !e.altKey && (e.key === "`" || e.key === "~")) {
      e.preventDefault();
      if (uiPrefs.useV03Shell) rightPane.toggle("terminal");
      else terminal.toggle();
      return;
    }
    const k = e.key.toLowerCase();
    if (e.shiftKey && k === "d") {
      e.preventDefault();
      if (uiPrefs.useV03Shell) rightPane.toggle("activity"); // Diagnostics is v2; Activity is closest stand-in
      else active = "diagnostics";
      return;
    }
    // Ctrl+0 closes the right pane under v0.4.1 (focus chat full-width).
    if (uiPrefs.useV03Shell && !e.shiftKey && e.key === "0") {
      e.preventDefault();
      rightPane.close();
      return;
    }
    // Ctrl+1..7 — right-pane page toggles under v0.4.1, mapped by activity-bar
    // order so a user-reordered bar matches its kbd shortcuts.
    if (uiPrefs.useV03Shell && /^[1-7]$/.test(e.key)) {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const id = rightPane.order[idx];
      if (id) rightPane.toggle(id);
      return;
    }
    if (e.shiftKey) return;
    if (k === "k") { e.preventDefault(); paletteOpen = true; return; }
    // Ctrl+, opens Settings under v0.3 (modal). Under v0.2, Ctrl+P historically
    // routed to Servers section — keep that for v0.2 compat.
    if (e.key === "," && uiPrefs.useV03Shell) { e.preventDefault(); gotoSettings("appearance"); return; }
    if (k === "p") { e.preventDefault(); gotoSettings("servers"); return; }
    if (k === "n") { e.preventDefault(); openAddServer(); return; }
    if (uiPrefs.useV03Shell) return;
    if (/^[1-6]$/.test(e.key)) {
      e.preventDefault();
      const tab = (["browse", "activity", "sync", "assistant", "conflicts", "settings"] as Tab[])[parseInt(e.key, 10) - 1];
      active = tab;
    }
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
  // fingerprint, show a confirmation dialog before we trust it.
  let fingerprintHandled = $state<string | null>(null);
  let shellAlive = true;
  onDestroy(() => { shellAlive = false; });
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
      if (!shellAlive) return;
      if (ok) connection.confirmFingerprint();
      else connection.cancelFingerprint();
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
    onOpenPalette={() => (paletteOpen = true)}
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

  {#if uiPrefs.useV03Shell}
    <ChatTabsBar />
    <div class="body" data-v04-1="true">
      <main class="pane">
        <AssistantPage />
      </main>
      <RightPane />
      <ActivityBar />
    </div>
  {:else}
    <div class="body">
      <TabRail {active} onChange={(t) => (active = t)} />

      <main class="pane">
        {#if visited.has("browse") || active === "browse"}
          <div class="page-shell" hidden={active !== "browse"}>
            <TwoPane onAddServer={() => openAddServer(null)} />
          </div>
        {/if}
        {#if visited.has("activity") || active === "activity"}
          <div class="page-shell" hidden={active !== "activity"}>
            <ActivityFeed />
          </div>
        {/if}
        {#if visited.has("sync") || active === "sync"}
          <div class="page-shell" hidden={active !== "sync"}>
            <SyncPage />
          </div>
        {/if}
        {#if visited.has("assistant") || active === "assistant"}
          <div class="page-shell" hidden={active !== "assistant"}>
            <AssistantPage />
          </div>
        {/if}
        {#if visited.has("conflicts") || active === "conflicts"}
          <div class="page-shell" hidden={active !== "conflicts"}>
            <ConflictsPage />
          </div>
        {/if}
        {#if visited.has("settings") || active === "settings"}
          <div class="page-shell" hidden={active !== "settings"}>
            {#key settingsSection}
              <Settings
                initialSection={settingsSection}
                onAddServer={() => openAddServer(null)}
                onEditServer={(s) => openAddServer(s)}
                onDeleteServer={(s) => deleteServer(s)}
                onLaunchKeygen={() => (keygenOpen = true)}
              />
            {/key}
          </div>
        {/if}
        {#if visited.has("diagnostics") || active === "diagnostics"}
          <div class="page-shell" hidden={active !== "diagnostics"}>
            <Diagnostics />
          </div>
        {/if}
      </main>
    </div>

    <TerminalPanel />
  {/if}
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

  <CommandPalette
    open={paletteOpen}
    {commands}
    onClose={() => (paletteOpen = false)}
  />

  <UpdateDialog />

  <ActivityToast />

  {#if uiPrefs.useV03Shell && settingsModalOpen}
    <div class="slideover-scrim" onclick={closeSettingsModal} role="presentation"></div>
    <aside class="slideover" aria-label="Settings">
      <button
        class="slideover-close"
        type="button"
        onclick={closeSettingsModal}
        title="Close (Esc)"
        aria-label="Close settings"
      >
        <X size={14}/>
      </button>
      {#key settingsSection}
        <Settings
          initialSection={settingsSection}
          onAddServer={() => openAddServer(null)}
          onEditServer={(s) => openAddServer(s)}
          onDeleteServer={(s) => deleteServer(s)}
          onLaunchKeygen={() => (keygenOpen = true)}
        />
      {/key}
    </aside>
  {/if}

</div>

<style>
  .shell {
    display: grid;
    grid-template-rows: 44px 1fr 22px;
    height: 100%;
    width: 100%;
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
  .body {
    flex: 1;
    display: grid;
    grid-template-columns: var(--rail-w, 48px) minmax(0, 1fr);
    transition: grid-template-columns 220ms cubic-bezier(0.4, 0, 0.2, 1);
    min-height: 0;
    min-width: 0;
    overflow: visible;
    position: relative;
  }
  /* v0.4.1 shell: no left rail. Three-column grid:
     [chat | right-pane (0 when closed) | 40px activity bar].
     --right-pane-w is driven by rightPane.svelte.ts (CSS var); collapses to
     0px when activeId is null so chat takes everything except the bar. No
     grid-template-columns transition — see v0.3 history for why pointer-
     driven width was unbearable when eased. */
  .body[data-v04-1="true"] {
    grid-template-columns: minmax(0, 1fr) var(--right-pane-w, 0px) 40px;
    transition: none;
  }
  .pane {
    min-height: 0; min-width: 0;
    overflow: hidden;
    display: flex; flex-direction: column;
  }
  .page-shell {
    flex: 1; min-height: 0; min-width: 0;
    display: flex; flex-direction: column;
  }
</style>
