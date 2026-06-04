# Frontend — Security, Error & Perf

_65 confirmed findings._ [← back to index](README.md)

## Severity-Sorted Findings Table

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| High | Split pane `#each` keyed by index | `AssistantPage.svelte:205` | Key by `p.tabId ?? i` |
| Medium | `style` globally whitelisted in DOMPurify — CSS injection | `Markdown.svelte:351` | Restrict `style` to `span` only |
| Medium | `applyTaskCreate` IDs from `tasks.length` — silent TaskUpdate mismatch | `assistant.svelte.ts:550` | Separate `taskCreateCount` field |
| Medium | 6 config mutators unguarded — silent invoke throw | `assistant.svelte.ts:1734` | try/catch + surface via `lastNotice` |
| Medium | Tooltip literal `{paneIndexFor(id)}` never interpolated | `ChatTabsBar.svelte:484` | Template literal `` ` `` |
| Medium | Send/Stop/Queue button no accessible name | `Composer.svelte:1513` | Dynamic `aria-label` mirroring mode |
| Medium | `user-image-thumb` button no accessible name | `MessageBubble.svelte:662` | Add `aria-label` or descriptive `alt` |
| Medium | `submitAskUser`/`cancelAskUser` errors swallowed silently | `ToolChip.svelte:683` | Local `askError` state + render near buttons |
| Medium | `openTab` rejection swallowed in `onRowContext` | `HistoryDrawer.svelte:39` | Add `.catch()` |
| Medium | Per-turn DOM parse + TreeWalker in `$derived` at ~24 fps | `Markdown.svelte:358` | Pre-split in `parsed`; update class/CSS var per tick |
| Medium | `opener` undefined — `openSource` always silently no-ops | `ActivityPanel.svelte:253` | Import `openUrl` from `@tauri-apps/plugin-opener` |
| Medium | Focus outline removed on `.cp-item:focus-visible` with no replacement | `CommandPalette.svelte:390` | Custom focus ring via `outline: 2px solid var(--accent)` |
| Medium | `deleteAllConversations` — partial delete + unconditional state wipe | `persistence.ts:302` | Move wipe inside try; use `Promise.allSettled` |
| Medium | `refreshConversations` swallows IPC errors with no user signal | `persistence.ts:73` | Set `host.lastError` in catch |
| Medium | Orphan boundary block left on `listen()` rejection | `compaction.ts:143` | Remove boundary in `finally` on failure |
| Medium | `browser_show`/`browser_hide` rejections silently swallowed | `WebBrowserPage.svelte:96,98` | Add `.catch(e => error = String(e))` |
| Low | `role=slider` with `tabindex="-1"` — unreachable by keyboard/AT | `Composer.svelte:1159` | `tabindex="0"` + `onkeydown` on slider |
| Low | `role=listbox` with `<button>` children — no `role="option"` | `Composer.svelte:1053,1073,1100` | Switch to `role="menu"` + `role="menuitem"` |
| Low | Live-pill buttons (agent/shell/tool/queue) no accessible name | `Composer.svelte:1394` | `aria-label` mirroring tooltip text |
| Low | `wand-pulse` animation missing `prefers-reduced-motion` guard | `Composer.svelte:2395` | Add `animation: none` in reduced-motion block |
| Low | `mic-pulse` + `mic-spin` animations missing reduced-motion guard | `Composer.svelte:1832,1839` | Extend existing reduced-motion block |
| Low | `applyTodoWrite` unconditional array reassign on every block | `assistant.svelte.ts:536` | Shallow equality guard before assign |
| Low | Non-JSON backend lines injected as assistant message text | `assistant.svelte.ts:728` | Heuristic panic/error prefix → route to `onError` |
| Low | Raw CLI lines in user-visible `lastError` | `assistant.svelte.ts:886` | Move raw samples to `console.warn`; show fingerprint only |
| Low | `void this.send()` in `queueMicrotask` — pre-invoke exceptions silently lost | `assistant.svelte.ts:2057` | `.catch(e => tab.onError(String(e)))` |
| Low | `role="menu"` no arrow-key navigation | `ChatTabsBar.svelte:803,843` | Add ArrowDown/Up/Home/End keydown handler on container |
| Low | `aria-pressed` + `aria-checked` on same `role="checkbox"` element | `ToolChip.svelte:608` | Remove `aria-pressed`; keep only `aria-checked` |
| Low | "Other" option button missing `role` + `aria-checked` | `ToolChip.svelte:640` | Mirror sibling pattern |
| Low | `selectConvo` error swallows — detail pane silently empty | `HistoryDrawer.svelte:101` | Local `detailError` state + render error message |
| Low | `assistantStore.init()` rejection unhandled | `SettingsPage.svelte:192` | Add `.catch()` |
| Low | Empty catch on `app_version` IPC | `SettingsPage.svelte:190` | At minimum `console.warn` |
| Low | Progressbar `aria-valuemax=0` during download start phase | `SettingsPage.svelte:680` | `aria-valuemax={prog.total > 0 ? prog.total : undefined}` |
| Low | Progressbar has no accessible label | `SettingsPage.svelte:680` | `aria-label={\`Downloading ${m.display_name}\`}` |
| Low | 1s ticker forces full Steps list re-render every second | `ActivityPanel.svelte:451` | `$derived` map keyed by step id for label values |
| Low | Resize separator — no keyboard handler, `svelte-ignore` suppression | `ChatRail.svelte:233` | `tabindex="0"` + ArrowLeft/Right `onkeydown` |
| Low | Search clear button missing `aria-label` | `ChatRail.svelte:172` | `aria-label="Clear search"` |
| Low | Tauri event listeners never unregistered (dev HMR leak) | `stt.svelte.ts:154` | Add `destroy()` draining `this.unlisten` |
| Low | Partial `listen()` subscription on single shared try/catch | `stt.svelte.ts:177` | One try/catch per `listen()` call |
| Low | `openReleasePage` error swallowed — no user feedback | `UpdateDialog.svelte:69` | Write `updates.downloadError` in catch |
| Low | `#each notes` keyed by index | `UpdateDialog.svelte:172` | Key by `ln.kind + '|' + ln.text` |
| Low | `{@html ch}` in EditDiff — no DOMPurify | `EditDiff.svelte:286` | Sanitize Shiki output before `{@html}` |
| Low | `copyTimer` setTimeout not cleared on unmount | `EditDiff.svelte:226` | `onDestroy(() => clearTimeout(copyTimer))` |
| Low | `restoreTabs` partial-mutate state persisted on failure | `tabs.ts:269` | Reset `openTabs`/`panes` in catch before `persistTabs()` |
| Low | Icon-only 'Forget' button no accessible label | `AssistantWelcome.svelte:271` | `aria-label="Forget {leafName(r)}"` |
| Low | O(N²) `flat.indexOf(it)` per rendered item in CommandPalette | `CommandPalette.svelte:266` | `$derived` Map keyed by `it.id` |
| Low | `aria-pressed` misused on navigation buttons | `ActivityBar.svelte:164,202` | Replace with `role="tab"` + `aria-selected` or `aria-current` |
| Low | `pointermove`/`pointerup` on buttons — silent drag break if capture fails | `ActivityBar.svelte:166` | Attach handlers to container; log capture failures |
| Low | `$effect` branch-load — no in-flight cancellation (stale-write race) | `HomePage.svelte:26` | Compare root before writing `workspaceBranch` |
| Low | Branch-load error swallowed — no UI signal | `HomePage.svelte:26` | Surface `workspaceBranchError` flag |
| Low | `invoke('app_version')` error swallowed | `updates.svelte.ts:107` | `this.state = 'error'` or surface notice |
| Low | Root-cause download error lost in fallback catch | `updates.svelte.ts:155` | `String(e)` primary; log secondary |
| Low | `disabled` attribute hides options from AT | `Select.svelte:158` | Use `aria-disabled` + conditional click guard |
| Low | `data:` URI passthrough to native webview | `WebBrowserPage.svelte:25` | Restrict to `https?://` only in `normalizeUrl` |
| Low | Async keyboard handler rejections void-cast | `AppShell.svelte:53,63,69,74` | `.catch(e => toast/log)` on each call |
| Low | O(N×M) per-model timing average in `summarize()` | `telemetry.ts:152` | Accumulate per-bucket arrays in main loop |
| Low | `snapshot()` calls `Date.now()` twice — inconsistent `capturedAt`/`durationMs` | `telemetry.ts:18` | Single `const now = Date.now()` |
| Low | `aria-live` region emits no text — no completion announcement | `SplashOverlay.svelte:56` | Inject visually-hidden `<span>Ready</span>` on `exiting` |
| Low | Svelte `fly` transitions ignore `prefers-reduced-motion` | `ToastHost.svelte:41` | Gate `duration`/`y` on `matchMedia` result |
| Low | Window control Promises dropped (minimize/maximize/close) | `Titlebar.svelte:76` | `.catch(console.error)` on each (idiomatic Tauri) |
| Low | `copyCommand` setTimeout not cancelled on re-entry | `cliUpdate.svelte.ts:138` | Store handle; `clearTimeout` before re-set |
| Low | `JSON.parse` failure silently swallowed in `workspace.init()` | `workspace.svelte.ts:93` | `console.warn` + DiagBus emit |
| Low | `new Set([...spread])` on every `setActive()` first-visit | `workspace.svelte.ts:126` | Accepted pattern per Svelte 5 rules; cosmetic nit |
| Low | `loadPersisted()` no field-level validation on `BrowserTab` | `browser-tabs.svelte.ts:20` | Validate field types before returning |
| Low | Empty catch on `localStorage.removeItem` in `reset()` | `browser-tabs.svelte.ts:67` | Match `savePersisted()` pattern with `console.warn` |
| Low | Page title rendered as `<span>` inside `<header>` landmark | `PageHeader.svelte:32` | Change to `<h2>` |

---

## Per-Finding Detail

**[High] Split pane `#each` keyed by index — `AssistantPage.svelte:205`**
`{#each assistant.panes as p, i (i)}` uses position as identity. When a pane is spliced from the middle (`closePane`, `reorderTabs`), Svelte reuses the DOM/component instance at position N for the logical pane that was at N+1, silently misassigning reactive state, scroll position, and all child component state. `p.tabId` is already available and passed as a prop — it should be the key. Fix: `{#each assistant.panes as p, i (p.tabId ?? i)}` and ensure every pane has a stable id.

**[Medium] `style` globally whitelisted in DOMPurify — CSS injection — `Markdown.svelte:351`**
`"style"` appears in the global `ALLOWED_ATTR` list covering all tags. The inline comment's safety claim is wrong: `text` is raw LLM output; only fenced code blocks go through Shiki — all other markdown content also passes through with `style` allowed. A model response like `<div style="position:fixed;top:0;left:0;width:100vw;height:100vh">` passes sanitization and can redress the assistant pane. Shiki only needs `style` on `<span>` elements. Fix: restrict via a DOMPurify hook or `ALLOWED_STYLE_ATTR: ['span']` (DOMPurify 3.x).

**[Medium] `applyTaskCreate` IDs from `tasks.length` — silent TaskUpdate mismatch — `assistant.svelte.ts:550`**
`const id = String(this.tasks.length + 1)` treats the shared tasks array (which may already contain `todo-<content>` entries from `applyTodoWrite`) as a 1-based creation counter. If TodoWrite runs first, the first TaskCreate task gets id `"3"` while the CLI sends `taskId: "1"` — `applyTaskUpdate` finds no match and silently no-ops, dropping all status updates. Fix: separate `taskCreateCount` field incremented only in `applyTaskCreate`, reset alongside `tasks`.

**[Medium] 6 config mutators unguarded — silent invoke throw — `assistant.svelte.ts:1734`**
`setUseFullConfig`, `setTrustLevel`, `setAutoCompactThreshold`, `setCompactModel` (and partially `setApiKey`/`setMaxBudgetUsd`) have no try/catch. Their call sites in SettingsPage use `void`/no-catch. On throw, the state mutation line after `await invoke(...)` is skipped — frontend retains the old value while the backend was never updated — with no user signal. Fix: `try { await invoke(...); this.field = v; } catch (e) { this.lastNotice = String(e); throw e; }`.

**[Medium] Tooltip literal `{paneIndexFor(id)}` never interpolated — `ChatTabsBar.svelte:484`**
`use:tooltip={"Open in pane {paneIndexFor(id)}"}` — curly braces inside a JS string literal are plain characters, not Svelte template expressions. The tooltip always renders the literal text `Open in pane {paneIndexFor(id)}`. Fix: template literal `` use:tooltip={`Open in pane ${paneIndexFor(id)}`} ``.

**[Medium] Send/Stop/Queue button no accessible name — `Composer.svelte:1513`**
The primary action button switches between Send/Stop/Queue modes. Its only content is icon-slot SVGs; no `aria-label` and no `aria-describedby`. The tooltip action creates a floating `div[role=tooltip]` but never links it to the button. Screen readers announce an unlabelled button. Fix: `aria-label={mode === 'stop' ? 'Stop current turn' : mode === 'queue' ? 'Queue message' : 'Send message'}`.

**[Medium] `user-image-thumb` button no accessible name — `MessageBubble.svelte:662`**
Contains only `<img alt="">` (decorative) and `use:tooltip`. The tooltip action never sets `aria-label` or `aria-describedby` on the host element. WCAG 4.1.2 failure. Fix: add `aria-label="View full size image"` directly on the button, or a descriptive `alt` on the img.

**[Medium] `submitAskUser`/`cancelAskUser` errors swallowed silently — `ToolChip.svelte:683`**
Both catch blocks only `console.warn` and reset `askSubmitting`. No `askError` state exists; the ask-body renders no error message on IPC failure. The form silently returns to editable state — users cannot distinguish a transient retry from a permanent failure. Fix: introduce `let askError = $state<string|null>(null)`, set in catch, render near action buttons.

**[Medium] `openTab` rejection swallowed in `onRowContext` — `HistoryDrawer.svelte:39`**
`void assistant.openTab(id).then(...)` with no `.catch()`. On rejection, the context menu never appears and no error surfaces. Fix: `.catch(err => console.error('openTab failed', err))` at minimum, or `try/catch` async IIFE.

**[Medium] Per-turn DOM parse + TreeWalker in `$derived` at ~24 fps — `Markdown.svelte:358`**
`revealWords()` creates a `<template>`, sets `.innerHTML` (full HTML parse), walks all text nodes, and serializes back via `.innerHTML`. The `processed = $derived.by()` calls this on every `shownCount` change, which the rAF loop advances at WORD_MS=42ms. For a 1000-word response this runs 600+ DOM parse/serialize cycles. Work is on a detached template (no reflow), but it is real JS cost bounded to the animation window. Fix: pre-split words in the `parsed` pass; per-tick only update CSS custom property `--shown` to control visibility.

**[Medium] `opener` undefined — `openSource` always silently no-ops — `ActivityPanel.svelte:253`**
`opener` is never imported or declared. The guard `if (!opener) return` evaluates to `if (undefined) return`, so `openSource` always exits early. The feature is completely dead. Fix: import `{ openUrl }` from `@tauri-apps/plugin-opener`; replace `opener.openUrl(...)` with `openUrl(...)`.

**[Medium] Focus outline removed on `.cp-item:focus-visible` — `CommandPalette.svelte:390`**
`.cp-item:focus-visible { outline: none }` with no custom replacement. `activeIdx` is only updated via `onmouseenter` and arrow-key handlers — not on native tab focus — so tabbing directly into the list provides no visible focus indicator. Fix: `outline: 2px solid var(--accent); outline-offset: -2px;`.

**[Medium] `deleteAllConversations` partial delete + unconditional state wipe — `persistence.ts:302`**
The try/catch wraps the delete loop, but the state-wipe block (drop every open tab, null active-convo fields) runs unconditionally after it. A mid-loop failure leaves undeleteed records on disk while tabs are dropped. `refreshConversations` at line 321 rescues the records, but tab context and active-session state are lost. Fix: move the wipe inside the try block, or use `Promise.allSettled` and only wipe successfully-deleted ids.

**[Medium] `refreshConversations` swallows IPC errors — `persistence.ts:73`**
Catch block does only `console.warn`; never sets `host.lastError`. Called after every save, rename, delete, and title-generation pass. A persistent IPC failure leaves `host.conversations` stale with no user indication. Fix: set `host.lastError` in catch, or re-throw to let call-sites decide.

**[Medium] Orphan boundary block on `listen()` rejection — `compaction.ts:143`**
`stagedBoundary` is appended to `tab.messages` before the try block. If `await listen(...)` at line 162 rejects, execution jumps to finally — which only resets `compactingNow` and calls `progressUnlisten()`. Both cleanup paths (filter the boundary out) are inside the try body and unreachable on a listen rejection. The boundary with `streaming: true` persists permanently. Fix: move the push inside try after `listen()` succeeds, or remove the boundary in finally when the operation did not complete.

**[Medium] `browser_show`/`browser_hide` rejections silently swallowed — `WebBrowserPage.svelte:96,98`**
Both `void invoke("browser_show").then(syncBounds)` and `void invoke("browser_hide")` have no `.catch()`. On failure the native webview stays hidden/shown, `syncBounds` never runs, and no error surfaces. Fix: `.catch((e) => { error = String(e); })` on each.

**[Low] `role=slider` with `tabindex="-1"` — `Composer.svelte:1159`**
ARIA requires interactive widgets to be directly focusable. `tabindex="-1"` permanently excludes the slider from Tab order. The inner `.effort-notch` buttons are natively focusable, so AT impact is reduced, but the slider role violation is real. Fix: `tabindex="0"` + `onkeydown` ArrowLeft/Right on the slider div.

**[Low] `role=listbox` with `<button>` children — `Composer.svelte:1053,1073,1100`**
Three menus (slash, mention, settings) use `role="listbox"` but their children are `<button>` elements with no `role="option"` and no `aria-selected`. ARIA ownership violation. Fix: switch to `role="menu"` + `role="menuitem"` on each button (simpler for command menus).

**[Low] Live-pill buttons no accessible name — `Composer.svelte:1394`**
Four `live-pill` buttons for agent/shell/tool/queue counts lack `aria-label`. The tooltip text is not in the accessibility tree. Fix: `aria-label={`${agentCount} sub-agents running — click to open Activity`}` etc.

**[Low] `wand-pulse` animation missing `prefers-reduced-motion` guard — `Composer.svelte:2395`**
`.wandbtn.enhancing` fires `wand-pulse` (infinite scale 1→1.08). The existing `@media (prefers-reduced-motion: reduce)` block at line 2418 suppresses several animations but omits this one. Fix: add `.wandbtn.enhancing { animation: none; }` inside the reduced-motion block.

**[Low] `mic-pulse` + `mic-spin` missing reduced-motion guard — `Composer.svelte:1832,1839`**
Two infinite mic animations are absent from the sole reduced-motion block (line 1867) which only covers `.mic-wave span`. Fix: extend the block with `.micbtn.recording { animation: none; }` and `:global(.mic-spin) { animation: none; }`.

**[Low] `applyTodoWrite` unconditional array reassign — `assistant.svelte.ts:536`**
`this.tasks = next` fires on every TodoWrite block even when content is unchanged, triggering full Svelte reactive propagation. Fix: shallow equality guard before assigning, or in-place status-only mutation.

**[Low] Non-JSON backend lines injected as assistant message text — `assistant.svelte.ts:728`**
On JSON.parse failure during active streaming, the raw line is appended via `enqueueText()` rather than routing to `onError`. A Rust panic trace on stdout would surface as garbled assistant text instead of an error banner. Fix: heuristic check for panic/error prefixes → route to `onError`.

**[Low] Raw CLI lines in user-visible `lastError` — `assistant.svelte.ts:886`**
Up to 3 raw CLI stdout lines (240 chars each) are interpolated into `this.lastError` (not `{@html}`, so no XSS). File paths or env values from the CLI environment surface in the user-visible error banner. Fix: move `nonJsonSamples` to `console.warn`; show only the event-type fingerprint in the UI string.

**[Low] `void this.send()` in `queueMicrotask` — pre-invoke exceptions silently lost — `assistant.svelte.ts:2057`**
`send()`'s try/catch only wraps the `invoke()` call. Pre-invoke code (ensureTab, message construction, UUID generation) runs outside it. A throw there propagates into the `void`-cast queueMicrotask and is silently dropped; the queue item is already popped. Fix: `.catch(e => tab.onError(String(e)))`.

**[Low] `role="menu"` no arrow-key navigation — `ChatTabsBar.svelte:803,843`**
`proj-menu` and `view-menu` have `role="menu"` with `role="menuitem"` children but register only Escape in their keydown handlers. Arrow-key focus management required by ARIA authoring practices is absent. Fix: ArrowDown/Up/Home/End focus management on the container.

**[Low] `aria-pressed` + `aria-checked` on `role="checkbox"` — `ToolChip.svelte:608`**
Multi-select option buttons carry `role="checkbox"` alongside both `aria-pressed` and `aria-checked` simultaneously. `aria-pressed` belongs to `role="button"` and is incompatible with checkbox. Fix: remove `aria-pressed` entirely; keep only `aria-checked`.

**[Low] "Other" option button missing `role` + `aria-checked` — `ToolChip.svelte:640`**
All sibling option buttons carry `role="radio"/"checkbox"` and `aria-checked`; the "Other (custom)" button at line 640 omits both, rendering as a plain button inside the radiogroup container. Fix: mirror the sibling pattern.

**[Low] `selectConvo` error swallows — detail pane silently empty — `HistoryDrawer.svelte:101`**
Catch block sets `detailRecord = null` with no error state, no console.error, no user message. The detail pane shows "No recap available" / "No messages yet" — indistinguishable from an empty conversation. Fix: local `detailError` state variable, rendered in the detail pane.

**[Low] `assistantStore.init()` rejection unhandled — `SettingsPage.svelte:192`**
`void assistantStore.init().then(...)` with no `.catch()`. If `init()` rejects (listen() failure), auth state stays uninitialised silently. In practice internal guards make this near-impossible, but the `.catch()` is still missing. Fix: add `.catch(e => console.error('assistantStore.init failed', e))`.

**[Low] Empty catch on `app_version` IPC — `SettingsPage.svelte:190`**
`try { appVersion = await invoke<string>("app_version"); } catch {}` — fully empty catch. On failure `appVersion` stays `'?'` with zero diagnostic signal. Fix: `catch (e) { console.warn('app_version invoke failed', e); }`.

**[Low] Progressbar `aria-valuemax=0` + no accessible label — `SettingsPage.svelte:680`** (two findings, same element)
`aria-valuemax={prog.total}` is unguarded — during the `"start"` phase `prog.total=0`, producing an invalid progressbar state. Additionally, no `aria-label` or `aria-labelledby` links the bar to the model name shown in a sibling element. Fix: `aria-valuemax={prog.total > 0 ? prog.total : undefined}` and `aria-label={\`Downloading ${m.display_name}\`}`.

**[Low] 1s ticker forces full Steps list re-render — `ActivityPanel.svelte:451`**
`agoLabel(r)` reads reactive `now` (updated every second) inline in both `#each baseSteps` and `#each extraSteps`. Every tick invalidates all rendered step rows. `STEP_CAP=4` limits the default-collapsed case to 4 rows, so practical impact is minimal. Fix: `$derived` map keyed by step id for label values, keeping the expensive list diffing outside the per-tick path.

**[Low] Resize separator no keyboard handler — `ChatRail.svelte:233`**
`role="separator"` div with `onpointerdown`/`ondblclick` but no `tabindex` and no `onkeydown`. Two `svelte-ignore` comments suppress a11y warnings without fixing them. Keyboard-only users cannot resize the rail. Fix: `tabindex="0"` + ArrowLeft/Right handler calling `setChatRailWidth`.

**[Low] Search clear button missing `aria-label` — `ChatRail.svelte:172`**
Button contains only `<X size={10} />` with no `aria-label`. Screen readers announce an unlabelled button. Fix: `aria-label="Clear search"`.

**[Low] Tauri event listeners never unregistered (dev HMR leak) — `stt.svelte.ts:154`**
`private unlisten: UnlistenFn[]` is populated but never drained — no `destroy()` method exists. In production (singleton) this is fine. In dev HMR each reload accumulates a new set of handlers, causing duplicate state mutations. Fix: `destroy()` method draining the array; call from component `onDestroy`.

**[Low] Partial `listen()` subscription on shared try/catch — `stt.svelte.ts:177`**
Five sequential `await listen()` calls share one try/catch. If a mid-sequence call rejects, earlier listeners are retained while later channels (stt://error, stt://download_progress) are never subscribed — partial wiring with no diagnostic. In practice Tauri's local IPC registration never partially fails. Fix: one try/catch per `listen()` call.

**[Low] `openReleasePage` error swallowed — `UpdateDialog.svelte:69`**
`catch (e) { console.warn("openUrl failed", e) }` never writes `updates.downloadError`, so the existing error card is never shown on failure. Fix: call `updates.setDownloadError(String(e))` (or equivalent) in catch.

**[Low] `#each notes` keyed by index — `UpdateDialog.svelte:172`**
`{#each notes as ln, i (i)}` causes full DOM tear-down when `notesMarkdown` changes. In practice this happens at most once per update check, making impact negligible. Fix: key by `ln.kind + '|' + ln.text`.

**[Low] `{@html ch}` in EditDiff — no DOMPurify — `EditDiff.svelte:286`**
Shiki-highlighted HTML is rendered via `{@html ch}` with no sanitization. `highlighter.svelte.ts:15` claims "DOMPurify sanitizes downstream" — that contract only holds in Markdown.svelte. Shiki HTML-escapes its output, so no currently exploitable path exists, but the defense-in-depth gap is real. Fix: sanitize Shiki output before `{@html}`, or centralize sanitization inside `highlightSync()`.

**[Low] `copyTimer` setTimeout not cleared on unmount — `EditDiff.svelte:226`**
1400ms timer fires post-unmount writing `copied = false` on a detached instance. No crash, no data leak; risk is limited to hot-reload scenarios. Fix: `onDestroy(() => { if (copyTimer) clearTimeout(copyTimer); })`.

**[Low] `restoreTabs` partial state persisted on failure — `tabs.ts:269`**
If `loadConversation()` throws after `host.openTabs` is written (line 242) but before pane restoration, the finally block persists the partial state to localStorage. `refreshConversations` recovers records, but tab layout is lost. Fix: reset `openTabs`/`panes` to clean defaults in catch before `persistTabs()` runs.

**[Low] Icon-only 'Forget' button no accessible label — `AssistantWelcome.svelte:271`**
Button contains only `<X size={11}/>` with `use:tooltip={"Forget"}`. Tooltip is mouse-only. Fix: `aria-label="Forget {leafName(r)}"`.

**[Low] O(N²) `flat.indexOf(it)` in CommandPalette — `CommandPalette.svelte:266`**
`{@const idx = flat.indexOf(it)}` inside the inner `#each` loop scans the full flat array per item. At ~15 items benign; degrades with conversation count. Fix: `const flatIdx = $derived(new Map(flat.map((it, i) => [it.id, i])))` and `flatIdx.get(it.id) ?? 0`.

**[Low] `aria-pressed` misused on navigation buttons — `ActivityBar.svelte:164,202`**
Both workspace and Settings buttons use `aria-pressed={isActive}` — they are navigation items, not toggles. AT announces them as toggle buttons. Fix: `role="tab" aria-selected={isActive}` (add `role="tablist"` to parent), or `aria-current="page"`.

**[Low] `pointermove`/`pointerup` on individual buttons — `ActivityBar.svelte:166`**
Handlers are per-button rather than on the container. If `setPointerCapture` fails (swallowed in try/catch at line 47), fast drags past button gaps silently stop tracking. In WebView2 capture virtually never fails, but the swallowed failure path and missing `pointercancel` handler are real gaps. Fix: attach `onpointermove`/`onpointerup`/`onpointercancel` to the container; log capture failures.

**[Low] `$effect` branch-load stale-write race — `HomePage.svelte:26`**
`loadWorkspaceBranch()` has no in-flight guard. Rapid workspace switches can queue concurrent `invoke('assistant_workspace_branch')` calls that race to write `host.workspaceBranch`. Worst case: briefly stale branch label. Fix: compare captured root against current before writing, or debounce the effect.

**[Low] Branch-load error swallowed — `HomePage.svelte:26` / `workspace.ts:97`**
`catch (e) { console.warn(...); host.workspaceBranch = null; }` — the null is indistinguishable in the UI from a legitimate non-git folder. Fix: set a `workspaceBranchError` flag so the UI can render "branch unavailable" vs. no-branch.

**[Low] `invoke('app_version')` error swallowed — `updates.svelte.ts:107`**
Catch only emits `console.warn`; `this.state` is never set to `'error'` for this path. `currentVersion` stays `'?'` silently. The main `check_for_updates` path does set `this.state = 'error'` correctly. Fix: at minimum `console.warn`; ideally propagate to state so the dialog doesn't show `v? → v1.2.3`.

**[Low] Root-cause download error lost in fallback catch — `updates.svelte.ts:155`**
`this.downloadError = String(e2 ?? e)`: since `e2` is always a caught exception (never nullish), `??` always resolves to `e2`, permanently discarding the original `e` from the primary failure. Fix: `String(e) + "; fallback: " + String(e2)` or log `e` separately.

**[Low] `disabled` attribute hides options from AT — `Select.svelte:158`**
Native `disabled` on `<button role="option">` removes the element from the accessibility tree. ARIA `listbox` spec expects `aria-disabled="true"` to keep disabled options announced. Fix: replace `disabled={o.disabled}` with `aria-disabled={o.disabled}` and guard the click handler.

**[Low] `data:` URI passthrough to native webview — `WebBrowserPage.svelte:25`**
`normalizeUrl` explicitly allows `data:` URIs to pass unsanitized to `invoke("browser_navigate")`. A `data:text/html,...` URI can execute arbitrary HTML/JS in the child webview. Currently only user-typed input reaches this path (no programmatic injection), making this self-XSS only. Fix: restrict to `https?://`; reject `data:`, `javascript:`, `file:`, `blob:`.

**[Low] Async keyboard handler rejections void-cast — `AppShell.svelte:53,63,69,74`**
All four `void assistant.{openTab,newTab,closeTab,cycleTab}()` calls in `onGlobalKey` discard rejections entirely. The underlying functions are `async` with no internal try/catch, so errors from `loadConversation` or `stop` propagate to the void. Fix: `.catch(e => toast/console.error)` on each.

**[Low] O(N×M) per-model timing average in `summarize()` — `telemetry.ts:152`**
For each model key, `this.turns.filter(...)` re-scans all turns — O(M×N) total. At typical session sizes (1-2 models, hundreds of turns) negligible, but unnecessary. Fix: accumulate `ttfps`/`doneTimes` arrays per model bucket in the main loop; eliminate the post-loop re-scan.

**[Low] `snapshot()` calls `Date.now()` twice — `telemetry.ts:18`**
`capturedAt` and `durationMs` use separate `Date.now()` calls; the invariant `durationMs === capturedAt − startedAt` can be violated by scheduler jitter. Fix: `const now = Date.now();` before the return object.

**[Low] `aria-live` region emits no text — `SplashOverlay.svelte:56`**
`role="status"` + `aria-live="polite"` with purely visual children. No text is injected on completion. When `exiting` fires, `aria-hidden="true"` silences the region entirely rather than announcing "Ready". `aria-hidden="false"` (non-exiting branch) is a no-op and should be omitted. Fix: inject `<span class="sr-only">Ready</span>` on `exiting`; conditionally omit `aria-hidden`.

**[Low] Svelte `fly` transitions ignore `prefers-reduced-motion` — `ToastHost.svelte:41`**
`in:fly` and `out:fly` are JS-driven and bypass CSS `@media (prefers-reduced-motion)`. The existing CSS block at line 175 strips `transition` from `.toast` but has no effect on Svelte transitions. Fix: `const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;` → gate `duration`/`y` on that value.

**[Low] Window control Promises dropped — `Titlebar.svelte:76`**
`win.minimize()`/`win.toggleMaximize()`/`win.close()` Promises are dropped in onclick handlers. These are local Tauri IPC calls that virtually never fail in practice; the pattern is idiomatic in Tauri apps. Adding `.catch(console.error)` would be the minimum improvement but is very low priority.

**[Low] `copyCommand` setTimeout not cancelled on re-entry — `cliUpdate.svelte.ts:138`**
No stored handle — rapid repeated calls stack timers that all eventually reset `this.copied`. The class is a persistent singleton so no stale-instance leak. Fix: store handle as `_copyTimer`; `clearTimeout` before re-setting.

**[Low] `JSON.parse` failure silently swallowed in `workspace.init()` — `workspace.svelte.ts:93`**
`} catch { /* parse fail → default order */ }` — empty catch with a comment. Only affects workspace panel ordering; safe fallback, but invisible to diagnostics. Fix: `catch (e) { console.warn('[workspace] ORDER_KEY parse failed:', e); }`.

**[Low] `new Set([...spread])` on every `setActive()` first-visit — `workspace.svelte.ts:126`**
With only 3 `WorkspaceId` values and a `has()` guard, this executes at most twice per session. The pattern IS the required Svelte 5 immutable-update idiom. This is a cosmetic nit, not a defect — the `new Set(this.everOpened); .add(id)` alternative is equivalent.

**[Low] `loadPersisted()` no field-level validation on `BrowserTab` — `browser-tabs.svelte.ts:20`**
JSON.parse result is cast to `Persisted` with only `Array.isArray` check. Individual field types (id, name, localPath, remotePath) are unvalidated. Corrupt localStorage data passes into `$state`. The claimed path-traversal vector to MCP tools is wrong (`localPath` never flows to `read_file`/`list_dir`), but type validation is still good practice. Fix: validate `typeof tab.id/name/localPath/remotePath === 'string'` per entry.

**[Low] Empty catch on `localStorage.removeItem` in `reset()` — `browser-tabs.svelte.ts:67`**
`try { localStorage.removeItem(STORAGE_KEY); } catch {}` — empty body, inconsistent with `savePersisted()` which logs via `console.warn`. Fix: `catch (e) { console.warn('browser-tabs reset failed', e); }`.

**[Low] Page title rendered as `<span>` inside `<header>` landmark — `PageHeader.svelte:32`**
`<span class="head-title">{title}</span>` inside a `<header>`. Screen-reader heading navigation (H key in NVDA/JAWS) skips it. Fix: change to `<h2 class="head-title">` (or `<h1>` if only one PageHeader mounts per page).
