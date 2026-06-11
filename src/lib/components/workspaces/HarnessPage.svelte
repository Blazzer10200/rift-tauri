<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Copy, Check, Trash2, Radio, Cpu, GitBranch, Zap, Clock, RotateCw, Layers, History as HistoryIcon, MessageCircle, Gauge, Boxes } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import CostPage from "./CostPage.svelte";
  import SwarmPage from "./SwarmPage.svelte";
  import { effortToFlag } from "../../state/assistant/helpers";
  import { tooltip } from "$lib/actions/tooltip";
  import { SessionTelemetry } from "../../state/assistant/telemetry";
  import {
    listSessionLogs, loadSessionLog, deleteSessionLog,
    type SessionLogMeta, type SessionSnapshot,
  } from "../../state/assistant/sessionLog";

  type DiagEvent = {
    at: string; seq: number; stage: string; level: string;
    resource: string | null; file: string | null; message: string; fields?: unknown;
  };

  // ── formatters ──
  function fmtTok(n: number | null | undefined): string {
    if (n == null) return "—";
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return String(n);
  }
  function fmtUsd(n: number | null | undefined): string {
    if (n == null) return "—";
    return "$" + (n < 1 ? n.toFixed(3) : n.toFixed(2));
  }
  function fmtMs(n: number | null | undefined): string {
    if (n == null) return "—";
    return n >= 1000 ? (n / 1000).toFixed(1) + "s" : Math.round(n) + "ms";
  }
  function fmtDur(ms: number | null | undefined): string {
    if (ms == null) return "—";
    const s = Math.floor(ms / 1000);
    if (s < 60) return s + "s";
    const m = Math.floor(s / 60);
    if (m < 60) return m + "m " + (s % 60) + "s";
    const h = Math.floor(m / 60);
    return h + "h " + (m % 60) + "m";
  }
  function fmtTime(iso: string): string {
    const d = new Date(iso);
    return isNaN(d.getTime()) ? "--:--:--" : d.toTimeString().slice(0, 8);
  }
  function fmtDate(ts: number | null | undefined): string {
    if (ts == null) return "—";
    const d = new Date(ts);
    if (isNaN(d.getTime())) return "—";
    return d.toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }
  // Compact relative age for the session strip ("3m", "2h", "Mon", "Apr 4").
  function fmtAgo(ts: number): string {
    const diff = Date.now() - ts;
    const m = Math.floor(diff / 60000);
    if (m < 1) return "now";
    if (m < 60) return m + "m";
    const h = Math.floor(m / 60);
    if (h < 24) return h + "h";
    const d = new Date(ts);
    const days = Math.floor(h / 24);
    if (days < 7) return d.toLocaleDateString(undefined, { weekday: "short" });
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
  function cleanPath(p: string | null | undefined): string {
    return p ? p.replace(/^\\\\\?\\/, "").split(/[\\/]/).pop() || p : "—";
  }
  // Per-model hue (app convention: sonnet=blue, opus=purple, haiku=teal).
  function modelHue(name: string): number {
    if (name.includes("haiku")) return 175;
    if (name.includes("opus")) return 280;
    if (name.includes("sonnet")) return 225;
    return 163;
  }
  function shortModel(id: string): string {
    return id.replace(/^claude-/, "").replace(/-(\d)-(\d)$/, " $1.$2");
  }

  const live = $derived(assistant.streaming);

  // Harness sub-tab: live telemetry dashboard vs the cross-session cost cockpit
  // (idea-phase-plan §1e). Kept here (not a 5th workspace) to preserve the
  // 4-workspace IA invariant.
  let subtab = $state<"telemetry" | "cost" | "swarm">("telemetry");

  // ── Active-conversation context (the hero gauge is intentionally per-tab:
  //    it measures how full the CURRENT conversation is, not the session).
  //    Live-only — past sessions have no live ctx, so the hero swaps to a
  //    session-overview panel for them. ──
  const lt = $derived(assistant.lastTurnUsage);
  const su = $derived(assistant.sessionUsage);
  const ctxPct = $derived(Math.min(100, assistant.ctxPct));
  const ctxZone = $derived(ctxPct < 60 ? "ok" : ctxPct < 85 ? "warn" : "hot");

  // ── Session source selection ────────────────────────────────────────────
  //    Every cumulative cell reads from `source` (a snapshot), so the same
  //    dashboard renders the LIVE session, any persisted PAST session, or an
  //    empty placeholder (the "all" aggregate uses its own section). EMPTY_SNAP
  //    is a throwaway telemetry snapshot — gives the exact zero-filled summary
  //    shape so every derived stays non-null without a hand-written stub. ──
  const EMPTY_SNAP = new SessionTelemetry().snapshot() as SessionSnapshot;
  let view = $state<"live" | "all" | string>("live");
  // Diagnostic cards (reliability / session details / tools granted) collapse by
  // default so the core dashboard fits one viewport with no scroll; one click reveals.
  let showDetails = $state(false);
  let sessions = $state<SessionLogMeta[]>([]);
  let loadedSnaps = $state<Record<string, SessionSnapshot>>({});

  const liveId = $derived(assistant.telemetry.id);
  const isLive = $derived(view === "live");
  const isAll = $derived(view === "all");

  // Re-fold the live snapshot whenever a turn lands (`turns[]` is mutated
  // imperatively, so `void su.turns` is the tracked trigger). Also re-fold when
  // streaming flips: the turn count bumps on the `result` event, but the final
  // turn's `doneAt` is written later in the stream-end handler (same sync block
  // as `streaming = false`). Without this, `doneAt`-derived summary metrics
  // (avg turn, output t/s) stay stale for the last turn.
  const liveSnap = $derived.by(() => { void su.turns; void assistant.streaming; return assistant.telemetry.snapshot() as SessionSnapshot; });
  const source = $derived.by((): SessionSnapshot => {
    if (view === "live") return liveSnap;
    if (view === "all") return EMPTY_SNAP;
    return loadedSnaps[view] ?? EMPTY_SNAP;
  });
  const sessionLoading = $derived(!isLive && !isAll && !loadedSnaps[view]);
  // Past sessions = persisted minus the live one (the Live pill represents it).
  const pastSessions = $derived(sessions.filter((s) => s.id !== liveId));

  // Lazy-load a past snapshot on first selection; cache it.
  $effect(() => {
    const v = view;
    if (v === "live" || v === "all" || loadedSnaps[v]) return;
    void loadSessionLog(v).then((snap) => {
      if (snap) loadedSnaps = { ...loadedSnaps, [v]: snap };
    });
  });

  async function refreshSessions() { sessions = await listSessionLogs(); }
  async function removeSession(id: string) {
    await deleteSessionLog(id);
    if (view === id) view = "live";
    const next = { ...loadedSnaps };
    delete next[id];
    loadedSnaps = next;
    await refreshSessions();
  }

  // Keep the strip fresh as the live session persists (debounced re-list).
  let relistTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    void su.turns;
    if (relistTimer) clearTimeout(relistTimer);
    relistTimer = setTimeout(() => { void refreshSessions(); }, 1800);
  });

  // ── Cumulative metrics (sourced) ──
  const sum = $derived(source.summary);
  // One "no data yet" semantic for the whole KPI rail: dim — until the first turn.
  const kpiFresh = $derived(sum.totalTurns === 0);
  // Spread into a fresh array so downstream folds (tok / cacheEff / turnViz)
  // invalidate per turn: telemetry mutates `turns` in place, so the bare
  // reference is === across snapshots and memoized deriveds never recompute
  // (token flow / cache / timeline would stick at their first-seen value).
  // Live uptime ticks off a 1s clock (the snapshot's durationMs is frozen at
  // the last turn's fold, so an idle session would otherwise under-report its
  // age). Archived sessions keep their true recorded duration.
  let nowTick = $state(Date.now());
  const uptime = $derived(isLive ? Math.max(0, nowTick - liveSnap.startedAt) : source.durationMs);
  const srcTurns = $derived([...source.turns]);
  // `?? {}` guards keep a partial/legacy stored snapshot (missing a summary
  // sub-object) from throwing in Object.entries and breaking the whole page.
  const byModel = $derived(Object.entries(sum.byModel ?? {}));
  const toolRows = $derived(Object.entries(sum.toolNameCounts ?? {}).sort((a, b) => b[1] - a[1]));
  const toolMax = $derived(toolRows.length ? toolRows[0][1] : 1);

  // Session token totals — folded straight off the turn records (every turn
  // with usage, incl. ones byModel skips) so Token flow / cache rate match the
  // raw /diag export exactly.
  const tok = $derived.by(() => {
    let input = 0, output = 0, cacheRead = 0, cacheCreate = 0;
    for (const t of srcTurns) {
      const u = t.resultUsage || t.envelopeUsage;
      if (!u) continue;
      input += u.input; output += u.output; cacheRead += u.cacheRead; cacheCreate += u.cacheCreate;
    }
    return { input, output, cacheRead, cacheCreate };
  });
  const tokTotal = $derived(tok.input + tok.output + tok.cacheRead + tok.cacheCreate);
  const cacheEff = $derived.by(() => {
    const seen = tok.input + tok.cacheRead;
    return seen > 0 ? (tok.cacheRead / seen) * 100 : 0;
  });
  const tokMax = $derived(Math.max(1, tok.input, tok.output, tok.cacheRead, tok.cacheCreate));
  const tokenBars = $derived([
    { k: "Input",       v: tok.input,       hue: 232 },
    { k: "Output",      v: tok.output,      hue: 165 },
    { k: "Cache read",  v: tok.cacheRead,   hue: 200 },
    { k: "Cache write", v: tok.cacheCreate, hue: 252 },
  ]);

  // ── Per-turn timeline (the richest signal) ──
  const turnViz = $derived.by(() => {
    const rows = srcTurns.map((t, i) => {
      const u = t.resultUsage || t.envelopeUsage;
      return {
        i: i + 1,
        model: t.modelId || t.model,
        cost: t.costUsd,
        dur: t.doneAt != null ? t.doneAt - t.ts : null,
        ttfp: t.firstPaintAt != null ? t.firstPaintAt - t.ts : null,
        // Silent pre-paint stall not attributable to thinking (spawn/prefill/queue).
        deadWait: t.firstPaintAt != null ? Math.max(0, (t.firstPaintAt - t.ts) - t.thinkingTotalMs) : null,
        tools: t.toolUses.length,
        think: t.thinkingCount > 0,
        thinkMs: t.thinkingTotalMs,
        out: u?.output ?? 0,
        end: t.endKind ?? "success",
      };
    });
    const maxDur = Math.max(1, ...rows.map((r) => r.dur ?? 0));
    return { rows, maxDur };
  });
  const thinkMsTotal = $derived.by(() => {
    let s = 0;
    for (const t of srcTurns) s += t.thinkingTotalMs || 0;
    return s;
  });
  const resolvedModelId = $derived.by(() => {
    for (let i = srcTurns.length - 1; i >= 0; i--) if (srcTurns[i].modelId) return srcTurns[i].modelId;
    return null;
  });

  const effortFlag = $derived(effortToFlag(assistant.thinkingEffort, assistant.effectiveModel));
  const writeOK = $derived(assistant.trustLevel !== "readonly");

  // Reasoning footprint for the hero footer (real per-turn breakdown, live).
  const ltBreak = $derived([
    { k: "in",     v: lt?.input },
    { k: "out",    v: lt?.output },
    { k: "cached", v: lt ? lt.cacheRead + lt.cacheCreate : null },
  ]);

  // ── Timing / reliability / config rows ──
  const timeRows = $derived([
    { k: "avg first paint", v: fmtMs(sum.avgTtfpMs) },
    { k: "avg dead wait",   v: fmtMs(sum.avgDeadWaitMs), warn: (sum.avgDeadWaitMs ?? 0) > 8000 },
    { k: "avg turn",        v: fmtMs(sum.avgDoneMs) },
    { k: "reasoning time",  v: fmtMs(thinkMsTotal || null) },
    { k: "tool vs model",   v: (sum.totalToolActiveMs || sum.totalModelMs) ? `${fmtMs(sum.totalToolActiveMs || null)} / ${fmtMs(sum.totalModelMs || null)}` : "—" },
    { k: "thinking turns",  v: String(sum.thinkingTurns) },
    { k: "max parallel",    v: sum.mostParallelTurn ? sum.mostParallelTurn.maxConcurrentTools + "×" : "—" },
    { k: "cold-start cache", v: sum.coldStartCacheCreate != null ? fmtTok(sum.coldStartCacheCreate) + " tok" : "—" },
  ]);
  const relRows = $derived([
    { k: "stale cache",   v: sum.staleCacheTurns,                    warn: sum.staleCacheTurns > 0 },
    { k: "tool errors",   v: sum.toolErrorTotal,                     warn: sum.toolErrorTotal > 0 },
    { k: "blank turns",   v: sum.blankTurns,                         warn: sum.blankTurns > 0 },
    { k: "env fallback",  v: sum.envelopeFallbacks,                  warn: sum.envelopeFallbacks > 0 },
    { k: "session lost",  v: sum.eventCounts?.["session.lost"] ?? 0,   warn: (sum.eventCounts?.["session.lost"] ?? 0) > 0 },
    { k: "model swaps",   v: sum.eventCounts?.["model.change"] ?? 0,   warn: false },
  ]);
  // When nothing is flagged, collapse the 6-cell grid to one "all clear" line —
  // a wall of zeroes reads as noise; the absence of warnings is the signal.
  const relClean = $derived(relRows.every((r) => !r.warn));
  // Config adapts: live = resolved-by-CLI runtime state; past = what the
  // snapshot recorded (model/workspace) + the session's own shape.
  const cfgRows = $derived(isLive
    ? [
        { k: "Model",          v: resolvedModelId ? shortModel(resolvedModelId) : assistant.effectiveModel },
        { k: "Thinking",       v: assistant.thinkingEffort + (effortFlag ? ` · ${effortFlag}` : "") },
        { k: "Permission",     v: assistant.permissionMode },
        { k: "Trust",          v: assistant.trustLevel },
        { k: "Context window", v: fmtTok(assistant.ctxWindow) },
        { k: "Auto-compact",   v: assistant.autoCompactThreshold ? Math.round(assistant.autoCompactThreshold * 100) + "%" : "off" },
        { k: "CLI version",    v: assistant.auth?.cliVersion ?? "—" },
        { k: "Install",        v: assistant.auth?.installMethod ?? "—" },
        { k: "Auth",           v: assistant.auth?.subscriptionType || assistant.auth?.authMethod || "—" },
      ]
    : [
        { k: "Model",     v: source.model ? shortModel(source.model) : (resolvedModelId ? shortModel(resolvedModelId) : "—") },
        { k: "Workspace", v: cleanPath(source.workspace) },
        { k: "Started",   v: fmtDate(source.startedAt) },
        { k: "Ended",     v: fmtDate(source.capturedAt) },
        { k: "Duration",  v: fmtDur(source.durationMs) },
        { k: "Turns",     v: String(sum.totalTurns) },
        { k: "Tool calls", v: String(sum.toolCallTotal) },
        { k: "Session",   v: typeof view === "string" ? view.slice(0, 8) : "—" },
      ]);

  // ── Gauge geometry ──
  const R = 82, C = 2 * Math.PI * 82;
  const ctxDash = $derived(C * (1 - ctxPct / 100));
  // Zone threshold ticks on the context ring (60% = filling, 85% = near-full).
  function tickAt(pct: number) {
    const a = (-90 + (pct / 100) * 360) * (Math.PI / 180);
    return { x1: 100 + (R - 9) * Math.cos(a), y1: 100 + (R - 9) * Math.sin(a),
             x2: 100 + (R + 9) * Math.cos(a), y2: 100 + (R + 9) * Math.sin(a) };
  }
  const ticks = [tickAt(60), tickAt(85)];

  // ── All-sessions aggregate ──
  const allAgg = $derived.by(() => {
    let cost = 0, turns = 0, tools = 0, dur = 0;
    for (const s of sessions) { cost += s.totalCostUsd; turns += s.totalTurns; tools += s.toolCallTotal; dur += s.durationMs; }
    return { count: sessions.length, cost, turns, tools, dur };
  });
  const allMaxCost = $derived(Math.max(0.0001, ...sessions.map((s) => s.totalCostUsd)));
  const allChrono = $derived([...sessions].sort((a, b) => a.startedAt - b.startedAt));
  const allByModel = $derived.by(() => {
    const m: Record<string, number> = {};
    for (const s of sessions) { const k = s.model || "unknown"; m[k] = (m[k] ?? 0) + 1; }
    return Object.entries(m).sort((a, b) => b[1] - a[1]);
  });
  const allModelMax = $derived(allByModel.length ? allByModel[0][1] : 1);

  // ── Diagnostics live stream — only `log` + `system` stages fire post
  //    pure-assistant rip, so we surface everything (no dead-stage toggle).
  //    Live-only: the stream is the running backend, not a frozen session. ──
  const LOG_CAP = 160;
  let log = $state<DiagEvent[]>([]);
  let levelFilter = $state<"all" | "warn" | "error">("all");
  let textFilter = $state("");
  let logCopied = $state(false);
  let logCopiedTimer: ReturnType<typeof setTimeout> | null = null;

  function levelTone(l: string): string {
    if (l === "error") return "danger";
    if (l === "warn") return "warn";
    if (l === "info") return "info";
    return "subtle";
  }
  const shownLog = $derived(
    log.filter((ev) => {
      if (levelFilter === "error" && ev.level !== "error") return false;
      if (levelFilter === "warn" && !(ev.level === "warn" || ev.level === "error")) return false;
      if (textFilter) {
        const q = textFilter.toLowerCase();
        if (!`${ev.message} ${ev.stage} ${ev.resource ?? ""}`.toLowerCase().includes(q)) return false;
      }
      return true;
    }).slice(-80),
  );

  async function copyLog() {
    try {
      await navigator.clipboard.writeText(JSON.stringify(shownLog, null, 2));
      logCopied = true;
      if (logCopiedTimer) clearTimeout(logCopiedTimer);
      logCopiedTimer = setTimeout(() => { logCopied = false; logCopiedTimer = null; }, 1400);
    } catch (e) { console.error("copy log failed", e); }
  }

  onMount(() => {
    let alive = true;
    let un: UnlistenFn | null = null;
    void listen<DiagEvent>("diag://event", (e) => {
      log = [...log, e.payload].slice(-LOG_CAP);
    }).then((u) => { if (alive) un = u; else u(); })
      .catch((err) => console.warn("diag listen failed", err));
    void assistant.refreshAuth().catch(() => {});
    void refreshSessions();
    // 1s heartbeat so the live session's uptime/duration advances while idle;
    // reads nowTick only in the live branch of `uptime`, so archived views
    // never recompute off it.
    const tick = setInterval(() => { if (isLive) nowTick = Date.now(); }, 1000);
    return () => {
      alive = false;
      un?.();
      clearInterval(tick);
      if (logCopiedTimer) { clearTimeout(logCopiedTimer); logCopiedTimer = null; }
      if (relistTimer) { clearTimeout(relistTimer); relistTimer = null; }
    };
  });
