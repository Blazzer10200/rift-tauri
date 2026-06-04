# Frontend — Correctness & Reactivity

_62 confirmed findings._ [← back to index](README.md)

## Frontend — Correctness & Reactivity

### Severity-Sorted Findings

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| medium | Missing `return` after Task/Agent block — falls through to ToolChip append | `assistant.svelte.ts:606` | Add `return;` after the Task/Agent block, matching TodoWrite/TaskCreate/TaskUpdate |
| medium | applyTaskCreate numeric IDs incompatible with applyTodoWrite content-keyed IDs | `assistant.svelte.ts:550` | Unify ID scheme or add cross-API guard in applyTodoWrite |
| medium | `expandedThinking` keyed by unstable grouped[] index — wrong block expanded after streaming recompute | `MessageBubble.svelte:683` | Key by `unit.key` (stable string) instead of positional `ui` |
| medium | `askSubmitting` never reset on success — submit/cancel buttons permanently stuck | `ToolChip.svelte:687` | Add `finally { if (!askAnswered) askSubmitting = false; }` |
| medium | `openTab` called as side-effect of right-click context menu | `HistoryDrawer.svelte:39` | Remove `openTab` from `onRowContext`; let menu actions decide |
| medium | Undeclared `opener` causes ReferenceError at runtime | `ActivityPanel.svelte:253` | Import `openUrl` from `@tauri-apps/plugin-opener`; remove `opener` object |
| medium | Whisper `stop()` silently no-ops during recording start-up race | `stt.svelte.ts:340` | Track `startInvoked` flag synchronously in `start()`; invoke stop unconditionally |
| medium | Auto-expand heuristic always returns 1 for same-line-count diffs — SMALL_DIFF threshold useless | `EditDiff.svelte:178` | Count changed lines from diff chunks, not line-count delta |
| medium | `{#each assistant.panes}` keyed by index instead of stable `p.tabId` | `AssistantPage.svelte:205` | Change `(i)` to `(p.tabId)` |
| medium | pause/resume resets full timeout, ignoring elapsed time | `ToastHost.svelte:38` · `toast.svelte.ts:83` | Track `startedAt`; resume with `remaining = totalMs − elapsed` |
| medium | Enter key always confirms regardless of focused element | `Confirm.svelte:40` | Guard with `document.activeElement` check before calling `decide(true)` |
| low | Unsafe cast on assistant envelope — `usage` field missing from StreamEnvelope type | `assistant.svelte.ts:787` | Add `usage?: Record<string, unknown>` to the `assistant` variant in types.ts |
| low | Unsafe `as string` cast on unknown IPC input fields before `String()` coercion | `assistant.svelte.ts:601` | Remove inner `as string`; `String(block.input?.subagent_type ?? "fork")` is sufficient |
| low | Unsafe `as` cast adds impossible `undefined` union arm on IPC done-event payload | `assistant.svelte.ts:1373` | Drop cast; use the already-correct listen generic |
| low | `drainQueue` microtask re-queues into orphaned TabState after tab drop | `assistant.svelte.ts:2054` | Guard re-queue: check tab still in `this.tabs` before writing back |
| low | `removeQueued` silently no-ops for non-active tabs via proxy getter | `assistant.svelte.ts:2092` | Accept optional `tabId`; route to specific tab's queue |
| low | Tooltip shows raw `{paneIndexFor(id)}` — string literal instead of template literal | `ChatTabsBar.svelte:484` | Change to backtick template: `` `Open in pane ${paneIndexFor(id)}` `` |
| low | `lastSeenUpdate` plain `let` resets on re-mount — spurious task pulse on workspace switch | `ChatTabsBar.svelte:352` | Declare as `$state(0)` or narrow effect deps with `untrack` |
| low | Unsafe `as` cast on MultiEdit `edits` array — silent undefined if payload shape changes | `MessageBubble.svelte:712` | Narrow via explicit type-guard before constructing the spread |
| low | `$effect` initializes `expanded` to `false` then immediately overrides — spurious flush on card mount | `ToolChip.svelte:33` | Replace with `let expanded = $state(isCard)` |
| low | `{#if true}` always-true conditional — Other option always rendered | `ToolChip.svelte:635` | Remove wrapper or add `allowOther?: boolean` to AskQuestion type |
| low | Unsafe `as TextBlock` cast in `msgTextPreview` — missing type predicate | `HistoryDrawer.svelte:173` | Use `.filter((b): b is TextBlock => b.type === "text")` |
| low | `$effect` writes back to tracked `targetSettingsSection` dep without `untrack` | `SettingsPage.svelte:60` | Wrap `clearSettingsSection()` call in `untrack(...)` |
| low | `asstApiKeyDraft` never cleared after save — Save button stays permanently enabled | `SettingsPage.svelte:129` | Add `asstApiKeyDraft = ""` after successful `setApiKey` call |
| low | Unsafe indexed access: `modelDownloads[m.id]` typed as non-nullable | `SettingsPage.svelte:669` | Change field to `Partial<Record<string, DownloadProgress>>` in stt.svelte.ts |
| low | `$derived` with side effect mutates plain `everStreamed` variable | `Markdown.svelte:353` | Move assignment to a `$effect` watching `streaming` |
| low | `pointercancel` not handled — resize stuck + listener leak | `ChatRail.svelte:39` | Register `pointercancel` on `onUp`; remove it there too |
| low | `fmtTime` uses `Date.now()` at render time — relative timestamps never auto-refresh | `ChatRail.svelte:216` | Drive from a `$state` tick updated every 60s via `setInterval` |
| low | Untyped IPC payload cast on `ModelInfo[]` and `DownloadProgress` events | `stt.svelte.ts:206` | Add field-presence guards before acting on payloads |
| low | `iconFor()` implicitly returns `undefined` — no null-guard at call site | `UpdateDialog.svelte:29` | Add explicit return type + default branch; or null-guard at call site |
| low | `closeAllTabs` skips teardown and leaves stale pane references | `tabs.ts:487` | Iterate and call `dropTab`/`pruneTabUi` per entry; scrub panes before persist |
| low | `restoreTabs` double-loads on startup — first load silently overwritten | `tabs.ts:265` | Resolve winning tab in one pass; issue single `loadConversation` |
| low | `greeting()` computed once at mount, stale across hour boundaries | `AssistantWelcome.svelte:131` · `HomePage.svelte:67` | Use `$derived(greeting())` in both files |
| low | `loadWorkspaceBranch` $effect fires without load guard | `AssistantWelcome.svelte:85` | Guard with `&& assistant.workspaceBranch == null` |
| low | `$effect` reads then writes `fracs` — spurious re-run on every drag event | `AssistantPage.svelte:94` | Read `fracs.length` via `untrack()`; only track `assistant.panes.length` |
| low | Cold-start metrics silently null when turn[0] fails the modelId filter | `telemetry.ts:67` | Move `i === 0` cold-start block before the `continue` guard |
| low | Inner `const u` shadows outer `const u` in same loop iteration | `telemetry.ts:142` | Remove inner `const u`; reuse the outer binding |
| low | `onComplete` fires after component may be destroyed | `SplashOverlay.svelte:47` | Track `destroyed` flag via `onDestroy`; guard `onComplete()` call |
| low | `aria-hidden` bound to string `"false"` instead of boolean/omit | `SplashOverlay.svelte:59` | Use `aria-hidden={exiting \|\| undefined}` |
| low | Unsafe `any` icon prop flows into untyped component invocation | `ToastHost.svelte:23` | Type `icon` as `typeof Info \| undefined` using lucide-svelte's Icon type |
| low | Unvalidated localStorage spread into typed `CodePrefs` state | `ui-prefs.svelte.ts:64` | Validate each field before spreading; reject unknown types |
| low | Redundant `as WorkspaceId[]` cast suppresses TypeScript narrowing | `Titlebar.svelte:16` | Remove the cast; `.filter()` already returns `WorkspaceId[]` |
| low | `everOpened` hard-coded seed `"chat"` stale before `init()` fires | `workspace.svelte.ts:65` | Initialize to empty Set; let `init()` seed authoritatively |
| low | Unsafe `as Persisted` cast — `activeIdx` and tab fields unvalidated | `browser-tabs.svelte.ts:20` | Type-guard `activeIdx` (number) and required tab fields before returning |
| low | Dead SFTP `remotePath` field in `BrowserTab` type and all mutation paths | `browser-tabs.svelte.ts:5` | Remove field, `defaultRemote` param, and `updateRemotePath()` |
| low | Unhandled promise rejection on Shiki init failure | `Markdown.svelte:16` | Add `.catch(() => { /* plain-text fallback */ })` |
| low | Re-exported `WorkspaceId` leaks unrelated module's public surface | `command-palette.svelte.ts:37` | Remove the re-export; consumers import from `./workspace.svelte` |
| low | `accessibility.init()` silently discards persisted state when localStorage throws | `accessibility.svelte.ts:27` | Wrap init body and setter writes in try/catch |
| low | `loadWorkspaceFiles` stale-overwrite race on workspace switch | `workspace.ts:81` | Re-check `host.workspace.current === root` after await before assigning |
| low | `run()` swallows action errors and always closes menu | `FilePathMenu.svelte:44` | Move `onClose()` inside try; surface failures via toast |
| low | Global event listeners leak if component unmounts before setTimeout fires | `OpenInPaneMenu.svelte:38` | Track `destroyed` flag; skip `addEventListener` if already destroyed |
| low | `tone` prop silently no-ops when `icon` is omitted | `EmptyState.svelte:23` | Apply `data-tone` to root `.empty` div unconditionally |
| low | `aria-hidden` suppresses the only accessible name on logo image | `RiftLogo.svelte:11` | Decorative: `alt=""`; or remove `aria-hidden` if branded |
| low | `dontAsk` state not reset on programmatic close | `Confirm.svelte:26` | Add `$effect(() => { if (!open) dontAsk = false; })` |
| low | ~200+ lines of dead CSS for stripped multi-step onboarding flow | `onboarding.css:21` | Delete all `.ob-rail`, `.ob-steps`, `.ob-sync*`, `.ob-log*`, etc. blocks |
| low | Dead CSS `--muted` alias referencing deleted SyncPage | `app.css:113` | Remove the `--muted` declaration and comment |
| low | `$effect` clamps `activeIdx` by reading it as tracked dep — churn on every keystroke | `CommandPalette.svelte:184` | Wrap `activeIdx` read in `untrack()`; only track `flat.length` |
| low | Empty catch swallows non-compat errors in tooltip `onFocus` | `tooltip.ts:200` | Narrow to `catch (e) { if (!(e instanceof DOMException)) throw e; }` |
| low | `lastNotice` not cleared on compaction failure paths | `compaction.ts:117` | Clear `host.lastNotice` in finally block or before each failure return |
| low | Unsafe non-null cast on `import.meta.hot` bypasses type safety | `updates.svelte.ts:235` | Use optional cast pattern: `const hot = (import.meta as { hot?: ... }).hot; if (hot) hot.dispose(...)` |
| low | `openMenu` with empty options sets `highlight=0`; ArrowKey produces `NaN` highlight | `Select.svelte:55` | Guard `openMenu`: return early if `!options.length` |
| low | Async `checkOnLaunch()` return value silently discarded in `$effect` | `AppShell.svelte:35` | Add `void` prefix to signal intentional discard |
| low | `$state<Set>` initial seed `"chat"` stale if stored activeId differs | `workspace.svelte.ts:65` | Initialize `everOpened` to empty Set; let `init()` seed it |

