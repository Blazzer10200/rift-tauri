<script lang="ts">
  import { FolderOpen, Activity, TriangleAlert, Cog, Download, DownloadCloud, UploadCloud, RefreshCw } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { connection } from "../../state/connection.svelte";
  import { updates } from "../../state/updates.svelte";
  import { syncModal } from "../../state/sync-modal.svelte";

  type Tab = "browse" | "activity" | "conflicts" | "settings" | "diagnostics";

  let { active, onChange }: {
    active: Tab;
    onChange: (t: Tab) => void;
  } = $props();

  type TabTone = "accent" | "info" | "danger" | "neutral";
  const tabs: { id: Tab; label: string; icon: typeof FolderOpen; kbd: string; tone: TabTone; count?: () => number; countCls?: string }[] = [
    { id: "browse",    label: "Browser",   icon: FolderOpen,    kbd: "1", tone: "accent" },
    { id: "activity",  label: "Activity",  icon: Activity,      kbd: "2", tone: "info",   count: () => connection.activityFeed.length, countCls: "" },
    { id: "conflicts", label: "Conflicts", icon: TriangleAlert, kbd: "3", tone: "danger", count: () => connection.conflictCount, countCls: "danger" },
    { id: "settings",  label: "Settings",  icon: Cog,           kbd: "4", tone: "neutral" },
  ];

  const watcherOn = $derived(
    connection.status?.state === "watching" || connection.status?.state === "idle" || connection.status?.state === "syncing"
  );
  const canSync = $derived(watcherOn && !syncModal.open);
  const activeIdx = $derived(tabs.findIndex((t) => t.id === active));
  const indicatorVisible = $derived(activeIdx >= 0);

  let pulling = $state(false);
  let pushing = $state(false);
  let scanning = $state(false);

  async function reconcile() {
    if (!canSync || pulling || pushing || scanning) return;
    scanning = true;
    syncModal.start();
    try {
      const fired = await invoke<boolean>("diag_force_drift_scan");
      if (!fired) syncModal.fail("Not connected — start watcher first.");
    } catch (e) {
      syncModal.fail(String(e));
    } finally {
      scanning = false;
    }
  }

  async function pullAll() {
    if (!canSync || pulling || pushing || scanning) return;
    pulling = true;
    syncModal.start("pull");
    try {
      const fired = await invoke<boolean>("diag_force_pull_now");
      if (!fired) syncModal.fail("Not connected — start watcher first.");
    } catch (e) {
      syncModal.fail(String(e));
    } finally {
      pulling = false;
    }
  }

  async function pushAll() {
    if (!canSync || pulling || pushing || scanning) return;
    pushing = true;
    syncModal.start("push");
    try {
      const fired = await invoke<boolean>("diag_force_push_now");
      if (!fired) syncModal.fail("Not connected — start watcher first.");
    } catch (e) {
      syncModal.fail(String(e));
    } finally {
      pushing = false;
    }
  }
</script>

