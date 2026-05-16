# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.56-alpha — 2026-05-15 — AI Assistant + full UI consistency rework

The big one. Nine sessions (S60-68) of work covering Rift's biggest identity change since v0.2.0: an in-app **AI Assistant** that lets you talk to Claude against an open project folder, plus a top-to-bottom UI consistency pass that re-shaped every page around a canonical skeleton. Both `cargo check` + `npm run check` clean.

### AI Assistant — in-app Claude (S60-66)

A new **Assistant** tab (Ctrl+3, BETA chip in the rail). Auth piggybacks on your existing `claude` CLI session — no separate login. API-key fallback in Settings → Assistant for pay-per-token. No OAuth (Anthropic policy blocks third-party app bridging of claude.ai sessions; we use the CLI session that ships with `claude`).

**Tools (MCP, self-exec):** Rift ships a stdio MCP server inside its own binary; when you send a message, Rift spawns the `claude` CLI with `--mcp-config` pointing back at itself + `--allowed-tools mcp__rift__*`. Three read-only tools: `read_file` (≤500 KB UTF-8), `list_dir` (≤500 entries), `grep` (walkdir + regex, ≤200 matches, skips `node_modules/.git/build/dist/target/binaries`). All paths canonicalized + checked against `RIFT_MCP_ROOTS` env before access. Plus `TodoWrite` for in-chat task tracking.

**Workspace decoupled from FiveM Sync:** Assistant has its own VSCode-style "Open Folder" model independent of your synced server folders. `AssistantConfig` tracks `current_root` + `recent_roots` (MRU, cap 10). Folder picker via `tauri-plugin-dialog` v2. Falls back to AutoSync workspace roots if no folder is explicitly open. Works on any language/framework — the system addendum no longer assumes FiveM.

**Chat surface:** `AssistantHeader` (model pill, dock toggle, +new, history button, auth-warn chip when degraded), `Composer` (autosize, send→stop morph, queue pills, slash menu w/ 9 commands + `/model` sub-picker, model persists to localStorage, ↑/↓ recall over 50-prompt history), `MessageBubble` (avatar gutter, icon-only, copy btn), `EmptyState` (state-aware: no folder = accent CTA card; synced server = green fallback card; folder open = accent folder card), `TasksDock` (auto-opens on first TodoWrite or first MCP tool call, elapsed timer, expandable op cards w/ input JSON + result block), `HistoryDrawer` (slides in from left, per-row rename + two-step delete, JSON-per-convo persistence at `~/.rift/assistant/conversations/`).

**Markdown renderer:** `marked` + `marked-alert` + `DOMPurify`. Full GFM: headings w/ accent left-bars, custom dot bullets, dashed indent guides on nested lists, full-width rounded tables w/ zebra + hover, code-block accent left-stripe, GitHub alerts (slim inline row, not bulky boxes), diff blocks w/ line numbers + +/- tinted rows + hunk separators, `<kbd>` caps, collapsible `<details>`. Task lists auto-extract into the Tasks dock w/ a `📋 Sent to Tasks panel` pill replacing the inline list.

**Real stop button:** Backend tracks the CLI child PID; `assistant_stop` Tauri command dispatches `taskkill /F /T` on Windows / `kill -TERM` on Unix. `USER_STOPPED` atomic distinguishes user-stop (clean `done`) from silent CLI crash (`error`). Frontend Composer button has 3 modes: idle = Send, streaming = Stop (red), streaming + draft = Queue (muted-accent, appends to message queue, auto-drains via `queueMicrotask`).

**Auto-scroll respects user intent:** `stickToBottom` flag tracks whether user is within 80px of the tail. ResizeObserver on the messages container snaps to bottom only when stuck — you can scroll up to read earlier turns while a stream is in flight without being yanked back.

### UI consistency rework (S67-68)

