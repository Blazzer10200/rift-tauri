# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-27 (late night) — v0.4.33 SHIPPED + signing-key rotated

**v0.4.33 LIVE** → https://github.com/Blazzer10200/rift-releases/releases/tag/v0.4.33. Single commit `f8ed3fd` bundled browser dock + multi-target CDP + assistant M6/M7 + prior committed v0.4.33 work. Follow-up `40f4be4` = pubkey + release.ps1 fixes. Detail in CHANGELOG.

**Signing key rotated** because prior `rift.key` had a lost passphrase. New ed25519 keypair, password `rift-updater-2026` in `.secrets/env.ps1` (gitignored). New pubkey in `tauri.conf.json`. **CONSEQUENCE:** v0.4.32 clients reject v0.4.33 signature → one-time manual `Setup.exe` install required (see Next Step 1).

**Pipeline polish.** `release.ps1` auto-loads key via PS `ReadAllText` (bash `$(<file)` strips trailing newline = corrupt key), dot-sources `.secrets/env.ps1`, pipes empty stdin to `cmd /c npm run tauri build` as failsafe, warns when password env missing. Old key archived `rift.key.encrypted-irrecoverable-2026-05-27`; new key backed up OneDrive + iCloud `rift-signing-key-backup/`.

### Next Steps
1. **Tell buddy:** download + run `Rift_0.4.33_x64-setup.exe` from the release link ONE TIME (v0.4.32 pubkey ≠ v0.4.33 pubkey, in-app updater will refuse). v0.4.34+ auto-updates resume.
2. **🔄 NEXT SESSION = updater system overhaul** (user-flagged). Today's ship surfaced the brittleness — encrypted key w/ lost passphrase, `rpassword` CONIN$ prompt that can't be piped, shim-chain swallows stdin. Decision needed: harden `release.ps1` incrementally vs greenfield rebuild (maybe drop tauri-updater entirely). Start session w/ `/plan`.
3. **MCP browser tools** — expose `browser_navigate` / `browser_eval` / screenshot via `mcp_server.rs` so the assistant can drive the dock.
4. Retire `scripts/release-bridge.ps1` (spent).

---

## Session 2026-05-27 (night) — in-app browser dock + multi-target CDP [compressed → shipped]
Native child webview (Tauri `unstable` `Window::add_child`) embedded in the assistant page, no taskbar bleed, native scroll/select/click. New `browser/mod.rs` + `commands/browser.rs` (7 async commands — sync deadlocks `add_child` on Windows), `WebBrowserPage.svelte`, `browserDock.svelte.ts`. CDP wrapper got a per-target connection registry (`-t browser` flag, auto-heal across nav). All shipped in v0.4.33. Detail → CHANGELOG + git.

## Session 2026-05-27 (evening) — dev-speed + assistant split M6/M7 [compressed → shipped]
Hook fix (cargo-check-kills-dev), svelte-check bin path, M6 tabs, M7 compaction, Composer ctx-gauge + attach, MessageBubble turn-actions regroup, ChatTabsBar detail popover. Shipped in v0.4.33.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. Latest public release = **v0.4.33** (shipped 2026-05-27 via Tauri-only `release.ps1`, signing key rotated — see late-night session above). Branch `updater-migration` still in flight; merge to main is its own task. Tauri 2 + Svelte 5 + Rust + russh.

**Next session = updater system overhaul** per user request — see Next Steps in late-night session. Start with `/plan` to weigh incremental hardening vs greenfield replacement.

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- `C:/Users/BLAZZER/.tauri/rift.key` — Tauri-updater signing key, ROTATED 2026-05-27 (v0.4.33). Password `rift-updater-2026` lives in `.secrets/env.ps1` (gitignored). Pubkey in `tauri.conf.json::plugins.updater.pubkey` matches the new key; do NOT regenerate without a transition release (would strand v0.4.33 clients). Old encrypted key archived as `*.encrypted-irrecoverable-2026-05-27`. Backups: OneDrive + iCloud `rift-signing-key-backup/*.bak-2026-05-27-rotated-pwd`.
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
