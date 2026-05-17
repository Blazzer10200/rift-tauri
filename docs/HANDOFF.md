# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 75 — 2026-05-17 — v0.3 UI Redesign (Phases A → C+1)

**Twenty-three commits 0b39dc8 → bf64470 on main, all flag-gated behind `uiPrefs.useV03Shell` (default OFF, toggle in Settings → Appearance labeled "Experimental").** Flag-off path stays pixel-identical to v0.2 — zero regression risk; flag-on enables the new shell. Daily-driveable but not the default until polish settles from real use.

**Phase A — shell skeleton.** AppShell body grid rewired from `[rail | pane]` to `[rail | main | dock]`. `panel-types.ts` + extended `ui-prefs` panel state (PanelState, PRESETS, dockWidth, maximized, presetPicked). Dock primitive (`Dock.svelte`, `PanelShell.svelte`, `AddPanelMenu.svelte`, `PresetPicker.svelte`) + 8 panel stubs + registry. TabRail gets `mode="panels"` prop. Settings becomes slide-over modal w/ Esc + X dismiss. First-launch preset picker (Minimal / Standard / Power).

**Phase B — 5 surface migrations.** SyncPage → Sync panel (ConflictsPage folds in as `<details>` section, conflict count pip in header). TerminalPanel → Terminal panel (drops overlay mode under v0.3, branches at component-top, terminal.toggle() v0.3-aware). ActivityFeed → Activity panel (cleanest migration, event count pip). TwoPane → Files panel + summary-card-in-dock + maximize-to-view button. TasksDock + HistoryDrawer hoisted out of AssistantPage. AssistantPage shrinks to header + chat + composer. `assistant.send` auto-open writes parallel `uiPrefs.setPanelOpen("tasks", true)` calls for v0.3 parity.

**Phase C — polish.** Maximize-to-center swap actually wired: clicking ⛶ pulls panel body into `<main class="pane">`, chat hides, Esc restores. Accordion dock by default (only one panel open at a time, shift-click / Ctrl+Shift+N bypasses). TwoPane gets summary-card mode at narrow dock widths. `applyOpenState` helper cleans up `uiPrefs.maximized` if the maximized panel gets closed via rail.

**Phase C+1 — final polish.** Sync gets same summary-card pattern as Files (state pill + counts + "View drift in center" button). Terminal auto-maximizes on first-open via `$effect` + `onMount` fallback (PanelShell lazy-mounts so `$effect` baseline-undefined misses the very first open — `onMount` IS that transition).

**PanelShell architecture note:** registry-based mount, NOT slot-based. `<def.component title icon/>` instantiates from PANELS record. Wrappers ARE the bodies. Added `getCount?: () => number` + `getTone?` to PanelDef for reactive header pips (Sync danger, Activity info, Tasks info, History info).

## Sessions 69–74 — 2026-05-15/16 — v0.2.57 work (collapsed)

S69 fixed Assistant blank-response (cmd-shim `--output-format` mangling) + ext-thinking surface (`MAX_THINKING_TOKENS=10000`). S70 shipped CDP autonomous-verify infra (`scripts/cdp/serve.cjs`, port 9223). S71 (Phase 1) harness pull-through — `AssistantConfig.use_full_config` default ON drops `--strict-mcp-config` + `--disable-slash-commands`. S72 (Phase 2) native `--session-id` / `--resume` replaced hand-rolled history replay; `--max-budget-usd` + Settings cost cap. S73 (Phase 3) Rift-native sprint — per-turn WorkspaceContext addendum + `mcp__rift__remote_bash` over loopback NDJSON bridge + workspace shell lock. S74 (Phase 4) UX polish — diff view in Edit cards, per-message cost+model badge, @-file mentions, code-block copy, conversation search, context-aware empty-state. Plus streaming pacer + caret + dev WebView2 isolation folder.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Last shipped: **v0.2.56-alpha** (`687edb8` on main). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater (release repo `Blazzer10200/rift-releases`), NSIS perUser installer.

**Source state:** v0.2.57-alpha version-bumped across 3 lockstep files BUT NOT YET RUN through `scripts/release.ps1`. Next actual ship = run release.ps1 against the current main. CHANGELOG entry for v0.2.57 is in.

**v0.3 shell:** EXPERIMENTAL, behind `useV03Shell` toggle in Settings → Appearance. 23 commits on main, default off. Live design docs in `docs/design/v0.3-{brainstorm,ui-redesign}.md`. Shipped phase plans archived to `docs/archive/design/`.

