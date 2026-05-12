# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 31 — 2026-05-12 — sync verification, structural cleanup, UI consolidation

Three workstreams, all frontend/ops — no Rust changes, no version bump (still on `v0.2.38-alpha-test`).

### What landed
- **Sync verification** — diffed remote ↔ local on `endure-rp`. 4130/4130 file-match post-fix. 33 missing locally were all stock `[ox]/ox_lib/web/build/fonts/*.ttf` + `index.html` (library build output, NOT user data). Pulled via direct `ssh+tar` (bypassed rift mid-dev). 3 stale `.rift-lock` files in `[endure]/endure_skills/` cleaned. Zero user data lost from v0.2.38 ping-pong incident.
- **Structural cleanup of `[world]`** — Found 558 files of FiveM map content nested wrongly inside `[endure]/endure_skills/[world]/`. Moved 2 unique resources (`evo_apy_motel` 18 files, `mlo-deadoralive` 266 files) up to top-level `[world]/`. Merged 3 partial-duplicate resources (mlo-destruction +11, pillbox +17, postapo-interior +12). Dropped `[endure]/endure_skills` entirely (was just a test resource per user). Final: `[world]/` = mapping resources only (6 dirs). 3890/3890 file-match. Per-side `mv` + `cp -rn` for no-clobber merge.
- **UI consolidation pass** (10 files + 1 new helper) — Logo: replaced inline SVG in Titlebar with `<img src="/favicon.png">` sourced from `src-tauri/icons/icon.png` so browser tab + in-app + desktop installer share one asset. Duplicates killed across Titlebar/TopBar/TabRail/StatusHero/StatusBar: server name 3→2, locks 3→1 (StatusBar only, hide-when-zero), watcher 2→1, user@host 2→1, pending/failed 2→1 each. TopBar's pill trims SHA fingerprint to tooltip only. TabRail's old foot stats (Watching/Watcher/Locks) replaced with **Quick Actions** panel (chunky Pull all / Push all buttons calling `diag_force_pull_now`/`diag_force_push_now`). StatusHero collapses to a single line when idle (*"All quiet — N folders watched · last activity HH:MM"*), expands to conflict/queued/error/last-activity cards only when state warrants. StatusBar cells hide-when-zero, watcher is now a clickable pill. LocalPane + RemotePane MODIFIED column uses relative time helper (`src/lib/utils/time.ts`: Today/Yesterday/Mon/MM-DD format) + `white-space: nowrap` — killed the wrap bug. Trey-friendly relabels: pending→queued, failed→errors.
- **Animation polish** — TabRail active-tab indicator is now a single sliding `.rail-indicator` div with `z-index: 1` (transforms 220ms cubic-bezier between rows; fades to opacity 0 when active tab not in visible list, e.g. Diagnostics). Connected dot in TopBar pill breathes 2.6s ease (CSS in app.css, `prefers-reduced-motion` guarded). Page transitions in AppShell wrapped in `{#key active}` with `in:fly(y:6, duration:180, delay:90, quintOut)` + `out:fade(90)` for sequential drawer-slide-up between Browser/Activity/Conflicts/Settings.
- **Drift tab removed** — user confirmed nobody uses it (and the bucket-string-mismatch bug from S30 backlog had been silently broken). Deleted `DriftReview.svelte`, dropped tab entry from TabRail, removed branch from AppShell, removed Ctrl+3 mapping, renumbered Ctrl shortcuts (Browser=1, Activity=2, Conflicts=3, Settings=4). Drift ENGINE (`drift_scanner.rs`, `diag_force_drift_scan`, snapshot baseline, per-row Browser indicators, Sync modal results) all preserved — only the UI page is gone.

### Verify
- `svelte-check`: 0 errors, 2 pre-existing `<section>` a11y warnings (still on backlog).
- File count: 3991 → 3990 (DriftReview gone).
- No `cargo` rebuild needed — zero Rust changes this session.

