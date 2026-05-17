# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Sessions 78–85 — 2026-05-17 — v0.4.1 refactor + audit fix-passes + UI fixes

S78–79 dropped dock-accordion (right side = one full page from 40px `ActivityBar`) under `useV03Shell`; `0.4.1-alpha` committed but `release.ps1` NOT invoked yet. S80 re-verified 42-item audit (16 silently fixed by v0.3/v0.4). S81–S84 closed 16 audit items + 1 downgrade across 4 batches. S85 (`bbd7642`) shipped 2 live UI fixes: dedupe History button (hidden under v0.3/v0.4.1 since ActivityBar covers it); auto-recover from claude's "No conversation found" on `--resume` via new `assistant://session-lost` event → frontend nulls `convoCreatedAt`, re-sends as first-turn. All `cargo check`/test/`npm run check` clean. **Audit Open: 42 → 7 actionable** (6 LOW lib/config + 1 MED sync). Detail in [docs/AUDIT.md](docs/AUDIT.md).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Last commit on main: **v0.4.1-alpha** (not yet uploaded). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**v0.4.1 shell:** EXPERIMENTAL behind `useV03Shell` toggle (storage key kept verbatim, never rename). Settings → Appearance, default OFF. Spec at `docs/design/v0.4.1-right-pane-refactor.md`; archive after ship.

**CDP autonomous-verify live** — `scripts/run-dev.bat` sets WebView2 port; `npm run cdp:serve` wraps on 9223; drive via `bash scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. Smoke: `scripts/cdp/smoke-v04-1.sh`.

**v0.2 queue:** EACCES auto-fix-perms; auto-Mirror on rename; integration tests phase 1; dry-run Mirror preview; `lib.rs` → `commands/*.rs` split; LocalPane/RemotePane shared-logic extract; connection.connecting pill desync; Diagnostics canonical-skeleton.

**Audit queue (post-S85):** 7 open (6 LOW lib/config + 1 MED sync `flush_batch` inline count). All blocked upstream or need design (velopack async, Tailwind hashed CSS, Phase-6 secrets, capabilities perm-usage audit). Full list in [docs/AUDIT.md](docs/AUDIT.md).

**Multi-user:** Trey OFF Mirror until on latest + fresh-Pulled baseline.

**Don't reintroduce:** dock primitive; maximize-to-center; `PanelState.slot`; `dockSplitPct`; Tasks as registry peer; AddPanelMenu; TabRail under v0.4.1; OpRail/TopBar.

**Ship pipeline:** `powershell -NoProfile -File ./scripts/release.ps1` — build → vpk pack → upload to `rift-releases`.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only (NASM blocks aws-lc-rs).
- `~/.rift/*.json` compat — keep `serde(flatten) extra`.
- `VelopackApp::build().run()` MUST be first call in `lib.rs::run()`.
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver).
- DriftWatcher conflict-rename guard — never overwrite dirty local. `.rift-trail.jsonl` ignore rule mandatory (pull→push loop without).
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_via` strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `last_scan_entries` = `std::sync::Mutex` (NOT tokio) — notify handler context.
- `force_pull_now`/`force_push_now` invariants preserved (v0.2.43).
- **`FileAttributes::default()` for SETSTAT = data-loss** — use `empty()`.
- `SftpClient::delete` routes by remote stat → dirs via `delete_recursive_via`.
- `mkdir_p_via` chmods each segment 2775 (shared-group pushes).
- Upload pre-flight SHA-collapse before raising CONFLICT (v0.2.32).
- `DriftBucket::ToDelete` deletes LOCAL; `ToDeleteRemote` deletes REMOTE (mirror-on + has-baseline gate). Mass local-delete circuit breaker `(count*0.30).clamp(5,25)`; ToDeleteRemote bypasses.
- Time displays MUST pass `[], { hour12: true }` (locale 24h on non-US).
- `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- russh `Config { keepalive 20s/3, window 2 MiB, packet 32 KiB }` in `sftp::open_session` + `tunnel::start`.
- **v0.2.46+ data-integrity stack** — see git log; covers `mkdir_p_strict_via`, batch pre-mkdir, lock-release invariants, debounce/coalesce, `with_t` timeouts, stale-lock sweep gates, RenameMode arms, Mirror typed-confirm, auto-reconnect rolling-window.
- **v0.2.56:** Assistant tab self-execs MCP via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; CLI passes `--mcp-config` + `--allowed-tools mcp__rift__*`.
- **v0.4 chat tabs:** `openTabs` (`rift.ui.tabs.v1`) filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt` (NOT `currentConvoId`) so `newTab`-minted ids route as `--session-id`.
- **v0.4.1 right-pane:** `useV03Shell` storage key kept verbatim. `rightPane.activeId` = `rift.ui.right-pane.v1`; width = `rift.ui.right-pane-w.v1` (320–1200, default 560); order = `rift.ui.activitybar-order.v1`. `--right-pane-w` → 0px when `activeId === null`. Left-edge-resize only — no internal split, no maximize.
- **S79 layout:** right-pane page wrappers at `src/lib/components/right-pane/` (moved from deleted `dock/panels/`). Registry export `PANELS` + type `PanelDef` names retained for min churn.
- **S85 session-lost (`bbd7642`):** backend emits `assistant://session-lost {session_id, prompt}` on stderr-match "No conversation found with session ID:" + `!is_first_turn`. Frontend `onSessionLost` pops failed pair, nulls `convoCreatedAt`, re-sends. Don't gate first-turn through this path.
