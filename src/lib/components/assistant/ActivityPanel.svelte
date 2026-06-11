<script lang="ts">
  // Merged side panel — one scrolling surface (no more Session/Activity tabs).
  // Top-to-bottom it answers "what's happening now" then "what it produced"
  // then "session review":
  //   Now strip   — live turn state + elapsed (while streaming)
  //   Last turn   — idle recap: duration / tools / files / cost + reply preview
  //   Plan        — TodoWrite plan + progress
  //   Steps       — settled tool-call log, turn-separated
  //   Outputs     — files written/edited this convo
  //   Sources     — URLs fetched/opened + queries searched
  // The redundant "This session" stat card was dropped — tok/s · tools · cost
  // already live in the status bar. Everything here is per-tab reactive state.
  import { onMount, onDestroy } from "svelte";
  import { fmtClock } from "./composer/helpers";
  import { fly, fade, slide } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { cubicOut } from "svelte/easing";
  import {
    Loader2, Terminal, Bot, Wrench, Activity, Sparkles,
    ChevronDown, ChevronUp, FileText, FilePen, Search, Globe,
    ListChecks, Circle, CheckCircle2, XCircle,
    StopCircle, Minimize2, Copy, ArrowDownToLine, Check, X, GitCompare, HelpCircle, Bell,
  } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import type { Block, ChatMessage } from "../../state/assistant.svelte";
  import { liveActivity, shellLabel } from "../../state/assistant/helpers";
  import { tooltip } from "$lib/actions/tooltip";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let { tabId = null }: { tabId?: string | null } = $props();

  const tab = $derived(tabId == null ? assistant.activeTab : assistant.tabFor(tabId));
  const messages = $derived<ChatMessage[]>(tab?.messages ?? []);
  const streaming = $derived(tab?.streaming ?? false);

  // Ticker — drives live elapsed readouts. Mounts only while the panel is open.
  // #166: adaptive cadence — 1s while streaming (the live elapsed counter needs
  // it), 10s when idle so the settled Steps list isn't re-rendered every second
  // just to age relative "Nm ago" labels that only change once a minute.
  let now = $state(Date.now());
  $effect(() => {
    const period = streaming ? 1000 : 10000;
    const t = setInterval(() => { now = Date.now(); }, period);
    return () => clearInterval(t);
  });

  // Mount-time stamp standing in for legacy blocks missing a startedAt — pinned
  // so `running` recomputes only when messages / agentSpawns change, not every
  // tick (see CR4 history).
  const mountTs = Date.now();

  // Motion — reuse the app's spring-out curve (≈ cubic-bezier(0.22,1,0.36,1)).
  // Every transition collapses to 0ms under prefers-reduced-motion.
  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const flipOpts = () => ({ duration: reducedMotion ? 0 : 240, easing: cubicOut });
  const rowIn = (i = 0) => (reducedMotion ? { duration: 0 } : { y: 6, opacity: 0, duration: 200, delay: Math.min(i, 4) * 35, easing: cubicOut });
  const rowOut = () => ({ duration: reducedMotion ? 0 : 240 });
  // Expand-reveal motion: the overflow rows mount in the SAME frame, so the
  // base `rowIn` stagger would hand all of them an identical flat delay → they
  // sit invisible, then pop in together (reads as lag). Indexing locally from 0
  // gives a quick top-down cascade with no leading dead-time; collapse fades fast.
  const rowInExtra = (i = 0) => (reducedMotion ? { duration: 0 } : { y: 5, opacity: 0, duration: 150, delay: Math.min(i, 6) * 22, easing: cubicOut });
  const rowOutExtra = () => ({ duration: reducedMotion ? 0 : 120 });

  // ── Now: canonical live turn state (tab.activity) ──────────────────────
  const turnStartedAt = $derived(tab?.activity.turnStartedAt ?? null);
  const liveItems = $derived(liveActivity(messages, tab?.agentSpawns ?? [], mountTs));
  // Thinking is a turn state (owned by the Now strip), not a discrete work
  // unit — the Running list shows only tool/shell/agent calls.
  const isThinking = $derived(liveItems.some((r) => r.kind === "thinking"));
  const running = $derived(liveItems.filter((r) => r.kind !== "thinking"));
  // Headline label: friendly caption matching the transcript rail.
  const nowLabel = $derived.by(() => {
    if (running.length === 1) return running[0].label;
    if (running.length > 1) return `Running ${running.length} actions`;
    if (isThinking) return "Thinking…";
    return "Responding…";
  });

  // ── Completion acknowledgment ──────────────────────────────────────────
  // liveActivity returns only PENDING units, so a finished tool would vanish
  // with no feedback. We retain a just-finished row for ~1.2s showing its
  // outcome (✓ + duration, or ✕ on error) before easing it out — so the panel
  // visibly reacts to each call LANDING instead of silently dropping it.
  // ── Steps log — full chronological record of every tool call this turn
  // (done + pending), newest-first, exactly like the mock "Steps" stream.
  // Each row: a category icon, the target (file / cmd / query), and a
  // "Verb · Xs ago" sub-line; write rows carry a +adds / −dels stat. The Now
  // strip above owns the single live headline, so this is the persistent log,
  // not a duplicate live readout.
  type StepCat = "read" | "write" | "shell" | "agent" | "search" | "web" | "ask" | "notify" | "meta";
  type StepRow = {
    id: string;
    cat: StepCat;
    verb: string;
    target: string;
    status: "pending" | "done" | "error";
    startedAt: number;
    durationMs: number | null;
    add: number | null;
    del: number | null;
    meta: string | null;
    turn: number;
  };

  function classifyTool(
    name: string,
    input: Record<string, unknown>,
  ): { cat: StepCat; verb: string; target: string; add: number | null; del: number | null } {
    const s = (k: string) => (typeof input[k] === "string" ? (input[k] as string) : "");
    switch (name) {
      case "Read":
        return { cat: "read", verb: "Read", target: basename(s("file_path")) || "file", add: null, del: null };
      case "Write":
        return { cat: "write", verb: "Write", target: basename(s("file_path")) || "file", add: lineCount(input.content), del: 0 };
      case "Edit":
      case "MultiEdit":
      case "NotebookEdit":
        return { cat: "write", verb: "Edit", target: basename(s("file_path") || s("notebook_path")) || "file", add: lineCount(input.new_string), del: lineCount(input.old_string) };
      case "Grep":
        return { cat: "search", verb: "Grep", target: s("pattern") || "search", add: null, del: null };
      case "Glob":
        return { cat: "search", verb: "Glob", target: s("pattern") || "glob", add: null, del: null };
      case "Bash":
        return { cat: "shell", verb: "Run", target: shellLabel(s("command")) || "shell", add: null, del: null };
      case "WebFetch":
        return { cat: "web", verb: "Fetch", target: hostnameOrSelf(s("url")), add: null, del: null };
      case "WebSearch":
        return { cat: "web", verb: "Search", target: s("query") || "search", add: null, del: null };
      default: {
        // MCP tools arrive as mcp__<server>__<tool> — humanize instead of
        // echoing the raw id twice (title + sub-line).
        const mcp = name.match(/^mcp__.+?__(.+)$/);
        if (mcp) {
          switch (mcp[1]) {
            case "read_file": return { cat: "read", verb: "Read", target: basename(s("path")) || "file", add: null, del: null };
            case "list_dir": return { cat: "read", verb: "Listed", target: s("path") && s("path") !== "." ? basename(s("path")) : "workspace", add: null, del: null };
            case "grep": return { cat: "search", verb: "Grep", target: s("pattern") || "search", add: null, del: null };
            case "ask_user": {
              // input = { questions: [{ question, header, options }] }
              const qs = Array.isArray(input.questions) ? (input.questions as Array<Record<string, unknown>>) : [];
              const q = typeof qs[0]?.question === "string" ? (qs[0].question as string) : "";
              return { cat: "ask", verb: "Asked", target: q || "a question", add: null, del: null };
            }
            case "notify": return { cat: "notify", verb: "Notified", target: s("title") || "notification", add: null, del: null };
            case "open_browser": return { cat: "web", verb: "Opened", target: stripProto(s("url")) || "browser", add: null, del: null };
            default: {
              if (mcp[1].startsWith("git_")) return { cat: "shell", verb: "Ran", target: `git ${mcp[1].slice(4)}`, add: null, del: null };
              const t = s("path") || s("pattern") || s("url") || s("query") || "";
              return { cat: "meta", verb: "Ran", target: t ? (basename(t) || t) : mcp[1].replace(/_/g, " "), add: null, del: null };
            }
          }
        }
        const t = s("file_path") || s("path") || s("pattern") || s("url") || s("query") || s("command") || "";
        return { cat: "meta", verb: name, target: t ? (basename(t) || t) : name, add: null, del: null };
      }
    }
  }

  // Tools surfaced elsewhere: agent launches ride agentSpawns; TodoWrite is the
  // Plan card. Keep them out of the Steps log so nothing double-lists.
  const STEP_SKIP = new Set(["Task", "Agent", "TodoWrite", "TaskCreate", "TaskUpdate"]);

  const steps = $derived.by<StepRow[]>(() => {
    const out: StepRow[] = [];
    let turn = 0;
    for (const m of messages) {
      if (m.role === "assistant") turn++;
      for (const b of m.blocks as Block[]) {
        if (b.type !== "tool" || STEP_SKIP.has(b.name)) continue;
        const c = classifyTool(b.name, (b.input ?? {}) as Record<string, unknown>);
        const status: StepRow["status"] =
          b.status === "done" ? "done" : (b.status === "error" || b.isError) ? "error" : "pending";
        out.push({ id: b.id, cat: c.cat, verb: c.verb, target: c.target, status, startedAt: b.startedAt ?? mountTs, durationMs: b.durationMs ?? null, add: c.add, del: c.del, meta: null, turn: Math.max(1, turn) });
      }
    }
    for (const a of tab?.agentSpawns ?? []) {
      // Spawns carry no message link — slot them into the latest turn already underway.
      let tn = 1;
      for (const r of out) if (r.startedAt <= a.startedAt) tn = Math.max(tn, r.turn);
      out.push({
        id: a.id, cat: "agent", verb: "Agent", target: a.description,
        status: a.completedAt != null ? (a.isError ? "error" : "done") : "pending",
        startedAt: a.startedAt, durationMs: a.completedAt != null ? a.completedAt - a.startedAt : null,
        add: null, del: null, meta: a.subagentType, turn: tn,
      });
    }
    return out.sort((x, y) => y.startedAt - x.startedAt);
  });

  let stepsExpanded = $state(false);
  const STEP_CAP = 4;
  // Steps is the calm, settled HISTORY — pending units live solely in the Now
  // cluster (headline + live rows), so a running tool isn't spinning in two
  // places at once. Once it lands it flies into this log.
  const settledSteps = $derived(steps.filter((s) => s.status !== "pending"));
  // Steps = actions (what Claude did); writes/edits are artifacts owned by
  // Outputs → Diff, so they're excluded here to kill the double-listing.
  const logSteps = $derived(settledSteps.filter((s) => s.cat !== "write"));
  const baseSteps = $derived(logSteps.slice(0, STEP_CAP));
  const extraSteps = $derived(logSteps.slice(STEP_CAP));
  // Separators render only when the log spans more than one turn.
  const turnCount = $derived(new Set(logSteps.map((s) => s.turn)).size);

  // ── Outputs — deduped artifacts (what the turn PRODUCED) ────────────────
  // Steps answers "what happened, in order"; Outputs answers "which files
  // changed", collapsing N edits to one file into a single net +/− row.
  // Derived off `steps` (write cat, settled) so it's preview-aware for free.
  type OutputRow = { id: string; target: string; add: number; del: number; edits: number; startedAt: number; status: StepRow["status"] };
  const outputs = $derived.by<OutputRow[]>(() => {
    const map = new Map<string, OutputRow>();
    for (const s of steps) { // steps is newest-first → first seen wins for id/status
      if (s.cat !== "write" || s.status === "pending") continue;
      const cur = map.get(s.target);
      if (!cur) map.set(s.target, { id: s.id, target: s.target, add: s.add ?? 0, del: s.del ?? 0, edits: 1, startedAt: s.startedAt, status: s.status });
      else { cur.add += s.add ?? 0; cur.del += s.del ?? 0; cur.edits += 1; if (s.status === "error") cur.status = "error"; }
    }
    return [...map.values()].sort((a, b) => b.startedAt - a.startedAt);
  });

  // ── Sources — URLs fetched/opened + web queries, deduped, newest-first ──
  type SourceRow = { id: string; kind: "url" | "query"; value: string; label: string; startedAt: number };
  const sources = $derived.by<SourceRow[]>(() => {
    const map = new Map<string, SourceRow>();
    for (const m of messages) {
      for (const b of m.blocks as Block[]) {
        if (b.type !== "tool") continue;
        const inp = (b.input ?? {}) as Record<string, unknown>;
        const url = typeof inp.url === "string" ? (inp.url as string) : null;
        const query = !url && typeof inp.query === "string" ? (inp.query as string) : null;
        const value = url ?? query;
        if (!value || map.has(value)) continue;
        map.set(value, {
          id: b.id, kind: url ? "url" : "query", value,
          label: url ? stripProto(url) : value,
          startedAt: b.startedAt ?? mountTs,
        });
      }
    }
    return [...map.values()].sort((a, b) => b.startedAt - a.startedAt);
  });
  let sourcesExpanded = $state(false);
  const SOURCE_CAP = 4;

  function agoShort(ts: number): string {
    const sec = Math.max(0, Math.round((now - ts) / 1000));
    if (sec < 60) return "just now";
    if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
    if (sec < 86400) return `${Math.floor(sec / 3600)}h ago`;
    return `${Math.floor(sec / 86400)}d ago`;
  }

  function agoLabel(r: StepRow): string {
    if (r.status === "pending") return r.cat === "write" ? "Writing…" : "Running…";
    const end = r.durationMs != null ? r.startedAt + r.durationMs : r.startedAt;
    const sec = Math.max(0, Math.round((now - end) / 1000));
    const t = sec < 1 ? "just now" : sec < 60 ? `${sec}s ago` : sec < 3600 ? `${Math.floor(sec / 60)}m ago` : `${Math.floor(sec / 3600)}h ago`;
    return `${r.verb} · ${t}`;
  }

  // ── Turn-end confirmation ──────────────────────────────────────────────
  // A clean completion currently just makes the Now strip vanish. Hold a brief
  // "Done · {dur}" success cap so a finished turn reads as closure, not a drop.
  let finishedAt = $state<number | null>(null);
  let finishedMs = $state<number | null>(null);
  let finishTimer: ReturnType<typeof setTimeout> | null = null;
  let wasStreaming = false;
  let lastTurnStart = 0;
  $effect(() => {
    if (streaming) {
      wasStreaming = true;
      if (turnStartedAt != null) lastTurnStart = turnStartedAt;
      return;
    }
    if (!wasStreaming) return;
    wasStreaming = false;
    // Only celebrate a real completion (had a start, no trailing error).
    if (lastTurnStart > 0 && !tab?.lastError) {
      finishedMs = Date.now() - lastTurnStart;
      finishedAt = Date.now();
      if (finishTimer) clearTimeout(finishTimer);
      finishTimer = setTimeout(() => { finishedAt = null; }, reducedMotion ? 0 : 2600);
    }
  });
  const showFinished = $derived(finishedAt != null && !streaming);
  onDestroy(() => { if (finishTimer) clearTimeout(finishTimer); });

  // ── Last-turn recap — the idle headline. Answers "what did that turn
  // accomplish" at a glance before the user digs into the Steps log.
  const lastTurn = $derived.by(() => {
    if (streaming) return null;
    const m = [...messages].reverse().find((x) => x.role === "assistant");
    if (!m) return null;
    let ms = 0, tools = 0;
    const files = new Set<string>();
    for (const b of m.blocks as Block[]) {
      if ((b.type === "thinking" || b.type === "tool") && b.durationMs != null) ms += b.durationMs;
      if (b.type !== "tool") continue;
      tools++;
      const name = b.name.replace(/^mcp__rift__/, "");
      const inp = (b.input ?? {}) as Record<string, unknown>;
      const fp = typeof inp.file_path === "string" ? inp.file_path
        : typeof inp.notebook_path === "string" ? (inp.notebook_path as string) : null;
      if (fp && (name === "Edit" || name === "MultiEdit" || name === "NotebookEdit" || name === "Write")) files.add(fp);
    }
    const cost = typeof m.costUsd === "number" ? m.costUsd : null;
    if (ms === 0 && tools === 0 && cost == null) return null;
    const reply = (m.blocks as Block[])
      .filter((b): b is Extract<Block, { type: "text" }> => b.type === "text")
      .map((b) => b.text.trim())
      .filter(Boolean)
      .join(" ")
      .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
      .replace(/[`*_#]/g, "")
      .replace(/\s+/g, " ")
      .trim();
    const preview = reply.length > 200 ? reply.slice(0, 200).trimEnd() + "…" : reply || null;
    return { ms, tools, files: files.size, cost, preview };
  });

  // ── Context meter — tokens / window, same source as the composer gauge ──
  const ctxTokens = $derived(assistant.ctxTokensFor(tab));
  const ctxWindow = $derived(assistant.ctxWindowFor(tab));
  const ctxPct = $derived(assistant.ctxPctFor(tab));
  const ctxTone = $derived(ctxPct >= 92 ? "crit" : ctxPct >= 75 ? "warn" : "ok");
  const ctxTitle = $derived(
    `Context: ${ctxTokens.toLocaleString()} / ${ctxWindow.toLocaleString()} tokens (${ctxPct.toFixed(1)}%) — fills as the conversation grows`,
  );

  // ── Tasks (TodoWrite plan) ─────────────────────────────────────────────
  const tasks = $derived(tab?.tasks ?? []);
  const taskCounts = $derived.by(() => {
    const total = tasks.length;
    const done = tasks.filter((t) => t.status === "completed").length;
    const active = tasks.filter((t) => t.status === "in_progress").length;
    return { total, done, active };
  });
  const taskPct = $derived(taskCounts.total > 0 ? (taskCounts.done / taskCounts.total) * 100 : 0);

  const isEmpty = $derived(
    !streaming && steps.length === 0 && tasks.length === 0,
  );

  const cost = $derived(tab?.totalCostUsd ?? null);

  // Scroll the transcript to a tool block + briefly flash it.
  function jumpTo(blockId: string) {
    const el = document.getElementById(`actnode-${blockId}`);
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    el.classList.add("act-flash");
    setTimeout(() => el.classList.remove("act-flash"), 1100);
  }

  function lineCount(s: unknown): number {
    if (typeof s !== "string" || s.length === 0) return 0;
    return s.split("\n").length;
  }
  function basename(p: string): string {
    const m = p.match(/[^\\/]+$/);
    return m ? m[0] : p;
  }
  function stripProto(u: string): string {
    return u.replace(/^https?:\/\//, "").replace(/\/$/, "");
  }
  function hostnameOrSelf(u: string): string {
    try { return new URL(u).hostname.replace(/^www\./, ""); }
    catch { return u; }
  }
  async function openSource(item: { kind: "url" | "query"; value: string }) {
    if (item.kind === "url") await openUrl(item.value);
    else if (item.kind === "query") await navigator.clipboard?.writeText(item.value);
  }

  // ── Quick actions ──────────────────────────────────────────────────────
  // The dock is permanent now, so the common turn controls live here at the
  // top instead of being keyboard-only / buried. Interrupt shows only while a
  // turn streams; the rest act on the whole conversation.
  let rootEl = $state<HTMLDivElement | undefined>();
  let copied = $state(false);
  const canCompact = $derived(!streaming && messages.length >= 4);

  function interrupt() { void assistant.stop(tabId); }
  function closeDock() { assistant.ui.dockOpen = false; }
  // Outputs is the index; the Session Diff overlay is the detail. Open it
  // scrolled to this file (matched by basename).
  function openDiff(target: string | null) {
    assistant.ui.diffTarget = target;
    assistant.ui.diffOpen = true;
  }
  function doCompact() { if (canCompact) void assistant.compactConversation(undefined, tabId); }
  function jumpLatest() {
    const scroll = rootEl?.closest(".pane-shell")?.querySelector<HTMLElement>(".scroll");
    if (scroll) scroll.scrollTo({ top: scroll.scrollHeight, behavior: "smooth" });
  }
  async function copyTranscript() {
    const lines: string[] = [];
    for (const m of messages) {
      if (m.role === "system") continue;
      const text = (m.blocks as Block[])
        .filter((b): b is Extract<Block, { type: "text" }> => b.type === "text")
        .map((b) => b.text.trim())
        .filter(Boolean)
        .join("\n\n");
      if (!text) continue;
      lines.push(`### ${m.role === "user" ? "User" : "Claude"}\n\n${text}`);
    }
    try {
      await navigator.clipboard?.writeText(lines.join("\n\n"));
      copied = true;
      setTimeout(() => { copied = false; }, 1400);
    } catch (e) { console.warn("[ActivityPanel] copy transcript failed", e); }
  }

  function fmtDur(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const s = ms / 1000;
    return s < 60 ? `${s.toFixed(1)}s` : `${Math.floor(s / 60)}m ${String(Math.floor(s % 60)).padStart(2, "0")}s`;
  }
</script>

<div class="activity" bind:this={rootEl}>
  <!-- Dock header — title + live/stop pill + close ───────────────────────── -->
  <header class="dock-head">
    <span class="dock-title">Activity</span>
    {#if streaming}
      <button type="button" class="dock-stop" onclick={interrupt} use:tooltip={"Stop generating"}>
        <i class="stop-dot"></i>
        <span class="stop-on">Working</span>
        <span class="stop-off"><StopCircle size={12} />Stop</span>
      </button>
    {/if}
    <button type="button" class="dock-x" onclick={closeDock} use:tooltip={"Close panel"}><X size={13} /></button>
  </header>

  <!-- Quick actions — idle only. While streaming, the merged Working→Stop in
       the header is the single turn control (mock: ct-qbar gated on !streaming). -->
  {#if !isEmpty && !streaming}
    <div class="qbar">
      <button type="button" class="qbtn" onclick={copyTranscript} use:tooltip={"Copy the transcript as Markdown"}>
        {#if copied}<Check size={14} class="qb-ok" /><span>Copied</span>
        {:else}<Copy size={14} /><span>Copy</span>{/if}
      </button>
      <button type="button" class="qbtn" onclick={doCompact} disabled={!canCompact}
        use:tooltip={canCompact ? "Compact — summarize history into a fresh, smaller context" : "Compact needs ≥4 messages and an idle turn"}>
        <Minimize2 size={14} /><span>Compact</span>
      </button>
      <button type="button" class="qbtn" onclick={jumpLatest} use:tooltip={"Jump to the latest message"}>
        <ArrowDownToLine size={14} /><span>Latest</span>
      </button>
    </div>
  {/if}

  <!-- Now strip — live turn headline + elapsed, capped by a brief Done state ── -->
  {#if streaming || showFinished}
    <div class="now"
      class:think={streaming && isThinking && running.length === 0}
      class:done={!streaming}
      transition:slide={{ duration: reducedMotion ? 0 : 180, easing: cubicOut }}>
      {#if !streaming}
        <CheckCircle2 size={14} class="now-done-ic" />
      {:else if isThinking && running.length === 0}
        <Sparkles size={14} class="mon-pulse" />
      {:else}
        <Loader2 size={14} class="mon-spin" />
      {/if}
      {#key streaming ? nowLabel : "Done"}
        <span class="now-label">{streaming ? nowLabel : "Done"}</span>
      {/key}
      {#if streaming && turnStartedAt != null}
        <span class="now-el mono">{fmtClock(now - turnStartedAt)}</span>
      {:else if !streaming && finishedMs != null}
        <span class="now-el mono">{fmtDur(finishedMs)}{#if settledSteps.length > 0} · {settledSteps.length} {settledSteps.length === 1 ? "step" : "steps"}{/if}</span>
      {/if}
    </div>
    <!-- Live rows — the running units live ONLY here (Steps stays settled), so
         "Running N actions" keeps its detail without double-spinning below.
         A single action is already the headline, so rows show only at 2+. -->
    {#if streaming && running.length > 1}
      <ul class="now-live" transition:slide={{ duration: reducedMotion ? 0 : 180, easing: cubicOut }}>
        {#each running as r, i (r.label)}
          <li in:fly={rowIn(i)} out:fade={rowOut()} animate:flip={flipOpts()}>
            <Loader2 size={12} class="mon-spin" />
            <span class="nl-label">{r.label}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  {#if ctxTokens > 0}
    <div class="ctx-meter" data-tone={ctxTone} use:tooltip={ctxTitle} role="img" aria-label={ctxTitle}>
      <div class="cm-row">
        <span class="cm-label">Context</span>
        <span class="cm-val mono">{(ctxTokens / 1000).toFixed(ctxTokens < 9950 ? 1 : 0)}K<span class="cm-dim"> / {Math.round(ctxWindow / 1000)}K</span></span>
      </div>
      <div class="cm-bar" data-tone={ctxTone}><i style="width: {Math.min(100, ctxPct)}%"></i></div>
      <div class="cm-foot mono">
        <span>{ctxPct < 1 ? "<1" : Math.round(ctxPct)}% used</span>
        {#if cost != null && cost > 0}<span class="cm-cost">${cost.toFixed(2)}</span>{/if}
      </div>
    </div>
  {/if}

  {#if isEmpty}
    <div class="empty-note">
      <Activity size={16} />
      <span class="en-title">Nothing here yet</span>
      <span>As Claude works, live progress, the plan, files touched, and web sources collect here.</span>
    </div>
  {:else}
    <!-- Last-turn recap — idle headline above the log sections ────────────── -->
    {#if lastTurn && !showFinished}
      <section class="sect recap" in:fade={{ duration: reducedMotion ? 0 : 160 }}>
        <header class="sect-head">
          <CheckCircle2 size={13} class="recap-ic" />
          <span class="sect-title">Last turn</span>
        </header>
        <div class="recap-grid">
          <div class="rc"><span class="rc-v mono">{fmtDur(lastTurn.ms)}</span><span class="rc-k">duration</span></div>
          <div class="rc"><span class="rc-v mono">{lastTurn.tools}</span><span class="rc-k">tool{lastTurn.tools === 1 ? "" : "s"}</span></div>
          {#if lastTurn.files > 0}
            <div class="rc"><span class="rc-v mono">{lastTurn.files}</span><span class="rc-k">file{lastTurn.files === 1 ? "" : "s"}</span></div>
          {/if}
          {#if lastTurn.cost != null}
            <div class="rc"><span class="rc-v mono">{lastTurn.cost > 0 && lastTurn.cost < 0.01 ? "<$0.01" : `$${lastTurn.cost.toFixed(2)}`}</span><span class="rc-k">cost</span></div>
          {/if}
        </div>
        {#if lastTurn.preview}
          <p class="recap-preview">{lastTurn.preview}</p>
        {/if}
      </section>
    {/if}

    <!-- Plan — TodoWrite objective + live progress (pinned above Steps) ────── -->
    {#if tasks.length > 0}
      <section class="sect plan-card" class:active={taskCounts.active > 0} class:done={taskCounts.active === 0 && taskCounts.done === taskCounts.total}>
        <header class="plan-head">
          <span class="plan-ic"><ListChecks size={13} /></span>
          <span class="plan-title">Plan</span>
          {#if taskCounts.active > 0}
            <span class="plan-live"><i></i>in progress</span>
          {:else if taskCounts.done === taskCounts.total}
            <span class="plan-live done"><Check size={11} />complete</span>
          {/if}
          <span class="plan-count mono">{taskCounts.done}<span class="counter-sep">/</span>{taskCounts.total}</span>
        </header>
        <div class="plan-bar" role="progressbar" aria-valuenow={taskCounts.done} aria-valuemax={taskCounts.total}>
          <div class="plan-fill" style="width: {taskPct}%"></div>
        </div>
        <ul class="plan-list">
          {#each tasks as t (t.id)}
            <li class="plan-task" data-status={t.status}>
              <span class="plan-tic">
                {#if t.status === "completed"}<CheckCircle2 size={14} />
                {:else if t.status === "in_progress"}<Loader2 size={14} class="mon-spin" />
                {:else}<Circle size={14} />{/if}
              </span>
              <span class="plan-tt">{t.content}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- Steps — full chronological log of every tool call (mock "Steps") ───── -->
    {#if logSteps.length > 0}
      <section class="sect">
        <header class="sect-head">
          <Activity size={12} />
          <span class="sect-title">Steps</span>
          <span class="badge mono">{logSteps.length}</span>
        </header>
        {#snippet stepBtn(r: StepRow)}
          <button
            type="button"
            class="ev"
            class:pending={r.status === "pending"}
            data-cat={r.cat}
            data-status={r.status}
            onclick={() => jumpTo(r.id)}
            use:tooltip={"Jump to this step in the transcript"}
          >
            <span class="ev-ico">
              {#if r.status === "pending"}<Loader2 size={14} class="mon-spin" />
              {:else if r.status === "error"}<XCircle size={14} />
              {:else if r.cat === "write"}<FilePen size={14} />
              {:else if r.cat === "read"}<FileText size={14} />
              {:else if r.cat === "shell"}<Terminal size={14} />
              {:else if r.cat === "agent"}<Bot size={14} />
              {:else if r.cat === "search"}<Search size={14} />
              {:else if r.cat === "web"}<Globe size={14} />
              {:else if r.cat === "ask"}<HelpCircle size={14} />
              {:else if r.cat === "notify"}<Bell size={14} />
              {:else}<Wrench size={14} />{/if}
            </span>
            <span class="ev-main">
              <span class="ev-target mono">{r.target}</span>
              <span class="ev-sub">{agoLabel(r)}</span>
            </span>
            <span class="ev-right">
              {#if r.cat === "write" && r.status !== "pending"}
                <span class="ev-stat mono"><span class="add">+{r.add ?? 0}</span>{#if r.del}<span class="del">−{r.del}</span>{/if}</span>
              {:else if r.meta}
                <span class="ev-meta mono">{r.meta}</span>
              {:else if r.status !== "pending" && r.durationMs != null}
                <span class="ev-meta mono">{fmtDur(r.durationMs)}</span>
              {/if}
            </span>
          </button>
        {/snippet}
        <ul class="rows">
          {#each baseSteps as r, i (r.id)}
            {@const prev = i > 0 ? baseSteps[i - 1] : null}
            <li in:fly={rowIn(i)} out:fade={rowOut()} animate:flip={flipOpts()}>
              {#if turnCount > 1 && (!prev || prev.turn !== r.turn)}
                <div class="turn-sep"><span>Turn {r.turn}</span><i></i><span class="ts-ago">{agoShort(r.startedAt)}</span></div>
              {/if}
              {@render stepBtn(r)}
            </li>
          {/each}
          {#if stepsExpanded}
            {#each extraSteps as r, i (r.id)}
              {@const prev = i > 0 ? extraSteps[i - 1] : (baseSteps.length > 0 ? baseSteps[baseSteps.length - 1] : null)}
              <li in:fly={rowInExtra(i)} out:fade={rowOutExtra()} animate:flip={flipOpts()}>
                {#if turnCount > 1 && (!prev || prev.turn !== r.turn)}
                  <div class="turn-sep"><span>Turn {r.turn}</span><i></i><span class="ts-ago">{agoShort(r.startedAt)}</span></div>
                {/if}
                {@render stepBtn(r)}
              </li>
            {/each}
          {/if}
        </ul>
        {#if logSteps.length > STEP_CAP}
          <button type="button" class="rows-more" onclick={() => (stepsExpanded = !stepsExpanded)}>
            {#if stepsExpanded}<ChevronUp size={13} />Show less{:else}<ChevronDown size={13} />Show {logSteps.length - STEP_CAP} more{/if}
          </button>
        {/if}
      </section>
    {/if}

    <!-- Outputs — deduped files the turn touched (artifacts, not chronology). -->
    {#if outputs.length > 0}
      <section class="sect">
        <header class="sect-head">
          <FilePen size={12} />
          <span class="sect-title">Outputs</span>
          <span class="badge mono">{outputs.length}</span>
          <button type="button" class="sect-link" onclick={() => openDiff(null)} use:tooltip={"View every change this session (Ctrl+Shift+D)"}>
            <GitCompare size={12} />Diff
          </button>
        </header>
        <ul class="rows">
          {#each outputs as o, i (o.target)}
            <li in:fly={rowIn(i)} out:fade={rowOut()} animate:flip={flipOpts()}>
              <button
                type="button"
                class="ev out"
                data-status={o.status}
                onclick={() => openDiff(o.target)}
                use:tooltip={"View the changes to this file"}
              >
                <span class="ev-ico">
                  {#if o.status === "error"}<XCircle size={14} />{:else}<FilePen size={14} />{/if}
                </span>
                <span class="ev-main">
                  <span class="ev-target path mono">{o.target}</span>
                  <span class="ev-sub">{o.edits === 1 ? "1 edit" : `${o.edits} edits`}</span>
                </span>
                <span class="ev-right">
                  <span class="ev-stat mono"><span class="add">+{o.add}</span>{#if o.del}<span class="del">−{o.del}</span>{/if}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- Sources — URLs fetched/opened + web queries (deduped, click to open/copy). -->
    {#if sources.length > 0}
      <section class="sect">
        <header class="sect-head">
          <Globe size={12} />
          <span class="sect-title">Sources</span>
          <span class="badge mono">{sources.length}</span>
        </header>
        <ul class="rows">
          {#each (sourcesExpanded ? sources : sources.slice(0, SOURCE_CAP)) as src, i (src.value)}
            <li in:fly={rowIn(i)} out:fade={rowOut()} animate:flip={flipOpts()}>
              <button
                type="button"
                class="ev"
                onclick={() => openSource(src)}
                use:tooltip={src.kind === "url" ? "Open in your browser" : "Copy this search query"}
              >
                <span class="ev-ico">
                  {#if src.kind === "url"}<Globe size={14} />{:else}<Search size={14} />{/if}
                </span>
                <span class="ev-main">
                  <span class="ev-target mono">{src.label}</span>
                  <span class="ev-sub">{src.kind === "url" ? `Link · ${agoShort(src.startedAt)}` : `Search · ${agoShort(src.startedAt)}`}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
        {#if sources.length > SOURCE_CAP}
          <button type="button" class="rows-more" onclick={() => (sourcesExpanded = !sourcesExpanded)}>
            {#if sourcesExpanded}<ChevronUp size={13} />Show less{:else}<ChevronDown size={13} />Show {sources.length - SOURCE_CAP} more{/if}
          </button>
        {/if}
      </section>
    {/if}

  {/if}
</div>

<style>
  .activity {
    width: 100%; flex: 1;
    display: flex; flex-direction: column;
    min-height: 0; overflow-x: hidden; overflow-y: auto;
    box-sizing: border-box;
  }
  .activity::-webkit-scrollbar { width: 8px; height: 0; }
  .activity::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 4px; }
  .activity::-webkit-scrollbar-thumb:hover { background: var(--fg-faint); }

  /* Dock header — pinned 44px bar: title + live/stop pill + close. */
  .dock-head {
    display: flex; align-items: center; gap: 10px;
    height: 44px; padding: 0 12px; flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    position: sticky; top: 0; z-index: 5;
    background: var(--bg);
  }
  .dock-title { flex: 1; font-size: var(--fs-sm); font-weight: 650; color: var(--fg); }
  /* Working pill — live pulse at rest, morphs to a danger Stop on hover. */
  .dock-stop {
    display: inline-flex; align-items: center; gap: 6px;
    height: 25px; padding: 0 10px; border-radius: 999px; cursor: pointer;
    font: inherit; font-size: 11px; font-weight: 600; color: var(--accent);
    border: 1px solid var(--ghost-border); background: var(--accent-soft);
    transition: background 140ms ease, border-color 140ms ease, color 140ms ease;
  }
  .dock-stop .stop-dot {
    width: 6px; height: 6px; border-radius: 50%; background: var(--accent);
    box-shadow: 0 0 7px color-mix(in oklab, var(--accent) 55%, transparent);
    animation: mon-live-pulse 1.3s ease-in-out infinite;
  }
  .dock-stop .stop-off { display: none; align-items: center; gap: 5px; }
  .dock-stop:hover { color: var(--danger); border-color: color-mix(in oklab, var(--danger) 45%, var(--border)); background: color-mix(in oklab, var(--danger) 12%, transparent); }
  .dock-stop:hover .stop-dot, .dock-stop:hover .stop-on { display: none; }
  .dock-stop:hover .stop-off { display: inline-flex; }
  .dock-stop:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .dock-x {
    width: 26px; height: 26px; display: grid; place-items: center;
    border: 0; background: transparent; color: var(--fg-faint);
    border-radius: 6px; cursor: pointer; flex-shrink: 0;
    transition: background 120ms ease, color 120ms ease;
  }
  .dock-x:hover { background: var(--surface-hover); color: var(--fg); }
  .dock-x:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  /* Quick-actions — three separate bordered pills (mock ct-qbar / ct-qbtn). */
  .qbar {
    display: flex; gap: 6px; padding: 10px 12px; flex-shrink: 0;
    border-bottom: 1px solid var(--border);
  }
  .qbtn {
    flex: 1; min-width: 0;
    display: inline-flex; align-items: center; justify-content: center; gap: 6px;
    height: 32px; padding: 0 8px;
    border: 1px solid var(--border); border-radius: 8px;
    background: color-mix(in oklch, var(--bg-elev-1) 50%, transparent);
    color: var(--fg-2); font: inherit; font-size: 11px; font-weight: 600;
    white-space: nowrap; cursor: pointer;
    transition: background 130ms ease, color 130ms ease, border-color 130ms ease;
  }
  .qbtn :global(svg) { flex-shrink: 0; color: var(--fg-subtle); transition: color 130ms ease; }
  .qbtn:hover:not(:disabled) { background: var(--bg-elev-2); color: var(--fg); border-color: var(--border-strong); }
  .qbtn:hover:not(:disabled) :global(svg) { color: var(--accent); }
  .qbtn:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .qbtn:disabled { opacity: 0.4; cursor: default; }
  .qbar :global(.qb-ok) { color: var(--ok) !important; }

  /* Now strip */
  .now {
    display: flex; align-items: center; gap: 9px;
    padding: 11px 14px; flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    background: var(--accent-soft);
  }
  .now :global(svg) { color: var(--accent); flex-shrink: 0; }
  .now-label {
    flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--fs-sm); font-weight: 600; color: var(--fg);
    /* Crossfade when the headline changes (Thinking → Running 2 actions → Done),
       keyed via {#key} in the template — mirrors the transcript stage-label. */
    animation: now-label-in 280ms ease-out;
  }
  @keyframes now-label-in {
    from { opacity: 0; transform: translateY(-1px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .now-el { flex-shrink: 0; font-size: 11px; color: var(--accent); font-variant-numeric: tabular-nums; }
  /* Turn-end confirmation — a calm green cap, not another alert. */
  .now.done { background: color-mix(in oklch, var(--ok) 11%, transparent); }
  .now.done :global(svg) { color: var(--ok); }
  .now.done .now-label, .now.done .now-el { color: var(--ok); }

  /* Live rows — the running units, sole home for in-flight detail. Sits under
     the Now headline in the same accent wash; Steps below stays settled-only. */
  .now-live {
    list-style: none; margin: 0; padding: 2px 14px 11px;
    display: flex; flex-direction: column; gap: 5px;
    background: var(--accent-soft);
    border-bottom: 1px solid var(--border);
  }
  .now-live li {
    display: flex; align-items: center; gap: 8px;
    font-size: 11.5px; color: var(--fg-2);
  }
  .now-live :global(svg) { color: var(--accent); flex-shrink: 0; }
  .nl-label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .sect { display: flex; flex-direction: column; border-bottom: 1px solid var(--border); }
  .sect:last-of-type { border-bottom: none; }
  .sect-head {
    display: flex; align-items: center; gap: 8px;
    padding: 11px 14px 8px;
    color: var(--fg); font-size: var(--fs-sm); font-weight: 600; flex-shrink: 0;
  }
  .sect-head :global(svg) { color: var(--accent); flex-shrink: 0; }
  .sect-title { color: var(--fg); }
  .badge {
    margin-left: auto; font-size: 10px; padding: 2px 7px;
    background: var(--accent-soft); color: var(--accent);
    border-radius: 999px; font-variant-numeric: tabular-nums; font-weight: 600;
  }
  .counter-sep { opacity: 0.55; margin: 0 1px; }
  /* Section action link (Outputs → open full diff). Quiet until hover. */
  .sect-link {
    margin-left: 8px;
    display: inline-flex; align-items: center; gap: 4px;
    padding: 3px 8px; border-radius: 6px;
    border: 1px solid var(--border); background: transparent;
    color: var(--fg-muted); font: inherit; font-size: 10.5px; font-weight: 600;
    cursor: pointer; flex-shrink: 0;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .sect-link :global(svg) { color: var(--fg-subtle); transition: color 120ms ease; }
  .sect-link:hover { background: var(--accent-soft); color: var(--accent); border-color: color-mix(in oklab, var(--accent) 32%, var(--border)); }
  .sect-link:hover :global(svg) { color: var(--accent); }
  .sect-link:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  /* Running rows. A faint spine threads the category icons into one timeline;
     the icons' opaque fill punches through it, so each reads as a node. */
  .rows { position: relative; list-style: none; margin: 0; padding: 4px 8px 12px; display: flex; flex-direction: column; gap: 2px; }
  .rows::before {
    content: ""; position: absolute;
    left: 31px; /* rows pad-left 8 + ev pad-left 10 + icon half 13 */
    top: 17px; bottom: 24px; width: 1.5px;
    background: var(--border); border-radius: 2px;
    z-index: 0; pointer-events: none;
  }
  .rows-more {
    display: flex; align-items: center; justify-content: center; gap: 6px;
    width: calc(100% - 16px); margin: 0 8px 10px; height: 30px;
    border: 1px solid var(--border); border-radius: 8px; background: transparent;
    color: var(--fg-muted); font: inherit; font-size: 11px; font-weight: 600;
    cursor: pointer; transition: background 130ms ease, color 130ms ease, border-color 130ms ease;
  }
  .rows-more:hover { background: var(--surface-hover); color: var(--fg); border-color: var(--border-strong); }
  .rows-more :global(svg) { color: var(--fg-subtle); transition: color 130ms ease; }
  .rows-more:hover :global(svg) { color: var(--accent); }
  /* Step rows — mock ct-ev: 26px boxed category icon + two-line main
     (mono target / verb·ago sub) + right-side diff-stat or meta. */
  .ev {
    display: flex; align-items: center; gap: 10px;
    width: 100%; padding: 9px 10px; border-radius: 9px;
    background: none; border: 0; text-align: left; font: inherit; cursor: pointer;
    color: var(--fg-2);
    transition: background 120ms ease;
  }
  .ev:hover { background: var(--surface-hover); }
  .ev:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .ev-ico {
    position: relative; z-index: 1; /* above the timeline spine → reads as a node */
    width: 26px; height: 26px; border-radius: 7px;
    display: grid; place-items: center; flex-shrink: 0;
    background: var(--bg-elev-2); color: var(--fg-subtle);
    transition: background 160ms ease, color 160ms ease;
  }
  /* Opaque tints — translucent -soft fills let the timeline spine bleed through. */
  .ev[data-cat="write"] .ev-ico { background: color-mix(in oklab, var(--accent) 14%, var(--bg-elev-2)); color: var(--accent); }
  .ev[data-cat="ask"] .ev-ico { background: color-mix(in oklab, var(--accent) 14%, var(--bg-elev-2)); color: var(--accent); }
  .ev.pending .ev-ico { background: color-mix(in oklab, var(--accent) 14%, var(--bg-elev-2)); color: var(--accent); }
  .ev[data-status="error"] .ev-ico { background: color-mix(in oklab, var(--danger) 14%, var(--bg-elev-2)); color: var(--danger); }
  .ev-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .ev-target { font-size: 11.5px; color: var(--fg); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* File rows clip from the START so the extension survives: …vity-panel.svelte
     beats activity-pane… . rtl anchors the ellipsis left; latin keeps its order. */
  .ev-target.path,
  .ev[data-cat="read"] .ev-target,
  .ev[data-cat="write"] .ev-target { direction: rtl; text-align: left; }
  .ev-sub { font-size: 10px; color: var(--fg-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ev.pending .ev-sub { color: var(--accent); }
  .ev-right { flex-shrink: 0; display: inline-flex; align-items: center; }
  .ev-meta { font-size: 10px; color: var(--fg-subtle); }
  .ev-stat { display: inline-flex; gap: 6px; font-size: 11px; }
  .ev-stat .add { color: var(--ok); }
  .ev-stat .del { color: var(--danger); }

  /* Plan card — objective + live progress, lifted out of the plain section
     grammar with a soft gradient wash + bolder bar. */
  .plan-card {
    padding: 12px 14px 13px;
    background: linear-gradient(180deg, color-mix(in oklch, var(--accent-soft) 35%, transparent), transparent 90%);
  }
  .plan-card.active {
    background: linear-gradient(180deg, color-mix(in oklch, var(--accent-soft) 60%, transparent), transparent 92%);
  }
  .plan-head { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
  .plan-ic { display: inline-flex; color: var(--accent); flex-shrink: 0; }
  .plan-title { font-size: 12px; font-weight: 650; letter-spacing: -0.01em; color: var(--fg); }
  .plan-live { display: inline-flex; align-items: center; gap: 5px; font-size: 10px; font-weight: 600; color: var(--accent); }
  .plan-live i { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 6px var(--ring); animation: mon-live-pulse 1.3s ease-in-out infinite; }
  .plan-live.done { color: var(--ok); }
  .plan-live.done :global(svg) { color: var(--ok); }
  .plan-count { margin-left: auto; font-size: 11.5px; font-weight: 600; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .plan-bar { height: 5px; border-radius: 999px; background: var(--bg-elev-3); overflow: hidden; margin-bottom: 9px; }
  .plan-fill {
    height: 100%; border-radius: 999px;
    background: linear-gradient(90deg, var(--accent), var(--accent-hover));
    box-shadow: 0 0 10px var(--ring);
    transition: width 460ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .plan-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  .plan-task {
    position: relative; display: flex; align-items: flex-start; gap: 9px;
    padding: 6px 9px; border-radius: 8px;
    font-size: 12px; line-height: 1.4; color: var(--fg-2);
    transition: background 180ms ease;
  }
  .plan-tic { display: inline-flex; align-items: center; margin-top: 0.5px; flex-shrink: 0; color: var(--fg-faint); }
  .plan-tt { flex: 1; min-width: 0; word-break: break-word; }
  .plan-task[data-status="completed"] .plan-tic { color: var(--accent); }
  .plan-task[data-status="completed"] .plan-tt { color: var(--fg-subtle); }
  .plan-task[data-status="pending"] .plan-tt { color: var(--fg-muted); }
  /* in-progress — the spotlight: ghost fill + emerald left-bar + bold label */
  .plan-task[data-status="in_progress"] { background: var(--accent-soft); box-shadow: inset 2px 0 0 var(--accent); }
  .plan-task[data-status="in_progress"] .plan-tic { color: var(--accent); }
  .plan-task[data-status="in_progress"] .plan-tt { color: var(--fg); font-weight: 550; }

  /* Context meter — dock-level twin of the composer gauge. */
  .ctx-meter { padding: 12px 14px 13px; border-bottom: 1px solid var(--border); }
  .cm-row { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 7px; }
  .cm-label { font-size: 10px; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); }
  .cm-val { font-size: var(--fs-xs); color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .cm-dim { color: var(--fg-faint); }
  .cm-bar { height: 6px; border-radius: 999px; background: var(--bg-elev-3); overflow: hidden; }
  .cm-bar i { display: block; height: 100%; border-radius: 999px; background: var(--accent); transition: width 400ms var(--ease-page), background 200ms var(--ease-soft); }
  .cm-bar[data-tone="warn"] i { background: var(--warn); }
  .cm-bar[data-tone="crit"] i { background: var(--danger); }
  .cm-foot { display: flex; justify-content: space-between; margin-top: 6px; font-size: 10.5px; color: var(--fg-subtle); }
  .cm-cost { color: var(--fg-muted); }
  .ctx-meter[data-tone="warn"] .cm-foot { color: var(--warn); }
  .ctx-meter[data-tone="crit"] .cm-foot { color: var(--danger); }

  :global(.tl-node.act-flash) {
    animation: act-flash 1.1s cubic-bezier(0.22, 1, 0.36, 1) both;
    border-radius: 8px;
  }
  @keyframes act-flash {
    0%   { box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 70%, transparent); background: color-mix(in oklab, var(--accent) 14%, transparent); }
    100% { box-shadow: 0 0 0 2px transparent; background: transparent; }
  }

  /* Empty state — centered icon + caption (mock ct-dock-empty). */
  .empty-note {
    display: flex; flex-direction: column; align-items: center; gap: 9px;
    text-align: center; padding: 36px 24px;
    color: var(--fg-faint); font-size: 11px; line-height: 1.5;
  }
  .empty-note :global(svg) { color: var(--fg-subtle); opacity: 0.6; }
  .empty-note .en-title { font-size: 12px; font-weight: 600; color: var(--fg-2); }

  /* Turn separators — only when the Steps log spans multiple turns. */
  .turn-sep {
    position: relative; z-index: 1;
    display: flex; align-items: center; gap: 8px;
    padding: 9px 4px 5px;
    font-size: 9.5px; font-weight: 650; letter-spacing: 0.07em; text-transform: uppercase;
    color: var(--fg-faint);
  }
  .turn-sep i { flex: 1; height: 1px; background: var(--border); }
  .turn-sep .ts-ago { text-transform: none; letter-spacing: 0; font-weight: 500; }

  .mono { font-family: var(--font-mono, monospace); }
  .activity :global(.mon-spin) { animation: mon-spin 0.9s linear infinite; }
  .activity :global(.mon-pulse) { animation: mon-live-pulse 1.4s ease-in-out infinite; }
  @keyframes mon-spin { to { transform: rotate(360deg); } }
  @keyframes mon-live-pulse { 0%,100% { opacity: 0.4; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) {
    .activity :global(.mon-spin), .activity :global(.mon-pulse) { animation: none; }
    .now-label { animation: none; }
    :global(.tl-node.act-flash) { animation: none; }
  }

  /* Last-turn recap — compact stat grid capping the idle panel. */
  .recap :global(.recap-ic) { color: var(--ok, var(--accent)); }
  .recap-grid { display: flex; gap: 18px; padding: 2px 14px 8px; }
  .recap-preview {
    margin: 0 14px 12px; padding: 1px 0 1px 10px;
    border-left: 2px solid color-mix(in oklab, var(--accent) 35%, var(--border));
    font-size: 11px; line-height: 1.45; color: var(--fg-muted);
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
  }
  .rc { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .rc-v { font-size: 14px; font-weight: 700; letter-spacing: -0.01em; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1.15; }
  .rc-k { font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.07em; color: var(--fg-faint); }
</style>
