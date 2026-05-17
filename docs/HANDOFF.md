# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 76 — 2026-05-17 — v0.3 mid-session polish + v0.4 design locked

**v0.3 shell mid-session polish (uncommitted, on main working tree):**

Real-driver feedback exposed three friction points. Fixed in-session, all flag-gated.

1. **Two-sidebar fatigue.** Left rail + right dock sandwiching chat felt cramped. Dropped TabRail under v0.3 entirely; body grid `[rail | main | dock]` → `[main | dock]`. All 8 panel headers now always render in the dock (no AddPanel button); `open` field repurposed as expanded/collapsed. PanelShell whole-header click toggles. Shift-click stacks past accordion (matches `Ctrl+Shift+1..8`). AddPanelMenu deleted.
2. **No visual hierarchy.** Open panel gets a 2px accent left-rail + lifted `bg` + bolder title. Dock background lifted to `bg-elev-1` so it reads as a distinct surface. Sync + Files summary cards harmonized — compact 2-row info + small inline "Open" maximize button. Dock max width 560 → 460.
3. **Dock-resize handle laggy.** Three compounding causes: (a) 4px hit strip too thin → widened to 10px straddling the dock border seam, visual stays 2px via `::after`; (b) synchronous `localStorage.setItem` on every pointermove → split into `setDockWidthLive` (state+CSS) used during drag + `persistDockWidth` called once on pointerup; RAF-throttled the move handler; (c) `transition: grid-template-columns 220ms` on `.body[data-v03]` made the grid chase the cursor — dropped under v0.3 (kept under v0.2 for rail-pin animation). Now snappy.

Settings copy updated — "rail icon" reference removed, now mentions `Ctrl+Shift+1…8`. CDP-verified end-to-end (Files, Sync, Tasks empty, Activity empty, Settings slide-over, maximize-to-center + Esc restore).

Files touched: `AppShell.svelte`, `Dock.svelte`, `PanelShell.svelte`, `FilesPanel.svelte`, `SyncPage.svelte`, `ui-prefs.svelte.ts`, `Settings.svelte`. Deleted: `AddPanelMenu.svelte`.

**v0.4 design — chat tabs + split dock — locked:**

User-driven scope: top-of-app browser-style chat tabs + dock can grow to ~50vw and split horizontally into LEFT/RIGHT slots with independent panel stacks. Spec lives in [`docs/design/v0.4-tabs-and-split-dock.md`](design/v0.4-tabs-and-split-dock.md) — every fork decided, executing session transcribes only. Three phases: tabs (state + ChatTabsBar + AppShell mount), split (slot field + per-slot grid + internal handle), polish + ship. Built on the v0.3 toggle; v0.2 untouched throughout.

## Sessions 69–75 — 2026-05-15/17 — Assistant maturity + v0.3 shell (collapsed)

S69 fixed Assistant blank-response (cmd-shim arg mangling) + ext-thinking via `MAX_THINKING_TOKENS=10000`. S70 shipped CDP autonomous-verify infra (`scripts/cdp/serve.cjs`, port 9223). S71 (Phase 1) harness pull-through — `use_full_config` default ON. S72 (Phase 2) native `--session-id`/`--resume` + Settings cost cap. S73 (Phase 3) Rift-native sprint — per-turn `WorkspaceContext` + `mcp__rift__remote_bash` + workspace shell lock. S74 (Phase 4) UX polish — diff cards, cost+model badge, @-file mentions, code-block copy, conversation search, context-aware empty-state, streaming pacer. S75 v0.3 UI redesign Phases A→C+1 (23 commits `0b39dc8..bf64470`), all behind `useV03Shell` toggle.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Last shipped: **v0.2.56-alpha** (`687edb8` on main). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**Source state:** v0.2.57-alpha version-bumped across 3 lockstep files + CHANGELOG entry in, NOT YET shipped via `scripts/release.ps1`. S76 v0.3 polish is uncommitted on main working tree — squash into the v0.2.57 ship OR commit separately before/after.

