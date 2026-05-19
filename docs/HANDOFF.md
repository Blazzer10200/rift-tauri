# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 109 — 2026-05-19 — v0.4.12-alpha shipped — UI shell redesign + security batch

S105 through S109 collapsed into one ship batch. Per-tab streaming pipeline + telemetry overhaul + cache discovery (S105/S106) → IPC token strip + TOFU guard + mcp-config cleanup (#9.1/9.2/#10) → context-pill envelope fix (#1) → UI shell Phase 1 (StatusBar extend + PageHeader sweep across all 8 workspaces, S108) → Phase 2 (Sync dashboard rebuild: `WatchedFoldersTable` + `RecentActivityCard` + `DriftSummaryCard` w/ Activity deeplink, new `list_watched_folders` Tauri cmd) → Phase 3 (Composer `(?)` popover + pills on textarea row + `scrollbar-gutter: stable` (#6), Settings-as-workspace via `SettingsPage.svelte` + `dialogs.svelte.ts` callbacks store, gear dropped from ActivityBar, `Ctrl+,` flips to settings workspace, `+` button moved to right end of tab strip) → console noise sweep (#22) + dead-file cleanup.

8 commits `4c7c9f9..c384034` + `2a97689`. Pushed `main`; release v0.4.12-alpha published to Blazzer10200/rift-releases w/ Setup.exe + nupkg (full + delta) + Portable.zip + RELEASES assets.

> S105–S108 collapsed into the v0.4.12-alpha ship above. S104 → `git log`. S100 + S96–99 in [HANDOFF-archive.md](archive/HANDOFF-archive.md).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.12-alpha** (shipped 2026-05-19). Tauri 2 + Svelte 5 + Rust + russh. Clean tree.

**Workspace shell** = single shell. Activity bar right; 9 workspaces (chat/sync/files/conflicts/diagnostics/terminal/activity/history/settings) + 2 disabled stubs (agents/attachments). Order persisted to `rift.ui.workspace-order.v1`; active to `rift.ui.workspace.v1`. Phase 1/2/3 of `ui-shell-redesign` all shipped — Settings overlay machinery is gone; everything renders inside a uniform 46px PageHeader + extended 22px StatusBar.

**CDP** — `scripts/run-dev.bat` + `npm run cdp:serve`; `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. devtools:// filter merged S106.

**Next priorities (pick one):** (a) Compaction Phase B — wire around sonnet effort-flag cache bust (`docs/design/assistant-compaction.md`); (b) bg-tab session-lost retry on cwd-hash mismatch; (c) expose `xhigh`/`max` effort tiers (composer pill cycles `none→quick→deep`); (d) `lib.rs` split into `commands/*.rs` (1790L, CLAUDE.md queue (e), ISSUES.md #20).

**Open ISSUES** (full list `docs/ISSUES.md`): Tier 1 #21 (zero tests) + #15 (signing) + #9.3 (keyring) deferred to Phase 6. Tier 3 #2 (tool-block rhythm), #5 (status hub), #11 remainder (Appearance "More soon" + SSH Keys empty + font-picker class). Tier 4 #4/#7/#16/#17/#18/#20/#23/#24/#25/#26.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only (NASM/aws-lc-rs blocked). russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `last_scan_entries` = `std::sync::Mutex`. `force_pull_now`/`force_push_now` invariants preserved.
- `FileAttributes::default()` for SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: `--permission-mode bypassPermissions` (NOT dontAsk) + full `BUILTINS` in `--allowed-tools` across all three branches.
- TabState: per-tab field → add to TabState class + getter on AssistantStore. Never put per-tab state back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` when attachments present. 20MiB cap + `image/*` gate.
- Settings is now a workspace (kbd 9), `Ctrl+,` flips workspace; do NOT reintroduce the slideover scrim/aside. Dialog callbacks ride `src/lib/state/dialogs.svelte.ts`, populated by AppShell at mount.
- `list_watched_folders` Tauri cmd returns name + remote_root + cached file_count from `FolderCountCache`; lock count + last-event derived client-side from `connection.locks` + `connection.activityFeed`.
