<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    RefreshCw, DownloadCloud, UploadCloud, AlertTriangle,
    ChevronRight, CheckCircle2, CircleAlert, Inbox, History,
  } from "lucide-svelte";
  import { connection } from "../../state/connection.svelte";
  import { syncPage, type ResourceGroup, type DriftEntry } from "../../state/sync-page.svelte";

  let unlistenDiag: UnlistenFn | null = null;

  const watcherOn = $derived(
    connection.status?.state === "watching" ||
    connection.status?.state === "idle" ||
    connection.status?.state === "syncing"
  );
  const canSync = $derived(watcherOn && !syncPage.busy);
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
    void syncPage.refresh();
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
    if (unlistenDiag) unlistenDiag();
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

<section class="page">
  <header class="hero">
    <div class="hero-left">
      <h2>Sync</h2>
      {#if !watcherOn}
        <span class="pill muted"><CircleAlert size={11}/> Not connected</span>
      {:else if isEmpty}
        <span class="pill ok"><span class="dot"></span> Everything synced</span>
      {:else}
        <span class="pill info">
          {totals.total} pending · {groups.length} resource{groups.length === 1 ? "" : "s"}
        </span>
      {/if}
      <span class="meta" title="Last drift scan">
        Last scan <span class="mono">{scanAgeLabel()}</span>
      </span>
    </div>
    <div class="hero-right">
      <button
        class="btn ghost"
        type="button"
        onclick={() => syncPage.rescan()}
        disabled={!canSync}
        title="Re-scan both sides for drift"
      >
        <RefreshCw size={13} class={syncPage.busy ? "spin" : ""} />
        <span>Rescan</span>
      </button>
      <button
        class="btn info"
        type="button"
        onclick={() => syncPage.pullAll()}
        disabled={!canSync || totals.pull + totals.del === 0}
        title="Pull every ToPull + ToDelete entry"
      >
        <DownloadCloud size={13}/> Pull all
      </button>
      <button
        class="btn warn"
        type="button"
        onclick={() => syncPage.pushAll()}
        disabled={!canSync || totals.push === 0}
        title="Push every dirty + ToPush entry"
      >
        <UploadCloud size={13}/> Push all
      </button>
    </div>
  </header>

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
    <div class="shrink-banner" role="alert" in:fade={{ duration: 120 }}>
      <div class="shrink-head">
        <AlertTriangle size={13}/>
        <span class="shrink-msg">
          Listing shrink detected — <strong>{folder.resource_name}</strong>:
          baseline {folder.baseline_count} → scan {folder.listing_count}.
          Bracket aborted to prevent phantom deletes.
        </span>
        <span class="shrink-tip" title="Until rebaselined, new resources or files added inside this bracket will be invisible to Rift — they won't appear in the queue and won't sync to the server. Click Rebaseline if this shrink reflects an intentional cleanup; click Dismiss only if you expect the next scan to catch up.">
          Why this matters
        </span>
      </div>
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
      {:else}
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
                        <span class="entry-path mono" title={e.local_path}>{relPathLabel(e)}</span>
                        <span class="entry-reason" title={e.reason}>{e.reason}</span>
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
      <span class="foot-hint">
        {#if hasSelection}
          <Inbox size={11}/>
          <span>{selectionSize} item{selectionSize === 1 ? "" : "s"} queued · </span>
          <span class="foot-cta">Apply</span>
          <span> to dispatch</span>
        {:else if !isEmpty}
          <span>Pick rows below to queue them, or use</span>
          <span class="foot-cta">Pull all</span> <span>/</span> <span class="foot-cta">Push all</span>
          <span>above.</span>
        {/if}
      </span>
    </div>
  </footer>
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

  /* ── Hero ───────────────────────────────────────────── */
  .hero {
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    padding: 14px 18px 10px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .hero-left { display: flex; align-items: center; gap: 10px; min-width: 0; flex-wrap: wrap; }
  .hero h2 {
    margin: 0;
    font-size: var(--fs-lg);
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .meta {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }
  .hero-right { display: flex; gap: 6px; flex-wrap: wrap; }

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
  .shrink-head {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .shrink-msg { flex: 1; min-width: 240px; }
  .shrink-tip {
    font-size: var(--fs-xs);
    color: var(--warn);
    text-decoration: underline dotted;
    text-underline-offset: 2px;
    cursor: help;
    white-space: nowrap;
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
    grid-template-columns: 18px 60px minmax(0, 1fr) minmax(0, auto);
    gap: 10px;
    align-items: center;
    padding: 5px 4px;
    cursor: pointer;
    transition: background 100ms ease, box-shadow 100ms ease;
  }
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
  .entry-reason {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    justify-self: end;
    max-width: 240px;
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
</style>
