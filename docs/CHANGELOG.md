# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — on top of v0.20.9 — Round 5 hardening (config migration race)

> Unshipped on `main`, pending the next bump. Verified: cargo check clean (0 errors/warnings, isolated target).

**Hardened (round 5 — config write-path race, adversarially-verified):**
- **api_key keychain-migration save is now lock-guarded** — `config.rs::load_config` runs the one-shot plaintext→keychain migration both unlocked (getters) and locked (setters). Its `save_config` previously ran without `CONFIG_WRITE_LOCK`, so a getter racing an active setter could land its tmp-rename second and silently clobber the setter's change. Now wrapped in `CONFIG_WRITE_LOCK.try_lock()`: the migration persists only when the lock is free (non-reentrant std Mutex → `WouldBlock` skips the save when a setter on the same thread already holds it; the field stays in JSON and re-migrates on the next uncontended cold load). No deadlock, no lost setting. (Last open finding from the round-3 review — `draggingTabId` dead-feature wiring stays catalogued in ISSUES #46/#36, out of scope for the harden loop.)

## v0.20.9 — 2026-06-21 — Recovery tools + per-project chat scoping + 4-round hardening pass

> Verified: svelte-check 0/0 (4105) · cargo check clean (0 errors/warnings, isolated-target recompile of all edited .rs — dev app untouched).

**Hardened (round 4 — under-swept surfaces: stt / diagnostics / oneshot / workspace, adversarially-verified):**
- **STT model integrity on 416-resume** — `model_manager.rs`: when HuggingFace returns HTTP 416 (the `.partial` already covers the full file), the resume path now runs a full-file SHA256 verify before promoting `.partial → final`, mirroring the normal resume path. A truncated/corrupt partial is renamed `.badhash` and errored instead of silently installed as the live model.
- **STT title generation can't wedge the app** — `oneshot.rs::assistant_generate_title` now bounds the CLI stdout-read + `child.wait()` in a 30s `tokio::time::timeout` and `start_kill()`s the child on expiry. Unlike the enhance path it had no cancel registry, so a hung `claude` (OAuth re-prompt / network stall / broken pipe) previously left the command future permanently unresolved (only an app restart recovered it).
- **STT config writes serialized** — `stt/mod.rs`: `load_config`/`save_config` now take a process-wide `STT_WRITE_LOCK` (mirrors `assistant::config::CONFIG_WRITE_LOCK`); rapid Settings toggles via the sync `stt_set_config` command could otherwise interleave truncate+write and silently drop a save.
- **Workspace file-walk off the Tokio worker** — `assistant_list_workspace_files` is now `async` + `spawn_blocking`; the up-to-4000-entry `walkdir` traversal no longer runs inline on an executor thread (a large monorepo / network FS could stall it for seconds). Shared sync helper `list_workspace_files_sync` backs both the command and `stt::workspace_context`.
- **401 error no longer leaks the home-dir path** — `turn.rs::auth_rejection_message` now surfaces only the CLI *filename* (`claude.cmd`), not the absolute install path (`C:\Users\<name>\AppData\…`), which was rendered verbatim in the chat error bubble and any diagnostics export. Full path still lives where it belongs: Settings → CLI session.
- **Diag-bus Lagged warn off the async task** — `diagnostics/mod.rs`: the `RecvError::Lagged` handler now writes directly to the file sink instead of `log::warn!`, which re-entered `LogForwarder` (file mutex-lock + flush) on the Tokio task and re-published onto the very bus that just lagged.

