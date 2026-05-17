# rift-tauri — Changelog Archive

> Retired entries from `docs/CHANGELOG.md`. Newest first. Pre-archive history also available via `git log -- docs/CHANGELOG.md`.

## v0.4.0-alpha — 2026-05-17 — Chat tabs + split dock (experimental v0.4)

Two layered features on top of the v0.3 single-canvas shell: browser-style chat tabs at the top of Rift, and a right-side dock that can grow up to half-screen and split horizontally into left + right slots, each carrying its own panel stack. Both ride the existing `uiPrefs.useV03Shell` toggle (Settings → Appearance → Experimental). v0.2 path remains pixel-identical — the toggle is the rollback.

### Chat tabs (Phase 1)

A dedicated 34px row sits below the Titlebar, only when v0.3 is on. Each tab is one Claude conversation; opening many lets you context-switch without losing state. Close keeps the convo in History. `MessageSquare` icon swaps to a pulsing dot while that tab is mid-stream. Tab titles auto-fill from the first user message (40-char cap) — unsaved new tabs show "New chat" until the first send saves them.

`AssistantStore` grows `openTabs: string[]` plus `openTab`/`closeTab`/`newTab`/`reorderTabs`/`cycleTab`/`closeAllTabs`/`closeOthers`/`closeTabsToRight`. Persistence is `localStorage["rift.ui.tabs.v1"] = { openTabs, activeTabId }`. On init, stored tab ids filter against `assistant_list_conversations` — orphan ids drop silently. `send()` now keys "first turn vs resume" off `convoCreatedAt`, not `currentConvoId`, so newTab can mint the id up-front without breaking the CLI's `--session-id` path. Click-to-switch handles unsaved-new-tab targets in-memory instead of disk-loading a record that doesn't exist yet.

Keybinds: `Ctrl+T` new tab · `Ctrl+W` close active · `Ctrl+Tab` / `Ctrl+Shift+Tab` cycle · `Alt+1..9` jump. Drag a tab to reorder (HTML5 DnD + tail-zone for append). Empty state (no tabs) replaces chat + composer with a centered "+ New chat" card hinting at History.

### Split dock (Phase 2)

`PanelState` grows a `slot: "left" | "right"` field. localStorage migration on load — any panel without `slot` defaults to "left", so existing v0.3 users see no visible change. The outer dock width recomputes its max per resize: `Math.min(900, innerWidth - 480)`, reserving a 480px chat minimum. Double-click the outer resize handle to snap to ~50% viewport.

Inside the dock, a CSS grid `[left slot] [4px split-handle] [right slot]` collapses to a single column when the right slot is empty. Drag a panel header across the slot boundary to reassign — the right slot appears as a "Drop here → New right slot" target during left-source drag; an occupied slot shows a soft outline + dragover tint. New `dockSplitPct = $state(50)` drives `--dock-split-pct` on `:root`; the internal 10px split-handle hit area (2px visual via `::after`, RAF-throttled, persist-on-release) follows the same drag-vs-release pattern as the outer width handle. Min/max 20–80%, double-click snaps to 50.

Accordion sweep (`applyOpenState(closeOthers=true)`) restricts to the dragged panel's slot — opening a left-slot panel no longer collapses a right-slot panel. `Ctrl+1..8` still toggles the named panel regardless of which slot it lives in.

### Polish (Phase 3)

Settings → Appearance picks up a Layout sub-card under the v0.3 accordion toggle: "Reset dock split" button (→ 50%), "Close all chat tabs" button, and a kbd cheat sheet. New `scripts/cdp/smoke-v04.sh` runs 23 checks end-to-end (Ctrl+W loop reset → 3-tab open → keyboard cycle → close-middle → cross-slot drag → per-slot accordion → split resize → maximize+Esc → empty-right collapse). 23/23 green.

CDP wrapper (`scripts/cdp/serve.cjs`) — Key dispatch grows on-demand resolution for digits 0-9 + letters a-z; `KEY_DEFS` stays the source of truth for special keys.

### Don't-touch carryovers from v0.3

Registry-based PanelShell mount, `applyOpenState` clearing `maximized` when its panel closes, Terminal lazy-mount fallback, slide-over Settings, S76 dock-resize snappiness (RAF + `setDockWidthLive`/`persistDockWidth` split + no grid transition under v0.3). All v0.2 codepaths preserved verbatim.

### Verify

