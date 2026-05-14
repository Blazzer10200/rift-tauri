# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 59 — 2026-05-14 — v0.2.55-alpha Sync overhaul SHIPPED

**Status: SHIPPED on main, Velopack tag `v0.2.55-alpha` live.**

### Completed
- **Pull/Push rescan-after-dispatch** — `pullAll`/`pushAll`/`applySelected`/`confirmMirrorApply` chain to `rescan()` not `refresh()`. Pushes no longer hidden after Pull all completes.
- **Auto-scan on first connect** — `AppShell` $effect watches `connection.status.state` → fires `syncPage.maybeAutoScan(key)` once per server-key when watcher ready. Latch cleared on disconnect.
- **One-button Sync (pull then push)** — replaced separate Pull/Push buttons w/ single primary `Sync (N↓ M↑)`. Sequences `sync_pull_pending` → 2.5s drain → `sync_push_pending` → 1.2s → rescan. Phase labels live (`Pulling…` / `Pushing…`). Conflicts + Mirror remote-deletes stay gated.
- **Auto-rescan periodic** — kebab cycle `off→30s→1m→2m→5m→10m→off`, localStorage-persisted. Timer lives in AppShell (survives tab switches), gated busy/preview/disconnect. `$effect` cleanup tears down on toggle/interval change.
- **Tab-switch flash fix** — dropped `{#key active}` + `in:fly`+`out:fade`. Lazy-mount + keep-alive: each page mounts once on first visit, `hidden` attr toggles visibility. No remount → no transition cascade → no flash. Inner re-keys (`settingsSection`, `selectedConflict`) preserved.
- **Sync page reskin Phase A** — hero compacted to `[⋯][↻][Apply Mirror (cond)][Sync]`. Kebab w/ Mirror toggle / Auto-rescan / Sweep / Advanced (Pull-only, Push-only) / Design preview. Kebab anchored `right: 0` (was `left: 0` → viewport clip).
- **Two-line entry rows** — path + size line 1, reason + relative mtime line 2. `formatSize`/`formatMtimeRel` helpers.
- **Selection breakdown footer** — tone-tinted `2 push · 2 pull · 1 delete` replaces generic hint.
- **Empty-state subtitle + ghost rescan** — Last scan + folder count + `Rescan now` button.
- **Design preview fixture** — kebab toggle injects 9-entry fixture across 3 resources for UI review, dispatch gated.

### Key Decisions
- syncNow uses timing-based drain (2.5s) not event-driven — backend has no hard pull-complete signal yet. Refine later if tight in practice.
- Auto-rescan opt-in default OFF — most users on a single-machine workflow don't need it. Teams w/ remote teammates flip it on.
- Lazy-mount visited Set + `|| active === X` fallback covers same-render-frame race.

### Files Modified
- `src/lib/components/AppShell.svelte`, `src/lib/components/sync/SyncPage.svelte`
- `src/lib/state/sync-page.svelte.ts`
- Version bump THREE files: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` → 0.2.55-alpha

### Next Steps
1. Test installed build: tab switches (no flash), auto-rescan cycling, Sync button mid-flight, Preview toggle.
2. v0.2.56 queue: connecting-pill desync (item h), EACCES auto-fix (item a), dry-run Mirror preview (item d), integration tests (item c), lib.rs split (item e).

---

## Session 58 — 2026-05-14 — Terminal UI overhaul (shipped via S59 batch)

Terminal: borderless-max clip fix, window-resize reflow, HMR tab-explosion fix, Settings → Terminal panel (font/cursor/scrollback/bell/themes), `@xterm/addon-search` (Ctrl+F), QoL (inline rename, file-drop paste, Ctrl+Shift+[/]/T, clear button). Custom Rift-themed dropdowns + slider replace native `<select>`. Full detail: `git log -- docs/HANDOFF.md`.

---

## Session 57 — 2026-05-13 — v0.2.54-alpha SHIPPED — see `git log -- docs/HANDOFF.md`

Trey onboarding hotfix: fresh-install bootstrap (`try_watch` now `mkdir_all`s missing subdirs) + titlebar dropdown clip. Trey's profile: Tailscale host `100.122.178.19`, user `treyday`, remoteRoot `/opt/fxserver/server/txData/Qbox_F8F761.base/resources`.

## RESUME HERE — first read every new session

**Project:** rift-tauri. Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.55-alpha** SHIPPED. Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**v0.2.56 queue (carried from v0.2.55 minus shipped items):** (a) Rift-side EACCES auto-fix-perms affordance — detect "Permission denied" on create-tmp and surface a "Fix prod perms?" button that runs chown+chmod via existing SSH session; (b) auto-Mirror on detected rename only (when notify pairs `Name(From)+Name(To)` w/ matching basenames within debounce window, silent remote-delete; mysterious local-missing still requires typed confirm); (c) integration test suite phase 1 (10 mock-SFTP scenarios — needs SftpClient trait abstraction or testcontainers); (d) Dry-run Mirror preview pre-confirm; (e) `lib.rs` split (1747 L, 52 commands) — needs per-domain `commands/*.rs` design; (f) `reqwest` + `ureq` consolidation — blocked on velopack 0.0.1298's sync `UpdateSource`; (g) LocalPane/RemotePane shared-logic extraction; (h) connection.connecting pill desync from status.state — pill stuck "Connecting" while engine reports `watching`; add derived guard so `state in {watching,idle,syncing}` overrides `connecting`.

**Multi-user warning.** Trey: keep him OFF Mirror until he's on latest + fresh-Pulled baseline. v0.2.55 introduces auto-rescan (off by default) — safe for him to receive.

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
