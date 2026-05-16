# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

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
