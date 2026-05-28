<script lang="ts">
  import {
    Activity, AlertTriangle, Ban, Clock, FileWarning, FolderSync,
    GitPullRequestArrow, Lock, Network, Pause, Play, Trash2,
    Zap, Info, ClipboardCopy, Check, Stethoscope,
  } from "lucide-svelte";
  import { diagnostics, type DiagLevel, type DiagStage } from "../../state/diagnostics.svelte";
  import { connection } from "../../state/connection.svelte";
  import PageHeader from "../shell/PageHeader.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  let { embedded = false }: { embedded?: boolean } = $props();

  let expanded = $state<number | null>(null);
  let copyState = $state<"idle" | "copying" | "ok" | "err">("idle");
  let copySize = $state(0);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Fire-and-forget: events emitted before wire() resolves are not captured.
    // Ring buffer (1000 entries) on the backend covers the gap.
    void diagnostics.wire();
    return () => {
      diagnostics.dispose();
      if (copyTimer) clearTimeout(copyTimer);
    };
  });

  const ROW_H = 36;
  const OVERSCAN = 6;
  let scroller: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewport = $state(600);

  $effect(() => {
    if (!scroller) return;
    const el = scroller;
    const onScroll = () => { scrollTop = el.scrollTop; };
    const ro = new ResizeObserver(() => { viewport = el.clientHeight; });
    el.addEventListener("scroll", onScroll, { passive: true });
    ro.observe(el);
    return () => {
      el.removeEventListener("scroll", onScroll);
      ro.disconnect();
    };
  });

  const events = $derived(diagnostics.events);
  const startIdx = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const endIdx = $derived(
    Math.min(events.length, Math.ceil((scrollTop + viewport) / ROW_H) + OVERSCAN),
  );
  const slice = $derived(events.slice(startIdx, endIdx));
  const padTop = $derived(startIdx * ROW_H);
  const padBot = $derived(Math.max(0, (events.length - endIdx) * ROW_H));

  function fmtTime(iso: string): string {
    try {
      const d = new Date(iso);
      return d.toLocaleTimeString([], { hour12: true }) +
        "." + String(d.getMilliseconds()).padStart(3, "0");
    } catch { return iso; }
  }

  function fmtAge(iso: string | null | undefined): string {
    if (!iso) return "—";
    try {
      const ms = Date.now() - new Date(iso).getTime();
      if (ms < 1000) return `${ms}ms`;
      if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
      if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m`;
      return `${Math.floor(ms / 3_600_000)}h`;
    } catch { return "—"; }
  }

  type Variant = "ok" | "warn" | "danger" | "info" | "muted";

  function levelVariant(l: DiagLevel): Variant {
    if (l === "error") return "danger";
    if (l === "warn") return "warn";
    if (l === "info") return "info";
    return "muted";
  }

  function stageIcon(s: DiagStage) {
    if (s === "fs_event") return Activity;
    if (s === "ignored") return Ban;
    if (s === "debounced" || s === "queued") return Clock;
    if (s === "queue_dropped") return FileWarning;
    if (s.startsWith("upload")) return FolderSync;
    if (s.startsWith("lock")) return Lock;
    if (s.startsWith("drift")) return GitPullRequestArrow;
    if (s.startsWith("bridge")) return Network;
    if (s === "rescan_signal") return Zap;
    if (s.startsWith("sftp")) return Network;
    if (s.startsWith("remote_scan")) return GitPullRequestArrow;
    if (s.startsWith("remote_pull")) return FolderSync;
    if (s === "log") return Info;
    return AlertTriangle;
  }

  function stageVariant(s: DiagStage, l: DiagLevel): Variant {
    if (l === "error") return "danger";
    if (l === "warn") return "warn";
    if (s === "queue_dropped" || s === "rescan_signal" || s === "upload_fail" || s === "lock_held_by_other" || s === "remote_pull_fail") return "warn";
    if (s.startsWith("upload") || s === "atomic_rename" || s === "remote_pull_done") return "ok";
    if (s.startsWith("drift") || s.startsWith("bridge") || s.startsWith("remote")) return "info";
    if (s === "ignored") return "muted";
    return "info";
  }

  async function copyReport() {
    if (copyState === "copying") return;
    copyState = "copying";
    if (copyTimer) { clearTimeout(copyTimer); copyTimer = null; }
    try {
      const json = await diagnostics.generateReport();
      await navigator.clipboard.writeText(json);
      copySize = Math.round(json.length / 1024);
      copyState = "ok";
    } catch (e) {
      console.error("copyReport failed", e);
      copyState = "err";
    }
    copyTimer = setTimeout(() => { copyState = "idle"; }, 2400);
  }

  const lastRemoteScan = $derived(diagnostics.events.find((e) => e.stage === "remote_scan_result")?.at ?? null);
  const lastPullAt = $derived(diagnostics.events.find((e) => e.stage === "remote_pull_done")?.at ?? null);
  const pulledTotal = $derived((diagnostics.countsByStage["remote_pull_done"] ?? 0));
  const pullFailedTotal = $derived((diagnostics.countsByStage["remote_pull_fail"] ?? 0));

  type Tile = { label: string; value: string; hint: string; tone: Variant };
  const tileGroups = $derived.by<{ label: string; items: Tile[] }[]>(() => {
    const s = diagnostics.state;
    return [
      {
        label: "Sync engine",
        items: [
          { label: "Auto-sync",     value: s?.autosync_state ?? "—",            hint: s?.autosync_detail ?? "",     tone: (s?.autosync_state === "error" ? "danger" : s?.autosync_state ? "info" : "muted") },
          { label: "Watching",      value: String(s?.watcher_count ?? 0),       hint: "folders",                    tone: "muted" },
          { label: "Queue pending", value: String(s?.queue_pending ?? 0),       hint: "files debouncing/in-flight", tone: ((s?.queue_pending ?? 0) > 50 ? "warn" : "muted") },
          { label: "Failed",        value: String(s?.queue_failed ?? 0),        hint: "retry pending",              tone: ((s?.queue_failed ?? 0) > 0 ? "danger" : "muted") },
          { label: "Dropped",       value: String(s?.queue_dropped_total ?? 0), hint: "channel-full events lost",   tone: ((s?.queue_dropped_total ?? 0) > 0 ? "danger" : "muted") },
          { label: "Ignored",       value: String(s?.ignored_total ?? 0),       hint: "rule-filtered total",        tone: "muted" },
        ],
      },
      {
        label: "Conflicts & locks",
        items: [
          { label: "Conflicts", value: String(s?.conflicts ?? 0),    hint: "needs resolve",  tone: ((s?.conflicts ?? 0) > 0 ? "warn" : "muted") },
          { label: "Locks",     value: String(connection.lockCount), hint: "presence locks", tone: "muted" },
        ],
      },
      {
        label: "Drift activity",
        items: [
          { label: "Last remote scan", value: fmtAge(lastRemoteScan),                hint: "drift watcher tick",        tone: "muted" },
          { label: "Pulled",           value: String(pulledTotal),                   hint: pullFailedTotal > 0 ? `${pullFailedTotal} failed` : (lastPullAt ? `last ${fmtAge(lastPullAt)} ago` : "from remote"), tone: (pullFailedTotal > 0 ? "warn" : "muted") },
          { label: "Last rescan",      value: fmtAge(s?.last_rescan_signal_at),      hint: "kernel event-drop signal",  tone: (s?.last_rescan_signal_at ? "warn" : "muted") },
          { label: "Last drift",       value: fmtAge(s?.last_drift_scan_at),         hint: "reconcile fired",           tone: "muted" },
          { label: "Bus lag",          value: String(s?.bus_lag_total ?? 0),         hint: "diag events the UI missed", tone: ((s?.bus_lag_total ?? 0) > 0 ? "warn" : "muted") },
          { label: "Total emitted",    value: String(s?.events_emitted_total ?? 0),  hint: "lifetime diag events",      tone: "muted" },
        ],
      },
    ];
  });
</script>

<section class="diag">
  {#if !embedded}
    <PageHeader
      icon={Stethoscope}
      title="Diagnostics"
      subtitle="{diagnostics.events.length} event{diagnostics.events.length === 1 ? '' : 's'} captured"
      tone="info"
    >
      {#snippet actions()}
        <button class="btn ghost sm" type="button" onclick={() => diagnostics.togglePause()} use:tooltip={diagnostics.paused ? "Resume capture" : "Pause capture"}>
          {#if diagnostics.paused}<Play size={11}/> Resume{:else}<Pause size={11}/> Pause{/if}
        </button>
        <button class="btn ghost sm" type="button" onclick={() => diagnostics.clear()} disabled={diagnostics.events.length === 0} use:tooltip={"Clear captured events"}>
          <Trash2 size={11}/> Clear
        </button>
      {/snippet}
    </PageHeader>
  {/if}

  {#each tileGroups as g (g.label)}
    <div class="tile-group">
      <div class="group-label">{g.label}</div>
      <div class="tiles">
        {#each g.items as t (t.label)}
          <div class="tile" data-tone={t.tone}>
            <div class="tile-label">{t.label}</div>
            <div class="tile-value mono">{t.value}</div>
            <div class="tile-hint">{t.hint}</div>
          </div>
        {/each}
      </div>
    </div>
  {/each}

  <div class="hero-row">
    <button class="hero-btn" type="button" onclick={copyReport} disabled={copyState === "copying"} data-state={copyState}>
      {#if copyState === "ok"}
        <Check size={16}/>
        <span class="hero-text">
          <span class="hero-title">Copied · {copySize} KB</span>
          <span class="hero-sub">Paste it to Claude</span>
        </span>
      {:else if copyState === "err"}
        <AlertTriangle size={16}/>
        <span class="hero-text">
          <span class="hero-title">Copy failed</span>
          <span class="hero-sub">Check console for the error</span>
        </span>
      {:else}
        <ClipboardCopy size={16}/>
        <span class="hero-text">
          <span class="hero-title">Copy diagnostic report</span>
          <span class="hero-sub">{copyState === "copying" ? "Capturing…" : "Bundles state, recent events, profile, locks, conflicts"}</span>
        </span>
      {/if}
    </button>
  </div>

  <div class="list" bind:this={scroller}>
    {#if events.length === 0}
      <div class="empty">
        No events captured yet. Connect a server and edit a file to see the pipeline live.
      </div>
    {:else}
      <div style="height:{padTop}px"></div>
      {#each slice as e, i (e.seq)}
        {@const Icon = stageIcon(e.stage)}
        {@const v = stageVariant(e.stage, e.level)}
        {@const open = expanded === e.seq}
        {@const _i = i}
        <div class="row" class:open style="height:{open ? "auto" : ROW_H + "px"}">
          <button class="row-head" type="button" onclick={() => (expanded = open ? null : e.seq)} use:tooltip={open ? "Collapse" : "Expand"}>
            <span class="time mono">{fmtTime(e.at)}</span>
            <span class="kind" data-variant={v} use:tooltip={e.stage}>
              <Icon size={12}/>
            </span>
            <span class="stage mono">{e.stage}</span>
            <span class="lvl-pip" data-variant={levelVariant(e.level)}>{e.level}</span>
            <div class="text">
              <span class="label">{e.message}</span>
              {#if e.file || e.resource}
                <span class="detail mono">{e.resource ?? ""}{e.resource && e.file ? " · " : ""}{e.file ?? ""}</span>
              {/if}
            </div>
          </button>
          {#if open}
            <pre class="payload mono">{JSON.stringify({ at: e.at, seq: e.seq, fields: e.fields }, null, 2)}</pre>
          {/if}
        </div>
      {/each}
      <div style="height:{padBot}px"></div>
    {/if}
  </div>

  {#if diagnostics.paused}
    <div class="paused-banner mono">Capture paused</div>
  {/if}
</section>

<style>
  .diag {
    display: flex; flex-direction: column;
    flex: 1; min-height: 0;
    padding: 10px 14px 14px;
    background: var(--bg);
    color: var(--fg);
    position: relative;
  }

  .tile-group { margin-bottom: 10px; }
  .group-label {
    display: flex; align-items: center; gap: 10px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    padding: 2px 2px 6px;
  }
  .group-label::after {
    content: "";
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 6px;
  }
  .tile {
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 9px;
    display: flex; flex-direction: column; gap: 1px;
    min-width: 0;
    transition: border-color 140ms ease, background 140ms ease, transform 140ms ease;
  }
  .tile:hover {
    border-color: color-mix(in oklch, var(--accent) 20%, var(--border));
    transform: translateY(-1px);
  }
  .tile-label { font-size: var(--fs-xs); color: var(--fg-subtle); }
  .tile-value {
    font-size: var(--fs-lg); font-weight: 600;
    color: var(--fg);
    line-height: 1.2;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .tile-hint { font-size: 10px; color: var(--fg-faint); }
  /* Tone tints — warn/danger get both border + background so a non-zero
     metric pops out of the grid instead of needing the user to read every
     cell to find the one that matters. */
  .tile[data-tone="warn"] {
    border-color: color-mix(in oklch, var(--warn) 38%, var(--border));
    background: color-mix(in oklch, var(--warn) 7%, var(--bg-elev-1));
  }
  .tile[data-tone="warn"]   .tile-value { color: var(--warn); }
  .tile[data-tone="warn"]   .tile-label { color: color-mix(in oklch, var(--warn) 50%, var(--fg-subtle)); }
  .tile[data-tone="danger"] {
    border-color: color-mix(in oklch, var(--danger) 42%, var(--border));
    background: color-mix(in oklch, var(--danger) 8%, var(--bg-elev-1));
  }
  .tile[data-tone="danger"] .tile-value { color: var(--danger); }
  .tile[data-tone="danger"] .tile-label { color: color-mix(in oklch, var(--danger) 55%, var(--fg-subtle)); }
  .tile[data-tone="info"]   .tile-value { color: var(--accent); }

  .hero-row {
    display: flex; align-items: stretch; gap: 8px;
    margin-bottom: 10px;
  }
  .hero-btn {
    flex: 1;
    display: flex; align-items: center; gap: 12px;
    padding: 12px 16px;
    background: color-mix(in oklch, var(--accent) 14%, var(--bg-elev-1));
    border: 1px solid color-mix(in oklch, var(--accent) 45%, var(--border));
    border-radius: var(--radius);
    color: var(--fg);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 120ms, border-color 120ms;
  }
  .hero-btn:hover:not(:disabled) {
    background: color-mix(in oklch, var(--accent) 22%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--accent) 65%, var(--border));
  }
  .hero-btn:disabled { opacity: 0.7; cursor: progress; }
  .hero-btn[data-state="ok"] {
    background: color-mix(in oklch, var(--ok) 14%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--ok) 50%, var(--border));
  }
  .hero-btn[data-state="err"] {
    background: color-mix(in oklch, var(--danger) 14%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--danger) 50%, var(--border));
  }
  .hero-text { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
  .hero-title { color: var(--fg); font-size: var(--fs-md); font-weight: 600; }
  .hero-sub { color: var(--fg-muted); font-size: var(--fs-xs); }

  .list {
    flex: 1; min-height: 0; overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .row {
    display: flex; flex-direction: column;
    border-bottom: 1px solid var(--border);
  }
  .row:last-child { border-bottom: 0; }
  .row.open { background: var(--bg-elev-1); }
  .row-head {
    display: grid;
    grid-template-columns: 92px 28px 130px 50px 1fr;
    gap: 10px;
    align-items: center;
    padding: 0 12px;
    height: 36px;
    background: transparent;
    border: 0;
    color: inherit;
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
    width: 100%;
  }
  .row-head:hover { background: var(--surface-hover); }
  .time { color: var(--fg-subtle); font-size: var(--fs-xs); }
  .kind {
    width: 22px; height: 22px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-2); color: var(--fg-muted);
  }
  .kind[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kind[data-variant="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .kind[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
  .kind[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }
  .stage {
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .lvl-pip {
    display: inline-flex; align-items: center; justify-content: center;
    height: 16px; padding: 0 6px;
    border-radius: 8px;
    background: var(--bg-elev-3);
    color: var(--fg-subtle);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .lvl-pip[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .lvl-pip[data-variant="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .lvl-pip[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
  .lvl-pip[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }
  .text { display: flex; flex-direction: column; min-width: 0; }
  .label {
    color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .detail {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .payload {
    margin: 0; padding: 8px 12px;
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    border-top: 1px solid var(--border);
    white-space: pre-wrap;
    word-break: break-all;
  }

  .empty {
    padding: 24px;
    color: var(--fg-muted);
    text-align: center;
    font-size: var(--fs-sm);
  }

  .paused-banner {
    position: absolute;
    bottom: 22px; left: 50%; transform: translateX(-50%);
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 4px 10px;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    box-shadow: var(--shadow);
  }
</style>
