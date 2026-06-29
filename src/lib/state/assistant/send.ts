// M9 (per docs/design/assistant-svelte-split.md) — the send orchestrator
// lifted out of `src/lib/state/assistant.svelte.ts` as free fns over the
// AssistantStore ref: turn dispatch (send + slash commands), the per-tab
// outbound queue (drain + remove), mid-turn steer, stop, retry, copy and
// prompt recall. enhancePrompt (stateless wand) and the turn-complete hook
// wiring stay on the store.
//
// Like M8 (streaming.ts) this imports the store/tab types directly
// (type-only — erased, no runtime cycle) instead of a structural shape copy:
// send touches most of the store's public surface. Bodies ported verbatim
// (this.* → store.*) so the baked-in invariants survive: the auth chokepoint,
// queue-on-busy, #143 ensureTab-before-field-writes, #146 streamingMsgIdx
// cache, #148 drain re-queue on tab switch, #179 flush-before-stop, #184
// stale-error clear, #185 retry re-entrance guard.

import { invoke } from "@tauri-apps/api/core";
import { accessibility } from "../accessibility.svelte";
import { toast, notify } from "../toast.svelte";
import type { AssistantStore, TabState } from "../assistant.svelte";
import type { Block, ChatMessage, TurnRecord } from "./types";
import { effortToFlag, FABLE_SUNSET_MS } from "./helpers";
import { finalizeInflightBlocks } from "./streaming";

// One-shot per app session — the sunset warning shouldn't nag on every send.
let fableSunsetNoticed = false;

