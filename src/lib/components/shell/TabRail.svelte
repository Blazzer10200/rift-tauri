<script lang="ts">
  import { FolderOpen, Activity, TriangleAlert, Cog, Download, DownloadCloud, UploadCloud } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { connection } from "../../state/connection.svelte";
  import { updates } from "../../state/updates.svelte";
  import { syncModal } from "../../state/sync-modal.svelte";

  type Tab = "browse" | "activity" | "conflicts" | "settings" | "diagnostics";

  let { active, onChange }: {
    active: Tab;
    onChange: (t: Tab) => void;
  } = $props();

  const tabs: { id: Tab; label: string; icon: typeof FolderOpen; kbd: string; count?: () => number; countCls?: string }[] = [
    { id: "browse",    label: "Browser",   icon: FolderOpen,    kbd: "1" },
    { id: "activity",  label: "Activity",  icon: Activity,      kbd: "2", count: () => connection.activityFeed.length, countCls: "" },
    { id: "conflicts", label: "Conflicts", icon: TriangleAlert, kbd: "3", count: () => connection.conflictCount, countCls: "danger" },
    { id: "settings",  label: "Settings",  icon: Cog,           kbd: "4" },
  ];

  const watcherOn = $derived(
    connection.status?.state === "watching" || connection.status?.state === "idle" || connection.status?.state === "syncing"
  );
  const canSync = $derived(watcherOn && !syncModal.open);
  const activeIdx = $derived(tabs.findIndex((t) => t.id === active));
  const indicatorVisible = $derived(activeIdx >= 0);

  let pulling = $state(false);
  let pushing = $state(false);

  async function pullAll() {
    if (!canSync || pulling) return;
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
    if (!canSync || pushing) return;
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

<div class="rail">
  <div class="group" style="--active-y: {Math.max(0, activeIdx) * 31}px">
    <div class="rail-indicator" aria-hidden="true" data-visible={indicatorVisible}></div>
    {#each tabs as t (t.id)}
      {@const Icon = t.icon}
      {@const c = t.count ? t.count() : 0}
      <button
        class="rail-btn"
        data-active={active === t.id}
        onclick={() => onChange(t.id)}
        title="{t.label} (Ctrl+{t.kbd})"
        type="button"
      >
        <Icon size={16}/>
        <span class="label">{t.label}</span>
        {#if c > 0}
          <span class="count-pip {t.countCls ?? ''}">{c}</span>
        {/if}
        <span class="rail-kbd kbd">⌘{t.kbd}</span>
      </button>
    {/each}
  </div>

  <div class="bottom">
    {#if updates.state === "available" && updates.info}
      <button
        class="update-pill"
        type="button"
        onclick={() => updates.open()}
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
        type="button"
        onclick={pullAll}
        disabled={!canSync || pulling}
        title={watcherOn ? "Pull all changes from remote" : "Connect a server first"}
      >
        <DownloadCloud size={14}/>
        <span>Pull all</span>
        {#if pulling}<span class="qa-spin"></span>{/if}
      </button>
      <button
        class="qa-btn"
        type="button"
        onclick={pushAll}
        disabled={!canSync || pushing}
        title={watcherOn ? "Push all local changes to remote" : "Connect a server first"}
      >
        <UploadCloud size={14}/>
        <span>Push all</span>
        {#if pushing}<span class="qa-spin"></span>{/if}
      </button>
    </div>
  </div>
</div>

<style>
  .rail {
    display: flex; flex-direction: column;
    background: var(--bg);
    border-right: 1px solid var(--border);
    padding: 8px;
    min-height: 0;
    width: 200px;
  }
  .group { display: flex; flex-direction: column; gap: 1px; position: relative; }
  .rail-indicator {
    position: absolute;
    left: 0; top: 0;
    width: 2px; height: 30px;
    background: var(--accent);
    border-radius: 2px;
    transform: translateY(var(--active-y, 0px));
    transition: transform 220ms cubic-bezier(0.4, 0, 0.2, 1), opacity 160ms ease;
    pointer-events: none;
    opacity: 0;
    z-index: 1;
  }
  .rail-indicator[data-visible="true"] { opacity: 1; }
  .rail-btn {
    display: flex; align-items: center; gap: 9px;
    width: 100%; height: 30px; padding: 0 10px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
    position: relative;
    transition: background 100ms ease, color 100ms ease;
  }
  .rail-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .rail-btn[data-active="true"] {
    background: var(--surface); color: var(--fg);
  }
  .label { flex: 1; }
  .rail-kbd {
    margin-left: auto;
    opacity: 0;
    transition: opacity 80ms;
  }
  .rail-btn:hover .rail-kbd { opacity: 1; }
  .rail-btn[data-active="true"] .rail-kbd { opacity: 0.7; }

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
    display: flex; flex-direction: column; gap: 4px;
    padding: 10px 4px 4px;
    border-top: 1px solid var(--border);
  }
  .qa-label {
    padding: 0 6px 4px;
    color: var(--fg-faint);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 500;
  }
  .qa-btn {
    display: inline-flex; align-items: center; gap: 9px;
    width: 100%; height: 32px; padding: 0 10px;
    background: var(--surface);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font: inherit; font-size: var(--fs-sm);
    cursor: pointer;
    transition: background 100ms ease, border-color 100ms ease, color 100ms ease, transform 100ms ease;
    position: relative;
  }
  .qa-btn > span:not(.qa-spin) { flex: 1; text-align: left; }
  .qa-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--accent) 35%, var(--border));
    color: var(--accent);
  }
  .qa-btn:active:not(:disabled) { transform: translateY(1px); }
  .qa-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .qa-btn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .qa-spin {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--accent);
    animation: qa-pulse 1.4s ease-in-out infinite;
    flex-shrink: 0;
  }
  @keyframes qa-pulse {
    0%, 100% { opacity: 0.4; transform: scale(0.85); }
    50%      { opacity: 1;   transform: scale(1.15); }
  }
</style>
