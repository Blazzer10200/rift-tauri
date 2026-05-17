<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import {
    RefreshCw, DownloadCloud, UploadCloud, AlertTriangle,
    ChevronRight, CheckCircle2, CircleAlert, History,
    Wrench, Trash2, Eye, EyeOff, MoreHorizontal, Check, RefreshCcw, Timer,
  } from "lucide-svelte";
  import PageHeader from "../shell/PageHeader.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import ConflictsPage from "../conflicts/ConflictsPage.svelte";

  // v0.2.53 Mirror typed-confirm gate. User must type "MIRROR" to enable
  // the Confirm button. Prevents accidental destructive remote deletes.
  let mirrorConfirmText = $state("");
  const MIRROR_CONFIRM_PHRASE = "MIRROR";
  // Track which shrink-banners are expanded. Default collapsed —
  // banner is dense and steals vertical space when several brackets fire.
  let shrinkExpanded = $state<Set<string>>(new Set());
  function toggleShrink(key: string) {
    const next = new Set(shrinkExpanded);
    if (next.has(key)) next.delete(key); else next.add(key);
    shrinkExpanded = next;
  }
  import { connection } from "../../state/connection.svelte";
  import { syncPage, type ResourceGroup, type DriftEntry } from "../../state/sync-page.svelte";

  let unlistenDiag: UnlistenFn | null = null;
  // v0.2.55 Phase A reskin: hero overflow kebab for low-frequency utilities
  // (Mirror toggle, Sweep stale locks, Preview). Click-outside closes.
  let overflowOpen = $state(false);
  let overflowRef = $state<HTMLDivElement | null>(null);

  function onDocMouseDown(ev: MouseEvent) {
    if (!overflowOpen) return;
    if (overflowRef && ev.target instanceof Node && !overflowRef.contains(ev.target)) {
      overflowOpen = false;
    }
  }
  function onDocKey(ev: KeyboardEvent) {
    if (overflowOpen && ev.key === "Escape") overflowOpen = false;
  }

  const watcherOn = $derived(
    connection.status?.state === "watching" ||
    connection.status?.state === "idle" ||
    connection.status?.state === "syncing"
  );
  const canSync = $derived(watcherOn && !syncPage.busy && !syncPage.previewMode);
  const totals = $derived(syncPage.totals);
  const groups = $derived(syncPage.groups);
  const selectionSize = $derived(syncPage.selected.size);
  const hasSelection = $derived(selectionSize > 0);
  const isEmpty = $derived(totals.total === 0);
  const bucketsActive = $derived(
    (totals.push > 0 ? 1 : 0) + (totals.pull > 0 ? 1 : 0) +
    (totals.del > 0 ? 1 : 0) + (totals.conf > 0 ? 1 : 0)
  );

  onMount(async () => {
    window.addEventListener("mousedown", onDocMouseDown);
    window.addEventListener("keydown", onDocKey);
    syncPage.loadAutoRescanPrefs();
    void syncPage.refresh();
    // v0.2.53: sync the Mirror toggle w/ backend state (it's session-scoped
    // on the engine; if a prior session left it on, surface that).
    try {
      const enabled = await invoke<boolean>("sync_get_mirror_mode");
      syncPage.mirrorEnabled = enabled;
    } catch { /* not connected — keep default false */ }
    try {
      unlistenDiag = await listen<{ stage: string }>("diag://event", (e) => {
        if (e.payload.stage === "drift_scan_result") {
          void syncPage.refresh();
        }
      });
    } catch (err) {
      console.error("SyncPage: failed to attach diag listener", err);
    }
  });

  onDestroy(() => {
    window.removeEventListener("mousedown", onDocMouseDown);
    window.removeEventListener("keydown", onDocKey);
    if (unlistenDiag) unlistenDiag();
  });

  // v0.2.55 Phase A: pretty-print bytes (B/KB/MB/GB, tabular-nums friendly).
  function formatSize(bytes: number): string {
    if (!bytes || bytes < 0) return "";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1048576).toFixed(1)} MB`;
    return `${(bytes / 1073741824).toFixed(2)} GB`;
  }

  // v0.2.55 Phase A: ISO mtime → "Ns/Nm/Nh/Nd ago".
  function formatMtimeRel(iso: string | null): string {
    if (!iso) return "";
    const t = new Date(iso).getTime();
    if (Number.isNaN(t)) return "";
    const diffSec = Math.floor((Date.now() - t) / 1000);
    if (diffSec < 0) return "future";
    if (diffSec < 60) return `${diffSec}s ago`;
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
    return `${Math.floor(diffSec / 86400)}d ago`;
  }

  // v0.2.55 Phase A: selection breakdown — count selected entries per bucket
  // so the active footer can show "2 push · 2 pull · 1 delete" inline.
  const selBreakdown = $derived.by(() => {
    let push = 0, pull = 0, del = 0, delRem = 0, conf = 0;
    if (syncPage.selected.size === 0) return { push, pull, del, delRem, conf };
    const byPath = new Map<string, DriftEntry>();
    for (const e of syncPage.entries) byPath.set(e.local_path, e);
    for (const path of syncPage.selected) {
      const e = byPath.get(path);
      if (!e) continue;
      if (e.bucket === "to_push") push++;
      else if (e.bucket === "to_pull") pull++;
      else if (e.bucket === "to_delete") del++;
      else if (e.bucket === "to_delete_remote") delRem++;
      else if (e.bucket === "conflict") conf++;
    }
    return { push, pull, del, delRem, conf };
  });

  function relPathLabel(e: DriftEntry): string {
    return e.rel_path || e.local_path;
  }

  function bucketLabel(b: string): string {
    if (b === "to_push") return "push";
    if (b === "to_pull") return "pull";
    if (b === "to_delete") return "delete";
    if (b === "conflict") return "conflict";
    return b;
  }

  function bucketTone(b: string): "push" | "pull" | "delete" | "conflict" | "" {
    if (b === "to_push") return "push";
    if (b === "to_pull") return "pull";
    if (b === "to_delete") return "delete";
    if (b === "conflict") return "conflict";
    return "";
  }

  function dominantTone(g: ResourceGroup): "push" | "pull" | "delete" | "conflict" {
    const cs = [
      { k: "conflict" as const, n: g.conflict.length },
      { k: "delete" as const,   n: g.to_delete.length },
      { k: "push" as const,     n: g.to_push.length },
      { k: "pull" as const,     n: g.to_pull.length },
    ];
    cs.sort((a, b) => b.n - a.n);
    return cs[0].k;
  }

  function groupSelectionState(g: ResourceGroup): "none" | "some" | "all" {
    const items = [...g.to_push, ...g.to_pull, ...g.to_delete];
    if (items.length === 0) return "none";
    let on = 0;
    for (const it of items) if (syncPage.selected.has(it.local_path)) on++;
    if (on === 0) return "none";
    if (on === items.length) return "all";
    return "some";
  }

  function toggleGroupSelect(g: ResourceGroup) {
    const st = groupSelectionState(g);
    if (st === "all") syncPage.clearSelectionIn(g);
    else syncPage.selectAllIn(g);
  }

  function selectedCountIn(g: ResourceGroup): number {
    let n = 0;
    for (const e of [...g.to_push, ...g.to_pull, ...g.to_delete]) {
      if (syncPage.selected.has(e.local_path)) n++;
    }
    return n;
  }

  function selectedDeletesIn(g: ResourceGroup): number {
    let n = 0;
    for (const e of g.to_delete) if (syncPage.selected.has(e.local_path)) n++;
    return n;
  }

  function deleteThresholdHint(g: ResourceGroup): number {
    const total = g.to_push.length + g.to_pull.length + g.to_delete.length + g.conflict.length;
    if (total <= 0) return 5;
    return Math.min(25, Math.max(5, Math.floor(total * 0.3)));
  }

  function scanAgeLabel(): string {
    if (!syncPage.scannedAt) return "never";
    const ageSec = Math.floor((Date.now() - syncPage.scannedAt) / 1000);
    if (ageSec < 60) return `${ageSec}s ago`;
    if (ageSec < 3600) return `${Math.floor(ageSec / 60)}m ago`;
    return `${Math.floor(ageSec / 3600)}h ago`;
  }
</script>

{#snippet headerPills()}
  {#if !watcherOn}
    <span class="pill muted"><CircleAlert size={11}/> Not connected</span>
  {:else if isEmpty}
    <span class="pill ok"><span class="dot"></span> Everything synced</span>
  {:else}
    <span class="pill info">
      {totals.total} pending · {groups.length} resource{groups.length === 1 ? "" : "s"}
    </span>
  {/if}
{/snippet}

{#snippet toolbarActions()}
      <div class="kebab-wrap" bind:this={overflowRef}>
        <button
          class="btn ghost icon"
          type="button"
          onclick={() => (overflowOpen = !overflowOpen)}
          aria-haspopup="menu"
          aria-expanded={overflowOpen}
          title="More options"
        >
          <MoreHorizontal size={14}/>
        </button>
        {#if overflowOpen}
          <div class="kebab-menu" role="menu" in:fade={{ duration: 80 }}>
            <button
              type="button"
              role="menuitem"
              class="kebab-item"
              class:active={syncPage.mirrorEnabled}
              disabled={!watcherOn || syncPage.busy || syncPage.previewMode}
              onclick={() => { syncPage.toggleMirror(!syncPage.mirrorEnabled); overflowOpen = false; }}
              title="Mirror mode: local-missing files surface as 'delete on remote' bucket instead of pull-restore. Destructive — opt-in only."
            >
              <span class="kebab-check">{#if syncPage.mirrorEnabled}<Check size={12}/>{/if}</span>
              <Trash2 size={13}/>
              <span>Mirror mode</span>
              {#if syncPage.mirrorEnabled}<span class="kebab-tag danger">on</span>{/if}
            </button>
            <button
              type="button"
              role="menuitem"
              class="kebab-item"
              class:active={syncPage.autoRescanEnabled}
              onclick={() => syncPage.cycleAutoRescan()}
              title="Periodic auto-rescan — catches remote-side drift (teammate pushes, out-of-band edits) the local watcher can't see. Click cycles: off → 30s → 1m → 2m → 5m → 10m → off."
            >
              <span class="kebab-check">{#if syncPage.autoRescanEnabled}<Check size={12}/>{/if}</span>
              <Timer size={13}/>
              <span>Auto-rescan</span>
              {#if syncPage.autoRescanEnabled}
                <span class="kebab-tag">{syncPage.autoRescanLabel}</span>
              {:else}
                <span class="kebab-tag muted">off</span>
              {/if}
            </button>
            <button
              type="button"
              role="menuitem"
              class="kebab-item"
              disabled={!canSync}
              onclick={() => { syncPage.sweepStaleLocks(); overflowOpen = false; }}
              title="Reclaim our own stale .rift-lock files across every watched root"
            >
              <span class="kebab-check"></span>
              <Wrench size={13}/>
              <span>Sweep stale locks</span>
            </button>
            <div class="kebab-divider"></div>
            <div class="kebab-section-label">Advanced</div>
            <button
              type="button"
              role="menuitem"
              class="kebab-item"
              disabled={!canSync || totals.pull + totals.del === 0}
              onclick={() => { syncPage.pullAll(); overflowOpen = false; }}
              title="Pull only — fetch every ToPull + ToDelete entry without pushing"
            >
              <span class="kebab-check"></span>
              <DownloadCloud size={13}/>
              <span>Pull only</span>
              {#if totals.pull + totals.del > 0}<span class="kebab-tag">{totals.pull + totals.del}</span>{/if}
            </button>
            <button
              type="button"
              role="menuitem"
              class="kebab-item"
              disabled={!canSync || totals.push === 0}
              onclick={() => { syncPage.pushAll(); overflowOpen = false; }}
              title="Push only — upload every dirty + ToPush entry without pulling first"
            >
              <span class="kebab-check"></span>
              <UploadCloud size={13}/>
              <span>Push only</span>
              {#if totals.push > 0}<span class="kebab-tag">{totals.push}</span>{/if}
            </button>
            <div class="kebab-divider"></div>
            <button
              type="button"
              role="menuitem"
              class="kebab-item"
              class:active={syncPage.previewMode}
              onclick={() => { syncPage.previewMode ? syncPage.exitPreview() : syncPage.enterPreview(); overflowOpen = false; }}
              title="Toggle design-preview fixture covering every bucket. Apply buttons are gated while active."
            >
              <span class="kebab-check">{#if syncPage.previewMode}<Check size={12}/>{/if}</span>
              {#if syncPage.previewMode}<EyeOff size={13}/>{:else}<Eye size={13}/>{/if}
              <span>{syncPage.previewMode ? "Exit preview" : "Design preview"}</span>
            </button>
          </div>
        {/if}
      </div>
      <button
        class="btn ghost icon"
        type="button"
        onclick={() => syncPage.rescan()}
        disabled={!canSync}
        title="Re-scan both sides for drift"
        aria-label="Rescan"
      >
        <RefreshCw size={13} class={syncPage.busy ? "spin" : ""} />
      </button>
      {#if syncPage.mirrorEnabled && totals.delRemote > 0}
        <button
          class="btn danger"
          type="button"
          onclick={() => syncPage.openMirrorConfirm()}
          disabled={!canSync}
          title="Delete {totals.delRemote} file(s)/folder(s) from remote — requires typed confirm"
        >
          <Trash2 size={13}/> Apply Mirror ({totals.delRemote})
        </button>
      {/if}
      <button
        class="btn primary sync-btn"
        type="button"
        onclick={() => syncPage.syncNow()}
        disabled={!canSync || (totals.push + totals.pull + totals.del === 0)}
        title="Pull then Push — canonical sync ordering. Pulls remote changes first so push never dispatches against a stale baseline. Conflicts and Mirror remote-deletes stay gated."
      >
        <RefreshCcw size={13} class={syncPage.syncPhase !== "idle" ? "spin" : ""}/>
        {#if syncPage.syncPhase === "pulling"}
          <span>Pulling… <span class="sync-sub">({totals.pull + totals.del})</span></span>
        {:else if syncPage.syncPhase === "pushing"}
          <span>Pushing… <span class="sync-sub">({totals.push})</span></span>
        {:else}
          <span>Sync</span>
          {#if totals.push + totals.pull + totals.del > 0}
            <span class="sync-sub">({totals.pull + totals.del}↓ {totals.push}↑)</span>
          {/if}
        {/if}
      </button>
{/snippet}

<section class="page" class:v03={uiPrefs.useV03Shell}>
  {#if !uiPrefs.useV03Shell}
    <PageHeader
      icon={RefreshCcw}
      title="Sync"
      tone={!watcherOn ? "neutral" : isEmpty ? "ok" : "info"}
      subtitle={watcherOn ? `Last scan ${scanAgeLabel()}` : "Not connected"}
    >
      {#snippet extras()}{@render headerPills()}{/snippet}
      {#snippet actions()}{@render toolbarActions()}{/snippet}
    </PageHeader>
  {:else}
    <div class="v03-toolbar" role="toolbar" aria-label="Sync actions">
      {@render toolbarActions()}
    </div>
  {/if}

  {#if uiPrefs.useV03Shell && connection.conflictCount > 0}
    <details class="conflicts-inline" open>
      <summary class="conflicts-inline-summary">
        <AlertTriangle size={12}/>
        <span class="conflicts-inline-label">
          {connection.conflictCount} conflict{connection.conflictCount === 1 ? "" : "s"}
        </span>
        <ChevronRight class="conflicts-inline-chev" size={12}/>
      </summary>
      <div class="conflicts-inline-body">
        <ConflictsPage />
      </div>
    </details>
  {/if}

  {#if syncPage.previewMode}
    <div class="banner preview" role="status" in:fade={{ duration: 120 }}>
      <Eye size={13}/>
      <span><strong>Preview mode</strong> — synthetic fixture, dispatch buttons gated. Use to review UI states + plan reskin.</span>
      <button class="banner-x" onclick={() => syncPage.exitPreview()} aria-label="Exit preview">×</button>
    </div>
  {/if}

  {#if syncPage.errorMsg}
    <div class="banner" role="alert" in:fade={{ duration: 120 }}>
      <AlertTriangle size={13}/>
      <span>{syncPage.errorMsg}</span>
    </div>
  {/if}

  {#if syncPage.lastRebaselineResult}
    <div class="banner ok" role="status" in:fade={{ duration: 120 }}>
      <CheckCircle2 size={13}/>
      <span>
        Rebaselined <strong>{syncPage.lastRebaselineResult.resource}</strong> —
        snapshot {syncPage.lastRebaselineResult.oldCount} → {syncPage.lastRebaselineResult.newCount},
        {syncPage.lastRebaselineResult.localOnly} local-only file{syncPage.lastRebaselineResult.localOnly === 1 ? "" : "s"} now queued for Push.
      </span>
      <button class="banner-x" onclick={() => (syncPage.lastRebaselineResult = null)} aria-label="Dismiss">×</button>
    </div>
  {/if}

  {#each syncPage.visibleAbortedShrunk as folder (folder.remote_root)}
    {@const isConfirming = syncPage.confirmingRebaseline === folder.remote_root}
    {@const isBusy = syncPage.rebaselining.has(folder.remote_root)}
    {@const isOpen = shrinkExpanded.has(folder.remote_root) || isConfirming}
    <div class="shrink-banner" class:collapsed={!isOpen} role="alert" in:fade={{ duration: 120 }}>
      <button class="shrink-head" type="button" onclick={() => toggleShrink(folder.remote_root)} aria-expanded={isOpen}>
        <AlertTriangle size={13}/>
        <span class="shrink-msg">
          <strong>{folder.resource_name}</strong>
          <span class="shrink-counts mono">{folder.baseline_count} → {folder.listing_count}</span>
          {#if isOpen}<span class="shrink-detail">— bracket aborted to prevent phantom deletes.</span>{/if}
        </span>
        <ChevronRight class="shrink-chev" size={12}/>
      </button>
      {#if isOpen}
      <div class="shrink-explain" in:fade={{ duration: 100 }}>
        <span class="shrink-tip" title="Until rebaselined, new resources or files added inside this bracket will be invisible to Rift — they won't appear in the queue and won't sync to the server.">
          Until rebaselined, new files inside this bracket won't sync. Rebaseline if this shrink was intentional; Dismiss only if you expect the next scan to catch up.
        </span>
      </div>
      {/if}
      {#if isConfirming}
        <div class="shrink-confirm" in:fade={{ duration: 100 }}>
          <span>
            Replace snapshot rows for <strong>{folder.resource_name}</strong> with current ground truth.
            Local-only files will queue for Push on next reconcile. Re-hashes every file from disk — slower for large brackets, exact for correctness.
          </span>
          <div class="shrink-actions">
            <button class="primary" disabled={isBusy} onclick={() => syncPage.confirmRebaseline()}>
              {isBusy ? "Rebaselining…" : "Confirm Rebaseline"}
            </button>
            <button onclick={() => syncPage.cancelRebaseline()} disabled={isBusy}>Cancel</button>
          </div>
        </div>
      {:else if isOpen}
        <div class="shrink-actions">
          <button class="primary" disabled={isBusy} onclick={() => syncPage.requestRebaseline(folder.remote_root)}>
            <History size={12}/> Rebaseline
          </button>
          <button onclick={() => syncPage.dismissShrunk(folder.remote_root)}>Dismiss</button>
        </div>
      {/if}
    </div>
  {/each}

  {#if !isEmpty && bucketsActive > 1}
    <div class="totals" in:fade={{ duration: 140 }}>
      {#if totals.push > 0}
        <span class="tot" data-tone="push"><span class="n">{totals.push}</span><span class="l">push</span></span>
      {/if}
      {#if totals.pull > 0}
        <span class="tot" data-tone="pull"><span class="n">{totals.pull}</span><span class="l">pull</span></span>
      {/if}
      {#if totals.del > 0}
        <span class="tot" data-tone="delete"><span class="n">{totals.del}</span><span class="l">delete</span></span>
      {/if}
      {#if totals.conf > 0}
        <span class="tot" data-tone="conflict"><span class="n">{totals.conf}</span><span class="l">conflict</span></span>
      {/if}
    </div>
  {/if}

  <div class="body">
    {#if isEmpty}
      <div class="empty" in:fade={{ duration: 160 }}>
        {#if !watcherOn}
          <div class="empty-icon muted"><CircleAlert size={28}/></div>
          <div class="empty-title">Not connected</div>
          <p class="empty-hint">Connect a server first, then rescan to see pending drift.</p>
        {:else if syncPage.loading}
          <div class="empty-icon muted"><span class="spin big"></span></div>
          <div class="empty-title">Scanning…</div>
          <p class="empty-hint">Reading both sides — hang tight.</p>
        {:else}
          <div class="empty-icon ok"><CheckCircle2 size={28}/></div>
          <div class="empty-title">Everything in sync</div>
          <p class="empty-hint">Both sides match. Local edits will appear here as drift.</p>
          <div class="empty-sub">
            <span>Last scan <span class="mono">{scanAgeLabel()}</span></span>
            {#if connection.status?.watches}
              <span class="dotsep">·</span>
              <span>{connection.status.watches} folder{connection.status.watches === 1 ? "" : "s"} watched</span>
            {/if}
          </div>
          <button
            class="btn ghost sm empty-action"
            type="button"
            onclick={() => syncPage.rescan()}
            disabled={!canSync}
          >
            <RefreshCw size={12} class={syncPage.busy ? "spin" : ""}/>
            <span>Rescan now</span>
          </button>
        {/if}
      </div>
    {:else}
      <ul class="resources">
        {#each groups as g, i (g.resource)}
          {@const isOpen = syncPage.expanded.has(g.resource)}
          {@const selState = groupSelectionState(g)}
          {@const selCount = selectedCountIn(g)}
          {@const delsel = selectedDeletesIn(g)}
          {@const delThresh = deleteThresholdHint(g)}
          {@const tone = dominantTone(g)}
          <li
            class="resource"
            data-tone={tone}
            class:open={isOpen}
            in:fly={{ y: 4, duration: 160, delay: Math.min(i, 8) * 20, easing: quintOut }}
          >
            <button
              class="res-header"
              type="button"
              onclick={() => syncPage.toggleExpanded(g.resource)}
              aria-expanded={isOpen}
            >
              <span class="chev" class:rot={isOpen}><ChevronRight size={13}/></span>
              <span class="res-name mono" title={g.resource}>{g.resource}</span>
              <span class="res-counts">
                {#if g.to_push.length > 0}<span class="pip" data-tone="push">{g.to_push.length} push</span>{/if}
                {#if g.to_pull.length > 0}<span class="pip" data-tone="pull">{g.to_pull.length} pull</span>{/if}
                {#if g.to_delete.length > 0}<span class="pip" data-tone="delete">{g.to_delete.length} delete</span>{/if}
                {#if g.conflict.length > 0}<span class="pip" data-tone="conflict">{g.conflict.length} conflict</span>{/if}
              </span>
              {#if selCount > 0}
                <span class="sel-badge">{selCount} selected</span>
              {/if}
            </button>

            {#if isOpen}
              <div class="res-body" in:fly={{ y: -2, duration: 140, easing: quintOut }}>
                <div class="res-toolbar">
                  <label class="chk">
                    <input
                      type="checkbox"
                      checked={selState === "all"}
                      indeterminate={selState === "some"}
                      onchange={() => toggleGroupSelect(g)}
                    />
                    <span>Select all in <span class="mono">{g.resource}</span></span>
                  </label>
                  {#if g.to_delete.length >= delThresh}
                    <span class="guard-warn" title="Threshold scales as 30% of total files (clamped 5-25). Explicit user selection bypasses the breaker, but the action is logged to the activity feed.">
                      <AlertTriangle size={11}/>
                      {#if delsel > 0}
                        {delsel}/{g.to_delete.length} deletes selected · over guard ({delThresh}) — will warn
                      {:else}
                        {g.to_delete.length} deletes exceed guard ({delThresh}) — apply will warn but proceed
                      {/if}
                    </span>
                  {/if}
                </div>

                <ul class="entries">
                  {#each [...g.to_push, ...g.to_pull, ...g.to_delete, ...g.conflict] as e, j (e.local_path)}
                    {@const etone = bucketTone(e.bucket)}
                    {@const interactive = e.bucket !== "conflict"}
                    {@const sizeBytes = e.bucket === "to_pull" ? e.remote_size : e.local_size}
                    {@const mtimeIso = e.bucket === "to_pull" ? e.remote_mtime : e.local_mtime}
                    <li
                      class="entry"
                      data-tone={etone}
                      in:fade={{ duration: 140, delay: Math.min(j, 12) * 8 }}
                    >
                      <label class="entry-row" class:disabled={!interactive}>
                        <input
                          type="checkbox"
                          checked={syncPage.selected.has(e.local_path)}
                          onchange={() => syncPage.toggleSelected(e.local_path)}
                          disabled={!interactive}
                        />
                        <span class="entry-bucket" data-tone={etone}>{bucketLabel(e.bucket)}</span>
                        <div class="entry-main">
                          <div class="entry-line-1">
                            <span class="entry-path mono" title={e.local_path}>{relPathLabel(e)}</span>
                            {#if sizeBytes > 0}<span class="entry-meta-right">{formatSize(sizeBytes)}</span>{/if}
                          </div>
                          <div class="entry-line-2">
                            <span class="entry-reason" title={e.reason}>{e.reason}</span>
                            {#if mtimeIso}<span class="entry-meta-right muted">{formatMtimeRel(mtimeIso)}</span>{/if}
                          </div>
                        </div>
                      </label>
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if !isEmpty}
  <footer class="footer" class:active={hasSelection}>
    <div class="footer-left">
      <button
        class="btn primary"
        type="button"
        onclick={() => syncPage.applySelected()}
        disabled={!canSync || !hasSelection}
      >
        {hasSelection ? `Apply ${selectionSize} selected` : "Apply selected"}
      </button>
      {#if hasSelection}
        <button
          class="btn ghost sm"
          type="button"
          onclick={() => syncPage.clearAllSelection()}
          disabled={syncPage.busy}
        >Clear</button>
      {/if}
    </div>
    <div class="footer-right">
      {#if hasSelection}
        <span class="sel-breakdown" in:fade={{ duration: 120 }}>
          {#if selBreakdown.push > 0}<span class="br" data-tone="push">{selBreakdown.push} push</span>{/if}
          {#if selBreakdown.pull > 0}<span class="br" data-tone="pull">{selBreakdown.pull} pull</span>{/if}
          {#if selBreakdown.del > 0}<span class="br" data-tone="delete">{selBreakdown.del} delete</span>{/if}
          {#if selBreakdown.delRem > 0}<span class="br" data-tone="delete">{selBreakdown.delRem} remote-delete</span>{/if}
        </span>
      {:else if !isEmpty}
        <span class="foot-hint">
          <span>Pick rows below to queue them, or use</span>
          <span class="foot-cta">Pull all</span> <span>/</span> <span class="foot-cta">Push all</span>
          <span>above.</span>
        </span>
      {/if}
    </div>
  </footer>
  {/if}

  {#if syncPage.mirrorConfirm}
    <div
      class="modal-backdrop"
      role="presentation"
      onclick={() => { syncPage.cancelMirrorConfirm(); mirrorConfirmText = ""; }}
      onkeydown={(e) => { if (e.key === "Escape") { syncPage.cancelMirrorConfirm(); mirrorConfirmText = ""; } }}
    >
      <div
        class="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="mirror-confirm-title"
        tabindex="-1"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <header class="modal-head">
          <AlertTriangle size={16}/>
          <h2 id="mirror-confirm-title">Mirror — Delete on remote</h2>
        </header>
        <div class="modal-body">
          <p>
            This will delete <strong>{syncPage.mirrorConfirm.count}</strong> file{syncPage.mirrorConfirm.count === 1 ? "" : "s"}/folder{syncPage.mirrorConfirm.count === 1 ? "" : "s"} from <strong>remote</strong>. Local is already missing them.
          </p>
          <p class="caution">
            This action is <strong>irreversible</strong>. Make sure all teammates are on a synced baseline before proceeding — anyone running an older Rift install will see these files re-appear after their next push.
          </p>
          <label class="confirm-input">
            Type <code>{MIRROR_CONFIRM_PHRASE}</code> to confirm:
            <input
              type="text"
              bind:value={mirrorConfirmText}
              placeholder={MIRROR_CONFIRM_PHRASE}
              autocomplete="off"
              spellcheck="false"
            />
          </label>
        </div>
        <footer class="modal-foot">
          <button
            class="btn ghost"
            type="button"
            onclick={() => { syncPage.cancelMirrorConfirm(); mirrorConfirmText = ""; }}
          >Cancel</button>
          <button
            class="btn danger"
            type="button"
            disabled={mirrorConfirmText !== MIRROR_CONFIRM_PHRASE || syncPage.busy}
            onclick={() => { void syncPage.confirmMirrorApply(); mirrorConfirmText = ""; }}
          >Confirm — Delete {syncPage.mirrorConfirm.count} on remote</button>
        </footer>
      </div>
    </div>
  {/if}
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    background: var(--bg);
    color: var(--fg);
  }

  /* v0.2.53 Mirror confirm modal (toggle moved into kebab v0.2.55) */
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000; padding: 20px;
  }
  .modal {
    background: var(--bg); color: var(--fg); border: 1px solid var(--danger);
    border-radius: 10px; max-width: 540px; width: 100%;
    box-shadow: 0 24px 60px rgba(0,0,0,0.5);
  }
  .modal-head {
    display: flex; align-items: center; gap: 8px;
    padding: 14px 18px; border-bottom: 1px solid var(--border);
    color: var(--danger);
  }
  .modal-head h2 { margin: 0; font-size: var(--fs-md); }
  .modal-body { padding: 16px 18px; display: flex; flex-direction: column; gap: 12px; }
  .modal-body p { margin: 0; line-height: 1.5; }
  .modal-body .caution { color: var(--warn); font-size: var(--fs-sm); }
  .modal-body code {
    background: var(--bg-elevated); padding: 1px 6px; border-radius: 4px;
    border: 1px solid var(--border); font-family: var(--font-mono); font-size: 0.95em;
  }
  .confirm-input { display: flex; flex-direction: column; gap: 6px; font-size: var(--fs-sm); }
  .confirm-input input {
    padding: 6px 10px; background: var(--bg-elevated); color: var(--fg);
    border: 1px solid var(--border); border-radius: 6px; font-family: var(--font-mono);
  }
  .confirm-input input:focus { outline: none; border-color: var(--danger); }
  .modal-foot {
    display: flex; justify-content: flex-end; gap: 8px;
    padding: 12px 18px; border-top: 1px solid var(--border);
  }

  /* ── Banner ─────────────────────────────────────────── */
  .banner {
    margin: 8px 18px 0;
    padding: 7px 10px;
    border-radius: var(--radius);
    font-size: var(--fs-sm);
    display: flex; align-items: center; gap: 8px;
    background: var(--danger-soft);
    color: var(--danger);
    border: 1px solid color-mix(in oklch, var(--danger) 30%, transparent);
  }

  .banner.ok {
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: var(--fg);
    border-color: color-mix(in oklch, var(--accent) 30%, transparent);
  }
  .banner.preview {
    background: color-mix(in oklch, var(--info) 12%, transparent);
    color: var(--fg);
    border-color: color-mix(in oklch, var(--info) 32%, transparent);
  }
  .banner-x {
    margin-left: auto;
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    padding: 0 4px;
    font-size: 16px;
    line-height: 1;
  }
  .banner-x:hover { color: var(--fg); }

  /* ── Suspicious-shrink banner (v0.2.49) ─────────────── */
  .shrink-banner {
    margin: 6px 18px 0;
    padding: 8px 10px;
    border-radius: var(--radius);
    font-size: var(--fs-sm);
    background: var(--warn-soft);
    color: var(--fg);
    border: 1px solid color-mix(in oklch, var(--warn) 32%, transparent);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .shrink-banner.collapsed { padding: 4px 10px; gap: 0; }
  .shrink-head {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    width: 100%;
    background: transparent;
    border: 0;
    color: inherit;
    padding: 0;
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
  }
  .shrink-head :global(.shrink-chev) {
    margin-left: auto;
    color: var(--warn);
    transition: transform 180ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .shrink-banner:not(.collapsed) .shrink-head :global(.shrink-chev) {
    transform: rotate(90deg);
  }
  .shrink-msg { flex: 1; min-width: 240px; display: inline-flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .shrink-counts {
    font-size: var(--fs-xs);
    color: var(--warn);
    background: color-mix(in oklch, var(--warn) 18%, transparent);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
  }
  .shrink-detail { color: var(--fg-muted); font-size: var(--fs-xs); }
  .shrink-explain { font-size: var(--fs-xs); color: var(--fg-muted); line-height: 1.45; }
  .shrink-tip {
    color: var(--warn);
    cursor: help;
  }
  .shrink-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .shrink-actions button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elev-1);
    color: var(--fg);
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .shrink-actions button:hover:not(:disabled) {
    background: var(--bg-elev-2);
  }
  .shrink-actions button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .shrink-actions button.primary {
    background: var(--warn);
    border-color: var(--warn);
    color: var(--bg);
  }
  .shrink-actions button.primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .shrink-confirm {
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: color-mix(in oklch, var(--warn) 14%, var(--bg-elev-1));
    border: 1px solid color-mix(in oklch, var(--warn) 24%, transparent);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* ── Totals strip (hide-when-zero) ──────────────────── */
  .totals {
    display: flex; flex-wrap: wrap; gap: 6px;
    padding: 10px 18px 4px;
  }
  .tot {
    display: inline-flex; align-items: baseline; gap: 6px;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elev-1);
    font-size: var(--fs-sm);
  }
  .tot .n { font-weight: 600; font-variant-numeric: tabular-nums; color: var(--fg); }
  .tot .l { color: var(--fg-muted); font-size: var(--fs-xs); text-transform: uppercase; letter-spacing: 0.5px; }
  .tot[data-tone="push"]     { background: var(--accent-soft); border-color: color-mix(in oklch, var(--accent) 28%, transparent); }
  .tot[data-tone="push"] .n  { color: var(--accent); }
  .tot[data-tone="pull"]     { background: var(--info-soft);   border-color: color-mix(in oklch, var(--info) 28%, transparent); }
  .tot[data-tone="pull"] .n  { color: var(--info); }
  .tot[data-tone="delete"]   { background: var(--danger-soft); border-color: color-mix(in oklch, var(--danger) 28%, transparent); }
  .tot[data-tone="delete"] .n{ color: var(--danger); }
  .tot[data-tone="conflict"]   { background: var(--warn-soft);  border-color: color-mix(in oklch, var(--warn) 28%, transparent); }
  .tot[data-tone="conflict"] .n{ color: var(--warn); }

  /* ── Body / empty ───────────────────────────────────── */
  .body {
    flex: 1; min-height: 0;
    overflow: auto;
    padding: 10px 18px 14px;
  }
  .empty {
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    text-align: center;
    padding: 56px 18px;
    gap: 8px;
  }
  .empty-icon {
    width: 48px; height: 48px;
    border-radius: 50%;
    display: inline-flex; align-items: center; justify-content: center;
    margin-bottom: 4px;
  }
  .empty-icon.ok    { background: var(--ok-soft);    color: var(--ok); }
  .empty-icon.muted { background: var(--bg-elev-2); color: var(--fg-muted); }
  .empty-title { font-size: var(--fs-md); font-weight: 600; color: var(--fg); }
  .empty-hint { margin: 0; color: var(--fg-muted); font-size: var(--fs-sm); max-width: 380px; }

  /* ── Resource cards ─────────────────────────────────── */
  .resources { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .resource {
    --tone: var(--accent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elev-1);
    overflow: hidden;
    transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }
  .resource[data-tone="push"]     { --tone: var(--accent); }
  .resource[data-tone="pull"]     { --tone: var(--info); }
  .resource[data-tone="delete"]   { --tone: var(--danger); }
  .resource[data-tone="conflict"] { --tone: var(--warn); }
  .resource:hover {
    border-color: color-mix(in oklch, var(--tone) 35%, var(--border));
    background: color-mix(in oklch, var(--tone) 6%, var(--bg-elev-1));
  }
  .resource.open {
    border-color: color-mix(in oklch, var(--tone) 28%, var(--border));
    background: var(--bg-elev-1);
    box-shadow: inset 2px 0 var(--tone);
  }

  .res-header {
    display: flex; align-items: center; gap: 10px;
    width: 100%;
    padding: 9px 12px;
    background: transparent;
    border: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .res-header:focus-visible { outline: none; box-shadow: inset 0 0 0 2px var(--ring); }

  .chev {
    display: inline-flex;
    color: var(--fg-faint);
    transition: transform 140ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .chev.rot { transform: rotate(90deg); color: var(--tone); }
  @media (prefers-reduced-motion: reduce) {
    .chev { transition: none; }
  }

  .res-name {
    font-weight: 600;
    font-size: var(--fs-md);
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 280px;
  }
  .res-counts { display: flex; gap: 5px; margin-left: auto; align-items: center; flex-wrap: wrap; }

  .pip {
    display: inline-flex; align-items: center;
    font-size: var(--fs-xs);
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    border: 1px solid transparent;
  }
  .pip[data-tone="push"]     { background: var(--accent-soft); color: var(--accent); border-color: color-mix(in oklch, var(--accent) 22%, transparent); }
  .pip[data-tone="pull"]     { background: var(--info-soft);   color: var(--info);   border-color: color-mix(in oklch, var(--info) 22%, transparent); }
  .pip[data-tone="delete"]   { background: var(--danger-soft); color: var(--danger); border-color: color-mix(in oklch, var(--danger) 22%, transparent); }
  .pip[data-tone="conflict"] { background: var(--warn-soft);   color: var(--warn);   border-color: color-mix(in oklch, var(--warn) 22%, transparent); }

  .sel-badge {
    margin-left: 6px;
    font-size: var(--fs-xs);
    color: var(--accent);
    padding: 1px 7px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklch, var(--accent) 30%, transparent);
    border-radius: 999px;
    font-weight: 500;
  }

  /* ── Resource body ──────────────────────────────────── */
  .res-body {
    padding: 0 12px 10px;
    border-top: 1px solid color-mix(in oklch, var(--tone) 20%, var(--border));
  }
  .res-toolbar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 0 6px;
    font-size: var(--fs-sm);
    gap: 8px;
    flex-wrap: wrap;
  }
  .chk {
    display: inline-flex; align-items: center; gap: 7px;
    color: var(--fg-muted);
    cursor: pointer;
  }
  .chk input { accent-color: var(--tone); }
  .guard-warn {
    display: inline-flex; align-items: center; gap: 5px;
    color: var(--warn);
    background: var(--warn-soft);
    border: 1px solid color-mix(in oklch, var(--warn) 30%, transparent);
    padding: 2px 7px;
    border-radius: 999px;
    font-size: var(--fs-xs);
  }

  /* ── Entries ────────────────────────────────────────── */
  .entries { list-style: none; margin: 0; padding: 0; }
  .entry {
    --etone: var(--fg-muted);
    border-top: 1px solid var(--border);
  }
  .entry[data-tone="push"]     { --etone: var(--accent); }
  .entry[data-tone="pull"]     { --etone: var(--info); }
  .entry[data-tone="delete"]   { --etone: var(--danger); }
  .entry[data-tone="conflict"] { --etone: var(--warn); }

  .entry-row {
    display: grid;
    grid-template-columns: 18px 60px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    padding: 7px 4px;
    cursor: pointer;
    transition: background 100ms ease, box-shadow 100ms ease;
  }
  .entry-row input[type="checkbox"] { margin-top: 3px; }
  .entry-row .entry-bucket { margin-top: 2px; }
  .entry-main {
    display: flex; flex-direction: column; gap: 2px; min-width: 0;
  }
  .entry-line-1, .entry-line-2 {
    display: flex; align-items: center; gap: 8px; min-width: 0;
  }
  .entry-line-1 .entry-path { flex: 1; min-width: 0; }
  .entry-line-2 .entry-reason {
    flex: 1; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }
  .entry-meta-right {
    margin-left: auto; flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    color: var(--fg-muted);
  }
  .entry-meta-right.muted { color: var(--fg-faint); }
  .entry-row:hover {
    background: color-mix(in oklch, var(--etone) 8%, transparent);
    box-shadow: inset 2px 0 var(--etone);
  }
  .entry-row.disabled { cursor: default; color: var(--fg-faint); }
  .entry-row input { accent-color: var(--etone); }

  .entry-bucket {
    font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px;
    padding: 1px 6px; border-radius: var(--radius-xs);
    background: var(--bg-elev-2);
    text-align: center;
    font-weight: 600;
    color: var(--fg-muted);
  }
  .entry-bucket[data-tone="push"]     { background: var(--accent-soft); color: var(--accent); }
  .entry-bucket[data-tone="pull"]     { background: var(--info-soft);   color: var(--info); }
  .entry-bucket[data-tone="delete"]   { background: var(--danger-soft); color: var(--danger); }
  .entry-bucket[data-tone="conflict"] { background: var(--warn-soft);   color: var(--warn); }

  .entry-path {
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg);
  }

  /* ── Footer ─────────────────────────────────────────── */
  .footer {
    display: flex; justify-content: space-between; align-items: center; gap: 8px;
    padding: 10px 18px;
    border-top: 1px solid var(--border);
    background: var(--bg);
    transition: background 140ms ease, border-color 140ms ease;
  }
  .footer.active {
    background: color-mix(in oklch, var(--accent) 5%, var(--bg));
    border-top-color: color-mix(in oklch, var(--accent) 25%, var(--border));
  }
  .footer-left, .footer-right { display: flex; gap: 6px; align-items: center; }
  .foot-hint {
    display: inline-flex; align-items: center; gap: 4px;
    color: var(--fg-faint);
    font-size: var(--fs-xs);
    flex-wrap: wrap;
  }
  .foot-cta {
    color: var(--fg-2);
    font-weight: 500;
  }

  .spin.big { width: 22px; height: 22px; border-width: 3px; border: 3px solid currentColor; border-right-color: transparent; border-radius: 50%; display: inline-block; }

  /* ── v0.2.55 Phase A: overflow kebab ──────────────────── */
  .kebab-wrap { position: relative; display: inline-flex; }
  .btn.icon {
    padding: 4px 6px;
    min-width: 0;
  }
  .kebab-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    min-width: 260px;
    width: max-content;
    max-width: 360px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.42);
    z-index: 100;
    padding: 4px;
    display: flex; flex-direction: column; gap: 1px;
  }
  .kebab-item {
    display: flex; align-items: center; gap: 8px;
    padding: 7px 10px;
    background: transparent;
    border: 0;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    border-radius: var(--radius-sm);
    cursor: pointer;
    width: 100%;
    white-space: nowrap;
  }
  .kebab-item > span:not(.kebab-check):not(.kebab-tag) {
    flex: 1;
    min-width: 0;
  }
  .kebab-item:hover:not(:disabled) { background: var(--bg-elev-2); }
  .kebab-item:disabled { opacity: 0.45; cursor: not-allowed; }
  .kebab-item.active { color: var(--accent); }
  .kebab-check {
    display: inline-flex;
    width: 12px; height: 12px;
    align-items: center; justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }
  .kebab-tag {
    margin-left: auto;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    color: var(--fg-muted);
  }
  .kebab-tag.danger { background: var(--danger-soft); color: var(--danger); }
  .kebab-tag.muted { opacity: 0.55; }
  .kebab-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 2px;
  }
  .kebab-section-label {
    padding: 4px 10px 2px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--fg-faint);
  }

  /* Combined sync button — primary CTA, replaces dual Pull/Push (v0.2.55) */
  .sync-btn { gap: 6px; }
  .sync-sub {
    font-size: var(--fs-xs);
    color: color-mix(in oklch, currentColor 70%, transparent);
    font-variant-numeric: tabular-nums;
    margin-left: 2px;
  }

  /* ── v0.2.55 Phase A: selection breakdown in footer ───── */
  .sel-breakdown {
    display: inline-flex; gap: 10px; align-items: center;
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  .sel-breakdown .br {
    color: var(--fg-muted);
    font-weight: 500;
  }
  .sel-breakdown .br[data-tone="push"]   { color: var(--accent); }
  .sel-breakdown .br[data-tone="pull"]   { color: var(--info); }
  .sel-breakdown .br[data-tone="delete"] { color: var(--danger); }

  /* ── v0.2.55 Phase A: empty-state subtitle + action ───── */
  .empty-sub {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }
  .empty-sub .dotsep { opacity: 0.6; }
  .empty-action {
    margin-top: 10px;
  }

  /* ── v0.3 panel-shell mode (flag-on) ─────────────────── */
  .page.v03 .body { padding: 8px 12px 10px; }
  .page.v03 .totals { padding: 8px 12px 4px; }
  .page.v03 .banner { margin: 6px 12px 0; }
  .page.v03 .shrink-banner { margin: 4px 12px 0; }
  .page.v03 .footer { padding: 8px 12px; }

  .v03-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
  }

  .conflicts-inline {
    margin: 6px 12px 0;
    border: 1px solid color-mix(in oklch, var(--warn) 32%, transparent);
    border-radius: var(--radius);
    background: var(--warn-soft);
    overflow: hidden;
  }
  .conflicts-inline-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    cursor: pointer;
    list-style: none;
    color: var(--warn);
    font-size: var(--fs-sm);
    user-select: none;
  }
  .conflicts-inline-summary::-webkit-details-marker { display: none; }
  .conflicts-inline-label { font-weight: 500; }
  .conflicts-inline-summary :global(.conflicts-inline-chev) {
    margin-left: auto;
    transition: transform 140ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .conflicts-inline[open] .conflicts-inline-summary :global(.conflicts-inline-chev) {
    transform: rotate(90deg);
  }
  .conflicts-inline-body {
    border-top: 1px solid color-mix(in oklch, var(--warn) 24%, transparent);
    background: var(--bg);
    /* ConflictsPage owns flex column + scroll — cap height so nested
       scroll doesn't push the rest of the sync body off-screen. */
    max-height: 320px;
    overflow: auto;
    display: flex;
    flex-direction: column;
  }

</style>