**Hardened (deep review — adversarially-verified findings):**
- **Log sink survives mutex poison** — `diagnostics::file_log_write` now recovers a poisoned `FILE_LOG` lock (`into_inner()`) instead of silently dropping every subsequent write. A panic mid-write previously blacked out the only persistent log sink in a GUI prod build (stderr is `/dev/null`). Also: env_logger (dev stderr) now receives the SCRUBBED message, so home-dir paths no longer leak to the dev terminal.
- **STT no longer hangs on a stalled audio device** — capture-init `recv()` got a 10s `recv_timeout`; a Bluetooth/WASAPI device-enum hang on Windows can't wedge `stt_start_recording` forever and burn a Tokio blocking-pool slot. Haiku-cleanup failure now emits `stt://error` (`cleanup_failed`) instead of silently returning the raw transcript.
- **Background streaming tab no longer leaks its subprocess** — `closeTab` stops the CLOSING tab's CLI subprocess directly (probing its own `streaming`), not just the active tab's. Closing a background tab mid-stream previously orphaned the `claude` process (burning tokens, events silently dropped).
- **Conversation saves serialized** — added `CONVO_WRITE_LOCK` + per-call tmp suffix so two windows saving the same convo id can't race on a shared `.tmp` and silently install stale data (mirrors `CONFIG_WRITE_LOCK`).
- **Enhance cancel-before-register race closed** — a Discard fired in the spawn→PID-register gap is now honored (pre-registered sentinel pid); previously the cancel was lost and the billed enhance ran to completion.
- **MCP `read_file` respects SKIP_DIRS on the canonical path** — a workspace-internal symlink into `node_modules`/`.git`/`target` can no longer bypass the exclusion that `grep` already enforces.

**Hardened (round 2 — under-covered surfaces, adversarially-verified):**
- **CLI can't wedge on a broken stdin** — `turn.rs`: (1) a steer-flush write failure now `break 'outer`s the turn loop instead of escaping only the inner `for` and spinning a dead pipe; (2) `handle_permission_request` now returns `io::Result` and the caller surfaces + exits on a failed `write_control_response` — previously a broken pipe after the user's Allow/Deny left the CLI blocked on a response that never arrived (turn hung, no error, until the 30-min permission timeout).
- **PermissionRegistry entry can't leak on task abort** — `register_guarded` returns an RAII `PermissionGuard` whose `Drop` cancels the entry, covering the case where `stdout_task` is aborted mid-await (the explicit `cancel` never ran when the future was cancelled).
- **`icacls` lockdown off the Tokio worker** — `write_mcp_config` now fires the blocking `icacls` DACL-tightening on a detached OS thread instead of blocking an executor thread per turn (sub-100ms normally, but seconds under AV/contention).
- **Update integrity: `arm_repair` bumps `download_epoch`** — a normal download in flight when Repair is triggered can no longer flip `downloaded=true` against the repair plan (would arm an apply with the wrong package on disk).
- **`now_ms()` safe-fails to `i64::MAX`** — a broken clock (pre-1970) now treats every OAuth token as expired (clear "expired" message) instead of unexpired (confusing 401). `auth_update.rs` `CLAUDE_EXE` writes recover a poisoned lock (`into_inner()`), matching `cli_install.rs` — a no-op there would spawn the pre-update binary.
- **`browser://load` scoped to its host window** — `emit_to("main", …)` not a global `emit`, so a second window's address bar/spinner no longer tracks navigations from the dock it doesn't own (multi-window state bleed).
- **Frontend unmount hygiene** — `Composer` gained an `onDestroy` clearing `steerFlashTimer`/`undoTimer`, releasing a held PTT (mic was left recording on the global stt singleton when a pane closed mid-hold), and cancelling an in-flight `enhancePrompt` (Haiku spawn kept billing after close); `stt.sendRequested` reset wrapped in `untrack()` (removes a spurious effect re-run). `AssistantPane`'s first-send flip timer now self-cancels via the shared `clear` (no stale style write on a detached node).