---

### Per-Finding Details

**[medium] Missing `return` after Task/Agent block** (`assistant.svelte.ts:606`)
The Task/Agent branch pushes to `this.agentSpawns` but has no early `return`. Execution continues past the DENY set (which excludes Task/Agent), so `this.activity.currentLabel` is set AND `mutateStreaming` appends a ToolChip — double-processing every agent spawn inconsistently with every other internal tool name. Fix: add `return;` after the closing `}` of the Task/Agent block, matching lines 587/592/597.

**[medium] applyTaskCreate numeric IDs incompatible with applyTodoWrite content-keyed IDs** (`assistant.svelte.ts:550`)
`applyTaskCreate` assigns `id = String(this.tasks.length + 1)` (`"1"`, `"2"`, …). If `applyTodoWrite` fires later it wholesale-replaces `this.tasks` with `todo-<content>`-keyed IDs, after which any `applyTaskUpdate("2")` silently no-ops. Both paths share the same `this.tasks` with no cross-API guard. Fix: unify on one ID scheme or reset numeric-ID tasks when `applyTodoWrite` fires.

**[medium] `expandedThinking` keyed by unstable grouped[] index** (`MessageBubble.svelte:683`)
`expandedThinking` stores the positional `ui` index from the `$derived` `grouped` array. During streaming, new blocks shift every index, so a user-expanded think block at `ui=2` silently loses its open state once a preceding block is inserted (the block shifts to `ui=3`). Fix: key by `unit.key` (e.g. `"b_3"`), which is stable and computed once at grouping time.

