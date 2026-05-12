# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 53 — 2026-05-12 — v0.2.49 foundation pass (in progress, not shipped)

**Status: Backend + frontend complete, builds green. Validation + ship queued for next session.**

### Root cause nailed (Bug 5 regression)
`SuspiciousEmptyAborted` guard fires on every [endure]/[ox]/[voice]/[cfx-default] bracket because Session 7's mass-cleanup left snapshot baselines frozen at ~600+ files while real remote now has 56–90. Guard correctly prevents phantom deletes — it just can't distinguish intentional cleanup from transient SFTP failure. Every new resource dropped into these brackets is invisible to Rift until rebaselined.

### Shipped this session (not yet versioned/committed)

**Item 1 — Rebaseline UX:**
- `sync_snapshot.rs` — `replace_under()` atomic rewrite
- `drift_scanner.rs` — `AbortedShrunkFolder` struct + `aborted_shrunk: Vec<...>` on `ScanResult`; `SuspiciousEmptyAborted { baseline_count, listing_count }` carries counts
- `sync/mod.rs` — re-exported `AbortedShrunkFolder`
- `auto_sync.rs` — `last_aborted_shrunk` engine state; reconcile caches + emits `BaselineShrinkDetected`; `aborted_shrunk()` accessor; `rebaseline_folder()` method + `walk_local_rebaseline` helper
- `diagnostics/mod.rs` — `BaselineShrinkDetected` + `BaselineRebaselined` stages
- `lib.rs` — `sync_get_aborted_shrunk` + `sync_rebaseline_folder` Tauri commands registered
- `sync-page.svelte.ts` — `AbortedShrunkFolder` type, state, `visibleAbortedShrunk`, `rebaseline()` / `confirm` / `dismiss` actions
- `SyncPage.svelte` — shrink banner per aborted bracket (warn-toned), "Why this matters" tooltip, inline confirm card, ok-banner w/ result delta

**Item 2 — Listing accuracy instrumentation:**
- `sftp/list.rs` — `list_via_exec` counts raw_lines vs emitted, skipped_short/bad_size/by_ext, samples; emits `RemoteScanResult` Warn diag + `eprintln!` when raw≠emitted. Surfaces the 5 dropped lines from [endure] for item 2 fix.

### Validation fixture
`endure_rifttest_diag1/test.lua` stays on local disk as the test target. After rebaseline, it + any other local-only files in the 4 shrunk brackets surface as ToPush.

### Files modified
- `src-tauri/src/state/sync_snapshot.rs`
- `src-tauri/src/sync/drift_scanner.rs`
- `src-tauri/src/sync/mod.rs`
- `src-tauri/src/sync/auto_sync.rs`
- `src-tauri/src/diagnostics/mod.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/sftp/list.rs`
- `src/lib/state/sync-page.svelte.ts`
- `src/lib/components/sync/SyncPage.svelte`

---

## Session 52 — 2026-05-12 — Push-reliability arc shipped + validated (v0.2.46 → v0.2.48)

**Status: VALIDATED end-to-end.** Cross-session Rescan after v0.2.48 confirmed full sync, zero pending, zero false deletes; `[endure]/endure_rifttest` (7 files local, 0 remote) surfaced + pushed cleanly. Quote: "Rift is now trustworthy enough that I can stop reaching for SSH on this codebase."

**Arc summary.** v0.2.46 closed Bug 2 (silent file drops — strict mkdir at `sftp/ops.rs::mkdir_p_strict_via` + batch pre-mkdir in `flush_batch`), Bug 3 (orphan `.rift-lock` files — release-on-every-terminal-path in `process_entry` wrapper + `path.is_file()` gate in `queue_path`), Bug 4 (dir-create race), plus F7 (`wait_for_readable` 3.2 s exponential backoff). v0.2.47 attempted Bug 5 via Created+Dir reconcile kick + added dormant `_disabled_*` prefix-ignore (Bug 6 was misdiagnosis — divergence was real Push-orphan leftover, not park-dir noise). v0.2.48 closed Bug 5 properly via 500 ms-debounced + `AtomicBool`-coalesced reconcile in `on_fs_event`, and closed Bug 7 (`build`/`dist` dropped from `ignored_directory_names()` — server-side `find -prune` was killing FiveM `web/build/` + `web/dist/` trees, producing 45 phantom ToDelete-local rows).

**Verify:** `cargo check` clean, `cargo test --lib sync::ignore` 11/11 pass. All four releases on `Blazzer10200/rift-releases`. Last commit `7dd7992`.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri (Rift). Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.48-alpha** (v0.2.49 work landed, not yet bumped/committed). Tauri 2 + Svelte 5 + Rust + russh/russh-sftp. Velopack updater, NSIS perUser installer.

**State:** v0.2.49 foundation pass is code-complete + builds green. NOT yet shipped (no version bump, no CHANGELOG, no commit). Validation before ship.

