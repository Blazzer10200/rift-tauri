# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-27 (night) — in-app web browser dock

### Completed
- **M0 transport spike** — `chromiumoxide` attach-mode to WebView2 CDP port proven: connect → `new_page` → navigate → screenshot. Spike at `%TEMP%/cdp-spike/` (disposable).
- **Browser feature — native child webview** via Tauri `unstable` `Window::add_child`. No screenshot-stream, no CDP page, no separate taskbar window. Ships as part of the chat workspace.
- **New files:** `src-tauri/src/browser/mod.rs` (native webview lifecycle), `src-tauri/src/commands/browser.rs` (7 async Tauri commands), `src/lib/components/webview/WebBrowserPage.svelte` (address bar + stage placeholder + bounds reporting), `src/lib/state/browserDock.svelte.ts` (open/width state, persisted).
- **Wired into:** `lib.rs` (mod + command registry), `commands/mod.rs` (pub use), `AppShell.svelte` (dock.init()), `ChatTabsBar.svelte` (Browser toggle button), `AssistantPage.svelte` (workbench layout + dock divider + show/hide $effect).
- `cargo check` 0 errors, `npm run check` 0/0. Dev running w/ feature live.

### In Progress — RESUME HERE
- **Native webview confirmed loading** (CDP target = `https://example.com/ | Example Domain`). NOT yet user-visually confirmed: (1) no taskbar window, (2) page visible + positioned inside panel. User was about to check when compaction triggered.
- **Two bugs fixed during session:** (a) `about:blank` deadlock — sync commands deadlock `add_child` on Windows (Tauri known issue); fixed by making all `browser_*` commands `async`. (b) CDP-created page produced separate taskbar window — reason we switched from screenshot-stream to native child webview.

### Key Decisions
- Native child webview (`unstable`) over screenshot-stream CDP — no taskbar entry, native scroll/select/click, simpler.
- Browser lives in the **assistant page** as a toggleable dock (not a separate workspace rail entry — reverted cleanly).
- Commands **must be async** — sync + `add_child` deadlocks the Windows main thread.

### Failed / Don't Retry
- **Sync `#[tauri::command]` + `add_child`** — deadlocks. All browser commands are now `async`. Don't revert.
- **CDP screenshot-stream approach** — `new_page` spawns a visible top-level WebView2 window (taskbar entry). Dropped.

### Next Steps
1. **User visual confirm** — check taskbar (single Rift icon?) and dock renders correctly. If positioning is off, `browser_set_bounds` / the ResizeObserver in WebBrowserPage are the fix points.
2. **Agent MCP tools** — expose `browser_navigate` / `browser_eval` / screenshot to the in-app Claude assistant via `mcp_server.rs` so user can tell the assistant to drive the browser.
3. **Prod CDP gating** — `unstable` multiwebview doesn't need CDP, but the `tauri = { features = ["unstable"] }` flag in `Cargo.toml` is now live; note before v0.4.33 ship.
4. **Click/type proxying** — removed from native approach (native webview handles input directly); no Rust work needed.
5. **M8/M9 assistant split** (pre-existing) — streaming pump + send/stop extract.

### Files Modified
- `src-tauri/Cargo.toml` — `tauri = { features = ["unstable"] }`, chromiumoxide removed
- `src-tauri/src/lib.rs` — `pub mod browser`, command registry (7 browser_* commands)
- `src-tauri/src/commands/mod.rs` — `pub mod browser; pub use browser::*`
- `src-tauri/src/browser/mod.rs` (new)
- `src-tauri/src/commands/browser.rs` (new)
- `src/lib/state/browserDock.svelte.ts` (new)
- `src/lib/components/webview/WebBrowserPage.svelte` (new)
- `src/lib/components/AppShell.svelte` — `browserDock.init()` in onMount
- `src/lib/components/shell/ChatTabsBar.svelte` — Browser toggle button
- `src/lib/components/assistant/AssistantPage.svelte` — workbench/dock layout

---

## Session 2026-05-27 (evening) — dev-speed + assistant split M6/M7 [compressed]
Hook fix (cargo-check-kills-dev), svelte-check bin path, M6 tabs, M7 compaction, Composer ctx-gauge + attach, MessageBubble turn-actions regroup, ChatTabsBar detail popover. All `npm run check` 0/0, CDP-verified. Detail → CHANGELOG + git.

