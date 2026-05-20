# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 112 — 2026-05-20 — Multi-wave codebase audit complete (no source edits)

**27-agent audit, 3 waves, read-only.** Wave 1 backend (11 agents, 105 findings), Wave 2 frontend (8 agents, 80 findings), Wave 3 cross-cutting (8 agents, 47 findings). **232 total findings folded into `docs/ISSUES.md` #34-#265.** Severity: **16 HIGH** / 96 MED / 103 LOW / 14 INFO + 8 dupes collapsed. Three audit commits on `main` (`d36c501` wave 1, `cb3ec5e` wave 2, `3006cd3` wave 3) — **unpushed, origin/main 3 behind**. Per-agent reports + 3 SYNTHESIS files at `state/audit-2026-05-20/` (gitignored). Plan that ran the show: `state/audit-2026-05-20/AUDIT-PLAN.md`.

Spot-checked Top 5 ship-blockers vs source — all confirmed real.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.13-alpha**. Tauri 2 + Svelte 5 + Rust + russh. **3 unpushed audit commits + working-tree README.md tweak** (added ISSUES quick link) — bundled into one housekeeping commit this session.

**Next session's job: fix the 16 HIGH severity findings.** Recommended sequence below.

### Top 5 ship-blockers — fix as v0.4.14-alpha hotfix (one commit batch, ~30 LOC across 5 files)

| # | Where | What | Fix sketch |
|---|---|---|---|
| **#36** | `src-tauri/src/lib.rs:805` | `save_server` `.or_else(\|_\| Ok(default()))` swallows config-load errors → entire server list overwritten on next save (data-loss) | Drop `.or_else`; propagate error w/ `.map_err(\|e\| format!(...))?` |
| **#74** | `src-tauri/src/sync/drift_scanner.rs:228` | `walk_local` panic in `spawn_blocking` → `.unwrap_or_default()` returns empty `local_map` → bypasses guard at L241 → mass ToPull overwrites real local files | Match on `JoinError`; abort scan w/ `FolderScan::ScanFailed` variant |
| **#41** | `src-tauri/src/.../remote_bridge.rs:250-258` | Bridge lock leak on exec error → permanently blocks ALL users on that remote root | RAII guard struct (`BridgeLockGuard`) w/ `Drop` impl releasing lock |
| **#219** | `src-tauri/src/lib.rs:1743` | No `panic::set_hook` installed → every async-task panic dies silently | Install hook after `LogForwarder` setup; route to `tracing::error!` + DiagBus |
| **#163** | `src/lib/components/assistant/UpdateDialog.svelte:409` | `scrollbar-gutter: stable` reintroduced — direct **CRITICAL DON'T-TOUCH violation** (HANDOFF said it leaks WebView2 arrow-buttons) | Delete the line |

Verify each: `cargo check --manifest-path src-tauri/Cargo.toml` for #36/#74/#41/#219, `npm run check` for #163.

### Remaining HIGH severity (11) — triage into v0.4.15+ batches after hotfix

4 frontend + 4 backend + 3 cross-cutting. Pull list from `docs/ISSUES.md` `## Priority tiers` section once the Top 5 are shipped.

### Existing pre-audit priorities (still valid, lower urgency than HIGH findings)

(a) Drop `src-tauri/installer-splash.png` for themed installer. (b) Compaction Phase B (`docs/design/assistant-compaction.md`). (c) bg-tab session-lost retry on cwd-hash mismatch. (d) expose `xhigh`/`max` effort tiers. (e) `lib.rs` split into `commands/*.rs` (ISSUES #20).

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
- Assistant scrollbar: `.scroll` + `.strip` BOTH set `scrollbar-width: none` + `::-webkit-scrollbar { display: none }` — don't reintroduce `scrollbar-gutter: stable`, it leaks the WebView2 arrow-buttons on top-right. **#163 violator in UpdateDialog.svelte:409 is on the Top 5 fix list.**
- AssistantPage `onMount` auto-fires `newTab()` if `openTabs.length === 0` after init resolves. Don't reintroduce the empty-tabs CTA.
- `UpdateService` is managed Tauri state — register w/ `.manage(Arc::new(UpdateService::new(...)))` in `lib.rs::run()`. `apply_updates` is split: `download_update` then `apply_pending_update`.