export async function send(store: AssistantStore, prompt: string) {
  const trimmed = prompt.trim();
  // Empty prompts are allowed when attachments are staged (paste-and-go).
  // Drop only if the prompt AND both attachment kinds are empty.
  if (!trimmed && store.composerAttachments.length === 0 && store.composerTextAttachments.length === 0) return;
  // Try-handle as a slash command first; if it matched, we're done.
  if (trimmed.startsWith("/") && runSlash(store, trimmed)) return;
  // Auth chokepoint — every send path funnels here (composer Enter/button,
  // queue drains, programmatic retries). A turn with no usable Claude session
  // dies as "claude exited with 1"; block it, re-probe (state may be stale),
  // and surface the reason. Slash commands above are local, so they still run.
  if (!store.authReady) {
    notify.danger("Claude isn't set up", {
      detail: store.auth?.summary ?? "Open Settings to sign in or add an API key.",
    });
    void store.refreshAuth();
    return;
  }
  // Already streaming on this tab → queue instead of dropping.
  if (store.streaming) {
    store.queue = [...store.queue, { id: crypto.randomUUID(), text: trimmed }];
    return;
  }
  // Phase 2 (S72): the CLI owns conversation state now. First turn mints a
  // UUID and passes `--session-id`; subsequent turns pass `--resume`.
  // v0.4: newTab() mints currentConvoId up-front so the tab can render
  // before send() — gate isFirstTurn on convoCreatedAt instead so the very
  // first send still passes --session-id, not --resume.
  if (!store.currentConvoId) {
    store.currentConvoId = crypto.randomUUID();
  }
  // #143: per-tab fields live on TabState now — ensureTab BEFORE touching
  // them so the writes don't no-op via the store's activeTab=null setter
  // path. ensureTab seeds cliSessionId to convoId for fresh tabs.
  const tab = store.ensureTab(store.currentConvoId, store.currentConvoId);
  const isFirstTurn = !tab.convoCreatedAt;
  if (!tab.cliSessionId) {
    tab.cliSessionId = store.currentConvoId;
  }
  if (!tab.convoCreatedAt) {
    tab.convoCreatedAt = Date.now();
    tab.convoTitle = null;
  }
  // Capture the model this session pins to backend-side on its first turn — the
  // backend ignores a later picker switch on a resumed session, so this is the
  // model the running turns truly use (drives the picker's "this session" tag +
  // "New chat in <model>" honesty). Only set once, on the first turn.
  if (isFirstTurn && !tab.pinnedModel) {
    tab.pinnedModel = store.effectiveModel;
  }
  // A real turn — advance the sidebar's activity clock. Tab-switch auto-saves
  // deliberately don't touch this, so opening a chat no longer reshuffles.
  tab.lastActivityAt = Date.now();
  // v0.4: catches the raw newConversation→send path (slash /new) so tabs
  // never drift out of sync with the streaming convo.
  if (!store.openTabs.includes(store.currentConvoId)) {
    store.openTabs = [...store.openTabs, store.currentConvoId];
    store.persistTabs();
  }
  tab.beginTurn();
  store.lastNotice = null;
  // #184: clear stale error banner so it doesn't bleed into the new turn.
  // Setter routes to tab.lastError when activeTab is set, store-level otherwise.
  store.lastError = null;
  // No workspace AND no scratch fallback → the backend runs the turn in no-tools
  // mode; say so up front. In local mode (scratch available) the turn silently
  // runs in `%LOCALAPPDATA%\Rift\local` with full tools — the "Local" badge is
  // the signal, so stay quiet.
  if (!store.workspace.current && !store.localScratchPath) {
    notify.warn("No folder open", {
      detail: "The assistant can't read or edit files this turn. Open one from the title bar.",
    });
  }
  // turn.rs swaps Fable to Opus silently once the limited run ends — warn ahead.
  if (!fableSunsetNoticed && store.effectiveModel === "claude-fable-5"
      && Date.now() >= FABLE_SUNSET_MS - 7 * 86_400_000) {
    fableSunsetNoticed = true;
    notify.warn(
      Date.now() >= FABLE_SUNSET_MS
        ? "Fable's limited run has ended — this turn falls back to Opus 4.8."
        : "Heads up: Fable retires June 22 — chats fall back to Opus 4.8 after that.",
    );
  }
  // Telemetry: build the turn record + attach to tab. TabState fills it as
  // envelopes arrive; finalized in onDone/onError.
  const attachBytes = store.composerAttachments.reduce((s, a) => s + a.sizeBytes, 0);
  // The send tier IS the user's persisted tier — no per-turn auto-scaling.
  // (#244's autoScaleEffort is retired: the default tier now maps to `medium`,
  // so a trivial greeting is already light, AND any per-turn effort change forces
  // a warm-pool cold respawn — `effort_level` is baked into the SpawnKey
  // (warm_pool.rs), so the "optimization" silently triggered the ~1.7s slow path
  // it was meant to avoid. Stable effort across turns = the warm child is reused.)
  const sendEffort = store.thinkingEffort;
  const turnRecord: TurnRecord = {
    ts: Date.now(),
    convoId: store.currentConvoId,
    cliSessionId: tab.cliSessionId,
    isFirstTurn,
    model: store.effectiveModel,
    effort: sendEffort,
    effortFlag: effortToFlag(sendEffort, store.effectiveModel),
    promptLen: trimmed.length,
    promptPreview: trimmed.length > 120 ? trimmed.slice(0, 120) + "…" : trimmed,
    attachmentsCount: store.composerAttachments.length,
    attachmentsBytes: attachBytes,
    envelopeUsage: null,
    resultUsage: null,
    modelId: null,
    costUsd: null,
    deltaCount: 0,
    streamEventCount: 0,
    assistantEnvCount: 0,
    maxStreamGapMs: 0,
    toolUses: [],
    thinkingCount: 0,
    thinkingTotalMs: 0,
    thinkingBlocks: [],
    envelopeFallback: false,
    blankTurn: false,
    firstPaintAt: null,
    doneAt: null,
    endKind: null,
  };
  store.telemetry.pushTurn(turnRecord);
  tab.currentTurnRecord = turnRecord;
  // Track for /retry and Up-arrow recall. De-dupe consecutive identicals.
  if (tab.promptHistory[tab.promptHistory.length - 1] !== trimmed) {
    tab.promptHistory = [...tab.promptHistory, trimmed].slice(-50);
  }
  // User bubble text: when paste-and-go with no text, show an attachment
  // marker so the bubble isn't blank. Counts both kinds.
  const attachCount = store.composerAttachments.length;
  const textCount = store.composerTextAttachments.length;
  const markerParts: string[] = [];
  if (attachCount > 0) markerParts.push(`📎 ${attachCount} image${attachCount === 1 ? "" : "s"}`);
  if (textCount > 0) markerParts.push(`📄 ${textCount} file${textCount === 1 ? "" : "s"}`);
  const bubbleText = trimmed.length > 0 ? trimmed : markerParts.join(" · ");
  // Build the user message blocks — image blocks (one per attachment) first,
  // then the text block. Order matches the visual stack (thumbs above text)
  // in MessageBubble's user-side render path.
  const userBlocks: Block[] = [];
  for (const a of store.composerAttachments) {
    userBlocks.push({
      type: "image",
      mime: a.mime,
      dataBase64: a.dataBase64,
      sizeBytes: a.sizeBytes,
    });
  }
  // Text-file attachments: a compact marker block listing the filenames goes in
  // the visible bubble (the full contents are inlined into the prompt below, not
  // shown — they'd flood the transcript). One line per file so the user sees
  // what they sent.
  for (const t of store.composerTextAttachments) {
    userBlocks.push({ type: "text", text: `📄 ${t.name}${t.truncated ? " (truncated)" : ""}` });
  }
  userBlocks.push({ type: "text", text: bubbleText });
  tab.messages = [
    ...tab.messages,
    { id: crypto.randomUUID(), role: "user", blocks: userBlocks },
  ];
  const asst: ChatMessage = { id: crypto.randomUUID(), role: "assistant", blocks: [] };
  tab.messages = [...tab.messages, asst];
  tab.streamingMsgId = asst.id;
  // #146: asst placeholder is at the tail of messages; cache its index so
  // mutateStreaming can index-replace instead of scanning the full array.
  tab.streamingMsgIdx = tab.messages.length - 1;
  // Snapshot attachments for this turn + clear the composer so a fast retype
  // doesn't accidentally re-attach.
  const turnAttachments = store.composerAttachments.map((a) => ({
    mime: a.mime,
    dataBase64: a.dataBase64,
  }));
  // Inline text-file attachments into the prompt as fenced blocks before the
  // user's typed text. The backend pipes `prompt` to the CLI verbatim, so no
  // backend change is needed — the assistant simply sees the file contents.
  const textBlocks = store.composerTextAttachments
    .map((t) => `\`\`\`${t.name}\n${t.text}\n\`\`\``)
    .join("\n\n");
  const effectivePrompt = textBlocks
    ? (trimmed ? `${textBlocks}\n\n${trimmed}` : textBlocks)
    : trimmed;
  store.composerAttachments = [];
  store.composerTextAttachments = [];
  try {
    await invoke("assistant_send", {
      prompt: effectivePrompt,
      sessionId: tab.cliSessionId,
      isFirstTurn,
      model: store.effectiveModel,
      attachments: turnAttachments.length > 0 ? turnAttachments : null,
      dyslexiaMode: accessibility.dyslexiaMode,
      thinkingEffort: sendEffort,
      thinkingEnabled: store.thinkingEnabled,
      permissionMode: store.permissionMode,
      // priorContextSummary is intentionally omitted (defaults to None backend-
      // side): the CLI does compaction natively in-process now, so Rift never
      // re-injects a prior-conversation summary. The backend keeps the param as a
      // forward-compat hook (turn.rs Phase C) — don't re-add a hard-coded null here.
      // Per-tab root: each pane/window runs its turns in its own folder. Only
      // read on the first turn backend-side (then pinned per-session).
      root: store.effectiveRoot(tab),
    });
  } catch (e) {
    tab.onError(String(e));
  }
}

