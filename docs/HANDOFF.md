# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 77 — 2026-05-17 — v0.4 chat tabs + split dock

Three phases on top of v0.3, all flag-gated under `uiPrefs.useV03Shell`. v0.2 path pixel-identical.

**Phase 1 — Chat tabs.** New `ChatTabsBar.svelte` (34px row under Titlebar). `AssistantStore` grows `openTabs: string[]` + tab helpers (open/close/new/reorder/cycle/closeAll/closeOthers/closeTabsToRight). Storage `rift.ui.tabs.v1= { openTabs, activeTabId }`; init filters ids against `assistant_list_conversations`. `send()` now keys first-turn off `convoCreatedAt` so `newTab`-minted ids still route `--session-id`. Unsaved-new-tab switch handled in-memory (no disk hit). Keys: `Ctrl+T`/`Ctrl+W`/`Ctrl+Tab`/`Ctrl+Shift+Tab`/`Alt+1..9`. HTML5 DnD reorder + tail-zone. HistoryDrawer + AssistantHeader + `/new` slash branch to `newTab` under v0.3. Empty-tabs CTA replaces chat+composer.

**Phase 2 — Split dock.** `PanelState.slot: "left"|"right"` (legacy → left on migration). Outer max `Math.min(900, innerWidth - 480)` per resize; dblclick outer handle → ~half viewport. Inside: grid `[left][4px handle][right]`, collapses to one column when right empty. Cross-slot drop via header drag; empty-right shows "Drop here" hint during left-source drag. `dockSplitPct = $state(50)` → `:root --dock-split-pct` (no inline-style override, so programmatic resize works). 20–80% clamp, dblclick → 50. Per-slot accordion: `applyOpenState` sweep restricted to dragged panel's slot. `Ctrl+1..8` panel toggles unaware of slot.

**Phase 3 — Polish.** Settings → Appearance picks up Layout sub-card (Reset split, Close all tabs, kbd cheat). `scripts/cdp/smoke-v04.sh` 23/23 PASS end-to-end. CDP `serve.cjs` `KEY_DEFS` auto-resolves digits + letters. CHANGELOG v0.4.0-alpha live (597w), v0.2.57-alpha archived. Three-file bump → **0.4.0-alpha**. `npm run check`: 0 errors, 6 warnings (4 pre-existing, 2 fixed via slot `role="region"`).

**Source state:** v0.4.0-alpha committed but **NOT YET shipped** via `scripts/release.ps1`. Quit Rift dev first (build collides w/ incremental rebuild lock). Self-replace dance applies if an installed Rift is running.

## Sessions 69–76 — 2026-05-15/17 — Assistant maturity + v0.3 shell + v0.4 spec (collapsed)

S69 blank-response fix + ext-thinking. S70 CDP autonomous-verify infra. S71 harness pull-through. S72 native session-id + cost cap. S73 Rift-native (`mcp__rift__remote_bash` + workspace shell lock). S74 UX polish (diff cards, cost+model badge, @-files, code-block copy, conversation search, streaming pacer). S75 v0.3 UI redesign Phases A→C+1 (23 commits flag-gated). **S76** mid-session v0.3 polish — dropped TabRail under v0.3, always-render panel headers, snappy dock-resize (RAF + drag/persist split + no grid transition); locked the v0.4 spec.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Last commit on main: **v0.4.0-alpha** (uncomitted to GitHub releases yet). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**Source state:** v0.4.0-alpha — three-file lockstep + CHANGELOG live entry + HANDOFF rewritten + S77 phases committed. `scripts/release.ps1` not yet invoked. Run it next session after dev is quit.

**v0.3 + v0.4 shell:** EXPERIMENTAL behind `useV03Shell` toggle (Settings → Appearance, default OFF). v0.4 = chat tabs + split dock; spec archived at `docs/archive/design/v0.3-ui-redesign.md`. The v0.4 spec doc (`docs/design/v0.4-tabs-and-split-dock.md`) stays live until v0.4 ships.

**CDP autonomous-verify is live for dev** — `scripts/run-dev.bat` sets the WebView2 port; `npm run cdp:serve` wraps it on 9223; drive via `bash scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. New: `scripts/cdp/smoke-v04.sh` (23-check end-to-end smoke).

**v0.2 queue (carryover):** (a) EACCES auto-fix-perms; (b) auto-Mirror on detected rename; (c) integration test phase 1 (needs SftpClient trait); (d) dry-run Mirror preview; (e) `lib.rs` split (1771L) → per-domain `commands/*.rs`; (f) `reqwest`+`ureq` consolidation (blocked on velopack); (g) LocalPane/RemotePane shared-logic extract; (h) connection.connecting pill desync; (i) Diagnostics page canonical-skeleton.

**Multi-user warning:** Trey: keep him OFF Mirror until on latest + fresh-Pulled baseline.

**Don't reintroduce:** OpRail, TopBar, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn`, FiveM-specific Assistant copy, AddPanelMenu (S76 dropped it). TabRail under v0.3. The 220ms `transition: grid-template-columns` on `.body[data-v03]`. `localStorage.setItem` on every pointermove during dock resize. Per-pointermove localStorage during the internal split-resize (same drag/persist split pattern applied there too).

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
- **v0.2.56:** Assistant tab self-execs MCP server via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; `RIFT_MCP_ROOTS` scopes filesystem; CLI spawn passes `--mcp-config` + `--allowed-tools mcp__rift__*`. Files tab drag uses pointer events, NOT HTML5 DnD.
- **v0.3 shell (flag-gated):** PanelShell registry-based mount. PanelDef carries optional `getCount`/`getTone`. `applyOpenState` clears `maximized` if the maximized panel closes. AppShell branches body grid on `useV03Shell`; all v0.2 codepaths preserved verbatim. Terminal panel uses `$effect`+`onMount` fallback for maximize-on-first-open.
- **S76 v0.3 polish:** No TabRail under v0.3. All panel headers always-visible. Dock width 260–(viewport-aware max). Width-resize: `setDockWidthLive` during drag + `persistDockWidth` on release + RAF-throttle + `transition: none` on `.body[data-v03]` grid. Whole-header click toggles.
- **v0.4 (S77):** `openTabs` storage `rift.ui.tabs.v1` filters against `assistant_list_conversations` on init. `send()` keys isFirstTurn off `convoCreatedAt` (not `currentConvoId`) so `newTab`-minted ids still route as `--session-id`. `PanelState.slot` localStorage migration defaults to `"left"`. Dock-split uses `:root` `--dock-split-pct` only (no inline style on `.dock-body` — that broke programmatic resize via dynamic import). Internal split handle follows the same drag/persist split pattern as the outer width handle.
