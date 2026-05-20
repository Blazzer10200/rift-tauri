# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 112 — 2026-05-20 — Multi-wave codebase audit (no source edits)

27-agent read-only audit, 3 waves. **232 findings folded into `docs/ISSUES.md` #34-#265** (16 HIGH / 96 MED / 103 LOW / 14 INFO + 8 dupes). Commits `d36c501` (W1 backend 105) / `cb3ec5e` (W2 frontend 80) / `3006cd3` (W3 cross-cutting 47) on `main`, **unpushed (origin 3 behind)**. Per-agent reports + SYNTHESIS files at `state/audit-2026-05-20/` (gitignored). Top 5 ship-blockers spot-checked against source — all confirmed real.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.13-alpha**. Tauri 2 + Svelte 5 + Rust + russh. **3 unpushed audit commits + working-tree README.md tweak** (added ISSUES quick link) — bundled into one housekeeping commit this session.

**Next session's job: fix the 16 HIGH severity findings.** Recommended sequence below.

### Top 5 ship-blockers — fix as v0.4.14-alpha hotfix (one commit batch, ~30 LOC across 5 files)

| # | file:line | Symptom → fix |
|---|---|---|
| **#36** | `lib.rs:805` | `save_server` `.or_else(\|_\|Ok(default()))` on config load → server list nuked on next save. **Fix:** drop fallback, propagate error via `.map_err(...)?`. |
| **#74** | `sync/drift_scanner.rs:228` | `walk_local` panic → `.unwrap_or_default()` empty map → bypasses L241 guard → mass ToPull overwrites local. **Fix:** match `JoinError`, abort scan. |
| **#41** | `remote_bridge.rs:250-258` | Bridge lock leak on exec error → blocks all users on that remote. **Fix:** `BridgeLockGuard` RAII w/ `Drop`. |
| **#219** | `lib.rs:1743` | No `panic::set_hook` → silent async-task panics app-wide. **Fix:** install hook after `LogForwarder`, route to `tracing::error!` + DiagBus. |
| **#163** | `UpdateDialog.svelte:409` | `scrollbar-gutter: stable` reintroduced — **DON'T-TOUCH violation**. **Fix:** delete line. |

Verify: `cargo check` for #36/#74/#41/#219, `npm run check` for #163.

### After hotfix

Remaining **11 HIGH** (4 FE + 4 BE + 3 cross-cutting) → triage into v0.4.15+ batches from `docs/ISSUES.md` Priority Tiers. Pre-audit queue still valid but lower urgency: (a) installer-splash.png, (b) Compaction Phase B, (c) bg-tab session-lost retry, (d) `xhigh`/`max` effort tiers, (e) `lib.rs` → `commands/*.rs` split.

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
