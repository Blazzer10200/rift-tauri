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
// stale-error clear, #185 retry re-entrance guard, and the Phase-C
// pendingCompactionSummary drain.

import { invoke } from "@tauri-apps/api/core";
import { accessibility } from "../accessibility.svelte";
import { toast } from "../toast.svelte";
import type { AssistantStore, TabState } from "../assistant.svelte";
import type { Block, ChatMessage, TurnRecord } from "./types";
import { effortToFlag, FABLE_SUNSET_MS } from "./helpers";

// One-shot per app session — the sunset warning shouldn't nag on every send.
let fableSunsetNoticed = false;

export async function send(store: AssistantStore, prompt: string) {
  const trimmed = prompt.trim();
  // Empty prompts are allowed when attachments are staged (paste-and-go).
  // Drop only if BOTH the prompt and attachments are empty.
  if (!trimmed && store.composerAttachments.length === 0) return;
  // Try-handle as a slash command first; if it matched, we're done.
  if (trimmed.startsWith("/") && runSlash(store, trimmed)) return;
  // Auth chokepoint — every send path funnels here (composer Enter/button,
  // queue drains, programmatic retries). A turn with no usable Claude session
  // dies as "claude exited with 1"; block it, re-probe (state may be stale),
  // and surface the reason. Slash commands above are local, so they still run.
  if (!(store.auth?.pill === "green" || store.auth?.pill === "yellow")) {
    store.lastNotice =
      store.auth?.summary ??
      "Claude isn't set up on this machine — open Settings to sign in or add an API key.";
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
  // path. ensureTab seeds cliSessionId to convoId for fresh tabs; compaction
  // remints it later without recreating the tab.
  const tab = store.ensureTab(store.currentConvoId, store.currentConvoId);
  const isFirstTurn = !tab.convoCreatedAt || tab.forceNextFirstTurn;
  tab.forceNextFirstTurn = false;
  if (!tab.cliSessionId) {
    tab.cliSessionId = store.currentConvoId;
  }
  if (!tab.convoCreatedAt) {
    tab.convoCreatedAt = Date.now();
    tab.convoTitle = null;
  }
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
  // No workspace → the backend silently runs the turn in no-tools mode; say so
  // up front instead of letting the user discover it from the reply.
  if (!store.workspace.current) {
    store.lastNotice = "No folder open — the assistant can't read or edit files this turn. Open one from the title bar.";
  }
  // turn.rs falls back to the Anthropic key when the active custom provider has
  // none saved — warn before that key is sent to a third-party endpoint.
  const prov = store.activeProvider;
  if (prov && !prov.hasKey && store.hasApiKey) {
    store.lastNotice = `Provider "${prov.name}" has no API key saved — your Anthropic key is being used for ${prov.baseUrl}. Add a provider key in Settings if that's not intended.`;
  }
  // turn.rs swaps Fable to Opus silently once the limited run ends — warn ahead.
  if (!fableSunsetNoticed && store.effectiveModel === "claude-fable-5"
      && Date.now() >= FABLE_SUNSET_MS - 7 * 86_400_000) {
    fableSunsetNoticed = true;
    store.lastNotice = Date.now() >= FABLE_SUNSET_MS
      ? "Fable's limited run has ended — this turn falls back to Opus 4.8."
      : "Heads up: Fable retires June 22 — chats fall back to Opus 4.8 after that.";
  }
  // Telemetry: build the turn record + attach to tab. TabState fills it as
  // envelopes arrive; finalized in onDone/onError.
  const attachBytes = store.composerAttachments.reduce((s, a) => s + a.sizeBytes, 0);
  const turnRecord: TurnRecord = {
    ts: Date.now(),
    convoId: store.currentConvoId,
    cliSessionId: tab.cliSessionId,
    isFirstTurn,
    model: store.effectiveModel,
    effort: store.thinkingEffort,
    effortFlag: effortToFlag(store.thinkingEffort, store.effectiveModel),
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
  store.telemetry.turns.push(turnRecord);
  tab.currentTurnRecord = turnRecord;
  // Track for /retry and Up-arrow recall. De-dupe consecutive identicals.
  if (tab.promptHistory[tab.promptHistory.length - 1] !== trimmed) {
    tab.promptHistory = [...tab.promptHistory, trimmed].slice(-50);
  }
  // User bubble text: when paste-and-go with no text, show an attachment
  // marker so the bubble isn't blank.
  const attachCount = store.composerAttachments.length;
  const bubbleText =
    trimmed.length > 0
      ? trimmed
      : attachCount === 1
      ? "📎 1 image"
      : `📎 ${attachCount} images`;
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
  store.composerAttachments = [];
  // Phase C: drain pendingCompactionSummary onto THIS turn only.
  // The new CLI session was minted at compactConversation() but is
  // empty — this summary is the model's only context for what came
  // before. Cleared immediately after dispatch; never persists across
  // turns. If the invoke itself fails the summary is lost (next send
  // starts cold) — acceptable since the boundary message stays in the
  // UI for the user to copy out if they need to manually re-seed.
  const priorSummary = tab.pendingCompactionSummary ?? null;
  tab.pendingCompactionSummary = null;
  try {
    await invoke("assistant_send", {
      prompt: trimmed,
      sessionId: tab.cliSessionId,
      isFirstTurn,
      model: store.effectiveModel,
      attachments: turnAttachments.length > 0 ? turnAttachments : null,
      dyslexiaMode: accessibility.dyslexiaMode,
      thinkingEffort: store.thinkingEffort,
      permissionMode: store.permissionMode,
      priorContextSummary: priorSummary,
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
  if (!tab || tab !== store.activeTab || tab.streaming || tab.queue.length === 0 || tab.lastError) return;
  // Rail-v2: steer-mode chips don't start turns — they ride the next one
  // (flushSteerChips injects them at its first stream line). Fire the first
  // queue-mode chip; if the rail is ALL steer chips there is no next turn to
  // ride, so degrade the head to a normal send — the queue can never strand.
  const next = tab.queue.find((q) => q.mode !== "steer") ?? tab.queue[0];
  tab.queue = tab.queue.filter((q) => q.id !== next.id);
  // #148: capture the active convo at pop time; if the user switches tabs OR
  // a new turn starts before the microtask fires, re-queue the head and bail.
  // The next completion or tab activation re-drains — never a silent strand.
  const capturedConvoId = store.currentConvoId;
  queueMicrotask(() => {
    if (store.currentConvoId !== capturedConvoId || tab.streaming) {
      if ([...store.tabs.values()].includes(tab)) {
        tab.queue = [next, ...tab.queue];
      }
      return;
    }
    send(store, next.text).catch(e => tab.onError(String(e)));
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
  tab.streaming = false;
  tab.streamingMsgId = null;
  tab.streamingMsgIdx = null;
  tab.deltaCount = 0;
  tab.envelopeTextBuffer = '';
  tab.seenToolUseIds.clear();
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
}

/** Steer the RUNNING turn: inject `text` into the live CLI stdin so the agent
 *  course-corrects at its next loop step (no restart, no lost work). Unlike
 *  the queue, this does NOT wait for the turn to finish. Falls back to the
 *  queue if the turn already ended (or the tab isn't streaming). Defaults to
 *  the focused-pane tab when `tabId` is omitted. */
export async function steer(store: AssistantStore, text: string, tabId?: string | null) {
  const trimmed = text.trim();
  if (!trimmed) return;
  const tab = tabId ? store.tabFor(tabId) : store.activeTab;
  if (!tab) return;
  const enqueue = () => {
    tab.queue = [...tab.queue, { id: crypto.randomUUID(), text: trimmed }];
  };
  // No active turn locally → nothing to steer; queue as a normal follow-up.
  if (!tab.streaming) {
    enqueue();
    return;
  }
  const sid = tab.cliSessionId;
  try {
    const res = await invoke<string>("assistant_steer", { sessionId: sid, text: trimmed });
    if (res === "no_active_turn") {
      // Turn ended between keypress and IPC — don't lose the message.
      enqueue();
      return;
    }
    store.telemetry.event("turn.steer", { convoId: sid });
    // Make the steer VISIBLE: drop an inline marker into the streaming
    // assistant bubble at the point it landed, so the user sees their
    // interjection in the transcript instead of it vanishing into stdin.
    tab.messages = tab.messages.map((m) =>
      m.id === tab.streamingMsgId
        ? { ...m, blocks: [...m.blocks, { type: "steer", text: trimmed, at: Date.now() }] }
        : m,
    );
    toast.push({
      severity: "info",
      title: "Steering",
      detail: trimmed.length > 60 ? trimmed.slice(0, 60) + "…" : trimmed,
      timeoutMs: 2500,
    });
  } catch (e) {
    console.warn("assistant_steer failed", e);
    enqueue();
  }
}

export function removeQueued(store: AssistantStore, id: string, tabId?: string) {
  const tab = tabId ? store.tabFor(tabId) : store.activeTab;
  if (tab) tab.queue = tab.queue.filter((q) => q.id !== id);
}

/** Rail-v2: inject every steer-mode chip into the just-started turn, in queue
 *  order. Called via TabState.onTurnStarted (first stream line — the backend
 *  steer registry is guaranteed live by then). Chips that miss the turn fall
 *  back to the queue inside steer(), losing their steer mark — correct, since
 *  the turn they were aimed at is gone. */
export function flushSteerChips(store: AssistantStore, tab: TabState | null) {
  if (!tab || !tab.streaming) return;
  const chips = tab.queue.filter((q) => q.mode === "steer");
  if (chips.length === 0) return;
  tab.queue = tab.queue.filter((q) => q.mode !== "steer");
  // Resolve this tab's convoId (Map key) so steer() targets THIS tab even if
  // the user switched panes — mirrors handleTurnComplete's reverse lookup.
  let tabId: string | null = null;
  for (const [cid, t] of store.tabs) {
    if (t === tab) { tabId = cid; break; }
  }
  void (async () => {
    for (const c of chips) {
      await steer(store, c.text, tabId);
    }
  })();
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
    case "history":
      store.ui.historyOpen = true;
      return true;
    case "stop":
      void stop(store);
      return true;
    case "model": {
      const v = arg.toLowerCase();
      if (v === "sonnet" || v === "opus" || v === "haiku") {
        store.setModel(v);
        store.lastNotice = `Model switched to ${v}.`;
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
      store.lastNotice =
        store.totalCostUsd != null
          ? `Session cost: $${store.totalCostUsd.toFixed(4)} USD across ${store.messages.filter((m) => m.role === "assistant").length} turn(s).`
          : "No cost recorded yet — send a message first.";
      return true;
    case "tools":
      store.lastNotice =
        "Tools available this turn: " +
        "Read / Write / Edit (files); Bash (shell, in workspace cwd); " +
        "Glob (filename patterns); Grep (content search); " +
        "WebFetch / WebSearch (open web); " +
        "TodoWrite (multi-step plans → Tasks dock). " +
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
          store.lastNotice = `Telemetry copied — ${snap.turnCount} turn(s), ${snap.events.length} event(s), ${sizeKb}KB. Paste into a code block here.`;
        })
        .catch((e) => { store.lastError = `Clipboard write failed: ${String(e)}`; });
      return true;
    }
    case "diag-clear":
      store.telemetry.reset();
      store.lastNotice = "Telemetry buffer cleared — fresh capture starting now.";
      return true;
    case "stats": {
      // Inline-readable session summary — same data as /diag's `summary`
      // block but rendered as a short notice line so you can pattern-hunt
      // without dumping JSON. Cheap to fire repeatedly mid-session.
      const snap = store.telemetry.snapshot();
      const s = snap.summary;
      if (s.totalTurns === 0) {
        store.lastNotice = "No turns captured yet this session — send a message first.";
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
    case "compact": {
      // Phase C: full compact action. arg becomes the focus hint.
      void store.compactConversation(arg || undefined);
      return true;
    }
    case "summarize": {
      // Compaction Phase B debug — dry-runs the summarize primitive
      // and renders the result as a notice. No state mutation; the
      // actual compaction flow lands in Phase C.
      store.lastNotice = "Summarizing… (cheap model, no state change)";
      void store.summarizeCurrentSession(arg || undefined).then((res) => {
        if (!res) return; // error already on lastError
        const tk = res.inputTokens + res.cacheReadTokens + res.cacheCreateTokens;
        store.lastNotice =
          `Summary ($${res.costUsd.toFixed(4)} · ${tk.toLocaleString()} in / ${res.outputTokens.toLocaleString()} out · ${res.model}):\n\n${res.summary}`;
      });
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
        .then(() => { store.lastNotice = `Copied to clipboard: ${cmd}`; })
        .catch((e) => { store.lastError = `Clipboard write failed: ${String(e)}`; });
      return true;
    }
    case "help":
      store.lastNotice =
        "Slash commands: /new · /clear · /history · /model · /retry · /copy · /stop · /tools · /cost · /usage · /compact · /summarize · /openincli · /diag · /diag-clear · /help. " +
        "/clear wipes the current chat in place (old convo saved to History); /new opens a separate tab. /openincli copies a `claude --resume` command for the standalone CLI. " +
        "/compact summarizes the current session + remints the CLI session id; the next turn carries the summary forward. " +
        "/summarize dry-runs Phase-B compaction summarize (no state change). " +
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
    store.lastNotice = `Copied ${text.length.toLocaleString()} chars to clipboard.`;
  } catch (e) {
    store.lastError = `Clipboard write failed: ${String(e)}`;
  }
}

/** Up-arrow recall. Returns the n-th-most-recent prompt, or null. */
export function recallPrompt(store: AssistantStore, offsetFromEnd: number): string | null {
  const idx = store.promptHistory.length - 1 - offsetFromEnd;
  return idx >= 0 ? store.promptHistory[idx] : null;
}