/** Fire the next queued message on `tab`, if any. Idempotent + guarded so
 *  it's safe to call from every terminal turn path (onDone / onError /
 *  session-lost) AND on tab activation — backgrounded completions defer the
 *  drain (auto-sending into a tab the user isn't looking at is surprising),
 *  so returning to the tab must re-trigger it or the queue strands forever.
 *  Bails unless `tab` is the active tab and idle. */
export function drainQueue(store: AssistantStore, tab: TabState | null) {
  // No `tab.lastError` gate: it's cleared by beginTurn (inside send()), which
  // only drainQueue→send reaches — gating here would deadlock the queue behind
  // a stale error forever. send() clears lastError before anything visible.
  //
  // Split-pane: the old `tab !== store.activeTab` gate stranded a queued
  // message forever on any pane the user wasn't focused on — in a 2-pane split
  // BOTH panes are visible, so a completion on the unfocused pane could never
  // drain its own queue until the user clicked it. Allow the active tab (the
  // single-pane case + the focused pane) OR any tab shown in a sibling pane;
  // still block a truly backgrounded tab (open in no pane, not active) so we
  // don't auto-send into a chat the user can't see (the original intent).
  const drainEligible =
    tab === store.activeTab ||
    store.panes.some((p) => p.tabId != null && store.tabFor(p.tabId) === tab);
  if (!tab || !drainEligible || tab.streaming || tab.queue.length === 0) return;
  // Fire the head of the queue as the next turn. The queue order IS the send
  // order (drag-to-reorder in the rail). RR7: PEEK the head, don't pop yet.
  // Popping here (before the microtask) left a window where closeTab could read
  // tab.queue.length for its "N discarded" toast AFTER the item was already
  // removed but BEFORE the microtask could re-queue it — undercounting by one
  // and silently dropping the in-flight item when dropTab removed the tab first.
  // The item now stays in the queue until the microtask actually sends it, so
  // closeTab's count is always accurate.
  const next = tab.queue[0];
  // #148: capture the COMPLETING TAB's own convoId (its Map key) at peek time,
  // not the globally-active convo. The old `store.currentConvoId` capture
  // bailed whenever the user merely shifted pane focus during the microtask —
  // in split-pane that stranded the completing pane's queue even though its own
  // tab was unchanged. Reverse-look-up the key so the guard tracks THIS tab.
  let capturedTabConvoId: string | null = null;
  for (const [cid, t] of store.tabs) {
    if (t === tab) { capturedTabConvoId = cid; break; }
  }
  queueMicrotask(() => {
    // Bail if the tab was retired (closed/cleared) or a new turn already started
    // on it. Focus moving to a sibling pane no longer cancels the drain.
    if (!capturedTabConvoId || store.tabs.get(capturedTabConvoId) !== tab || tab.streaming) return;
    // Item is still in the queue (we peeked); remove it now that we're committed
    // to sending. If it's already gone (raced drain), bail.
    if (!tab.queue.some((q) => q.id === next.id)) return;
    tab.queue = tab.queue.filter((q) => q.id !== next.id);
    // Route through the STORE wrapper (not the bare sendImpl) with the draining
    // tab's id: sendImpl keys off currentConvoId, so a drain on a non-focused
    // pane-visible tab must retarget first or the queued message fires into the
    // focused pane instead. store.send(text, tabId) does that retarget.
    store.send(next.text, capturedTabConvoId).catch(e => tab.onError(String(e)));
  });
}

