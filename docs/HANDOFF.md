# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 54 — 2026-05-12 — v0.2.50 SHIPPED (connection-reliability foundation)

**Status: SHIPPED.** Source `5cb20e5` on main, pushed. Velopack release live at https://github.com/Blazzer10200/rift-releases/releases/tag/v0.2.50-alpha (3-file delta from v0.2.49). v0.2.49 sanity check failed during validation — orphan locks + wedged uploads — pivoted to fix foundation before Mirror mode.

### Diagnosed + fixed
1. **Editor-tmp ignore.** Patterns like `client.lua.tmp.9076.310b94f68378` captured by watcher → acquired remote `.rift-lock` → upload failed (editor renamed away) → orphan locks. `classify()` extended w/ tight `.tmp.<pid>.<hex>` rule (pid ≤8 digits, hash ≥8 hex, no third dot-seg). Added `.crswap`/`.crdownload`. **Root cause of recurring orphan locks.**
2. **SFTP op-level timeouts.** `with_t()` in `sftp/transfer.rs` wraps every op — `T_QUICK` 10 s (cleanup/close/set_metadata), `T_NORMAL` 30 s (mkdir/rename/create), `T_BODY` 120 s (write/read body). `sftp/list.rs` LIST_T 120 s on exec/serial/worker paths. Timeout → wedged-connection error + `DiagStage::ConnectionWedged` emit.
3. **Lock-release race.** `process_entry` terminal release: `tokio::spawn`+`track_background` → inline-await w/ 5 s timeout. Spawn could be aborted by engine `stop()` before SFTP delete fired (probe2.txt repro).
4. **Sweep locks button.** New `sync_sweep_stale_locks` Tauri cmd + button next to Rescan. Walks watched roots, reclaims own-locks older than STALE_SEC.
5. **Prod perms fix (out-of-band).** Chowned 9× `[qbx]/qbx_*` from `root:root drwxr-xr-x` → `blazzer:fxserver drwxrwsr-x` — source of "create tmp Qbox_F8F76…" red errors. `mkdir_p_via` chmods only on dir CREATION, not pre-existing.

### Verify
`cargo check` 5.06 s clean · `cargo test --lib` 46 passed (incl. new `editor_tmp_pid_hex` + `cr_swap_and_download`) · `svelte-check` 0/0 across 3996 files · release build 1m 08s · `vpk pack` 5.1 s · GH release published as pre-release.

### Note re v0.2.49 listing-accuracy
56≠61 mismatch never reproduced on validation Rescan (got 0 ToPush, both sides 61). Instrumentation kept in place for recurrence. Closing as "fixed in effect."

---

## Session 52-53 — 2026-05-12 — Push-reliability + Rebaseline arc shipped (v0.2.46 → v0.2.49)

**Arc summary.** v0.2.46 Bug 2/3/4 fixes (strict mkdir, orphan-lock release-on-terminal-path, wait_for_readable backoff). v0.2.47 dormant `_disabled_*` ignore. v0.2.48 Bug 5 + Bug 7 (Created+Dir debounce w/ AtomicBool coalesce, web/build prune-exclusion). v0.2.49 SuspiciousEmptyAborted rebaseline UX (banner per shrunk bracket, "Why this matters" tooltip, `replace_under()` atomic snapshot rewrite, `sync_get_aborted_shrunk`+`sync_rebaseline_folder` cmds) + `list_via_exec` raw-vs-emitted instrumentation. All validated end-to-end on Endure.

**Verify:** all releases on `Blazzer10200/rift-releases`. Last source commit before v0.2.50: `8f38806`.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri (Rift). Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.50-alpha** SHIPPED (commit `5cb20e5`, Velopack live). Tauri 2 + Svelte 5 + Rust + russh/russh-sftp. Velopack updater, NSIS perUser installer.

**State:** v0.2.50 connection-reliability foundation shipped + pushed. Existing v0.2.49 installs auto-update via 3-file delta. Endure validation NOT yet done on shipped binary — first action.

**FIRST ACTION next session:**
1. Launch Rift — confirm auto-update lands you on 0.2.50-alpha
2. Connect Endure RP, edit a file in VSCode/editor, save, watch Diagnostics — confirm zero `editor-tmp(.tmp.<pid>.<hex>)` rows now bucket as "skipped: file locked or unreadable" (they should be Ignored cleanly upstream)
3. SSH-verify no orphan `.rift-lock` files left in `[endure]` or `[qbx]` after a normal edit session
4. Click "Sweep locks" button → confirm any straggler orphans clear
5. If all clean → v0.2.50 validated, move to v0.2.51 planning

**v0.2.51 queue (deferred from v0.2.50):**
- Full auto-reconnect w/ 3× exponential backoff on `DiagStage::ConnectionWedged` (currently surfaces error + user clicks Sweep + manual reconnect via server switcher)
- Mirror mode for Push-all (Bug 1) — `local-missing + remote-has + baseline-exists` → remote-delete bucket, Mirror toggle, typed-confirm gate, dry-run preview default
- Integration test suite (phase 1, 10 scenarios w/ mock SFTP) — gates Mirror ship
- Listing-accuracy targeted fix if 56≠61 mismatch ever reproduces (v0.2.49 instrumentation still live)

**Smaller queued:** mass-delete guard tune, Terminal Settings sub-view, Appearance controls. Force-reconnect button (skipped tonight; user has manual disconnect+reconnect via server switcher).

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
- **v0.2.50 connection-reliability stack — DO NOT REGRESS:** `sftp/transfer.rs::with_t` op-level timeouts (T_QUICK 10 s / T_NORMAL 30 s / T_BODY 120 s) on every SFTP op + LIST_T 120 s on listing; `ConnectionWedged` diag emit on timeout; `auto_sync::process_entry` terminal lock-release is INLINE await w/ 5 s timeout (NOT `tokio::spawn` — was source of orphan-lock race); `sync/ignore.rs` `.tmp.<pid>.<hex>` rule tight-matched (pid ≤8 digits, hash ≥8 hex chars, no third dot-segment) — broadening would catch legit files; `sync_sweep_stale_locks` cmd uses `LockPresence::sweep_stale_mine` so it ONLY clears own-user locks, never foreign.