---

## v0.4.33 work — COMMITTED to `updater-migration` (2026-05-27), NOT released

Gated on two-machine confirm. Full detail → CHANGELOG. Summary: all 5 permission modes verified, enhancer streaming fixed, ctx-pill fix (39 tests pass — **not yet live-verified vs real long task**), chat UI polish (900px column, ask_user retheme, composer ghost fix). This machine running local `0.4.33-alpha` exe (manual swap, Velopack shell). **Other machine still on genuine 0.4.32.** GitHub release untouched.

---

## Branch in flight — `updater-migration` (Velopack → tauri-plugin-updater)

**v0.4.32-alpha SHIPPED 2026-05-26** to `Blazzer10200/rift-releases` (bridge already ran — all assets live, `latest.json` polled 108×). Branch version files now read **0.4.33-alpha** (bumped 2026-05-27 for the local build, uncommitted). Brief: [docs/design/updater-migration.md](design/updater-migration.md). Signing key `C:/Users/BLAZZER/.tauri/rift.key` — **backed up 2026-05-27 to OneDrive + iCloud** (`rift-signing-key-backup/`). `release.ps1` = Tauri-only path for v0.4.33+. `release-bridge.ps1` = the one-time v0.4.32 bridge, now spent (retire it).

Audit 2026-05-27 RESOLVED (prior session) — [docs/archive/audit-2026-05-27.md](archive/audit-2026-05-27.md). Open queue: 10 issues (#4 #7 #14 #15 #17 #20-M6-M9 #21 #29 #89 #265) → ISSUES.md.

---

## Ship v0.4.33 (all feature work committed, NOT released)
1. Key backup DONE (OneDrive + iCloud `rift-signing-key-backup/`); verify cloud copies synced. Feature commits DONE on `origin/updater-migration` (`4d669bf` + this session's v0.4.33 batch).
2. **THE GATE — confirm BOTH machines on v0.4.32 before shipping v0.4.33.** v0.4.33 ships via Tauri-only `release.ps1` with NO Velopack assets; a machine still on v0.4.31 would be permanently stranded. Setup.exe downloads on the live 0.4.32 release = 0, so the Tauri install path may not have run on either — verify both first.
3. **Ship v0.4.33-alpha** (after gate): `pwsh scripts/bump.ps1 0.4.33-alpha` (3 version files) → set date on v0.4.33 CHANGELOG entry → quit dev (frees `C:\cargo-targets`) → `pwsh scripts/release.ps1` (NOT release-bridge). Then retire `release-bridge.ps1`.
4. Optional pre-ship: live-verify the ctx-pill fix vs a real long task (CDP multi-step).

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. Latest public release = **v0.4.32-alpha** (shipped 2026-05-26, bridge). Next ship = v0.4.33 on `updater-migration` — gated on two-machine confirm (see Resume). v0.4.33 work (permission modes Piece 1+2, ctx-pill fix, enhancer streaming+actionable) all COMMITTED, tree clean. Tauri 2 + Svelte 5 + Rust + russh.

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- `C:/Users/BLAZZER/.tauri/rift.key` — Tauri-updater signing key. Lose it and no v0.4.32+ install can update. Pubkey in `tauri.conf.json::plugins.updater.pubkey`; do NOT regenerate.
- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `bundle.targets:["nsis"]`. `createUpdaterArtifacts: true`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations`. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: always `--input-format stream-json` + `--permission-prompt-tool stdio` + initialize handshake + stdin kept open for turn. `--allowed-tools` is mode-aware: bypass/auto = full BUILTINS; prompting modes = SAFE_BUILTINS only.
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` w/ attachments. 20MiB cap.
- Settings is workspace (kbd **5** post-v0.4.30 rail trim), `Ctrl+,` flips; no slideover scrim.
- `tauri.conf.json` `dragDropEnabled: false` — required for HTML5 DnD.
- AssistantPane drop handlers on `.pane` outer only — inner overlays break preventDefault chain.
- `compactionHistory[]` is camelCase in persisted JSON. Don't rename.
- `.shell` MUST be `position: fixed; inset: 0` (AppShell). `body.win-maximized .shell { inset: 8px }` for borderless-maximized.
