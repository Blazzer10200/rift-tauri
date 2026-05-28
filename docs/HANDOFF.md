# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-28 — v0.4.34 SHIPPED + post-ship cleanup

**v0.4.34 LIVE** → https://github.com/Blazzer10200/rift-releases/releases/tag/v0.4.34. The brittle updater system is gone for good. No more signing keys, no more `latest.json`, no more `.sig` files, no more CONIN$ passphrase trap. The new path is `commands/update.rs` polling `api.github.com/.../releases/latest`, semver-comparing, opening Setup.exe in the user's browser on confirm. ~200L Rust + ~150L Svelte replaced ~700L of plugin glue + signing infrastructure.

**Cleanup landed in same session** — `scripts/release-bridge.ps1` deleted (spent), `docs/design/updater-migration.md` deleted (superseded by the v0.4.34 implementation itself), `.github/workflows/release.yml` placeholder deleted (referenced vpk + Azure signing — all dead), `Releases/` scratch dir cleared (27 stale velopack nupkgs + RELEASES + assets.win.json + Rift-win-Setup.exe), `src-tauri/src/assistant::kill_child_processes_on_exit` removed (orphan after deleting `on_before_exit` hook). HANDOFF + CHANGELOG + CLAUDE.md + ISSUES.md (#14/#15/#17) all reflect current reality.

### Next Steps
1. **Tell buddy:** download + run `Rift_0.4.34_x64-setup.exe` from the release link ONE TIME (v0.4.33's old plugin can't see v0.4.34 — no `latest.json` published this release). v0.4.35+ auto-updates resume via the new GH-release-API path with zero friction.
2. **Delete the signing keys** (no longer needed): `~/.tauri/rift.key`, `~/.tauri/rift.key.pub`, OneDrive + iCloud `rift-signing-key-backup/`. Optional but recommended — they're attack surface for zero gain now.
3. **Merge `updater-migration` → `main`** when buddy is on v0.4.34 + you've confirmed the in-app updater dialog says "You're up to date" running v0.4.34.
4. **Real `release.yml` CI** (issue #14) — much smaller scope now that no signing secrets exist; ~30L on `tag-push: v*`. Optional, not blocking anything.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. Latest public release = **v0.4.34** (shipped 2026-05-28 via GH-release-API path, no signing key). Branch `updater-migration` still in flight; merge to main is queued (see Next Step 3). Tauri 2 + Svelte 5 + Rust + russh.

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`REPO` → public `rift-releases`, NOT source repo. Hardcoded in `release.ps1` + `commands/update.rs::RELEASES_REPO`.
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
- Updater is signing-key-free as of v0.4.34. Do NOT reintroduce `tauri-plugin-updater` / `createUpdaterArtifacts` / `.sig` / `latest.json` — see CHANGELOG v0.4.34 for why.