/** Stop a tab's in-flight stream. Defaults to the focused-pane tab when
 *  `tabId` is omitted. Pre-clears the tab's streaming flag synchronously
 *  so any late `done` event for this session is idempotent. Other tabs
 *  keep streaming. */
export async function stop(store: AssistantStore, tabId?: string | null) {
  const tab = tabId ? store.tabFor(tabId) : store.activeTab;
  if (!tab || !tab.streaming) return;
  const sid = tab.cliSessionId;
  // #179: flush pacer-buffered text into the message BEFORE clearing
  // streamingMsgId — otherwise mutateStreaming's early-return drops it.
  tab.flushPendingText();
  // RR10/#64-fe: settle in-flight thinking/tool blocks before clearing
  // streamingMsgId, else a user Stop mid-reasoning persists a stuck
  // status:"active" thinking chip + status:"pending" tool chips in history.
  finalizeInflightBlocks(tab);
  tab.streaming = false;
  tab.streamingMsgId = null;
  tab.streamingMsgIdx = null;
  tab.deltaCount = 0;
  tab.envelopeTextBuffer = '';
  tab.seenToolUseIds.clear();
  // Mirror onStreamDone/onStreamError cleanup: stop() is the only terminal
  // path that runs while the late `done` event is discarded (onStreamDone
  // early-returns on !streaming), so these would otherwise leak past the turn.
  // Stale entries cause dead Allow/Deny chips + wrong-request ask_user binding
  // on the NEXT turn (leftover requestId FIFO-pairs with a fresh toolUseId).
  if (tab.permissionPrompts.size > 0) tab.permissionPrompts = new Map();
  tab.unboundAskUserRequestIds = [];
  tab.unboundAskUserToolUseIds = [];
  // RR7: clear ask_user bindings here too (third terminal path) — same dead-chip
  // reasoning as onStreamDone/onStreamError.
  if (tab.askUserBindings.size > 0) tab.askUserBindings = new Map();
  tab.activity = { ...tab.activity, currentLabel: null };
  // Telemetry finalize as user-stop before the late done event lands.
  if (tab.currentTurnRecord) {
    tab.currentTurnRecord.doneAt = Date.now();
    tab.currentTurnRecord.endKind = "user-stop";
    tab.currentTurnRecord = null;
  }
  store.telemetry.event("turn.stop", { convoId: tab.cliSessionId });
  try {
    await invoke("assistant_stop", { sessionId: sid });
  } catch (e) {
    console.warn("assistant_stop failed", e);
  }
  // Drain any message queued during the turn — the discarded late `done` event
  // never fires onTurnComplete, so without this the queue strands until a
  // tab-switch. Mirrors onStreamError's completion-hook call.
  tab.onTurnComplete?.(tab);
}

