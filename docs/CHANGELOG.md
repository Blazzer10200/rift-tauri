# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.1.4-alpha — 2026-05-08 — Phase 5 dialogs + 1i write-back + Phase 0 stub cleanup

Migration core complete. All Phase 5 dialogs land; 1i ConfigStore write-back closes via TOFU fingerprint auto-persist + AddServer save_server cmd. Dev-only — no public ship.

### Phase 5 dialogs (`src/lib/components/dialogs/`)
- **`AddServer.svelte`** — 3-step stepper (Connection → Workspace → Bridge & Save). Per-step validation gates Continue button; allValid gates Save. Edit-mode pre-fills + preserves stable `key` + existing `fingerprint`/`addedAt`/`bridgeToken`. Auto-suggests display name from host (Add only). `txAdmin` Test opens via `plugin:opener|openUrl`.
- **`Bootstrap.svelte`** — driven by `BootstrapDetection` payload; renders state-specific copy for all 6 states (Synced / MissingLocalRoot / Empty / Uninitialized / Partial / BadRemoteRoot). BadRemoteRoot refuses bulk download + retitles to point at profile fix. Chunked download (50/chunk) via `bootstrap_list_files` → `download_paths`; cancellable mid-flight.
- **`Keygen.svelte`** — surfaces existing `default_ssh_key_exists` / `generate_default_ssh_key` / `read_default_ssh_pub_key`. Copy via `navigator.clipboard`. Refresh on `open` toggles.
- **`Reupload.svelte`** — Skip / Always / Re-upload triplet for future edit-in-place autosync prompts.
- **`Confirm.svelte`** — generic alertdialog w/ `isDanger` palette + optional "Don't ask again" checkbox. Esc=cancel, Enter=confirm.
- **`CommandPalette.svelte`** — Ctrl+K modal. Tokenized AND-match filter over registered Commands. ↑↓ navigate, Enter run (defers to next tick so action can open another dialog without z-order conflict), Esc close. Mouse hover updates selection.

### Backend (`src-tauri/src/lib.rs` + `profile/mod.rs`)
- **`save_server(profile, edit_key) -> ServerProfile`** — round-trips `RiftConfig` w/ `serde(flatten) extra` preserved. Add path slugifies `name` + applies `unique_key` collision resolution; edit path enforces stable `key` + preserves `fingerprint` if form didn't supply one. First save also sets `last_selected` if previously empty.
- **`delete_server(key)`** — removes profile; demotes `last_selected` to first remaining server when affected.
- **`bootstrap_list_files(server_key, local_root) -> Vec<(remote, local)>`** — recursive walk (depth 8, skips `/[disabled]/`), maps remote paths to local destinations, returns job list ready for `download_paths`.
- **`profile::slugify`** — lowercase, non-alphanumeric → single hyphen, trim trailing hyphens, "server" fallback for empty.
- **`profile::unique_key`** — `base` if no collision else `base-2`, `base-3`, …
- **`RiftConfig::save(&self)`** — atomic write helper. Refactored `set_last_selected` to use it.
- **TOFU fingerprint persist (1i closure)** — `persist_fingerprint_if_new(key, fp)` called from `open_sftp_for` + `start_autosync` + `scan_drift` post-connect when profile fingerprint is empty. Refuses to overwrite a mismatched pinned value (logs `warn!` instead).

### Wire-up
- `ServerPicker` rewired: Add/Edit/Delete row buttons + Setup-key launcher in header.
- `AppShell` mounts all 6 dialogs, registers 11 palette commands, binds Ctrl+K (palette) + Ctrl+P (picker), Settings tab gets direct-action buttons (picker / keygen / bootstrap).
- `connection.svelte.ts` adds `deleteServer(key)` helper.

### Cleanup
Removed Phase 0 stubs from `lib.rs`: `sftp_list` Tauri command, `ConnectArgs` + `ListEntry` types, `Client` Handler, `connect_sftp`, `addr_to_string`, duplicate `load_servers` cmd. ~110L dead code gone. `serde::{Deserialize, Serialize}` import retained (still needed by remaining types).

### Verified
`cargo check` ✓ · `cargo clippy --lib --tests` ✓ zero warnings · `cargo test --lib` 47/47 pass · `npm run check` 0 errors 0 warnings.

### Versions bumped
`Cargo.toml`, `package.json`, `tauri.conf.json` → 0.1.4. v0.1.3 entries archived to `archive/CHANGELOG-archive.md`.