**[medium] `askSubmitting` never reset on success** (`ToolChip.svelte:687`)
Both `submitAskUser` and `cancelAskUser` set `askSubmitting = true` before the IPC await but only reset it in the catch block. On the happy path, `askSubmitting` stays `true` until `tool.status` transitions to `"done"`. If the backend status event is delayed or dropped, the submit and cancel buttons are permanently disabled with no recovery. Fix: add a `finally { if (!askAnswered) askSubmitting = false; }` to both functions.

**[medium] `openTab` called as side-effect of right-click** (`HistoryDrawer.svelte:39`)
`onRowContext` checks `!assistant.openTabs.includes(id)` and calls `assistant.openTab(id)` before setting `ctxMenu`. A right-click on any closed conversation forcibly opens and activates it, regardless of what the user picks (or dismisses) from the menu. Fix: remove `openTab` from `onRowContext`; set `ctxMenu` unconditionally and let menu actions handle opening.

**[medium] Undeclared `opener` causes ReferenceError** (`ActivityPanel.svelte:253`)
`openSource` calls `opener.openUrl(item.value)` but `opener` is never imported or declared. In ESM strict mode `if (!opener)` throws `ReferenceError: opener is not defined` before the guard runs. The function has no call sites in the current template so this is unreachable today, but any wiring of the Sources panel will immediately throw. Fix: `import { openUrl } from "@tauri-apps/plugin-opener"` and call it directly.