`svelte-check` clean. `cargo check` clean (no Rust changes this arc). CDP smoke `bash scripts/cdp/smoke-v04.sh` 23/23 PASS. v0.3 toggle OFF renders pixel-identical to v0.2.56-alpha.

## v0.2.57-alpha — 2026-05-17 — Assistant maturity + experimental v0.3 shell

Seven sessions (S69–S75) of layered work. The Assistant gains harness pull-through, native session resume, workspace context, remote-bash, multi-user awareness, streaming polish, and a per-message cost+model badge. An experimental v0.3 single-canvas shell ships flag-gated behind Settings → Appearance → "Experimental v0.3 shell layout" (default OFF — current v0.2 shell unchanged). Both `cargo check` + `npm run check` clean.

### Assistant maturity (S69–S74)

S69 fixed the blank-response bug — Windows `claude.cmd` shim was mangling `--output-format stream-json` arg quoting. Spawn invokes the underlying JS bin directly via `node` to bypass. Same session surfaced extended thinking via `MAX_THINKING_TOKENS=10000` env (only the env var works; `--settings '{"thinking":...}'` + `--permission-mode plan` do not).

S70 shipped CDP autonomous-verify infra. `scripts/cdp/serve.cjs` (port 9223) wraps WebView2's CDP endpoint with a persistent ws. `bash scripts/cdp/c.sh {health|state|eval|type|click|wait|shot|key|shutdown}` drives + observes the running UI without screenshots. ~40-60ms per call. Used for Phase A/B/C verification this arc.

S71 (Phase 1) harness pull-through. `AssistantConfig.use_full_config` default ON drops `--strict-mcp-config` + `--disable-slash-commands` so user MCPs and slash commands layer alongside Rift's. API-key mode forces off via `--bare`. Multi-user `<cwd-hash>` collision resolved (per-user `~/.claude/` isolates).

S72 (Phase 2) native session-id + resume. `--session-id` on turn 1, `--resume` on follow-ups, deleted the hand-rolled history replay. `AssistantConfig.max_budget_usd` + Settings "Per-turn cost cap" — only `--max-budget-usd` shipped, `--max-turns` is not a real CLI flag.

S73 (Phase 3) Rift-native sprint. Per-turn `WorkspaceContext` addendum: live foreign-locks + AutoSync queue + recent DiagBus events spliced onto the system prompt at spawn. `mcp__rift__remote_bash(command, timeout_secs?)` tool exec's over the auto-sync engine's live russh session via a loopback NDJSON bridge (`assistant/remote_bridge.rs`); ~5ms loopback RTT vs ~500ms cold SSH dial. Env-gated by `RIFT_REMOTE_SHELL_ENABLED=1`. Workspace-scoped `<remote_root>/.rift-shell.rift-lock` advisory lock; foreign holders surface as a "trey (4m)" pill in `AssistantHeader`.

S74 (Phase 4) UX polish, seven items: (1) `/tools` slash notice rewritten for full Claude Code parity + conditional remote_bash line; (2) diff view in Edit op-cards — TasksDock swaps raw JSON dump for unified red/green list; (3) per-message cost+model badge — "Sonnet 4.6 · $0.0772" pill in MessageBubble's role-row; (4) @-file mention picker — new `assistant_list_workspace_files` Tauri cmd (SKIP_DIRS mirror, 4000-cap), three-tier fuzzy ranking; (5) code-block copy buttons via `annotateCodeBlocks()`; (6) conversation search in HistoryDrawer; (7) context-aware empty-state — detects FiveM/RedM via `fxmanifest.lua`. Plus streaming pacer (~120 ch/s drip from rAF queue, auto-drain in 400ms) + blinking-caret + soft fade-mask chrome on streaming bubbles, and a dedicated `WEBVIEW2_USER_DATA_FOLDER` for dev so it doesn't collide with the installed Rift's lock.

### Experimental v0.3 shell — flag-gated (S75)

Twenty-three commits, all behind `uiPrefs.useV03Shell`. Flag-off path renders pixel-identical to v0.2 — zero regression risk.

