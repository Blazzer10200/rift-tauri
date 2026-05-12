# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 26 — 2026-05-12 — Sync modal + scan cancel + v0.2.20 cleanup

### Completed
- **SyncModal** ([src/lib/components/sync/SyncModal.svelte](src/lib/components/sync/SyncModal.svelte) + [state/sync-modal.svelte.ts](src/lib/state/sync-modal.svelte.ts)). Center-stage overlay replaces the corner `ScanProgressChip`. Listens to `diag://event` (drift_scan_start/progress/result) + `autosync://activity` directly; manages its own state machine (scanning → complete | cancelled | error). Watchdog: 30s of silence shows a "scan may be slow" banner but doesn't auto-fail (fixes v0.2.19's false-alarm timeout where the backend kept running after the toast fired). Backdrop blur disables clicks during scan; Esc cancels mid-scan or dismisses afterwards.
- **Scan cancel plumbing** ([drift_scanner.rs:scan_with_cancel](src-tauri/src/sync/drift_scanner.rs) + [auto_sync.rs:cancel_drift_reconcile](src-tauri/src/sync/auto_sync.rs) + new `diag_cancel_drift_scan` Tauri cmd). `CancellationToken` checked between folders; on bail returns partial `ScanResult` w/ `cancelled: true`. `current_scan_cancel: std::sync::Mutex<Option<CancellationToken>>` on engine (sync mutex because `kick_drift_reconcile` is called from the notify event handler, no async context). On cancel: partial push set is NOT auto-enqueued — user asked to stop.
- **Path-guards re-applied** ([lib.rs:upload_paths/download_paths](src-tauri/src/lib.rs)). Each job's local + remote target now validated against `profile.local_root` / `profile.remote_root` via `path_guard::validate_local_child` / `validate_remote_child`. Closes the v0.2.19 rebase gap (took origin's --ours, lost the guards).
- **Open in default editor** ([LocalPane.svelte](src/lib/components/browser/LocalPane.svelte)). New context menu item (files only) calls `openPath()` from `@tauri-apps/plugin-opener`. No Rust cmd needed — opener plugin already in `default.json` capabilities. Closes the v0.2.19 UX gap.
- **Dead code removal.** `auto_sync.rs::just_pulled_suppress_until` (field, init, fn, callsite, getter) — all 5 sites. Superseded by v0.2.18's `recently_written` (5s post-write window). Also dropped `ScanProgressChip.svelte` + `scan-progress.svelte.ts` (replaced by SyncModal); unwired from `AppShell.svelte`.

### Verify
- `cargo check`: clean (3.32s incremental).
- `svelte-check`: 0 errors, 6 a11y warnings (all pre-existing patterns).
- **Trey's v0.2.19 diag** (saved here for context): healthy state, 0 locks/conflicts/pending, drift_scan took 894 entries / 8 folders in ~45s → tripped the v0.2.19 30s frontend timeout. v0.2.20 modal will sit patient and surface the full result.

### Flagged for future
- **Pull-side modal action.** Modal shows "Pull X" count but doesn't yet have a "Review Pull" button that opens the existing DriftReview screen. Trey hit this — sees 894 ToPull, no obvious path to act on them from the modal. ~20min: add footer button when `result.pull > 0`, route through existing DriftReview component.
- **a11y warnings** — 6 leftover (`Settings`/`LocalPane`/`RemotePane`/`SyncModal`). Pre-existing patterns; sweep in a UX-only pass.

### Next Steps (post-ship)
1. Trey field-tests v0.2.20: sync button → modal opens → scan completes cleanly → counts visible. Hit Cancel mid-scan to verify the cancel button stops the reconcile.
2. Next brainstorm pick — recommend **#4 per-resource Live vs Manual sync mode** (highest day-to-day value) or **#2 buddy presence indicators** (good ride-along with the modal's activity feed since both surface "who's doing what").
3. **Deferred:** #3 bridge realtime (multi-day), #5 DPAPI, "Review Pull" button in modal.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`.

**Current state (post S26):** **v0.2.20-alpha-test SHIPPED** to `rift-releases`. Builds on v0.2.19. Net-new in v0.2.20: SyncModal + scan cancel + path-guard restore + Open-in-editor + v0.2.19 cleanup punchlist. Trey auto-updates via Velopack on next Rift launch.

**Known leftover (cleanup for v0.2.21):**
- SyncModal "Review Pull" button — wire to existing DriftReview component when `result.pull > 0`.
- Sweep pre-existing a11y warnings (Settings/LocalPane/RemotePane).

## CRITICAL DON'T-TOUCH
- russh `ring` backend + reqwest `rustls` features only (NASM blocks aws-lc-rs)
- `~/.rift/*.json` compat — don't change rename rules; don't drop `serde(flatten) extra`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver)
- DriftWatcher conflict-rename guard — never overwrite dirty local
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo
- `path_guard.rs` API frozen (`validate_remote_child`, `validate_local_child`) — `edit/in_place.rs` + lib cmds depend
- `rename_via` is strict (user-facing); `rename_overwriting_via` is ONLY for atomic upload tmp-swap
- **Source `.secrets/env.sh` first on ship/auth tasks** — Claude Code bash is non-interactive, won't auto-load. Past miss: spent 20min suggesting `gh auth login` when token was already there.
- **`current_scan_cancel` is std::sync::Mutex** (NOT tokio) — `kick_drift_reconcile` is sync and called from notify event handler; `blocking_lock` on a tokio Mutex there panics. Don't "fix" it.
