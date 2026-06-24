# Review Wave-2 Backlog (cont.174, 2026-06-23)

> Confirmed findings from the 507-agent frontend review workflow (161 raised → 73 adversarially-verified confirmed). Two independent parses were run; **this file uses the higher-detail parse**. (Raw agent outputs were ephemeral and have been pruned — this curated backlog is the surviving record.)
>
> **⚠ RE-TRIAGED 2026-06-24 (cont.189) — Tier A + B + C1–C22 only.** Those ranges were re-grepped against current code. **The cont.174 fix-pass + later work landed all of Tier A + nearly all of B + C1–C22.** Of that scope only **C5 + C7** stay open (judgment calls — see bottom). The 4 still-live ones (A1, C2, C3, C20) were fixed + committed `b0ec8b6`. **C23–C35 + Tier D were NOT re-triaged this pass — status unknown, re-grep before acting.** Do NOT re-investigate the FIXED set below.
>
> **VERIFIED FIXED (do NOT re-fix):**
> - **A1** `parseAskUserResult` multi-select split → `b0ec8b6` (US `\x1F` delimiter, backend+frontend lockstep, +3 vitest).
> - **A2** `diffArrays` quadratic Myers diff → FIXED: `diffCountsCached` (streamModel.ts:165,230) memoizes per tool id, runs once not per-frame. (Backlog's "NOT covered" note was stale.)
> - **A4/A5** web-context injection (`WebBrowserPage.svelte:164-165`): `i` flag + body strips `[Page context:` + `[End page context]`. FIXED.
> - **B1** init double-register → `initGen` cancellation (assistant.svelte.ts:851,959,994). **B2** enhance spawn leak → RR9 unconditional `enhanceSeq` bump (Composer.svelte:51-60). **B5–B11** all FIXED (stt `polishGuard`/`cancelPolish`, updates `repairing` flag, settings validate).
> - **C1** dropTab saveTimer clear (persistence.ts). **C4** telemetry caps (500/2000). **C6** rawLineLog capped. **C8** QueueRail prevSteerCount reset. **C9** EnhanceBar `{#key enhancedPreview}`. **C10** slashOpen Enter swallowed. **C11** mentionResults `$derived.by` (single scan). **C16** StreamAskUser `untrack`. **C17** parseAskUserResult greedy first-group. **C18** liveTokens no `void now`. **C19** hoverTimer teardown `$effect`. **C21** AssistantPane FLIP reset. **C22** onWinDrop captures targetConvoId pre-await.
> - **C2** startLogin uncancellable loop, **C3** loadWorkspaceFiles guard race, **C20** AssistantWelcome concurrent load → all `b0ec8b6`.
>
> **Discipline:** findings are HINTS, not facts. The two parses DISAGREE on line numbers + some detail (e.g. dropTab cited as both `:555` and `persistence.ts:173`) — **re-grep each by symbol before editing.** Wave-1 had 7 false-positives + a split verdict. The fix-pass already shifted some lines. Apply in green-verified batches; re-verify line numbers fresh.

## Tier A — `final: high` (3/3, do first; these are the real high-severity set)

| # | Title | File (re-grep) | Fix |
|---|---|---|---|
| A1 | `parseAskUserResult` splits multi-select on literal `", "` — phantom sub-answers when a label contains `", "` (data integrity) | `stream/streamModel.ts:401` | Use a distinct separator (`\x1F`) or JSON-encode the answer array backend-side |
| A2 | `diffArrays` re-runs full Myers diff every stream chunk for all completed edits — O(edits×deltas) quadratic | `stream/streamModel.ts:183` (`messageToTurn`) | Memoize `diffArrays` per ToolBlock `id+status`; recompute only on status change. (NOT the same as the shipped `diffCountsCached`) |
| A3 | AssistantWelcome loads files/branch from focused-pane root — secondary pane shows PRIMARY pane's project | `assistant/AssistantWelcome.svelte:42` | Pass pane-local root explicitly / gate on `workspaceFilesLoadingFor === paneRoot` |
| A4 | `addToChat`: `[Page context:` header not stripped from body — nested/duplicate context block → prompt injection in user turn | `webview/WebBrowserPage.svelte:163` | Also replace `[Page context:` occurrences in body before append |
| A5 | Prompt-injection via case-variant sentinel — `/\[End page context\]/g` lacks `i` flag, `[END PAGE CONTEXT]` bypasses | `webview/WebBrowserPage.svelte:163` | Add `i` flag: `/\[End page context\]/gi` |

## Tier B — `final: medium` (mostly 3/3; correctness + UX)

| # | Title | File (re-grep) | Fix |
|---|---|---|---|
| B1 | `init()` double-registration race — `destroy()` clears `initPromise` while `initInner()` in-flight → double listeners on HMR/rapid mount | `state/assistant.svelte.ts:826-832` | Cancellation flag (not clearing initPromise); check flag before each listener registration |
| B2 | `dismissEnhanced` leaks Haiku spawn in init window — guard `enhancing && enhanceRequestId` false before requestId assigned → subprocess runs to billing | `assistant/Composer.svelte:561` | Bump `enhanceSeq` unconditionally at start of `dismissEnhanced` before the live-check |
| B3 | Markdown preview parse+sanitize synchronous every keypress (`streaming=false` bypasses rAF throttle) | `assistant/Composer.svelte:1023` | Pass `streaming={true}` in preview path or rAF-debounce `previewDraft` |
| B4 | AssistantPane no virtualization — all messages mounted, O(n) DOM+effects (also #59 deferral) | `assistant/AssistantPane.svelte:456-474` | Intersection-observer lazy mount / virtual list |
| B5 | STT in-flight `polishWebSpeechFinal` leaks capTimer + stale `$state` writes on HMR destroy — `destroy()` doesn't bump `polishGuard` | `state/stt.svelte.ts:186` | `this.polishGuard++` at top of `destroy()` |
| B6 | `polishing` not reset on `consume()` — next session's polish silently skipped | `state/stt.svelte.ts:327` | `cancelPolish()` inside `consume()` before returning transcript |
| B7 | `polishing` not reset on `setConfig()` restart — new session inherits `polishing=true`, blocks all subsequent polishing | `state/stt.svelte.ts:303` | `cancelPolish()` before `recognition.abort()` in `setConfig()` |
| B8 | "scratch that" with leading text drops both segment AND pre-command words | `state/stt.svelte.ts:705` | Always pop last segment, then push `rest` back if non-empty |
| B9 | Whisper "send it" never fires when transcript IS only the command (`t.length>0` false) | `state/stt.svelte.ts:582` | `send = true` unconditionally when send command matched |
| B10 | `download()` failure during `repair()` shows generic "Update couldn't install" (wrong copy) | `state/updates.svelte.ts:265-278` | Add `repairing` flag; "Repair failed" copy when set |
| B11 | `saveAsstMaxBudget`: typing `0` shows "Cleared" success but draft not cleared (UX mismatch) | `settings/SettingsPage.svelte:210-221` | Validate before `setMaxBudgetUsd`; explicit error for invalid |

## Tier C — `final: low`, votes 3/3 (real but minor; batch by area)

| # | Title | File (re-grep) | Fix |
|---|---|---|---|
| C1 | `dropTab` doesn't cancel armed `saveTimer` — zombie `doSave` → `maybeGenerateTitle` on dead tab | `state/assistant.svelte.ts:555` (or `persistence.ts:173` — re-grep) | `clearTimeout(tab.saveTimer)` in `dropTab`; also guard `host.tabs.has(convoId)` in `maybeGenerateTitle` |
| C2 | `startLogin` 180s `while`-loop uncancellable across destroy/HMR — writes dead store 3 min | `state/assistant.svelte.ts:1288` | Cancellation flag set by `destroy()`, checked each iteration |
| C3 | `loadWorkspaceFiles` finally clears guard unconditionally (concurrent-root window) | `state/assistant/workspace.ts:139-152` | `if (host.workspaceFilesLoadingFor === root)` before clearing |
| C4 | telemetry `turns[]`/`events[]` unbounded (only `/diag-clear` resets) | `state/assistant/telemetry.ts:17` | Ring-buffer cap 500 turns / 2000 events |
| C5 | `agentSpawns[]` never reset in `beginTurn`/`clear` — O(n) scans grow per turn | `state/assistant.svelte.ts:259` | Reset in `beginTurn()` or cap+evict |
| C6 | `rawLineLog` O(n) `shift()` per stream line at cap | `state/assistant/streaming.ts:679` | Ring buffer (head/tail) |
| C7 | `promptPreview` (120 chars user text) in every TurnRecord, no TTL (privacy) | `state/assistant/send.ts:119` | Redact after turn / omit from TurnRecord |
| C8 | `prevSteerCount` leaks across tab switches — spurious rail pulse | `composer/QueueRail.svelte:87` | Reset in tab-identity `$effect` or `{#key tabId}` |
| C9 | EnhanceBar index-keyed `{#each}` skips materialize anim on regenerate | `composer/EnhanceBar.svelte:142` | `{#key enhanceSeq}` to force remount |
| C10 | `slashOpen` Enter dispatches real turn on unknown `/zzz` | `assistant/Composer.svelte:864` | Early return when `slashOpen && slashFiltered.length===0` |
| C11 | `mentionResults` O(N) fuzzy scan TWICE per keystroke (oninput + onkeyup) | `assistant/Composer.svelte:204` | Drop redundant onkeyup; debounce oninput |
| C12 | SettingsMenu `$effect` conflates resize-listener registration w/ repositioning → re-adds listener on recompute | `composer/SettingsMenu.svelte:63` | Split into two `$effect`s |
| C13 | `enhanceStatus` renders model-supplied path w/o length cap → overflow | `composer/EnhanceBar.svelte:87` | Truncate path 80 chars backend-side |
| C14 | Markdown render `$effect` resets cursor counters on `shikiReady` flip → re-animates revealed words | `assistant/Markdown.svelte:442` | Track `prevHtml`; reset counters only when `baseHtml` changed |
| C15 | Markdown `parsed.items` → `pinTasksFromChecklist` writes store at ~60fps, multi-instance clobber | `assistant/Markdown.svelte:472` | Debounce store write / gate on `streaming=false` |
| C16 | StreamAskUser reset `$effect` reads `$state` it writes — re-fires every selection click | `stream/StreamAskUser.svelte:38` | `untrack()` the guard read |
| C17 | `parseAskUserResult` lazy regex — model `\nA: ` truncates question, forges displayed answer | `stream/streamModel.ts:398` | Make first group greedy |
| C18 | `liveTokens` `$derived` reads `void now` → re-eval every 1s no-op | `stream/StreamTurn.svelte:87` | Remove `void now` |
| C19 | `hoverTimer` not torn down on ConversationList unmount — fires 460ms post-unmount | `shell/ConversationList.svelte:106` | `$effect(() => () => clearTimeout(hoverTimer))` |
| C20 | AssistantWelcome rapid root-switch → two concurrent IPC loads | `assistant/AssistantWelcome.svelte:42` | Add `!workspaceFilesLoadingFor` to load condition |
| C21 | AssistantPane FLIP `prevEmpty` not reset on tabId change — spurious `runComposerFlip` (no-ops but wastes tick) | `assistant/AssistantPane.svelte:198` | Reset `prevEmpty`+`flipFirst` in tab-switch `$effect` |
| C22 | AppShell `onWinDrop` captures live activeTab at invoke-time not drop-time — wrong tab on mid-await switch | `components/AppShell.svelte:82` | Capture `targetConvoId` before await, pass explicitly |
| C23 | STT `init()` listeners leak when `destroy()` races mid-await sub() loop | `state/stt.svelte.ts:186` | Abort flag checked in `sub()` before pushing to `unlisten[]` |
| C24 | STT unbounded `segments[]` + O(n) join per utterance commit (continuous mode) | `state/stt.svelte.ts:721` | Cap 500 / collapse committed segments to one string |
| C25 | `modelDownloads` full object spread per download-progress chunk | `state/stt.svelte.ts:261` | Throttle progress 150ms/key |
| C26 | Voice auto-send triggerable by ambient audio, no confirm window | `state/stt.svelte.ts:582` | Debounce / grace-window cancel before send |
| C27 | STT `destroy()` doesn't invalidate in-flight polish → writes new instance's store | `state/stt.svelte.ts:186` | `this.polishGuard++` in `destroy()` (same as B5) |
| C28 | CommandPalette double `$effect` subscription (openTick + open) — redundant re-run on close | `dialogs/CommandPalette.svelte:179` | Read only `openTick` |
| C29 | `repair()` no `state==="available"` guard — silently discards pending update | `state/updates.svelte.ts:296` | `|| state === "available"` in early-return |
| C30 | UpdateDialog release notes index-keyed — in-place patch on content change | `dialogs/UpdateDialog.svelte:211` | Key `` `${i}:${ln.kind}` `` |
| C31 | Tauri error objects stringified into Settings UI — leak Rust error chains/paths | `settings/SettingsPage.svelte:205,217` | `console.error` full; show generic summary |
| C32 | `refreshConversations` no size bound — O(N log N) sort all on every palette open (8 shown) | `dialogs/CommandPalette.svelte:179-183` | Rust-side limit / memoize sort `$derived` |
| C33 | WebBrowserPage `$effect` reads+writes `pendingUrl` w/o untrack — redundant re-run per nav | `webview/WebBrowserPage.svelte:210` | `untrack()` the clear write |
| C34 | StatsPanel `now` frozen at mount — wrong day boundary past midnight / range change | `home/StatsPanel.svelte:29` | `let now = $state(...)`, refresh in `$effect` on range |
| C35 | Context-menu `innerText` on `<pre>` forces layout reflow in right-click handler | `state/contextMenu.svelte.ts:191` | `innerText`→`textContent` |

## Tier D — `final: low`, votes 2/3 (verify hardest; some may be false-positive)

| # | Title | File (re-grep) | Fix |
|---|---|---|---|
| D1 | `applyTodoWrite` byContent map drops dup-content keys → second task gets fresh id | `state/assistant/streaming.ts:334` | `Map<string,string[]>` by content, match by position |
| D2 | `setConfig()` restart-token setTimeout closes over stale `this.recording` (double start race) | `state/stt.svelte.ts:307` | `!this.recording` check inside setTimeout |
| D3 | EditDiff `hl()` in `$derived` → hidden dep on `shikiReady`, O(N) re-derive on flip | `assistant/EditDiff.svelte:212-226` | Split highlight from line construction into two `$derived`s |
| D4 | MessageBubble no virtualization for tool chips — all mount on render | `assistant/MessageBubble.svelte:496` | Collapse-all-but-first / virtualize |
| D5 | QueueRail `sendQueuedNow` removes chip before steer resolves → reorder on `no_active_turn` | `composer/QueueRail.svelte:67` | Await steer before removing chip |
| D6 | SubAgentDock index-keyed blocks — **verifiers note append-only makes this safe in practice** (likely skip) | `assistant/SubAgentDock.svelte:158` | Stable `b.id` keys if pursued |
| D7 | SubAgentDock `b.result` no cap — but gated behind explicit expand (low impact) | `assistant/SubAgentDock.svelte:181` | Truncate ~4096 chars |
| D8 | `cliUpdate` AbortController timer in finally — fires during `res.json()` → spurious "timed out" | `state/cliUpdate.svelte.ts:255-300` | `clearTimeout` after `!res.ok` check, before parse |
| D9 | Settings error/output `pre-wrap` exposes ANSI/control sequences from CLI | `settings/SettingsPage.svelte:479-481` | Strip ANSI `/\x1b\[[0-9;]*m/g` + truncate 500 |
| D10 | Unsanitized `[` in page title/URL — malformed delimiter header | `webview/WebBrowserPage.svelte:166-168` | Add `[` to strip: `/[\[\]\r\n]/g` |
| D11 | Unbounded `composerDraft` growth from repeated addToChat (~40KB/block) | `webview/WebBrowserPage.svelte:171-172` | Cap 200k chars + warn |
| D12 | Unbounded model-list DOM from adversarial local endpoint | `state/localLlm.svelte.ts:138` | `.slice(0,100)` |

## Patterns (sweep candidates, not one-offs)
- **Timer/listener cleanup gap** (B5, C1, C2, C12, C19, C23, C27, D2): systemic missing `$effect`/`onDestroy` teardown — a focused cleanup sweep would clear ~8 at once.
- **Unbounded arrays → OOM** (C4, C5, C7, C24, D11, D12): cap-on-write discipline.
- **Svelte5 `$effect` self-invalidation** (C14, C16, C33): `untrack()` writes that feed reads.
- **Index-keyed `{#each}`** (C9, C30, D6): stable-id keys.
- **`stt.svelte.ts` is the single densest hotspot** (~11 findings) — STT-focused pass clears a big chunk.
- **Prompt-injection via webview page content** (A4, A5, D10): one hardening of `WebBrowserPage` sanitization covers all.

## STILL OPEN after cont.189 triage (Tier A/B/C1–C22 scope)
Both are deliberate skips — real findings, but the "right" fix is a judgment call, not a mechanical one-liner:
- **C5** — `tab.agentSpawns` never reset in `beginTurn()` (`streaming.ts`), grows O(n) per turn. NOT auto-fixed: the activity dock + `helpers.ts:267` render *completed* spawns from earlier in the conversation, so a blind reset-per-turn would wipe history the UI intends to show. Needs a decision: cap+evict (keep recent N) vs reset-per-turn (lose cross-turn view) vs leave (it's perf, not correctness). Lean cap+evict.
- **C7** — `promptPreview` (120 chars of user text) stored in every `TurnRecord` with no TTL (`send.ts:119`), a mild privacy/retention concern. NOT auto-fixed: redacting/omitting it may break whatever surfaces read the preview (telemetry, recent-turn UI). Needs a policy call on retention vs the feature that consumes it before editing.
