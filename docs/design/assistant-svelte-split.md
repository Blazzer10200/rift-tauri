# Design — `assistant.svelte.ts` hot-file split

> Brief for the #20 hot-file split. Authoritative state in [src/lib/state/assistant.svelte.ts](../../src/lib/state/assistant.svelte.ts) — **2314 lines** as of 2026-05-28 (down from 3356L). Split lands one module per PR. This file enumerates concerns, lifts cleanly-detachable subsystems, and ranks extraction order by blast-radius.
>
> **STATUS 2026-06-09: COMPLETE — M0–M9 ALL SHIPPED.** M0–M7 (v0.4.31–v0.4.33); M8 `streaming.ts` + M9 `send.ts` landed 2026-06-09 (`b4ea421`, `ea514e8`) under the `assistant.playback.test.ts` regression net. `assistant.svelte.ts` 3356L → **1700L** — under the 2000-line hot-file threshold. Note: M8/M9 deviate from the M3-M7 structural-host-type pattern — they `import type { TabState/AssistantStore }` from the parent directly (type-only, erased, no runtime cycle) b/c their surface (~30 fields + self-referential IoC hooks) would drift as a shape copy. The residual file is the store shell (auth/settings/getters/init wiring) — bigger than the ≤500L guess below, but every enumerated concern is out. Backend follow-on: [assistant-mod-split.md](assistant-mod-split.md).

## Invariants (carry forward)

- **TabState ownership** (per CLAUDE.md): per-tab field MUST live on `TabState` w/ a getter on `AssistantStore`. Never back on the store. Compaction state, queue, draft, attachments, messages, streaming, agent spawns, askUser bindings are all already per-tab.
- **Active-tab routing**: `lastError`, `lastNotice` etc. route to the active tab when one is focused — preserve the getter/setter shape on whichever module ends up owning them.
- **No external import churn**: every concern that gets extracted must be re-exported (or remain on AssistantStore via delegation) at the same call-site shape. `import { assistant } from "$lib/state/assistant.svelte"` MUST keep working.
- **Persisted JSON contract**: `compactionHistory[]` is camelCase. `openTabs`, `panes`, `currentConvoId` shapes locked.
- **CompactionHistoryEntry / ConversationRecord / TurnRecord** type defs are load-bearing — pull them into shared `types.ts` not into a single concern module.

## Module boundaries (proposed)

Below: each candidate module names a concern, lists the line ranges in the current file, what it owns, what it calls outward, and whether it is TabState-bound or Store-bound. Line numbers are 2026-05-26 — re-grep on extract.

### M0 — `assistant/types.ts` (shared types only)

- **Range:** lines 13–215 (all `export type` + internal `type` declarations) + the TurnRecord at 267–337.
- **Owns:** `WorkspaceState`, `AuthStatus`, `ToolBlock`, `TextBlock`, `ThinkingBlock`, `BoundaryBlock`, `ImageBlock`, `Block`, `ChatMessage`, `ConversationMeta`, `SummarizeResult`, `CompactionHistoryEntry`, `ConversationRecord`, `ContentBlock`, `StreamDelta`, `StreamEvent`, `StreamEnvelope`, `RemoteLockEvt`, `RemoteShellEvt`, `ThinkingEffort`, `TurnRecord`, `PaneState`, `MAX_PANES`.
- **Touchpoints:** every other module imports from this. No code in here — types + the `MAX_PANES = 4` const + zero-cost discriminant helpers only.
- **Class:** neither — pure ambient types.
- **Blast radius:** zero — pure compile-time movement. Extract first.

### M1 — `assistant/helpers.ts` (pure functions, zero state)

- **Range:** 216–380 — `loadModel`, `saveModel`, `loadEffort`, `saveEffort`, `flattenToolResult`, `previewToolInput`, `messagesHaveContextSignals`, `effortToFlag`.
- **Owns:** localStorage I/O for model + effort prefs (pure side-effects on `globalThis`). Tool-input preview heuristics. Effort → CLI-flag mapper.
- **Touchpoints:** AssistantStore (init reads model + effort), TabState (none directly — store passes effortToFlag result into TurnRecord), MessageBubble (none).
- **Class:** neither — module-level functions.
- **Blast radius:** zero — already pure. Extract second.

### M2 — `assistant/telemetry.ts` (SessionTelemetry class)

