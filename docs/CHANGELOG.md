# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.12-alpha — 2026-05-19 — UI shell redesign + security batch

**Per-tab streaming pipeline + telemetry overhaul + cache discovery (S105/S106).** Moved per-tab streaming state (`streamingMsgId`, `currentTurnRecord`, `deltaCount`, `envelopeTextBuffer`, etc.) off the assistant store onto a `TabState` class so concurrent multi-tab sends stop racing. Compaction Phase A3/A4/A5 + base telemetry + `/diag` slash landed alongside; thinking-block `endThinking` bug fixed and `TurnRecord` expanded with `effortFlag`/`streamEventCount`/`toolUses`/`thinkingBlocks`/`maxStreamGapMs`. New `/stats` slash surfaces per-session telemetry. Cache-bust discovery: Sonnet busts the prompt-cache on effort-flag flip, Opus survives — wiring around that lands in Phase B.

**IPC token strip + TOFU guard + mcp-config cleanup (#9.1, #9.2, #10).** `ServerProfilePublic` DTO no longer crosses the IPC boundary with `bridge_token`; it carries a `hasBridgeToken` boolean instead, and the save path preserves the existing token on empty-edit. `require_pinned_fingerprint` guard inserted at the entry of ~9 IPC commands (scan_drift, start_autosync, open_sftp_for, …) so a profile with no pinned fingerprint can't silently TOFU; the dead `persist_fingerprint_if_new` path is gone. `mcp-config.json` now chmods to 0600 on write, and `cleanup_mcp_config_on_exit` fires through `RunEvent::Exit` so the temp file doesn't linger after a crash.

**Context pill envelope suppression (#1).** Per-turn usage counts now ignore the SDK envelope's `cache_read_input_tokens` and accumulate only `result` events, so the chat-thread context pill stops flickering mid-stream.

**UI shell — Phase 1 (StatusBar + PageHeader sweep).** StatusBar grew pending-queue + failed + conflicts (danger) + last-scan-Xs-ago + bridge pill (gated on `hasBridgeToken && watcherOn`) + app-version chip. `connection.lastScanAt` stamps on `startAutosyncForSelected` resolve + every `drift_scan_result` diag event. PageHeader (46px, tone variants, snippet API) now wraps all eight active workspaces — Chat (BETA chip dropped), Sync, Files, Activity, Conflicts, Diagnostics, Terminal, History — replacing the eight bespoke headers each had grown. Per-page connection badges removed; that state lives in StatusBar now.

**UI shell — Phase 2 (Sync dashboard).** SyncPage's empty hero is gone. When there's no drift, the page renders a three-card dashboard: `WatchedFoldersTable` (rows = engine's watched folders, file count from the cached `FolderCountCache`, last-event + lock count derived client-side); `RecentActivityCard` (last 5 from `connection.activityFeed`, with an "Open Activity" tail link); `DriftSummaryCard` (groups `sync_get_drift_snapshot` by resource, green-check chip when empty). Clicking a folder row sets `connection.activityFilter` + flips to the Activity workspace, which consumes the field on mount. New backend cmd `list_watched_folders` returns `Vec<{name, remote_root, file_count}>` — lock count + last-event time are derived frontend-side from existing stores.

**UI shell — Phase 3 (composer + Settings workspace + tab gutter).** The composer's hint row collapses behind a `(?)` info button (140ms fade + 4px translate-y + scale 0.98→1 pop) so the `Quick` + model pills sit on the textarea row itself, reclaiming ~30px per chat tab. Settings is now a workspace (`Ctrl+,` → `workspace.setActive("settings")`, kbd 9, gear dropped from `ActivityBar.svelte` bottom group); the overlay scrim + slideover machinery in `AppShell.svelte` is gone, dialog callbacks ride a tiny `dialogs.svelte.ts` store the new `SettingsPage.svelte` wrapper consumes. `Ctrl+1..9` now spans the full workspace row. The chat-tab `+` button moved out of the scrollable `.strip` to the right end of `ChatTabsBar.svelte` w/ a 5px gap from the activity-bar boundary; `scrollbar-gutter: stable` on the AssistantPage scroller kills the horizontal jump when overflow appears (#6).

**Console noise + dead-file sweep (#22).** Removed `console.debug` at the S105 cache probe, the S106 envelope-fallback telemetry, and the idle non-JSON stream path in `assistant.svelte.ts`. `stt.svelte.ts` warns at load-config, stop, and recognition error paths downgraded to `console.debug` — none surface user-actionable info. Unused `.empty-icon.ok` / `.empty-sub` / `.empty-action` CSS rules deleted from SyncPage now that the dashboard owns the empty state.

Net diff: 3 new Sync cards + `SettingsPage` workspace wrapper + `dialogs.svelte.ts` store + 1 backend cmd (`list_watched_folders`) + 1 backend helper (`watched_folders_dashboard`). Frontend `npm run check` 0/0/4050. 3-file version bump 0.4.11-alpha → 0.4.12-alpha.

## v0.4.11-alpha — 2026-05-18 — Assistant context + workspace cwd fixes

Three compounding bugs caused the Assistant to read stale/missing context across turns or land its cwd in the wrong workspace folder.

**cwd pinned per session.** Sidecar `~/.rift/assistant/sessions/<uuid>.cwd` written on first turn, overrides root resolution on every subsequent turn. The claude CLI's `--resume <uuid>` only searches the current cwd's `~/.claude/projects/<cwd-hash>/` ([claude-code#35226](https://github.com/anthropics/claude-code/issues/35226) — no fallback). Workspace switches between turns were aiming `--resume` at a different hash dir → session-lost → frontend popped messages, silently restarted. Legacy convos auto-migrate on next resume; sidecar cleaned up on convo delete.

**Per-turn workspace state moved from `--append-system-prompt` → user-turn `<system-reminder>`.** Live AutoSync state (foreign locks, sync queue, recent diag events) was being spliced into the system prompt every turn, busting the prompt-cache prefix every call. Static addendum (tool list, ACT FIRST, dyslexia, remote_shell desc) stays in `--append-system-prompt`; per-turn snapshot rides stdin. Newline-separated since stdin has no argv constraint. Added `--exclude-dynamic-system-prompt-sections` so the CLI's own cwd/env/git/memory-path auto-injection also leaves the cached prefix.

**Common-ancestor cwd when AutoSync supplies >1 root.** FiveM resources auto-discover into one FolderWatch each — `[bracket]` resources sort first in ASCII (`[` = 0x5B) so `[voice]/` became `roots[0]` and the model's cwd landed inside a single resource instead of `<server>/resources/` where every resource is visible. Now compute lexical common ancestor and prepend to `roots`; individual roots stay in the list so MCP path safety is unchanged. Guards: ancestor must share a path beyond fs root, must have a parent, must exist on disk.

Also drops the broken titlebar command-palette button + `CommandPalette` component (S97 "leave it and ship" resolution — unresolved Svelte 5 reactivity bug on `paletteOpen` state, Ctrl+K path also broken). ~327 LOC deleted (palette + titlebar wire-up), ~190 LOC added (assistant fixes). 3-file bump 0.4.10-alpha → 0.4.11-alpha.

## v0.4.10-alpha — 2026-05-18 — Workspace shell

Activity bar now swaps the main pane instead of opening a 320-1200px sidecar. Eight reachable workspaces in default order (Chat · Sync · Files · Conflicts · Diagnostics · Terminal · Activity · History). Agents + Attachments render as disabled "Coming soon" tiles (Phase B). Settings gear at the bottom of the activity bar (was palette / Ctrl+, only). ChatTabsBar mounts only inside the Chat workspace — swapping away hides the strip, swapping back restores it with all tabs intact.

Conflicts + Diagnostics were unreachable from chrome in v0.4.1 (palette-only, and Ctrl+Shift+D routed to Activity, not Diagnostics). Both now have first-class activity-bar entries with their proper components.

Keybindings — Ctrl+1..8 swap workspaces (mapped via the user's activity-bar order so reorders survive); Ctrl+0 returns to Chat (was "close right pane"); Ctrl+\` switches to Terminal workspace; Ctrl+Shift+D goes to Diagnostics; chat-tab keybinds (Ctrl+T/W/Tab, Alt+1..9) gated on `workspace.activeId === "chat"` so they don't hijack from a focused Terminal / Files surface.

Dropped: v0.2 tab-rail shell, `useV03Shell` toggle, RightPane sidecar + 200px width-resize machinery, panel-types/right-pane state, 5 right-pane wrapper components, 2 stub components, smoke-v04-1.sh. ~956 LOC deleted, ~150 LOC added net. localStorage `rift.ui.right-pane.v1` migrates to `rift.ui.workspace.v1`; legacy keys swept on first launch (idempotent — safe to re-run on every boot).

Verified end-to-end by [`scripts/cdp/smoke-v04-10.sh`](../scripts/cdp/smoke-v04-10.sh) — DOM-level assertions for shell shape, activity-bar order, disabled-stub semantics, workspace-swap, ChatTabsBar gating, settings modal, keybindings, and localStorage migration. 3-file version bump 0.4.9-alpha → 0.4.10-alpha.

