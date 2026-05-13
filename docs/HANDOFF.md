# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 56 — 2026-05-13 — Repo cleanup pass (branch `cleanup/full-audit`)

**Status: BRANCH OPEN, NOT MERGED.** All work on `cleanup/full-audit`. Verify: `cargo check` clean, **46/46 backend tests pass**, **svelte-check 0/0 across 3996 files**, vitest 6/6 pass.

**auto_sync.rs split executed** (codex-2026-05-12 item 10 deferred work). 2933 → 1954 lines (-33%). `try_watch` / `stop_watch` / `on_fs_event` / `queue_path` / `mark_recently_written` / `is_recently_written` moved to `sync/auto_sync/watch.rs` (327 L). `flush_batch` / `process_entry` / `process_entry_body` / `mark_failed` + `EntryResult` enum moved to `sync/auto_sync/flush.rs` (610 L). Submodule privacy gives access to parent's private fields w/o pub-shims. `path.rs` (160 L) untouched. Main `auto_sync.rs` keeps engine struct, start/stop, drift reconcile, force_pull/push, status surface, log helpers.

**Other cleanup (in order):** (a) `Releases/` pruned to last 2 versions — freed ~249 MB (gitignored, no commit); (b) `components.json` deleted (shadcn scaffold abandoned, aliases `$lib/components/ui` and `$lib/hooks` never existed); (c) 13 dead shadcn CSS aliases removed from `app.css` lines 76-90 (`--background`/`--card`/`--popover`/`--primary`/`--secondary`/`--destructive`/`--input`/+ foregrounds); kept `--muted` (used by `SyncPage.svelte:503`); (d) `@vitest/coverage-v8` removed from package.json devDeps (declared, never invoked); (e) `docs/audit/` consolidated — 8 stale audit files → `AUDIT-ARCHIVE.md` (resolved) + `AUDIT-OPEN.md` (still-outstanding); stale `DriftReview.svelte` refs dropped (file removed in `79f6fae`); (f) deleted `Releases/build-v0.2.45-alpha.log` stale log.

**Not done (explicit defer):** lib.rs split (1747 L, 52 commands) — needs per-domain `commands/*.rs` design; `reqwest` + `ureq` consolidation — blocked on velopack 0.0.1298's sync `UpdateSource`; LocalPane/RemotePane shared-logic extraction — pair w/ scan-frontend HIGH-sev stale-closure fixes.

**Working tree:** `cleanup/full-audit` branch, unstaged. Next action: review diff, commit, optionally merge to main.

---

## Session 55 — 2026-05-13 — v0.2.51 → v0.2.53 SHIPPED

Three releases today: `d51e0c7` v0.2.53 (Mirror + auto-reconnect), `e29543c` v0.2.52 (notify-rs rename-event + state-machine + watched-root vanish poll), `b47fef9` v0.2.51 ("Disconnected" relabel hotfix). Velopack at `Blazzer10200/rift-releases`. Full per-version detail in `git log`. Prod-side: `[endure]`/`[ox]`/`[community]`/`[depend]` subdirs chowned `blazzer:fxserver drwxrwsr-x` — fxserver.service still runs as root, recurrence expected until v0.2.54 fxserver-user fix.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri. Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.53-alpha** SHIPPED (`d51e0c7`). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**FIRST ACTION:** confirm Rift auto-updated to 0.2.53; verify push to any bracket works post-chmod (test with a small edit + save in `[endure]` + check Diagnostics for green sync rows, no red "create tmp" failures).

**v0.2.54 queue:** (a) **Edit fxserver.service** on prod to `User=fxserver` + `Group=fxserver` — kills the chmod-recurrence root cause; (b) Rift-side EACCES auto-fix-perms affordance — detect "Permission denied" on create-tmp and surface a "Fix prod perms?" button that runs the chown+chmod via existing SSH session; (c) auto-Mirror on detected rename only (when notify pairs `Name(From)+Name(To)` w/ matching basenames within debounce window, silent remote-delete; mysterious local-missing still requires typed confirm); (d) integration test suite phase 1 (10 mock-SFTP scenarios — needs SftpClient trait abstraction or testcontainers); (e) Dry-run Mirror preview pre-confirm.

