<script lang="ts">
  import "$lib/styles/stream.css";
  import { Check, Copy, RotateCcw, AlertTriangle } from "lucide-svelte";
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
  import PermissionBar from "../PermissionBar.svelte";
  import { messageToTurn, groupBlocks, fmtDur, classifySay, VERB_ING, tasksToPlanItems, type StreamTool } from "./streamModel";
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

  // Decide how a `say` block renders given the narration-density pref. Returns
  // "full" (normal prose block), "muted" (demoted inline connective beat), or
  // "hide". The live-streaming LAST block is always shown full — never demote the
  // text that's actively typing. `prose` is always full regardless of pref.
  function sayMode(text: string, isLiveLast: boolean): "full" | "muted" | "hide" {
    if (isLiveLast) return "full";
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
        : thinkingNow
          ? (liveSecs != null ? `Thinking… ${liveSecs}s` : "Thinking…")
          : "Working…"
  );

  // Live footer verb: a pending tool drives the real action word ("Reading X");
  // with no tool in flight (e.g. the model is thinking before any tool call) we
  // cycle a whimsical present-participle every ~2.4s so the turn reads as alive
  // instead of frozen. Mirrors app/stream.jsx StreamFooter (the DS reference).
  const WHIM_WORDS = [
    "Thinking", "Sussing", "Spelunking", "Pondering", "Brewing",
    "Reckoning", "Mulling", "Cogitating", "Hatching", "Conjuring",
    "Noodling", "Untangling",
  ];
  let whimTick = $state(0);
  $effect(() => {
    if (!streaming || liveTool) return;
    const id = setInterval(() => (whimTick = (whimTick + 1) % WHIM_WORDS.length), 2400);
    return () => clearInterval(id);
  });
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
    if (!streaming || liveTool || liveTokens != null || liveSecs == null || thinkingNow) return 0;
    if (liveSecs >= 150) return 3;
    if (liveSecs >= 60) return 2;
    if (liveSecs >= 20) return 1;
    return 0;
  });
  const footerVerb = $derived(
    liveTool
      ? VERB_ING[liveTool.kind]
      : stallLevel === 3
        ? "No response — ending soon"
        : stallLevel === 2
          ? "Still waiting for a response"
          : stallLevel === 1
            ? "Waiting on the model"
            : `${WHIM_WORDS[whimTick]}…`,
  );
</script>

<div class="sturn">
  <div class="sturn-head {streaming ? 'live' : ''}" class:awaiting-head={awaitingInput}>
    <span class="sh-dot"></span>
    <span class="sh-label">{headLabel}</span>
  </div>

  {#if turn.thinking && !turn.thinking.active}
    <StreamThinking active={turn.thinking.active} durSecs={turn.thinking.durSecs} text={turn.thinking.text} />
  {/if}

  {#each groups as g, gi (gi)}
    {#if g.type === "say"}
      {@const sm = sayMode(g.text, streaming && gi === groups.length - 1)}
      {#if sm === "hide"}
        <!-- connective narration demoted to nothing (focused mode / pure filler) -->
      {:else if sm === "muted"}
        <div class="snarr-beat">{g.text.trim()}</div>
      {:else}
        <!-- Trailing caret on the actively-streaming final text block only (CC-UI
             ref §2): a thin bar, step-end ~1s blink, scoped to the one block
             receiving tokens. Every settled block has none. -->
        <div class="snarr">
          <Markdown text={g.text} {streaming} />{#if streaming && gi === groups.length - 1}<span class="stream-caret" aria-hidden="true"></span>{/if}
        </div>
      {/if}
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
            <StreamAskUser tool={seg.tool} />
          {:else if seg.tool.kind === "shell"}
            <StreamShell tool={seg.tool} streaming={streaming && gi === groups.length - 1} />
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

  {#if streaming}
    <div class="sfooter" class:stalled={stallLevel > 0} class:awaiting={awaitingInput}>
      {#key footerVerb}<span class="sf-verb">{footerVerb}</span>{/key}
      {#if awaitingInput}
        <!-- Parked on the user: no climbing clock (it's human time, not the
             model's), no token meter. Just the calm verb + a nudge. -->
        <span class="sf-meta sf-await-hint">— choose an option above to continue</span>
      {:else}
        <!-- The command/file caption is intentionally NOT repeated here — it
             already shows in the active work row above (was a duplicate). -->

        {#if liveSecs != null}
          <span class="sf-pip">·</span>
          <span class="sf-meta"><AnimatedCount value={liveSecs} durationMs={300} />s</span>
        {/if}
        {#if liveTokens != null}
          <span class="sf-pip">·</span>
          <span class="sf-meta"><AnimatedCount value={liveTokens} format={fmtTokens} /> tokens</span>
        {/if}
      {/if}
    </div>
    {#if stallLevel >= 3}
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
  {:else if turn.outcome !== "text"}
    <div class="sapplied" data-outcome={turn.outcome}>
      {#if turn.outcome === "applied"}
        <span class="ok"><Check size={13} strokeWidth={2.5} /> Applied</span>
        <span class="files">{turn.files} file{turn.files === 1 ? "" : "s"}</span>
      {:else if turn.outcome === "failed"}
        <span class="bad"><AlertTriangle size={13} strokeWidth={2.5} /> Changes failed</span>
      {:else}
        <span class="ran"><Check size={13} strokeWidth={2.5} /> Done</span>
      {/if}
      {#if turn.meta}
        <span class="sapplied-meta">{turn.meta.time}</span>
        {#if turn.meta.cost}<span class="sapplied-cost" use:tooltip={"Total cost of this turn"}>{turn.meta.cost}</span>{/if}
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
