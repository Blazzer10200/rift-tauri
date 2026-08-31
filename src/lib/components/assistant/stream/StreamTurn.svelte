<script lang="ts">
  import "$lib/styles/stream.css";
  import { Check, Copy, RotateCcw, AlertTriangle, Zap, CornerDownRight, ChevronRight } from "@lucide/svelte";
  import { slide } from "svelte/transition";
  import Markdown from "../Markdown.svelte";
  import StreamThinking from "./StreamThinking.svelte";
  import WorkLine from "./WorkLine.svelte";
  import WriteBatch from "./WriteBatch.svelte";
  import StreamPlan from "./StreamPlan.svelte";
  import StreamWeb from "./StreamWeb.svelte";
  import StreamResult from "./StreamResult.svelte";
  import StreamAgent from "./StreamAgent.svelte";
  import StreamAskUser from "./StreamAskUser.svelte";
  import StreamShell from "./StreamShell.svelte";
  import StreamExitPlan from "./StreamExitPlan.svelte";
  import PermissionBar from "../PermissionBar.svelte";
  import { messageToTurn, groupBlocks, answerStartIndex, fmtDur, classifySay, VERB_ING, VERB_PAST, tasksToPlanItems, type Group, type StreamTool } from "./streamModel";
  import { assistant, type ChatMessage, type TabState } from "$lib/state/assistant.svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { fmtTokens, isOpenAIModel } from "$lib/state/assistant/helpers";
  import { modelProviderLabel } from "$lib/state/assistant/providerDisplay";
  import AnimatedCount from "./AnimatedCount.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  // `tab` is THIS pane's tab. Live-turn status (activity timer, live tokens,
  // plan tasks) must read from it, not the global `assistant.*` getters — those
  // delegate to the single focused activeTab, so in split-pane every pane would
  // otherwise mirror the focused pane's "Working… 109s". Falls back to the
  // active tab when omitted (single-pane callers).
  let { message, streaming = false, isLast = false, tab = null, tabId = null, workspaceRoot = null }:
    { message: ChatMessage; streaming?: boolean; isLast?: boolean; tab?: TabState | null; tabId?: string | null; workspaceRoot?: string | null } = $props();
  const liveTab = $derived(tab ?? assistant.activeTab);
  const turnModel = $derived(message.model ?? liveTab?.lastModelId ?? liveTab?.modelOverride ?? null);
  const turnProvider = $derived(turnModel ? modelProviderLabel(turnModel) : null);
  const chatGptTurn = $derived(isOpenAIModel(turnModel));
  const providerSubject = $derived(turnProvider ?? "The assistant");
  const providerInline = $derived(turnProvider ?? "the assistant");
  const providerPossessive = $derived(turnProvider ? `${turnProvider}'s` : "the assistant's");

  const turn = $derived(messageToTurn(message));
  const groups = $derived(groupBlocks(turn.blocks));
  // Completed turns separate the evidence from the answer: setup narration,
  // thinking, tools, and mid-turn steers fold into one activity receipt while
  // trailing prose remains visible. Live turns stay chronological so nothing
  // moves while the user is watching it arrive.
  const answerAt = $derived(answerStartIndex(groups));
  const activityGroups = $derived(groups.slice(0, answerAt));
  const answerGroups = $derived(groups.slice(answerAt));
  const actionCount = $derived(turn.blocks.filter((b) => b.type === "tool").length);
  const hasActivity = $derived(!!turn.thinking || activityGroups.length > 0);
  let activityOpen = $state(false);
  const activityShellCount = $derived(activityGroups.reduce((count, group) => {
    if (group.type !== "work") return count;
    return count + group.segs.reduce((n, seg) =>
      n + (seg.seg === "rich" && seg.tool.kind === "shell" ? 1 : 0), 0);
  }, 0));
  let outputBulkOpen = $state(false);
  let outputBulkVersion = $state(0);
  function toggleActivityOutput() {
    outputBulkOpen = !outputBulkOpen;
    outputBulkVersion += 1;
  }
  const activityTitle = $derived(
    turn.bgAgents > 0
      ? "Agent working in background"
      : turn.outcome === "applied"
        ? "Changes applied"
        : turn.outcome === "failed"
          ? "Changes failed"
          : turn.outcome === "planned"
            ? "Plan prepared"
            : actionCount > 0
              ? "Work completed"
              : "Reasoning",
  );
  const completionTokens = $derived(
    typeof message.outputTokens === "number" && message.outputTokens > 0 ? message.outputTokens : null,
  );
  const completionCost = $derived(
    typeof message.costUsd === "number" && message.costUsd > 0 ? `$${message.costUsd.toFixed(2)}` : null,
  );
  // Light timeline rail — a thin connecting line down the turn body so a
  // multi-step turn reads as one sequence (history's spine, minus the bullet
  // chrome; stream stays boxless). Pure-text answers skip it — indenting prose
  // under a rail would claim structure that isn't there.
  const railed = $derived(groups.some((g) => g.type === "work") || !!turn.thinking);

  // Newer CLI emits one TaskCreate per item, so a plan tool block carries no
  // `todos[]` — its own `items` is empty. The store aggregates the whole plan in
  // `assistant.tasks` (TaskCreate/TaskUpdate/TodoWrite/checklist-pin all feed it),
  // so fall back to that for the plan card. Gate the live aggregate to the LAST
  // turn: with one plan per conversation, only the current turn's card should
  // mirror the live state — older plan blocks resolve to empty and are skipped
  // (the {#if planItemsFor(...).length} guard at the render site), so a plan-turn
  // followed by an update-turn shows ONE card, not a duplicate per turn.
  const livePlanItems = $derived(isLast ? tasksToPlanItems(liveTab?.tasks ?? []) : []);
  const planItemsFor = (tool: StreamTool) => (tool.items?.length ? tool.items : livePlanItems);

  // In a prompting permission mode (default / acceptEdits / plan) the CLI raises a
  // `can_use_tool` ask for a gated tool mid-turn. The Allow/Deny bar (PermissionBar)
  // historically rendered ONLY in MessageBubble (persisted history) — never in this
  // LIVE timeline — so a default-mode user could never approve: no bar appeared, the
  // ask timed out at 120s, and the turn died "Changes failed". This surfaces the bar
  // inline on the live tool row(s) holding a pending prompt, keyed by tool_use_id.
  // Reads liveTab.permissionPrompts (reactive Map) so it appears the instant the ask
  // lands. `pendingPerms` returns the tools in a segment that currently have an ask.
  const pendingPerms = (tools: StreamTool[]): StreamTool[] =>
    liveTab && liveTab.permissionPrompts.size > 0
      ? tools.filter((t) => liveTab.permissionPrompts.has(t.id))
      : [];

  const plainText = $derived(
    message.blocks.map((b) => (b.type === "text" ? b.text : "")).join("").trim(),
  );

  // Narration a user WATCHED stream must never be yanked away. Without this
  // latch, a filler/connective line rendered "full" while live got reclassified
  // the instant the next tool started (or the turn ended) — text collapsed to a
  // muted beat or vanished mid-read, the subtle "streaming feels glitchy" jank.
  // Latch every say index that ever rendered full while live; demotion then only
  // applies to blocks that were never live-shown (history loads, older turns).
  // Session-scoped by design — a reopened convo demotes normally.
  const liveShown = new Set<number>();
  $effect(() => {
    if (!streaming) return;
    const gi = groups.length - 1;
    if (groups[gi]?.type === "say") liveShown.add(gi);
  });

  // Decide how a `say` block renders given the narration-density pref. Returns
  // "full" (normal prose block), "muted" (demoted inline connective beat), or
  // "hide". The live-streaming LAST block is always shown full — never demote the
  // text that's actively typing. `prose` is always full regardless of pref.
  function sayMode(text: string, isLiveLast: boolean, gi: number): "full" | "muted" | "hide" {
    if (isLiveLast || liveShown.has(gi)) return "full";
    const w = classifySay(text);
    if (w === "prose") return "full";
    const pref = uiPrefs.narration;
    if (pref === "chatty") return "full";
    if (pref === "focused") return "hide";
    // balanced: hide pure filler, demote connective beats to a muted line.
    return w === "filler" ? "hide" : "muted";
  }

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  // Clear a pending copy-reset timer on unmount so it can't fire into a
  // destroyed component.
  $effect(() => () => { if (copyTimer) clearTimeout(copyTimer); });
  async function copy() {
    if (!plainText) return;
    try {
      await navigator.clipboard.writeText(plainText);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = false; copyTimer = null; }, 1200);
    } catch (e) {
      console.warn("copy failed", e);
    }
  }

  // live status: the last pending tool drives the footer verb
  const liveTool = $derived.by((): StreamTool | null => {
    for (const g of groups) {
      if (g.type !== "work") continue;
      for (const seg of g.segs) {
        const tools = seg.seg === "rich" ? [seg.tool] : seg.tools;
        for (const t of tools) if (t.status === "pending") return t;
      }
    }
    return null;
  });
  // Awaiting human input: a pending ask_user (or permission) tool parks the
  // turn on YOU, not the model. The elapsed timer + shimmer would otherwise
  // read as "the bot is grinding" when it's actually idle waiting for a click —
  // the #1 "why is this so slow" illusion (a 124s ask_user looks like a 124s
  // stall). Freeze the clock, calm the chrome, and say so plainly.
  const awaitingInput = $derived(liveTool?.kind === "ask");
  // Manual /compact turn: the CLI compacts natively — no tools, no text, just
  // silence until the compact_boundary lands — so the generic "Working…" read
  // as a hang. Dedicated status + progress card below keep it honest.
  const manualCompacting = $derived(streaming && !chatGptTurn && !!liveTab?.compactingTurn);

  // Live footer meta — spec's `Unfurling… 5s · 312 tokens`. 1s ticker drives
  // elapsed + tokens; both pull from the assistant store (turnStartedAt +
  // liveOutputTokens already maintained by streaming.ts, no backend work).
  let now = $state(0);
  $effect(() => {
    if (!streaming) return;
    now = Date.now();
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  const liveSecs = $derived.by(() => {
    const t = liveTab?.activity.turnStartedAt;
    return streaming && t != null && now > 0 ? Math.max(0, Math.round((now - t) / 1000)) : null;
  });
  const liveTokens = $derived.by(() => {
    const lt = liveTab?.liveOutputTokens ?? 0;
    return streaming && lt > 0 ? lt : null;
  });

  // Auto-compaction detector. The CLI compacts in-process when context reaches
  // its auto-compact trigger — at turn start or mid-turn the stream just goes
  // silent for minutes until the compact_boundary lands, which used to read as
  // an unexplained hang (the stall copy even blamed the model). No early event
  // exists, so infer it: near the trigger + a long stream silence + not
  // thinking/parked. The trigger honors the user's auto-compact tuning
  // (autoCompactTriggerFor — a 250K override on a 1M window fires at 25% of
  // the gauge, which the old `ctxPct >= 80` gate could never see; null =
  // user disabled auto-compact, so never claim it). A wrong guess is cheap —
  // the card hedges and the backend watchdog still auto-ends a truly dead
  // turn. lastStreamEventAt is a plain field; the 1s `now` ticker is what
  // re-evaluates this.
  const sinceEventSecs = $derived.by(() => {
    if (!streaming || now === 0) return 0;
    const last = liveTab?.lastStreamEventAt ?? liveTab?.activity.turnStartedAt;
    return last != null ? Math.max(0, (now - last) / 1000) : 0;
  });
  const acTrigger = $derived(liveTab ? assistant.autoCompactTriggerFor(liveTab) : null);
  const autoCompacting = $derived(
    streaming && !manualCompacting && !awaitingInput && !turn.thinking?.active &&
    acTrigger != null && assistant.ctxTokensFor(liveTab) >= acTrigger * 0.9 &&
    sinceEventSecs >= 12,
  );
  const compacting = $derived(manualCompacting || autoCompacting);
  // Honest-estimate progress: the CLI emits no compaction progress events, so
  // pace a bar off the history size and hold it short of done — the card leaves
  // the moment the boundary lands. Elapsed anchors to the silence start for
  // auto (compaction began when the stream went quiet), turn start for manual.
  const compactElapsed = $derived(autoCompacting ? sinceEventSecs : (liveSecs ?? 0));
  const compactEstSecs = $derived(
    Math.min(210, Math.max(35, 20 + (assistant.ctxTokensFor(liveTab) / 1000) * 0.3)),
  );
  const compactProgress = $derived(compacting ? Math.min(0.97, compactElapsed / compactEstSecs) : 0);
  const compactEtaLabel = $derived.by(() => {
    const left = Math.round(compactEstSecs - compactElapsed);
    return left > 8 ? `~${fmtDur(left)} left` : "wrapping up…";
  });

  // The head is reserved for turn-wide states that have no transcript block.
  // Thinking always stays in StreamThinking below, including text-less passes,
  // so a pass gaining plaintext never swaps one top status row for another.
  const thinkingNow = $derived(
    streaming && !!turn.thinking?.active && !turn.thinking?.text && !liveTool && liveTokens == null,
  );
  const headLabel = $derived(
    !streaming
      ? `Worked for ${fmtDur(turn.totalSecs)}`
      : awaitingInput
        ? "Waiting for you"
        : compacting
          ? (autoCompacting ? "Auto-compacting conversation…" : "Compacting conversation…")
          : "Working…"
  );
  // Completed-turn hover timestamp — `message.ts` stamped at send (2026-07-02+);
  // absent on older convos, hidden while live (the head already ticks then).
  const turnTime = $derived.by(() => {
    const t = message.ts;
    if (!t || streaming) return null;
    const d = new Date(t);
    const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
    return d.toDateString() === new Date().toDateString()
      ? time
      : `${d.toLocaleDateString(undefined, { month: "short", day: "numeric" })} · ${time}`;
  });

  // Live footer verb: a pending tool drives the real action word ("Reading X").
  // With NO tool in flight (model reasoning before/between tool calls) we no
  // longer invent a whimsical verb ("Mapping…", "Pondering…") — that claimed an
  // action the model wasn't doing and read as the app fabricating status. Fall
  // back to the honest turn state instead: "Thinking…" when a reasoning pass is
  // live, else a plain "Working…". The head shows the same state word, so the
  // two never contradict, and the footer only says something specific when a
  // real tool is actually running.
  // Broader than thinkingNow: a MID-turn reasoning pass (after tokens/tools
  // already landed) previously fell through to a bare "Working…" for its whole
  // duration — the model was thinking, the footer just never said so.
  const reasoningNow = $derived(streaming && !!turn.thinking?.active);
  const idleVerb = $derived(reasoningNow ? "Thinking" : "Working");
  // Stall watchdog: the turn is live but NOTHING has come back — no tool in
  // flight, no output tokens. A short wait is normal model latency (first token
  // ~4s); a long silence can be the model OR a wedged local provider process. We
  // DON'T claim to know which — the old copy asserted "it's the Anthropic API",
  // which was a guess that read as a lie when the real cause was a stuck CLI.
  // The backend watchdog (STREAM_NO_PROGRESS_SECS=180s in turn.rs) auto-ends a
  // truly silent turn with an honest error; this UI just sets expectations until
  // then. Tiers: 0 normal · 1 soft (≥20s) · 2 strong (≥60s) · 3 wedged (≥150s,
  // approaching the backend's auto-end). Keep in lockstep w/ turn.rs's ceiling.
  const stallLevel = $derived.by(() => {
    // Active thinking is NOT a stall — Opus 4.8/4.7 ship thinking.display
    // "omitted", so a real reasoning pass streams only a signature (no text, no
    // token count), leaving liveTokens null for the whole think. Without this
    // guard the watchdog escalated to "Waiting on the model" while the head was
    // still showing "Thinking…" — the model was working, but the footer read as
    // a 45s hang. Gate the stall on that same text-less pass so the two never
    // contradict.
    // Compaction is a legitimately silent turn (summary runs CLI-side, nothing
    // streams until the boundary) — the watchdog copy would call it a stall.
    if (!streaming || liveTool || liveTokens != null || liveSecs == null || reasoningNow || compacting) return 0;
    if (liveSecs >= 150) return 3;
    if (liveSecs >= 60) return 2;
    if (liveSecs >= 20) return 1;
    return 0;
  });
  const footerVerb = $derived(
    compacting
      ? "Summarizing older messages"
      : liveTool
      ? VERB_ING[liveTool.kind]
      : stallLevel === 3
        ? "No response — ending soon"
        : stallLevel === 2
          ? "Still waiting for a response"
          : stallLevel === 1
            ? "Waiting on the model"
            : `${idleVerb}…`,
  );
  // The head already owns the turn-level state word ("Working…"/"Thinking…"), so
  // repeating idleVerb in the footer just stacked the SAME word top-and-bottom
  // (owner: "it feels duplicated"). The footer earns its row only when it can say
  // something the head can't: a real tool action, a compaction, a stall warning,
  // or an awaiting-input prompt. In the plain idle/reasoning gap the verb is
  // suppressed and the footer becomes a clean detail line — last-action trail +
  // the elapsed·tokens meter — with no echoed word.
  const footerVerbShown = $derived(
    liveTool || compacting || stallLevel > 0 || awaitingInput ? footerVerb : null,
  );
  // The caption of the live tool ("composer.svelte", "git status …") rides next
  // to the verb so the footer reads as one honest phrase — "Reading composer.svelte"
  // — instead of a bare verb with the target stranded in a work row above. Dir
  // prefix dropped here (the row above carries the full path); footer stays tight.
  // Suppressed in the 'focused' (calm) narration density: that mode's whole point
  // is a clean, minimal stream, so the footer stays a bare verb + meter there.
  // Some caps ARE verb phrases (ExitPlanMode → "Proposed a plan") — pairing
  // those with a footer verb reads doubled ("Proposed a plan Proposed a plan").
  const capIsVerbPhrase = (t: StreamTool) => t.cap === VERB_PAST[t.kind];
  const footerCap = $derived(
    liveTool && !awaitingInput && uiPrefs.narration !== "focused" && !capIsVerbPhrase(liveTool)
      ? (liveTool.cap ?? null)
      : null,
  );
  // Dead-air trail — with no tool in flight the footer used to collapse to a
  // bare "Working…" for the whole between-tools gap (the #1 "is it stuck?"
  // read on long turns). Keep the verb honest, but carry the LAST finished
  // action as a dim past-tense trail ("· Read Composer.svelte") so the quiet
  // stretch still says where the turn is. Same focused-mode gate as footerCap.
  const lastDone = $derived.by((): StreamTool | null => {
    let last: StreamTool | null = null;
    for (const g of groups) {
      if (g.type !== "work") continue;
      for (const seg of g.segs) {
        const tools = seg.seg === "rich" ? [seg.tool] : seg.tools;
        for (const t of tools) if (t.status !== "pending") last = t;
      }
    }
    return last;
  });
  const idleTrail = $derived(
    !liveTool && !awaitingInput && stallLevel === 0 && lastDone && uiPrefs.narration !== "focused"
      ? capIsVerbPhrase(lastDone)
        ? VERB_PAST[lastDone.kind]
        : `${VERB_PAST[lastDone.kind]} ${lastDone.cap}`
      : null,
  );
</script>

<div class="sturn">
  {#snippet renderGroup(g: Group, gi: number, historicalReceipt: boolean)}
    {#if g.type === "say"}
      {@const sm = sayMode(g.text, streaming && gi === groups.length - 1, gi)}
      {#if sm === "hide"}
        <!-- connective narration demoted to nothing (focused mode / pure filler) -->
      {:else if sm === "muted"}
        <div class="snarr-beat">{g.text.trim()}</div>
      {:else}
        <div class="snarr">
          <Markdown text={g.text} {streaming} {workspaceRoot} />
        </div>
      {/if}
    {:else if g.type === "steer"}
      <div class="ssteer" class:has-imgs={g.imgs.length > 0} use:tooltip={`Sent while ${providerInline} was working — read mid-turn`}>
        <CornerDownRight size={11} />
        <span class="ssteer-body">
          <span class="ssteer-text">{g.text}</span>
          {#if g.imgs.length > 0}
            <span class="ssteer-imgs">
              {#each g.imgs as im, ii (ii)}
                {@const safeMime = /^image\/(png|jpeg|gif|webp|svg\+xml|avif|bmp)$/.test(im.mime ?? "") ? im.mime : "image/png"}
                <img class="ssteer-thumb" src={`data:${safeMime};base64,${im.dataBase64}`} alt="" loading="lazy" />
              {/each}
            </span>
          {/if}
        </span>
        <span class="ssteer-tag">mid-turn</span>
      </div>
    {:else}
      {#each g.segs as seg, si (si)}
        {#if seg.seg === "rich"}
          {#if seg.tool.kind === "plan"}
            {@const planItems = planItemsFor(seg.tool)}
            {#if planItems.length}<StreamPlan items={planItems} />{/if}
          {:else if seg.tool.kind === "web" || seg.tool.kind === "fetch"}
            <StreamWeb tool={seg.tool} />
          {:else if seg.tool.kind === "test" || seg.tool.kind === "lint"}
            <StreamResult tool={seg.tool} />
          {:else if seg.tool.kind === "agent"}
            {@const spawn = liveTab?.agentSpawns.find((a) => a.id === seg.tool.id)}
            {@const childSpawns = liveTab?.agentSpawns.filter((a) => a.parentSpawnId === seg.tool.id) ?? []}
            <StreamAgent tool={seg.tool} {spawn} {childSpawns} {workspaceRoot} />
          {:else if seg.tool.kind === "ask"}
            <StreamAskUser tool={seg.tool} live={streaming} />
          {:else if seg.tool.kind === "exitplan"}
            <StreamExitPlan tool={seg.tool} tab={liveTab} {isLast} {workspaceRoot} />
          {:else if seg.tool.kind === "shell"}
            <StreamShell
              tool={seg.tool}
              poll={seg.poll}
              historical={historicalReceipt}
              bulkOpen={outputBulkOpen}
              bulkVersion={outputBulkVersion}
            />
          {/if}
          {#each pendingPerms([seg.tool]) as pt (pt.id)}
            <PermissionBar toolUseId={pt.id} toolName={pt.name} />
          {/each}
        {:else if seg.seg === "edit"}
          <WriteBatch tools={seg.tools} {workspaceRoot} />
          {#each pendingPerms(seg.tools) as pt (pt.id)}
            <PermissionBar toolUseId={pt.id} toolName={pt.name} />
          {/each}
        {:else}
          <WorkLine tools={seg.tools} {workspaceRoot} />
          {#each pendingPerms(seg.tools) as pt (pt.id)}
            <PermissionBar toolUseId={pt.id} toolName={pt.name} />
          {/each}
        {/if}
      {/each}
    {/if}
  {/snippet}

  <!-- The head earns its row only for awaiting-input or compaction states.
       StreamThinking owns reasoning from first signal through completion. -->
  {#if streaming && (awaitingInput || compacting)}
  <div class="sturn-head live" class:awaiting-head={awaitingInput}>
    <span class="sh-dot"></span>
    <span class="sh-label">{headLabel}</span>
  </div>
  {/if}

  {#if streaming}
    <div class="sturn-body" class:railed class:live={streaming}>
      <!-- Live stays chronological: nothing is regrouped while it arrives. -->
      {#if turn.thinking}
        <StreamThinking active={turn.thinking.active} durSecs={turn.thinking.durSecs} text={turn.thinking.text} {workspaceRoot} />
      {/if}
      {#each groups as g, gi (gi)}{@render renderGroup(g, gi, false)}{/each}
    </div>
  {:else}
    {#if hasActivity}
      <section class="sactivity" data-outcome={turn.outcome} class:open={activityOpen}>
        <button
          class="sactivity-head"
          type="button"
          aria-expanded={activityOpen}
          onclick={() => (activityOpen = !activityOpen)}
        >
          <span class="sactivity-chev" class:open={activityOpen}><ChevronRight size={13} /></span>
          <span class="sactivity-mark" aria-hidden="true">
            {#if turn.outcome === "failed"}<AlertTriangle size={12} />{:else}<Check size={12} />{/if}
          </span>
          <span class="sactivity-title">{activityTitle}</span>
          {#if turn.outcome === "applied" && turn.files > 0}
            <span class="sactivity-detail">{turn.files} file{turn.files === 1 ? "" : "s"}</span>
          {:else if actionCount > 0}
            <span class="sactivity-detail">{actionCount} action{actionCount === 1 ? "" : "s"}</span>
          {/if}
          <span class="sactivity-meta">
            {#if turn.totalSecs >= 1}<span>{fmtDur(turn.totalSecs)}</span>{/if}
            {#if completionTokens}<span>↑ {fmtTokens(completionTokens)}</span>{/if}
            {#if completionCost}<span class="sactivity-cost">{completionCost}</span>{/if}
            {#if message.fast}<span class="sactivity-fast"><Zap size={10} />fast</span>{/if}
            {#if turnTime}<span class="sactivity-time" use:tooltip={"When this turn ran"}>{turnTime}</span>{/if}
          </span>
        </button>
        {#if activityOpen}
          <div class="sactivity-body" transition:slide={{ duration: 180 }}>
            {#if activityShellCount > 0}
              <div class="sactivity-tools">
                <span>{activityShellCount} terminal {activityShellCount === 1 ? "result" : "results"}</span>
                <button type="button" onclick={toggleActivityOutput} aria-pressed={outputBulkOpen}>
                  {outputBulkOpen ? "Collapse output" : "Expand output"}
                </button>
              </div>
            {/if}
            <div class="sturn-body" class:railed={activityGroups.some((g) => g.type === "work") || !!turn.thinking}>
              {#if turn.thinking}
                <StreamThinking active={false} durSecs={turn.thinking.durSecs} text={turn.thinking.text} {workspaceRoot} />
              {/if}
              {#each activityGroups as g, gi (gi)}{@render renderGroup(g, gi, true)}{/each}
            </div>
          </div>
        {/if}
      </section>
    {/if}

    {#if answerGroups.length > 0}
      <div class="sanswer">
        {#each answerGroups as g, gi (gi)}
          {#if g.type === "say"}<div class="snarr"><Markdown text={g.text} {workspaceRoot} /></div>{/if}
        {/each}
      </div>
    {/if}

    {#if !hasActivity && (turn.totalSecs >= 1 || completionTokens || completionCost || message.fast)}
      <div class="sreceipt">
        {#if turn.totalSecs >= 1}<span>{fmtDur(turn.totalSecs)}</span>{/if}
        {#if completionTokens}<span>↑ {fmtTokens(completionTokens)} tokens</span>{/if}
        {#if completionCost}<span class="sreceipt-cost">{completionCost}</span>{/if}
        {#if message.fast}<span class="sreceipt-fast"><Zap size={10} />fast</span>{/if}
        {#if turnTime}<span class="sreceipt-time" use:tooltip={"When this turn ran"}>{turnTime}</span>{/if}
      </div>
    {/if}
  {/if}

  {#if streaming && (compacting || !thinkingNow)}
    <!-- While a pure reasoning pass is live StreamThinking already owns the
         status, so the footer stays suppressed until a tool or token lands. -->
    <div class="sfooter" class:stalled={stallLevel > 0} class:awaiting={awaitingInput}>
      <span class="sf-dot" aria-hidden="true"></span>
      {#key footerVerbShown ?? idleTrail}<span class="sf-verb-wrap">{#if footerVerbShown}<span class="sf-verb">{footerVerbShown}</span>{/if}{#if footerCap}<span class="sf-cap" class:sf-lead={!footerVerbShown}>{footerCap}</span>{:else if idleTrail}<span class="sf-cap sf-last" class:sf-lead={!footerVerbShown}>{footerVerbShown ? "· " : ""}{idleTrail}</span>{/if}</span>{/key}
      {#if awaitingInput}
        <!-- Parked on the user: no climbing clock (it's human time, not the
             model's), no token meter. Just the calm verb + a nudge. -->
        <span class="sf-meta sf-await-hint">— choose an option above to continue</span>
      {:else if liveSecs != null || liveTokens != null}
        <!-- one quiet left-clustered line (comp cstat): dot · verb · elapsed · ↑ tokens -->
        <span class="sf-cluster">
          {#if liveSecs != null}
            <span class="sf-meta"><AnimatedCount value={liveSecs} format={fmtDur} durationMs={300} /></span>
          {/if}
          {#if liveSecs != null && liveTokens != null}<span class="sf-pip">·</span>{/if}
          {#if liveTokens != null}
            <span class="sf-meta">↑ <AnimatedCount value={liveTokens} format={fmtTokens} /> tokens</span>
          {/if}
        </span>
      {/if}
    </div>
    {#if compacting}
      <div class="scompact-card">
        <div class="scompact-track"><span class="scompact-fill" style="transform:scaleX({compactProgress})"></span></div>
        <div class="scompact-meta">
          <span>~{Math.round(compactProgress * 100)}%</span>
          <span class="scompact-pip">·</span>
          <span>{liveTab?.messages.length ?? 0} messages · {fmtTokens(assistant.ctxTokensFor(liveTab))} tokens of history</span>
          <span class="scompact-pip">·</span>
          <span>{compactEtaLabel}</span>
        </div>
        <div class="scompact-note">
          {#if autoCompacting}
            This chat filled up {providerPossessive} working memory, so {providerInline} is making a quick recap of the
            older messages to clear some room. That usually takes a minute or two, and the turn
            picks back up on its own when it's done. Everything you see here stays put.
          {:else}
            {providerSubject} is rolling the older messages into a short recap to free up room. The full
            chat stays on screen, and things pick up right where they left off once the recap
            is ready.
          {/if}
        </div>
      </div>
    {:else if stallLevel >= 3}
      <div class="sstall-note">
        No output yet after a long wait — {providerInline} may be slow, or its local
        connection may be stuck. Rift will end the turn automatically if nothing
        arrives shortly. You can press Stop now and try again.
      </div>
    {:else if stallLevel > 0}
      <div class="sstall-note">
        Still waiting on the first token. This is usually just model latency —
        it'll stream as soon as a response starts. You can keep waiting or press Stop.
      </div>
    {/if}
  {/if}

  {#if !streaming && (plainText.length > 0 || isLast)}
    <div class="msg-acts">
      {#if plainText.length > 0}
        <button class="msg-act" class:copied type="button" onclick={copy}>
          {#if copied}<Check size={13} strokeWidth={2.5} />Copied{:else}<Copy size={13} />Copy{/if}
        </button>
      {/if}
      {#if isLast}
        <button class="msg-act" type="button" onclick={() => assistant.retryLast(tabId)}>
          <RotateCcw size={13} />Retry
        </button>
      {/if}
    </div>
  {/if}
</div>