**[medium] Whisper `stop()` silently no-ops during startup race** (`stt.svelte.ts:340`)
For the Whisper engine, `this.recording` is only set `true` when the async `stt://state: recording` event arrives, not inside `start()`. Any `stop()` call in the window between `stt_start_recording` returning and the event firing hits the `if (!this.recording && !this.transcribing) return` guard and silently returns an empty transcript while recording continues indefinitely. Fix: track a synchronous `startInvoked` flag in `start()` and invoke `stt_stop_recording` unconditionally when it is set.

**[medium] Auto-expand heuristic always returns 1 for same-line-count diffs** (`EditDiff.svelte:178`)
The formula `Math.abs(oldLines.length − newLines.length) + (oldStr === newStr ? 0 : 1)` evaluates to `1` for any two strings that differ but have the same line count (e.g., a 50-line rewrite). Since `1 <= SMALL_DIFF (12)`, such diffs always auto-expand, defeating the collapse heuristic. Fix: count changed lines from the actual diff chunks.

**[medium] `{#each assistant.panes}` keyed by index** (`AssistantPage.svelte:205`)
Index-keying causes Svelte to hand an existing `AssistantPane` instance the props of the wrong pane after mid-list removal or reorder. Scroll position, local `$state`, and focus bleed across the wrong tab. Fix: change `(i)` to `(p.tabId)`.

**[medium] pause/resume resets full timeout, ignoring elapsed time** (`ToastHost.svelte:38` · `toast.svelte.ts:83`)
`pause()` calls `clearTimer` (deletes the map entry) with no elapsed-time recording. `resume()` unconditionally reschedules using the original full duration — so a toast hovered at T=3900ms on a 4000ms timer resets to a fresh 4000ms window. The `if (this.timers.has(id)) return` guard in `resume()` is also dead after a pause (the entry was deleted). Fix: record `startedAt` on schedule; resume with `remaining = totalMs − elapsed`.

**[medium] Enter key always confirms regardless of focused element** (`Confirm.svelte:40`)
`onKey` is bound to `svelte:window` and calls `decide(true)` on Enter with no `document.activeElement` check. When focus is on the Cancel button or "Don't ask again" checkbox, Enter still returns `confirmed: true`, including on `isDanger` paths. Fix: guard with `document.activeElement` before calling `decide(true)`.

---

**[low] Unsafe cast on assistant envelope — `usage` field missing from StreamEnvelope** (`assistant.svelte.ts:787`)
The `assistant` variant of `StreamEnvelope` has no `usage` property, requiring an unsafe widening cast to access `env.message.usage`. Protocol renames silently produce `undefined` with no compiler error. Fix: add `usage?: Record<string, unknown>` to the `assistant` variant in types.ts.

