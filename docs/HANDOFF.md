# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 111 — 2026-05-19 — v0.4.13-alpha shipped — Assistant UI overhaul + update-flow restyle

**Update-flow restyle (S110):** `UpdateInfoDto` + `UpdateService` upgraded to managed Tauri state w/ `Arc`; `apply_updates` split into `download_update` (streams `update-progress`) + `apply_pending_update`. Frontend store = 8-state machine + progress + `dismissedVersion` snooze + derived labels. `UpdateDialog` restyled (gradient header, version-diff chips, markdown-lite notes, shimmer progress, ready-card). New `UpdateToast.svelte` slides bottom-right (12s auto-dismiss, hover-paused). StatusBar pill gated on available/ready + toast dismissed + dialog closed. `release.ps1` got conditional `--splashImage` flag.

**Assistant UI overhaul (S111):** Killed empty-tabs gate — first tab auto-opens (−85 LOC). User msgs right-aligned neutral `--bg-elev-2` + 12px radius; user avatar dropped. Turn-badge inline beside "Claude" (was floating right). Messages widened to `min(960px, 88ch)` w/ 20px gap + faint top-border between bubbles. Header `+` labeled "New"; tasks-toggle gated on `taskCount > 0`; ws-chip neutral. EmptyState anchored 12vh top, cards 520px, suggestion clamp 1→2 lines, stagger entrance + hero-glyph breathe + card press states. Composer baseline-centered (mic 26 borderless, hint 22, effort 22, model 24 w/ ▾ caret, send 28×28); `:has(textarea:not(:placeholder-shown))` flips to flex-end on multi-line. Scrollbar nuked on `.scroll` + `.strip` (kills WebView2 arrow-buttons leaking top-right). Jump-to-latest pill on scroll-up. ChatTabsBar new-tab slide-in 220ms.

CDP-verified; svelte-check 0/0/4051; 3-file bump 0.4.12-alpha → 0.4.13-alpha. S109 → archive.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.13-alpha** (shipped 2026-05-19). Tauri 2 + Svelte 5 + Rust + russh. Clean tree.

**Workspace shell** = single shell. Activity bar right; 9 workspaces (chat/sync/files/conflicts/diagnostics/terminal/activity/history/settings) + 2 disabled stubs (agents/attachments). Order persisted to `rift.ui.workspace-order.v1`; active to `rift.ui.workspace.v1`. Phase 1/2/3 of `ui-shell-redesign` all shipped — Settings overlay machinery is gone; everything renders inside a uniform 46px PageHeader + extended 22px StatusBar.

**Assistant page (v0.4.13)** — auto-opens first tab on mount; user msgs right-aligned neutral; turn-badge inline; jump-to-latest pill; scrollbar hidden on `.scroll` + `.strip`; composer baseline-centered w/ `:has()` flex-end multi-line escalation; ▾ caret on model pill.

**Update flow (v0.4.13)** — `UpdateToast` slides up bottom-right on availability; restyled `UpdateDialog` w/ markdown-lite notes; 8-state store machine; conditional `--splashImage` flag in release.ps1 (drop `src-tauri/installer-splash.png` to activate).

**CDP** — `scripts/run-dev.bat` + `npm run cdp:serve`; `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. devtools:// filter merged S106.

**Next priorities (pick one):** (a) Drop `src-tauri/installer-splash.png` (560×140-ish dark PNG) to activate themed installer; (b) Compaction Phase B — wire around sonnet effort-flag cache bust (`docs/design/assistant-compaction.md`); (c) bg-tab session-lost retry on cwd-hash mismatch; (d) expose `xhigh`/`max` effort tiers; (e) `lib.rs` split into `commands/*.rs` (ISSUES.md #20).

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
- Assistant scrollbar: `.scroll` + `.strip` BOTH set `scrollbar-width: none` + `::-webkit-scrollbar { display: none }` — don't reintroduce `scrollbar-gutter: stable`, it leaks the WebView2 arrow-buttons on top-right.
- AssistantPage `onMount` auto-fires `newTab()` if `openTabs.length === 0` after init resolves. Don't reintroduce the empty-tabs CTA.
- `UpdateService` is managed Tauri state — register w/ `.manage(Arc::new(UpdateService::new(...)))` in `lib.rs::run()`. `apply_updates` is split: `download_update` then `apply_pending_update`.
