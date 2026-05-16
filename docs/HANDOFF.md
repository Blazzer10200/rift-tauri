# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 68 — 2026-05-15 — v0.2.56-alpha SHIPPED — UI polish + privacy scrub

**Status: SHIPPED.** Source pushed `687edb8`. Velopack release live at `https://github.com/Blazzer10200/rift-releases/releases/tag/v0.2.56-alpha` (prerelease, Setup.exe + delta from v0.2.55). Both `svelte-check` + `cargo check` clean.

### Completed (this session)
- **TabRail polish** — kbd hints slimmed (⌘N → digit `N`), Assistant `BETA` chip, pin-open button (chevron click locks rail at 220px, content reflows via `--rail-w` CSS var, persists `rift.ui.rail-pinned.v1`). Only SVG rotates on hover, button body stays still.
- **Files tab drag-reorder** — pointer events (HTML5 DnD flaky in webview2). Live shuffle during drag via `animate:flip` (220ms); dragged tab gets scale(1.04) + accent ring + shadow. Idle hover 1px lift on 2+ tabs (`:has`).
- **Sync shrink-banner collapsible** — collapsed by default (resource name + `142 → 38` chip); click head to expand explainer + Rebaseline/Dismiss.
- **Sync empty-state** — `{#if !isEmpty}` guard, dead "Apply selected" footer gone.
- **About page** — Paths section (Config + Logs w/ "Open" via plugin-opener) + Diagnostics ("Copy diagnostic info").
- **Privacy scrub** on diagnostic copy: paths → `<user>` placeholder, drop full `navigator.userAgent`, redact active server name. Audited: no email / real name / IPs / Trey info / telemetry in source. Fully standalone.
- Misc copy: Edit Server subhead "FiveM dev server" → "dev server"; Settings Assistant α-refs dropped.
- v0.2.56-alpha cut + Velopack upload (delta from v0.2.55 generated: 5 files, 3 patched).

### Key Decisions
- Pointer events > HTML5 DnD inside webview2 — `<button>` children swallow mousedown, drag never inits. Tried twice. Use pointer events for any future drag UX.
- Live reorder during drag (not on-drop) — feels native. `animate:flip` handles slide animations free.
- Scrubbed full UA from copy diagnostic — leaks Windows build + webview2 version. `navigator.platform` stays.

### Failed / Don't Retry
- HTML5 DnD w/ `<button>` inside draggable `<div>` — Chromium webview eats it. Pointer events only.
- Calling `/git-ship` via Skill tool — `disable-model-invocation` blocks. Run `scripts/release.ps1` directly (it's the full pipeline: build → vpk pack → upload to releases repo). `/git-ship` itself is source commit+push only.

### Next Steps
1. **Confirm delete `src/lib/components/assistant/ToolCallCard.svelte`** — orphaned since S63, zero imports. User ok needed.
2. v0.2.57 queue (carried — see below).

### Files Modified
TabRail.svelte, ui-prefs.svelte.ts, AppShell.svelte, SyncPage.svelte, AddServer.svelte, Settings.svelte, TwoPane.svelte, browser-tabs.svelte.ts. Versions bumped in package.json + Cargo.toml + tauri.conf.json. CHANGELOG.md extended w/ full S60-68 arc.

---

## Older sessions (shipped — full detail in git log)

- **S60-66** — AI Assistant page (auth, MCP tools, TodoWrite dock, conversation history, slash cmds, markdown renderer, workspace decoupling, state-aware EmptyState). Tab at Ctrl+3.
- **S67** — Canonical page skeleton (PageHeader/PageToolbar/PageFooter/EmptyState primitives). 5 pages converted. Titlebar declutter, TabRail 3-group rework + RIFT wordmark.
- **S59** — v0.2.55-alpha: one-button Sync, auto-rescan, keep-alive tabs (no flash on switch).
- **S58** — Terminal UI overhaul (Settings panel, search Ctrl+F, themes).
- **S57** — v0.2.54-alpha: fresh-install bootstrap hotfix for Trey onboarding.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri. Path `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.56-alpha SHIPPED** (`687edb8` on main). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater (release repo `Blazzer10200/rift-releases`), NSIS perUser installer.

**Immediate:** confirm-delete `assistant/ToolCallCard.svelte` (orphan since S63).

**v0.2.57 queue:** (a) Rift-side EACCES auto-fix-perms affordance — detect "Permission denied" on create-tmp, surface "Fix prod perms?" button running chown+chmod via existing SSH session; (b) auto-Mirror on detected rename only (notify pairs `Name(From)+Name(To)` w/ matching basenames in debounce window, silent remote-delete; mysterious local-missing still needs typed confirm); (c) integration test suite phase 1 (10 mock-SFTP scenarios — needs SftpClient trait abstraction); (d) dry-run Mirror preview pre-confirm; (e) `lib.rs` split (1747 L, 52 cmds) — needs per-domain `commands/*.rs` design; (f) `reqwest`+`ureq` consolidation — blocked on velopack 0.0.1298 sync `UpdateSource`; (g) LocalPane/RemotePane shared-logic extraction; (h) connection.connecting pill desync — pill stuck "Connecting" while engine reports `watching`; add derived guard so `state in {watching,idle,syncing}` overrides `connecting`; (i) Diagnostics page canonical-skeleton conversion (hidden Ctrl+Shift+D tab, lowest priority); (j) Appearance settings — fill or hide; currently dead "Coming soon" wall.

**Multi-user warning.** Trey: keep him OFF Mirror until on latest + fresh-Pulled baseline. v0.2.56 introduces Assistant tab (BETA, safe for him) + drag-reorder + pin-rail (UX-only, safe).

**Don't reintroduce:** OpRail, TopBar (merged), rail kbd hints as bulky chips, StatusBar ⌘K pip, titlebar gear, StatusHero big H1, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn` legacy dead CSS, `bg-backlog.sh`, `diag_*` cmd names, `drift_watcher::spawn`/`run_tick`/`flush_cycle`, FiveM-specific framing in Assistant copy.

**Ship-pipeline reminder.** `scripts/release.ps1` IS the full ship (build → vpk pack → upload to rift-releases). `/git-ship` is only source commit+push. Run `release.ps1` directly via `powershell -NoProfile -File ./scripts/release.ps1` after version bump.

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
- **v0.2.52 watcher + state-machine — DO NOT REGRESS:** explicit `Modify(ModifyKind::Name(RenameMode::From))→Deleted` + `RenameMode::To→Created` arms (Windows notify never emits `RenameMode::Both`); `consecutive_failed_batches` threshold 3 before Error escalation; 5s watched-root-vanish poll w/ de-dup HashSet for issue #403.
- **v0.2.53 Mirror + auto-reconnect — DO NOT REGRESS:** Mirror mode is session-scoped (`mirror_mode: AtomicBool`), resets on engine restart by design — don't persist. UI typed-confirm gate requires literal "MIRROR". Auto-reconnect rolling-window threshold = 3 wedges in 60s w/ `reconnecting` guard.
- **v0.2.56 (this release) — DO NOT REGRESS:** Assistant tab self-execs MCP server via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; `RIFT_MCP_ROOTS` env scopes filesystem access; CLI spawn passes `--mcp-config` + `--allowed-tools mcp__rift__*`; system addendum split into `_TOOLS` / `_NO_WS`. TabRail `--rail-w` CSS var drives AppShell grid column. Files tab drag uses pointer events, NOT HTML5 DnD.