/** Steer the RUNNING turn: inject `text` into the live CLI stdin so the agent
 *  course-corrects at its next loop step (no restart, no lost work). Unlike
 *  the queue, this does NOT wait for the turn to finish. Falls back to the
 *  queue if the turn already ended (or the tab isn't streaming). Defaults to
 *  the focused-pane tab when `tabId` is omitted. */
export async function steer(
  store: AssistantStore,
  text: string,
  tabId?: string | null,
  attachments?: { mime: string; dataBase64: string }[],
): Promise<"steered" | "no_active_turn" | "queued"> {
  const trimmed = text.trim();
  if (!trimmed) return "queued";
  const tab = tabId ? store.tabFor(tabId) : store.activeTab;
  if (!tab) return "queued";
  const enqueue = () => {
    tab.queue = [...tab.queue, { id: crypto.randomUUID(), text: trimmed }];
  };
  // No active turn locally → nothing to steer; queue as a normal follow-up.
  if (!tab.streaming) {
    enqueue();
    return "queued";
  }
  const sid = tab.cliSessionId;
  try {
    const turnAttachments = attachments && attachments.length > 0 ? attachments : null;
    const res = await invoke<string>("assistant_steer", {
      sessionId: sid,
      text: trimmed,
      attachments: turnAttachments,
    });
    if (res === "no_active_turn") {
      // Turn ended between keypress and IPC — don't lose the message.
      enqueue();
      return "no_active_turn";
    }
    store.telemetry.event("turn.steer", { convoId: sid });
    // Make the steer VISIBLE: drop an inline marker into the streaming
    // assistant bubble at the point it landed, so the user sees their
    // interjection in the transcript instead of it vanishing into stdin.
    tab.messages = tab.messages.map((m) =>
      m.id === tab.streamingMsgId
        ? {
            ...m,
            blocks: [
              ...m.blocks,
              {
                type: "steer" as const,
                text: trimmed,
                at: Date.now(),
                ...(turnAttachments ? { attachments: turnAttachments } : {}),
              },
            ],
          }
        : m,
    );
    toast.push({
      severity: "info",
      title: "Steering",
      detail: trimmed.length > 60 ? trimmed.slice(0, 60) + "…" : trimmed,
      timeoutMs: 2500,
    });
    return "steered";
  } catch (e) {
    console.warn("assistant_steer failed", e);
    enqueue();
    return "queued";
  }
}