- **Range:** 382–581 — `SessionTelemetry` class, `event()`, `snapshot()`, `reset()`.
- **Owns:** session-scoped event log, turn array, snapshot serializer. Non-reactive on purpose (per existing comment at 1660).
- **Touchpoints:** AssistantStore holds one instance at `this.telemetry`; TabState only mutates `currentTurnRecord` (a TurnRecord ref pushed onto telemetry.turns in send()). No reverse calls — telemetry knows nothing about Store/Tab.
- **Class:** Store-bound (one instance per store). The class definition is self-contained.
- **Blast radius:** zero — class is internal, no external imports of `SessionTelemetry` exist.

### M3 — `assistant/workspace.ts` (folder open/recent/files)

- **Range:** scattered — fields at 1683–1691 (`workspace`, `workspaceFiles`, `workspaceFilesLoadingFor`), methods at 2092–2157 (`refreshWorkspace`, `pickFolder`, `setRoot`, `clearRoot`, `removeRecentRoot`, `loadWorkspaceFiles`).
- **Owns:** workspace state object, files list, IPC calls `assistant_get_workspace_state`, `assistant_pick_folder`, `assistant_set_root`, `assistant_clear_root`, `assistant_remove_recent_root`, `assistant_list_workspace_files`.
- **Touchpoints:** EmptyState reads `assistant.workspace`. setRoot is called from picker. No TabState dependency.
- **Class:** Store-bound (one workspace per process). Pull into a `WorkspaceController` w/ field-by-field $state, exposed on AssistantStore via `get workspace() { return this.workspaceCtl.state }` so reactive readers don't break.
- **Blast radius:** LOW — ~70 lines, single domain, only EmptyState reads. Extract third.

### M4 — `assistant/attachments.ts` (image paste/staging)

- **Range:** TabState.attachments field 622, methods 2811–2836 (`addAttachment`, `removeAttachment`, `clearAttachments`).
- **Owns:** per-tab attachment array, 20 MiB cap enforcement.
- **Touchpoints:** AssistantPane drop handler + composer paste route through addAttachment. send() reads `composerAttachments` (active-tab accessor) + snapshots it for IPC.
- **Class:** **TabState-bound** (per-tab). Extract as a free-function module that takes `(tab, …args)` rather than a class — TabState fields stay on TabState, only the *logic* moves. Pattern: `export function addAttachment(tab, att) { … }`; AssistantStore method becomes a 3-line thunk routing to active tab.
- **Blast radius:** LOW — 30 lines, narrow API.

### M5 — `assistant/persistence.ts` (conversation save/load/list/delete)

