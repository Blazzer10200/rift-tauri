<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile, type ConflictRecord } from "../state/connection.svelte";
  import TopBar from "./TopBar.svelte";
  import ServerPicker from "./ServerPicker.svelte";
  import StatusHero from "./StatusHero.svelte";
  import ActivityToast from "./ActivityToast.svelte";
  import TwoPane from "./browser/TwoPane.svelte";
  import ActivityFeed from "./activity/ActivityFeed.svelte";
  import DriftReview from "./drift/DriftReview.svelte";
  import ConflictList from "./conflicts/ConflictList.svelte";
  import ConflictResolver from "./conflicts/ConflictResolver.svelte";
  import AddServer from "./dialogs/AddServer.svelte";
  import Bootstrap from "./dialogs/Bootstrap.svelte";
  import Keygen from "./dialogs/Keygen.svelte";
  import Confirm from "./dialogs/Confirm.svelte";
  import CommandPalette, { type Command } from "./dialogs/CommandPalette.svelte";

  type Tab = "browse" | "activity" | "drift" | "conflicts" | "settings";

  let active = $state<Tab>("browse");
  let pickerOpen = $state(false);
  let version = $state("?");
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

  const tabs: { id: Tab; label: string }[] = [
    { id: "browse", label: "Browse" },
    { id: "activity", label: "Activity" },
    { id: "drift", label: "Drift" },
    { id: "conflicts", label: "Conflicts" },
    { id: "settings", label: "Settings" },
  ];

  // ── command registry ──────────────────────────────────────────────
  const commands = $derived<Command[]>([
    { id: "switch-server", title: "Switch server…", subtitle: "Open the server picker", shortcut: "Ctrl+P",
      run: () => (pickerOpen = true) },
    { id: "add-server", title: "Add server…", subtitle: "Configure a new SSH/SFTP target",
      run: () => openAddServer() },
    { id: "setup-key", title: "SSH key setup…", subtitle: "Generate or copy your default ed25519 key",
      run: () => (keygenOpen = true) },
    { id: "bootstrap", title: "Bootstrap from remote…",
      subtitle: connection.selected ? `Pull missing files for ${connection.selected.name}` : "Connect a server first",
      run: () => openBootstrap() },
    { id: "tab-browse",   title: "Go to Browse",   run: () => (active = "browse")   },
    { id: "tab-activity", title: "Go to Activity", run: () => (active = "activity") },
    { id: "tab-drift",    title: "Go to Drift",    run: () => (active = "drift")    },
    { id: "tab-conflicts",title: "Go to Conflicts",run: () => (active = "conflicts")},
    { id: "tab-settings", title: "Go to Settings", run: () => (active = "settings") },
    { id: "disconnect",   title: "Disconnect",     subtitle: "Stop auto-sync + tunnel",
      run: () => connection.disconnect() },
    { id: "reload",       title: "Reload servers",
      run: () => connection.loadServers() },
  ]);

  // ── lifecycle ──────────────────────────────────────────────────────
  onMount(async () => {
    try { version = await invoke<string>("app_version"); } catch {}
    await connection.wireEvents();
    await connection.loadServers();
    await connection.refreshStatus();
    if (!connection.selectedKey && connection.servers.length === 0) {
      pickerOpen = true;
    }
    window.addEventListener("keydown", onGlobalKey);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onGlobalKey);
    connection.disposeEvents();
  });

  function onGlobalKey(e: KeyboardEvent) {
    // Ctrl+K (or Cmd+K) — command palette
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      paletteOpen = true;
    }
    // Ctrl+P — server picker
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "p") {
      e.preventDefault();
      pickerOpen = true;
    }
  }

  // ── dialog launchers ───────────────────────────────────────────────
  function openAddServer(server: ServerProfile | null = null) {
    editingServer = server;
    addServerOpen = true;
    pickerOpen = false;
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

  async function openBootstrap() {
    const sel = connection.selected;
    if (!sel) {
      askConfirm({
        title: "No server selected",
        body: "Pick or add a server first, then run Bootstrap.",
        isDanger: false,
      });
      return;
    }
    bootstrapDetection = await detectBootstrapSafe(sel.key);
    bootstrapOpen = true;
  }

  function askConfirm(opts: { title: string; body: string; isDanger?: boolean }): Promise<boolean> {
    return new Promise((resolve) => {
      confirmState = {
        open: true,
        title: opts.title,
        body: opts.body,
        isDanger: opts.isDanger ?? false,
        onResult: (ok) => resolve(ok),
      };
    });
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
  <TopBar {version} onPickServer={() => (pickerOpen = true)} />

  <div class="body">
    <nav class="sidebar">
      {#each tabs as t (t.id)}
        <button
          class="tab"
          class:active={active === t.id}
          onclick={() => (active = t.id)}
          type="button"
        >
          {t.label}
          {#if t.id === "conflicts" && connection.conflictCount > 0}
            <span class="badge danger">{connection.conflictCount}</span>
          {/if}
          {#if t.id === "activity" && connection.activityFeed.length > 0}
            <span class="badge dim">{connection.activityFeed.length}</span>
          {/if}
          {#if t.id === "browse" && connection.locks.length > 0}
            <span class="badge warn">{connection.locks.length}</span>
          {/if}
        </button>
      {/each}
      <div class="sidebar-spacer"></div>
      <button class="tab hint" onclick={() => (paletteOpen = true)} type="button" title="Command palette">
        ⌕ Commands <kbd>Ctrl+K</kbd>
      </button>
    </nav>

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
            <div class="placeholder"><h2>Conflicts</h2><p class="hint">{connection.conflicts.length === 0 ? "No conflicts." : "Pick a conflict from the list."}</p></div>
          {/if}
        </div>
      {:else if active === "settings"}
        <div class="placeholder">
          <h2>Settings</h2>
          <p>Manage servers from the picker. Use the command palette (<kbd>Ctrl+K</kbd>) for everything else.</p>
          <div class="settings-actions">
            <button onclick={() => (pickerOpen = true)} type="button">Open server picker</button>
            <button onclick={() => (keygenOpen = true)} type="button">SSH key setup…</button>
            <button onclick={openBootstrap} type="button" disabled={!connection.selected}>Bootstrap from remote…</button>
          </div>
        </div>
      {/if}
    </main>
  </div>

  <ServerPicker
    open={pickerOpen}
    onClose={() => (pickerOpen = false)}
    onAdd={() => openAddServer(null)}
    onEdit={(s) => openAddServer(s)}
    onDelete={(s) => deleteServer(s)}
    onLaunchKeygen={() => { pickerOpen = false; keygenOpen = true; }}
  />

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

  <CommandPalette
    open={paletteOpen}
    commands={commands}
    onClose={() => (paletteOpen = false)}
  />

  <ActivityToast />
</div>

<style>
  .shell {
    display: flex; flex-direction: column;
    height: 100vh;
    background: #0F0F12;
    color: #E8E8EE;
  }
  .body { display: flex; flex: 1; min-height: 0; }
  .sidebar {
    width: 168px;
    background: #0F0F12;
    border-right: 1px solid #26262E;
    display: flex; flex-direction: column;
    padding: 10px 8px;
    gap: 2px;
  }
  .sidebar-spacer { flex: 1; }
  .tab {
    background: transparent;
    border: 0;
    color: #7A7A85;
    text-align: left;
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
    display: flex; align-items: center; gap: 8px;
  }
  .tab:hover { background: #17171C; color: #E8E8EE; }
  .tab.active {
    background: #15101E;
    color: #E8E8EE;
    border-left: 2px solid #8B6BE6;
    padding-left: 10px;
    font-weight: 600;
  }
  .tab.hint { font-size: 11px; color: #7A7A85; justify-content: space-between; }
  .tab.hint kbd {
    background: #1F1F26;
    color: #7A7A85;
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 2px;
    font-family: Consolas, monospace;
  }
  .badge {
    margin-left: auto;
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 999px;
    color: white;
  }
  .badge.danger { background: #FF5C6B; }
  .badge.warn { background: #F0B95C; color: #15101E; }
  .badge.dim { background: #26262E; color: #7A7A85; }
  .conflicts-pane { display: flex; flex: 1; min-height: 0; }

  .pane { flex: 1; min-height: 0; overflow: hidden; display: flex; flex-direction: column; }
  .pane > :global(.placeholder) { overflow: auto; flex: 1; }
  .browse-stack { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .placeholder { padding: 22px; color: #E8E8EE; }
  .placeholder h2 { margin: 0 0 8px; font-size: 16px; }
  .placeholder p { color: #7A7A85; font-size: 13px; margin: 4px 0; }
  .placeholder kbd {
    background: #1F1F26;
    color: #E8E8EE;
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 2px;
    font-family: Consolas, monospace;
  }
  .settings-actions { display: flex; gap: 8px; margin-top: 14px; }
  .settings-actions button {
    background: #1F1F26;
    border: 1px solid #26262E;
    color: #E8E8EE;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  .settings-actions button:hover:not(:disabled) { background: #26262E; }
  .settings-actions button:disabled { opacity: 0.5; cursor: not-allowed; }
  .hint { color: #7A7A85; font-size: 12px; }
</style>
