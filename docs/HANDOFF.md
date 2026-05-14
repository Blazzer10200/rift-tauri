# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 57 — 2026-05-13 — v0.2.54-alpha SHIPPED (Trey onboarding hotfix)

**Status: SHIPPED on main (`09c0a81`), Velopack tag `v0.2.54-alpha` live.** Cleanup branch merged.

Trey onboarding surfaced two blockers, both fixed + shipped same session:

1. **Fresh-install bootstrap** — `auto_sync::try_watch` was returning `Ok(false)` silently when per-folder local subdirs didn't exist, leaving fresh installs with `watches=0` and Sync page falsely showing "Everything in sync" against empty-local + populated-remote. Now: `mkdir_all`s the missing subdir when profile `local_root` exists, logs to diagnostics, attaches watcher normally. Profile `local_root` missing still bails (genuine config error). Drift scanner now sees remote tree → ToPull entries surface → Pull all works as documented in ONBOARDING.md.
2. **Titlebar server-picker dropdown clipped** — `.left` had `overflow: hidden` for drag-region containment, which also clipped the dropdown menu vertically. Moved overflow constraint to child spans via `text-overflow: ellipsis` on `.svr-name` / `.svr-host`; dropdown renders unblocked. Defensive `z-index: 100→1000`.

**Server-side check for Trey** — fully ready. Trey's SSH key active on both `/home/blazzer/.ssh/authorized_keys` AND `/home/treyday/.ssh/authorized_keys` (CT 120). `treyday` user (uid 1001) in `fxserver` group. All 4 chowned brackets clean (`drwxrwsr-x blazzer:fxserver`, zero root-owned, zero non-g+w). **HANDOFF v0.2.54 item (a) was already done** — `fxserver.service` already runs `User=fxserver Group=fxserver`, NOT root. Stale claim removed from queue.

**Trey's profile** — use Tailscale host `100.122.178.19`, user `treyday`, remoteRoot `/opt/fxserver/server/txData/Qbox_F8F761.base/resources`. Bridge token optional (set `rift_bridge_token "3QeHEnvFAYJKJEdX3MNNX10ZPwnFJ6jr8jUvYX+uc38="` if wanted later, port 30120).

## RESUME HERE — first read every new session

**Project:** rift-tauri. Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.54-alpha** SHIPPED (`09c0a81`). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**FIRST ACTION:** confirm Trey auto-updated to 0.2.54 + completed initial Pull all. After Pull all completes, sync activity should be live both directions (his pushes land cleanly, your edits on `[endure]` brackets land on his next bg fetch).

**v0.2.55 queue (carried from v0.2.54 minus shipped items):** (a) Rift-side EACCES auto-fix-perms affordance — detect "Permission denied" on create-tmp and surface a "Fix prod perms?" button that runs chown+chmod via existing SSH session; (b) auto-Mirror on detected rename only (when notify pairs `Name(From)+Name(To)` w/ matching basenames within debounce window, silent remote-delete; mysterious local-missing still requires typed confirm); (c) integration test suite phase 1 (10 mock-SFTP scenarios — needs SftpClient trait abstraction or testcontainers); (d) Dry-run Mirror preview pre-confirm; (e) `lib.rs` split (1747 L, 52 commands) — needs per-domain `commands/*.rs` design; (f) `reqwest` + `ureq` consolidation — blocked on velopack 0.0.1298's sync `UpdateSource`; (g) LocalPane/RemotePane shared-logic extraction — pair w/ scan-frontend HIGH-sev stale-closure fixes; (h) connection.connecting pill desync from status.state — observed in Trey screenshot 2026-05-13 (pill stuck "Connecting" while engine reports `watching`); add derived guard so `state in {watching,idle,syncing}` overrides `connecting`.

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