**Hardened (round 3 — turn-control + persistence + render correctness, adversarially-verified):**
- **Queued message no longer deadlocks after an error** — `send.ts::drainQueue` dropped the `|| tab.lastError` guard: a turn that ended in error left the queue gated forever (every queued follow-up stuck behind a flag nothing cleared). Queue now drains on the next idle tab regardless of the prior turn's error state.
- **Stop() no longer strands the queue or stale prompts** — `assistant_stop` now clears `permissionPrompts`/`unboundAskUser*` FIFO state and fires `onTurnComplete` after the stop round-trips, so a user-initiated stop can't leave a half-bound ask_user prompt or a never-completing turn behind.
- **Close-all stops background streams** — `tabs.ts::closeAllTabs` now `await`s `stop(id)` for every still-streaming tab before dropping them (mirrors `closeOtherTabs`); previously closing the window mid-stream orphaned the non-active tabs' CLI subprocesses.
- **Stale conversation load can't overwrite a newer one** — `persistence.ts::loadConversation` got a per-host load-generation token: a slow load that resolves after the user has already switched conversations is now discarded instead of clobbering the active tab. Malformed persisted JSON now `dropTab`s the half-built tab (was left as a permanent error stub) and the block-map walk null-guards `m.blocks`.
- **Tool-group expand/collapse fixed** — `MessageBubble` group-open state was a `Set` XORed against the default-open flag, so toggling a default-open group did nothing visible (and default-closed groups inverted globally). Replaced with an explicit `Map<key,bool>` keyed per group.
- **Ask-question selection survives streaming** — `ToolChip`'s reset `$effect` now early-returns when the questions/selection arrays are already in sync, so an unrelated re-render mid-stream can't wipe the user's in-progress multi-select answer.
- **Workspace config writes serialized** — `assistant_clear_root` + `assistant_remove_recent_root` now take `CONFIG_WRITE_LOCK` (matching `set_root`/`set_tab_root`); a clear racing a set could otherwise read-modify-write a torn recent-roots list.
- **ask_user / permission emit can't hang 10/30 min on a closed window** — `bridge.rs::ask_user_op` and `turn.rs` permission emit now pre-check `get_webview_window(label).is_some()`: Tauri's `emit_to` returns `Ok(())` for a missing label (zero webviews matched), so the prior `is_err()` deny-path never fired — a window closed mid-prompt parked the MCP/CLI on the full timeout. Now they cancel the registry entry and return an immediate "UI unreachable" error.

**Added (per-project chat scoping):**
- **The conversation sidebar now scopes to the open project.** Chats were a single global list mixing every folder's conversations together (confusing — opening `rgb-orchestrator` still showed `rift-tauri`/`pulse` chats). Now the sidebar shows only the open folder's chats by default, with a **"This project / All projects"** toggle under the search box; All-projects mode adds a per-row folder-name label so nothing is ambiguous. Switching folders switches the list.
  - **Backend:** `Conversation` + `ConversationMeta` gain `workspace_root` (`convo_store.rs`); `assistant_save_conversation` persists it. `assistant_list_conversations` **backfills legacy chats** (no stamped root) from the existing per-session cwd sidecar, so pre-existing conversations land in their real project automatically rather than going Unfiled.
  - **Frontend:** `buildSaveRecord` stamps `tab.workspaceRoot ?? activeRoot` (`persistence.ts`); `ConversationList.svelte` filters by `assistant.activeRoot` (case-insensitive path match) with the toggle persisted on `shell` (`rift.ui.conv-all-projects.v1`). No open folder → shows everything (chats never stranded).

**Added (self-recovery for end users):**
- **Install buttons for missing local tools** (Settings → About → Local tools): when `git`/`node`/`cargo`/`code` isn't detected, an Install button runs `winget install --id <pkg> -e` in a visible console (`Git.Git`, `OpenJS.NodeJS.LTS`, `Rustlang.Rustup`, `Microsoft.VisualStudioCode`). Falls back to an actionable error if winget is absent. (`env_checks.rs::install_local_tool`, `environment.svelte.ts::install`).
- **Repair installation button** (Settings → About → Help & diagnostics): force-re-downloads and reinstalls the current version over corrupted/missing program files via Velopack, then restarts. Arms the latest Full release as the pending plan (`update_service.rs::arm_repair`, `commands/update.rs::repair_install`) and reuses the normal download→apply chain (`updates.svelte.ts::repair`). Guarded by a confirm dialog.

**Fixed (design-system fidelity):**
- Composer permission pill now shows the DS short form ("Ask first" / "Auto-edit" / "Plan" / "Auto" / "Bypass") instead of the verbose label ("Bypass permissions") — matches `app/composer.jsx` `PERM_SHORT`. (`modelMatrix.ts` adds `short`; `Composer.svelte` renders `currentMode.short`).
- SettingsMenu model popover fully converted to the portal/`position:fixed` pattern (all child CSS namespaced under `:global(.settings-menu …)`) so it anchors to its trigger instead of floating mid-screen.
- Command palette Settings deep-links collapsed to the real 4-section IA (`appearance · chat · speech · about`).

