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
import { notify } from "../toast.svelte";
import type { AssistantStore, TabState } from "../assistant.svelte";
import type { Block, ChatMessage, ModelSel, QueueItem, TurnRecord } from "./types";
import { effortToFlag, fableAvailable, fastEligible, haikuAvailable, FABLE_SUNSET_MS } from "./helpers";
import { mcpPanel } from "../mcp-panel.svelte";
import { appendSteerBlock, finalizeInflightBlocks, removeSteerBlock } from "./streaming";

// One-shot per app session — the sunset warning shouldn't nag on every send.
let fableSunsetNoticed = false;

/** Attachments threaded explicitly through a send — queue drains pass the
 *  drained item's snapshot here instead of restoring it onto tab state, so a
 *  concurrent composer send can never read (or clobber) another message's files. */
export type SendPayload = {
  images: NonNullable<QueueItem["images"]>;
  textFiles: NonNullable<QueueItem["textFiles"]>;
};

export async function send(
  store: AssistantStore,
  prompt: string,
  targetConvoId?: string | null,
  opts?: { payload?: SendPayload; requeueFront?: boolean },
) {
  const trimmed = prompt.trim();
  // Explicit-target sends (queue drains, per-pane bubble actions) scope every
  // tab read/write below to THAT tab without retargeting pane focus. Omitted →
  // the focused-pane path, behavior unchanged (the resolved tab IS activeTab).
  const targetTab = targetConvoId ? store.tabFor(targetConvoId) : null;
  if (targetConvoId && !targetTab) return; // target retired mid-flight
  const liveTab = targetTab ?? store.activeTab;
  // Attachment source: explicit payload (queue drain / gate re-entry) or the
  // tab's composer staging. With a payload the composer is never read NOR
  // cleared — its staged files belong to whatever the user is typing next.
  const payload = opts?.payload ?? null;
  // Empty prompts are allowed when attachments are staged (paste-and-go).
  // Drop only if the prompt AND both attachment kinds are empty.
  if (!trimmed && (payload
    ? payload.images.length === 0 && payload.textFiles.length === 0
    : (liveTab?.attachments.length ?? 0) === 0 && (liveTab?.textAttachments.length ?? 0) === 0)) return;
  // Try-handle as a slash command first; if it matched, we're done. (A KNOWN
  // slash command never reaches the queue — send() consumes it before the
  // queue-on-busy branch — so drain-path re-entry always falls through here.)
  if (trimmed.startsWith("/") && runSlash(store, trimmed, liveTab ?? null)) return;
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
  // Already streaming on this tab → STEER first (inject into the live turn so
  // Claude reads it after the current tool call — same turn, same context),
  // queue only as the fallback. Snapshot the composer attachments NOW: send()
  // clears them right after, so a message that didn't capture them would
  // deliver with its image/files silently dropped (the user's "I queued an
  // image and it vanished" bug).
  if (liveTab?.streaming) {
    const images = payload ? payload.images : liveTab.attachments.map((a) => ({
      id: a.id, mime: a.mime, dataBase64: a.dataBase64, sizeBytes: a.sizeBytes,
    }));
    const textFiles = payload ? payload.textFiles : liveTab.textAttachments.map((t) => ({
      id: t.id, name: t.name, text: t.text, sizeBytes: t.sizeBytes, truncated: t.truncated,
    }));
    const item: QueueItem = {
      id: crypto.randomUUID(),
      text: trimmed,
      ...(images.length ? { images } : {}),
      ...(textFiles.length ? { textFiles } : {}),
    };
    if (!payload) {
      // Clear the composer so the snapshotted attachments don't ALSO ride the
      // current turn / linger as a double-send. Mirrors send()'s own clear.
      liveTab.attachments = [];
      liveTab.textAttachments = [];
    }
    await steerOrQueue(store, liveTab, item, opts?.requeueFront === true);
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
  // Track the model this chat's turns run on. First turn (or a legacy tab with
  // no capture yet) seeds it; a later differing pick is a deliberate mid-chat
  // switch — the backend honors it and re-pins (turn.rs), and the transcript
  // marker is inserted just before this turn's bubbles below.
  if (!tab.pinnedModel) {
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
  // Resolve this turn's attachments BEFORE the first await — explicit payload
  // (queue drain) or a composer snapshot, composer cleared at snapshot time.
  // Reading tab.attachments after the gate below let a send parked on it
  // consume attachments that belonged to a different message.
  const turnImages = payload ? payload.images : tab.attachments.map((a) => ({
    id: a.id, mime: a.mime, dataBase64: a.dataBase64, sizeBytes: a.sizeBytes,
  }));
  const turnTextFiles = payload ? payload.textFiles : tab.textAttachments.map((t) => ({
    id: t.id, name: t.name, text: t.text, sizeBytes: t.sizeBytes, truncated: t.truncated,
  }));
  if (!payload) {
    tab.attachments = [];
    tab.textAttachments = [];
  }
  // Post-stop gate (see TabState.staleTerminalUntil): starting the next turn
  // before the stopped turn's terminal event lands would let that stale event
  // finalize the new turn, and the new backend send would consume the old
  // turn's stop marker (remapping its stale DONE to a spurious ERROR).
  while (tab.staleTerminalUntil > Date.now()) {
    await new Promise((r) => setTimeout(r, 50));
    // Another send won the gate while we waited — re-enter from the top so
    // this message takes the queue path (attachments ride the payload; the
    // composer was already snapshotted + cleared above). Re-enter with the
    // RESOLVED `convoId`, NOT the original `targetConvoId`: a no-tabId retry
    // (retryLast / /retry) has targetConvoId=undefined, so re-passing it would
    // re-resolve `liveTab = store.activeTab` — and if the user switched panes
    // during the 50ms wait, the retried message would land in the wrong tab.
    // `convoId` pins it to the tab this gate was actually waiting on.
    if (tab.streaming) {
      return send(store, prompt, convoId, {
        payload: { images: turnImages, textFiles: turnTextFiles },
        requeueFront: opts?.requeueFront,
      });
    }
  }
  // #80: mint this turn's epoch — the backend stamps it on every event the
  // turn emits, so the listeners can tell a stale event from the live turn.
  tab.turnEpoch += 1;
  tab.beginTurn();
  // Manual /compact rides to the CLI as a normal turn (runSlash falls through)
  // but produces no tools/text until the boundary — flag it so the live turn
  // says "Compacting conversation…" instead of a generic "Working…".
  tab.compactingTurn = /^\/compact\b/i.test(trimmed);
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
  const attachBytes = turnImages.reduce((s, a) => s + a.sizeBytes, 0);
  // The send tier IS the user's persisted tier — no per-turn auto-scaling.
  // (#244's autoScaleEffort is retired: the default tier now maps to `medium`,
  // so a trivial greeting is already light, AND any per-turn effort change forces
  // a warm-pool cold respawn — `effort_level` is baked into the SpawnKey
  // (warm_pool.rs), so the "optimization" silently triggered the ~1.7s slow path
  // it was meant to avoid. Stable effort across turns = the warm child is reused.)
  const sendEffort = store.effortFor(tab);
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
    attachmentsCount: turnImages.length,
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
    thinkingMsBeforeFirstPaint: null,
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
  const attachCount = turnImages.length;
  const textCount = turnTextFiles.length;
  const markerParts: string[] = [];
  if (attachCount > 0) markerParts.push(`📎 ${attachCount} image${attachCount === 1 ? "" : "s"}`);
  if (textCount > 0) markerParts.push(`📄 ${textCount} file${textCount === 1 ? "" : "s"}`);
  const bubbleText = trimmed.length > 0 ? trimmed : markerParts.join(" · ");
  // Build the user message blocks — image blocks (one per attachment) first,
  // then the text block. Order matches the visual stack (thumbs above text)
  // in MessageBubble's user-side render path.
  const userBlocks: Block[] = [];
  for (const a of turnImages) {
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
  for (const t of turnTextFiles) {
    userBlocks.push({ type: "text", text: `📄 ${t.name}${t.truncated ? " (truncated)" : ""}` });
  }
  userBlocks.push({ type: "text", text: bubbleText });
  // Mid-chat model switch → drop an inline marker above this turn's bubbles so
  // the change is visible where it happened, then track the new running model
  // (which also clears the picker's divergence note — the switch took effect).
  if (tab.pinnedModel && tab.pinnedModel !== effModel) {
    tab.messages = [...tab.messages, {
      id: crypto.randomUUID(),
      role: "system",
      blocks: [{ type: "modelSwitch", from: tab.pinnedModel, to: effModel, at: Date.now() }],
      ts: Date.now(),
    }];
    tab.pinnedModel = effModel;
  }
  tab.messages = [
    ...tab.messages,
    { id: crypto.randomUUID(), role: "user", blocks: userBlocks, ts: Date.now() },
  ];
  // Persist the user message NOW — saves otherwise only happen at turn-complete,
  // so anything that kills the app mid-turn (update apply, crash) lost the
  // just-typed prompt (v0.131.0 incident). Fired before the assistant
  // placeholder joins so the record never ends on an empty bubble.
  store.scheduleSave(true, convoId);
  // Snapshot the permission mode this turn runs with — TurnSummary's badge
  // reads the message copy so a later mode switch can't relabel history.
  const asst: ChatMessage = {
    id: crypto.randomUUID(), role: "assistant", blocks: [],
    permissionMode: store.permissionMode,
    ts: Date.now(),
  };
  tab.messages = [...tab.messages, asst];
  tab.streamingMsgId = asst.id;
  // #146: asst placeholder is at the tail of messages; cache its index so
  // mutateStreaming can index-replace instead of scanning the full array.
  tab.streamingMsgIdx = tab.messages.length - 1;
  // Wire shape for the backend: mime + data only (ids stay FE-side). The
  // composer was already snapshotted + cleared before the post-stop gate.
  const turnAttachments = turnImages.map((a) => ({
    mime: a.mime,
    dataBase64: a.dataBase64,
  }));
  // Inline text-file attachments into the prompt as fenced blocks before the
  // user's typed text. The backend pipes `prompt` to the CLI verbatim, so no
  // backend change is needed — the assistant simply sees the file contents.
  const textBlocks = turnTextFiles
    .map((t) => `\`\`\`${t.name}\n${t.text}\n\`\`\``)
    .join("\n\n");
  const effectivePrompt = textBlocks
    ? (trimmed ? `${textBlocks}\n\n${trimmed}` : textBlocks)
    : trimmed;
  try {
    await invoke("assistant_send", {
      prompt: effectivePrompt,
      sessionId: tab.cliSessionId,
      turnEpoch: tab.turnEpoch,
      isFirstTurn,
      model: effModel,
      attachments: turnAttachments.length > 0 ? turnAttachments : null,
      dyslexiaMode: accessibility.dyslexiaMode,
      thinkingEffort: sendEffort,
      thinkingEnabled: store.thinkingOnFor(tab),
      permissionMode: store.permissionMode,
      // Sent pre-gated by model family so an ineligible model never carries a
      // stale global `on` into the SpawnKey; the backend re-gates by CLI
      // version (caps.fast_mode) on top.
      fastMode: store.fastMode && fastEligible(effModel),
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

/** Steer-first delivery for a send that arrived while `tab` was streaming.
 *  Renders the message as an inline steer marker in the live assistant bubble
 *  (optimistic), then asks the backend to inject it into the in-flight turn's
 *  stdin. Ok = Claude reads it after the current tool call (same turn). Any
 *  failure — turn ended in the race window, dead child, idle session — removes
 *  the marker and parks the message in the ordinary queue, so nothing is ever
 *  silently lost. Wire prompt mirrors send(): fenced text-file blocks above
 *  the typed text; images ride the attachments array. */
async function steerOrQueue(
  store: AssistantStore,
  tab: TabState,
  item: QueueItem,
  requeueFront: boolean,
) {
  const enqueue = () => {
    // A drained head that lost the post-stop gate race re-parks at the FRONT —
    // it was next in line; appending would shuffle it behind newer messages.
    tab.queue = requeueFront ? [item, ...tab.queue] : [...tab.queue, item];
  };
  const sid = tab.cliSessionId;
  if (!sid) {
    enqueue();
    return;
  }
  const imgCount = item.images?.length ?? 0;
  const fileCount = item.textFiles?.length ?? 0;
  // Marker text mirrors the user-bubble fallback: attachment counts when the
  // typed text is empty (paste-and-go steer).
  const markerParts: string[] = [];
  if (imgCount > 0) markerParts.push(`📎 ${imgCount} image${imgCount === 1 ? "" : "s"}`);
  if (fileCount > 0) markerParts.push(`📄 ${fileCount} file${fileCount === 1 ? "" : "s"}`);
  const markerText = item.text.length > 0 ? item.text : markerParts.join(" · ");
  const blockId = appendSteerBlock(
    tab,
    markerText,
    (item.images ?? []).map((a) => ({ mime: a.mime, dataBase64: a.dataBase64 })),
    fileCount,
  );
  if (!blockId) {
    // Nothing streaming to attach to (terminal raced us) — plain queue path.
    enqueue();
    return;
  }
  // Up-arrow recall should see steered prompts too — same dedupe as send().
  if (item.text && tab.promptHistory[tab.promptHistory.length - 1] !== item.text) {
    tab.promptHistory = [...tab.promptHistory, item.text].slice(-50);
  }
  const textBlocks = (item.textFiles ?? [])
    .map((t) => `\`\`\`${t.name}\n${t.text}\n\`\`\``)
    .join("\n\n");
  const effectivePrompt = textBlocks
    ? (item.text ? `${textBlocks}\n\n${item.text}` : textBlocks)
    : item.text;
  try {
    await invoke("assistant_steer", {
      sessionId: sid,
      prompt: effectivePrompt,
      attachments: imgCount > 0
        ? item.images!.map((a) => ({ mime: a.mime, dataBase64: a.dataBase64 }))
        : null,
    });
    store.telemetry.event("turn.steer", { convoId: sid, promptLen: item.text.length });
  } catch {
    // Not delivered — fall back to the queue as its own turn. If the turn
    // ended while we waited, kick the drain so the message fires now instead
    // of stranding until the next tab activation.
    removeSteerBlock(tab, blockId);
    enqueue();
    if (!tab.streaming) drainQueue(store, tab);
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
    // Fire the bare sendImpl with an explicit target — it scopes every tab
    // read/write to the draining tab itself, so a drain on a sibling-pane tab
    // no longer yanks the user's pane focus (the old store.send() retarget
    // called setFocusedPane from this non-user-initiated path). The item's
    // attachments ride the explicit payload — never staged on tab state, where
    // a concurrent composer send could consume (or clobber) them.
    send(store, next.text, capturedTabConvoId, {
      payload: { images: next.images ?? [], textFiles: next.textFiles ?? [] },
      requeueFront: true,
    }).catch(e => tab.onError(String(e)));
  });
}

/** Stop a tab's in-flight stream. Defaults to the focused-pane tab when
 *  `tabId` is omitted. Pre-clears the tab's streaming flag synchronously
 *  so any late `done` event for this session is idempotent. Other tabs
 *  keep streaming. */
export async function stop(store: AssistantStore, tabId?: string | null) {
  const tab = tabId ? store.tabFor(tabId) : store.activeTab;
  if (!tab) return;
  // Normal case: a live turn. But a wedged turn can leave the tab NOT streaming
  // while a tool block is still pending / the activity label still spins / the
  // backend PID is still tracked — the "can't stop the fake Editing-file
  // spinner" prod wedge. So don't inertly early-return on !streaming: only bail
  // when the tab is genuinely idle AND clean. Otherwise fall through to a
  // hard-clear (finalizeInflightBlocks below sweeps any pending block).
  const hasLingeringWork =
    tab.streamingMsgId != null ||
    tab.activity.currentLabel != null ||
    tab.agentSpawns.some((a) => a.completedAt == null) ||
    tab.shellRows.length > 0;
  if (!tab.streaming && !hasLingeringWork) return;
  const sid = tab.cliSessionId;
  // #179: flush pacer-buffered text into the message BEFORE clearing
  // streamingMsgId — otherwise mutateStreaming's early-return drops it.
  tab.flushPendingText();
  // RR10/#64-fe: settle in-flight thinking/tool blocks before clearing
  // streamingMsgId, else a user Stop mid-reasoning persists a stuck
  // status:"active" thinking chip + status:"pending" tool chips in history.
  finalizeInflightBlocks(tab);
  tab.streaming = false;
  // Arm the post-stop gate: the killed turn's terminal done/error is still in
  // flight from the backend. send() defers the next turn on this tab until the
  // handler consumes it (clearing this) or the deadline passes — else that
  // stale terminal would finalize the NEXT turn (same session id on the wire).
  tab.staleTerminalUntil = Date.now() + 2000;
  tab.streamingMsgId = null;
  tab.streamingMsgIdx = null;
  tab.lastTurnDoneAt = null; // user-stopped — a trailing continuation must not merge into it
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
function runSlash(store: AssistantStore, input: string, tab: TabState | null): boolean {
  const [cmd, ...rest] = input.slice(1).split(/\s+/);
  const arg = rest.join(" ").trim();
  switch (cmd.toLowerCase()) {
    case "clear":
      void store.clearConversation();
      return true;
    case "compact":
      // NOT consumed on the happy path (unique among cases): the literal
      // "/compact [instructions]" rides to the CLI as a normal turn — the CLI
      // compacts natively and its compact_boundary event renders the pill
      // (streaming.ts::appendCliCompaction). Only the meaningless states stop here.
      if (!tab?.convoCreatedAt) {
        notify.info("Nothing to compact yet — send a message first.");
        return true;
      }
      if (tab.streaming) {
        notify.info("A turn is still running — wait for it to finish (or /stop) before compacting.");
        return true;
      }
      return false;
    case "new":
      void store.newTab();
      return true;
    case "stop":
      void stop(store);
      return true;
    case "model": {
      const v = arg.toLowerCase();
      // Availability-gated like the picker: haiku only while enabled, fable
      // maps to its full id. A disabled model reads as unknown — honest copy.
      const id: ModelSel | null =
        v === "sonnet" || v === "opus" ? v
        : v === "fable" && fableAvailable() ? "claude-fable-5"
        : v === "haiku" && haikuAvailable() ? "haiku"
        : null;
      if (id) {
        store.setModel(id);
        notify.ok(`Model switched to ${v}.`);
      } else {
        const names = ["sonnet", "opus", ...(fableAvailable() ? ["fable"] : []), ...(haikuAvailable() ? ["haiku"] : [])];
        store.lastError = `Unknown model "${arg}". Use ${names.join(", ")}.`;
      }
      return true;
    }
    case "retry":
      void retryLast(store);
      return true;
    case "copy":
      void copyLastAssistant(store);
      return true;
    case "usage": {
      const t = tab ?? store.activeTab;
      if (t) t.usageOpen = "full";
      return true;
    }
    case "cost":
      if (store.totalCostUsd != null) {
        const turns = store.messages.filter((m) => m.role === "assistant").length;
        notify.info("Session cost", { detail: `$${store.totalCostUsd.toFixed(4)} USD · ${turns} turn(s)` });
      } else {
        notify.info("No cost recorded yet — send a message first.");
      }
      return true;
    case "mcp": {
      // Centered dialog (McpServersDialog), not a notice/toast — owner call
      // 2026-07-10. Data = two truth layers merged (mcpStatus.ts): `claude
      // mcp list` via the backend — the user's real harness config (user
      // scope + project .mcp.json), health-checked now, works before any
      // turn — overlaid with this chat's init-frame statuses (headless auth
      // ≠ terminal auth, so the session's view wins per name).
      mcpPanel.show();
      void mcpPanel.refresh(store.workspace.current, store.activeTab?.mcpServers ?? null);
      return true;
    }
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
        "Slash commands: /new · /clear · /compact · /model · /retry · /copy · /stop · /tools · /mcp · /cost · /usage · /openincli · /diag · /diag-clear · /help. " +
        "/clear wipes the current chat in place (old convo saved to History); /new opens a separate tab. /compact summarizes older turns to free context — the thread keeps going. /openincli copies a `claude --resume` command for the standalone CLI. " +
        "/diag exports session telemetry as JSON to clipboard. Up-arrow recalls previous prompts. " +
        "Your own Claude Code skills and commands (from ~/.claude and the project's .claude folder) show up in the / menu too — they run through the CLI.";
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
