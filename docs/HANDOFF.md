# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Branch in flight — `updater-migration` — Velopack → tauri-plugin-updater

**Branch:** `updater-migration` (HEAD on main = v0.4.31-alpha). Backend + frontend code complete; `/check` clean (0 errors / 0 warnings), `cargo check` clean. Brief: [docs/design/updater-migration.md](design/updater-migration.md). Old `updater-overhaul-awaiting-2machine-test` stash was **dropped** — superseded.

Done on the branch:
- Velopack + ureq removed; `tauri-plugin-updater` + `tauri-plugin-process` added.
- `update_service.rs` deleted; `commands/update.rs` rewritten against `UpdaterExt`.
- `lib.rs`: dropped `VelopackApp::build().run()` + `UpdateService` state; added plugin inits.
- `assistant::kill_child_processes_on_exit` salvaged → wired via `on_before_exit`.
- Frontend store API preserved; added `update-size` listener + auto-DL on launch.
- `tauri.conf.json`: `createUpdaterArtifacts: true`, pubkey + `installMode: "passive"`.
- Capabilities: `updater:default`, `process:default`.
- `scripts/release.ps1` rewritten (Tauri-only, v0.4.33+). `scripts/release-bridge.ps1` written (one-time v0.4.32 hybrid).
- Signing key generated at `C:/Users/BLAZZER/.tauri/rift.key` (passwordless); `TAURI_SIGNING_PRIVATE_KEY_PATH` exported via `.secrets/env.sh`.

Smoke build verified end-to-end:
- `npm run tauri build` produces `Rift_0.4.32-alpha_x64-setup.exe` (7.2 MB) + `.sig` (424 B minisign).
- Required env: `TAURI_SIGNING_PRIVATE_KEY=<path>` AND `TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""` (both in `.secrets/env.sh`). The empty password is non-negotiable even on a passwordless key — bundler still prompts otherwise.
- `release.ps1` + `release-bridge.ps1` setup-exe glob is now version-scoped (`*_${version}_*-setup.exe`) so the shared bundle dir's accumulated artifacts don't trip "exactly one" preflight.
- `vpk`, `gh`, `npm` all on PATH. `latest.json` shape dry-run matches Tauri's expected schema.

**Resume here (everything else still gated on you):**
1. **BACK UP `C:/Users/BLAZZER/.tauri/rift.key` OFF-MACHINE.** Vault / encrypted drive / 1Password. Lose this file = no v0.4.32+ install can ever update again. Hard gate.
2. `pwsh scripts/release-bridge.ps1` → ships v0.4.32-alpha hybrid (vpk + tauri-updater assets).
3. Update both machines to v0.4.32. Expect 5-10 min apply hang on v0.4.31→v0.4.32 (documented in CHANGELOG; manual Setup.exe in release assets is the escape hatch). Confirm BOTH on v0.4.32 before proceeding.
4. For v0.4.33+: regular flow — `bump.ps1` → CHANGELOG entry → `pwsh scripts/release.ps1` (clean Tauri-only).
5. After v0.4.33 ships clean, retire `release-bridge.ps1` (delete or leave as historical).

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.31-alpha** shipped 2026-05-26. Migration branch above gates next ship. Tauri 2 + Svelte 5 + Rust + russh.

**Velopack U+00D7 fix** in pre-migration `release.ps1::Convert-ToAsciiSafe` — preserved in new `release.ps1` + `release-bridge.ps1` as `×`→`x` regex line (see CHANGELOG v0.4.27).

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- `C:/Users/BLAZZER/.tauri/rift.key` — Tauri-updater signing key. Lose it and no v0.4.32+ install can update. Pubkey in `tauri.conf.json::plugins.updater.pubkey`; do NOT regenerate. Env: `TAURI_SIGNING_PRIVATE_KEY_PATH` via `.secrets/env.sh`.
- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `bundle.targets:["nsis"]`. `createUpdaterArtifacts: true`.
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