**v0.3 shell:** EXPERIMENTAL behind `useV03Shell` toggle (Settings → Appearance, default OFF). Polished this session; daily-driveable. Spec: `docs/design/v0.3-ui-redesign.md`.

**v0.4 era:** spec'd this session in `docs/design/v0.4-tabs-and-split-dock.md`. Concrete + decided. Pick up via kickoff prompt prepared by S76.

**CDP autonomous-verify is live for dev** — `scripts/run-dev.bat` sets the WebView2 port; `npm run cdp:serve` wraps it on 9223; drive via `bash scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. Use BEFORE asking the user to screenshot anything.

**v0.2.57 queue (carryover):** (a) EACCES auto-fix-perms; (b) auto-Mirror on detected rename; (c) integration test phase 1 (needs SftpClient trait); (d) dry-run Mirror preview; (e) `lib.rs` split (1771L) → per-domain `commands/*.rs`; (f) `reqwest`+`ureq` consolidation (blocked on velopack); (g) LocalPane/RemotePane shared-logic extract; (h) connection.connecting pill desync; (i) Diagnostics page canonical-skeleton.

**Multi-user warning:** Trey: keep him OFF Mirror until on latest + fresh-Pulled baseline.

**Don't reintroduce:** OpRail, TopBar, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn`, FiveM-specific Assistant copy, AddPanelMenu (S76 dropped it).

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
- Mass local-delete circuit breaker: `(file_count * 0.30).clamp(5, 25)`. ToDeleteRemote bypasses.
- `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- russh `Config { keepalive_interval: 20s, keepalive_max: 3, window_size: 2 MiB, maximum_packet_size: 32 KiB }` in both `sftp::open_session` + `tunnel::start`.
- **v0.2.46+ data-integrity stack:** `mkdir_p_strict_via`, batch pre-mkdir in `flush_batch`, lock release on every `process_entry` terminal path, `path.is_file()` gate in `queue_path`, `wait_for_readable` 6×exp-backoff.
- **v0.2.48:** `ignored_directory_names()` excludes `build`+`dist`; Created+Dir 500ms debounce + `pending_dir_reconcile` AtomicBool coalesce.
- **v0.2.50:** `with_t` op timeouts + LIST_T 120s; `ConnectionWedged` diag emit; `process_entry` terminal lock-release INLINE w/ 5s timeout; `.tmp.<pid>.<hex>` rule tight-matched; `sync_sweep_stale_locks` ONLY clears own-user locks.
- **v0.2.52:** explicit `RenameMode::From→Deleted`+`To→Created` arms; `consecutive_failed_batches` threshold 3; 5s watched-root-vanish poll.
- **v0.2.53:** Mirror session-scoped (`mirror_mode: AtomicBool`). UI typed-confirm requires "MIRROR". Auto-reconnect rolling-window 3 wedges in 60s.
- **v0.2.56:** Assistant tab self-execs MCP server via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; `RIFT_MCP_ROOTS` scopes filesystem; CLI spawn passes `--mcp-config` + `--allowed-tools mcp__rift__*`. TabRail `--rail-w` CSS var drives AppShell grid (v0.2 only). Files tab drag uses pointer events, NOT HTML5 DnD.
- **v0.3 shell (flag-gated):** PanelShell registry-based mount. PanelDef carries optional `getCount`/`getTone`. `applyOpenState` clears `maximized` if the maximized panel closes. AppShell branches body grid on `useV03Shell`; all v0.2 codepaths preserved verbatim. Terminal panel uses `$effect`+`onMount` fallback for maximize-on-first-open.
- **S76 v0.3 polish:** No TabRail under v0.3. All panel headers always-visible. `panel.open` = expanded state. Dock width 260–460. Width-resize: `setDockWidthLive` during drag + `persistDockWidth` on release + RAF-throttle + `transition: none` on `.body[data-v03]` grid. Whole-header click toggles; `.caret` is a passive span; action buttons `stopPropagation`.
