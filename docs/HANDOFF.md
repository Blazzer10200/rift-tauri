# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 25 — 2026-05-11 — Lock heartbeat (brainstorm #1)

### Completed
- **Stale-lock heartbeat** ([sync/lock_presence.rs](src-tauri/src/sync/lock_presence.rs)). Own lock files now re-stamp `since` field every `HEARTBEAT_SEC` (60s) so long-running edits (>180s `STALE_SEC` sweep threshold) don't get their own lock swept by another Rift. New `last_heartbeat: DashMap<String, Instant>` tracks per-lock refresh times. `acquire()` records on successful upload, `release()` clears, `stop()` clears for symmetry. New `refresh_my_locks()` walks held locks each `poll_once()` (10s) and re-uploads body if ≥60s elapsed. Failed refresh writes leave lock as-is (next poll retries; total connection loss → lock goes stale → reclaimed, desired behavior). 3× headroom (60s heartbeat vs 180s sweep) absorbs 2 missed beats.
- **Cleanup.** Removed orphan git worktree `.claude/worktrees/determined-driscoll-32834e/` + branch `claude/determined-driscoll-32834e` (S24-flagged).
- **Brainstorm research** ([scout]). Playwright MCP v0.0.75 (May 7 2026) now uses accessibility-tree output — the 4.3M-token-per-screenshot disaster that triggered the ban is obsolete. BUT Playwright doesn't drive Tauri apps anyway (WebView2). Proper Tauri test path is `tauri-driver` + Microsoft Edge Driver. User opted to skip the lock test ("AI is doing the file work anyway") so no immediate testing pipeline needed.

### Flagged for future
- **Local pane has no "Open in editor" action.** S24 dropped the lying Edit button; nothing replaces it. Real UX gap when a human (not AI) needs to edit a file Rift is browsing. ~15min fix: add `local_open_in_default(path)` Tauri cmd via `opener::open`, wire into LocalPane ctx menu.
- **Lock orphans visible in field test.** Screenshot showed `.rift-lock` files alongside every file + a `fxmanifest.lua.tmp.PID.HASH.rift-lock` (lock for a temp-upload filename — should never exist). Likely pre-heartbeat detritus. Will sweep on next poll once Rift runs for >180s.

### Next Steps (post-ship)
1. Field-test the heartbeat when convenient (test plan in S25 conversation; not blocking).
2. Pick next brainstorm item — recommend **#4 per-resource sync mode** (Live vs Manual toggle) for highest day-to-day-value-per-hour.
3. **Deferred:** #2 buddy presence, #3 bridge realtime (multi-day), #5 DPAPI, "Open in editor" action.

---

## Session 24 — 2026-05-11 — Reconcile UX + echo-loop + orphan-tmp self-heal (summary)

Listing path-guard regression reverted (hybrid policy: nav free, destructive gated). Middle-rail Edit/Diff/Delete buttons removed; Sync button drives new `ScanProgressChip` with per-folder progress via `DriftScanProgress` diag stage. Drift-discovered ToPush now auto-enqueues via `kick_drift_reconcile` signature change (`&Arc<Self>`). Toast vocabulary unified via new `FlashToast`. Orphan `.rift-tmp` self-heal in `sftp/mod.rs::upload_atomic_via` (best-effort `remove_file(&tmp)` before `create()`) — clears foreign-owned tmps when parent dir is group-writable. Pull→push echo fix was attempted via `just_pulled_suppress_until` but dropped during rebase — v0.2.18's `recently_written` (5s window) already covers it. Field-test confirmed remaining four. Brainstorm (post-S24): #1 stale-lock (DONE in S25), #2 buddy presence, #3 bridge realtime, #4 per-resource sync mode, #5 DPAPI.

**Do NOT collapse** auto_sync ⊕ drift_scanner ⊕ drift_watcher trio. Distinct safety roles.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`.

**Current state (post S25):** v0.2.19-alpha-test on `rift-releases`. Builds on v0.2.18 (treyday onboarding session — `recently_written` pull→push loop fix, bulk conflict resolve, op-rail Delete, dir-recursion, manual-transfer events, rename/delete error visibility). v0.2.19 adds: S23 audit hardening, S24 reconcile UX + orphan-tmp self-heal, S25 lock heartbeat. S24's `just_pulled_suppress_until` echo fix was dropped during rebase as duplicate of v0.2.18's `recently_written` (same idea, theirs landed first).

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
