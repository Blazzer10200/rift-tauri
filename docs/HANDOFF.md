# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 52 — 2026-05-12 — Push-reliability arc shipped + validated (v0.2.46 → v0.2.48)

**Status: VALIDATED end-to-end.** Cross-session Rescan after v0.2.48 confirmed full sync, zero pending, zero false deletes; `[endure]/endure_rifttest` (7 files local, 0 remote) surfaced + pushed cleanly. Quote: "Rift is now trustworthy enough that I can stop reaching for SSH on this codebase."

**Arc summary.** v0.2.46 closed Bug 2 (silent file drops — strict mkdir at `sftp/ops.rs::mkdir_p_strict_via` + batch pre-mkdir in `flush_batch`), Bug 3 (orphan `.rift-lock` files — release-on-every-terminal-path in `process_entry` wrapper + `path.is_file()` gate in `queue_path`), Bug 4 (dir-create race), plus F7 (`wait_for_readable` 3.2 s exponential backoff). v0.2.47 attempted Bug 5 via Created+Dir reconcile kick + added dormant `_disabled_*` prefix-ignore (Bug 6 was misdiagnosis — divergence was real Push-orphan leftover, not park-dir noise). v0.2.48 closed Bug 5 properly via 500 ms-debounced + `AtomicBool`-coalesced reconcile in `on_fs_event`, and closed Bug 7 (`build`/`dist` dropped from `ignored_directory_names()` — server-side `find -prune` was killing FiveM `web/build/` + `web/dist/` trees, producing 45 phantom ToDelete-local rows).

**Verify:** `cargo check` clean, `cargo test --lib sync::ignore` 11/11 pass. All four releases on `Blazzer10200/rift-releases`. Last commit `7dd7992`.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri (Rift). Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.48-alpha**. Tauri 2 + Svelte 5 + Rust + russh/russh-sftp. Velopack updater, NSIS perUser installer.

**State:** sync engine is now trustworthy on the data-integrity axis. Push uploads no longer drop files silently, orphan locks no longer leak, FiveM web bundles diff correctly, new resource subtrees surface within 500 ms.

**Top-of-queue for v0.2.49:** Mirror mode for Push-all (Bug 1) — needs new drift bucket for `local-missing + remote-has + baseline-exists` → propose remote-delete + Mirror toggle in Sync hero + confirm-deletes step pre-dispatch. Twice-bitten in this arc by Push not propagating deletes. Significant classification change + frontend work.

**Smaller queued:** stale-lock sweep UI button (existing `sweep_stale_mine` handles own-user on watcher attach, manual button is DX polish). Mass-delete guard fine-tune (still open from v0.2.45). Terminal Settings sub-view (APIs ready on `terminal.svelte.ts`). Density/font/accent-tint Appearance controls.

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
