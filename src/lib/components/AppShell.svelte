<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile, type ConflictRecord } from "../state/connection.svelte";
  import Titlebar from "./shell/Titlebar.svelte";
  import TabRail from "./shell/TabRail.svelte";
  import StatusBar from "./shell/StatusBar.svelte";
  import TopBar from "./TopBar.svelte";
  import Settings from "./settings/Settings.svelte";
  import StatusHero from "./StatusHero.svelte";
  import ActivityToast from "./ActivityToast.svelte";
  import ScanProgressChip from "./ScanProgressChip.svelte";
  import TwoPane from "./browser/TwoPane.svelte";
  import ActivityFeed from "./activity/ActivityFeed.svelte";
  import Diagnostics from "./diagnostics/Diagnostics.svelte";
  import DriftReview from "./drift/DriftReview.svelte";
  import ConflictList from "./conflicts/ConflictList.svelte";
  import ConflictResolver from "./conflicts/ConflictResolver.svelte";
  import AddServer from "./dialogs/AddServer.svelte";
  import Bootstrap from "./dialogs/Bootstrap.svelte";
  import Keygen from "./dialogs/Keygen.svelte";
  import Confirm from "./dialogs/Confirm.svelte";
  import Reupload, { type ReuploadChoice } from "./dialogs/Reupload.svelte";
  import CommandPalette, { type Command } from "./dialogs/CommandPalette.svelte";
  import UpdateDialog from "./dialogs/UpdateDialog.svelte";
  import { updates } from "../state/updates.svelte";

  type Tab = "browse" | "activity" | "drift" | "conflicts" | "settings" | "diagnostics";
  type SettingsSection = "appearance" | "tokens" | "servers" | "keys" | "sync" | "editor" | "about";

  let active = $state<Tab>("browse");
  let settingsSection = $state<SettingsSection>("appearance");
  let selectedConflict = $state<ConflictRecord | null>(null);

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

  $effect(() => {
    const list = connection.conflicts;
    if (selectedConflict && !list.some((c) => c.local_path === selectedConflict!.local_path)) {
      selectedConflict = null;
    }
  });

  function gotoSettings(s: SettingsSection = "appearance") {
    settingsSection = s;
    active = "settings";
  }

  // ── command registry ──────────────────────────────────────────────
  const commands = $derived<Command[]>([
    { id: "switch-server", group: "Servers", title: "Manage servers…", subtitle: "Add, edit, or delete servers", shortcut: "Ctrl+P",
      run: () => gotoSettings("servers") },
    { id: "add-server", group: "Servers", title: "Add server…", subtitle: "Configure a new SSH/SFTP target",
      run: () => openAddServer() },
    { id: "setup-key", group: "Servers", title: "SSH key setup…", subtitle: "Generate or copy your default ed25519 key",
      run: () => (keygenOpen = true) },
    { id: "bootstrap", group: "Sync", title: "Bootstrap from remote…",
      subtitle: connection.selected ? `Pull missing files for ${connection.selected.name}` : "Connect a server first",
      run: () => openBootstrap() },
    { id: "tab-browse",   group: "Go to", title: "Browser",  shortcut: "Ctrl+1", run: () => (active = "browse")   },
    { id: "tab-activity", group: "Go to", title: "Activity", shortcut: "Ctrl+2", run: () => (active = "activity") },
    { id: "tab-drift",    group: "Go to", title: "Drift",    shortcut: "Ctrl+3", run: () => (active = "drift")    },
    { id: "tab-conflicts",group: "Go to", title: "Conflicts",shortcut: "Ctrl+4", run: () => (active = "conflicts")},
    { id: "tab-settings", group: "Go to", title: "Settings", shortcut: "Ctrl+5", run: () => (active = "settings") },
    { id: "tab-diagnostics", group: "Go to", title: "Sync Inspector", subtitle: "Live diagnostics for the sync pipeline", shortcut: "Ctrl+Shift+D",
      run: () => (active = "diagnostics") },
    { id: "connect",      group: "Sync",  title: "Connect",        subtitle: connection.selected ? `Start auto-sync for ${connection.selected.name}` : "Pick a server first",
      run: () => connection.connect().catch((e) => console.error(e)) },
    { id: "disconnect",   group: "Sync",  title: "Disconnect",     subtitle: "Stop auto-sync + tunnel",
      run: () => connection.disconnect() },
    { id: "reload",       group: "Servers", title: "Reload servers",
      run: () => connection.loadServers() },
  ]);

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
    const meta = e.ctrlKey || e.metaKey;
    if (!meta) return;
    const k = e.key.toLowerCase();
    if (e.shiftKey && k === "d") { e.preventDefault(); active = "diagnostics"; return; }
    if (e.shiftKey) return;
    if (k === "k") { e.preventDefault(); paletteOpen = true; return; }
    if (k === "p") { e.preventDefault(); gotoSettings("servers"); return; }
    if (k === "n") { e.preventDefault(); openAddServer(); return; }
    if (/^[1-5]$/.test(e.key)) {
      e.preventDefault();
      const tab = (["browse", "activity", "drift", "conflicts", "settings"] as Tab[])[parseInt(e.key, 10) - 1];
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
    onOpenSettings={() => (active = "settings")}
  />

  <TopBar
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

  <div class="body">
    <TabRail {active} onChange={(t) => (active = t)} />

    <main class="pane">
      {#if active === "browse"}
        <div class="browse-stack">
          <StatusHero />
          <TwoPane />
        </div>
      {:else if active === "activity"}
        <ActivityFeed />
      {:else if active === "drift"}
        <DriftReview />
      {:else if active === "conflicts"}
        <div class="conflicts-pane">
          <ConflictList
            selected={selectedConflict}
            onSelect={(c: ConflictRecord) => (selectedConflict = c)}
          />
          {#if selectedConflict}
            {#key selectedConflict.local_path}
              <ConflictResolver conflict={selectedConflict} />
            {/key}
          {:else}
            <div class="placeholder"><h2>Conflicts</h2><p class="help">{connection.conflicts.length === 0 ? "No conflicts." : "Pick a conflict from the list."}</p></div>
          {/if}
        </div>
      {:else if active === "settings"}
        {#key settingsSection}
          <Settings
            initialSection={settingsSection}
            onAddServer={() => openAddServer(null)}
            onEditServer={(s) => openAddServer(s)}
            onDeleteServer={(s) => deleteServer(s)}
            onLaunchKeygen={() => (keygenOpen = true)}
          />
        {/key}
      {:else if active === "diagnostics"}
        <Diagnostics />
      {/if}
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

  <CommandPalette
    open={paletteOpen}
    {commands}
    onClose={() => (paletteOpen = false)}
  />

  <UpdateDialog />

  <ActivityToast />
  <ScanProgressChip />
</div>

<style>
  .shell {
    display: grid;
    grid-template-rows: 32px 44px 1fr 22px;
    height: 100vh;
    background: var(--bg);
    color: var(--fg);
  }
  .middle {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
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
    grid-template-columns: 200px 1fr;
    min-height: 0;
    overflow: hidden;
  }
  .pane {
    min-height: 0; min-width: 0;
    overflow: hidden;
    display: flex; flex-direction: column;
  }
  .conflicts-pane { display: flex; flex: 1; min-height: 0; }
  .browse-stack { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .placeholder { padding: 22px; color: var(--fg); overflow: auto; flex: 1; }
  .placeholder h2 { margin: 0 0 8px; font-size: var(--fs-lg); font-weight: 600; }
  .placeholder p { color: var(--fg-muted); font-size: var(--fs-md); margin: 4px 0; }
</style>