**[low] Unsafe `as string` on unknown IPC input fields** (`assistant.svelte.ts:601`)
`block.input?.subagent_type` and `block.input?.description` are cast `as string` before `String()`. The intermediate cast suppresses TypeScript narrowing; non-string values produce `[object Object]` silently. Fix: remove the inner casts — `String(block.input?.subagent_type ?? "fork")` already handles `unknown`.

**[low] Impossible `| undefined` union arm on IPC done-event cast** (`assistant.svelte.ts:1373`)
`e.payload as { session_id?: string } | undefined` introduces a dead `undefined` arm that Tauri's event contract never produces. Misleads readers about invariants and would hide a mis-routing if the guard were accidentally removed. Fix: drop the cast and use the already-correct listen generic.

**[low] `drainQueue` microtask re-queues into orphaned TabState** (`assistant.svelte.ts:2054`)
If the tab is closed between the guard at line 2045 and the microtask firing, the bail path writes `tab.queue = [next, ...tab.queue]` into an orphaned `TabState` no longer in `this.tabs`, silently losing the item. Fix: guard the re-queue with a check that the tab is still registered.

**[low] `removeQueued` only operates on active tab** (`assistant.svelte.ts:2092`)
The `queue` getter/setter proxy through `activeTab`, so `removeQueued` can never touch a background tab's queue. Currently dead code (zero call sites), but the public API is silently incorrect. Fix: accept an optional `tabId` param and route to the specific tab.

**[low] Tooltip shows raw `{paneIndexFor(id)}`** (`ChatTabsBar.svelte:484`)
`{"Open in pane {paneIndexFor(id)}"}` is a JS string literal, not a template literal. The tooltip always displays the literal text `Open in pane {paneIndexFor(id)}`. Fix: change to `` `Open in pane ${paneIndexFor(id)}` ``.

**[low] `lastSeenUpdate` plain `let` resets on re-mount** (`ChatTabsBar.svelte:352`)
`lastSeenUpdate` resets to `0` on component re-mount while `assistant.ui.tasksUpdatedAt` persists on the module-level class. On workspace switch, the effect re-fires and triggers a spurious 700ms pulse animation. Fix: declare as `$state(0)` or narrow effect deps.

**[low] Unsafe `as` cast on MultiEdit `edits` array** (`MessageBubble.svelte:712`)
`b.input.edits as Array<Record<string, unknown>>` and `b.input.file_path` are accessed without narrowing beyond `Array.isArray`. A payload shape change silently passes `file_path: undefined` to `EditDiff`. Fix: narrow via explicit type-guard before constructing the spread.

**[low] `$effect` initializes `expanded` via post-render effect** (`ToolChip.svelte:33`)
`expanded` is initialized `false` then overridden via `$effect(() => { if (isCard) expanded = true; })`, causing a spurious reactive flush on every card mount. Fix: `let expanded = $state(isCard)`.

**[low] `{#if true}` always-true conditional on Other option** (`ToolChip.svelte:635`)
The Other (custom) button is wrapped in a dead `{#if true}`, making it unconditional and preventing any future `allowOther: false` suppression. Fix: remove wrapper or add `allowOther?: boolean` to the AskQuestion type.

**[low] Unsafe `as TextBlock` cast without type predicate** (`HistoryDrawer.svelte:173`)
`.filter(b => b.type === "text")` returns `Block[]`; the subsequent `.map(b => (b as TextBlock).text)` silences the mismatch. Fix: `.filter((b): b is TextBlock => b.type === "text").map(b => b.text)`.

**[low] `$effect` writes back to tracked `targetSettingsSection` dep** (`SettingsPage.svelte:60`)
`clearSettingsSection()` writes back a tracked dependency, scheduling one extra effect run per deep-link. The `untrack` import already exists unused. Fix: wrap `clearSettingsSection()` in `untrack(...)`.

**[low] `asstApiKeyDraft` never cleared after save** (`SettingsPage.svelte:129`)
The Save button remains enabled after a successful save because `asstApiKeyDirty = asstApiKeyDraft.trim().length > 0` stays true. A second click re-submits the same key. Fix: add `asstApiKeyDraft = ""` after the successful `setApiKey` call.