## v0.20.8 — 2026-06-21 — Overnight efficiency + cleanup polish

> Stress-test + cleanup pass on top of v0.20.7. Verified: svelte-check 0/0 · vitest 210/210 · dev cargo rebuild clean · 0 prod npm vulns.

**Backend (turn hot-path efficiency):**
- Eliminated a redundant second `load_config()` disk read every turn (`config.rs::current_api_key_with` reuses the already-loaded config; `turn.rs:460`).
- Process-exit poll tightened 150ms → 50ms (`turn.rs:1366`) — faster turn-teardown.
- Stream char-pacer window 0.25s → 0.1s with a 360 c/s floor (`streaming.ts:288`) — snappier token render without dropping frames.

**Cleanup (dead code):**
- Removed `commands/git.rs` (3 registered Tauri commands with zero JS callers) + its mod/registration refs; removed `stt_set_engine` (superseded by `stt_set_config`); fixed 4 stale comments referencing the stripped remote half.

**Security:** `dompurify` 3.4.10 → 3.4.11 (GHSA-cmwh-pvxp-8882, ALLOWED_ATTR pollution — hits Rift's sanitizer `addHook` path). Prod vulns 1 → 0; remaining 4 are dev-only build deps.

**Frontend:** `ToolChip` ask-error color → real `--danger` token (was an undefined `--color-error` + hardcoded fallback).

## v0.20.7 — 2026-06-20 — Full redesign port + backend cleanup

> Whole UI re-ported to the design spec (`docs/design/rift-redesign.html`) at a strict fidelity bar — all 7 surfaces rebuilt + CDP-verified — plus a backend dead-code sweep.

**Changed (redesign — 7 surfaces):** Chat timeline → `StreamTurn`/`.sturn` (msg-acts row · jump-latest pill · turn-rail · FLIP), grouped tools → `.work` card (≥3). Home (warm/cold), Settings (bento → single-column RailShell, 5 tabs), Onboarding (centered card, 1180px), command palette. Composer → flat-bar + spec `.pop` popovers. Shell → global status bar; floating **Environment** pill replaces the old dock; SessionDiff dropped. Official Rift logo app-wide (neon-R squircle, transparent corners); screen-tint filter removed (a11y `warmTint` kept).

**Added:** Activity-dock toggle (Settings → Chat, default on). CLI backwards-compat hardening — version-gates every bleeding-edge spawn flag to the *installed* `claude` (`cli_caps.rs` table + `active_cli_version()`); 8 flags gated with fallbacks; below the 2.1.161 floor → update prompt; unreadable → conservative + Settings `⚠` banner.

**Fixed:** stream render defaulted to the legacy `MessageBubble` (streamMode defaulted OFF) → now ON; `stt.svelte.ts::setConfig` rolls back on backend reject; `deleteAllConversations` re-syncs in `finally`.

**Internal:** dead-code sweep (`rift_dir()`/`atomic_write_json()` + 2 unread `AudioCapture` fields, tightened `pub`). cargo test 64/64 · check 0/0 · vitest 210/210.

## Older versions

Full detail for any version: `git log -- docs/CHANGELOG.md`.

**v0.20.x:** .6 update/CLI-detection robustness (auto-retry + `CLAUDE_EXE` cache sync) · .5 CLI update-detection hardening · .4 DesignSync surfacing + method-aware chips · .3 effort-tier clamping + model-fallback correctness · .2 composer caption/slot polish · .1 thinking-off 400 fix (`nothink.rs` strips `clear_thinking_*`) · .0 cloud thinking on/off toggle. **v0.17–v0.19:** local-LLM no-think shim baked into backend · Local LLM page (context selector, model card, tok/s) · local-mode `num_ctx` truncation fix + Optimize-for-Rift. **v0.9–v0.16.x:** multi-window · Environment float · CLI-event transparency · shared `PageHero` + Home stats · Fable kill-switch · R2 update feed · minimal-core strip (−7,407 lines → 3 workspaces) · self-update brick fix. **v0.5–v0.8.x:** composer slim · dictation/PTT · loopback bridge · Fable 5 · backend split · tag-driven CI.