<aside class="rail" aria-label="Primary navigation">
  <div class="rail-panel">
    <div class="group" style="--active-y: {Math.max(0, activeIdx) * 31}px">
      <div class="rail-indicator" aria-hidden="true" data-visible={indicatorVisible} data-tone={tabs[activeIdx]?.tone ?? "accent"}></div>
      {#each tabs as t (t.id)}
        {@const Icon = t.icon}
        {@const c = t.count ? t.count() : 0}
        <button
          class="rail-btn"
          data-active={active === t.id}
          data-tone={t.tone}
          onclick={(e) => { onChange(t.id); (e.currentTarget as HTMLButtonElement).blur(); }}
          title="{t.label} (Ctrl+{t.kbd})"
          type="button"
        >
          <span class="rail-icon"><Icon size={16}/></span>
          <span class="label">{t.label}</span>
          {#if c > 0}
            <span class="count-pip {t.countCls ?? ''}">{c}</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="bottom">
      {#if updates.state === "available" && updates.info}
        <button
          class="update-pill"
          type="button"
          onclick={(e) => { updates.open(); (e.currentTarget as HTMLButtonElement).blur(); }}
          title="Update {updates.info.version} available — click for details"
        >
          <span class="up-dot"></span>
          <Download size={12}/>
          <span class="up-text">
            <span class="up-l">Update available</span>
            <span class="up-v mono">{updates.info.version}</span>
          </span>
        </button>
      {/if}

      <div class="qa">
        <div class="qa-label">Quick actions</div>
        <button
          class="qa-btn"
          data-tone="accent"
          type="button"
          onclick={(e) => { reconcile(); (e.currentTarget as HTMLButtonElement).blur(); }}
          disabled={!canSync || pulling || pushing || scanning}
          title={watcherOn ? "Reconcile — scan both sides for drift" : "Connect a server first"}
        >
          <span class="qa-icon"><RefreshCw size={16}/></span>
          <span>Reconcile</span>
          {#if scanning}<span class="qa-spin"></span>{/if}
        </button>
        <button
          class="qa-btn"
          data-tone="info"
          type="button"
          onclick={(e) => { pullAll(); (e.currentTarget as HTMLButtonElement).blur(); }}
          disabled={!canSync || pulling || pushing || scanning}
          title={watcherOn ? "Pull all changes from remote" : "Connect a server first"}
        >
          <span class="qa-icon"><DownloadCloud size={16}/></span>
          <span>Pull all</span>
          {#if pulling}<span class="qa-spin"></span>{/if}
        </button>
        <button
          class="qa-btn"
          data-tone="warn"
          type="button"
          onclick={(e) => { pushAll(); (e.currentTarget as HTMLButtonElement).blur(); }}
          disabled={!canSync || pulling || pushing || scanning}
          title={watcherOn ? "Push all local changes to remote" : "Connect a server first"}
        >
          <span class="qa-icon"><UploadCloud size={16}/></span>
          <span>Push all</span>
          {#if pushing}<span class="qa-spin"></span>{/if}
        </button>
      </div>
    </div>
  </div>
</aside>

<style>
  .rail {
    position: relative;
    width: 48px;
    height: 100%;
  }
  .rail-panel {
    position: absolute;
    left: 0; top: 0; bottom: 0;
    width: 48px;
    background: var(--bg);
    border-right: 1px solid var(--border);
    padding: 8px 6px;
    display: flex; flex-direction: column;
    min-height: 0;
    overflow: hidden;
    container-type: inline-size;
    z-index: 20;
    transition: width 220ms cubic-bezier(0.4, 0, 0.2, 1),
                box-shadow 220ms ease,
                border-right-color 220ms ease,
                padding 220ms ease;
  }
  .rail:hover .rail-panel,
  .rail:focus-within .rail-panel {
    width: 220px;
    padding: 8px;
    box-shadow: 6px 0 24px rgba(0, 0, 0, 0.28);
    border-right-color: var(--border-strong, var(--border));
  }

  .group { display: flex; flex-direction: column; gap: 1px; position: relative; }
  .rail-indicator {
    position: absolute;
    left: -6px; top: 0;
    width: 2px; height: 30px;
    background: var(--accent);
    border-radius: 2px;
    transform: translateY(var(--active-y, 0px));
    transition: transform 220ms cubic-bezier(0.4, 0, 0.2, 1), opacity 160ms ease, left 220ms ease, background 180ms ease;
    pointer-events: none;
    opacity: 0;
    z-index: 1;
  }
  .rail-indicator[data-tone="info"]    { background: var(--info); }
  .rail-indicator[data-tone="danger"]  { background: var(--danger); }
  .rail-indicator[data-tone="neutral"] { background: var(--fg-muted); }
  .rail-indicator[data-visible="true"] { opacity: 1; }
  .rail:hover .rail-indicator,
  .rail:focus-within .rail-indicator { left: -8px; }

  .rail-btn {
    --tone: var(--accent);
    display: flex; align-items: center; gap: 10px;
    width: 100%; height: 30px;
    padding: 0 8px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
    position: relative;
    transition: background 140ms ease, color 140ms ease;
    white-space: nowrap;
    overflow: hidden;
  }
  .rail-btn[data-tone="info"]    { --tone: var(--info); }
  .rail-btn[data-tone="danger"]  { --tone: var(--danger); }
  .rail-btn[data-tone="neutral"] { --tone: var(--fg-muted); }
  .rail-icon {
    flex-shrink: 0;
    display: inline-flex; align-items: center; justify-content: center;
    width: 20px;
    color: var(--tone);
    opacity: 0.75;
    transition: transform 180ms cubic-bezier(0.34, 1.56, 0.64, 1), opacity 140ms ease;
  }
  .rail-btn:hover {
    background: color-mix(in oklch, var(--tone) 10%, var(--surface-hover));
    color: var(--fg);
  }
  .rail-btn:hover .rail-icon { transform: scale(1.18); opacity: 1; }
  .rail-btn[data-active="true"] {
    background: color-mix(in oklch, var(--tone) 14%, var(--surface));
    color: var(--fg);
  }
  .rail-btn[data-active="true"] .rail-icon { opacity: 1; }
  .rail-btn:active { transform: translateY(0.5px); }
  .label { flex: 1; opacity: 1; transition: opacity 140ms ease; }
  .count-pip {
    flex-shrink: 0;
    min-width: 18px; height: 16px;
    padding: 0 5px;
    font-size: 10px; font-weight: 600;
    background: var(--surface-hover);
    color: var(--fg-muted);
    border-radius: 8px;
    display: inline-flex; align-items: center; justify-content: center;
    transition: opacity 140ms ease;
  }
  .count-pip.danger {
    background: color-mix(in oklch, var(--danger) 22%, transparent);
    color: var(--danger);
  }

  .bottom {
    margin-top: auto;
    display: flex; flex-direction: column;
  }
  .update-pill {
    margin-bottom: 8px;
    display: flex; align-items: center; gap: 8px;
    padding: 8px 10px;
    background: color-mix(in oklch, var(--accent) 10%, transparent);
    border: 1px solid color-mix(in oklch, var(--accent) 35%, transparent);
    color: var(--fg);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font: inherit; font-size: var(--fs-xs);
    text-align: left;
    overflow: hidden;
    transition: background 120ms, border-color 120ms;
  }
  .update-pill:hover {
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    border-color: color-mix(in oklch, var(--accent) 55%, transparent);
  }
  .up-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 30%, transparent);
    flex-shrink: 0;
    animation: up-pulse 2s ease-in-out infinite;
  }
  @keyframes up-pulse {
    0%, 100% { box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 30%, transparent); }
    50%      { box-shadow: 0 0 0 5px color-mix(in oklch, var(--accent) 14%, transparent); }
  }
  .up-text { display: flex; flex-direction: column; min-width: 0; line-height: 1.2; }
  .up-l { color: var(--fg); }
  .up-v { color: var(--fg-muted); font-size: var(--fs-xs); }

  .qa {
    display: flex; flex-direction: column; gap: 6px;
    padding: 12px 0 2px;
    border-top: 1px solid var(--border);
  }
  .qa-label {
    padding: 0 6px 8px;
    color: var(--fg-faint);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
  }
  .qa-btn {
    --tone: var(--accent);
    display: inline-flex; align-items: center; gap: 10px;
    width: 100%; height: 38px;
    padding: 0 10px;
    background: color-mix(in oklch, var(--tone) 8%, var(--surface));
    color: var(--fg);
    border: 1px solid color-mix(in oklch, var(--tone) 28%, var(--border));
    border-radius: var(--radius-sm);
    font: inherit; font-size: var(--fs-sm); font-weight: 600;
    letter-spacing: 0.01em;
    cursor: pointer;
    transition: background 140ms ease, border-color 140ms ease, color 140ms ease, transform 100ms ease, box-shadow 140ms ease;
    position: relative;
    overflow: hidden;
    white-space: nowrap;
  }
  .qa-btn[data-tone="info"]   { --tone: var(--info); }
  .qa-btn[data-tone="warn"]   { --tone: var(--warn); }
  .qa-btn[data-tone="accent"] { --tone: var(--accent); }
  .qa-btn > span:not(.qa-spin):not(.qa-icon) { flex: 1; text-align: left; }
  .qa-icon {
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--tone);
    transition: transform 140ms ease;
    flex-shrink: 0;
  }
  .qa-btn:hover:not(:disabled) {
    background: color-mix(in oklch, var(--tone) 22%, var(--surface));
    border-color: color-mix(in oklch, var(--tone) 55%, var(--border));
    color: color-mix(in oklch, var(--tone) 90%, var(--fg));
    box-shadow: 0 4px 12px color-mix(in oklch, var(--tone) 18%, transparent);
  }
  .qa-btn:hover:not(:disabled) .qa-icon { transform: scale(1.1); }
  .qa-btn:active:not(:disabled) { transform: translateY(1px); box-shadow: none; }
  .qa-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .qa-btn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .qa-spin {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--tone);
    animation: qa-pulse 1.4s ease-in-out infinite;
    flex-shrink: 0;
  }
  @keyframes qa-pulse {
    0%, 100% { opacity: 0.4; transform: scale(0.85); }
    50%      { opacity: 1;   transform: scale(1.15); }
  }

  /* Collapsed state: hide labels, kbd hints, qa header, button text. */
  @container (max-width: 130px) {
    .label, .qa-label, .up-text { display: none; }
    .qa-btn > span:not(.qa-spin):not(.qa-icon) { display: none; }
    .rail-btn, .qa-btn { justify-content: center; padding: 0; gap: 0; }
    .update-pill { justify-content: center; padding: 8px; }
    .count-pip {
      position: absolute; top: 2px; right: 2px;
      min-width: 14px; height: 14px;
      padding: 0 4px;
      font-size: 9px;
      pointer-events: none;
    }
    .qa { padding-top: 8px; }
  }
</style>