**CDP autonomous-verify is live for dev** — start dev via `scripts/run-dev.bat` (sets WebView2 CDP port), then `npm run cdp:serve` in another shell. Drive via `bash scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. Use BEFORE asking the user to screenshot anything.

**v0.2.57 queue (carryover, item (j) Appearance now ships w/ v0.3 toggle):** (a) EACCES auto-fix-perms affordance; (b) auto-Mirror on detected rename only; (c) integration test phase 1 — needs SftpClient trait abstraction; (d) dry-run Mirror preview pre-confirm; (e) `lib.rs` split (1771L, 52 cmds) → per-domain `commands/*.rs`; (f) `reqwest`+`ureq` consolidation — blocked on velopack 0.0.1298 sync `UpdateSource`; (g) LocalPane/RemotePane shared-logic extraction; (h) connection.connecting pill desync; (i) Diagnostics page canonical-skeleton (Ctrl+Shift+D).

**Multi-user warning:** Trey: keep him OFF Mirror until on latest + fresh-Pulled baseline.

**Don't reintroduce:** OpRail, TopBar (merged), rail kbd hints as bulky chips, StatusBar ⌘K pip, titlebar gear, StatusHero big H1, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn` legacy dead CSS, `bg-backlog.sh`, `diag_*` cmd names, `drift_watcher::spawn`/`run_tick`/`flush_cycle`, FiveM-specific framing in Assistant copy.

**Ship pipeline:** `scripts/release.ps1` IS the full ship (build → vpk pack → upload to rift-releases). Run via `powershell -NoProfile -File ./scripts/release.ps1`.

---

## CRITICAL DON'T-TOUCH

- russh `ring` backend + reqwest `rustls` only (NASM blocks aws-lc-rs). rustls dep is intentional dep-tree pin — 0 direct `use` is correct.
- `~/.rift/*.json` compat — never change rename rules; keep `serde(flatten) extra`.
- `VelopackApp::build().run()` MUST be first call in `lib.rs::run()`.
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver).
- DriftWatcher conflict-rename guard — never overwrite dirty local.
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it.
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo.
- `path_guard.rs` API frozen — `edit/in_place.rs` + lib cmds depend.
- `rename_via` strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `last_scan_entries` is `std::sync::Mutex` (NOT tokio) — notify handler context.
- `force_pull_now`/`force_push_now` invariants preserved (v0.2.43).
- **NEVER `FileAttributes::default()` for SETSTAT** — sends zeros → truncation + epoch mtime. Use `empty()`.
- `SftpClient::delete` routes by remote stat — dirs go through `delete_recursive_via`.
- `mkdir_p_via` chmods each segment to 2775 for shared-group pushes.
- Upload pre-flight SHA-collapse before raising CONFLICT (v0.2.32).
- `DriftBucket::ToDelete` = local+no-remote+has-baseline → delete LOCAL. `DriftBucket::ToDeleteRemote` (v0.2.53) = local-missing+remote-has+has-baseline + mirror-on → delete REMOTE.
- Time displays MUST pass `[], { hour12: true }` — locale-default emits 24h on non-US.
- Mass local-delete circuit breaker: `(file_count * 0.30).clamp(5, 25)`. ToDeleteRemote bypasses (user reached via typed-MIRROR gate).
- `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- russh `Config { keepalive_interval: 20s, keepalive_max: 3, window_size: 2 MiB, maximum_packet_size: 32 KiB }` in both `sftp::open_session` + `tunnel::start`.
- **v0.2.46+ data-integrity stack:** `mkdir_p_strict_via`, batch pre-mkdir in `flush_batch`, lock release on every `process_entry` terminal path, `path.is_file()` gate in `queue_path`, `wait_for_readable` 6×exp-backoff.
- **v0.2.48:** `ignored_directory_names()` excludes `build`+`dist`; Created+Dir 500ms debounce + `pending_dir_reconcile` AtomicBool coalesce.
- **v0.2.50:** `with_t` op timeouts (T_QUICK 10s / T_NORMAL 30s / T_BODY 120s) + LIST_T 120s; `ConnectionWedged` diag emit on timeout; `process_entry` terminal lock-release INLINE w/ 5s timeout; `.tmp.<pid>.<hex>` rule tight-matched; `sync_sweep_stale_locks` ONLY clears own-user locks.
- **v0.2.52:** explicit `RenameMode::From→Deleted` + `RenameMode::To→Created` arms; `consecutive_failed_batches` threshold 3; 5s watched-root-vanish poll w/ de-dup HashSet.
- **v0.2.53:** Mirror session-scoped (`mirror_mode: AtomicBool`), resets on engine restart. UI typed-confirm requires literal "MIRROR". Auto-reconnect rolling-window = 3 wedges in 60s.
- **v0.2.56:** Assistant tab self-execs MCP server via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; `RIFT_MCP_ROOTS` scopes filesystem; CLI spawn passes `--mcp-config` + `--allowed-tools mcp__rift__*`. TabRail `--rail-w` CSS var drives AppShell grid. Files tab drag uses pointer events, NOT HTML5 DnD.
- **v0.3 shell (flag-gated):** PanelShell registry-based mount (NOT slots) — `<def.component title icon/>`. PanelDef carries optional `getCount` / `getTone`. `applyOpenState` clears `maximized` if the maximized panel closes. AppShell branches body grid on `uiPrefs.useV03Shell`; all v0.2 codepaths preserved verbatim. Terminal panel uses `$effect` + `onMount` fallback for maximize-on-first-open (PanelShell lazy-mounts).
