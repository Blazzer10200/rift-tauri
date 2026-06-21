# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — on top of v0.20.8 — Recovery tools + composer DS pill + hardening pass (2 rounds)

> Unshipped batch on `main`, pending the next bump. Verified: svelte-check 0/0 (4105) · cargo check clean (0 errors/warnings, forced recompile of all edited .rs).

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