**[low] Unsafe indexed access on `modelDownloads`** (`SettingsPage.svelte:669`)
`Record<string, DownloadProgress>` does not include `undefined` in its index signature, so `stt.modelDownloads[m.id]` is typed non-nullable despite being `undefined` at runtime for most models. The `&& prog` guard works today but looks removable to a refactorer. Fix: change field to `Partial<Record<string, DownloadProgress>>`.

**[low] `$derived` with side effect mutates plain `everStreamed`** (`Markdown.svelte:353`)
Side effects in `$derived` computations are forbidden in Svelte 5; speculative re-runs could set `everStreamed = true` on non-streaming renders, permanently enabling word-reveal animation. Fix: move the assignment to a `$effect` watching `streaming`.

**[low] `pointercancel` not handled on resize** (`ChatRail.svelte:39`)
An OS dialog or alt-tab cancels the pointer; `onUp` never fires; `resizing` stays `true` permanently (disabling CSS transitions) and the two `window` listeners leak for the session. Fix: register `window.addEventListener("pointercancel", onUp)` alongside `pointerup`.

**[low] `fmtTime` timestamps never auto-refresh** (`ChatRail.svelte:216`)
`fmtTime(c.updatedAt)` calls `Date.now()` at render time with no timer. "now" labels persist indefinitely until an unrelated state change triggers a re-render. Fix: drive from a `$state` tick updated via `setInterval(..., 60_000)` in `onMount`.

**[low] Untyped IPC payload cast on `ModelInfo[]` and `DownloadProgress`** (`stt.svelte.ts:206`)
TypeScript generics provide no runtime validation. A new `phase` string from the backend silently skips all `if`-branches. Fix: add field-presence guards at the top of each listener.

**[low] `iconFor()` implicitly returns `undefined`** (`UpdateDialog.svelte:29`)
No explicit return type; no default branch. TypeScript infers `| undefined` but the call site at line 95 has no null-guard. Fix: add a `default: return Download;` branch or an explicit exhaustive return type.

**[low] `closeAllTabs` skips teardown and leaves stale pane references** (`tabs.ts:487`)
`host.tabs = new Map()` bypasses `dropTab`/`pruneTabUi` for each entry and never calls `scrubTabFromPanes`, so `host.panes` retains stale `tabId` strings that `persistTabs()` saves to localStorage. Fix: iterate `openTabs`, call `dropTab`/`pruneTabUi` per entry, then reset panes before persisting.

**[low] `restoreTabs` double-loads on startup** (`tabs.ts:265`)
`loadConversation` is called for `active` (line 245) then immediately overwritten by a second call for the focused pane tab (line 266) if they differ — wasted IPC. If the second call throws, `currentConvoId` is left set to the first tab. Fix: determine the winning tab in one pass before any `loadConversation` call.

**[low] `greeting()` computed once at mount — stale across hour boundaries** (`AssistantWelcome.svelte:131` · `HomePage.svelte:67`)
Both files store `greeting()` in a plain `const`. In Svelte 5 runes, this value has no reactive dependency and will never re-evaluate after an hour boundary. Fix: use `$derived(greeting())` in both files.

**[low] `loadWorkspaceBranch` $effect fires without load guard** (`AssistantWelcome.svelte:85`)
The effect has no `&& assistant.workspaceBranch == null` guard, so rapid workspace switches issue duplicate uncancelled IPC calls. The companion `loadWorkspaceFiles` effect correctly guards with `workspaceFiles.length === 0`. Fix: add the equivalent guard.

**[low] `$effect` reads then writes `fracs` — spurious re-run on every drag event** (`AssistantPage.svelte:94`)
The effect tracks `fracs` as a dependency (reading `fracs.length`), so every drag-position update at pointer-move frequency re-fires the effect body. No write occurs (guard is false), but `loadFracs` is not needed. Fix: read `fracs.length` via `untrack()`; only track `assistant.panes.length`.

**[low] Cold-start metrics silently null when turn[0] fails the modelId filter** (`telemetry.ts:67`)
The `i === 0` cold-start block sits after the `continue` guard. An aborted or error first turn (modelId == null, endKind !== "success") skips it, leaving `firstTurnCostUsd` and `coldStartCacheCreate` permanently null. Fix: move the cold-start block before the `continue` guard.

