# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** WPF→Tauri migration of Rift. Sibling repo to `Blazzer10200/rift` (WPF v13.55.x line still ships from there). User's daily app = WPF Rift. This project = the v14.0.0 future.

**🚨 PATH SANITY CHECK — DO THIS FIRST:** Confirm `pwd` is `C:/AI Workflow/rift-tauri/`. The sibling WPF repo at `C:/AI Workflow/Rift Project/` is **stable port-from reference** — DO NOT refactor it unless the user explicitly says "WPF" or "v13.x". When user says "rift" or "the rift project", default to `rift-tauri`.

**🚨 FIRST THING NEXT SESSION — COMMIT THE BACKLOG.** Everything from Sessions 2–9 is sitting uncommitted on top of the bare Phase 0 scaffold. Last commit = `d9464dc Phase 0 — Tauri 2.0 + Svelte 5 + russh-sftp + Velopack scaffold`. Working tree has: full `src-tauri/src/` backend tree (sftp, sync, bridge, profile, state, tunnel, transport, edit, bootstrap, local_fs, update_service), full `src/lib/` Svelte tree (components, state, dialogs), `docs/archive/`, modified Cargo.toml/package.json/tauri.conf.json/lib.rs/HANDOFF.md/CHANGELOG.md, plus `build/` (Vite output — likely belongs in `.gitignore`). User said this is intentional — the plan is `/git-ship` next session as one milestone commit. Don't start new work until that's done.

**Current state (post Session 9, v0.1.4-alpha — built but uncommitted):** Phase 5 ✅ shipped. All 6 dialogs (AddServer, Bootstrap, Keygen, Reupload, Confirm, CommandPalette) live in `src/lib/components/dialogs/`. 1i closed via TOFU auto-persist in `open_sftp_for` + `start_autosync` + `scan_drift`. Migration core is **functionally complete**. Phase 6 (v14.0.0 ship) is the only remaining phase but is deferred per user. Release build artifacts produced at `src-tauri/target/release/bundle/{nsis,msi}/` and `src-tauri/target/release/rift-tauri.exe` (gitignored).

**UI redesign anticipated:** WPF visual parity is non-binding from here. User explicitly said the Tauri/Svelte UI gives them better design control vs MahApps and that pixel-faithful porting is unnecessary. Future sessions can iterate freely on layout/palette/typography.

**Next session — start here:** (1) `/git-ship` the backlog FIRST. Then either (a) UI redesign exploration on the new dialogs + shell, or (b) Phase 6 ship prep when user is ready (Velopack-Rust release flow, dual-run validation, sunset banner on WPF v13.x).

**Release policy:** No public ship until user explicitly says go. Iterate via `npm run tauri dev`. v0.1.4 is dev-only.

## Verified phase status

| Phase | Scope | Status |
|---|---|---|
| 0 + 1a–1h, 1j | Backend (state, sftp, sync engine, tunnel, tail services) | ✅ |
| **1i** | **ConfigStore write-back** — `last_selected` ✅ · AddServer ✅ · TOFU fingerprint persist ✅ | ✅ Session 9 |
| 2 | UI shell — MainWindow port (Svelte 5 runes) | ✅ |
| 3 | Two-pane browser | ✅ |
| 4 | Sync surfaces (activity, drift, conflicts, locks, edit-in-place) | ✅ |
| **5** | **Dialogs — AddServer / Bootstrap / Keygen / Reupload / Confirm / CommandPalette** | ✅ Session 9 |
| 6 | v14.0.0 ship | ⏳ deferred |

## Session 9 — 2026-05-08 — Phase 5 dialogs + 1i closure + cleanup

