<script lang="ts">
  import { FolderOpen, Activity, GitPullRequestArrow, TriangleAlert, Cog } from "lucide-svelte";
  import { connection } from "../../state/connection.svelte";

  type Tab = "browse" | "activity" | "drift" | "conflicts" | "settings";

  let { active, onChange }: {
    active: Tab;
    onChange: (t: Tab) => void;
  } = $props();

  const tabs: { id: Tab; label: string; icon: typeof FolderOpen; kbd: string; count?: () => number; countCls?: string }[] = [
    { id: "browse",    label: "Browser",   icon: FolderOpen,           kbd: "1" },
    { id: "activity",  label: "Activity",  icon: Activity,             kbd: "2", count: () => connection.activityFeed.length, countCls: "" },
    { id: "drift",     label: "Drift",     icon: GitPullRequestArrow,  kbd: "3" },
    { id: "conflicts", label: "Conflicts", icon: TriangleAlert,        kbd: "4", count: () => connection.conflictCount, countCls: "danger" },
    { id: "settings",  label: "Settings",  icon: Cog,                  kbd: "5" },
  ];

  const watchingPath = $derived(connection.selected?.localRoot ?? "—");
  const autosyncOn = $derived(
    connection.status?.state === "watching" || connection.status?.state === "idle" || connection.status?.state === "syncing"
  );

  async function toggleAutoSync() {
    if (autosyncOn) {
      await connection.disconnect();
    } else {
      try { await connection.connect(); } catch (e) { console.error(e); }
    }
  }
</script>

<div class="rail">
  <div class="group">
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

  <div class="foot">
    <div class="stat">
      <span class="stat-label">Watching</span>
      <span class="stat-val mono">{watchingPath}</span>
    </div>
    <div class="stat">
      <span class="stat-label">Auto-sync</span>
      <button
        class="autosync-toggle"
        type="button"
        onclick={toggleAutoSync}
        disabled={!connection.selected || connection.connecting}
        title={autosyncOn ? "Click to stop auto-sync" : connection.connecting ? "Connecting…" : "Click to start auto-sync"}
      >
        {#if connection.connecting}
          <span class="pill info"><span class="dot"></span>connecting…</span>
        {:else if autosyncOn}
          <span class="pill ok"><span class="dot"></span>on</span>
        {:else}
          <span class="pill muted"><span class="dot"></span>off</span>
        {/if}
      </button>
    </div>
    <div class="stat">
      <span class="stat-label">Locks</span>
      <span class="stat-val mono">{connection.lockCount}</span>
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
  .group { display: flex; flex-direction: column; gap: 1px; }
  .rail-btn {
    display: flex; align-items: center; gap: 9px;
    width: 100%; height: 30px; padding: 0 10px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
    position: relative;
  }
  .rail-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .rail-btn[data-active="true"] {
    background: var(--surface); color: var(--fg);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .label { flex: 1; }
  .rail-kbd {
    margin-left: auto;
    opacity: 0;
    transition: opacity 80ms;
  }
  .rail-btn:hover .rail-kbd { opacity: 1; }
  .rail-btn[data-active="true"] .rail-kbd { opacity: 0.7; }

  .foot {
    margin-top: auto;
    display: flex; flex-direction: column; gap: 2px;
    padding: 8px 6px 4px;
    border-top: 1px solid var(--border);
    font-size: var(--fs-xs);
  }
  .stat { display: flex; justify-content: space-between; align-items: center; padding: 3px 4px; gap: 8px; min-width: 0; }
  .stat-label { color: var(--fg-subtle); white-space: nowrap; }
  .stat-val { color: var(--fg-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; max-width: 120px; }
  .pill { height: 18px; }
</style>