**[low] Inner `const u` shadows outer `const u`** (`telemetry.ts:142`)
Line 77 and line 142 both declare `const u = t.resultUsage || t.envelopeUsage` in the same loop iteration (outer body vs inner `if` block). Expressions are identical; shadow is accidental. Fix: remove the inner declaration and reuse the outer binding.

**[low] `onComplete` fires after component may be destroyed** (`SplashOverlay.svelte:47`)
`finally` sets `exiting = true`, awaits `sleep(exitMs)`, then calls `onComplete()`. Fast HMR or double-mount can destroy the component during the sleep, after which `onComplete` fires into a stale closure. Fix: set `destroyed = true` in `onDestroy`; guard `onComplete()` call.

**[low] `aria-hidden` bound to string `"false"`** (`SplashOverlay.svelte:59`)
`aria-hidden={exiting ? "true" : "false"}` renders as a string attribute. Some AT treat any non-absent `aria-hidden` value as truthy. Fix: `aria-hidden={exiting || undefined}`.

**[low] Unsafe `any` icon prop flows into untyped component invocation** (`ToastHost.svelte:23`)
`item.icon` is typed `any`, so `<Icon size={12}/>` is an untyped call. A non-component value silently compiles and crashes at runtime. Fix: type `icon` as `typeof Info | undefined` using lucide-svelte's exported `Icon` type.

**[low] Unvalidated localStorage spread into `CodePrefs`** (`ui-prefs.svelte.ts:64`)
`{ ...DEFAULT_CODE, ...c }` is applied with only an `object` type check. A stored string `"false"` for `ligatures` is truthy and yields `"normal"` instead of `"none"` in `applyCode`. Fix: validate each field type before spreading.

**[low] Redundant `as WorkspaceId[]` cast** (`Titlebar.svelte:16`)
`.filter()` already returns `WorkspaceId[]`; the trailing cast is a no-op that will hide mismatches on future `WorkspaceId` changes. Fix: remove the cast.

**[low] `everOpened` hard-coded seed `"chat"` before `init()`** (`workspace.svelte.ts:65`)
First-render subscribers see `"chat"` as opened even when stored `activeId` is different, mounting the `chat` component one tick early. Fix: initialize to `new Set<WorkspaceId>()`; let `init()` seed authoritatively.

**[low] Unsafe `as Persisted` cast in browser-tabs** (`browser-tabs.svelte.ts:20`)
Only `Array.isArray(parsed.tabs)` is checked. A string `activeIdx` produces `NaN` via coercion; missing tab fields propagate as `undefined` into `this.tabs`. Fix: type-guard `activeIdx` (number) and required tab string fields before returning.

**[low] Dead SFTP `remotePath` field in `BrowserTab`** (`browser-tabs.svelte.ts:5`)
`BrowserTab.remotePath`, `hydrate(defaultRemote)`, `open(…, remotePath)`, and `updateRemotePath()` are all dead post-2026-06-03 SFTP strip. No consumer reads `remotePath`. Fix: remove the field, parameter, and method; update all call-sites.

**[low] Unhandled promise rejection on Shiki init failure** (`Markdown.svelte:16`)
`whenReady().then(...)` has no `.catch()`. A grammar bundle failure leaves `shikiReady = false` forever and fires `window.onunhandledrejection` silently. Fix: add `.catch(() => { /* plain-text fallback */ })`.

**[low] `WorkspaceId` re-exported from command-palette module** (`command-palette.svelte.ts:37`)
A stray `export type { WorkspaceId }` couples two unrelated modules. Any consumer importing from this path silently breaks if `workspace.svelte` renames the type. Fix: remove line 37.

**[low] `accessibility.init()` unguarded localStorage calls** (`accessibility.svelte.ts:27`)
`init()` and all setter methods call `localStorage.getItem`/`setItem` with no try/catch. A `SecurityError` (private-browsing) propagates uncaught, leaving the class partially initialized. Fix: wrap `init()` body and setter writes in try/catch, matching the pattern in `browserDock`.

**[low] `loadWorkspaceFiles` stale-overwrite race on workspace switch** (`workspace.ts:81`)
The concurrent-call guard is only checked at call entry. If the workspace root changes while the `invoke` is in-flight, the old result overwrites `workspaceFiles` under the new root. Fix: re-check `host.workspace.current === root` after the await before assigning.