### New components (`src/lib/components/dialogs/`)
- **`AddServer.svelte`** — 3-step stepper (Connection → Workspace → Bridge & Save) w/ per-step validation, summary card, edit-mode pre-fill, auto-suggest display name from host, `txAdmin` URL test via `plugin:opener|openUrl`.
- **`Bootstrap.svelte`** — 6-state-aware UI driven by `detect_bootstrap` payload; chunked download via `bootstrap_list_files` + `download_paths` (50/chunk progress).
- **`Keygen.svelte`** — wraps existing `default_ssh_key_exists` / `generate_default_ssh_key` / `read_default_ssh_pub_key`; Copy pubkey via `navigator.clipboard`.
- **`Reupload.svelte`** — Skip / Always / Re-upload triplet. Wired for future edit-in-place + autosync prompts.
- **`Confirm.svelte`** — generic Yes/No w/ `isDanger` styling + optional "Don't ask again" checkbox.
- **`CommandPalette.svelte`** — Ctrl+K. Tokenized AND-match fuzzy filter over registered actions; ↑↓/Enter/Esc keybinds; mouse hover sets selection.

### Backend additions (lib.rs + profile/mod.rs)
- **`save_server(profile, edit_key)`** — adds or updates a server. `editKey=None` → slugify name + `unique_key` collision resolution. Edit mode preserves stable key + existing fingerprint.
- **`delete_server(key)`** — removes profile; if it was `last_selected`, falls back to first remaining server.
- **`bootstrap_list_files(server_key, local_root)`** — recursive remote walk (depth 8, skips `[disabled]/`) returning (remote, local) job pairs ready for `download_paths`.
- **`profile::slugify`** + **`profile::unique_key`** + **`RiftConfig::save`** — write-back foundation.
- **TOFU fingerprint persist** — `persist_fingerprint_if_new(key, fp)` called from `open_sftp_for` + `start_autosync` + `scan_drift` after successful connect when profile fingerprint is empty. Refuses to silently overwrite a mismatched pinned value (logs `warn!`).

### Wire-up
- `ServerPicker` rewired w/ Add/Edit/Delete buttons + Setup-key launcher.
- `AppShell` mounts all 6 dialogs as state-driven components, registers 11 commands (Switch / Add / Setup-key / Bootstrap / 5 tab-jumps / Disconnect / Reload), binds Ctrl+K (palette) + Ctrl+P (picker), surfaces Settings tab w/ direct buttons.
- `connection.svelte.ts` adds `deleteServer(key)` helper.

### Cleanup
- Removed Phase 0 stubs from `lib.rs`: `sftp_list` cmd, `ConnectArgs`/`ListEntry` types, `Client` Handler, `connect_sftp`, `addr_to_string`, duplicate `load_servers` cmd. ~110L dead code gone.
- Bumped to v0.1.4 across `Cargo.toml`, `package.json`, `tauri.conf.json`.

### Verified
`cargo check` ✓ · `cargo clippy --lib --tests` ✓ zero warnings · `cargo test --lib` 47/47 ✓ · `npm run check` 0 errors 0 warnings.

### Smoke test recipe (`npm run tauri dev`)
1. Empty `~/.rift/rift.json` → picker opens auto → "+ Add server" → stepper → Save → server appears + `last_selected` persisted.
2. Edit existing → fields pre-fill → Save → key stays stable, fingerprint preserved.
3. Ctrl+K → fuzzy search "boot" → Bootstrap → detection runs → download or BadRemoteRoot guidance.
4. SSH key setup from picker → generates default ed25519 if missing → Copy pubkey works.
5. Delete from picker → Confirm dialog (danger style) → server removed.
6. First connect to a fingerprint-less profile → after `start_autosync` succeeds, `~/.rift/rift.json` should now contain `"fingerprint": "SHA256:<b64>"` for that server.

## CRITICAL DON'T-TOUCH (carries forward)
- russh `ring` backend (NASM blocker on aws-lc-rs)
- reqwest `rustls` features only — never enable `default-tls`/`native-tls` (drags openssl)
- npm runner, NOT pnpm
- File-format compat w/ WPF `~/.rift/*.json` — never change PascalCase/camelCase rename rules; never drop the `serde(flatten) extra` bag on `RiftConfig`
- Velopack `VelopackApp::build().run()` first call in `run()`