**FIRST ACTION next session:**
1. Launch Rift dev (`scripts/run-dev.bat`), connect Endure RP, Rescan
2. Confirm 4 shrunk-bracket banners appear: [endure], [ox], [voice], [cfx-default]
3. Rebaseline [endure] — confirm `endure_rifttest_diag1/test.lua` surfaces as ToPush
4. Check Diagnostics panel for `remote_scan_result` Warn event naming the 5 dropped lines (item 2 data)
5. If all good → `/git-ship` v0.2.49

**Item 2 (listing accuracy):** 5 files vanish inside `list_via_exec` parse loop on [endure]. Instrumentation now logs raw_lines vs emitted. Root cause not yet known (tab-in-filename or newline most likely). Fix after validation confirms item 1 working — same release.

**v0.2.50:** Mirror mode (Bug 1) — `local-missing + remote-has + baseline-exists` → remote-delete bucket, Mirror toggle, confirm-deletes prestep. Integration test suite (phase 1) gates this.

**Smaller queued:** stale-lock sweep button, mass-delete guard tune, Terminal Settings sub-view, Appearance controls.

**Don't reintroduce:** OpRail, TopBar (merged), rail kbd hints, StatusBar ⌘K pip, titlebar gear, StatusHero big H1, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn`/`.vdivider` dead CSS, `bg-backlog.sh`, `diag_*` cmd names (renamed to `sync_*` in S44), `drift_watcher::spawn` / `run_tick` / `flush_cycle` (deleted v0.2.38 — auto-path ping-ponged; Push/Pull buttons only).

---

## CRITICAL DON'T-TOUCH

- russh `ring` backend + reqwest `rustls` only (NASM blocks aws-lc-rs).
- `~/.rift/*.json` compat — don't change rename rules; keep `serde(flatten) extra`.
- `VelopackApp::build().run()` MUST be first call in `lib.rs::run()`.
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver).
- DriftWatcher conflict-rename guard — never overwrite dirty local.
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it.
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo.
- `path_guard.rs` API frozen — `edit/in_place.rs` + lib cmds depend.
- `rename_via` strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- Source `.secrets/env.sh` first on ship/auth tasks — non-interactive bash won't auto-load.
- `last_scan_entries` is `std::sync::Mutex` (NOT tokio) — called from notify handler; tokio `blocking_lock` panics.
- `force_pull_now` does inline drift scan + dispatches w/ guard. Don't optimize to cache-only — stale tombstones fire mass deletes.
- `force_push_now` (v0.2.43+) promotes scan ToPush → dirty, auto-scans on cold cache, clears cache after non-cancelled push.
- **NEVER `FileAttributes::default()` for SETSTAT** — sends zeros → file truncation + epoch mtime. Use `FileAttributes::empty()` + set only fields you want.
- `SftpClient::delete` routes by remote stat — dirs go through `delete_recursive_via`. Don't shortcut to `remove_file`.
- `mkdir_p_via` chmods each segment to 2775 (setgid + group-writable) for shared-group teammate pushes. Don't drop SETSTAT.
- Upload pre-flight SHA-collapse before raising CONFLICT (v0.2.32).
- `DriftBucket::ToDelete` = `local + no remote + has_baseline`. Routes to `delete_local_one` (guards on foreign-lock + dirty-local + empty-parent walk-up).
- All time displays MUST pass `[], { hour12: true }` — locale-default emits 24h on non-US.
- Mass local-delete circuit breaker: `(file_count * 0.30).clamp(5, 25)`. v0.2.45 explicit-user-selection from Sync page bypasses w/ WARN log; scan-driven paths still hard-block.
- `spawn_frontend_pump` 200/s rate-limit; critical stages bypass. Don't fold critical events back into throttled path.
- russh `Config { keepalive_interval: 20s, keepalive_max: 3, window_size: 2 MiB, maximum_packet_size: 32 KiB }` in both `sftp::open_session` + `tunnel::start`.
- **v0.2.46+ data-integrity stack — DO NOT REGRESS:** `mkdir_p_strict_via` (loud parent-dir creation failure), batch pre-mkdir in `flush_batch` (eliminates worker-race on fresh trees), lock release on every `process_entry` terminal path, `path.is_file()` gate in `queue_path` lock-acquire, `wait_for_readable` 6×exp-backoff (~3.2 s).
- **v0.2.48 ignore symmetry — DO NOT REGRESS:** `ignored_directory_names()` excludes `build` + `dist`; remote `find -prune` would otherwise kill FiveM `web/build/` + `web/dist/` ui_page bundles.
- **v0.2.48 Created+Dir debounce — DO NOT REGRESS:** 500 ms delay + `pending_dir_reconcile: AtomicBool` coalesce around `kick_drift_reconcile` in `on_fs_event`. Immediate kick fires before files land on disk.