Four new shell primitives: `PageHeader` (46px, tone accent stripe, extras/actions snippets), `PageToolbar` (36px, left/right slots), `PageFooter` (44px, active tint), `EmptyState` (52px glyph circle + title + hint + body slot). All in `src/lib/components/shell/`. Five pages converted to the canonical skeleton: **Conflicts** (new `ConflictsPage` wrapper, double-title killed, icon-rich EmptyState), **Activity** (PageHeader w/ Pause/Clear, segctl moved to PageToolbar, EmptyState w/ Rescan/ClearFilters CTAs), **Files** (renamed from Browser, PageHeader w/ status subtitle + "New tab" action, EmptyState w/ "Add a server" CTA), **Sync** (hero → PageHeader, status pill in extras snippet), **Assistant** (already canonical, only token cleanup). Settings excluded — left-nav IS the chrome.

**Titlebar declutter:** connection pill removed, state folded into the server-picker dot (breathes 2.6s on ok, 1.4s on connecting). One fewer element in the 44px bar.

**StatusBar simplified:** dropped redundant queued / errors / conflicts counts (already surfaced in PageHeader subtitle + TabRail pip). Kept state-toggle, bg-sync pill, locks count.

**TabRail rework:** RIFT wordmark + favicon at top; three groups (workspace: Files/Sync · ai: Assistant · status: Conflicts/Activity) with hairline dividers; Settings bottom-anchored (VSCode convention). Active icon tinted in the tab's tone color + drop-shadow glow (huge "where am I" signal in collapsed 48px state). Container query hides labels + kbd hints + BETA pill in collapsed state. Pin button at top (click chevron) locks rail at 220px and reflows main content via `--rail-w` CSS var; persists in localStorage. Hover-expand still works when unpinned. Assistant tab now carries a `BETA` chip matching the AssistantHeader style. Kbd hints slimmed from bulky `Ctrl 1` chips to a quiet right-edge digit at 55% opacity (tooltip on hover still shows full `Files (Ctrl+1)`).

**Files tab drag-reorder:** click + hold any tab, drag past another → tabs shuffle in real time as the cursor crosses midpoints. Pointer events (HTML5 DnD is unreliable in webview2). `animate:flip` (220ms) slides untouched tabs into new positions. Dragged tab gets `scale(1.04)` + accent ring + soft shadow + `z-index: 2`. Idle hover lifts 1px (only when 2+ tabs exist, via `:has()`). Order persists to localStorage via existing `browserTabs.persist()`.

**Sync shrink-banner collapsible:** "Listing shrink detected" banners now collapse to a one-liner by default (resource name + `142 → 38` count chip + chevron). Click to expand the explainer + Rebaseline/Dismiss actions. Saves vertical space when several brackets fire at once.

**About page fleshed out:** new Paths section (Config dir + Logs dir w/ "Open" buttons via `plugin-opener`); new Diagnostics section (one-click "Copy diagnostic info" → clipboard with version, platform, scrubbed paths, server count + status). Paths section gives users a quick way to reach config + logs without hunting through `%APPDATA%`.

**Privacy scrub on diagnostic copy:** OS username replaced with `<user>` in copied paths (`C:\Users\BLAZZER\...` → `C:\Users\<user>\...`, also handles `/home/<name>` + `/Users/<name>`). Active server name redacted (only "configured" / "none" + state). `navigator.userAgent` dropped entirely (was leaking Windows build + webview2 version) — kept `navigator.platform` which is generic.

### Polish + bug fixes (S68)
* Edit Server dialog subhead: "FiveM dev server" → "dev server" (Assistant decoupling).
* Settings → Assistant API-key copy: dropped `α1`/`α2` phase chips, now phase-neutral.
* Sync empty-state footer: wrapped in `{#if !isEmpty}` so the dead "Apply selected" button no longer renders when everything's synced.
* TabRail kbd hints: `⌘N` → `N` (Mac glyphs were wrong on Windows; tooltip retains full `Ctrl+N`).
* StatusHero deleted — was a Browser-only strip duplicating titlebar info.

### Verify

`svelte-check` 0 errors / 0 warnings across 4020 files. `cargo check` clean. Privacy audit confirmed: no email / real name / IPs / multi-user info in source code, no telemetry, no phone-home. App is fully standalone.

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