**[low] `run()` in FilePathMenu swallows action errors** (`FilePathMenu.svelte:44`)
All five menu actions silently close the menu on Tauri invoke failure with only `console.warn`. Fix: move `onClose()` inside the `try` block (success-only) and surface errors via toast.

**[low] Global event listeners leak if component unmounts before setTimeout fires** (`OpenInPaneMenu.svelte:38`)
The zero-delay `setTimeout` adds listeners after the `onMount` cleanup has already run, leaving them permanently attached with no removal path. Fix: set `destroyed = true` in cleanup; skip `addEventListener` if already destroyed.

**[low] `tone` prop silently no-ops when `icon` is omitted** (`EmptyState.svelte:23`)
`data-tone` is only applied inside `{#if icon}`. Callers passing `tone="danger"` without an icon get default accent styling with no error. Fix: apply `data-tone={tone}` to the root `.empty` div unconditionally.

**[low] `aria-hidden` suppresses alt text on logo image** (`RiftLogo.svelte:11`)
`alt="Rift"` and `aria-hidden="true"` coexist; the alt is unreachable to AT. Fix: decorative → `alt=""`; branded → remove `aria-hidden`.

**[low] `dontAsk` state not reset on programmatic close** (`Confirm.svelte:26`)
`dontAsk` is only reset inside `decide()`. A programmatic close (flipping `open=false`) leaves it dirty, so the checkbox renders pre-checked on the next open. Component is currently unused (zero call sites). Fix: `$effect(() => { if (!open) dontAsk = false; })`.

**[low] ~200+ lines of dead CSS for stripped onboarding flow** (`onboarding.css:21`)
The SFTP/sync multi-step walkthrough was removed 2026-06-03 but its CSS (`ob-rail`, `ob-steps`, `ob-sync*`, `ob-log*`, `ob-bar`, `ob-choices`, etc.) was not trimmed. Loaded globally, not tree-shaken. Fix: delete all unreferenced `.ob-*` blocks; only keep classes used by `OnboardingFlow.svelte` and `ClaudeAuth.svelte`.

**[low] Dead `--muted` CSS alias for deleted SyncPage** (`app.css:113`)
The `--muted: var(--bg-elev-2)` custom property and its comment reference the deleted `SyncPage.svelte`; no consumer references `var(--muted)`. Fix: remove the declaration.

**[low] `$effect` clamps `activeIdx` as tracked dep — churn on every keystroke** (`CommandPalette.svelte:184`)
The effect reads `activeIdx` (tracking it), then conditionally writes it, causing it to re-fire on every ArrowDown/ArrowUp. No infinite loop (guard catches stable state), but the body executes on every navigation key unnecessarily. Fix: read `activeIdx` via `untrack()`; only track `flat.length`.

**[low] Empty catch swallows non-compat errors in tooltip `onFocus`** (`tooltip.ts:200`)
The empty catch discards all exceptions from `node.matches(':focus-visible')`, not just the intended `NotSupportedError`. Fix: narrow to `catch (e) { if (!(e instanceof DOMException)) throw e; }`.

**[low] `lastNotice` not cleared on compaction failure paths** (`compaction.ts:117`)
`host.lastNotice = "Compacting conversation…"` is set at line 117 and cleared only on the success path. Both failure returns set `host.lastError` but leave the stale notice, showing both simultaneously. Fix: clear `host.lastNotice` in the `finally` block.

**[low] Unsafe non-null cast on `import.meta.hot`** (`updates.svelte.ts:235`)
The cast strips `hot?:` after the outer `if` guard has confirmed it is truthy, hiding the optional type from tsc. Fix: `const hot = (import.meta as { hot?: { dispose: ... } }).hot; if (hot) hot.dispose(...)`.

**[low] `openMenu` with empty options produces NaN highlight** (`Select.svelte:55`)
`Math.max(0, options.findIndex(...))` sets `highlight=0` on an empty array. An ArrowKey event then computes `(0 + dir + 0) % 0 = NaN`, leaving `highlight = NaN` permanently. Fix: guard `openMenu` with `if (!options.length) return`.

**[low] Async `checkOnLaunch()` discarded without `void`** (`AppShell.svelte:35`)
Unlike all other async calls in the same `$effect` (lines 53, 63, 69, 74), `checkOnLaunch()` lacks a `void` prefix. Rejections become unhandled Promise rejections silently. Fix: add `void updates.checkOnLaunch()`.
