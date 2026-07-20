<script lang="ts">
  import "$lib/styles/stream.css";
  import { Check, Copy, RotateCcw, AlertTriangle, Zap, CornerDownRight } from "lucide-svelte";
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
  import { messageToTurn, groupBlocks, fmtDur, classifySay, VERB_ING, VERB_PAST, tasksToPlanItems, type StreamTool } from "./streamModel";
  import { assistant, type ChatMessage, type TabState } from "$lib/state/assistant.svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import { fmtTokens } from "$lib/state/assistant/helpers";
  import AnimatedCount from "./AnimatedCount.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  // `tab` is THIS pane's tab. Live-turn status (activity timer, live tokens,
  // plan tasks) must read from it, not the global `assistant.*` getters — those
  // delegate to the single focused activeTab, so in split-pane every pane would
  // otherwise mirror the focused pane's "Working… 109s". Falls back to the
  // active tab when omitted (single-pane callers).
  let { message, streaming = false, isLast = false, tab = null }:
    { message: ChatMessage; streaming?: boolean; isLast?: boolean; tab?: TabState | null } = $props();
  const liveTab = $derived(tab ?? assistant.activeTab);

  const turn = $derived(messageToTurn(message));
  const groups = $derived(groupBlocks(turn.blocks));
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
  // as a hang. Dedicated status + reassurance note below keep it honest.
  const compacting = $derived(streaming && !!liveTab?.compactingTurn);

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

  // The head is the TURN-level status: "Working…" live, "Worked for Xs" done.
  // EXCEPTION — a thinking block that's active before any tool/text lands: Opus
  // omits thinking text, so that 6-8s gap otherwise shows only "Working…" with
  // no signal of what's happening → reads as a hang ("everything is slow"). Say
  // "Thinking…" plainly here so the gap reads as the model reasoning. Once a tool
  // or token lands, `thinkingNow` falls false and the head returns to "Working…"
  // (the footer verb + work rows take over). The done collapsible "Thought for
  // Xs" is still the StreamThinking block below.
  const thinkingNow = $derived(
    streaming && !!turn.thinking?.active && !liveTool && liveTokens == null,
  );
  const headLabel = $derived(
    !streaming
      ? `Worked for ${fmtDur(turn.totalSecs)}`
      : awaitingInput
        ? "Waiting for you"
        : compacting
          ? "Compacting conversation…"
          : thinkingNow
            ? (liveSecs != null ? `Thinking… ${fmtDur(liveSecs)}` : "Thinking…")
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
  // ~4s); a long silence can be the model OR a wedged local Claude process. We
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
    // a 45s hang. thinkingNow gates the head; gate the stall on it too so the
    // two never contradict.
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
  const footerCap = $derived(
    liveTool && !awaitingInput && uiPrefs.narration !== "focused" ? (liveTool.cap ?? null) : null,
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
      ? `${VERB_PAST[lastDone.kind]} ${lastDone.cap}`
      : null,
  );
</script>

<div class="sturn">
  <div class="sturn-head {streaming ? 'live' : ''}" class:awaiting-head={awaitingInput}>
    <span class="sh-dot"></span>
    <span class="sh-label">{headLabel}</span>
    {#if turnTime}<span class="sh-time" use:tooltip={"When this turn ran"}>{turnTime}</span>{/if}
  </div>

  <div class="sturn-body" class:railed class:live={streaming}>
  {#if turn.thinking && !turn.thinking.active}
    <StreamThinking active={turn.thinking.active} durSecs={turn.thinking.durSecs} text={turn.thinking.text} />
  {/if}

  {#each groups as g, gi (gi)}
    {#if g.type === "say"}
      {@const sm = sayMode(g.text, streaming && gi === groups.length - 1, gi)}
      {#if sm === "hide"}
        <!-- connective narration demoted to nothing (focused mode / pure filler) -->
      {:else if sm === "muted"}
        <div class="snarr-beat">{g.text.trim()}</div>
      {:else}
        <div class="snarr">
          <Markdown text={g.text} {streaming} />
        </div>
      {/if}
    {:else if g.type === "steer"}
      <div class="ssteer" use:tooltip={"Sent while Claude was working — read mid-turn"}>
        <CornerDownRight size={11} />
        <span class="ssteer-text">{g.text}</span>
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
            <StreamAgent tool={seg.tool} {spawn} />
          {:else if seg.tool.kind === "ask"}
            <StreamAskUser tool={seg.tool} live={streaming} />
          {:else if seg.tool.kind === "exitplan"}
            <StreamExitPlan tool={seg.tool} />
          {:else if seg.tool.kind === "shell"}
            <StreamShell tool={seg.tool} poll={seg.poll} />
          {/if}
          {#each pendingPerms([seg.tool]) as pt (pt.id)}
            <PermissionBar toolUseId={pt.id} toolName={pt.name} />
          {/each}
        {:else if seg.seg === "edit"}
          <WriteBatch tools={seg.tools} />
          {#each pendingPerms(seg.tools) as pt (pt.id)}
            <PermissionBar toolUseId={pt.id} toolName={pt.name} />
          {/each}
        {:else}
          <WorkLine tools={seg.tools} />
          {#each pendingPerms(seg.tools) as pt (pt.id)}
            <PermissionBar toolUseId={pt.id} toolName={pt.name} />
          {/each}
        {/if}
      {/each}
    {/if}
  {/each}
  </div>

  {#if streaming && (compacting || !thinkingNow)}
    <!-- While a pure reasoning pass is live the HEAD already shows "Thinking… Xs"
         — the footer would just duplicate it (and there's no action/tokens to
         report yet), so it's suppressed until a tool or token lands. -->
    <div class="sfooter" class:stalled={stallLevel > 0} class:awaiting={awaitingInput}>
      {#key footerVerbShown ?? idleTrail}<span class="sf-verb-wrap">{#if footerVerbShown}<span class="sf-verb">{footerVerbShown}</span>{/if}{#if footerCap}<span class="sf-cap" class:sf-lead={!footerVerbShown}>{footerCap}</span>{:else if idleTrail}<span class="sf-cap sf-last" class:sf-lead={!footerVerbShown}>{footerVerbShown ? "· " : ""}{idleTrail}</span>{/if}</span>{/key}
      {#if awaitingInput}
        <!-- Parked on the user: no climbing clock (it's human time, not the
             model's), no token meter. Just the calm verb + a nudge. -->
        <span class="sf-meta sf-await-hint">— choose an option above to continue</span>
      {:else if liveSecs != null || liveTokens != null}
        <!-- verb+caption read as one phrase on the left; elapsed·tokens sit as a
             quiet mono cluster hugging the right edge. -->
        <span class="sf-fill" aria-hidden="true"></span>
        <span class="sf-cluster">
          {#if liveSecs != null}
            <span class="sf-meta"><AnimatedCount value={liveSecs} format={fmtDur} durationMs={300} /></span>
          {/if}
          {#if liveSecs != null && liveTokens != null}<span class="sf-pip">·</span>{/if}
          {#if liveTokens != null}
            <span class="sf-meta"><AnimatedCount value={liveTokens} format={fmtTokens} /> tokens</span>
          {/if}
        </span>
      {/if}
    </div>
    {#if compacting}
      <div class="scompact-note">
        Older messages are being condensed into a short summary to free up
        context. Nothing is deleted — the full transcript stays right here,
        and the chat continues where it left off once the summary lands.
      </div>
    {:else if stallLevel >= 3}
      <div class="sstall-note">
        No output yet after a long wait — this can be a slow response or a stuck
        local Claude process. Rift will end the turn automatically if nothing
        arrives shortly. You can press Stop now and try again.
      </div>
    {:else if stallLevel > 0}
      <div class="sstall-note">
        Still waiting on the first token. This is usually just model latency —
        it'll stream as soon as a response starts. You can keep waiting or press Stop.
      </div>
    {/if}
  {:else if !streaming && turn.outcome !== "text"}
    <div class="sapplied" data-outcome={turn.outcome}>
      {#if turn.outcome === "applied"}
        <span class="ok"><Check size={13} strokeWidth={2.5} /> Applied</span>
        <span class="files">{turn.files} file{turn.files === 1 ? "" : "s"}</span>
      {:else if turn.outcome === "failed"}
        <span class="bad"><AlertTriangle size={13} strokeWidth={2.5} /> Changes failed</span>
      {:else if turn.outcome === "planned"}
        <span class="ran"><Check size={13} strokeWidth={2.5} /> Plan proposed</span>
      {:else}
        <span class="ran"><Check size={13} strokeWidth={2.5} /> Done</span>
      {/if}
      {#if turn.meta}
        <span class="sapplied-meta">{turn.meta.time}</span>
        {#if turn.meta.cost}<span class="sapplied-cost" use:tooltip={"Total cost of this turn"}>{turn.meta.cost}</span>{/if}
        {#if turn.meta.fast}<span class="sapplied-fast" use:tooltip={"This turn ran in fast mode — quicker Opus output, billed from your usage credits (pay-per-use)"}><Zap size={11} />fast</span>{/if}
      {/if}
    </div>
  {/if}

  {#if !streaming && (plainText.length > 0 || isLast)}
    <div class="msg-acts">
      {#if plainText.length > 0}
        <button class="msg-act" class:copied type="button" onclick={copy}>
          {#if copied}<Check size={13} strokeWidth={2.5} />Copied{:else}<Copy size={13} />Copy{/if}
        </button>
      {/if}
      {#if isLast}
        <button class="msg-act" type="button" onclick={() => assistant.retryLast()}>
          <RotateCcw size={13} />Retry
        </button>
      {/if}
    </div>
  {/if}
</div>