</script>

<div class="hwrap">
  <nav class="hsubtabs">
    <button class="hsub" class:on={subtab === "telemetry"} type="button" onclick={() => (subtab = "telemetry")}><Zap size={14} /> Telemetry</button>
    <button class="hsub" class:on={subtab === "cost"} type="button" onclick={() => (subtab = "cost")}><Gauge size={14} /> Cost</button>
    <button class="hsub" class:on={subtab === "swarm"} type="button" onclick={() => (subtab = "swarm")}><Boxes size={14} /> Swarm</button>
  </nav>
  {#if subtab === "cost"}
    <CostPage />
  {:else if subtab === "swarm"}
    <SwarmPage />
  {:else}
<div class="dash" data-live={live && isLive}>
  <!-- ── Header strip ── -->
  <header class="dhead">
    <div class="dhead-l">
      <div class="dhead-title">Harness <span class="dhead-spark"><Zap size={15} /></span></div>
      <div class="dhead-sub">
        {#if isAll}All logged sessions · {allAgg.count} logged
        {:else if isLive}Live Claude Code telemetry · this session
        {:else}Archived session · {fmtDate(source.startedAt)}{/if}
      </div>
    </div>
    <div class="dhead-chips">
      {#if isLive && assistant.auth}
        <span class="chip" class:ok={assistant.auth.pill === "green"} class:warn={assistant.auth.pill === "yellow" || assistant.auth.pill === "red"}><span class="chip-dot"></span>{assistant.auth.summary}</span>
      {/if}
      {#if !isAll}
        <span class="chip"><Cpu size={13} /> {isLive ? assistant.effectiveModel + (effortFlag ? ` · ${effortFlag}` : "") : (source.model ? shortModel(source.model) : "—")}</span>
        <span class="chip"><GitBranch size={13} /> {cleanPath(isLive ? assistant.workspace.current : source.workspace)}</span>
        <span class="chip"><Clock size={13} /> {fmtDur(uptime)}</span>
      {/if}
      {#if isLive}
        <span class="chip" class:livechip={live}><Radio size={13} /> {live ? "Streaming" : "Idle"}</span>
      {:else if !isAll}
        <span class="chip archived"><HistoryIcon size={13} /> archived</span>
      {/if}
    </div>
  </header>

  <!-- ── Session selector strip ── -->
  <div class="sesh">
    <button class="sesh-pill" class:on={isLive} type="button" onclick={() => (view = "live")}>
      <span class="sesh-led" class:beat={live}></span>
      <span class="sesh-pill-l">Live</span>
      <span class="sesh-pill-meta">{liveSnap.summary.totalTurns}t · {fmtUsd(liveSnap.summary.totalCostUsd)}</span>
    </button>
    <span class="sesh-div"></span>
    <div class="sesh-recent">
      {#if pastSessions.length === 0}
        <span class="sesh-none">Only this session so far — past sessions appear here as you go.</span>
      {:else}
        {#each pastSessions.slice(0, 4) as s (s.id)}
          <button class="sesh-pill ses" class:on={view === s.id} type="button" onclick={() => (view = s.id)}
            use:tooltip={`${fmtDate(s.startedAt)} · ${shortModel(s.model || "—")}\n${s.totalTurns} turns · ${s.toolCallTotal} tools · ${fmtUsd(s.totalCostUsd)} · ${fmtDur(s.durationMs)}`}>
            <span class="sesh-dot" style="--mH:{modelHue(s.model || '')}"></span>
            <span class="sesh-pill-l">{fmtAgo(s.startedAt)}</span>
            <span class="sesh-pill-meta">{s.totalTurns}t · {fmtUsd(s.totalCostUsd)}</span>
          </button>
        {/each}
      {/if}
    </div>
    <button class="sesh-pill sesh-all" class:on={isAll} type="button" onclick={() => (view = "all")} use:tooltip={"Browse all sessions"}>
      <Layers size={12} />
      <span class="sesh-pill-l">All</span>
      <span class="sesh-pill-meta">{sessions.length}</span>
      <span class="sesh-all-arrow">→</span>
    </button>
    <button class="sesh-refresh" type="button" onclick={refreshSessions} use:tooltip={"Refresh sessions"}><RotateCw size={13} /></button>
  </div>

  {#if isAll}
    <!-- ── All-sessions aggregate ── -->
    <div class="bento">
      <section class="cell sq"><div class="sq-v">{allAgg.count}</div><div class="sq-k">sessions</div><div class="sq-glow"></div></section>
      <section class="cell sq"><div class="sq-v">{fmtUsd(allAgg.cost)}</div><div class="sq-k">total cost</div><div class="sq-glow"></div></section>
      <section class="cell sq"><div class="sq-v">{allAgg.turns}</div><div class="sq-k">total turns</div><div class="sq-glow"></div></section>
      <section class="cell sq"><div class="sq-v">{allAgg.tools}</div><div class="sq-k">tool calls</div><div class="sq-glow"></div></section>

      <section class="cell full tl">
        <div class="cell-head">
          <span class="cell-title">Activity over time</span>
          <span class="cell-meta">{allChrono.length} sessions · bar = cost · click to open</span>
        </div>
        {#if allChrono.length === 0}
          <div class="empty tl-empty">No sessions logged yet.</div>
        {:else}
          <div class="tl-track">
            <span class="tl-grid"></span>
            {#each allChrono as s (s.id)}
              <button class="tl-bar tl-btn" type="button"
                aria-label={`Open session ${fmtDate(s.startedAt)}`}
                style="height:{Math.max(7, (s.totalCostUsd / allMaxCost) * 100)}%"
                onclick={() => (view = s.id)}
                use:tooltip={`${fmtDate(s.startedAt)} · ${shortModel(s.model || "—")}\n${s.totalTurns} turns · ${fmtUsd(s.totalCostUsd)} · ${fmtDur(s.durationMs)}`}></button>
            {/each}
          </div>
          <div class="tl-axis"><span>oldest</span><span>latest →</span></div>
        {/if}
      </section>

      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">Sessions by model</span></div>
        {#if allByModel.length === 0}
          <div class="empty">No sessions yet.</div>
        {:else}
          <div class="bars">
            {#each allByModel as [name, count] (name)}
              <div class="bar-row">
                <span class="bar-k mono">{shortModel(name)}</span>
                <span class="bar-track"><span class="bar-fill" style="width:{Math.max(2, (count / allModelMax) * 100)}%; --barH:{modelHue(name)}"></span></span>
                <span class="bar-v mono">{count}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">Lifetime</span><span class="cell-meta">across all logs</span></div>
        <div class="stat6">
          <div class="mini"><div class="mini-v mono">{fmtDur(allAgg.dur)}</div><div class="mini-k">total time</div></div>
          <div class="mini"><div class="mini-v mono">{allAgg.count ? fmtUsd(allAgg.cost / allAgg.count) : "—"}</div><div class="mini-k">avg / session</div></div>
          <div class="mini"><div class="mini-v mono">{allAgg.count ? Math.round(allAgg.turns / allAgg.count) : "—"}</div><div class="mini-k">avg turns</div></div>
        </div>
      </section>

      <section class="cell full">
        <div class="cell-head"><span class="cell-title">Session log</span><span class="cell-meta">newest first</span></div>
        {#if sessions.length === 0}
          <div class="empty">No sessions logged yet.</div>
        {:else}
          <div class="slog">
            {#each sessions as s (s.id)}
              <div class="slog-row" class:islive={s.id === liveId}>
                <button class="slog-open" type="button" onclick={() => (view = s.id === liveId ? "live" : s.id)}>
                  <span class="slog-dot" style="--mH:{modelHue(s.model || '')}"></span>
                  <span class="slog-when">{fmtDate(s.startedAt)}</span>
                  {#if s.id === liveId}<span class="slog-livetag">live</span>{/if}
                  <span class="slog-model mono">{shortModel(s.model || "—")}</span>
                  <span class="slog-stat mono">{s.totalTurns}t</span>
                  <span class="slog-stat mono">{s.toolCallTotal} tools</span>
                  <span class="slog-stat mono">{fmtDur(s.durationMs)}</span>
                  <span class="slog-cost mono">{fmtUsd(s.totalCostUsd)}</span>
                </button>
                <button class="slog-del" type="button" onclick={() => removeSession(s.id)} use:tooltip={"Delete session log"}><Trash2 size={13} /></button>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    </div>

  {:else if sessionLoading}
    <div class="loadbox"><span class="caret"></span> loading session…</div>

  {:else}
    <!-- ── Per-session bento (live or archived) ── -->
    <div class="bento">

      <!-- KPI rail (session-wide headline metrics) -->
      <section class="cell full kpi-rail">
        <div class="kpi"><span class="kpi-v" class:nodata={kpiFresh}>{kpiFresh ? "—" : fmtUsd(sum.totalCostUsd)}</span><span class="kpi-k">session cost</span></div>
        <div class="kpi"><span class="kpi-v" class:nodata={kpiFresh}>{kpiFresh ? "—" : sum.totalTurns}</span><span class="kpi-k">turns</span></div>
        <div class="kpi"><span class="kpi-v" class:nodata={kpiFresh}>{kpiFresh ? "—" : sum.toolCallTotal}</span><span class="kpi-k">tool calls</span></div>
        <div class="kpi"><span class="kpi-v" class:nodata={sum.outputTokensPerSec == null}>{sum.outputTokensPerSec ?? "—"}{#if sum.outputTokensPerSec}<span class="kpi-u"> t/s</span>{/if}</span><span class="kpi-k">output speed</span></div>
        <div class="kpi"><span class="kpi-v" class:nodata={kpiFresh}>{#if kpiFresh}—{:else}{cacheEff.toFixed(0)}<span class="kpi-u">%</span>{/if}</span><span class="kpi-k">cache hit</span></div>
        <div class="kpi"><span class="kpi-v" class:nodata={sum.avgTtfpMs == null}>{fmtMs(sum.avgTtfpMs)}</span><span class="kpi-k">avg first paint</span></div>
      </section>

      {#if isLive}
        <!-- HERO: context ring (active conversation) -->
        <section class="cell hero" class:waiting={!lt}>
          <div class="hero-glow"></div>
          <div class="hero-tag">active conversation</div>
          {#if !lt}
            <!-- No turn yet: a calm placeholder, not a dash-filled gauge. -->
            <div class="hero-wait">
              <div class="hero-wait-ring">
                <svg viewBox="0 0 200 200" class="gauge-svg"><circle cx="100" cy="100" r={R} class="gauge-track ghost" /></svg>
                <span class="hero-wait-ico"><MessageCircle size={26} strokeWidth={1.75} /></span>
              </div>
              <div class="hero-wait-t">Awaiting first turn</div>
              <div class="hero-wait-s">Context fills here once you send a message — capacity {fmtTok(assistant.ctxWindow)} tokens.</div>
            </div>
          {:else}
            <div class="gauge">
              <svg viewBox="0 0 200 200" class="gauge-svg">
                <defs>
                  <linearGradient id="ctxgrad" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0%" class="g0" />
                    <stop offset="100%" class="g1" />
                  </linearGradient>
                </defs>
                <circle cx="100" cy="100" r={R} class="gauge-track" />
                {#each ticks as t (t.x1)}
                  <line x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2} class="gauge-tick" />
                {/each}
                <circle cx="100" cy="100" r={R} class="gauge-fill"
                  stroke="url(#ctxgrad)"
                  stroke-dasharray={C} stroke-dashoffset={ctxDash}
                  transform="rotate(-90 100 100)" />
              </svg>
              <div class="gauge-mid">
                <div class="gauge-pct" data-zone={ctxZone}>{ctxPct.toFixed(0)}<span class="gauge-pct-u">%</span></div>
                <div class="gauge-cap">context used</div>
              </div>
            </div>
            <div class="hero-foot">
              <div class="hero-tok">{fmtTok(assistant.ctxTokens)} <span class="dim">/ {fmtTok(assistant.ctxWindow)} tokens</span></div>
              <div class="hero-zone" data-zone={ctxZone}>{ctxZone === "ok" ? "roomy" : ctxZone === "warn" ? "filling" : "near full"}</div>
              <div class="hero-last">
                <span class="hero-last-lbl">last turn</span>
                {#each ltBreak as b (b.k)}
                  <span class="hero-last-stat"><span class="hero-last-v mono">{fmtTok(b.v)}</span><span class="hero-last-k">{b.k}</span></span>
                {/each}
              </div>
            </div>
          {/if}
        </section>
      {:else}
        <!-- HERO (archived): session overview — headline + at-a-glance stats
             that complement (don't duplicate) the KPI rail above. -->
        <section class="cell hero ov">
          <div class="hero-glow"></div>
          <div class="ov-main">
            <div class="ov-head">
              <span class="ov-mdl-dot" style="--mH:{modelHue(source.model || resolvedModelId || '')}"></span>
              <span class="ov-mdl">{source.model ? shortModel(source.model) : (resolvedModelId ? shortModel(resolvedModelId) : "—")}</span>
            </div>
            <div class="ov-cost">{fmtUsd(sum.totalCostUsd)}</div>
            <div class="ov-cap">session cost</div>
            <div class="ov-when">{fmtDate(source.startedAt)} · {fmtDur(source.durationMs)}</div>
          </div>
          <div class="ov-stats">
            <div class="ov-stat"><span class="ov-stat-v mono">{fmtTok(tokTotal)}</span><span class="ov-stat-k">total tokens</span></div>
            <div class="ov-stat"><span class="ov-stat-v mono">{fmtMs(thinkMsTotal || null)}</span><span class="ov-stat-k">reasoning</span></div>
            <div class="ov-stat"><span class="ov-stat-v mono">{fmtMs(sum.avgDoneMs)}</span><span class="ov-stat-k">avg turn</span></div>
            <div class="ov-stat"><span class="ov-stat-v mono">{sum.mostParallelTurn ? sum.mostParallelTurn.maxConcurrentTools + "×" : "—"}</span><span class="ov-stat-k">peak parallel</span></div>
          </div>
        </section>
      {/if}

      <!-- token flow bars -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">Token flow</span><span class="cell-meta">{fmtTok(tokTotal)} total</span></div>
        <div class="bars">
          {#each tokenBars as b (b.k)}
            <div class="bar-row">
              <span class="bar-k">{b.k}</span>
              <span class="bar-track">
                <span class="bar-fill" class:flow={live && isLive} class:zero={b.v === 0} style="width:{b.v === 0 ? 0 : Math.max(2.5, (b.v / tokMax) * 100)}%; --barH:{b.hue}"></span>
              </span>
              <span class="bar-v mono">{fmtTok(b.v)}</span>
            </div>
          {/each}
        </div>
      </section>

      <!-- by model -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">By model</span></div>
        {#if byModel.length === 0}
          <div class="empty">No turns yet.</div>
        {:else}
          <div class="mdl-list">
            {#each byModel as [name, m] (name)}
              <div class="mdl-row" style="--mH:{modelHue(name)}">
                <span class="mdl-dot"></span>
                <span class="mdl-name mono">{shortModel(name)}</span>
                <span class="mdl-stats mono">{m.turns}t · {fmtUsd(m.costUsd)} · {fmtMs(m.avgTtfpMs)}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <!-- turn timeline (full width centerpiece) -->
      <section class="cell full tl">
        <div class="cell-head">
          <span class="cell-title">Turn timeline</span>
          <span class="cell-meta">{turnViz.rows.length} turns · bar = duration · ◦ reasoning</span>
        </div>
        {#if turnViz.rows.length === 0}
          <div class="empty tl-empty">No turns yet — send a message and each turn lands here as a bar.</div>
        {:else}
          <div class="tl-track">
            <span class="tl-grid"></span>
            {#each turnViz.rows.slice(-80) as r (r.i)}
              <span class="tl-bar" data-end={r.end}
                style="height:{Math.max(7, (r.dur ? r.dur / turnViz.maxDur : 0) * 100)}%"
                use:tooltip={`Turn ${r.i} · ${shortModel(r.model)}  ·  ${fmtUsd(r.cost)}\nttfp ${fmtMs(r.ttfp)} · dur ${fmtMs(r.dur)} · ${r.out} out\n${r.tools} tool${r.tools === 1 ? "" : "s"}${r.think ? ` · reasoned ${fmtMs(r.thinkMs)}` : ""}${r.deadWait && r.deadWait > 4000 ? ` · dead-wait ${fmtMs(r.deadWait)}` : ""}${r.end !== "success" ? `  · ${r.end}` : ""}`}>
                {#if r.think}<span class="tl-think"></span>{/if}
                {#if r.deadWait && r.deadWait > 8000}<span class="tl-dead"></span>{/if}
              </span>
            {/each}
          </div>
          <div class="tl-axis"><span>oldest</span><span>latest →</span></div>
        {/if}
      </section>

      <!-- tool activity bars -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">Tool activity</span>{#if sum.slowestTool}<span class="cell-meta">slowest {sum.slowestTool.name} · {fmtMs(sum.slowestTool.durationMs)}</span>{/if}</div>
        {#if toolRows.length === 0}
          <div class="empty">No tool calls yet.</div>
        {:else}
          <div class="bars">
            {#each toolRows.slice(0, 6) as [name, count] (name)}
              <div class="bar-row">
                <span class="bar-k mono">{name}</span>
                <span class="bar-track"><span class="bar-fill" style="width:{Math.max(2, (count / toolMax) * 100)}%; --barH:163"></span></span>
                <span class="bar-v mono">{count}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <!-- timing + reasoning -->
      <section class="cell full">
        <div class="cell-head"><span class="cell-title">Timing &amp; reasoning</span></div>
        <div class="stat6">
          {#each timeRows as r (r.k)}
            <div class="mini"><div class="mini-v mono" data-warn={r.warn}>{r.v}</div><div class="mini-k">{r.k}</div></div>
          {/each}
        </div>
      </section>

      <!-- Details toggle — collapses the diagnostic cards so the core fits one viewport -->
      <button class="cell full det-toggle" type="button" onclick={() => (showDetails = !showDetails)}>
        <span class="det-lbl">{showDetails ? "Hide" : "Show"} details</span>
        <span class="det-sub">reliability · session {isLive ? "config · diagnostics stream" : "details"}</span>
        <span class="det-caret" class:open={showDetails}>▾</span>
      </button>

      {#if showDetails}
      <!-- reliability / health -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">Reliability</span><span class="cell-meta">harness health</span></div>
        {#if relClean}
          <div class="rel-clear"><span class="rel-clear-dot"></span>All clear — no stale cache, tool errors, blank turns, or lost sessions.</div>
        {:else}
          <div class="stat6">
            {#each relRows as r (r.k)}
              <div class="mini"><div class="mini-v mono" data-warn={r.warn}>{r.v}</div><div class="mini-k">{r.k}</div></div>
            {/each}
          </div>
        {/if}
      </section>

      <!-- harness config -->
      <section class="cell wide">
        <div class="cell-head"><span class="cell-title">{isLive ? "Harness configuration" : "Session details"}</span><span class="cell-meta">{isLive ? "resolved by CLI" : "recorded"}</span></div>
        <div class="cfg">
          {#each cfgRows as r (r.k)}
            <div class="cfg-row"><span class="cfg-k">{r.k}</span><span class="cfg-v mono">{r.v}</span></div>
          {/each}
        </div>
      </section>

      <!-- tools granted strip -->
      <section class="cell full">
        <div class="cell-head"><span class="cell-title">Tools granted</span><span class="cell-meta">trust: {assistant.trustLevel}</span></div>
        <div class="grant-wrap">
          {#each ["read_file","list_dir","grep","git_status","git_diff","git_log","git_pull","git_commit","git_push"] as t (t)}
            {@const isWrite = t === "git_pull" || t === "git_commit" || t === "git_push"}
            {@const granted = !isWrite || writeOK}
            <span class="grant mono" class:write={isWrite} data-off={!granted}>{t}</span>
          {/each}
        </div>
      </section>

      <!-- diagnostics stream (live only) -->
      {#if isLive}
        <section class="cell full term">
          <div class="cell-head">
            <span class="cell-title"><span class="term-led" class:on={shownLog.length > 0}></span>Diagnostics stream</span>
            <div class="term-ctl">
              <div class="seg">
                {#each [{id:"all",l:"All"},{id:"warn",l:"Warn+"},{id:"error",l:"Err"}] as o (o.id)}
                  <button class="seg-btn" class:on={levelFilter === o.id} type="button" onclick={() => (levelFilter = o.id as typeof levelFilter)}>{o.l}</button>
                {/each}
              </div>
              <input class="term-in mono" type="text" placeholder="filter…" bind:value={textFilter} spellcheck="false" />
              <button class="ic-btn" type="button" onclick={copyLog} use:tooltip={"Copy as JSON"}>{#if logCopied}<Check size={14} />{:else}<Copy size={14} />{/if}</button>
              <button class="ic-btn" type="button" onclick={() => (log = [])} use:tooltip={"Clear"}><Trash2 size={14} /></button>
            </div>
          </div>
          <div class="term-body">
            {#if shownLog.length === 0}
              <div class="empty term-empty">waiting for backend events…<span class="caret"></span></div>
            {:else}
              {#each shownLog as ev (ev.seq)}
                <div class="tick">
                  <span class="tick-t mono">{fmtTime(ev.at)}</span>
                  <span class="tick-lvl mono" data-tone={levelTone(ev.level)}>{ev.level}</span>
                  <span class="tick-stage mono">{ev.stage}</span>
                  <span class="tick-msg">{ev.message}</span>
                </div>
              {/each}
            {/if}
          </div>
        </section>
      {/if}
      {/if}

    </div>
  {/if}
</div>
  {/if}
</div>

<style>
  /* ── Sub-tab shell (Telemetry | Cost) ── */
  .hwrap { flex: 1; min-height: 0; min-width: 0; display: flex; flex-direction: column; background: var(--bg); }
  .hsubtabs { display: flex; align-items: center; gap: 6px; padding: 10px 22px 0; flex: none; }
  .hsub { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 14px; border-radius: 999px; border: 1px solid var(--border); background: color-mix(in oklab, var(--surface) 60%, transparent); color: var(--fg-muted); font-size: var(--fs-xs); font-weight: 650; cursor: pointer; transition: border-color var(--dur-fast) var(--ease-soft), background var(--dur-fast) var(--ease-soft), color var(--dur-fast) var(--ease-soft); }
  .hsub:hover { border-color: var(--border-strong); color: var(--fg); }
  .hsub.on { border-color: var(--ghost-border); background: var(--accent-soft); color: var(--accent); }
  .hsub :global(svg) { opacity: 0.85; }

  .dash { position: relative; flex: 1; min-height: 0; min-width: 0; overflow-y: auto; overflow-x: hidden; background:
      radial-gradient(circle, color-mix(in oklab, var(--fg) 3%, transparent) 1px, transparent 1px) 0 0 / 26px 26px,
      radial-gradient(130% 90% at 50% -25%, color-mix(in oklab, var(--accent) 5%, transparent), transparent 52%),
      var(--bg); color: var(--fg); padding: 16px 22px 14px; }

  /* ── Header ── */
  .dhead { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; flex-wrap: wrap; margin-bottom: 14px; }
  .dhead-title { font-size: 23px; font-weight: 750; letter-spacing: -0.02em; display: flex; align-items: center; gap: 9px; }
  /* Spark only breathes while the session is live-streaming — motion means
     "something is happening", not perpetual decoration. */
  .dhead-spark { display: inline-flex; color: var(--fg-subtle); transition: color var(--dur-base) var(--ease-soft); }
  .dash[data-live="true"] .dhead-spark { color: var(--accent); animation: pulse 2s ease-in-out infinite; }
  .dhead-sub { font-size: var(--fs-xs); color: var(--fg-subtle); margin-top: 3px; }
  .dhead-chips { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .chip { display: inline-flex; align-items: center; gap: 6px; height: 27px; padding: 0 11px; border-radius: 999px; font-size: var(--fs-xs); font-weight: 600; border: 1px solid var(--border); background: color-mix(in oklab, var(--surface) 70%, transparent); color: var(--fg-muted); backdrop-filter: blur(8px); }
  .chip :global(svg) { color: var(--fg-subtle); }
  .chip.ok { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .chip.warn { background: var(--warn-soft); border-color: color-mix(in oklab, var(--warn) 28%, transparent); color: var(--warn); }
  .chip.archived { color: var(--fg-subtle); }
  .chip-dot { width: 7px; height: 7px; border-radius: 999px; background: currentColor; }
  .chip.livechip { color: var(--accent); border-color: var(--ghost-border); background: var(--accent-soft); }
  .chip.livechip :global(svg) { color: var(--accent); animation: pulse 2s ease-in-out infinite; }

  /* ── Session selector strip ── */
  .sesh { display: flex; align-items: center; gap: 7px; margin-bottom: 18px; min-width: 0; }
  /* Recent sessions — fixed strip, NO horizontal scroll. Caps to what fits the
     width (responsive count below); the rest live in the All view. */
  .sesh-recent { display: flex; align-items: center; gap: 7px; flex: 1; min-width: 0; overflow: hidden; }
  @media (max-width: 1280px) { .sesh-recent .ses:nth-child(n + 4) { display: none; } }
  @media (max-width: 1080px) { .sesh-recent .ses:nth-child(n + 3) { display: none; } }
  .sesh-all { flex: none; }
  .sesh-all-arrow { font-size: 12px; color: var(--fg-faint); margin-left: 1px; }
  .sesh-all.on .sesh-all-arrow { color: color-mix(in oklab, var(--accent) 70%, var(--fg)); }
  .sesh-div { width: 1px; height: 22px; background: var(--border); flex: none; }
  .sesh-pill { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 12px; border-radius: 999px; border: 1px solid var(--border); background: color-mix(in oklab, var(--surface) 60%, transparent); color: var(--fg-muted); font-size: var(--fs-xs); font-weight: 600; cursor: pointer; flex: none; white-space: nowrap; transition: border-color var(--dur-fast) var(--ease-soft), background var(--dur-fast) var(--ease-soft), color var(--dur-fast) var(--ease-soft); }
  .sesh-pill:hover { border-color: var(--border-strong); color: var(--fg); }
  .sesh-pill.on { border-color: var(--ghost-border); background: var(--accent-soft); color: var(--accent); }
  .sesh-pill :global(svg) { color: currentColor; opacity: 0.8; }
  .sesh-pill-l { font-weight: 650; }
  .sesh-pill-meta { font-family: var(--font-mono); font-size: 10px; color: var(--fg-subtle); }
  .sesh-pill.on .sesh-pill-meta { color: color-mix(in oklab, var(--accent) 80%, var(--fg)); }
  .sesh-led { width: 8px; height: 8px; border-radius: 50%; background: var(--fg-faint); flex: none; }
  .sesh-led.beat { background: var(--ok); box-shadow: 0 0 7px var(--ok); animation: pulse 2s ease-in-out infinite; }
  .sesh-dot { width: 8px; height: 8px; border-radius: 50%; flex: none; background: oklch(0.74 0.15 var(--mH)); box-shadow: 0 0 6px oklch(0.74 0.15 var(--mH) / 0.55); }
  .sesh-none { font-size: var(--fs-xs); color: var(--fg-subtle); padding: 0 4px; }
  .sesh-refresh { display: inline-grid; place-items: center; width: 30px; height: 30px; border-radius: 999px; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); cursor: pointer; flex: none; }
  .sesh-refresh:hover { color: var(--fg); border-color: var(--border-strong); }

  /* ── Bento grid ── */
  .bento { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; grid-auto-flow: dense; }
  .cell { position: relative; background: var(--surface); border: 1px solid var(--border); border-radius: 16px; padding: 11px 14px; overflow: hidden; min-width: 0; }
  .wide { grid-column: span 2; }
  .full { grid-column: 1 / -1; }

  /* Cells rise+fade in on mount, lightly staggered by DOM order. Because each
     view (live/all/archived) is a separate {#if} branch, switching remounts the
     grid — so this doubles as a crossfade between views, no extra wiring. */
  .bento > .cell, .bento > .det-toggle { animation: cellrise var(--dur-rise) var(--ease-page) both; }
  .bento > :nth-child(1) { animation-delay: 0ms; }
  .bento > :nth-child(2) { animation-delay: var(--stagger); }
  .bento > :nth-child(3) { animation-delay: calc(var(--stagger) * 2); }
  .bento > :nth-child(4) { animation-delay: calc(var(--stagger) * 3); }
  .bento > :nth-child(5) { animation-delay: calc(var(--stagger) * 4); }
  /* Cap the cascade so later cells (incl. the on-demand details reveal) don't
     sit invisible behind a long delay on click. */
  .bento > :nth-child(n + 6) { animation-delay: calc(var(--stagger) * 5); }

  .cell-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 9px; }
  .cell-title { font-size: var(--fs-sm); font-weight: 650; color: var(--fg); display: inline-flex; align-items: center; gap: 8px; }
  .cell-meta { font-size: 10.5px; color: var(--fg-subtle); font-family: var(--font-mono); }
  .empty { font-size: var(--fs-xs); color: var(--fg-subtle); padding: 10px 0; }
  .dim { color: var(--fg-subtle); }

  .loadbox { display: flex; align-items: center; gap: 9px; font-size: var(--fs-sm); color: var(--fg-subtle); font-family: var(--font-mono); padding: 50px 0; justify-content: center; }

  /* ── Hero context ring ── */
  .hero { grid-column: span 2; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; padding: 12px 16px; }
  .hero-tag { position: absolute; top: 14px; left: 16px; font-size: 9.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.09em; color: var(--fg-faint); z-index: 1; }
  /* Backlight — one soft radial behind the hero focal point (ring / cost),
     biased up to ~42% so it sits on the ring, not the footer. Kept OUTSIDE the
     gauge: nesting a blur() layer inside the gauge's stacking context made
     WebView2 composite it as an opaque black rect. Hero content is z-index:1,
     so z-index:0 keeps this behind it. */
  .hero-glow { position: absolute; top: 42%; left: 50%; transform: translate(-50%, -50%); width: 340px; height: 340px; border-radius: 50%; z-index: 0; filter: blur(32px); opacity: 0.5; transition: opacity var(--dur-rise) var(--ease-soft); pointer-events: none;
    background: radial-gradient(circle, color-mix(in oklab, var(--accent) 17%, transparent), transparent 66%); }
  .dash[data-live="true"] .hero-glow { opacity: 0.9; }
  .gauge { position: relative; width: 148px; height: 148px; z-index: 1; }

  /* Hero "awaiting first turn" — replaces the empty dash-filled gauge before any
     turn lands. Same footprint as the real gauge so nothing jumps on first use. */
  .hero.waiting { gap: 16px; }
  .hero-wait { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 13px; z-index: 1; padding: 6px 0; }
  .hero-wait-ring { position: relative; width: 148px; height: 148px; display: grid; place-items: center; }
  .hero-wait-ring .gauge-svg { position: absolute; inset: 0; }
  .gauge-track.ghost { stroke: color-mix(in oklab, var(--fg) 9%, transparent); stroke-width: 6; stroke-dasharray: 2.5 10; stroke-linecap: round; }
  .hero-wait-ico { display: grid; place-items: center; width: 56px; height: 56px; border-radius: 50%; color: var(--accent);
    background: color-mix(in oklab, var(--accent) 10%, transparent); border: 1px solid var(--accent-soft); }
  .hero-wait-t { font-size: var(--fs-md); font-weight: 650; color: var(--fg-2); letter-spacing: -0.01em; }
  .hero-wait-s { font-size: var(--fs-xs); color: var(--fg-subtle); text-align: center; max-width: 250px; line-height: 1.5; }
  .gauge-svg { width: 100%; height: 100%; }
  .gauge-track { fill: none; stroke: var(--bg-inset); stroke-width: 13; }
  .gauge-tick { stroke: color-mix(in oklab, var(--fg) 22%, transparent); stroke-width: 2; stroke-linecap: round; }
  .g0 { stop-color: oklch(0.66 0.16 var(--accent-h)); }
  .g1 { stop-color: oklch(0.85 0.13 var(--accent-h)); }
  .gauge-fill { fill: none; stroke-width: 13; stroke-linecap: round; transition: stroke-dashoffset var(--dur-slow) var(--ease-page); filter: drop-shadow(0 0 6px color-mix(in oklab, var(--accent) 45%, transparent)); }
  .gauge-mid { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .gauge-pct { font-size: 54px; font-weight: 760; letter-spacing: -0.03em; line-height: 1; color: var(--fg); font-variant-numeric: tabular-nums; }
  .gauge-pct[data-zone="warn"] { color: var(--warn); }
  .gauge-pct[data-zone="hot"] { color: var(--danger); }
  .gauge-pct-u { font-size: 22px; font-weight: 600; color: var(--fg-subtle); margin-left: 2px; }
  .gauge-cap { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 5px; text-transform: uppercase; letter-spacing: 0.06em; }
  .hero-foot { display: flex; flex-direction: column; align-items: center; gap: 5px; z-index: 1; width: 100%; max-width: 340px; }
  .hero-tok { font-size: var(--fs-sm); color: var(--fg-2); font-family: var(--font-mono); }
  .hero-zone { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; padding: 2px 10px; border-radius: 999px; }
  .hero-zone[data-zone="ok"] { color: var(--ok); background: var(--ok-soft); }
  .hero-zone[data-zone="warn"] { color: var(--warn); background: var(--warn-soft); }
  .hero-zone[data-zone="hot"] { color: var(--danger); background: color-mix(in oklab, var(--danger) 15%, transparent); }
  .hero-last { display: flex; align-items: center; justify-content: center; gap: 16px; margin-top: 2px; padding-top: 9px; width: 100%; border-top: 1px solid color-mix(in oklab, var(--border) 70%, transparent); }
  .hero-last-lbl { font-size: 9.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: var(--fg-faint); }
  .hero-last-stat { display: inline-flex; align-items: baseline; gap: 5px; }
  .hero-last-v { font-size: var(--fs-sm); font-weight: 650; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .hero-last-k { font-size: 10px; color: var(--fg-subtle); text-transform: uppercase; letter-spacing: 0.04em; }

  /* ── Hero (archived) session overview ── */
  /* Archived overview = horizontal split: headline (left) + at-a-glance grid
     (right), filling the span-2 cell instead of a lone centered number. */
  .ov { flex-direction: row; align-items: center; justify-content: space-between; gap: 20px; padding: 14px 22px; }
  .ov-main { display: flex; flex-direction: column; align-items: flex-start; gap: 3px; z-index: 1; min-width: 0; }
  .ov-head { display: inline-flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .ov-mdl-dot { width: 10px; height: 10px; border-radius: 50%; background: oklch(0.74 0.16 var(--mH)); box-shadow: 0 0 9px oklch(0.74 0.16 var(--mH) / 0.6); }
  .ov-mdl { font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); font-family: var(--font-mono); }
  .ov-cost { font-size: 46px; font-weight: 760; letter-spacing: -0.03em; line-height: 1; color: var(--fg); font-variant-numeric: tabular-nums; }
  .ov-cap { font-size: var(--fs-xs); color: var(--fg-muted); text-transform: uppercase; letter-spacing: 0.06em; }
  .ov-when { font-size: var(--fs-xs); color: var(--fg-subtle); font-family: var(--font-mono); margin-top: 4px; }
  .ov-stats { display: grid; grid-template-columns: repeat(2, 1fr); gap: 11px 26px; z-index: 1; padding-left: 22px; border-left: 1px solid color-mix(in oklab, var(--border) 70%, transparent); }
  .ov-stat { display: flex; flex-direction: column; gap: 2px; }
  .ov-stat-v { font-size: 17px; font-weight: 680; color: var(--fg); font-variant-numeric: tabular-nums; }
  .ov-stat-k { font-size: 10.5px; color: var(--fg-muted); text-transform: uppercase; letter-spacing: 0.04em; }

  /* ── KPI rail (session-wide headline metrics) ── */
  .kpi-rail { display: flex; flex-wrap: wrap; align-items: stretch; gap: 0; padding: 0; }
  .kpi { flex: 1 1 120px; display: flex; flex-direction: column; justify-content: center; gap: 3px; padding: 12px 18px; position: relative; }
  .kpi + .kpi { border-left: 1px solid color-mix(in oklab, var(--border) 70%, transparent); }
  .kpi-v { font-size: 24px; font-weight: 720; letter-spacing: -0.02em; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1; }
  .kpi-v.nodata { color: var(--fg-faint); }
  .kpi-u { font-size: 12px; color: var(--fg-subtle); font-weight: 500; }
  .kpi-k { font-size: var(--fs-xs); color: var(--fg-muted); text-transform: uppercase; letter-spacing: 0.05em; }

  /* ── Details toggle ── */
  .det-toggle { display: flex; align-items: center; gap: 12px; cursor: pointer; padding: 8px 16px; background: color-mix(in oklab, var(--surface) 55%, transparent); color: var(--fg-muted); transition: border-color var(--dur-fast) var(--ease-soft), color var(--dur-fast) var(--ease-soft), background var(--dur-fast) var(--ease-soft); }
  .det-toggle:hover { border-color: var(--border-strong); color: var(--fg); background: var(--surface); }
  .det-lbl { font-size: var(--fs-sm); font-weight: 650; color: var(--fg-2); }
  .det-sub { font-size: var(--fs-xs); color: var(--fg-subtle); font-family: var(--font-mono); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .det-caret { font-size: 12px; color: var(--fg-faint); transition: transform var(--dur-base) var(--ease-soft); }
  .det-caret.open { transform: rotate(180deg); }

  /* ── Bars ── */
  .bars { display: flex; flex-direction: column; gap: 8px; }
  .bar-row { display: grid; grid-template-columns: 84px 1fr 54px; align-items: center; gap: 11px; }
  .bar-k { font-size: var(--fs-xs); color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar-track { height: 9px; border-radius: 999px; background: var(--bg-inset); overflow: hidden; position: relative; }
  .bar-fill { position: absolute; inset: 0 auto 0 0; height: 100%; border-radius: 999px; transition: width var(--dur-slow) var(--ease-page);
    background: linear-gradient(90deg, oklch(0.62 0.15 var(--barH)), oklch(0.78 0.16 var(--barH))); }
  .bar-fill.zero { background: none; }
  .bar-fill.flow::after { content: ""; position: absolute; inset: 0; border-radius: 999px;
    background: linear-gradient(90deg, transparent, rgba(255,255,255,0.35), transparent); transform: translateX(-100%); animation: flow 1.5s linear infinite; }
  .bar-v { font-size: var(--fs-xs); color: var(--fg-2); text-align: right; }

  /* ── By model ── */
  .mdl-list { display: flex; flex-direction: column; gap: 9px; }
  .mdl-row { display: flex; align-items: center; gap: 9px; }
  .mdl-dot { width: 9px; height: 9px; border-radius: 50%; flex: none; background: oklch(0.74 0.16 var(--mH)); box-shadow: 0 0 8px oklch(0.74 0.16 var(--mH) / 0.6); }
  .mdl-name { font-size: var(--fs-xs); color: var(--fg); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mdl-stats { font-size: 10.5px; color: var(--fg-subtle); flex: none; }

  /* ── Turn timeline ── */
  .tl-track { position: relative; display: flex; align-items: flex-end; gap: 3px; height: 70px; padding-top: 10px; overflow-x: auto; overflow-y: hidden; border-bottom: 1px solid color-mix(in oklab, var(--border) 85%, transparent); }
  .tl-grid { position: absolute; inset: 10px 0 0 0; z-index: 0; pointer-events: none;
    background: repeating-linear-gradient(to top, transparent 0, transparent 32px, color-mix(in oklab, var(--border) 45%, transparent) 32px, color-mix(in oklab, var(--border) 45%, transparent) 33px); }
  .tl-bar { flex: 1 1 6px; min-width: 5px; max-width: 24px; border-radius: 3px 3px 2px 2px; position: relative; z-index: 1; align-self: flex-end;
    background: linear-gradient(180deg, oklch(0.82 0.13 var(--accent-h)), oklch(0.58 0.15 var(--accent-h)));
    transition: height var(--dur-slow) var(--ease-page); cursor: default; }
  .tl-bar:hover { filter: brightness(1.18); }
  .tl-btn { border: 0; padding: 0; cursor: pointer; }
  .tl-bar[data-end="error"] { background: linear-gradient(180deg, color-mix(in oklab, var(--danger) 88%, white), var(--danger)); }
  .tl-bar[data-end="user-stop"], .tl-bar[data-end="session-lost"] { background: var(--fg-faint); opacity: 0.55; }
  .tl-think { position: absolute; top: -7px; left: 50%; transform: translateX(-50%); width: 4px; height: 4px; border-radius: 50%;
    background: oklch(0.88 0.12 var(--accent-h)); box-shadow: 0 0 5px oklch(0.88 0.12 var(--accent-h)); }
  /* Silent pre-paint stall (>8s spawn/prefill/queue) — fixed danger hue, never themed. */
  .tl-dead { position: absolute; bottom: 0; left: 0; right: 0; height: 3px; border-radius: 0 0 2px 2px;
    background: var(--danger); box-shadow: 0 0 5px var(--danger); }
  .tl-axis { display: flex; justify-content: space-between; font-size: 10px; color: var(--fg-faint); margin-top: 6px; letter-spacing: 0.04em; }
  .tl-empty { padding: 30px 0; text-align: center; }

  /* ── 6-up stat grid (timing / reliability) ── */
  .stat6 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 9px 16px; }
  .full .stat6 { grid-template-columns: repeat(4, 1fr); }
  .mini-v { font-size: 18px; font-weight: 680; color: var(--fg); }
  .mini-v[data-warn="true"] { color: var(--warn); }
  .mini-k { font-size: 10.5px; color: var(--fg-muted); margin-top: 2px; }

  /* Reliability "all clear" — replaces the 6-zero grid when nothing is flagged. */
  .rel-clear { display: flex; align-items: center; gap: 9px; font-size: var(--fs-xs); color: var(--fg-muted); padding: 8px 0 4px; }
  .rel-clear-dot { width: 8px; height: 8px; border-radius: 50%; flex: none; background: var(--ok); box-shadow: 0 0 7px color-mix(in oklab, var(--ok) 70%, transparent); }

  /* ── Harness config ── */
  .cfg { display: grid; grid-template-columns: 1fr 1fr; gap: 2px 22px; }
  .cfg-row { display: flex; align-items: baseline; justify-content: space-between; gap: 10px; padding: 5px 0; border-bottom: 1px solid color-mix(in oklab, var(--border) 55%, transparent); }
  .cfg-k { font-size: var(--fs-xs); color: var(--fg-muted); }
  .cfg-v { font-size: var(--fs-xs); color: var(--fg-2); text-align: right; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* ── Grants ── */
  .grant-wrap { display: flex; flex-wrap: wrap; gap: 7px; }
  .grant { font-size: 11px; padding: 4px 10px; border-radius: 8px; background: var(--bg-inset); border: 1px solid var(--border); color: var(--fg-2); }
  .grant.write { border-color: color-mix(in oklab, var(--warn) 28%, var(--border)); color: var(--warn); background: var(--warn-soft); }
  .grant[data-off="true"] { opacity: 0.4; text-decoration: line-through; }

  /* ── Session log (all view) ── */
  .slog { display: flex; flex-direction: column; gap: 2px; }
  .slog-row { display: flex; align-items: center; gap: 4px; border-bottom: 1px solid color-mix(in oklab, var(--border) 50%, transparent); }
  .slog-row:last-child { border-bottom: 0; }
  .slog-open { flex: 1; min-width: 0; display: flex; align-items: center; gap: 12px; padding: 9px 6px; background: none; border: 0; cursor: pointer; color: var(--fg-2); text-align: left; border-radius: 8px; }
  .slog-open:hover { background: var(--surface-hover); }
  .slog-dot { width: 9px; height: 9px; border-radius: 50%; flex: none; background: oklch(0.74 0.16 var(--mH)); box-shadow: 0 0 7px oklch(0.74 0.16 var(--mH) / 0.55); }
  .slog-when { font-size: var(--fs-xs); color: var(--fg); flex: none; min-width: 116px; }
  .slog-livetag { font-size: 9px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--accent); background: var(--accent-soft); padding: 1px 7px; border-radius: 999px; flex: none; }
  .slog-model { font-size: 11px; color: var(--fg-muted); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .slog-stat { font-size: 11px; color: var(--fg-subtle); flex: none; min-width: 56px; text-align: right; }
  .slog-cost { font-size: var(--fs-xs); color: var(--fg); flex: none; min-width: 64px; text-align: right; font-weight: 600; }
  .slog-del { display: inline-grid; place-items: center; width: 30px; height: 30px; border-radius: 8px; border: 0; background: none; color: var(--fg-faint); cursor: pointer; flex: none; }
  .slog-del:hover { color: var(--danger); background: color-mix(in oklab, var(--danger) 12%, transparent); }

  /* ── Terminal / diagnostics ── */
  .term { padding-bottom: 0; }
  .term-led { width: 8px; height: 8px; border-radius: 50%; background: var(--fg-faint); display: inline-block; }
  .term-led.on { background: var(--ok); box-shadow: 0 0 8px var(--ok); animation: pulse 2s ease-in-out infinite; }
  .term-ctl { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; }
  .seg { display: inline-flex; background: var(--track); border: 1px solid var(--border); border-radius: 7px; padding: 2px; gap: 2px; }
  .seg-btn { height: 23px; padding: 0 10px; border: 0; border-radius: 5px; background: none; color: var(--fg-muted); font: inherit; font-size: 11px; font-weight: 600; cursor: pointer; }
  .seg-btn.on { background: var(--surface-hover); color: var(--fg); }
  .term-in { height: 26px; padding: 0 10px; border-radius: 7px; background: var(--field); border: 1px solid var(--field-border); color: var(--fg); font-size: 11px; width: 120px; }
  .term-in:focus { outline: 0; border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .ic-btn { display: inline-grid; place-items: center; width: 28px; height: 26px; border-radius: 7px; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); cursor: pointer; }
  .ic-btn:hover { color: var(--fg); border-color: var(--border-strong); }
  .term-body { max-height: 280px; overflow-y: auto; margin: 0 -17px; padding: 4px 17px 14px; border-top: 1px solid var(--border); font-family: var(--font-mono); }
  .tick { display: flex; align-items: baseline; gap: 10px; padding: 3px 0; font-size: 11.5px; line-height: 1.5; animation: tickin 320ms var(--ease-page); }
  .tick-t { color: var(--fg-faint); flex: none; }
  .tick-lvl { flex: none; width: 38px; text-transform: uppercase; font-size: 9.5px; font-weight: 700; }
  .tick-lvl[data-tone="danger"] { color: var(--danger); }
  .tick-lvl[data-tone="warn"] { color: var(--warn); }
  .tick-lvl[data-tone="info"] { color: var(--info); }
  .tick-lvl[data-tone="subtle"] { color: var(--fg-subtle); }
  .tick-stage { flex: none; color: var(--fg-subtle); }
  .tick-msg { flex: 1; min-width: 0; color: var(--fg-2); word-break: break-word; }
  .term-empty { display: flex; align-items: center; gap: 6px; padding: 14px 0; }
  .caret { width: 7px; height: 14px; background: var(--accent); display: inline-block; animation: blink 1.1s steps(2) infinite; border-radius: 1px; }

  /* ── Keyframes ── */
  @keyframes flow { to { transform: translateX(200%); } }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
  @keyframes blink { 0%, 50% { opacity: 1; } 50.01%, 100% { opacity: 0; } }
  @keyframes tickin { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: translateY(0); } }
  @keyframes cellrise { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }

  @media (max-width: 1080px) {
    .bento { grid-template-columns: repeat(2, 1fr); }
    .hero, .wide { grid-column: span 2; }
    .hero { grid-row: auto; }
    .stat6 { grid-template-columns: repeat(3, 1fr); }
    .cfg { grid-template-columns: 1fr; }
  }
  @media (prefers-reduced-motion: reduce) {
    .dhead-spark, .chip.livechip :global(svg), .term-led.on, .bar-fill.flow::after, .caret, .sesh-led.beat,
    .dash[data-live="true"] .dhead-spark, .bento > .cell, .bento > .det-toggle { animation: none; }
  }
</style>