Flag-on: chat is the permanent center, every other tool lives in a right-side dock. Eight panels (Tasks, Sync, Files, History, Agents stub, Terminal, Attachments stub, Activity Feed) wrap the existing v0.2 surfaces via a registry-based `PanelShell` primitive. First-launch preset picker offers Minimal (Tasks + History), Standard (5 panels), or Power (all 8). Settings becomes a slide-over modal (Esc + X dismiss). Panel headers carry optional reactive count pips (Sync conflicts in danger-tone, Activity events + Tasks + History in info-tone). Drag a panel header to reorder. Drag the dock's left edge to resize 280–560px. Accordion mode on by default (one panel open at a time; shift-click or Ctrl+Shift+N bypasses). Maximize-to-center: click ⛶ on any panel header and that panel takes over `<main class="pane">` while chat hides — Files + Sync use this for their drift-table / file-browser views via compact summary cards in the dock + a "View … in center" button. Terminal auto-maximizes on first open (xterm at 320px dock width is unusable). Esc restores chat. `applyOpenState` clears the maximized cursor if its panel is closed via rail.

Architecture note: PanelShell instantiates `def.component` from the PANELS registry directly — not slot-based. Wrappers ARE the bodies. PanelDef carries optional `getCount` / `getTone`. All v0.2 codepaths preserved verbatim under the `useV03Shell` branch.

Toggle: Settings → Appearance → "Experimental v0.3 shell layout." Restart required (some mount-time reads). Both `npm run check` + `cargo check` green at every commit.

### Verify

`svelte-check` 0 errors. `cargo check` clean. CDP smoke-pass per phase. v0.3 flag-off renders pixel-identical to v0.2.56-alpha.

## v0.2.56-alpha — 2026-05-15 — AI Assistant + full UI consistency rework

The big one. Nine sessions (S60-68) of work covering Rift's biggest identity change since v0.2.0: an in-app **AI Assistant** that lets you talk to Claude against an open project folder, plus a top-to-bottom UI consistency pass that re-shaped every page around a canonical skeleton.

Assistant tab (Ctrl+3, BETA chip) auth-piggybacks on the user's `claude` CLI session; API-key fallback for pay-per-token. Rift ships a stdio MCP server inside its own binary; CLI spawns w/ `--mcp-config` pointing back at itself + `--allowed-tools mcp__rift__*`. Three read-only tools (`read_file` ≤500KB, `list_dir` ≤500 entries, `grep` walkdir+regex ≤200 matches), all paths canonicalized + checked against `RIFT_MCP_ROOTS`. Plus `TodoWrite`. Workspace decoupled from FiveM Sync — VSCode-style "Open Folder," works on any stack. Chat surface: AssistantHeader, Composer (autosize, send→stop morph, slash menu w/ 9 cmds, ↑/↓ recall), MessageBubble (avatar gutter, copy btn), state-aware EmptyState, TasksDock (auto-opens on first TodoWrite/MCP tool call), HistoryDrawer (rename + two-step delete). Markdown via marked+marked-alert+DOMPurify, full GFM. Real stop button via taskkill/kill -TERM tracking child PID. Auto-scroll respects user intent (stickToBottom flag).

UI consistency: four new shell primitives (PageHeader, PageToolbar, PageFooter, EmptyState). Five pages converted to canonical skeleton (Conflicts, Activity, Files, Sync, Assistant). Titlebar declutter (connection pill folded into server-picker dot). StatusBar simplified. TabRail rework (groups + hairline dividers + active-tone glow + pin button + container query collapsing). Files tab drag-reorder via pointer events + animate:flip. Sync shrink-banner collapsible. About page Paths + Diagnostics sections w/ privacy scrub on copy.

Verify: svelte-check 0 errors across 4020 files, cargo check clean, privacy audit confirmed standalone.

## v0.2.55-alpha — 2026-05-14 — Sync page overhaul: one-button Sync, auto-rescan, keep-alive tabs

A focused UX pass on the Sync page — the most-used screen after Browser. Two longstanding annoyances (Pull-then-Push needing two clicks; pushes hidden after Pull all completes) are gone, drift now auto-rescans on first connect + on a user-settable interval, and tab switches lost their flash.

### One-button Sync (pull then push)

Replaced separate `Pull all` + `Push all` buttons in the hero with a single primary `Sync` button. Click sequences `sync_pull_pending` → 2.5 s drain → `sync_push_pending` → 1.2 s → rescan, in that order. Pull-before-push is canonical: it rebases local against remote so push never dispatches against a stale baseline. Button label live-updates `Sync (N↓ M↑)` → `Pulling… (N)` → `Pushing… (M)` so the phase is always visible. Pull-only / Push-only granular controls demoted into the new `⋯` kebab menu under an "Advanced" section. Conflicts stay in the conflict bucket (not auto-resolved); Mirror remote-deletes stay gated behind the typed-confirm modal.

