# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 78 — 2026-05-17 — v0.4.1 right-pane refactor (drop the dock)

Spec at `docs/design/v0.4.1-right-pane-refactor.md`. Four flag-gated commits on the existing `useV03Shell` toggle (storage key kept for upgrade compat). v0.2 path pixel-identical.

**Phase 1** — Tasks moved back to AssistantPage; dropped from `PanelId`/`PANEL_IDS`/`PRESETS`/dock registry; deleted `TasksPanel.svelte`. **Phase 2** — new `ActivityBar.svelte` (40px right-edge, 7 icons, drag-reorder) + `RightPane.svelte` (one full page, left-edge resize 320–1200) + `right-pane.svelte.ts` (storage migration: seeds `activeId` from legacy `panels.v1`'s exactly-one-open panel, renames `dock-w.v1` → `right-pane-w.v1`, deletes obsolete dock keys). Body grid → `[chat | --right-pane-w (0 closed) | 40px]` under `data-v04-1="true"`. Drops summary-card branches in `FilesPanel` + `SyncPage`. `TerminalPanel` + `terminal.toggle()` route through `rightPane`. **Phase 3** — deleted `Dock.svelte` / `PanelShell.svelte` / `PresetPicker.svelte`; `ui-prefs.svelte.ts` trimmed 325 → 58 lines (only `density` + `railPinned` + `useV03Shell` survive); `panel-types.ts` down to `PanelId` + `PANEL_IDS`; `AssistantPage` drops Phase-C maximize JSX/CSS; `TabRail` drops panel-mode dead code; Settings → Layout swaps "Reset dock split" → "Reset right pane" + drops accordion switch + new Ctrl+1..7 / Ctrl+0 kbd cheat. **Phase 4** — CHANGELOG-first (520w, v0.4.0-alpha archived), three-file bump → `0.4.1-alpha`, `npm run check` 0 errors / 1 pre-existing warning, `scripts/cdp/smoke-v04-1.sh` 35/35 PASS, v0.4 design doc archived → `docs/archive/design/`.

**Source state:** v0.4.1-alpha committed across four phase commits but **NOT YET shipped** via `scripts/release.ps1`. Quit Rift dev first (build collides w/ incremental rebuild lock). Self-replace dance applies if an installed Rift is running.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Last commit on main: **v0.4.1-alpha** (uncommitted to GitHub releases yet). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**Source state:** v0.4.1-alpha three-file lockstep + CHANGELOG live + S78 phases committed. `scripts/release.ps1` not yet invoked. Run after dev is quit.

**v0.4.1 shell:** EXPERIMENTAL behind `useV03Shell` toggle (Settings → Appearance, default OFF). Spec at `docs/design/v0.4.1-right-pane-refactor.md`; archive after ship. v0.2 path renders pixel-identical.

**CDP autonomous-verify live for dev** — `scripts/run-dev.bat` sets WebView2 port; `npm run cdp:serve` wraps on 9223; drive via `bash scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. Smoke: `scripts/cdp/smoke-v04-1.sh` (35-check).

**v0.2 queue (carryover):** (a) EACCES auto-fix-perms; (b) auto-Mirror on detected rename; (c) integration test phase 1 (needs SftpClient trait); (d) dry-run Mirror preview; (e) `lib.rs` split (1771L) → per-domain `commands/*.rs`; (f) `reqwest`+`ureq` consolidation (blocked on velopack); (g) LocalPane/RemotePane shared-logic extract; (h) connection.connecting pill desync; (i) Diagnostics page canonical-skeleton.

**Multi-user warning:** Trey: keep him OFF Mirror until on latest + fresh-Pulled baseline.

**Don't reintroduce:** OpRail, TopBar, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn`, FiveM-specific Assistant copy, AddPanelMenu, TabRail under v0.4.1, the dock primitive (Dock/PanelShell/PresetPicker — deleted S78), maximize-to-center (Esc no longer restores anything), `PanelState.slot` intra-dock split, `dockSplitPct` + internal split handle, Tasks as a peer of Files/Sync/etc, compact summary cards in panel surfaces, the dock-accordion concept.

**Ship pipeline:** `powershell -NoProfile -File ./scripts/release.ps1` — full ship (build → vpk pack → upload to `rift-releases`).

---

## CRITICAL DON'T-TOUCH

- russh `ring` backend + reqwest `rustls` only (NASM blocks aws-lc-rs). rustls dep is intentional pin — 0 direct `use` is correct.
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
- **v0.2.46+ data-integrity stack** (see `git log` for per-version detail): `mkdir_p_strict_via`, batch pre-mkdir in `flush_batch`, lock release on every `process_entry` terminal path, `path.is_file()` gate in `queue_path`, `wait_for_readable` 6×exp-backoff, `ignored_directory_names()` excludes `build`+`dist`, Created+Dir 500ms debounce + `pending_dir_reconcile` coalesce, `with_t` op timeouts + LIST_T 120s, `ConnectionWedged` diag, `.tmp.<pid>.<hex>` rule, `sync_sweep_stale_locks` own-user-only, `RenameMode::From→Deleted`+`To→Created` arms, `consecutive_failed_batches` threshold 3, 5s watched-root-vanish poll, Mirror session-scoped + UI typed "MIRROR", auto-reconnect 3 wedges/60s.
- **v0.2.56:** Assistant tab self-execs MCP server via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; `RIFT_MCP_ROOTS` scopes filesystem; CLI spawn passes `--mcp-config` + `--allowed-tools mcp__rift__*`. Files tab drag uses pointer events, NOT HTML5 DnD.
- **v0.4 chat tabs (S77 carry-forward):** `openTabs` storage `rift.ui.tabs.v1` filters against `assistant_list_conversations` on init. `send()` keys isFirstTurn off `convoCreatedAt` (not `currentConvoId`) so `newTab`-minted ids still route as `--session-id`.
- **v0.4.1 right-pane (S78):** `useV03Shell` storage key kept verbatim — never rename the slot. `rightPane.activeId` = `rift.ui.right-pane.v1`; width = `rift.ui.right-pane-w.v1` (320–1200, default 560); order = `rift.ui.activitybar-order.v1`. CSS var `--right-pane-w` collapses to `0px` when `activeId === null`. RightPane is left-edge-resize-only; no internal split, no maximize. Migration deletes legacy `panels.v1`/`dock-w.v1`/`dock-split.v1`/`maximized.v1`/`preset-picked.v1`/`dock-accordion.v1` on first launch.