- **Range:** field cluster 1731–1742 (`currentConvoId`, `conversations`, `openTabs`, `panes`, `focusedPaneIdx`, `draggingTabId`), methods 2158–2306 + 2596–2611. Plus `flushNow()` 2205, `scheduleSave()` and `buildSaveRecord()` (not shown in skeleton — re-grep). Plus `persistTabs()` (referenced at 2696).
- **Owns:** IPC `assistant_list_conversations`, `assistant_load_conversation`, `assistant_save_conversation`, `assistant_delete_conversation`, `assistant_rename_conversation`, debounced save timer (per-tab `tab.saveTimer`), tab/pane persistence (`assistant_persist_tabs`?), beforeunload flush.
- **Touchpoints:** Tabs lifecycle module (M6) calls into refreshConversations + load on open; Compaction (M7) calls scheduleSave after reminting. TabState owns `saveTimer` per-tab (#145 invariant) so scheduling stays on TabState; only the save plumbing extracts.
- **Class:** mixed — `saveTimer` STAYS on TabState (per invariant); free-function module operates on tab refs. `currentConvoId` + `conversations[]` STAY on Store (single source of "what convo is active" + "what convos exist").
- **Blast radius:** MEDIUM — ~250 lines, calls into beforeunload listener registered in init(). Extract after M3/M4.

### M6 — `assistant/tabs.ts` (tab lifecycle + panes)

- **Range:** fields 1733 (`openTabs`), 1738–1742 (`panes`, `focusedPaneIdx`, `draggingTabId`), methods 1437–1655 (`addPane`, `closePane`, `setFocusedPane`, `dropTabIntoPane`), 2405–2596 (`openTab`, `closeTab`, `newTab`, `reorderTabs`, `cycleTab`, `closeOtherTabs`, `closeAllTabs`, `closeTabsToRight`), 2522–2533 (`reorderTabs`). Scroll positions 1828–1841.
- **Owns:** which tabs are open, pane layout, drag-into-pane, MRU focus, scroll position cache (`tabScrolls` Map — re-grep).
- **Touchpoints:** TabBar reads `openTabs`, `panes`, `focusedPaneIdx`. ChatPaneWorkspace renders from `panes`. send() pushes new tab on `openTabs` if missing (line 2694–2697) — keep that thunk on Store.
- **Class:** Store-bound. `tabsByConvo: Map<string, TabState>` (re-grep — the actual TabState registry) stays on Store; the LIFECYCLE methods extract into a `TabsController` w/ Store back-ref so they can call `this.store.ensureTab(...)`.
- **Blast radius:** MEDIUM-HIGH — ~400 lines, many external readers (TabBar, ChatPaneWorkspace, AppShell keybinds). Extract after persistence is detached so the save calls don't pull persistence back in.

### M7 — `assistant/compaction.ts` (summarize + compact pipeline)

- **Range:** TabState fields 627 (`pendingCompactionSummary`), 636 (`forceNextFirstTurn`), 640 (`compactingNow`), 645 (`lastCompactionAt`), 649 (`compactionHistory`). Store fields 1671–1674 (`autoCompactThreshold`, `compactModel`). Methods 2964–3277 — `summarizeCurrentSession`, `compactConversation`, plus the auto-trigger $effect (re-grep — likely in init() at ~1846–1959).
- **Owns:** one-shot summarize IPC (`assistant_summarize`?), `compactConversation` orchestration, threshold check, cooldown enforcement, history append, boundary-block insertion, CLI session reminting.
- **Touchpoints:** send() drains `tab.pendingCompactionSummary` (line 2788–2789); persistence saves `compactionHistory`; AssistantStore.ctxPctFor reads `tab.sessionUsage` (M8 streaming) to decide auto-trigger.
- **Class:** mixed — TabState fields STAY on TabState; pipeline becomes free fns operating on tab refs + a Store reference for settings (threshold, model). The 5-min cooldown + first-turn force flag are critical invariants — guard w/ explicit tests when extracting.
- **Blast radius:** HIGH — load-bearing pipeline w/ several edge cases (#143 reminting, #145 saveTimer-per-tab, Phase C/D/E history). Extract LATE, only after M3-M6 reduce the surrounding file enough to navigate safely.

### M8 — `assistant/streaming.ts` (stream pump — biggest concern)

- **Range:** the whole TabState class 582–1314 minus the fields M4 (attachments) + M7 (compaction) + persistence saveTimer relocate. Methods: `constructor`, `resetUsage`, `beginTurn` (~723–917), `flushPendingText` (~918–1025), `tryBindAskUser` (~1026–1099), `onStream` (~1100–1230), `onDone` (~1231–1286), `onError` (~1287–1315).
- **Owns:** envelope/delta parsing, stream-event normalization, message mutation via `streamingMsgIdx` index-replace, tool_use → activity-label routing, thinking-block timing, askUser FIFO binding, raw line log, drain RAF batching, usage accumulator.
- **Touchpoints:** AssistantStore registers listeners in `init()` (lines 1846–1959) that fan-out to `tab.onStream/onDone/onError` via `tabByCliSession(session_id)`. The `onTodoApplied` / `onTurnComplete` / `shortToolLabel` hooks (701–710) are the inversion-of-control boundary — Store sets them at construction.
- **Class:** TabState stays a class b/c the state cluster is large + the hooks pattern relies on `this.onTodoApplied?.(this)` etc. But the METHODS extract into free fns `(tab, raw) => void`, and TabState's method bodies become 1-line thunks `onStream(raw) { handleStream(this, raw); }`. This keeps the external `tab.onStream(...)` call shape from `init()`'s listener while letting the 700-line bodies live in their own file.
- **Blast radius:** HIGHEST — the central nervous system. Extract LAST, only after every other concern is gone so the remaining file is just "TabState class shell + dispatch hooks". Mandate a full conversation playback test before merging.

### M9 — `assistant/send.ts` (send + slash + stop + retry)

- **Range:** 2658–2810 (`send`), plus `runSlash` (referenced at 2664 — re-grep), `stop` 2926–2951, `removeQueued` 2952, `retryLast` 3278, `copyLastAssistant` 3305, `recallPrompt` 3328.
- **Owns:** turn dispatch — IPC `assistant_send` arg construction (model, effort, attachments, priorContextSummary, dyslexiaMode), composer attachment snapshot+clear, TurnRecord seed, isFirstTurn gate, queue handling on busy tab.
- **Touchpoints:** depends on TabState (M8) for `beginTurn`, M4 for `composerAttachments`, M7 for `pendingCompactionSummary` drain, telemetry (M2) for `turns.push`, settings (model, effort) on Store.
- **Class:** Store-bound. Free-function module `send(store, prompt)` w/ Store thunk `send(p) { return doSend(this, p); }`.
- **Blast radius:** MEDIUM-HIGH — every UI surface that submits goes through `assistant.send`. Extract LATE (after M8) since it touches everything.

## Dependency graph

```
              ┌──────────────┐
              │  M0: types   │  (no deps; first out)
              └──────┬───────┘
                     │
        ┌────────────┼────────────┬─────────────┐
        ▼            ▼            ▼             ▼
  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │M1:      │  │M2:       │  │M3:       │  │M4:       │
  │helpers  │  │telemetry │  │workspace │  │attach    │
  └────┬────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
       │            │              │              │
       └────────────┴──────┬───────┴──────────────┘
                           ▼
                  ┌──────────────────┐
                  │M5: persistence   │  (uses types + tab refs)
                  └────────┬─────────┘
                           ▼
                  ┌──────────────────┐
                  │M6: tabs+panes    │  (calls M5 on open/close)
                  └────────┬─────────┘
                           ▼
                  ┌──────────────────┐
                  │M7: compaction    │  (reads M8 ctx; writes M5)
                  └────────┬─────────┘
                           ▼
                  ┌──────────────────┐
                  │M8: streaming     │  (the nervous system; LAST)
                  └────────┬─────────┘
                           ▼
                  ┌──────────────────┐
                  │M9: send          │  (orchestrates M2/M4/M7/M8)
                  └──────────────────┘
```

## Extraction order (blast-radius-ascending)

1. **M0** types → trivial, pure movement. ✅ DONE
2. **M1** helpers → already pure, no consumers outside this file. ✅ DONE
3. **M2** telemetry → self-contained class, single store ref. ✅ DONE
4. **M3** workspace → narrow IPC, one external reader. ✅ DONE
5. **M4** attachments → 30 lines, one external reader. ✅ DONE
6. **M5** persistence → save/load/list path, debounced; needs M0 + M4 first to compile cleanly. ✅ DONE
7. **M6** tabs+panes → biggest UI-facing surface; needs M5 in place so close/save can route. ✅ DONE
8. **M7** compaction → tight contract w/ M8 ctx readings + M5 save; do AFTER both. ✅ DONE
9. **M8** streaming → the rest of TabState; extract method bodies as free fns, leave TabState class as thin shell. ✅ DONE (2026-06-09)
10. **M9** send → orchestrator across M2/M4/M7/M8; last. ✅ DONE (2026-06-09)

At each step, the size reduction target is ~10–15% of file. After M9 the residual `assistant.svelte.ts` should be ≤500 lines containing only:
- AssistantStore class shell (auth + settings + the `getXxx` getters that delegate to controllers).
- The `export const assistant = new AssistantStore()` singleton.
- Hook wiring (`init()` listener registration that calls each module's setup fn).

## Hard rules for the executor

- **Per-tab fields → stay on TabState class declaration.** Only methods move. Free fns receive `tab` as first arg.
- **Public API frozen.** `assistant.send(...)`, `assistant.workspace.recent`, `assistant.openTabs` — every existing call site keeps working w/o churn.
- **One module per PR.** Don't batch M3 + M4 into one commit; the diff stops being reviewable past ~150 lines moved.
- **Verify per module:** `npm run check` clean + manually exercise the relevant surface (open/close tab, paste image, send, compact) via the dev server before the next extraction.
- **CompactionHistoryEntry shape locked.** Persisted JSON depends on the field names (`compactionHistory` invariant in CLAUDE.md). Don't rename during extraction.
- **TabState constructor signature locked.** AssistantStore.ensureTab passes `(cliSessionId)`. Any new param breaks the construction path in `loadConversation`.

## Follow-on (after this lands)

`auto_sync.rs` (2207L) is the next split candidate per #20. Its brief should follow once `assistant.svelte.ts` is below 2000 lines and the pattern (per-concern free-fn modules around a thin shell class) is validated here. Precedent for Rust side: `sync/auto_sync/flush.rs` + `watch.rs` already extracted — auto_sync.rs split should follow that pattern, NOT this TS one.