### Rescan-after-dispatch fix

Calling `Pull all` with mixed pull + push drift previously dispatched the pull, then `refresh()` returned an empty drift snapshot (backend clears cached pending entries on dispatch), so the page rendered "Everything in sync" — hiding pushes the user could clearly see two seconds earlier. Now `pullAll` / `pushAll` / `applySelected` / `confirmMirrorApply` all chain to `rescan()` instead of `refresh()`, so the next snapshot is a fresh `sync_reconcile` result with every remaining drift entry intact.

### Auto-scan on first connect

When the watcher transitions to `watching` / `idle` / `syncing` for the first time per server-key per session, the frontend auto-fires a drift `sync_reconcile`. Drops the "open Sync page → click Rescan → wait" first-launch ceremony — drift is already populated by the time the user navigates there. Latch clears on disconnect so reconnect re-fires.

### Auto-rescan (opt-in periodic)

Local watcher only sees local edits; remote drift from teammates pushing is invisible until manual rescan. New auto-rescan toggle in the kebab cycles `off → 30 s → 1 m → 2 m → 5 m → 10 m → off`. Persists to localStorage. Timer lives in `AppShell` (survives tab switches), gates on `enabled + watcher-ready + connected`, skips ticks when busy / loading / in preview. Interval changes tear down + recreate the timer cleanly via `$effect` cleanup.

### Tab-switch flash fix

`AppShell` was wrapping every page in `{#key active}` with `in:fly` (90 ms delay + 180 ms duration) + `out:fade` (90 ms). On every tab switch the active page fully unmounted + remounted, child components re-ran `onMount` (data fetches, listeners), and their own inner `in:fly` / `in:fade` transitions re-fired → cascade pop-in glitch. Now each page mounts once on first visit and stays mounted; `hidden` attribute toggles visibility instantly. Cold-launch unchanged (only Browse mounts initially). Inner re-key for `settingsSection` + `selectedConflict` preserved. Removed unused `fly` / `fade` / `quintOut` imports.

### UI reskin (Phase A)

* Hero compaction: `[⋯] [↻] [Apply Mirror (cond)] [Sync]` — three visible buttons down from seven. Kebab houses Mirror toggle, Auto-rescan, Sweep stale locks, Pull-only, Push-only, Design preview.
* Two-line entry rows: path + size on line 1, reason + relative mtime on line 2. `formatSize` (B/KB/MB/GB) + `formatMtimeRel` (s/m/h/d ago) helpers.
* Selection footer: tone-tinted breakdown (`2 push · 2 pull · 1 delete`) replaces the generic hint when items are selected.
* Empty-state subtitle: `Last scan Xs ago · N folders watched` + ghost `Rescan now` button.
* Design-preview fixture (Eye icon in kebab): injects 9-entry fixture across 3 resources covering every bucket + aborted-shrunk banner, dispatch buttons gated. Lets us screenshot every UI state without needing real drift.

### Verify

`svelte-check` 0 errors / 0 warnings across 3999 files.

## v0.2.54-alpha — 2026-05-13 — Fresh-install bootstrap + titlebar dropdown hotfix

Two onboarding bugs surfaced while bringing a second dev (Trey) on board for the first time. Both block the empty-local → populated-remote first sync path.

### Fresh-install bootstrap (Bug 1)

`auto_sync::try_watch` was silently returning `Ok(false)` when a folder's local subdir didn't exist on disk, leaving the engine with `watches = 0` for a brand-new install. The drift scanner only iterates registered folders, so Rescan returned zero entries → Sync page rendered "Everything in sync" → no way for the user to pull the remote tree down without finding the hidden `Ctrl+K → Bootstrap from remote…` dialog. Now: when the profile's `local_root` exists but a per-folder subdir doesn't, `try_watch` `mkdir_all`s the subdir, logs `"auto-created local folder for first-time bootstrap"` to diagnostics, and attaches the watcher normally. If the profile `local_root` itself is missing (genuine typo / config error) we still bail with `Ok(false)` + a clearer log, never silently mkdir somewhere unexpected. After this, a fresh install with empty local just works: connect → 8 bracket dirs auto-create → drift scan finds remote-only files → Sync page shows ToPull entries → Pull all streams the tree down.

