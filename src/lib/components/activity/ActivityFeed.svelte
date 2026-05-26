<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import {
    RefreshCw, Download, Trash2, AlertTriangle, Check,
    GitBranch, Network, Lock, XCircle, Info, Pause, Play,
    ChevronRight, ChevronDown, Copy, Folder, ExternalLink, Activity as ActivityIcon,
    Stethoscope,
  } from "lucide-svelte";
  import { connection, type ActivityRow, type ActivityKind } from "../../state/connection.svelte";
  import { diagnostics } from "../../state/diagnostics.svelte";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { fmtRelative } from "../../utils/time";
  import PageToolbar from "../shell/PageToolbar.svelte";
  import PageHeader from "../shell/PageHeader.svelte";
  import EmptyState from "../shell/EmptyState.svelte";
  import Diagnostics from "../diagnostics/Diagnostics.svelte";
  import { syncPage } from "../../state/sync-page.svelte";

  type Tab = "activity" | "diagnostics";
  let tab = $state<Tab>("activity");

  type Group = "all" | "sync" | "pull" | "delete" | "drift" | "conflict" | "bridge" | "block" | "error" | "system";

  // Sync→Activity deeplink (Phase 2): WatchedFoldersTable sets connection.activityFilter,
  // we consume on mount + clear so the user can still type freely afterward.
  let filter = $state(connection.activityFilter ?? "");
  if (connection.activityFilter) connection.activityFilter = null;
  let group = $state<Group>("all");
  let paused = $state(false);
  let frozen = $state<ActivityRow[]>([]);
  let selectedKey = $state<string | null>(null);
  let expandedGroups = $state<Set<string>>(new Set());

  // Burst-mode + sticky-scroll state. When new events arrive faster than
  // BURST_THRESHOLD events/window, freeze the rendered list and show a single
  // ticker banner. When the user has scrolled away from the top, hold the
  // scroll position so new rows don't shove their reading point — surface a
  // "N new" jump-to-top pip instead.
  let tbody: HTMLDivElement | undefined = $state();
  let bufferedFeed = $state<ActivityRow[]>([]);
  let bursting = $state(false);
  let burstCount = $state(0);
  let userScrolledAway = $state(false);
  let newSinceScroll = $state(0);
  let lastFeedLen = $state(0);
  let recentArrivals = $state<number[]>([]);
  const BURST_WINDOW_MS = 1_000;
  const BURST_THRESHOLD = 5;

  type Tone = "neutral" | "ok" | "info" | "warn" | "danger";
  const groups: { id: Group; label: string; tone: Tone }[] = [
    { id: "all",      label: "All",       tone: "neutral" },
    { id: "sync",     label: "Sync",      tone: "ok" },
    { id: "pull",     label: "Pull",      tone: "info" },
    { id: "delete",   label: "Delete",    tone: "warn" },
    { id: "drift",    label: "Drift",     tone: "warn" },
    { id: "conflict", label: "Conflicts", tone: "danger" },
    { id: "bridge",   label: "Bridge",    tone: "info" },
    { id: "block",    label: "Blocks",    tone: "warn" },
    { id: "error",    label: "Errors",    tone: "danger" },
    { id: "system",   label: "System",    tone: "neutral" },
  ];

  const liveMode = $derived(!paused && !bursting && !userScrolledAway);

  // Throttled mirror of connection.activityFeed for live mode. Without this,
  // every event triggers the full filter+regroup chain (`filtered` →
  // `rendered`) — costly at sustained 2-4 events/sec that fall just under the
  // burst threshold. Leading + trailing edge: first event shows immediately,
  // subsequent events coalesce into one rebuild per LIVE_THROTTLE_MS. (#254)
  const LIVE_THROTTLE_MS = 120;
  let liveSnapshot = $state<ActivityRow[]>(connection.activityFeed.slice());
  let lastSnapAt = 0;
  let snapTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    // Subscribe to feed length so this re-fires per event.
    const _len = connection.activityFeed.length;
    void _len;
    const now = Date.now();
    const elapsed = now - lastSnapAt;
    if (elapsed >= LIVE_THROTTLE_MS) {
      liveSnapshot = connection.activityFeed.slice();
      lastSnapAt = now;
      if (snapTimer) { clearTimeout(snapTimer); snapTimer = null; }
    } else if (!snapTimer) {
      snapTimer = setTimeout(() => {
        liveSnapshot = connection.activityFeed.slice();
        lastSnapAt = Date.now();
        snapTimer = null;
      }, LIVE_THROTTLE_MS - elapsed);
    }
  });
  onDestroy(() => { if (snapTimer) clearTimeout(snapTimer); });

  const source = $derived(
    paused ? frozen
    : liveMode ? liveSnapshot
    : bufferedFeed
  );

  function flushBuffer() {
    bufferedFeed = connection.activityFeed.slice();
    newSinceScroll = 0;
    bursting = false;
    recentArrivals = [];
  }

  function jumpToTop() {
    flushBuffer();
    userScrolledAway = false;
    if (tbody) tbody.scrollTop = 0;
  }

  // Watch the feed length — push arrival timestamps, detect bursts, and grow
  // newSinceScroll when the display is frozen (burst OR user-scrolled-away).
  $effect(() => {
    const len = connection.activityFeed.length;
    const delta = Math.max(0, len - untrack(() => lastFeedLen));
    if (delta === 0) return;
    if (delta > 0) {
      const now = Date.now();
      for (let i = 0; i < delta; i++) recentArrivals.push(now);
      const cutoff = now - BURST_WINDOW_MS;
      recentArrivals = recentArrivals.filter((t) => t >= cutoff);
      if (recentArrivals.length > BURST_THRESHOLD) {
        if (!bursting && !userScrolledAway) {
          // Entering burst — snapshot what's visible right now so the list
          // doesn't lurch when we flip to bufferedFeed.
          bufferedFeed = connection.activityFeed.slice(delta);
        }
        bursting = true;
      }
      burstCount = recentArrivals.length;
      if (!liveMode) newSinceScroll += delta;
    }
    lastFeedLen = len;
  });

  // Heartbeat — prune the arrival window + exit burst mode once quiet. Without
  // this, bursting would stick forever after the first flood.
  $effect(() => {
    const id = setInterval(() => {
      const cutoff = Date.now() - BURST_WINDOW_MS;
      recentArrivals = recentArrivals.filter((t) => t >= cutoff);
      burstCount = recentArrivals.length;
      if (bursting && recentArrivals.length === 0 && !userScrolledAway) {
        flushBuffer();
      }
    }, 400);
    return () => clearInterval(id);
  });

  // Scroll handler — sticky position when the user has scrolled past the top.
  // Returning to ~top auto-flushes so they don't have to click the pip.
  $effect(() => {
    if (!tbody) return;
    const el = tbody;
    const onScroll = () => {
      const top = el.scrollTop;
      if (top > 32) {
        if (!userScrolledAway) {
          bufferedFeed = connection.activityFeed.slice();
          userScrolledAway = true;
        }
      } else if (top <= 4) {
        if (userScrolledAway) {
          userScrolledAway = false;
          newSinceScroll = 0;
        }
      }
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  });

  function inGroup(r: ActivityRow, g: Group): boolean {
    if (g === "all") return true;
    if (g === "conflict") return r.kind === "conflict" || r.kind === "conflict_resolved";
    return r.kind === g;
  }

  // Group-counts memo — feed × 9 groups was O(N×9) per render. Single pass
  // builds a count map; lookups become O(1). (#203)
  const groupCounts = $derived.by<Record<Group, number>>(() => {
    const out: Record<Group, number> = {
      all: 0, sync: 0, pull: 0, delete: 0, drift: 0,
      conflict: 0, bridge: 0, block: 0, error: 0, system: 0,
    };
    for (const r of connection.activityFeed) {
      out.all++;
      if (r.kind === "conflict" || r.kind === "conflict_resolved") out.conflict++;
      else if (r.kind in out) out[r.kind as Group]++;
    }
    return out;
  });
  function countFor(g: Group): number {
    return groupCounts[g];
  }

  function rowKey(r: ActivityRow): string {
    return `${r.at}_${r.resource}_${r.file}_${r.action}`;
  }

  // Backend prepends newest to activityFeed[0]. Render order = backend order
  // → newest stays at the top. No reverse here (the prior reverse was a bug
  // that flipped the feed to oldest-first).
  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    return source.filter((r) => {
      if (!inGroup(r, group)) return false;
      if (!q) return true;
      const hay = `${r.resource} ${r.file} ${r.rel_path ?? ""} ${r.action}`.toLowerCase();
      return hay.includes(q);
    });
  });

  // Collapse consecutive runs of the SAME (resource, kind, action) within a
  // 10-second window into a single expandable group row. Singles + short runs
  // (<3) pass through unchanged.
  type Rendered =
    | { type: "single"; row: ActivityRow; key: string }
    | { type: "groupHeader"; key: string; rows: ActivityRow[]; expanded: boolean }
    | { type: "groupChild"; key: string; parentKey: string; row: ActivityRow };

  // Aggressive grouping kills the row-shoving flicker during sync bursts.
  // 2 events in 5s window means runs collapse immediately — the visual rhythm
  // settles into one growing group instead of dozens of new rows shoving older
  // ones down.
  const GROUP_WINDOW_MS = 5_000;
  const GROUP_MIN_RUN = 2;

  // Group-row aggregates. Group header used to render "—" in size + dur even
  // when children had real numbers; now it shows the run's total bytes + max
  // latency so the user gets a one-glance scope without expanding the group.
  function groupSize(rows: ActivityRow[]): number | null {
    let total = 0; let any = false;
    for (const r of rows) {
      if (r.size_bytes != null) { total += r.size_bytes; any = true; }
    }
    return any ? total : null;
  }
  function groupMaxLatency(rows: ActivityRow[]): number | null {
    let max = 0; let any = false;
    for (const r of rows) {
      if (r.latency_ms != null) { max = Math.max(max, r.latency_ms); any = true; }
    }
    return any ? max : null;
  }

  // Longest common prefix across rel_paths in a run. Drives the "path" column
  // of a group header — beats "9 items" because the user wants to see WHICH
  // dir was operated on.
  function commonPathPrefix(rows: ActivityRow[]): string {
    const paths = rows
      .map((r) => r.rel_path ?? r.file ?? "")
      .filter((p) => p.length > 0);
    if (paths.length === 0) return "";
    const split = paths.map((p) => p.split("/").filter(Boolean));
    const out: string[] = [];
    const minLen = Math.min(...split.map((s) => s.length));
    for (let i = 0; i < minLen; i++) {
      const seg = split[0][i];
      if (split.every((s) => s[i] === seg)) out.push(seg);
      else break;
    }
    if (out.length === 0) return paths[0].split("/")[0] ?? "";
    return out.join("/");
  }

  const rendered = $derived.by<Rendered[]>(() => {
    const out: Rendered[] = [];
    let i = 0;
    while (i < filtered.length) {
      const head = filtered[i];
      let j = i + 1;
      while (
        j < filtered.length &&
        filtered[j].resource === head.resource &&
        filtered[j].kind === head.kind &&
        filtered[j].action === head.action &&
        Math.abs(new Date(filtered[j].at).getTime() - new Date(filtered[j - 1].at).getTime()) <= GROUP_WINDOW_MS
      ) {
        j++;
      }
      const run = filtered.slice(i, j);
      if (run.length >= GROUP_MIN_RUN) {
        const key = `g_${head.resource}_${head.kind}_${head.action}_${head.at}`;
        const expanded = expandedGroups.has(key);
        out.push({ type: "groupHeader", key, rows: run, expanded });
        if (expanded) {
          for (const r of run) {
            out.push({ type: "groupChild", key: rowKey(r), parentKey: key, row: r });
          }
        }
      } else {
        for (const r of run) {
          out.push({ type: "single", row: r, key: rowKey(r) });
        }
      }
      i = j;
    }
    return out;
  });

  const selectedRow = $derived.by<ActivityRow | null>(() => {
    if (!selectedKey) return null;
    return filtered.find((r) => rowKey(r) === selectedKey) ?? null;
  });

  function fmtTime(iso: string): string {
    try {
      return fmtRelative(iso);
    } catch { return iso; }
  }

  function fmtFullTime(iso: string): string {
    try {
      return new Date(iso).toLocaleString([], { hour12: true });
    } catch { return iso; }
  }

  function fmtSize(n: number | null | undefined): string {
    if (n == null) return "—";
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(2)} MB`;
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function fmtLatency(ms: number | null | undefined): string {
    if (ms == null) return "—";
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  }

  function shortSha(s: string | null | undefined): string {
    if (!s) return "—";
    return s.length > 12 ? `${s.slice(0, 12)}…` : s;
  }

  // Resource string from the backend is the bare folder name (`endure`, `ox`)
  // OR is already bracketed by the user's own resource naming (`[ox]` in
  // FiveM). Wrap in single `[…]` only when not already wrapped — kills the
  // `[[ox]]` double-bracket bug from v0.2.35.
  function fmtResource(r: string): string {
    if (!r) return "—";
    if (r.startsWith("[") && r.endsWith("]")) return r;
    return `[${r}]`;
  }

  function pathOf(r: ActivityRow): string {
    return r.rel_path ?? r.file ?? "";
  }

  type Variant = "ok" | "warn" | "danger" | "info" | "muted";

  function kindIcon(k: ActivityKind) {
    switch (k) {
      case "sync": return RefreshCw;
      case "pull": return Download;
      case "delete": return Trash2;
      case "conflict": return AlertTriangle;
      case "conflict_resolved": return Check;
      case "drift": return GitBranch;
      case "bridge": return Network;
      case "block": return Lock;
      case "error": return XCircle;
      case "system": return Info;
    }
  }

  function kindVariant(k: ActivityKind): Variant {
    switch (k) {
      case "sync":
      case "conflict_resolved": return "ok";
      case "pull":
      case "bridge": return "info";
      case "drift":
      case "delete":
      case "block": return "warn";
      case "conflict":
      case "error": return "danger";
      default: return "muted";
    }
  }

  function togglePause() {
    if (paused) {
      paused = false;
      frozen = [];
    } else {
      frozen = [...connection.activityFeed];
      paused = true;
    }
  }

  function toggleGroup(key: string) {
    const next = new Set(expandedGroups);
    if (next.has(key)) next.delete(key); else next.add(key);
    expandedGroups = next;
  }

  function selectRow(key: string) {
    selectedKey = selectedKey === key ? null : key;
  }

  function resolveLocalPath(r: ActivityRow): string | null {
    return r.local_path ?? null;
  }

  let actionFlash = $state<string | null>(null);
  let flashTimer: ReturnType<typeof setTimeout> | null = null;
  function flash(msg: string) {
    actionFlash = msg;
    if (flashTimer) clearTimeout(flashTimer);
    flashTimer = setTimeout(() => { actionFlash = null; flashTimer = null; }, 1500);
  }
  onDestroy(() => { if (flashTimer) clearTimeout(flashTimer); });

  async function copyPath(r: ActivityRow) {
    const p = resolveLocalPath(r);
    if (!p) { flash("path unknown"); return; }
    try { await navigator.clipboard.writeText(p); flash("path copied"); }
    catch { flash("copy failed"); }
  }

  async function openFile(r: ActivityRow) {
    const p = resolveLocalPath(r);
    if (!p) { flash("path unknown"); return; }
    try { await openPath(p); flash("opened"); }
    catch (e) { flash(`open failed: ${e}`); }
  }

  async function revealInFolder(r: ActivityRow) {
    const p = resolveLocalPath(r);
    if (!p) { flash("path unknown"); return; }
    try { await revealItemInDir(p); flash("revealed in OS"); }
    catch (e) { flash(`reveal failed: ${e}`); }
  }
</script>

<section class="feed">
  <header class="af-head" data-tone="info">
    <div class="af-head-l">
      <span class="head-icon">
        {#if tab === "activity"}<ActivityIcon size={14}/>{:else}<Stethoscope size={14}/>{/if}
      </span>
      <div class="af-tabs" role="tablist" aria-label="Activity views">
        <button
          type="button" role="tab"
          aria-selected={tab === "activity"}
          data-active={tab === "activity"}
          onclick={() => (tab = "activity")}
        >
          Activity
          <span class="af-tab-count" data-zero={connection.activityFeed.length === 0}>{connection.activityFeed.length}</span>
        </button>
        <button
          type="button" role="tab"
          aria-selected={tab === "diagnostics"}
          data-active={tab === "diagnostics"}
          onclick={() => (tab = "diagnostics")}
        >
          Diagnostics
          <span class="af-tab-count" data-zero={diagnostics.events.length === 0}>{diagnostics.events.length}</span>
        </button>
      </div>
    </div>
    <div class="af-head-r">
      {#if tab === "activity"}
        <button
          class="btn sm"
          class:warn={paused}
          class:ghost={!paused}
          type="button"
          onclick={togglePause}
          title={paused ? "Resume feed" : "Pause feed"}
        >
          {#if paused}<Play size={11}/> Resume{:else}<Pause size={11}/> Pause{/if}
        </button>
        <button
          class="btn ghost sm"
          type="button"
          onclick={() => connection.clearActivity()}
          disabled={connection.activityFeed.length === 0}
          title="Clear feed"
        >
          <Trash2 size={11}/> Clear
        </button>
      {:else}
        <button
          class="btn sm"
          class:warn={diagnostics.paused}
          class:ghost={!diagnostics.paused}
          type="button"
          onclick={() => diagnostics.togglePause()}
          title={diagnostics.paused ? "Resume capture" : "Pause capture"}
        >
          {#if diagnostics.paused}<Play size={11}/> Resume{:else}<Pause size={11}/> Pause{/if}
        </button>
        <button
          class="btn ghost sm"
          type="button"
          onclick={() => diagnostics.clear()}
          disabled={diagnostics.events.length === 0}
          title="Clear captured events"
        >
          <Trash2 size={11}/> Clear
        </button>
      {/if}
    </div>
  </header>

  {#if tab === "diagnostics"}
    <Diagnostics embedded />
  {:else}

  <PageToolbar>
    <div class="segctl">
      {#each groups as g (g.id)}
        {@const n = g.id === "all" ? connection.activityFeed.length : countFor(g.id)}
        <button
          type="button"
          data-active={group === g.id}
          data-tone={g.tone}
          onclick={() => (group = g.id)}
        >
          {g.label}
          {#if g.id !== "all"}
            <span class="pip" data-zero={n === 0} data-tone={g.tone}>{n}</span>
          {/if}
        </button>
      {/each}
    </div>

    <input
      class="filter"
      type="text"
      placeholder="Filter resource / file / action…"
      bind:value={filter}
    />
  </PageToolbar>

  <div class="table">
    <div class="thead">
      <div class="th time">Time</div>
      <div class="th kind"></div>
      <div class="th resource">Resource</div>
      <div class="th path">Path</div>
      <div class="th action">Action</div>
      <div class="th size">Size</div>
      <div class="th dur">Dur</div>
    </div>

    <div class="tbody" bind:this={tbody}>
      {#if bursting || newSinceScroll > 0}
        <button
          type="button"
          class="burst-pip mono"
          onclick={jumpToTop}
          title={bursting ? "Burst in progress — click to resume + jump to top" : "Jump to top"}
        >
          {#if bursting}
            <span class="dot live"></span>
            live · {burstCount}/s · click to resume
          {:else}
            ↑ {newSinceScroll} new · click to jump to top
          {/if}
        </button>
      {/if}
      {#if rendered.length === 0}
        {#if connection.activityFeed.length === 0}
          <EmptyState
            icon={ActivityIcon}
            tone="info"
            title="No activity yet"
            hint="Sync, pull, and bridge events will appear here as they fire. Trigger a rescan to see anything that's diverged."
          >
            <button
              type="button"
              class="btn primary"
              disabled={!connection.status || connection.status.state === "disabled" || connection.status.state === "error"}
              onclick={() => syncPage.rescan()}
            >
              <RefreshCw size={13}/> Rescan now
            </button>
          </EmptyState>
        {:else}
          <EmptyState
            icon={ActivityIcon}
            tone="neutral"
            title="No matches"
            hint={filter ? `Nothing matches "${filter}".` : "No events in this group."}
          >
            {#if filter || group !== "all"}
              <button type="button" class="btn ghost sm" onclick={() => { filter = ""; group = "all"; }}>
                Clear filters
              </button>
            {/if}
          </EmptyState>
        {/if}
      {:else}
        {#each rendered as item (item.key)}
          {#if item.type === "groupHeader"}
            {@const r0 = item.rows[0]}
            {@const Icon = kindIcon(r0.kind)}
            {@const v = kindVariant(r0.kind)}
            {@const prefix = commonPathPrefix(item.rows)}
            <div
              class="tr group"
              data-expanded={item.expanded}
              data-variant={v}
              onclick={() => toggleGroup(item.key)}
              onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleGroup(item.key); } }}
              role="button"
              tabindex="0"
            >
              <div class="td time mono" title={fmtFullTime(r0.at)}>{fmtTime(r0.at)}</div>
              <div class="td kind">
                <span class="kchip" data-variant={v}>
                  <Icon size={11}/>
                </span>
                <span class="count-chip mono">{item.rows.length}×</span>
              </div>
              <div class="td resource mono">{fmtResource(r0.resource)}</div>
              <div class="td path mono" title={prefix}>{prefix || "—"}</div>
              <div class="td action">
                <span class="chev" data-expanded={item.expanded}>
                  {#if item.expanded}<ChevronDown size={11}/>{:else}<ChevronRight size={11}/>{/if}
                </span>
                <span class="action-text">{r0.action}</span>
                {#if r0.actor}<span class="actor-chip mono" title="Actor">{r0.actor}</span>{/if}
              </div>
              <div class="td size mono" title="Total across {item.rows.length} events">{fmtSize(groupSize(item.rows))}</div>
              <div class="td dur mono" title="Max latency across {item.rows.length} events">{fmtLatency(groupMaxLatency(item.rows))}</div>
            </div>
          {:else if item.type === "groupChild"}
            {@const r = item.row}
            {@const Icon = kindIcon(r.kind)}
            {@const v = kindVariant(r.kind)}
            {@const isSelected = selectedKey === item.key}
            <div
              class="tr child"
              data-selected={isSelected}
              data-variant={v}
              onclick={() => selectRow(item.key)}
              onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); selectRow(item.key); } }}
              role="button"
              tabindex="0"
            >
              <div class="td time mono" title={fmtFullTime(r.at)}>{fmtTime(r.at)}</div>
              <div class="td kind">
                <span class="kchip sm" data-variant={v}>
                  <Icon size={10}/>
                </span>
              </div>
              <div class="td resource mono">{fmtResource(r.resource)}</div>
              <div class="td path mono" title={pathOf(r)}>{pathOf(r) || "—"}</div>
              <div class="td action" title={r.action}>
                <span class="action-text">{r.action}</span>
                {#if r.actor}<span class="actor-chip mono" title="Actor">{r.actor}</span>{/if}
              </div>
              <div class="td size mono">{fmtSize(r.size_bytes)}</div>
              <div class="td dur mono">{fmtLatency(r.latency_ms)}</div>
            </div>
            {#if isSelected}
              {@render detailStrip(r, v)}
            {/if}
          {:else}
            {@const r = item.row}
            {@const Icon = kindIcon(r.kind)}
            {@const v = kindVariant(r.kind)}
            {@const isSelected = selectedKey === item.key}
            <div
              class="tr"
              data-selected={isSelected}
              data-variant={v}
              onclick={() => selectRow(item.key)}
              onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); selectRow(item.key); } }}
              role="button"
              tabindex="0"
            >
              <div class="td time mono" title={fmtFullTime(r.at)}>{fmtTime(r.at)}</div>
              <div class="td kind">
                <span class="kchip" data-variant={v}>
                  <Icon size={11}/>
                </span>
              </div>
              <div class="td resource mono">{fmtResource(r.resource)}</div>
              <div class="td path mono" title={pathOf(r)}>{pathOf(r) || "—"}</div>
              <div class="td action" title={r.action}>
                <span class="action-text">{r.action}</span>
                {#if r.actor}<span class="actor-chip mono" title="Actor">{r.actor}</span>{/if}
              </div>
              <div class="td size mono">{fmtSize(r.size_bytes)}</div>
              <div class="td dur mono">{fmtLatency(r.latency_ms)}</div>
            </div>
            {#if isSelected}
              {@render detailStrip(r, v)}
            {/if}
          {/if}
        {/each}
      {/if}
    </div>
  </div>

  {#if paused}
    <div class="paused-banner mono">
      Feed paused — {Math.max(0, connection.activityFeed.length - frozen.length)} new since pause
    </div>
  {/if}
  {#if actionFlash}
    <div class="action-flash mono">{actionFlash}</div>
  {/if}
  {/if}
</section>

{#snippet detailStrip(r: ActivityRow, variant: Variant)}
  {@const localPath = resolveLocalPath(r)}
  <div class="strip" data-variant={variant} role="region" aria-label="Event details">
    <div class="strip-meta">
      <div class="meta-row">
        <span class="meta-k">When</span>
        <span class="meta-v mono">{fmtFullTime(r.at)}</span>
      </div>
      {#if r.sha}
        <div class="meta-row">
          <span class="meta-k">SHA</span>
          <span class="meta-v mono" title={r.sha}>{shortSha(r.sha)}</span>
        </div>
      {/if}
      {#if localPath}
        <div class="meta-row path-row">
          <span class="meta-k">Path</span>
          <span class="meta-v mono" title={localPath}>{localPath}</span>
        </div>
      {/if}
    </div>
    <div class="strip-actions">
      <button
        class="btn ghost sm"
        type="button"
        onclick={(e) => { e.stopPropagation(); openFile(r); }}
        disabled={!localPath}
        title={localPath ?? "Path unknown"}
      >
        <ExternalLink size={11}/> Open
      </button>
      <button
        class="btn ghost sm"
        type="button"
        onclick={(e) => { e.stopPropagation(); copyPath(r); }}
        disabled={!localPath}
      >
        <Copy size={11}/> Copy
      </button>
      <button
        class="btn ghost sm"
        type="button"
        onclick={(e) => { e.stopPropagation(); revealInFolder(r); }}
        disabled={!localPath}
        title="Reveal in OS file browser"
      >
        <Folder size={11}/> Reveal
      </button>
    </div>
  </div>
{/snippet}

<style>
  .feed {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    background: var(--bg);
    color: var(--fg);
    position: relative;
  }

  /* Unified header — replaces PageHeader on this page so the tabs ARE the
     title. Tone stripe + sizing mirrors PageHeader so it blends w/ sibling
     pages (Sync/Files/Settings) at the workspace boundary. */
  .af-head {
    position: relative;
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    padding: 8px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex-shrink: 0;
    min-height: 46px;
  }
  .af-head::after {
    content: "";
    position: absolute;
    left: 0; right: 0; bottom: -1px;
    height: 2px;
    background: var(--info);
    opacity: 0.55;
    pointer-events: none;
  }
  .af-head-l { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1; }
  .af-head-r { display: flex; align-items: center; gap: 8px; }
  .af-head .head-icon {
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--info);
    transition: color 160ms;
  }

  .af-tabs {
    display: inline-flex;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 1px;
  }
  .af-tabs button {
    display: inline-flex; align-items: center; gap: 6px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-sm); font-weight: 500;
    padding: 4px 11px; height: 26px;
    border-radius: 4px;
    cursor: pointer;
    transition: color 120ms, background 120ms;
    letter-spacing: -0.01em;
  }
  .af-tabs button:hover { color: var(--fg); }
  .af-tabs button[data-active="true"] {
    color: var(--fg);
    background: color-mix(in oklch, var(--info) 18%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--info) 28%, transparent);
  }
  .af-tabs button:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .af-tab-count {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    min-width: 16px;
    text-align: center;
    border-radius: 999px;
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    letter-spacing: 0;
    font-variant-numeric: tabular-nums;
  }
  .af-tabs button[data-active="true"] .af-tab-count {
    background: color-mix(in oklch, var(--info) 22%, transparent);
    color: var(--info);
  }
  .af-tab-count[data-zero="true"] { opacity: 0.45; }

  .segctl {
    display: inline-flex; padding: 2px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    gap: 1px;
    overflow-x: auto;
    max-width: 60%;
  }
  .segctl button {
    --tone: var(--fg-muted);
    padding: 3px 9px; height: 22px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    border-radius: var(--radius-xs);
    cursor: pointer;
    display: inline-flex; align-items: center; gap: 6px;
    white-space: nowrap;
    transition: background 100ms ease, color 100ms ease, box-shadow 100ms ease;
  }
  .segctl button[data-tone="ok"]      { --tone: var(--ok); }
  .segctl button[data-tone="info"]    { --tone: var(--info); }
  .segctl button[data-tone="warn"]    { --tone: var(--warn); }
  .segctl button[data-tone="danger"]  { --tone: var(--danger); }
  .segctl button[data-tone="neutral"] { --tone: var(--fg-muted); }
  .segctl button:hover { color: var(--fg); background: color-mix(in oklch, var(--tone) 10%, transparent); }
  .segctl button[data-active="true"] {
    background: color-mix(in oklch, var(--tone) 18%, var(--surface));
    color: var(--tone);
    box-shadow: inset 2px 0 var(--tone);
    font-weight: 600;
  }
  .segctl button[data-tone="neutral"][data-active="true"] {
    color: var(--fg);
    background: var(--surface);
    box-shadow: inset 2px 0 var(--fg-muted);
  }
  .pip {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 16px; height: 14px; padding: 0 4px;
    border-radius: 7px;
    background: var(--bg-elev-3);
    color: var(--fg-subtle);
    font-size: 10px; line-height: 1;
    font-variant-numeric: tabular-nums;
  }
  .pip[data-zero="true"] { opacity: 0.45; }
  .pip[data-tone="ok"]:not([data-zero="true"])     { background: var(--ok-soft);     color: var(--ok); }
  .pip[data-tone="info"]:not([data-zero="true"])   { background: var(--info-soft);   color: var(--info); }
  .pip[data-tone="warn"]:not([data-zero="true"])   { background: var(--warn-soft);   color: var(--warn); }
  .pip[data-tone="danger"]:not([data-zero="true"]) { background: var(--danger-soft); color: var(--danger); }

  .filter {
    flex: 1; min-width: 0;
    height: 26px;
    background: var(--bg-elev-1);
    color: var(--fg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0 10px;
    font: inherit; font-size: var(--fs-sm);
    transition: border-color 100ms ease, box-shadow 100ms ease, background 100ms ease;
  }
  .filter:focus {
    outline: 0;
    border-color: var(--border-focus, var(--accent));
    box-shadow: 0 0 0 3px var(--ring);
  }
  .filter::placeholder { color: var(--fg-faint); }

  /* Table-style layout. Single source of truth for column widths lives on
     `.thead` and is mirrored on every `.tr` via display: grid. Keeps headers
     and rows aligned without per-cell width hacks. */
  .table {
    margin: 10px 14px 14px;
    flex: 1; min-height: 0;
    display: flex; flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    overflow: hidden;
  }
  .thead, .tr {
    display: grid;
    grid-template-columns:
      124px                /* time   */
      54px                 /* kind   */
      110px                /* res    */
      minmax(160px, 1fr)   /* path   */
      minmax(240px, 1fr)   /* action (now also carries actor as a chip) */
      72px                 /* size   */
      64px;                /* dur    */
    align-items: center;
    column-gap: 12px;
    padding: 0 12px;
    font-size: var(--fs-sm);
  }
  .thead {
    height: 30px;
    background: var(--bg-elev-2);
    border-bottom: 1px solid var(--border);
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    position: sticky; top: 0; z-index: 1;
  }
  .th { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .th.size, .th.dur { text-align: right; }

  .tbody {
    flex: 1; min-height: 0; overflow: auto;
  }

  .tr {
    height: 32px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    user-select: none;
    animation: row-flash 900ms ease-out 1;
  }
  @keyframes row-flash {
    0%   { background: color-mix(in oklch, var(--accent-soft) 75%, transparent); }
    100% { background: transparent; }
  }
  /* User prefers reduced motion → drop the flash entirely. */
  @media (prefers-reduced-motion: reduce) {
    .tr { animation: none; }
  }
  .tr:last-child { border-bottom: 0; }
  .tr:hover { background: var(--surface-hover); }
  .tr:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .tr[data-selected="true"] {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 var(--accent);
  }
  .tr[data-selected="true"][data-variant="ok"]     { background: var(--ok-soft);     box-shadow: inset 2px 0 var(--ok); }
  .tr[data-selected="true"][data-variant="info"]   { background: var(--info-soft);   box-shadow: inset 2px 0 var(--info); }
  .tr[data-selected="true"][data-variant="warn"]   { background: var(--warn-soft);   box-shadow: inset 2px 0 var(--warn); }
  .tr[data-selected="true"][data-variant="danger"] { background: var(--danger-soft); box-shadow: inset 2px 0 var(--danger); }
  .tr[data-selected="true"][data-variant="muted"]  { background: var(--surface-hover); box-shadow: inset 2px 0 var(--fg-muted); }
  .tr[data-variant="danger"] .td.action { color: var(--danger); }
  .tr[data-variant="warn"] .td.action { color: var(--warn); }

  /* Group header — bg tone-keyed by kind so burst patterns are scannable */
  .tr.group {
    font-weight: 500;
    box-shadow: inset 2px 0 transparent;
  }
  .tr.group[data-variant="ok"]     { background: color-mix(in oklch, var(--surface) 72%, var(--ok-soft));     box-shadow: inset 2px 0 color-mix(in oklch, var(--ok) 55%, transparent); }
  .tr.group[data-variant="info"]   { background: color-mix(in oklch, var(--surface) 72%, var(--info-soft));   box-shadow: inset 2px 0 color-mix(in oklch, var(--info) 55%, transparent); }
  .tr.group[data-variant="warn"]   { background: color-mix(in oklch, var(--surface) 72%, var(--warn-soft));   box-shadow: inset 2px 0 color-mix(in oklch, var(--warn) 55%, transparent); }
  .tr.group[data-variant="danger"] { background: color-mix(in oklch, var(--surface) 72%, var(--danger-soft)); box-shadow: inset 2px 0 color-mix(in oklch, var(--danger) 55%, transparent); }
  .tr.group[data-variant="muted"]  { background: color-mix(in oklch, var(--surface) 88%, var(--bg-elev-2)); }
  .tr.group:hover { filter: brightness(1.08); }
  .tr.child { background: color-mix(in oklch, var(--surface) 95%, var(--bg-elev-2)); }
  .tr.child .td.resource { padding-left: 14px; opacity: 0.7; }

  .td {
    min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: var(--fg);
  }
  .td.time { color: var(--fg-subtle); font-size: var(--fs-xs); }
  .td.kind { display: inline-flex; align-items: center; gap: 6px; }
  .td.resource { color: var(--fg-muted); }
  .td.path { color: var(--fg); }
  .td.action {
    display: inline-flex; align-items: center; gap: 6px;
    min-width: 0;
  }
  .action-text {
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    min-width: 0;
    flex: 0 1 auto;
  }
  .actor-chip {
    flex-shrink: 0;
    display: inline-flex; align-items: center;
    padding: 1px 6px;
    border-radius: 7px;
    background: var(--bg-elev-3);
    color: var(--fg-subtle);
    font-size: 10px;
    line-height: 1.4;
    max-width: 96px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .tr.group .actor-chip {
    background: color-mix(in oklch, var(--bg-elev-3) 80%, var(--surface));
  }
  .td.size, .td.dur { text-align: right; color: var(--fg-muted); font-size: var(--fs-xs); }

  .kchip {
    width: 22px; height: 22px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-2); color: var(--fg-muted);
    flex-shrink: 0;
  }
  .kchip.sm { width: 18px; height: 18px; }
  .kchip[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kchip[data-variant="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .kchip[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
  .kchip[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }

  .count-chip {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 7px;
    background: var(--bg-elev-3);
    color: var(--fg-muted);
    font-size: 10px; line-height: 1.4;
    font-variant-numeric: tabular-nums;
  }

  .chev {
    color: var(--fg-subtle);
    display: inline-flex; align-items: center; justify-content: center;
    flex-shrink: 0;
  }

  /* Inline detail strip — renders directly beneath the selected row. Spans
     the whole row width (NOT grid-aware) for legibility, indented to align
     with the path column visually. */
  .strip {
    padding: 8px 12px 10px;
    background: color-mix(in oklch, var(--accent-soft) 30%, var(--surface));
    border-bottom: 1px solid var(--border);
    box-shadow: inset 2px 0 var(--accent);
    display: flex; gap: 16px; align-items: flex-start;
    flex-wrap: wrap;
    /* No entry animation — during sync bursts the flicker was nauseating. */
  }
  .strip[data-variant="ok"]     { background: color-mix(in oklch, var(--ok-soft) 30%, var(--surface));     box-shadow: inset 2px 0 var(--ok); }
  .strip[data-variant="info"]   { background: color-mix(in oklch, var(--info-soft) 30%, var(--surface));   box-shadow: inset 2px 0 var(--info); }
  .strip[data-variant="warn"]   { background: color-mix(in oklch, var(--warn-soft) 30%, var(--surface));   box-shadow: inset 2px 0 var(--warn); }
  .strip[data-variant="danger"] { background: color-mix(in oklch, var(--danger-soft) 30%, var(--surface)); box-shadow: inset 2px 0 var(--danger); }
  .strip-meta {
    display: flex; gap: 10px 18px; flex-wrap: wrap;
    flex: 1; min-width: 0;
  }
  .meta-row {
    display: inline-flex; align-items: baseline; gap: 6px;
    font-size: var(--fs-xs);
    min-width: 0;
  }
  .meta-row.path-row { flex-basis: 100%; }
  .meta-k {
    color: var(--fg-subtle);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 10px;
  }
  .meta-v {
    color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    min-width: 0;
  }
  .strip-actions {
    display: inline-flex; gap: 6px;
    flex-shrink: 0;
  }


  /* Top-of-feed pip — visible only when bursting or scrolled-away. Sticky so
     it stays pinned even as the list scrolls beneath it. */
  .burst-pip {
    position: sticky; top: 0; z-index: 2;
    display: inline-flex; align-items: center; gap: 6px;
    margin: 4px auto;
    padding: 4px 10px;
    background: var(--accent-soft);
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    font-size: var(--fs-xs);
    cursor: pointer;
    box-shadow: var(--shadow);
    /* Center it across the tbody width. */
    align-self: center;
    width: fit-content;
    left: 50%; transform: translateX(-50%);
  }
  .burst-pip:hover { background: color-mix(in oklch, var(--accent-soft) 60%, var(--accent)); }
  .burst-pip .dot.live {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: var(--danger);
    box-shadow: 0 0 6px var(--danger);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .paused-banner, .action-flash {
    position: absolute;
    bottom: 22px; left: 50%; transform: translateX(-50%);
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 4px 10px;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    box-shadow: var(--shadow);
    z-index: 2;
  }
  .paused-banner {
    background: color-mix(in oklch, var(--warn-soft) 70%, var(--bg-elev-2));
    border-color: color-mix(in oklch, var(--warn) 40%, var(--border-strong));
    color: var(--warn);
  }
  .action-flash { bottom: 56px; }

  /* Narrow viewports: tighten the path col so action+actor chip stay
     visible. Actor is now folded into the action cell as a chip, so no
     column hiding is needed. */
  @media (max-width: 1400px) {
    .thead, .tr {
      grid-template-columns:
        110px 50px 96px minmax(140px, 1fr) minmax(200px, 1fr) 64px 56px;
    }
    .actor-chip { max-width: 72px; }
  }
</style>