**Smaller queued:** mass-delete guard tune, Terminal Settings, Appearance controls.

**Multi-user warning.** Trey is on a ~2wk stale Rift baseline. He auto-picks up v0.2.51-v0.2.53 on next Rift launch. Keep him OFF Mirror until he's on latest + has fresh-Pulled baseline.

**Don't reintroduce:** OpRail, TopBar (merged), rail kbd hints, StatusBar ⌘K pip, titlebar gear, StatusHero big H1, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn`/`.vdivider` dead CSS, `bg-backlog.sh`, `diag_*` cmd names, `drift_watcher::spawn`/`run_tick`/`flush_cycle`.

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
- Source `.secrets/env.sh` first on ship/auth tasks.
- `last_scan_entries` is `std::sync::Mutex` (NOT tokio) — notify handler context.
- `force_pull_now`/`force_push_now` invariants preserved (v0.2.43).
- **NEVER `FileAttributes::default()` for SETSTAT** — sends zeros → truncation + epoch mtime. Use `empty()`.
- `SftpClient::delete` routes by remote stat — dirs go through `delete_recursive_via`.
- `mkdir_p_via` chmods each segment to 2775 for shared-group pushes.
- Upload pre-flight SHA-collapse before raising CONFLICT (v0.2.32).
- `DriftBucket::ToDelete` = local+no-remote+has-baseline → delete LOCAL. `DriftBucket::ToDeleteRemote` (v0.2.53) = local-missing+remote-has+has-baseline + mirror-on → delete REMOTE.
- Time displays MUST pass `[], { hour12: true }` — locale-default emits 24h on non-US.
- Mass local-delete circuit breaker: `(file_count * 0.30).clamp(5, 25)`. Sync-page explicit-user-selection bypasses. ToDeleteRemote bypasses (user reached via typed-MIRROR gate).
- `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- russh `Config { keepalive_interval: 20s, keepalive_max: 3, window_size: 2 MiB, maximum_packet_size: 32 KiB }` in both `sftp::open_session` + `tunnel::start`.
- **v0.2.46+ data-integrity stack — DO NOT REGRESS:** `mkdir_p_strict_via`, batch pre-mkdir in `flush_batch`, lock release on every `process_entry` terminal path, `path.is_file()` gate in `queue_path`, `wait_for_readable` 6×exp-backoff.
- **v0.2.48 ignore symmetry — DO NOT REGRESS:** `ignored_directory_names()` excludes `build`+`dist` for FiveM `web/build/`+`web/dist/` ui_page bundles.
- **v0.2.48 Created+Dir debounce — DO NOT REGRESS:** 500 ms + `pending_dir_reconcile: AtomicBool` coalesce.
- **v0.2.50 connection-reliability stack — DO NOT REGRESS:** `sftp/transfer.rs::with_t` op timeouts (T_QUICK 10s / T_NORMAL 30s / T_BODY 120s) on every SFTP op + LIST_T 120s on listing; `ConnectionWedged` diag emit on timeout; `process_entry` terminal lock-release is INLINE await w/ 5s timeout (NOT `tokio::spawn`); `sync/ignore.rs` `.tmp.<pid>.<hex>` rule tight-matched (pid ≤8 digits, hash ≥8 hex, no 3rd dot-seg); `sync_sweep_stale_locks` ONLY clears own-user locks via `LockPresence::sweep_stale_mine`.
- **v0.2.52 watcher + state-machine — DO NOT REGRESS:** explicit `Modify(ModifyKind::Name(RenameMode::From))→Deleted` + `RenameMode::To→Created` arms (Windows notify never emits `RenameMode::Both`); `consecutive_failed_batches` threshold 3 before Error escalation (single fails stay `Watching` w/ retry-pending detail); 5s watched-root-vanish poll w/ de-dup HashSet for issue #403.
- **v0.2.53 Mirror + auto-reconnect — DO NOT REGRESS:** Mirror mode is session-scoped (`mirror_mode: AtomicBool`), resets on engine restart by design — don't persist. UI typed-confirm gate requires literal "MIRROR" before Confirm enables. Auto-reconnect rolling-window threshold = 3 wedges in 60s w/ `reconnecting` guard (no overlap); client-side only — no engine `Arc<SftpClient>` refactor.
