# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 68 — 2026-05-15 — UI polish + privacy scrub (ready to ship)

**Status: `npm run check` 0/0/0. v0.2.56-alpha cut is NEXT — all changes uncommitted, user runs `/git-ship` after bump.**

### Completed
- **TabRail kbd hints** — `⌘N` → slim digit `N` (no border, 55% opacity). Mac glyphs were wrong on Windows.
- **TabRail pin-open** — chevron at top now a pin button. Click locks rail at 220px; content reflows (`--rail-w` CSS var → AppShell grid). `localStorage` persisted (`rift.ui.rail-pinned.v1`). Only SVG rotates on hover, button body stays still.
- **Assistant BETA pill** in TabRail — matches AssistantHeader chip (warn-soft bg, warn text, 9px uppercase). Hidden in collapsed state.
- **Sync empty-state footer** — `{#if !isEmpty}` guard; dead "Apply selected" button gone when everything's synced.
- **Sync shrink-banner collapsible** — collapsed by default (one-liner: resource name + `142 → 38` count chip). Click head to expand; chevron rotates 90°. Actions (Rebaseline/Dismiss) only render when expanded.
- **Edit Server subhead** — "FiveM dev server" → "dev server" (phase-neutral).
- **Settings → Assistant α-refs** — dropped `α1`/`α2` chips; now "Stored as plaintext. Keychain migration planned."
- **About page** — Paths section (Config + Logs w/ "Open" button via plugin-opener), Diagnostics section ("Copy diagnostic info" one-click).
- **Files tab drag-reorder** — pointer-event based (HTML5 DnD flaky in webview2). `reorder(from, to)` in store. Live reorder during drag (crosses midpoint → tabs shuffle in real time). `animate:flip` (220ms) slides other tabs. Dragged tab: scale(1.04) + accent ring + shadow. Idle hover: 1px lift on 2+ tabs (`:has` selector).
- **Privacy scrub** — "Copy diagnostic info" now strips OS username from paths (`C:\Users\BLAZZER\` → `C:\Users\<user>\`), drops `navigator.userAgent` + active server name. No PII in copied output.
- **Privacy audit** — confirmed no email/real name/IPs/Trey info in source. Only Blazzer handle (Cargo author, bundle ID, update repo) — all public. Zero phone-home telemetry. App is fully standalone.

### Key Decisions
- Pointer events over HTML5 DnD for tab reorder — webview2 eats DnD events from `<button>` children. Pointer-event hit-testing on `.tabstrip` children is reliable.
- Live reorder (shuffle during drag) over drop-only — feels like Chrome/VSCode. `animate:flip` handles all slide animations without manual CSS.
- Scrub full UA string from diagnostic copy (kept `navigator.platform`) — UA leaks Windows build + webview2 version unnecessarily.

### Failed / Don't Retry
- HTML5 DnD (`draggable + ondragstart/ondragover/ondrop`) on `<div>` wrapping `<button>` — button element swallows mousedown in Chromium webview, drag never initiates. Tried twice. Use pointer events.

### Next Steps
1. **Cut v0.2.56-alpha** — bump THREE files (`package.json` + `Cargo.toml` + `tauri.conf.json`), write CHANGELOG covering S60-68 arc, user runs `/git-ship`.
2. **Confirm delete `ToolCallCard.svelte`** — orphaned since S63, zero imports. User ok needed before removal.

### Files Modified
- `src/lib/components/shell/TabRail.svelte` (kbd hints, pin button, BETA pill, CSS)
- `src/lib/state/ui-prefs.svelte.ts` (+railPinned, toggleRailPinned, applyRail)
- `src/lib/components/AppShell.svelte` (grid-template-columns CSS var)
- `src/lib/components/sync/SyncPage.svelte` (footer gate, shrink-banner collapsible)
- `src/lib/components/dialogs/AddServer.svelte` (subhead copy)
- `src/lib/components/settings/Settings.svelte` (α-refs, About paths+diagnostic, privacy scrub)
- `src/lib/components/browser/TwoPane.svelte` (drag-reorder: pointer events, animate:flip, CSS)
- `src/lib/state/browser-tabs.svelte.ts` (+reorder method)

---

## Session 67 — 2026-05-15 — Full UI consistency rework + TabRail polish

**Status: `cargo check` + `npm run check` 0/0/0. v0.2.56-alpha cut still pending (covers S60-67 arc).**

### Completed
- **4 shell primitives** — `PageHeader` (46px, `--fs-lg` title, tone accent stripe, extras/actions snippets), `PageToolbar` (36px, left/right slots), `PageFooter` (44px, active tint), `EmptyState` (52px glyph circle + title + hint + body slot). All in `src/lib/components/shell/`.
- **5 pages converted to canonical skeleton** — Conflicts (`ConflictsPage.svelte` new wrapper, double-title killed, icon-rich EmptyState), Activity (PageHeader w/ Pause/Clear, segctl→PageToolbar, EmptyState w/ Rescan/ClearFilters CTAs), Files/Browser (PageHeader w/ status subtitle + "New tab" action, empty state has "Add a server" CTA), Sync (hero→PageHeader, status pill in extras snippet), Assistant (already canonical, token fix only).
- **StatusHero deleted** — was Browser-only strip duplicating Titlebar. `src/lib/components/StatusHero.svelte` gone.
- **Token sweep** — `oklch(0.76 0.18 152)` → `var(--ok)` in `assistant/EmptyState` + `assistant/TasksDock`. Added `.pill.warn` to `app.css`. Removed unused `Inbox` import from Sync.
- **Titlebar declutter** — connection pill removed, state folded into server picker dot (now breathes on ok/connecting). One fewer element in the 44px bar.
- **StatusBar simplified** — dropped redundant queued/errors/conflicts counts. Kept: state-toggle (unique action), bg-sync pill, locks count.
- **Browser → Files rename** — TabRail label + PageHeader title + command palette entry. Tab id `browse` unchanged.
- **Sync subtitle** — shows "Not connected" instead of stale "Last scan X" when watcher is off.
- **TabRail full rework** — RIFT wordmark + favicon at top; 3 groups (workspace/ai/status) with hairline dividers; active icon tinted in tone color + glow drop-shadow; `⌘N` kbd hints on right when expanded; chevron hint → rotates on expand.

### Key Decisions
- `PageFooter` built but unused (designed for Sync bulk-op footer); keep as documented primitive.
- Markdown alert colors + `oklch(0.99 0 0)` pure-white-on-danger literals intentionally NOT tokenized — semantic contrast values, not design tokens.
- Settings left-nav stays, no PageHeader added (nav IS the chrome).

### Next Steps
1. **Cut v0.2.56-alpha** — bump THREE files (package.json + Cargo.toml + tauri.conf.json), CHANGELOG covers S60-67 arc, user runs `/git-ship`.
2. **Confirm delete `ToolCallCard.svelte`** — orphaned since S63, zero imports.
3. Possible: Conflicts sidebar auto-size (currently fixed 320px), TabRail pin-open localStorage toggle, Assistant tab first-on-launch default.

### Files Modified
- NEW: `src/lib/components/shell/PageHeader.svelte`, `PageToolbar.svelte`, `PageFooter.svelte`, `EmptyState.svelte`
- NEW: `src/lib/components/conflicts/ConflictsPage.svelte`
- DELETED: `src/lib/components/StatusHero.svelte`
- `src/lib/components/AppShell.svelte` (imports, page mounts, dead state/styles)
- `src/lib/components/shell/TabRail.svelte` (full rework — groups, wordmark, active tone, kbd hints)
- `src/lib/components/shell/Titlebar.svelte` (pill removed, dot state-aware + breathing)
- `src/lib/components/shell/StatusBar.svelte` (simplified)
- `src/lib/components/sync/SyncPage.svelte` (PageHeader, disconnected subtitle)
- `src/lib/components/activity/ActivityFeed.svelte` (PageHeader, PageToolbar, EmptyState CTAs)
- `src/lib/components/browser/TwoPane.svelte` (PageHeader, Files rename, EmptyState CTA, onAddServer prop)
- `src/lib/components/conflicts/ConflictList.svelte` (sidebar header stripped)
- `src/lib/components/assistant/EmptyState.svelte` (oklch → var(--ok))
- `src/lib/components/assistant/TasksDock.svelte` (oklch → var(--ok))
- `src/app.css` (+.pill.warn)

---

## Session 66 — 2026-05-15 — Assistant workspace decoupling + UI polish pass

**Status: Both `cargo check` + `npm run check` clean (0 errors, 0 warnings). v0.2.56-alpha cut still pending.**

### Completed
- **Workspace decoupling from FiveM/RedM** — Assistant now uses VSCode-style "Open Folder" model independent of Sync's server folders. `AssistantConfig` gains `current_root: Option<PathBuf>` + `recent_roots: Vec<PathBuf>` (cap 10, dedup, MRU-first). 4 new Tauri commands: `assistant_get_workspace`, `assistant_set_root` (validates dir, canonicalizes, updates recents), `assistant_clear_root`, `assistant_remove_recent_root`. All return `WorkspaceState { current, recent }`. `assistant_send` resolves roots: explicit `current_root` → AutoSync server folders → no-tools. AutoSync flow unchanged when no folder is open.
- **`tauri-plugin-dialog` v2** — added to `Cargo.toml` + `package.json` + registered in `lib.rs`. Capability `dialog:default` in `capabilities/default.json`. Native OS folder picker via `openDialog({ directory: true })`.
- **System addendum rewritten** — dropped "FiveM/RedM servers" framing from both `RIFT_SYSTEM_ADDENDUM_TOOLS` + `_NO_WS`. Now: "coding partner operating against a project folder the user has opened." Explicitly: "The user's project could be any language or framework; do not assume the stack."
- **EmptyState — state-aware redesign** — three distinct states, one focal point each: (1) no folder/server → accent-gradient "Open folder…" primary CTA card + compact recents list below; (2) synced-server fallback → green-tinted server card w/ quiet "Switch" button; (3) folder open → accent-tinted folder card w/ close button. Suggestions only render when workspace is wired (3 generic prompts, single column, no FiveM-specific). Hero headline adapts per state.
- **AssistantHeader pill removed** — redundant with composer's model pill. Replaced with `auth-warn` chip that only renders when auth is degraded (yellow=API key, red=not connected). Healthy state = no chip (cleaner header).
- **Model picker upgrade** — each row: `[✓] Sonnet [4.6] Balanced… [200K ctx]`. Grid: check | name+version | tagline | ctx-badge. Version chips accent-tinted when current. Context: Sonnet 4.6=200K, Opus 4.7=1M, Haiku 4.5=200K. Composer bottom pill: was `model: opus`, now `Sonnet [4.6]` with accent version badge.
- **Slash menu reorganized** — logical groups: conversation → model+compose → flow → info. `/clear` removed from picker (alias for `/new`, still works via `runSlash`). 9 visible entries.
- **HistoryDrawer autofocus a11y** — swapped `autofocus` attribute for a `use:focusOnMount` action (focus+select on mount, no lint warning).
- **Model name capitalized** in history drawer rows ("sonnet" → "Sonnet").
- **All FiveM refs in assistant code confirmed clean.** Remaining FiveM refs are in Sync/Bootstrap/AddServer (legit — Rift's primary product). No stale leaks.
- **`svelte-check` 0 errors 0 warnings** (was 2 warnings before: autofocus + line-clamp; both cleared).

### Load-Bearing Invariants
- AutoSync fallback still works untouched — if `current_root` is null, `assistant_send` checks `state.0.lock()` for engine folders exactly as before.
- `recent_roots` validated at display-time only; stale/deleted paths show in recents but fail gracefully on `assistant_set_root` ("not a directory" error surfaced to UI).

### Files Modified
- `src-tauri/Cargo.toml` (+tauri-plugin-dialog)
- `src-tauri/src/lib.rs` (plugin init + 4 new commands)
- `src-tauri/src/assistant/mod.rs` (AssistantConfig fields, RECENT_ROOTS_MAX, WorkspaceState, 4 commands, addendum rewrite, send workspace logic)
- `src-tauri/capabilities/default.json` (+dialog:default)
- `package.json` (+@tauri-apps/plugin-dialog)
- `src/lib/state/assistant.svelte.ts` (workspace state, pickFolder, setRoot, clearRoot, removeRecentRoot, refreshWorkspace)
- `src/lib/components/assistant/EmptyState.svelte` (full rewrite — state-aware)
- `src/lib/components/assistant/AssistantHeader.svelte` (workspace chip, folder btn, auth-warn replaces model pill)
- `src/lib/components/assistant/Composer.svelte` (model picker version/ctx, pill shows version, slash menu reorder)
- `src/lib/components/assistant/HistoryDrawer.svelte` (focusOnMount action, model capitalized)

### Next Steps
1. **Cut v0.2.56-alpha** — bump THREE files (package.json + Cargo.toml + tauri.conf.json), CHANGELOG covers S60-66 arc, user invokes `/git-ship`.
2. **`ToolCallCard.svelte`** — orphaned since S63, no imports. Confirm + delete with user.
3. **Test workspace picker** — open a non-FiveM project folder, verify tools fire + addendum correct.

---

## Session 65 — 2026-05-14 — Conversation history + slash expansion + markdown column polish

**Status: All wired, user signed off with "killed that shit." v0.2.56-alpha cut still pending.**

### Completed
- **Conversation history (full system)** — Rust: `~/.rift/assistant/conversations/<uuid>.json` one-file-per-convo, atomic-write via `.tmp`+rename, four Tauri commands (list/load/save/delete) w/ path-traversal guard. Frontend: `currentConvoId`, debounced `scheduleSave(700ms)`, auto-save on `onDone`, `newConversation()`/`loadConversation()`/`deleteConversation()`/`renameConversation()`. New `HistoryDrawer.svelte` — slides in from left w/ overlay backdrop, per-row rename (pencil + inline input) and two-step delete confirm. Header gets `+` (new) + `History` (chip shows count) buttons.
- **Slash commands expanded to 10** — added `/new`, `/history`, `/retry` (strips last user+assistant pair, re-fires), `/copy` (clipboard), `/cost` (session total), `/tools` (MCP discovery), plus Up/Down arrow recall via `promptHistory: string[]` (50-cap). `/clear` now == `/new` (saves before nuking).
- **`lastNotice` info banner** — separate from `lastError`; accent-tinted, click-to-dismiss, animated in. /help, /tools, /cost, /model now route here (was using red error styling).
- **`totalCostUsd` accumulation bug** — was overwriting per turn, now accumulates across session.
- **Kebab menu deleted** — header is just model pill + dock toggle + new/history. Re-probe auth lives in Settings.
- **TabRail reorganized** — primary group: Browser → Sync → Assistant → Conflicts → Activity. Settings bottom-anchored w/ divider (VSCode convention). Ctrl+1..6 renumbered to match.
- **EmptyState redesigned** — category-tinted cards (accent/info/teal/green), title+sub-prompt per card, workspace context chip (`Endure RP · blazzer@…` or yellow warn), section labels, glow on hero glyph.
- **Header model pill bug** — was hardcoded `"Sonnet · OAuth"`, now derives from `assistant.model`. Click cycles sonnet→opus→haiku. Persists via `setModel()` → localStorage.
- **Markdown column polish** — accent left-bar on `h2`/`h3`, `h2` bottom-border, custom dot bullets for `<ul>` (hollow rings for nested), dashed indent guide on nested lists, full-width rounded table w/ uppercase header + zebra + hover, code-block accent left-stripe, `::marker` accent-purple bold. Loose-list killer: `li > p:only-child { display: contents }` collapses marked's `<p>` wrappers so loose & tight read identically.

### Files Modified
- `src-tauri/src/assistant/mod.rs` (conversation types + 4 commands)
- `src-tauri/src/lib.rs` (register new commands)
- `src/lib/state/assistant.svelte.ts` (history state, 6 new slash cmds, promptHistory, lastNotice, cost fix)
- `src/lib/components/assistant/HistoryDrawer.svelte` (NEW)
- `src/lib/components/assistant/AssistantHeader.svelte` (kebab removed, +/History buttons)
- `src/lib/components/assistant/AssistantPage.svelte` (drawer mount, notice banner)
- `src/lib/components/assistant/Composer.svelte` (slash menu 10 entries, up/down recall)
- `src/lib/components/assistant/EmptyState.svelte` (full redesign)
- `src/lib/components/assistant/Markdown.svelte` (column polish, loose-list killer, ::marker)
- `src/lib/components/assistant/MessageBubble.svelte` (.content gap 8→4)
- `src/lib/components/shell/TabRail.svelte` (primary/footer split + divider)
- `src/lib/components/AppShell.svelte` (Ctrl+1..6 renumbered)
- `docs/HANDOFF.md`

### Next Steps
1. Cut v0.2.56-alpha — bump THREE files, write CHANGELOG covering S60-65 arc, user invokes `/git-ship`.
2. Decide on `ToolCallCard.svelte` (still orphaned, flagged S63).
3. User wanted to test markdown polish + history drawer in fresh session.

---

## Session 64 — 2026-05-14 — Slash commands + message queue + real stop button

**Status: All wired. Still no v0.2.56-alpha cut (user paused before bump).**

### Completed
- **Real backend stop** — `assistant_stop` Tauri cmd, tracks child PID in `Mutex<Option<u32>>`, dispatches `taskkill /F /T /PID` on Windows / `kill -TERM` on Unix. `USER_STOPPED: AtomicBool` flag distinguishes user-stop (emit `done`) from silent CLI crash (emit `error`). Wait-task checks the flag on non-success exit.
- **Frontend stop wired** — Composer button has 3 modes: idle+draft=Send, streaming+empty=Stop (red, kills via invoke), streaming+draft=Queue (muted-accent, appends instead of dropping).
- **Message queue** — `assistant.queue: {id, text}[]`. `send()` while streaming → queue. `onDone()` auto-drains via `queueMicrotask(send(next))`. Pills above composer w/ ellipsized text + X each, plus "Clear all" when 2+. `removeQueued(id)` + `clearQueue()` exposed.
- **Slash commands (client-side)** — `/clear`, `/stop`, `/model`, `/help`. Popup menu when draft starts w/ `/` and no space yet; ↑↓ nav, Tab/Enter pick, Esc cancel. Direct-fire commands skip textarea round-trip. Unknown slashes fall through to normal send (so `/path/to/file` in prompt still works).
- **`/model` sub-picker** — picking `/model` opens VSCode-style chooser w/ Sonnet/Opus/Haiku, current model marked ✓ + accent. Cursor pre-positioned on current. Header `Sonnet · OAuth` pill is now a button that cycles models on click; model name in header derives from `assistant.model` (was hardcoded — that was the bug).
- **Model persistence** — `assistant.setModel(v)` writes `rift.assistant.model` to localStorage; constructor reads it. Survives reloads.
- **Auto-scroll respect** — AssistantPage tracks `stickToBottom` via `onscroll` (within 80px of tail = true). `ResizeObserver` on messages container snaps only when stuck. User can scroll up mid-stream without being yanked back.
- **`/clear` stops in-flight stream too** — was dirty leaving zombie streamingMsgId behind.

### Key Decisions
- `/stop` halts current turn but does NOT clear queue — acts as "skip current". `/clear` is the nuke-everything path.
- Header pill click cycles model (Sonnet → Opus → Haiku → Sonnet) — quick swap w/o opening picker.
- Backend slash commands stay disabled (`--disable-slash-commands` flag retained); ours are Rift-side, never reach CLI.

### Files Modified
- `src-tauri/src/assistant/mod.rs` (PID tracking, USER_STOPPED atomic, `assistant_stop` cmd, Windows taskkill + Unix kill shellout)
- `src-tauri/src/lib.rs` (register `assistant_stop`)
- `src/lib/state/assistant.svelte.ts` (queue, model + persistence, runSlash, setModel, stop, removeQueued, clearQueue)
- `src/lib/components/assistant/Composer.svelte` (3-mode button, queue pills, slash menu, model sub-picker, clickable model pill)
- `src/lib/components/assistant/AssistantHeader.svelte` (model derived from state, clickable cycle pill)
- `src/lib/components/assistant/AssistantPage.svelte` (stickToBottom + ResizeObserver)

### Next Steps
1. User to test: `/help`, `/model` picker, queue 2+ messages, stop mid-stream, reload to verify model persists.
2. Still pending from S63: cut v0.2.56-alpha + decide on `ToolCallCard.svelte` (still orphaned).

---

## Session 63 — 2026-05-14 — Activity dock + chat cleanup (continued post-compact)

**Status: Mid-flight. All polish landed, no version bump yet. v0.2.56-alpha cut still pending.**

### Completed
- **3 remaining layout smoke tests landed** — collapsible `<details>` ✅, GFM task lists ✅ (iterated spacing 3x), `<kbd>` caps ✅.
- **Task list auto-import → Tasks dock** — `Markdown.svelte` now extracts checklists from rendered HTML, strips them from chat, calls `assistant.pinTasksFromChecklist()`. Inline accent pill replaces the stripped list: `📋 Sent to Tasks panel`. No buttons, no clicks — automatic.
- **Batch C animation polish** — tool card status pop (scale-from-0.6, 280ms cubic-bezier), error-flash background keyframe, message fly-in (4px slide-up + fade, 220ms), dock slide w/ opacity easing + min-width:280px on inner (no reflow jank), send/stop morph via stacked icons w/ rotate+scale crossfade.
- **Activity dock** (Claude Code Desktop style) — replaces dead Tokens row. New section with bold title matching Tasks weight, count badge, live elapsed timer (`30s` / `2m 14s`). `now-row` accent pill w/ spinner + monospace current-op label. **Op cards** stacked vertically: per-tool icon + name + summary + status icon; click to expand inline w/ input JSON + result block. Auto-opens dock on first MCP tool call (parity w/ first TodoWrite).
- **Chat cleanup** — removed inline `ToolCallCard` rendering + "Used N tools" chip from `MessageBubble`. Chat is now pure prose + the migration pill. All tool detail lives in dock.
- **Cleanup pass** — dropped dead `counts`/`bumpActivity` from state (chip system replaced by card list); renamed `RIFT_SYSTEM_ADDENDUM_A2` → `RIFT_SYSTEM_ADDENDUM_TOOLS`; scrubbed α1/α2 phase comments in mod.rs + mcp_server.rs + assistant.svelte.ts headers.

### Key Decisions
- Auto-import task lists into dock (option C) over manual pin button (option A) — user wanted full automation, not a click.
- Activity dock owns tool drill-down entirely; chat stays clean. Inspecting input/output = expand op card in dock.
- Dock auto-open: first TodoWrite OR first MCP tool. Manual close after = respected (same `dockAutoOpenedThisConvo` flag).

### Flagged for deletion (NOT deleted — confirm before removing)
- `src/lib/components/assistant/ToolCallCard.svelte` — zero imports remaining (replaced by inline op-card in `TasksDock.svelte`).

### Next Steps
1. **Cut v0.2.56-alpha** — bump THREE files (package.json + Cargo.toml + tauri.conf.json), write CHANGELOG entry covering Sessions 60-63 arc, run `/check` + `/test` + `/quick-review`, user invokes `/git-ship`.
2. Confirm + delete `ToolCallCard.svelte` after user approval.

### Files Modified
- `src/lib/components/assistant/Markdown.svelte` (auto-import task lists, tasks-migrated pill, $effect sync)
- `src/lib/components/assistant/TasksDock.svelte` (Activity section w/ op cards, elapsed timer)
- `src/lib/components/assistant/MessageBubble.svelte` (removed chip + tool-summary + drill-down)
- `src/lib/components/assistant/Composer.svelte` (stacked icon crossfade)
- `src/lib/components/assistant/AssistantPage.svelte` (dock slide opacity + min-width)
- `src/lib/components/assistant/ToolCallCard.svelte` (error flash + status-in animation — flagged dead)
- `src/lib/state/assistant.svelte.ts` (pinTasksFromChecklist, activity state slimmed, dock auto-open on tool)
- `src-tauri/src/assistant/mod.rs`, `mcp_server.rs` (phase-neutral comments + addendum rename)

---

## Session 62 — 2026-05-14 — Assistant UI overhaul + markdown renderer SHIPPED

**Status: All checks clean. v0.2.56-alpha not yet cut. Compact mid-session — markdown layout work continues after.**

### Completed
- **Tool name fix** — MCP tools renamed `rift__read_file→read_file` etc. so CLI tool IDs (`mcp__rift__read_file`) match the `--allowed-tools` allowlist. ToolSearch artifact filtered from parser.
- **Alpha→Beta rebrand** — `α2` chip replaced with `BETA` (warn-yellow). System addendum reworded to phase-neutral language.
- **Batch A — Assistant UI shell** — new components: `AssistantHeader` (model pill, dock toggle, kebab menu), `Composer` (autosize, send→stop morph), `MessageBubble` (avatar gutter, icon-only, copy btn), `EmptyState` (sparkle hero + 4 suggestion cards), `TasksDock` stub. `AssistantPage` thinned to compositor.
- **Batch B — Tasks dock wiring** — `TodoWrite` added to `--allowed-tools`; system addendum instructs Claude to use it for ≥3-step work. Parser intercepts `TodoWrite` tool_use → routes to `assistant.tasks[]` (not inline card). Dock auto-opens on first task emit, pulses header chip on subsequent updates. `clear()` resets tasks + arms auto-open.
- **Markdown renderer** — `Markdown.svelte` (new): `marked` + `marked-alert` + `DOMPurify`. Full support: headings (h1-h6 with distinct scale), bold/italic/del, inline code, fenced code blocks, lists, tables (GFM, zebra), blockquotes, HR, links (routed via `plugin-opener` to OS browser), task lists, `<kbd>`, `<details>/<summary>`, `<mark>`, GitHub-flavored alerts (NOTE/TIP/IMPORTANT/WARNING/CAUTION — slim inline row layout, colored label chip + content beside it), diff blocks (line numbers, old/new gutter, +/- tinted rows, hunk separator).
- **Heading scale** — h1=20px/h2=16px/h3=14px/h4=13px/h5-h6=12px uppercase spaced.
- **Alert redesign** — from bulky tinted boxes to slim inline-row (label chip left, body right, 6px padding).
- **Diff blocks** — 3-column grid (36px old# / 36px new# / code), 2px colored left rail, gutter rail, hunk separator divider between hunks, overflow-x scrollable.
- `marked`, `marked-alert`, `dompurify`, `@types/dompurify` added to package.json.

### Key Decisions
- Dock default closed; auto-opens on first TodoWrite emit per convo; user close respected after.
- Links intercepted at `.md` wrapper click handler → `openUrl()` to avoid webview inline navigation.
- Diff line numbers parsed from `@@ -A,B +C,D @@` hunk headers; counters maintained across lines.

### Next Steps (resume after compact)
1. **Collapsible `<details>`** — prompt test + verify CSS wired
2. **Task list checkboxes** — `- [x]` GFM prompt test
3. **`<kbd>` key caps** — prompt test
4. **Batch C animation polish** — tool card status micro-animations, message fly-in, panel slide cleanup
5. **Cut v0.2.56-alpha** — bump THREE files, changelog, `/git-ship`

### Files Modified
- New: `src/lib/components/assistant/` — `AssistantHeader`, `Composer`, `MessageBubble`, `EmptyState`, `TasksDock`, `Markdown` (all `.svelte`)
- Modified: `src/lib/components/assistant/AssistantPage.svelte`, `ToolCallCard.svelte`
- Modified: `src/lib/state/assistant.svelte.ts` (+`composerDraft`, +`ui`, +`tasks`, TodoWrite intercept)
- Modified: `src-tauri/src/assistant/mod.rs` (TodoWrite allowlist, system addendum, tool names)
- Modified: `src-tauri/src/assistant/mcp_server.rs` (tool name rename read_file/list_dir/grep)
- Modified: `package.json` (marked, marked-alert, dompurify, @types/dompurify)

---

## Session 61 — 2026-05-14 — Assistant page α2 SHIPPED (MCP server + read-only tools)

**Status: α2 functional, both checks clean. Manual smoke-test pending. No version bump yet.**

### Completed
- **MCP server** — `src-tauri/src/assistant/mcp_server.rs` (new, ~330 lines): stdio JSON-RPC 2.0 endpoint. Implements `initialize` / `tools/list` / `tools/call`. Three tools: `rift__read_file` (≤500 KB UTF-8), `rift__list_dir` (≤500 entries), `rift__grep` (walkdir+regex, ≤200 matches, glob support, skips node_modules/.git/build/dist/target/binaries). Path safety: all paths canonicalized + checked against `RIFT_MCP_ROOTS` env before access.
- **Self-exec MCP mode** — `lib.rs::run()` checks `RIFT_MCP_SERVER=1` env immediately after Velopack → runs `mcp_server::run_stdio()` and returns. No Tauri loop in this mode.
- **assistant_send updated** — queries `AutoSyncState` for active workspace roots → provisions `~/.rift/assistant/mcp-config.json` pointing CLI at `current_exe()` w/ env `RIFT_MCP_SERVER=1` + `RIFT_MCP_ROOTS`. Spawns CLI w/ `--mcp-config` + `--allowed-tools mcp__rift__*`. No workspace → falls back to no-tools turn w/ updated addendum.
- **System addendum split** — `RIFT_SYSTEM_ADDENDUM_A2` (describes 3 tools + α2 scope) and `RIFT_SYSTEM_ADDENDUM_NO_WS` (no workspace connected, direct user to Sync). Both single-line for .cmd-shim batch-arg compat.
- **Frontend block model** — `ChatMessage.blocks: Block[]` replaces `text: string`. Blocks are `{type:"text"}` or `{type:"tool"}`. Parser handles `tool_use` from `assistant` envelopes + `tool_result` from `user` envelopes matched by id.
- **ToolCallCard.svelte** (new) — collapsed-by-default card: per-tool icon (FileText/FolderTree/Search), one-liner summary, spinner-while-pending, expand shows input JSON + output. Status tones: running/ok/error.
- `regex = "1"` added to Cargo.toml. Checks: `svelte-check` 0 errors, `cargo check` clean.

### Key Decisions
- Self-exec subprocess over HTTP MCP server — avoids HTTP server framework dep, shares nothing, passes workspace roots via env. Clean enough for α2 scope.
- Tool blocks stripped from history replay — CLI handles tool protocol internally per-turn; only text blocks feed `build_prompt()`.

### Next Steps
1. **Manual smoke-test**: connect to Trey's server (or local resource), ask Claude "what files are in src-tauri/src/assistant?" — should see ToolCallCards with list_dir + read_file results.
2. **α3**: write tools (`rift__write_file`, `rift__edit_file`) + Apply/Reject diff gate in chat. Consider staging area.
3. **Cut v0.2.56-alpha** once α2 smoke-test passes — bump THREE files (package.json + Cargo.toml + tauri.conf.json).

### Files Modified
- New: `src-tauri/src/assistant/mcp_server.rs`, `src/lib/components/assistant/ToolCallCard.svelte`
- Modified: `src-tauri/src/assistant/mod.rs` (mcp_server pub mod + write_mcp_config + assistant_send rework + dual addenda), `src-tauri/src/lib.rs` (MCP branch), `src-tauri/Cargo.toml` (regex dep), `src/lib/state/assistant.svelte.ts` (block model), `src/lib/components/assistant/AssistantPage.svelte` (block rendering + α2 badge)

---

## Session 60 — 2026-05-14 — Assistant page α1 SHIPPED (planning + execution)

**Status: α1 functional, manual smoke-test pending. No version bump yet — stacking α2 before cutting v0.2.56-alpha.**

### Completed
- **Full design brief locked** — `docs/design/assistant-page.md` (248 lines). Architecture, auth model, tool routing, working directory, visibility surfaces, persistence schema, failures, phasing all documented + research-verified.
- **Auth model settled** — piggyback on user's existing `claude login` CLI session (primary); API-key fallback (secondary). No OAuth in Rift — policy-blocked for distributable apps (Anthropic April 2026+).
- **Tool routing locked** — all SDK built-ins disabled; Rift MCP server (α2+) is sole tool source, routes everything through existing safety stack (path_guard, circuit-breaker, Mirror gate).
- **α1 shipped** — `src-tauri/src/assistant/mod.rs` (new): `assistant_auth_probe`, `assistant_get/set_api_key`, `assistant_send`. Rust shells out to `claude` CLI via `--output-format stream-json`, streams NDJSON as `assistant://stream` + `assistant://done/error` Tauri events. Windows PATH fix via `where.exe` + `OnceLock`. `@anthropic-ai/claude-agent-sdk` NOT installed (Node-only, incompatible with webview).
- **Frontend** — `src/lib/state/assistant.svelte.ts`, `src/lib/components/assistant/AssistantPage.svelte` (green/yellow/red pill, message bubbles, composer, cost display). Tab at Ctrl+4 in TabRail + AppShell under v0.2.55 keep-alive pattern.
- **α1 gaps closed** — multi-turn context via `build_prompt()` history chain; `RIFT_SYSTEM_ADDENDUM` via `--append-system-prompt` (Claude states α1 limitations honestly); cost row hidden on OAuth path.

### Key Decisions
- Rust-CLI-spawn over Node Agent SDK — SDK is Node-only, can't run in Tauri webview. Discovered + corrected before install.
- `--append-system-prompt` not `--system-prompt` — preserves CLI's default CLAUDE.md piggyback context.
- Plaintext `~/.rift/assistant/config.json` for API key in α1, keychain migration deferred to α2.

### Failed / Don't Retry
- Agent SDK in Svelte renderer — Node-only (`child_process`, `fs`), incompatible with Tauri webview. Brief patched with "REJECTED on implementation" block.
- Claude.ai OAuth button — Anthropic policy blocks third-party distributable apps from bridging claude.ai login (actively enforced April 2026+).

### Hooks & Gotchas
- `Command::new("claude")` on Windows skips PATHEXT → claude.cmd not found. Fix: `resolve_claude_exe()` via `where.exe` + `OnceLock` cache in `mod.rs`.
- CC harness "Unhandled case: [object Object]" crashes on long thinking + complex tool-call combos. Mitigation: break work into smaller single-gap turns.

### Next Steps
1. Run four manual smoke tests (multi-turn memory, capability honesty, cost-row hidden on OAuth, conversation builds context).
2. α2: MCP server scaffold in `src-tauri/src/assistant/mcp_server.rs` + read-only tools (`rift__read_file`, `rift__list_dir`, `rift__grep`) — Claude can answer codebase questions, zero write risk.
3. α3: write tools + Apply/Reject diff gate.
4. Cut v0.2.56-alpha once α2 or α3 feels shippable.

### Files Modified
- New: `src-tauri/src/assistant/mod.rs`, `src/lib/state/assistant.svelte.ts`, `src/lib/components/assistant/AssistantPage.svelte`, `docs/design/assistant-page.md`
- Modified: `src-tauri/src/lib.rs:8,1745-1748`, `src/lib/components/shell/TabRail.svelte`, `src/lib/components/AppShell.svelte`, `src/lib/components/settings/Settings.svelte`, `docs/HANDOFF.md`

---

## Session 59 — 2026-05-14 — v0.2.55-alpha Sync overhaul SHIPPED

**Status: SHIPPED on main, Velopack tag `v0.2.55-alpha` live.**

### Completed
- **Pull/Push rescan-after-dispatch** — `pullAll`/`pushAll`/`applySelected`/`confirmMirrorApply` chain to `rescan()` not `refresh()`. Pushes no longer hidden after Pull all completes.
- **Auto-scan on first connect** — `AppShell` $effect watches `connection.status.state` → fires `syncPage.maybeAutoScan(key)` once per server-key when watcher ready. Latch cleared on disconnect.
- **One-button Sync (pull then push)** — replaced separate Pull/Push buttons w/ single primary `Sync (N↓ M↑)`. Sequences `sync_pull_pending` → 2.5s drain → `sync_push_pending` → 1.2s → rescan. Phase labels live (`Pulling…` / `Pushing…`). Conflicts + Mirror remote-deletes stay gated.
- **Auto-rescan periodic** — kebab cycle `off→30s→1m→2m→5m→10m→off`, localStorage-persisted. Timer lives in AppShell (survives tab switches), gated busy/preview/disconnect. `$effect` cleanup tears down on toggle/interval change.
- **Tab-switch flash fix** — dropped `{#key active}` + `in:fly`+`out:fade`. Lazy-mount + keep-alive: each page mounts once on first visit, `hidden` attr toggles visibility. No remount → no transition cascade → no flash. Inner re-keys (`settingsSection`, `selectedConflict`) preserved.
- **Sync page reskin Phase A** — hero compacted to `[⋯][↻][Apply Mirror (cond)][Sync]`. Kebab w/ Mirror toggle / Auto-rescan / Sweep / Advanced (Pull-only, Push-only) / Design preview. Kebab anchored `right: 0` (was `left: 0` → viewport clip).
- **Two-line entry rows** — path + size line 1, reason + relative mtime line 2. `formatSize`/`formatMtimeRel` helpers.
- **Selection breakdown footer** — tone-tinted `2 push · 2 pull · 1 delete` replaces generic hint.
- **Empty-state subtitle + ghost rescan** — Last scan + folder count + `Rescan now` button.
- **Design preview fixture** — kebab toggle injects 9-entry fixture across 3 resources for UI review, dispatch gated.

### Key Decisions
- syncNow uses timing-based drain (2.5s) not event-driven — backend has no hard pull-complete signal yet. Refine later if tight in practice.
- Auto-rescan opt-in default OFF — most users on a single-machine workflow don't need it. Teams w/ remote teammates flip it on.
- Lazy-mount visited Set + `|| active === X` fallback covers same-render-frame race.

### Files Modified
- `src/lib/components/AppShell.svelte`, `src/lib/components/sync/SyncPage.svelte`
- `src/lib/state/sync-page.svelte.ts`
- Version bump THREE files: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` → 0.2.55-alpha

### Next Steps
1. Test installed build: tab switches (no flash), auto-rescan cycling, Sync button mid-flight, Preview toggle.
2. v0.2.56 queue: connecting-pill desync (item h), EACCES auto-fix (item a), dry-run Mirror preview (item d), integration tests (item c), lib.rs split (item e).

---

## Session 58 — 2026-05-14 — Terminal UI overhaul (shipped via S59 batch)

Terminal: borderless-max clip fix, window-resize reflow, HMR tab-explosion fix, Settings → Terminal panel (font/cursor/scrollback/bell/themes), `@xterm/addon-search` (Ctrl+F), QoL (inline rename, file-drop paste, Ctrl+Shift+[/]/T, clear button). Custom Rift-themed dropdowns + slider replace native `<select>`. Full detail: `git log -- docs/HANDOFF.md`.

---

## Session 57 — 2026-05-13 — v0.2.54-alpha SHIPPED — see `git log -- docs/HANDOFF.md`

Trey onboarding hotfix: fresh-install bootstrap (`try_watch` now `mkdir_all`s missing subdirs) + titlebar dropdown clip. Trey's profile: Tailscale host `100.122.178.19`, user `treyday`, remoteRoot `/opt/fxserver/server/txData/Qbox_F8F761.base/resources`.

## RESUME HERE — first read every new session

**Project:** rift-tauri. Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.55-alpha** SHIPPED. v0.2.56-alpha in progress (not yet cut). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**🎯 Assistant page work complete through Session 66.** Full feature set: conversation history, slash commands (9 in menu + `/clear` alias), MCP workspace tools, TodoWrite dock, auto-scroll, message queue, markdown renderer, state-aware EmptyState, VSCode-style folder picker (decoupled from FiveM Sync), model picker with version+ctx, clean header. Both `cargo check` + `npm run check` 0 errors 0 warnings. **Immediate next:** cut v0.2.56-alpha — bump THREE files, CHANGELOG S60-66 arc, user runs `/git-ship`.

**v0.2.56 queue (carried from v0.2.55 minus shipped items):** (a) Rift-side EACCES auto-fix-perms affordance — detect "Permission denied" on create-tmp and surface a "Fix prod perms?" button that runs chown+chmod via existing SSH session; (b) auto-Mirror on detected rename only (when notify pairs `Name(From)+Name(To)` w/ matching basenames within debounce window, silent remote-delete; mysterious local-missing still requires typed confirm); (c) integration test suite phase 1 (10 mock-SFTP scenarios — needs SftpClient trait abstraction or testcontainers); (d) Dry-run Mirror preview pre-confirm; (e) `lib.rs` split (1747 L, 52 commands) — needs per-domain `commands/*.rs` design; (f) `reqwest` + `ureq` consolidation — blocked on velopack 0.0.1298's sync `UpdateSource`; (g) LocalPane/RemotePane shared-logic extraction; (h) connection.connecting pill desync from status.state — pill stuck "Connecting" while engine reports `watching`; add derived guard so `state in {watching,idle,syncing}` overrides `connecting`.

**Multi-user warning.** Trey: keep him OFF Mirror until he's on latest + fresh-Pulled baseline. v0.2.55 introduces auto-rescan (off by default) — safe for him to receive.

**Don't reintroduce:** OpRail, TopBar (merged), rail kbd hints, StatusBar ⌘K pip, titlebar gear, StatusHero big H1, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn`/`.vdivider` dead CSS, `bg-backlog.sh`, `diag_*` cmd names, `drift_watcher::spawn`/`run_tick`/`flush_cycle`.

---

## CRITICAL DON'T-TOUCH

- russh `ring` backend + reqwest `rustls` only (NASM blocks aws-lc-rs).
- `~/.rift/*.json` compat — don't change rename rules; keep `serde(flatten) extra`.
- `VelopackApp::build().run()` MUST be first call in `lib.rs::run()`.
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver).
- DriftWatcher conflict-rename guard — never overwrite dirty local.
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it.
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo.
- `path_guard.rs` API frozen — `edit/in_place.rs` + lib cmds depend.
- `rename_via` strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- Source `.secrets/env.sh` first on ship/auth tasks.
- `last_scan_entries` is `std::sync::Mutex` (NOT tokio) — notify handler context.
- `force_pull_now`/`force_push_now` invariants preserved (v0.2.43).
- **NEVER `FileAttributes::default()` for SETSTAT** — sends zeros → truncation + epoch mtime. Use `empty()`.
- `SftpClient::delete` routes by remote stat — dirs go through `delete_recursive_via`.
- `mkdir_p_via` chmods each segment to 2775 for shared-group pushes.
- Upload pre-flight SHA-collapse before raising CONFLICT (v0.2.32).
- `DriftBucket::ToDelete` = local+no-remote+has-baseline → delete LOCAL. `DriftBucket::ToDeleteRemote` (v0.2.53) = local-missing+remote-has+has-baseline + mirror-on → delete REMOTE.
- Time displays MUST pass `[], { hour12: true }` — locale-default emits 24h on non-US.
- Mass local-delete circuit breaker: `(file_count * 0.30).clamp(5, 25)`. Sync-page explicit-user-selection bypasses. ToDeleteRemote bypasses (user reached via typed-MIRROR gate).
- `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- russh `Config { keepalive_interval: 20s, keepalive_max: 3, window_size: 2 MiB, maximum_packet_size: 32 KiB }` in both `sftp::open_session` + `tunnel::start`.
- **v0.2.46+ data-integrity stack — DO NOT REGRESS:** `mkdir_p_strict_via`, batch pre-mkdir in `flush_batch`, lock release on every `process_entry` terminal path, `path.is_file()` gate in `queue_path`, `wait_for_readable` 6×exp-backoff.
- **v0.2.48 ignore symmetry — DO NOT REGRESS:** `ignored_directory_names()` excludes `build`+`dist` for FiveM `web/build/`+`web/dist/` ui_page bundles.
- **v0.2.48 Created+Dir debounce — DO NOT REGRESS:** 500 ms + `pending_dir_reconcile: AtomicBool` coalesce.
- **v0.2.50 connection-reliability stack — DO NOT REGRESS:** `sftp/transfer.rs::with_t` op timeouts (T_QUICK 10s / T_NORMAL 30s / T_BODY 120s) on every SFTP op + LIST_T 120s on listing; `ConnectionWedged` diag emit on timeout; `process_entry` terminal lock-release is INLINE await w/ 5s timeout (NOT `tokio::spawn`); `sync/ignore.rs` `.tmp.<pid>.<hex>` rule tight-matched (pid ≤8 digits, hash ≥8 hex, no 3rd dot-seg); `sync_sweep_stale_locks` ONLY clears own-user locks via `LockPresence::sweep_stale_mine`.
- **v0.2.52 watcher + state-machine — DO NOT REGRESS:** explicit `Modify(ModifyKind::Name(RenameMode::From))→Deleted` + `RenameMode::To→Created` arms (Windows notify never emits `RenameMode::Both`); `consecutive_failed_batches` threshold 3 before Error escalation (single fails stay `Watching` w/ retry-pending detail); 5s watched-root-vanish poll w/ de-dup HashSet for issue #403.
- **v0.2.53 Mirror + auto-reconnect — DO NOT REGRESS:** Mirror mode is session-scoped (`mirror_mode: AtomicBool`), resets on engine restart by design — don't persist. UI typed-confirm gate requires literal "MIRROR" before Confirm enables. Auto-reconnect rolling-window threshold = 3 wedges in 60s w/ `reconnecting` guard (no overlap); client-side only — no engine `Arc<SftpClient>` refactor.
