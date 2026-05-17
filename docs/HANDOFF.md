# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Sessions 78–86 — 2026-05-17 — v0.4.1 ship: refactor + audit pass + queue clear

S78–85 arc archived. **S86** closed audit MED S1 (`FolderCountCache`), `isHandshaking` pill fix, `file-display.ts` extract. Verify clean. **Audit Open: 7 → 6** (6 LOW lib/config; sync closed). Pre-ship security review — no new findings. CHANGELOG expanded for full release scope. See [docs/AUDIT.md](docs/AUDIT.md).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. **v0.4.1-alpha** is the live release target. Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**v0.4.1 shell:** EXPERIMENTAL behind `useV03Shell` toggle (key kept verbatim, never rename). Settings → Appearance, default OFF. Spec `docs/design/v0.4.1-right-pane-refactor.md`; archive after ship lands.

**CDP autonomous-verify live** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; drive via `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. Smoke: `scripts/cdp/smoke-v04-1.sh`.

**v0.2 queue (post-S86, each needs design — `/grill` or `/plan` first):** auto-Mirror on rename; dry-run Mirror preview; EACCES auto-fix-perms; `lib.rs`→`commands/*.rs` split; LocalPane/RemotePane full base-component extract (S86 shipped pane-utils slice only); Diagnostics canonical-skeleton; integration tests phase 1.

**Audit queue (post-S86):** 6 LOW lib/config, all blocked upstream. Full list in [docs/AUDIT.md](docs/AUDIT.md).

**Multi-user:** Trey OFF Mirror until on-latest + fresh-Pulled baseline.

**Don't reintroduce:** dock primitive, maximize-to-center, `PanelState.slot`, `dockSplitPct`, Tasks-as-peer, AddPanelMenu, TabRail under v0.4.1, OpRail/TopBar.

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
- **v0.2.46+ data-integrity stack** (git log): `mkdir_p_strict_via`, batch pre-mkdir, lock-release, debounce/coalesce, `with_t` timeouts, stale-lock gates, RenameMode arms, Mirror typed-confirm, auto-reconnect window.
- **v0.2.56:** Assistant tab self-execs MCP via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; CLI passes `--mcp-config` + `--allowed-tools mcp__rift__*`.
- **v0.4 chat tabs:** `openTabs` (`rift.ui.tabs.v1`) filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt` (NOT `currentConvoId`) so `newTab`-minted ids route as `--session-id`.
- **v0.4.1 right-pane:** keep `useV03Shell` storage key. Keys: `rightPane.activeId`=`rift.ui.right-pane.v1`; width=`rift.ui.right-pane-w.v1` (320–1200, default 560); order=`rift.ui.activitybar-order.v1`. `--right-pane-w` → 0 when `activeId===null`. Left-edge-resize only.
- **S79 layout:** right-pane pages at `src/lib/components/right-pane/`. Registry `PANELS` + type `PanelDef` names retained.
- **S85 session-lost (`bbd762`):** backend emits `assistant://session-lost {session_id, prompt}` on stderr-match "No conversation found with session ID:" + `!is_first_turn`. Frontend `onSessionLost` pops failed pair, nulls `convoCreatedAt`, re-sends. Don't gate first-turn through this path.
- **S86 pill desync:** `isHandshaking = $derived(connecting && status === null)` — UI labels read this; raw `connecting` still gates action buttons (anti-double-fire). Don't collapse.
- **S86 file-count cache:** `FolderCountCache { AtomicU64 count, AtomicI64 last_refresh_secs }` keyed by `remote_root`. 5-min TTL → `safe_count_files`; otherwise optimistic `apply_count_delta(created, deleted)` from INPUT entries (not per-entry success). `stop_watch` MUST evict cache. Threshold's 5..25 clamp forgives small drift.