### Flagged for v0.2.39+ (updated)
- **Pre-flight write probe** on connect — cheapest carry-over.
- **Activity-feed row grouping** — bulk reconciles spam 30+ identical rows.
- **`svelte-ignore` non-suppression** on `<section>` a11y warnings.
- **Remote `.rift-lock` cruft sweep** — `heal_owned_dirs`-style mirror on watch attach.
- **DELETED**: DriftReview bucket-string mismatch (page gone, bug moot).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`. Version pinned at **v0.2.38-alpha-test** (no bump this session; S31 was frontend/ops only).

**Current state (post S31):** v0.2.38 source on `origin/main` w/ S31 UI overhaul committed on top. **NOT yet published to `rift-releases`** — Blazzer still dev-testing. No-release directive holds: don't trigger `/git-ship` or any Velopack publish until explicitly cleared.

**Next session likely entry points:**
1. Blazzer sends after-screenshots from the UI overhaul — fine-tune anything off (logo size, collapsed-hero copy, sliding indicator timing, page-transition feel).
2. v0.2.38 dev-test continues — does the auto-sync rip kill the `[world]` ping-pongs?
3. If both hold: bump to v0.2.39 + ship pipeline (`/git-ship` user-invoked only).
4. Else: pick next backlog item.

**Orphan file** `scripts/bg-backlog.sh` is from an accidental cross-chat — left untracked locally, NOT committed. Safe to delete if Blazzer doesn't want it.

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
- **Source `.secrets/env.sh` first on ship/auth tasks** — Claude Code bash is non-interactive, won't auto-load
- **`last_scan_entries` is std::sync::Mutex** (NOT tokio) — `kick_drift_reconcile` is sync and called from notify event handler; tokio Mutex `blocking_lock` panics there. Don't "fix" it. (`current_scan_cancel` removed in v0.2.38 — no longer applies.)
- **`force_pull_now` does an inline drift scan + dispatches with the guard in front** (post-v0.2.38). Pre-v0.2.38 it dispatched from cache (drift_watcher's 10s tick kept cache fresh). With the tick deleted, cache freshness isn't guaranteed → Pull Now must do its own scan. Don't "optimize" back to cache-only dispatch — that's how stale tombstones could fire mass deletes.
- **NEVER use `FileAttributes::default()` for SETSTAT** — it sends `size: Some(0)`, `mtime: Some(0)`, `atime: Some(0)`, `uid/gid: Some(0)` which the server honors → file truncation + epoch mtime. Always use `FileAttributes::empty()` and explicitly set only the fields you want to change. See v0.2.27 post-mortem.
- **`SftpClient::delete` routes by remote stat** — dirs go through `delete_recursive_via`. Don't shortcut back to `remove_file` for "files only" — the push pipeline can't distinguish file from dir deletes ahead of time. See v0.2.29.
- **`mkdir_p_via` chmods each segment to 2775** — setgid + group-writable is required for shared-group teammates to push into each other's dirs. Don't drop the SETSTAT call — backlog gets healed too via `heal_owned_dirs` on watch attach. See v0.2.31.
- **Upload pre-flight SHA-collapse before raising CONFLICT** — when sizes all match + baseline SHA exists, hash local first (cheap), then remote via SSH exec. If both match baseline, refresh baseline mtime + drop the push. Mtime jitter (npm builds, SETSTAT, git checkout) flooded Trey w/ 53 phantom conflicts in v0.2.31; v0.2.32 fixed. See `auto_sync.rs:1522`.
- **`DriftBucket::ToDelete` is the tombstone path** — `local + no remote + has_baseline` MUST classify as `ToDelete`, NOT `ToPush`. Without it, deletes from teammates leave ghost files locally + risk accidental resurrection (autosync re-uploads on next touch). Dispatcher routes ToDelete → `drift_watcher::delete_local_one`, which guards on foreign-lock + dirty-local (skip unflushed edits — never blow away user's work). Empty-parent-dir cleanup walks up post-delete. See v0.2.33 post-mortem.
- **All time displays use `hour12: true`** — Blazzer requires 12-hour everywhere. Any new `toLocaleTimeString`/`toLocaleString` call MUST pass `[], { hour12: true }` explicitly (locale-default emits 24-hour on non-US machines). See v0.2.34 audit.
- **Mass local-delete circuit breaker lives in `force_pull_now`** (post-v0.2.38). Same `(file_count * 0.30).clamp(5, 25)` formula, same `BLOCKED — N local-deletes` activity row, same `kind=block`. Don't relocate this guard — Pull is the ONLY path that propagates tombstones since auto-sync was ripped, so the guard MUST sit there. See v0.2.36 + v0.2.38.
- **DO NOT restore `drift_watcher::spawn`, `run_tick`, `flush_cycle`, `auto_flush_enabled`, `remote_scan_interval_secs`, `drift_watcher_task`, `flush_task`, `LOOP_TICK_MS`, `track_pull_handle`, `register_scan_cancel`, `clear_scan_cancel`** — all deleted intentionally in v0.2.38. Auto-sync ping-ponged `pulled → removed locally → pulled → removed locally` on `[world]` resources at 10s tick because drift_watcher classified entries as `ToDelete` one tick + `ToPull` the next while baseline/devbridge raced. The fix was to delete the auto path entirely. Push/Pull buttons only. Watcher still runs to populate dirty queue (StatusBar pending count) but nothing flushes until user clicks. See v0.2.38 post-mortem.
