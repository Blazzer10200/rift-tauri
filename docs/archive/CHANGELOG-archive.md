# rift-tauri — Changelog Archive

> Retired entries from `docs/CHANGELOG.md`. Newest first. Pre-archive history also available via `git log -- docs/CHANGELOG.md`.


## v0.4.17-alpha — 2026-05-20 — S120 Wave-2 backend MED + LOW sweep (~40 issues)

**Sync engine (14).** `scan_drift` now takes `AutoSyncState` and reuses the running engine's `SftpClient` + `SyncSnapshot` when `profile_key()` matches — eliminates the second concurrent SSH session that pressured `MaxSessions` during ad-hoc scans; cold path still connects fresh (#54). `delete_server` is async, stops the matching engine + tunnel before removing from disk, so deleting an active server no longer orphans watchers / locks / SFTP (#56). New `canonicalize_owned_path` helper extracted from `validate_watched_local_path`; `resolve_conflicts_bulk` uses it inline so per-row failure surfaces the real reason instead of silent `false` (#55). `drift_scanner::scan_folder` takes `cancel: Option<&CancellationToken>`; per-entry check short-circuits the hash-budget loop so cancel actually interrupts SFTP hash calls mid-folder (#73). First-scan size-equal + mtime-mismatch + content-unverified buckets as `Conflict` instead of arbitrary mtime-newer-wins — re-extracted/rsync'd identical copies no longer silently push/pull (#75). `LockPresence` got `pending_locks` for in-flight acquire reservation; `my_locks.insert` only on upload success (#77). `atomic_write_json` tmp name now `<basename>.<pid>-<counter>.json.tmp` so flush + rebaseline can't truncate each other (#80). `SftpClient::close` drains `Mutex<Vec<Arc<Worker>>>` via `mem::take` (#83). `delete_recursive_via` wraps every SFTP op in a file-local `ops_with_t` mirroring transfer.rs discipline (#86). `rename_via` docs the known TOCTOU + points at `rename_overwriting_via` for intent-clarity (#84). `enqueue_for_flush_batch` dropped its bogus `async`; both callers updated (#91). MCP call-path gates `sync_status` / `remote_bash` arms via `bridge_enabled()` / `remote_shell_enabled()` matching the list-path (#72). MCP `run_stdio` returns on response-serialize fail instead of hanging the client (#70). MCP parse-error attempts a minimal `{id}` probe then emits `-32700` when id derivable (#68). `sync::ignore::classify` segment loop rewritten as positive-flow guard (#79).

**Concurrency / lifecycle (9).** `rebaseline_folder` bails on disposed engine (#97). `process_entry_body` no longer fabricates a `(0, Utc::now())` ConflictRecord on vanished local — logs + returns `Fail` (#98). `resolve_conflict::AcceptRemote` re-inserts conflict on download fail; `ForceLocal` kicks `drift_reconcile` post-enqueue (#94, #95). `set_mirror_mode` returns the new value, killing the TOCTOU read-back (#105). `apply_selected` emits a closing `DriftScanResult` w/ dispatched counts so the Sync modal spinner actually closes (#136). `recently_written` map sweeps every 5s on the root-vanish poll (entries >10s old evicted) — long-session leak gone (#101). `lock_presence::stop` aborts the cleanup task on its 2s timeout via `abort_handle` (#123). `try_read_lock` cleans scratch dir on all return paths (#122).

**Bootstrap / assistant / MCP (10).** `bootstrap::classify` adds small-sample `BadRemoteRoot` fallback (no `[bracketed]` + `count < 3`) + sets `missing_count = remote_count` in that branch (#110, #111). `assistant_delete_conversation` loads the convo first and cleans BOTH `<convo_id>` AND `<cli_session_id>` cwd sidecars post-delete (#114). Attachment cap strips ASCII whitespace before `len/4*3` so pasted base64 w/ CRLF doesn't false-trip (#116). `assistant_send` fails + kills child when `child.stdin.is_none()` instead of hanging (#117). `convo_path` / `session_cwd_path` cap id at 64 chars (#132). `common_ancestor` logs a warn when the lexical path isn't a directory so the `roots[0]` fallback is visible (#133). `walk_local` splits ignore probes by entry kind — dirs get bare-name (early prune), files get rel-path only (#137). `remote_bridge::start` race-loss uses `cloned().ok_or_else(...)` instead of `.unwrap()` (#118). `suppress_local_delete_uploads` window raised 2→5s w/ `SUPPRESS_WINDOW_SECS` const (#93).

**Hygiene (7).** `lock_presence::poll_once` honors `STALE_DELETE_MAX_FAILS` for foreign stale locks (#121). `register_conflict` re-stats local at decision time (#124). `safe_profile_key` allows `.` + warns on sanitization (#126). `atomic_write_json` cleans tmp on inner write/sync fail via closure-wrap (#128). `sftp::list` exec fast-path errors `log::debug!` instead of silent degrade (#130). `drift_scanner` got an invariant doc on `count_under` files-only contract for the suspicious-shrink guard (#138).

**Verify:** `cargo check` clean (1 pre-existing `private_interfaces` warning in `update_service.rs:199`, unrelated). `npm run check` clean. Release pipeline runs full `npm run tauri build`.
## v0.4.16-alpha — 2026-05-20 — S119 audit-batch sweep (26 issues)

**Sync correctness (6).** `apply_selected` push now registers a cancel token in `current_scan_cancel` and passes it into `flush_all_now` so the modal Cancel button actually halts mid-flight (#47). `flush_batch` count-delta moved per-entry into `process_entry`, keyed off the real `EntryResult::Ok` outcome + entry kind, so circuit-breaker drops + per-entry cancels no longer leak phantom counts into the cache (#49). `process_entry`'s outer `tokio::select!` arms were swapped — `r = work => r` first, `_ = ct.cancelled()` second — so a completed upload doesn't get silently dropped + re-queued when cancel and work resolve in the same poll (#50). `walk_local_rebaseline` now probes the rel-path with trailing slash before recursing so segment rules (`/.git/`, `/node_modules/`, `/[disabled]/`) fire on dirs and the walker skips them entirely instead of wasting recursion (#51). `apply_selected` ToDeleteRemote failure branch now also calls `snapshot.forget` to prevent a spurious local-delete row appearing next scan when the remote was already gone (#52). #42 (conflict copy re-entry) verified non-bug after re-reading `ignore::classify` — closed w/o code change.

**I/O & handle hygiene (11).** `scan_drift` closes its russh session if `SyncSnapshot::new` fails (#53). `expand_download_jobs` now returns `Result<…, String>` so `list_recursive` errors propagate instead of being swallowed as empty (#58). Both `download_paths` and `upload_paths` re-validate every expanded `(remote, local)` pair through `validate_remote_child` + `validate_local_child` after expansion, closing the symlink-escape gap on both transfer directions (#59 + #60). `delete_local_one` empty-dir cleanup now floors at the resource's `local_root` and bails the moment `cur` matches or escapes the floor (#76). FiveM `web/build` / `web/dist` ignore-bypass now also matches the bare-dir form (`ends_with`) so a listing entry without trailing slash isn't misreported as ignored (#78). `SyncSnapshot::set` and `forget` log save-failures via `log::error!` instead of `let _ = save_locked` (#81, partial — sig kept `()`). `heal_owned_dirs` drains its russh channel to close rather than breaking on `ExitStatus`, matching the v0.2.44 fix everywhere else (#82). `list_recursive_batch` belt-and-braces retry now wraps `list_recursive_via` in the same `LIST_T` timeout the worker paths use (#85). `upload_bytes` explicitly shutdowns the SFTP file handle on the write-Err path so the server-side handle doesn't linger (#87). `exec_bash` sends `chan.eof()` on timeout so a runaway remote process gets an orderly close signal (#88). `shell_quote` also rejects `\t` so a tab-bearing remote filename can't poison the `find -printf '%p\t%s\t%T@\n'` parser (#90).

**Assistant / MCP (5).** Bridge token security: `BridgeInfo` now carries both `token` (write-scoped) and `readonly_token`; `handle_conn` infers `Scope::Write` vs `Scope::ReadOnly` from which token presented; `dispatch` rejects `remote_bash` on read-only scope. `write_mcp_config` injects `RIFT_BRIDGE_READONLY_TOKEN` always, `RIFT_BRIDGE_TOKEN` only when `remote_shell_enabled`. `mcp_server::tool_sync_status` prefers readonly with write-fallback; `bridge_enabled()` accepts either. A compromised MCP tool with only the readonly token can no longer escalate to `remote_bash` (#62). Assistant stderr buffer capped at 64 KiB with line-boundary tail-preserving truncation + a `[... earlier stderr dropped ...]` marker so a wedged CLI spew can't OOM the parent (#66). `child.id() None` branch logs a warning so the orphan-or-instant-exit case is diagnosable (#67). MCP `handle_conn` unauthorized response now `write_line(...)?` + `write_half.shutdown().await` before returning the error, so the client sees the typed error instead of a connection-reset (#69, folded into #62). `tool_grep` streams an 8 KiB binary probe via `File::open` + `take(8192).read_to_end` BEFORE loading the full file, so a 5000-file scan no longer pulls 5000 full files into the stdio process (#71).

**Cleanup / hygiene (3).** `ActivityRow::default` uses `DateTime::UNIX_EPOCH` instead of the deprecated `from_timestamp(0,0).unwrap_or_else` form (#92). Dropped the redundant early `tokio::spawn(release)` in `process_entry_body` success — `process_entry`'s post-result inline release already covers both Ok and Fail (#100). 11 `eprintln!("[rift] sync_*` debug lines in `lib.rs` sync command handlers converted to `log::debug!` so production stderr stays clean (#104).

## v0.4.15-alpha — 2026-05-20 — N-pane split + ActivityBar redesign + sync polish

**Split-pane assistant goes N-wide (S117 + this session).** Original 2-pane split generalized to 1..4 horizontal panes. Store: `panes: PaneState[]` (always length≥1, no more null/tuple branching), new `addPane()` / `closePane(idx)` / `canAddPane` / `MAX_PANES=4`. `dropTabIntoPane(tabId, paneIdx: number)` rewritten — sibling-pane swap via `findIndex`, sentinel `paneIdx === panes.length` auto-adds a new pane at the right edge (cap-aware). `scrubTabFromPanes` / `setFocusedPane` / `assignFocusedPane` / `restoreTabs` all array-driven; restore clamps focused idx + prunes stale tab refs. `AssistantPage` renders via `{#each panes as p, i}` w/ 1px dividers between. `AssistantPane` (extracted from `AssistantPage` in S117): `paneIdx: number`, `min-width: 320px` (4×320=1280 fits any modern window), new pane-chrome — numbered badge + ✕ close button, visible only when split, only on hover/focused. `ChatTabsBar` dropped 2-color `in-p0/p1` underlines for a single `.in-pane` underline + numeric `.pane-badge` (scales 1-4); split-toggle button now calls `addPane()`, disabled at cap, shows current pane count. `Ctrl+\` adds a pane; `Ctrl+Shift+\` closes the focused pane (last pane uncloseable). `StatusHub` pane lookup via `findIndex`. `tauri.conf.json` carries `dragDropEnabled: false` from S117 — required for cross-region HTML5 DnD; Rift uses no file-drop Tauri events.

**ActivityBar redesign (S116).** Rail width 40→44px, icon 16→17px, inset pill hover, blended edge gradient, 3px active capsule, press scale 0.94. Replaced HTML5 DnD w/ pointer-event drag (WebView2 `<button>` eats `dragstart`): 4px movement threshold, floating icon + drop-line indicator + pulse, click-suppression post-drop so the icon doesn't fire a workspace-switch on release. `AppShell.svelte` grid column updated 40→44px to match.

**Sync hardening (uncommitted carry).** `auto_sync.rs` + `auto_sync/flush.rs` + `lock_presence.rs` — additional poison-path safety + diag-event coverage on top of the v0.4.14 sync batch. Net: ~150 line delta across the three files, no public API change.

**Verify:** `npm run check` 0 errors / 3 pre-existing CSS warnings across 4053 files. Backend `cargo check` clean against v0.4.14 baseline; release pipeline runs full `npm run tauri build`.

## v0.4.14-alpha — 2026-05-20 — Audit sweep + sync hardening + assistant polish

**HIGH-severity sweep (S113).** 14 of 16 outstanding HIGH audit items closed: #34 #35 #36 #39 #40 #41 #74 #139 #140 #141 #142 #163 #219 #220. Mix of correctness bugs in `assistant/mod.rs` + `auto_sync.rs` + `drift_scanner.rs` and one frontend scrollbar leak (#163 — `.scroll` + `.strip` now both nuke `scrollbar-width` + `::-webkit-scrollbar` so WebView2 stops leaking native arrow-buttons top-right). Deferred to Phase 6 keychain work: #37 + #38.

**Sync MED batch (S114).** 8 full + 2 partial fixes across the auto-sync engine. `is_pushing` flag flips to a safer Err path on poison (#43); `stop_watch` now unwatches before tearing down so notify events don't queue against a dead handler (#44); FS-drop counter went `AtomicU64` w/ a hard Error at 100 drops (#45, *partial* — full plumbing deferred); `pending_dir_reconcile` kicks before clearing so we don't lose a reconcile signal mid-flush (#46); `force_pull_now` poison path emits a diag event (#48); `download_paths` opens the SFTP channel before allocating CT to avoid lock-window starvation (#57); TOFU `write_probe` flips to `None` for hosts w/o a probe (#61); one mutex moved to `into_inner` over poison-unwrap (#63, *partial*); `CLAUDE_EXE` cache became a `Mutex` w/ an `is_file` revalidate so a deleted-then-reinstalled CLI re-resolves cleanly (#64); `save_config` writes to a tmp-then-rename so torn writes can't corrupt `~/.rift/config.json` (#65). Deferred: #47 (CT plumbing), #58 (DiagStage taxonomy decision), #59 + #60 (post-expansion guard), #62 (read/write token split — needs FE coord).

**Tier-2 frontend (S114).** 4 of 4 closed. #143: per-tab fields migrated off the store into `TabState` class w/ getters on `AssistantStore` — no more shared-state bleed across tabs. #144: `dropTab` + `pruneTabUi` clean up UI state for removed tabs so closed-tab debris doesn't outlive the conversation. #145: per-tab `saveTimer` + `snapshot.scheduleSave` + `flushNow` iterates every tab on shutdown — closing the window no longer drops in-flight writes from inactive tabs. #150: `Settings.svelte` `$effect` wrapped in `untrack` to break a reactive feedback loop on theme toggle. `ensureTab` precedes per-tab writes in `send`, `openTab`, `newTab`, `closeTab` so the per-tab store slot always exists before write.

**Assistant page polish (S115).** Merged `AssistantHeader` action chips into `ChatTabsBar` — single top strip, no duplicate `+ New` button (the old header is left on disk as dead code, explicit-delete only). `EmptyState`: `justify-content: center` + 12vh→24px padding + redundant workspace subtitle dropped (card already shows it) + suggestion cards now single-line ellipsised teaser. `.error` / `.notice` banners moved out of the message scroll into a sticky `.alerts` strip above the composer so error toasts don't scroll away mid-stream. New `StatusHub.svelte` (#5): spinner + live label + elapsed timer + Stop button, parked above the composer while streaming — status lives where the user's eyes are, not scrolled-up inside an old bubble. `MessageBubble` (#2): in-bubble stream-status removed (redundant w/ hub); role-row turn-badge gated to streaming-only; completed turns grow a `.turn-footer` (divider + model · cost) instead of an inline floater.

**Backend verify (S114 close):** `cargo check` clean (1 pre-existing warn carried). **Frontend verify:** `npm run check` 0/0 across 4051 files. 3-file bump 0.4.13-alpha → 0.4.14-alpha.

## v0.4.13-alpha — 2026-05-19 — Assistant UI overhaul + update-flow restyle

**Assistant page redesign (S111).** Killed the redundant empty-tabs gate — first tab auto-opens on mount, click-twice-to-chat dead funnel is gone (~85 LOC deleted from `AssistantPage.svelte`). User messages now right-align with a neutral `--bg-elev-2` surface + 12px radius (was left-aligned with `--accent-soft` colliding with reasoning blocks); user avatar dropped entirely — bubble position carries the role signal. Claude turn-badge ("Sonnet 4.6 · $0.0421") sits inline beside the role name instead of floating right via `margin-left:auto`. Copy button claims the right edge of the role row. Messages container widened from 860px to `min(960px, 88ch)` w/ 20px gap + faint top-border between adjacent bubbles for turn rhythm.

**Header de-twinning.** `+` button now labeled "New" so it stops reading as an icon-twin of the tasks-toggle. Tasks toggle only renders when `taskCount > 0` — perpetually-empty corner button gone. Workspace chip recolored from accent-purple to neutral `--bg-elev-2` (accent reserved for AI-originated surfaces only).

**EmptyState anchoring + stagger.** Hero anchored at 12vh from top instead of vertical-centered (no more 200px void above the suggestions). Workspace card + suggestions cards both widened to 520px to match. Suggestion-card prompt clamp bumped 1→2 lines so the helpful detail isn't truncated. Stagger entrance (60ms/100ms/140ms delays) + 4.2s hero-glyph breathe animation + press-state scales on cards.

**Composer normalization.** Width lockstep with messages (`min(960px, 88ch)`). All controls baseline-centered: mic 26px (borderless, faded), hint 22px, effort pill 22px (smaller — secondary toggle), model pill 24px w/ ▾ caret indicating it opens a menu, send 28×28 (down from 32, less aggressive CTA). Composer auto-switches `align-items` to `flex-end` via `:has(textarea:not(:placeholder-shown))` when multi-line so controls hug the bottom row. Queue block recolored neutral (was accent-soft dashed border). Focus glow transition smoothed `140ms → 200ms cubic-bezier`. Send-press scale 0.96 for tactile feedback.

**Scrollbar nuke.** `.scroll` chat container and `.strip` tab-bar both fully hide their scrollbars (`scrollbar-width: none` + `::-webkit-scrollbar { display: none }` + `::-webkit-scrollbar-button { display: none }`). The native WebView2 arrow-buttons that were leaking through default `::-webkit-scrollbar` rendering in the top-right corner are gone. Scroll still works via wheel/keyboard/touch.

**Jump-to-latest pill.** Floating pill above composer (`bottom: 84px`, center-aligned) appears when user scrolls up away from tail. Click → smooth-scroll back + re-arm stick-to-bottom. Tab-switch scroll restore also upgraded to smooth-scroll (streaming-delta autoscroll stays instant to avoid fighting itself).

**ChatTabsBar entrance.** New tabs slide in over 220ms cubic-bezier (translateY -4px → 0).

**Update flow restyle (S110).** `UpdateInfoDto` extended with `sizeBytes` + `notesMarkdown` + `releaseUrl` + `publishedAt`; `UpdateService` is now managed Tauri state (`Arc<UpdateService>`) so pending `UpdateInfo` survives between commands. Monolithic `apply_updates` split into `download_update` (streams `update-progress` i16 events + emits `update-downloaded`) + `apply_pending_update`. Frontend store grew an 8-state machine + progress + `dismissedVersion` snooze + `pillVisible`/`sizeLabel`/`publishedLabel` derived getters. `UpdateDialog` restyled (gradient header + glow, version-diff chips, release-notes card w/ markdown-lite renderer, shimmer progress card, green rocket ready-card, per-state footer). New `UpdateToast.svelte` slides up bottom-right when an update is available (12s auto-dismiss paused on hover, snooze × button). StatusBar pill (pulsing dot + sparkles + version) visible when available/ready + toast dismissed + dialog closed. `scripts/release.ps1` gained conditional `--splashImage` flag for the Velopack installer (active iff `src-tauri/installer-splash.png` exists).

Net: Frontend `npm run check` 0/0/4051. 3-file bump 0.4.12-alpha → 0.4.13-alpha.


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

## v0.4.9-alpha — 2026-05-17 — Embedded-Claude addendum overhaul (act-first, no-guess)

Behavioral fix for the "AI is 50% dumber, just gives advice instead of editing" complaint. [mod.rs:644](../../src-tauri/src/assistant/mod.rs#L644) `RIFT_SYSTEM_ADDENDUM_TOOLS` rewritten with explicit anti-laziness clauses (act-first, never guess, edit-then-verify, narrow reads, no re-reads). Added `MultiEdit` + `Agent` to advertised tool roster. Addenda append LAST → win tie-breakers vs inherited `~/.claude/` rule clusters on both Blazzer's + Trey's machines. Single-line `.cmd`-shim constraint preserved. Temporary unconditional baseline; Settings → Assistant → "Direct-action mode" toggle queued for later. 3-file bump 0.4.8 → 0.4.9-alpha.

## v0.4.8-alpha — 2026-05-17 — Hot-fix: dyslexia toggle "stuck on" + Appearance shell-switch "disappears"

Two UX bugs reported immediately after the v0.4.7 ship. (1) Dyslexia master toggle correctly cleared the system-prompt addendum on the next turn, but the visual effects (Lexend font, increased line-height) stayed because `apply()` in `accessibility.svelte.ts` wrote font/line-height CSS attrs unconditionally. Fix: dial attrs now gated on the master flag — off snaps the visual back to system defaults; persisted sub-dial state preserved so re-enabling restores. (2) AppShell renders Settings as a routed page in v0.2 vs a modal in v0.4.1; live-flipping mid-Settings reparented into a structure with no mount point and the panel vanished. Fix: `setUseV03Shell` now calls `window.location.reload()` after 120ms (lets localStorage commit), re-mounting cleanly with the new flag. Hint copy updated.

## v0.4.7-alpha — 2026-05-17 — Settings → Accessibility (dyslexia-friendly Assistant)

Trey-driven feature. New **Settings → Accessibility** section between *Assistant* and *Speech* (lucide `Accessibility` icon). One master toggle plus three independent dials, all persisted to localStorage and applied via `data-a11y-*` attributes on `<html>` so CSS overrides land instantly with no reflow shrapnel.

**Master toggle — Dyslexia-friendly mode.** First time on, seeds the recommended bundle: Lexend font + increased line-height. When on, [src/lib/state/assistant.svelte.ts](src/lib/state/assistant.svelte.ts) forwards a `dyslexiaMode: true` arg to the new `assistant_send` signature, and [src-tauri/src/assistant/mod.rs](src-tauri/src/assistant/mod.rs) appends a single-line addendum to the per-turn system prompt telling Claude to interpret phonetic/letter-swap typos and slurred-speech artifacts charitably.

**Three independent dials:** UI font (System ↔ Lexend, bundled via `@fontsource-variable/lexend`), increased line + letter spacing (scoped to `.bubble` / `.markdown-body` / textarea), warm reading tint (sepia overlay on message bubbles + code blocks only).

## v0.4.6-alpha — 2026-05-17 — Hot-fix: switch embedded Claude to `bypassPermissions`

A second Assistant session reported `mcp__rift__remote_bash` denied with: *"Permission to use mcp__rift__remote_bash has been denied because Claude Code is running in don't ask mode."* — surfaced after the v0.4.5 ship despite `mcp__rift__remote_bash` being in the `--allowed-tools` allowlist (verified). Root cause: [src-tauri/src/assistant/mod.rs:926](src-tauri/src/assistant/mod.rs#L926) passed `--permission-mode dontAsk`, which auto-DENIES anything that would otherwise prompt the user — and MCP tools (incl. `mcp__rift__remote_bash`) require per-call approval that `--allowed-tools` does NOT short-circuit in `dontAsk`. Rift has no interactive permission UI by design, so the right mode is `bypassPermissions` — auto-allows every call, and `--allowed-tools` continues to act as the actual gate over which tool names are reachable.

One-line change (`dontAsk` → `bypassPermissions`) plus a comment block explaining why. `--allowed-tools` from S91 unchanged: the full `BUILTINS` set + scoped `mcp__rift__*` (+ `mcp__rift__remote_bash` when the remote-shell toggle is on) still defines what's reachable.

This is the real root cause behind the chronic "permission denied" reports — S91 widened tool *availability* but every MCP call still hit the per-call approval gate. v0.4.6 closes that gate via mode switch.

Verify: `cargo check` clean (auto-verifier). Functional verification via CDP after binary install — re-run the previously-blocked `mcp__rift__remote_bash` from a Trey-mode session.

## v0.4.5-alpha — 2026-05-17 — Embedded Claude tool allowlist + STT alternates

**S91 priority 1 — permission denials in the Assistant tab.** Blazzer + Trey hit recurring "tool not in allowlist" / "permission denied" messages whenever the embedded Claude tried to spawn a subagent, run a background bash command, edit a notebook, etc. S88's fix added `Skill` to all three `--allowed-tools` branches in [src-tauri/src/assistant/mod.rs](src-tauri/src/assistant/mod.rs) but stopped short of the rest of the built-in surface. S91 widens to the full CLI built-in set: `Agent` (subagent spawning — used by `/plan`, `/quick-review`, `/check`), `AskUserQuestion`, `BashOutput` + `KillBash` + `KillShell` (the CLI auto-invokes these whenever Bash runs with `run_in_background: true`), `ExitPlanMode`, `MultiEdit`, `NotebookEdit`, `SlashCommand`. Refactored the three branch bodies to share a single `BUILTINS` const so future additions land in one place. MCP scope is unchanged — full-config keeps `mcp__*`, scoped branches keep the explicit `mcp__rift__*` entries (+ `mcp__rift__remote_bash` when the remote-shell toggle is on). Verified via CDP: a fresh Assistant tab successfully called Bash + spawned an Agent subagent with zero permission denials.

**S91 priority 2 — STT slurred-speech tolerance.** [src/lib/state/stt.svelte.ts](src/lib/state/stt.svelte.ts) requested only one recognition alternate (`r.maxAlternatives = 1`), so the engine's first guess won regardless of confidence. Bumped to 3 and added a `pickBestAlternate` helper in `onResult` that walks `results[i][0..length]` and returns the highest-confidence transcript. When WebView2's Azure backend bothers to score the alternates (sometimes it returns 0 for every one — spec-allowed), a cleaner lower-ranked variant can now win over the slurred primary. When all confidences are 0, behavior collapses back to `alt[0]` (no regression). Vocabulary hints + Azure-direct fallback deferred (stretch goals).

Verify: CDP-driven smoke test — fresh chat tab, "run pwd && ls then spawn a recon Agent to summarize" → assistant completed both calls cleanly, no denial strings in body text. svelte-check + cargo check both clean (auto-verifier).

## v0.4.4-alpha — 2026-05-17 — Revert TTS + workspace clean-out

User reversed course on text-to-speech same day v0.4.3 shipped: only STT was wanted. This release rips TTS out and folds in a wider hygiene pass.

**TTS removed end-to-end.** `src-tauri/src/tts/` deleted, `msedge-tts` crate dropped from `Cargo.toml` (Cargo.lock shed 30+ transitive crates incl. tungstenite/tokio-tungstenite, -447 lines). Frontend `src/lib/state/tts.svelte.ts` deleted; speaker toggle + per-message replay button + all 5 TTS cards in Settings + orphan voice-dropdown / slider / sub-head CSS purged. Settings section renamed `"Voice"` → `"Speech-to-text"` (id `"speech"`, icon `Mic`). `stt.svelte.ts:121` stale error string updated. `assistant.svelte.ts` lost `tts.init/feed/flush/cancel` integration points. STT is now the sole voice surface, unchanged.

**Workspace clean-out.** Dropped 6 dead npm deps (`bits-ui`, `clsx`, `tailwind-merge`, `tailwind-variants`, `tw-animate-css`, `@types/dompurify`) — leftover from the shadcn yank — pruning 18 packages from `package-lock.json` and cutting npm-audit findings 13 → 10. Deleted stale `scripts/cdp/smoke-v04.sh` (exercises retired dock primitive; `smoke-v04-1.sh` covers current shell). Cleared 24 CDP debug screenshots from `scripts/cdp/.tmp/` and the empty `.claude/worktrees/` directory. Removed pre-existing orphan `.reasoning-meta.subtle` CSS class from `MessageBubble.svelte` and trimmed 2 unused lucide-svelte icon imports (`ExternalLink` in `ConflictResolver.svelte`, `Upload` in `Bootstrap.svelte`). Pruned local branch `backup-s25` (4 unique commits from S22-S25 era) and remote orphan `claude/determined-driscoll-32834e`.

**Doc hygiene.** Project `CLAUDE.md` hot-files table refreshed to current line counts (10 backend + 5 frontend files drifted; `assistant/mod.rs` went 775 → 1167L, `Settings.svelte` 1060 → 1505L). Memory `project_rift_tauri.md` updated: current-state line rewritten as STT-only, two resolved caveats dropped (`state_referenced_locally`, `mode-watcher`). `docs/design/` claim corrected (no longer empty — carries `assistant-roadmap.md`).

Verify: `npm run check` clean (0 errors, 0 warnings, 0 files-with-problems for the first time in the v0.4.1 era). Net diff across 16 files: **-1858 / +149** = -1709 lines.

**S90 stress-test fix-ups.** Autonomous CDP-driven pass across every UI surface (ActivityBar / chat tabs / right-pane / Assistant / Settings × 7 / Sync / Files / Terminal / Velopack / status bar). Two latent UX bugs caught + fixed: (1) `right-pane.svelte.ts::init()` clamped the in-state width but didn't re-persist, so an out-of-range stored value survived across launches — now writes back the clamped width on first load. (2) `Composer.svelte` rendered the mic button unconditionally, so clicking it with STT disabled silently set `stt.lastError` and looked broken — now gated on `stt.config.enabled && stt.supported`, paired with an `onMount(() => stt.init())` so the gate reflects real backend config (without that, users with STT enabled would lose the mic until they touched Settings → Speech once). Tooling: extended `scripts/cdp/serve.cjs` `KEY_DEFS` with `Comma / Slash / Space / Period / Backquote / ArrowLeft / ArrowRight` so future CDP runs can drive `Ctrl+,`, `Ctrl+\``, etc. directly.

## v0.4.3-alpha — 2026-05-17 — Voice arc: text-to-speech + speech-to-text

Two-direction voice integration. Both surface through a new **Settings → Voice** section and toggle from the Assistant header / Composer respectively.

### Text-to-speech (Claude → audio)

`msedge-tts` Rust crate calls Microsoft Edge's read-aloud endpoint (Azure Neural voices, free, no API key). `src-tauri/src/tts/mod.rs` owns a single tokio task that drains a sentence queue serially and emits MP3 b64 over `tts://audio`. Frontend [src/lib/state/tts.svelte.ts](src/lib/state/tts.svelte.ts) buffers streaming text per message id, splits on `/[.!?]+["')\]]?\s+/`, dispatches each completed sentence, and plays back-to-back via HTMLAudioElement. Cancel = generation counter bumps (drops in-flight + queued) plus local queue clear.

AssistantHeader speaker toggle = single-click `enabled + auto_speak` on; click again mutes auto-speak (master stays on so per-message replay still works). MessageBubble gets a speaker icon next to copy for one-shot replay. Settings carries the voice picker (~500 Edge voices, English first), rate/pitch/volume sliders (-50..+50), and a Test button.

### Speech-to-text (audio → composer)

WebView2's built-in `SpeechRecognition` (Edge/Chromium → Azure when online) writes directly into `assistant.composerDraft`. Live interim text streams as the user speaks; final committed text replaces interim segments on each phrase commit. No Rust-side audio capture, no model download, no build deps. [src-tauri/src/stt/mod.rs](src-tauri/src/stt/mod.rs) only owns settings persistence at `~/.rift/stt-config.json`. Composer gets a mic button on the left — click to record (pulsing red), click again to stop (focus returns to textarea, cursor at end).

Settings exposes language (12 BCP-47 locales), live-partials toggle, continuous mode toggle, append-vs-replace insertion mode. Microphone permission prompts once via WebView; subsequent uses are silent.

### Why not whisper.cpp local

Pivoted away from `whisper-rs` mid-session. Build-time libclang requirement on Windows broke `cargo run` w/o LLVM installed; bindgen route would have forced every dev (Trey included) to install LLVM. Web Speech API delivers comparable quality (same Azure backbone as the TTS path) with zero install footprint, true real-time streaming, and no first-launch model download. Trade-off: requires internet (so does Anthropic, so does the TTS).

## v0.4.2-alpha — 2026-05-17 — Hot-fix: embedded Claude's `Skill` tool

Trey reported `/handoff`, `/check`, `/plan` etc. were rejected inside Rift's Assistant tab w/ "skills blocked." Root cause: `assistant_send` builds `--allowed-tools` as an explicit comma-list and `Skill` was missing from all three branches (full-config, scoped, scoped+remote-shell). The CLI's allowlist gate then denied the `Skill` tool even though `--disable-slash-commands` wasn't set. Added `Skill` to every branch in `assistant/mod.rs`; corrected the misleading `use_full_config` doc comment that claimed skills "always load via the CLI's own resolution" (true for command discovery, false for the tool gate).

Affects: every user of the embedded Claude in Rift since v0.2.56. Hot-fix only — no schema changes, no migration. Velopack delta is one Rust object.

## v0.4.1-alpha — 2026-05-17 — Right-pane refactor + audit fix-pass arc

Daily-driver use of v0.4.0-alpha surfaced two model mismatches, both corrected. `useV03Shell` toggle name + storage key reused for upgrade compat; v0.2 path stays pixel-identical.

### Tasks → AssistantPage (Phase 1)

Tasks is an Assistant property, not a Files/Sync peer. Dropped from `PanelId`/`PANEL_IDS`/`PRESETS`; `TasksPanel.svelte` deleted; `TasksDock` renders inside `AssistantPage` in both shells. Header Tasks toggle collapses to `assistant.ui.dockOpen`. Remaining kbd labels rebump (Sync → 1 … Activity → 7).

### ActivityBar + RightPane shell (Phase 2)

Right side = ONE full page at a time picked by a 40px vertical activity bar on the far edge (VS Code pattern). Files · Sync · Activity · Terminal · Agents · Attachments · History — drag-reorder via HTML5 DnD, persisted to `rift.ui.activitybar-order.v1`. `Ctrl+1..7` follows that order, `Ctrl+0` closes. `RightPane.svelte` lazy-mounts each page on first activate + everOpened-latches so scroll/selection/terminal session survive toggles. Left-edge resize 320..1200, dblclick snaps 50% (RAF-throttled).

`state/right-pane.svelte.ts` runs a one-shot storage migration: reads `rift.ui.panels.v1`, seeds `activeId` from the exactly-one-open panel (drops `tasks`), deletes the key. `dock-w.v1` → `right-pane-w.v1`. `dock-split.v1` / `maximized.v1` / `preset-picked.v1` / `dock-accordion.v1` all delete same boot.

Body grid `[chat | --right-pane-w (0 when closed) | 40px bar]`. `FilesPanel` always full `<TwoPane />`; `SyncPage` always full drift table + Mirror toolbar. `TerminalPanel` + `terminal.toggle()` route through `rightPane`.

### Dock primitive retired (Phase 3)

`Dock.svelte` + `PanelShell.svelte` + `PresetPicker.svelte` deleted. `ui-prefs.svelte.ts` 325L → 58L (only `density` / `railPinned` / `useV03Shell` survive). `PanelState` / `DockSlot` / `LayoutPreset` / `PRESETS` deleted. Settings → Appearance → Layout swaps "Reset dock split" → "Reset right pane" (`rightPane.reset()`); kbd cheat updated.

### Audit fix-pass arc (S80–S86 — 17 items closed)

S80 verification archived 16 items silently fixed by v0.3/v0.4 refactors. S81 (6): `editor_for` race-loss warn, `DiagBus` AtomicI64 lock-free, `DiagEvent.file` basename-only, `safe_profile_key` empty-sentinel, `lock_presence` stale-delete fail counter, `ssh_keygen` chmod 0o600. S82 (6): `RiftConfig::load` 1 MiB cap pre-parse, velopack-pin doc, `watch::try_watch` local_root forward-slash normalize, `sftp::close` lock-snapshot, `RIFT_UPDATE_FEED` gated `#[cfg(debug_assertions)]` (release builds cannot accept attacker-supplied local update feed), `in_place::Drop` detached thread. S83 (3): `LogForwarder::scrub_log_message` redacts `$USERPROFILE`/`$HOME` + OpenSSH/RSA/EC PRIVATE KEY markers; LocalPane/RemotePane `$effect` cleanup bumps `loadToken` for cancellation. S84: `compute_sha1` streams via BufReader (8 KiB buf, EINTR retry) — peak heap drops from O(N×file_size) to O(N×64 KiB). S86: `flush_batch` no longer awaits `safe_count_files` inline — `FolderCountCache { AtomicU64, AtomicI64 }` keyed by remote_root, 5-min TTL, per-batch `(created, deleted)` delta; `stop_watch` evicts.

### Session-lost auto-recovery + History dedupe

Long-idle resume sometimes hits "No conversation found with session ID:" even though the JSONL is intact on disk (claude's resume index drifts). Backend emits `assistant://session-lost {session_id, prompt}` on stderr-match AND non-first turn. Frontend pops the failed pair, nulls `convoCreatedAt` so next send uses `--session-id`, surfaces "Session was lost — retrying as a fresh start", re-sends. Tab-aware: ignored if user switched tabs while error was in flight. First-turn failures still go through normal error path.

Header History button now hidden under v0.4.1 (ActivityBar exposes it); v0.2 path keeps it (no ActivityBar there).

### S86 polish

`isHandshaking = $derived(connecting && status === null)` in `connection.svelte.ts` — Titlebar/StatusBar pills no longer flash "Connecting" during the post-handshake-pre-first-status window. Raw `connecting` still gates action buttons (anti-double-fire). New `lib/utils/file-display.ts` (`fmtSize`/`pickIcon`/`clampMenuPos`/`isEditableTarget`) shared by LocalPane + RemotePane (~68 LOC dedup).

### Verify

`npm run check` — 0 errors, 1 pre-existing warning (`.reasoning-meta.subtle`). `cargo check` — 0.50s clean. CDP smoke `scripts/cdp/smoke-v04-1.sh` green. v0.3 toggle OFF renders pixel-identical to v0.4.0-alpha's v0.2 path.

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
