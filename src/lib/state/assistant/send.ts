// M9 (per docs/design/assistant-svelte-split.md) — the send orchestrator
// lifted out of `src/lib/state/assistant.svelte.ts` as free fns over the
// AssistantStore ref: turn dispatch (send + slash commands), the per-tab
// outbound queue (drain + remove), stop, retry, copy and
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

export async function send(store: AssistantStore, prompt: string, targetConvoId?: string | null) {
  const trimmed = prompt.trim();
  // Explicit-target sends (queue drains, per-pane bubble actions) scope every
  // tab read/write below to THAT tab without retargeting pane focus. Omitted →
  // the focused-pane path, behavior unchanged (the resolved tab IS activeTab).
  const targetTab = targetConvoId ? store.tabFor(targetConvoId) : null;
  if (targetConvoId && !targetTab) return; // target retired mid-flight
  const liveTab = targetTab ?? store.activeTab;
  // Empty prompts are allowed when attachments are staged (paste-and-go).
  // Drop only if the prompt AND both attachment kinds are empty.
  if (!trimmed && (liveTab?.attachments.length ?? 0) === 0 && (liveTab?.textAttachments.length ?? 0) === 0) return;
  // Try-handle as a slash command first; if it matched, we're done. (A KNOWN
  // slash command never reaches the queue — send() consumes it before the
  // queue-on-busy branch — so drain-path re-entry always falls through here.)
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
  // Already streaming on this tab → queue instead of dropping. Snapshot the
  // composer attachments NOW: send() clears them right after enqueue, so a
  // queued message that didn't capture them would drain with its image/files
  // silently dropped (the user's "I queued an image and it vanished" bug).
  if (liveTab?.streaming) {
    const images = liveTab.attachments.map((a) => ({
      id: a.id, mime: a.mime, dataBase64: a.dataBase64, sizeBytes: a.sizeBytes,
    }));
    const textFiles = liveTab.textAttachments.map((t) => ({
      id: t.id, name: t.name, text: t.text, sizeBytes: t.sizeBytes, truncated: t.truncated,
    }));
    liveTab.queue = [...liveTab.queue, {
      id: crypto.randomUUID(),
      text: trimmed,
      ...(images.length ? { images } : {}),
      ...(textFiles.length ? { textFiles } : {}),
    }];
    // Clear the composer so the snapshotted attachments don't ALSO ride the
    // current turn / linger as a double-send. Mirrors send()'s own clear.
    liveTab.attachments = [];
    liveTab.textAttachments = [];
    return;
  }
  // Phase 2 (S72): the CLI owns conversation state now. First turn mints a
  // UUID and passes `--session-id`; subsequent turns pass `--resume`.
  // v0.4: newTab() mints currentConvoId up-front so the tab can render
  // before send() — gate isFirstTurn on convoCreatedAt instead so the very
  // first send still passes --session-id, not --resume.
  if (!targetTab && !store.currentConvoId) {
    store.currentConvoId = crypto.randomUUID();
  }
  const convoId = targetTab ? targetConvoId! : store.currentConvoId!;
  // #143: per-tab fields live on TabState now — ensureTab BEFORE touching
  // them so the writes don't no-op via the store's activeTab=null setter
  // path. ensureTab seeds cliSessionId to convoId for fresh tabs.
  const tab = store.ensureTab(convoId, convoId);
  // The model THIS tab's turns run on — per-tab override, else the global pick.
  const effModel = tab.modelOverride ?? store.model;
  const isFirstTurn = !tab.convoCreatedAt;
  if (!tab.cliSessionId) {
    tab.cliSessionId = convoId;
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
    tab.pinnedModel = effModel;
  }
  // A real turn — advance the sidebar's activity clock. Tab-switch auto-saves
  // deliberately don't touch this, so opening a chat no longer reshuffles.
  tab.lastActivityAt = Date.now();
  // v0.4: catches the raw newConversation→send path (slash /new) so tabs
  // never drift out of sync with the streaming convo.
  if (!store.openTabs.includes(convoId)) {
    store.openTabs = [...store.openTabs, convoId];
    store.persistTabs();
  }
  tab.beginTurn();
  store.lastNotice = null;
  // #184: clear stale error banner so it doesn't bleed into the new turn.
  tab.lastError = null;
  // No folder for THIS tab AND no scratch fallback → the backend runs the turn
  // in no-tools mode; say so up front. In local mode (scratch available) the
  // turn silently runs in `%LOCALAPPDATA%\Rift\local` with full tools — the
  // "Local" badge is the signal, so stay quiet.
  if (!store.effectiveRoot(tab) && !store.localScratchPath) {
    notify.warn("No folder open", {
      detail: "The assistant can't read or edit files this turn. Open one from the Workspace page.",
    });
  }
  // turn.rs swaps Fable to Opus silently once the limited run ends — warn ahead.
  if (!fableSunsetNoticed && effModel === "claude-fable-5"
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
  const attachBytes = tab.attachments.reduce((s, a) => s + a.sizeBytes, 0);
  // The send tier IS the user's persisted tier — no per-turn auto-scaling.
  // (#244's autoScaleEffort is retired: the default tier now maps to `medium`,
  // so a trivial greeting is already light, AND any per-turn effort change forces
  // a warm-pool cold respawn — `effort_level` is baked into the SpawnKey
  // (warm_pool.rs), so the "optimization" silently triggered the ~1.7s slow path
  // it was meant to avoid. Stable effort across turns = the warm child is reused.)
  const sendEffort = store.thinkingEffort;
  const turnRecord: TurnRecord = {
    ts: Date.now(),
    convoId,
    cliSessionId: tab.cliSessionId,
    isFirstTurn,
    model: effModel,
    effort: sendEffort,
    effortFlag: effortToFlag(sendEffort, effModel),
    promptLen: trimmed.length,
    promptPreview: trimmed.length > 120 ? trimmed.slice(0, 120) + "…" : trimmed,
    attachmentsCount: tab.attachments.length,
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
  const attachCount = tab.attachments.length;
  const textCount = tab.textAttachments.length;
  const markerParts: string[] = [];
  if (attachCount > 0) markerParts.push(`📎 ${attachCount} image${attachCount === 1 ? "" : "s"}`);
  if (textCount > 0) markerParts.push(`📄 ${textCount} file${textCount === 1 ? "" : "s"}`);
  const bubbleText = trimmed.length > 0 ? trimmed : markerParts.join(" · ");
  // Build the user message blocks — image blocks (one per attachment) first,
  // then the text block. Order matches the visual stack (thumbs above text)
  // in MessageBubble's user-side render path.
  const userBlocks: Block[] = [];
  for (const a of tab.attachments) {
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
  for (const t of tab.textAttachments) {
    userBlocks.push({ type: "text", text: `📄 ${t.name}${t.truncated ? " (truncated)" : ""}` });
  }
  userBlocks.push({ type: "text", text: bubbleText });
  tab.messages = [
    ...tab.messages,
    { id: crypto.randomUUID(), role: "user", blocks: userBlocks },
  ];
  // Snapshot the permission mode this turn runs with — TurnSummary's badge
  // reads the message copy so a later mode switch can't relabel history.
  const asst: ChatMessage = {
    id: crypto.randomUUID(), role: "assistant", blocks: [],
    permissionMode: store.permissionMode,
  };
  tab.messages = [...tab.messages, asst];
  tab.streamingMsgId = asst.id;
  // #146: asst placeholder is at the tail of messages; cache its index so
  // mutateStreaming can index-replace instead of scanning the full array.
  tab.streamingMsgIdx = tab.messages.length - 1;
  // Snapshot attachments for this turn + clear the composer so a fast retype
  // doesn't accidentally re-attach.
  const turnAttachments = tab.attachments.map((a) => ({
    mime: a.mime,
    dataBase64: a.dataBase64,
  }));
  // Inline text-file attachments into the prompt as fenced blocks before the
  // user's typed text. The backend pipes `prompt` to the CLI verbatim, so no
  // backend change is needed — the assistant simply sees the file contents.
  const textBlocks = tab.textAttachments
    .map((t) => `\`\`\`${t.name}\n${t.text}\n\`\`\``)
    .join("\n\n");
  const effectivePrompt = textBlocks
    ? (trimmed ? `${textBlocks}\n\n${trimmed}` : textBlocks)
    : trimmed;
  tab.attachments = [];
  tab.textAttachments = [];
  try {
    await invoke("assistant_send", {
      prompt: effectivePrompt,
      sessionId: tab.cliSessionId,
      isFirstTurn,
      model: effModel,
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
    // Restore the snapshotted attachments onto the DRAINING tab so send() picks
    // them up — send(store, text, convoId) below reads THIS tab's staged
    // attachments via its explicit target. Without this the queued image is
    // lost on drain.
    tab.attachments = next.images
      ? next.images.map((a) => ({ id: a.id, mime: a.mime, dataBase64: a.dataBase64, sizeBytes: a.sizeBytes }))
      : [];
    tab.textAttachments = next.textFiles
      ? next.textFiles.map((t) => ({ id: t.id, name: t.name, text: t.text, sizeBytes: t.sizeBytes, truncated: t.truncated }))
      : [];
    // Fire the bare sendImpl with an explicit target — it scopes every tab
    // read/write to the draining tab itself, so a drain on a sibling-pane tab
    // no longer yanks the user's pane focus (the old store.send() retarget
    // called setFocusedPane from this non-user-initiated path).
    send(store, next.text, capturedTabConvoId).catch(e => tab.onError(String(e)));
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
 *  duplicate. Aborts an in-flight stream first. `tabId` scopes the retry to
 *  that pane's tab (split-pane Retry button); omitted → focused tab. */
export async function retryLast(store: AssistantStore, tabId?: string | null) {
  // #185: re-entrance guard so a fast double-click only strips one pair.
  if (store.retrying) return;
  store.retrying = true;
  try {
    const tab = tabId ? store.tabFor(tabId) : store.activeTab;
    const last = tab?.promptHistory[tab.promptHistory.length - 1];
    if (!last || !tab) {
      store.lastError = "No previous prompt to retry.";
      return;
    }
    if (tab.streaming) {
      await stop(store, tabId);
    }
    // RR7: stop() awaits an IPC round-trip; the tab may have been closed or
    // replaced during it. Abort if the captured tab is gone — the prompt stays
    // in promptHistory. (With an explicit tabId, send() below targets the tab
    // directly, so mere focus movement no longer cancels the retry.)
    if ((tabId ? store.tabFor(tabId) : store.activeTab) !== tab) return;
    // Strip the trailing assistant turn (if any) and the matching user turn
    // so the replayed history doesn't double-include the prompt.
    const msgs = tab.messages.slice();
    if (msgs[msgs.length - 1]?.role === "assistant") msgs.pop();
    if (msgs[msgs.length - 1]?.role === "user") msgs.pop();
    tab.messages = msgs;
    await send(store, last, tabId ?? undefined);
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