export function removeQueued(store: AssistantStore, id: string, tabId?: string) {
  const tab = tabId ? store.tabFor(tabId) : store.activeTab;
  if (tab) tab.queue = tab.queue.filter((q) => q.id !== id);
}

/** Client-side slash commands. Returns true if input was consumed. */
function runSlash(store: AssistantStore, input: string): boolean {
  const [cmd, ...rest] = input.slice(1).split(/\s+/);
  const arg = rest.join(" ").trim();
  switch (cmd.toLowerCase()) {
    case "clear":
      void store.clearConversation();
      return true;
    case "new":
      void store.newTab();
      return true;
    case "stop":
      void stop(store);
      return true;
    case "model": {
      const v = arg.toLowerCase();
      if (v === "sonnet" || v === "opus" || v === "haiku") {
        store.setModel(v);
        notify.ok(`Model switched to ${v}.`);
      } else {
        store.lastError = `Unknown model "${arg}". Use sonnet, opus, or haiku.`;
      }
      return true;
    }
    case "retry":
      void retryLast(store);
      return true;
    case "copy":
      void copyLastAssistant(store);
      return true;
    case "usage":
      store.ui.usageOpen = true;
      return true;
    case "cost":
      if (store.totalCostUsd != null) {
        const turns = store.messages.filter((m) => m.role === "assistant").length;
        notify.info("Session cost", { detail: `$${store.totalCostUsd.toFixed(4)} USD · ${turns} turn(s)` });
      } else {
        notify.info("No cost recorded yet — send a message first.");
      }
      return true;
    case "tools":
      store.lastNotice =
        "Tools available this turn: " +
        "Read / Write / Edit (files); Bash (shell, in workspace cwd); " +
        "Glob (filename patterns); Grep (content search); " +
        "WebFetch / WebSearch (open web); " +
        "TaskCreate / TaskUpdate (multi-step plans → Tasks dock; TodoWrite on older CLI builds). " +
        "Rift MCP: read_file / list_dir / grep (workspace-scoped helpers); " +
        "git_status / git_diff / git_log (and pull/commit/push when trust permits).";
      return true;
    case "diag": {
      const snap = store.telemetry.snapshot();
      const json = JSON.stringify(snap, null, 2);
      const sizeKb = Math.round(json.length / 102.4) / 10;
      navigator.clipboard
        .writeText(json)
        .then(() => {
          notify.ok("Telemetry copied", {
            detail: `${snap.turnCount} turn(s), ${snap.events.length} event(s), ${sizeKb}KB — paste into a code block`,
          });
        })
        .catch((e) => { store.lastError = `Clipboard write failed: ${String(e)}`; });
      return true;
    }
    case "diag-clear":
      store.telemetry.reset();
      notify.info("Telemetry buffer cleared — fresh capture starting now.");
      return true;
    case "stats": {
      // Inline-readable session summary — same data as /diag's `summary`
      // block but rendered as a short notice line so you can pattern-hunt
      // without dumping JSON. Cheap to fire repeatedly mid-session.
      const snap = store.telemetry.snapshot();
      const s = snap.summary;
      if (s.totalTurns === 0) {
        notify.info("No turns captured yet this session — send a message first.");
        return true;
      }
      const slowT = s.slowestTurn ? ` slowest turn #${s.slowestTurn.idx} ${(s.slowestTurn.durationMs / 1000).toFixed(1)}s` : "";
      const costT = s.costliestTurn ? ` costliest #${s.costliestTurn.idx} $${s.costliestTurn.costUsd.toFixed(3)}` : "";
      const slowTool = s.slowestTool ? ` slowest tool ${s.slowestTool.name} ${(s.slowestTool.durationMs / 1000).toFixed(1)}s` : "";
      const stale = s.staleCacheTurns > 0 ? ` ⚠ ${s.staleCacheTurns} stale-cache turn(s)` : "";
      const tps = s.outputTokensPerSec != null ? `, ${s.outputTokensPerSec} tok/s` : "";
      store.lastNotice =
        `${s.totalTurns} turn(s), $${s.totalCostUsd.toFixed(3)}, ` +
        `avg TTFP ${s.avgTtfpMs ?? "—"}ms, ${s.toolCallTotal} tool call(s)${tps}.` +
        slowT + costT + slowTool + stale;
      return true;
    }
    case "openincli": {
      const sid = store.currentCliSessionId;
      const ws = store.workspace.current;
      if (!sid) {
        store.lastError = "No active session yet — send a message first.";
        return true;
      }
      const safeWs = ws ? ws.replace(/'/g, "'\\''") : null;
      const cmd = safeWs ? `cd '${safeWs}' && claude --resume ${sid}` : `claude --resume ${sid}`;
      navigator.clipboard
        .writeText(cmd)
        .then(() => { notify.ok("Copied to clipboard", { detail: cmd, mono: true }); })
        .catch((e) => { store.lastError = `Clipboard write failed: ${String(e)}`; });
      return true;
    }
    case "help":
      store.lastNotice =
        "Slash commands: /new · /clear · /model · /retry · /copy · /stop · /tools · /cost · /usage · /openincli · /diag · /diag-clear · /help. " +
        "/clear wipes the current chat in place (old convo saved to History); /new opens a separate tab. /openincli copies a `claude --resume` command for the standalone CLI. " +
        "/diag exports session telemetry as JSON to clipboard. Up-arrow recalls previous prompts.";
      return true;
    default:
      return false;
  }
}

/** Re-send the most recent user prompt. Drops the prior user+assistant
 *  pair from the visible history so the retry looks like a redo, not a
 *  duplicate. Aborts an in-flight stream first. */
export async function retryLast(store: AssistantStore) {
  // #185: re-entrance guard so a fast double-click only strips one pair.
  if (store.retrying) return;
  store.retrying = true;
  try {
    const tab = store.activeTab;
    const last = tab?.promptHistory[tab.promptHistory.length - 1];
    if (!last || !tab) {
      store.lastError = "No previous prompt to retry.";
      return;
    }
    if (tab.streaming) {
      await stop(store);
    }
    // RR7: stop() awaits an IPC round-trip; the user may have switched tabs
    // during it. send() below routes through store.currentConvoId (the LIVE
    // active tab), so retrying now would fire into the wrong tab. Abort if the
    // captured tab is no longer active — the prompt stays in promptHistory.
    if (store.activeTab !== tab) return;
    // Strip the trailing assistant turn (if any) and the matching user turn
    // so the replayed history doesn't double-include the prompt.
    const msgs = tab.messages.slice();
    if (msgs[msgs.length - 1]?.role === "assistant") msgs.pop();
    if (msgs[msgs.length - 1]?.role === "user") msgs.pop();
    tab.messages = msgs;
    await send(store, last);
  } finally {
    store.retrying = false;
  }
}

/** Copy the latest assistant message's text content to the clipboard. */
export async function copyLastAssistant(store: AssistantStore) {
  const last = [...store.messages].reverse().find((m) => m.role === "assistant");
  if (!last) {
    store.lastError = "No assistant response to copy.";
    return;
  }
  const text = last.blocks
    .map((b) => (b.type === "text" ? b.text : ""))
    .join("")
    .trim();
  if (!text) {
    store.lastError = "Last response had no text content.";
    return;
  }
  try {
    await navigator.clipboard.writeText(text);
    notify.ok(`Copied ${text.length.toLocaleString()} chars to clipboard.`);
  } catch (e) {
    store.lastError = `Clipboard write failed: ${String(e)}`;
  }
}

/** Up-arrow recall. Returns the n-th-most-recent prompt, or null. */
export function recallPrompt(store: AssistantStore, offsetFromEnd: number): string | null {
  const idx = store.promptHistory.length - 1 - offsetFromEnd;
  return idx >= 0 ? store.promptHistory[idx] : null;
}