### Titlebar server dropdown clipped (Bug 2)

The titlebar's server-picker dropdown menu opens below the 44 px titlebar row. Its parent `.left` flex container had `overflow: hidden` to constrain text overflow into the drag region, which also clipped the menu vertically — z-index can't escape an overflow-clip ancestor. Moved the overflow constraint from the parent down to the child spans (`.svr-name` / `.svr-host` now use `white-space: nowrap; text-overflow: ellipsis; overflow: hidden`) so long server/host text still truncates cleanly, but the dropdown can render outside the titlebar height. Defensive `z-index: 100 → 1000` on `.svr-menu` too.

### Verify

`cargo check` clean · `cargo test --lib` 46 passed · `svelte-check` 0 errors / 0 warnings across 3996 files.

## v0.2.53-alpha — 2026-05-13 — Mirror mode + auto-reconnect

The two queued safety nets land in one release. Mirror mode gives Rescan a recovery path when watcher events get missed (rare, but happens — e.g. notify-rs Windows issue #403 silently dropping events on a watched-dir delete). Auto-reconnect closes the loop on v0.2.50's `ConnectionWedged` detection: instead of just emitting a diag event and waiting for the user to click Sweep + manually reconnect, the frontend now self-heals after 3+ wedges in a 60 s window.

### Mirror mode (Bug 1)

New `DriftBucket::ToDeleteRemote` variant. When the drift scanner runs with `mirror = true` and sees `l.is_none() && r.is_some() && snap.is_some()`, it now buckets as ToDeleteRemote ("local deleted — removing remote") instead of ToPull ("remote-only — pull"). Normal mode keeps treating this case as ToPull (the safer non-destructive direction). The flag is session-scoped on the engine (`mirror_mode: AtomicBool`) and exposed via two new Tauri commands, `sync_set_mirror_mode(enabled)` and `sync_get_mirror_mode()`. Dispatch lives in `auto_sync::apply_selected`, which routes ToDeleteRemote entries to `sftp.delete(remote_path)` — the SftpClient::delete router already handles dirs through `delete_recursive_via` and files through `remove_file`, so folder deletes propagate cleanly. The mass-delete circuit breaker is intentionally skipped for ToDeleteRemote because the user reached dispatch through the typed-confirm modal — that gate is the consent.

Frontend: a "Mirror" toggle next to Rescan/Sweep on the Sync page (red accent when enabled). Toggling triggers an immediate Rescan so the bucket counts redraw. When entries are in the ToDeleteRemote bucket, a red "Apply Mirror (N)" button appears. Clicking opens a hard-gate modal: count of files to delete, warning copy about irreversibility and multi-user baseline coordination, and a typed-confirm input requiring the literal text "MIRROR" before the Confirm button enables. Backdrop click and Escape both cancel. Backend session-scoped means the toggle resets to off on engine restart — paranoia against accidental destructive ops on a fresh launch.

### Auto-reconnect (v0.2.50 follow-through)

`connection.svelte.ts` now listens to `diag://event` for `stage === "connection_wedged"` emits (these come from `sftp/transfer.rs::with_t` when an SFTP op blows the timeout). A rolling 60 s window holds the timestamps; once 3+ wedges land inside the window, the frontend calls `stop_autosync`, sleeps 1 s for clean teardown, then calls `startAutosyncForSelected()` to re-open the session with the same server + folder spec. A `reconnecting` guard prevents overlapping reconnects. Single wedges still don't reconnect — those usually self-resolve on the next op and aren't worth the session churn. Lives entirely client-side so we don't have to refactor the engine's owned `SftpSession` (which isn't behind a RwLock).

### Verify

`cargo check` clean 5.00 s · `cargo test --lib` 46 passed · `svelte-check` 0 errors / 0 warnings across 3996 files.

### Deferred to v0.2.54

- Integration test suite phase 1 — 10 mock-SFTP scenarios (clean reconcile, local-add, local-delete Normal + Mirror, remote-add, conflict, SuspiciousEmptyAborted, dry-run Mirror, Mirror-disabled-when-shrunk). Requires either an SftpClient trait abstraction for mocking or a testcontainers-based real SFTP server in CI — its own evening.
- Dry-run Mirror preview UI (current modal goes straight to confirm; a "preview rows" pre-confirm step would let the user spot-check before typing MIRROR).
