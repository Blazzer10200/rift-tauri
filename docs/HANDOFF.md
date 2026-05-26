# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## 2026-05-25 (PM) — autonomous cleanup pass (unshipped, on `main`)

5 commits past v0.4.30-alpha, ALL on `main`, NOT pushed. No version bump. Updater files untouched — in-flight overhaul below still owns its diff.

- `3e2795b` `0238695` `3f4e3b5` `b70fb16` `ea3d92c` — re-verification pruned ~65 stale Wave-2/3 blocks from `ISSUES.md` (815L → 484L). The tracker now reflects code. Saved 8+ reimplementations of already-shipped items (#134 tokio::join!, #231-233/251/253 release script, #258 writeln, #259 sha1 batch, etc.).
- Real engineering (3 fixes, all `cargo check` + `npm run check` clean):
  - **#200** `DriftSummaryCard.svelte` now consumes `syncPage.groups` instead of re-grouping `entries` (drops a parallel O(entries) pass per drift event; component is 35 lines shorter).
  - **#248** new `diag_log_frontend_error` Tauri cmd + `src/lib/util/diag.ts` helper. 4 sites wired: `connection.svelte.ts` (connect/auto-connect/auto-reconnect) + `SyncActivityBanner.svelte` (reconnect). Frontend failures now show up in Diagnostics panel + activity feed, not just devtools console.
  - **#249** `diag_state_pump` gated on subscriber refcount (`DiagPumpSubscribers(AtomicU64)`). Idle Rift no longer pays the 500ms collect-lock-emit cost when Diagnostics tab is closed. Refcount handles `<Diagnostics embedded />` nesting; `diagnostics.svelte.ts` `wire()`/`dispose()` manage 1:1 sub/unsub via `subscribed` flag.

Pre-existing `ReleaseMeta` visibility warn in `update_service.rs` is the only `cargo check` warning, unchanged.

## v0.4.30-alpha — 2026-05-25 — rail trim + command palette + observability cleanup (shipped)

Detail in CHANGELOG.

### In-flight (unshipped) — updater overhaul 2026-05-25

Root cause of 5-10 min "Applying…" on buddy's machine: `velopack::UpdateManager::apply_updates_and_restart` spawns `Update.exe --waitPid <pid>` and returns — does NOT exit Tauri. Fix: `app.exit(0)` after `svc.apply()` in `commands/update.rs::apply_pending_update`, 150ms IPC-flush delay.

Companion changes:
- `assistant::kill_child_processes_on_exit()` — `taskkill /F /T` tracked claude CLI children before exit so Update.exe doesn't trip on locked handles.
- Background pre-download on launch — `updates.checkOnLaunch` auto-fires `download()`; user-initiated apply skips straight to the swap.
- Apply-phase UX: `applyingHint` swaps copy at 20s + 90s; `applyingStuck` drives a Force-Quit dialog button.
- Delta-vs-full path logging in `UpdateService.download`.

Verified clean both stacks. NOT yet shipped — needs real two-machine apply test before bump.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.30-alpha** shipped 2026-05-25 (rail trim + command palette + observability cleanup). Tauri 2 + Svelte 5 + Rust + russh.

**Velopack U+00D7 fix** in `release.ps1::Convert-ToAsciiSafe` — don't remove (closed Wave-3 ship-blocker; see CHANGELOG v0.4.27).

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations`. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: `--permission-mode bypassPermissions` + full `BUILTINS` in `--allowed-tools`.
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` w/ attachments. 20MiB cap.
- Settings is workspace (kbd **5** post-v0.4.30 rail trim), `Ctrl+,` flips; no slideover scrim.
- `UpdateService` managed Tauri state — `download_update` then `apply_pending_update`.
- `tauri.conf.json` `dragDropEnabled: false` — required for HTML5 DnD.
- AssistantPane drop handlers on `.pane` outer only — inner overlays break preventDefault chain.
- `compactionHistory[]` is camelCase in persisted JSON. Don't rename.
- `.shell` MUST be `position: fixed; inset: 0` (AppShell). `body.win-maximized .shell { inset: 8px }` for borderless-maximized.
