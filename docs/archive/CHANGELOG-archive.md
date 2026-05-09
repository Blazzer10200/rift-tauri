# rift-tauri — Changelog Archive

Older changelog entries flow here as new versions ship. Live entry stays in `docs/CHANGELOG.md`.

## v0.1.6-alpha — 2026-05-09 — Backend audit sweep: 85 findings, 6204 LOC touched

Backend audit + fix pass across all 23 `src-tauri/src/` files; critical/high/medium findings landed. Dev-only — no public ship. UI untouched.

**Critical:** Mutex-poison hardening (`sync_snapshot.rs`/`remote_state.rs`/`discovery.rs`); `SaveLocalCopy` data-loss guard; atomic `RiftConfig::save()` (tmp+rename); fire-and-forget `JoinHandle` tracking via `background_tasks`; `stat_local` Option return for missing-metadata paths.

**High:** `get_remote_sha1` stderr capture; empty-root retry tightened (HashMap-keyed, was N round-trips); per-permanent-drop log warn; `spawn_blocking` for blocking I/O; `editor_for` TOCTOU fix (lock held across SFTP open).

**Medium / cleanup:** `SHA1_MAX_BYTES` consolidation (3 defs → 1, fixes WPF mismatch); `MTIME_TOLERANCE_SECS` exported; shared `transport::{ssh_handler, env}` modules (~50L deduped); `paths::dirs_home` public; `local_fs::list_directory` canonical walker; temp-dir collision fix (pid+short_id, was nanosecond); `ensure_workers` `FuturesUnordered` short-circuit; private key loaded once in `OwnedConnectArgs`; tokio features narrowed; dead deps removed (anyhow/thiserror/etc); `hex_upper` write! optimization (~20× cheaper SHA1).

**Verified:** `cargo clippy --lib --tests -- -D warnings` zero warnings; `cargo test --lib` 47+1 = 48/48; `npm run check` 318 files / 0 errors / 0 warnings.

**Versions:** `Cargo.toml`, `package.json`, `tauri.conf.json` → 0.1.6. v0.1.5 archived.

## v0.1.5-alpha — 2026-05-09 — UI redesign foundation: Tailwind v4 + shadcn-svelte + Claude Design brief

UI redesign substrate laid. No public ship; Phase 6 still deferred. Migration core (v0.1.4) remains functionally complete; this version adds the visual-iteration foundation and the Claude Design handoff package.

### Frontend stack additions
- **Tailwind v4** wired via `@tailwindcss/vite` plugin in `vite.config.js`.
- **shadcn-svelte (nova style, zinc base)** initialized — `components.json` configured for `$lib/components/ui` aliasing.
- **Full OKLCH theme tokens** (light + dark zinc palette, `@theme inline` mappings) replace the bare `#0F0F12`/`#E8E8EE` body styling in `src/app.css`. Designer can replace the entire palette without breaking component contracts.
- **Dark-first** — `class="dark"` baked into `src/app.html` for first-paint, `<ModeWatcher defaultMode="dark" />` mounted in `+layout.svelte` for runtime toggling.
- **Smoke test** — `button` component scaffolded via `npx shadcn-svelte add button` to validate end-to-end pipeline.

### Dep additions
- devDeps: `tailwindcss`, `@tailwindcss/vite`, `tw-animate-css`.
- deps: `clsx`, `tailwind-merge`, `mode-watcher`, `bits-ui`, `tailwind-variants` (auto-pulled by button).

### Helpers
- `src/lib/utils.ts` — `cn` helper + `WithElementRef` / `WithoutChild` / `WithoutChildren` types (required by shadcn-svelte v1.2.7+).

### Documentation
- `docs/design/CLAUDE-DESIGN-BRIEF.md` — token-efficient one-shot context for Anthropic Labs' Claude Design (claude.ai/design). Pre-digests product summary, tech constraints, full component inventory, current OKLCH tokens, non-goals, and 4 starter direction prompts (Linear-precise / Raycast-dark-glass / Sublime-terminal / Win11-Mica). Avoids the codebase-attach token-burn.

### Verified
`npm run check` 318 files 0 errors 0 warnings ✓ · `npm run tauri dev` boots clean (Vite 701 ms, cargo cached) ✓.

### Versions bumped
`Cargo.toml`, `package.json`, `tauri.conf.json` → 0.1.5. v0.1.4 entry archived to `archive/CHANGELOG-archive.md`.

---

## v0.1.4-alpha — 2026-05-08 — Phase 5 dialogs + 1i write-back + Phase 0 stub cleanup

Migration core complete. All Phase 5 dialogs land; 1i ConfigStore write-back closes via TOFU fingerprint auto-persist + AddServer save_server cmd. Dev-only — no public ship. Committed Session 10 as `5b9f5f7`.

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
Removed Phase 0 stubs from `lib.rs`: `sftp_list` Tauri command, `ConnectArgs` + `ListEntry` types, `Client` Handler, `connect_sftp`, `addr_to_string`, duplicate `load_servers` cmd. ~110L dead code gone.

### Verified
`cargo check` ✓ · `cargo clippy --lib --tests` ✓ zero warnings · `cargo test --lib` 47/47 pass · `npm run check` 0/0.

---

## v0.1.3-alpha — 2026-05-08 — Backend 100% + UI shell + browser + sync surfaces

Sub-phases 1d-1h, 1j, 2, 3, 4 all landed under v0.1.3 in dev-mode (no public ship). Backend SYNC engine, SftpClient, SshTunnel, tail services all complete. UI shell, two-pane browser, and sync surfaces (activity, drift, conflicts, lock badges, edit-in-place) all live.

Major adds:
- **Phase 1g — `tunnel/mod.rs`** (~190L). russh `direct-tcpip` forwarder replacing WPF's 398L `ssh.exe -L` shellout. Lifecycle: `start_autosync` opens before BridgeClient, `stop_autosync` closes after engine drain.
- **Phase 1h — SftpClient gap-fill** (409→1116L). Fingerprint pinning (substring-match Rust `SHA256:<b64>` + WPF `ssh-ed25519 256 SHA256:<b64>`); 4-way worker pool; `download_files_batch`/`upload_files_batch`; worker-aware `list_recursive_batch` w/ empty-root retry; `discover_manifest_folders`; `list_directory`; `ensure_remote_parent_dir`; `get_remote_folder_size`. Deps: `sha2`, `base64`.
- **Phase 1j — tail services** (5 modules). `local_fs.rs`, `bootstrap/mod.rs` (6-state classifier), `transport/ssh_keygen.rs` (in-process ed25519 via `ssh-key`), `update_service.rs` (Velopack wrap), `edit/in_place.rs` (notify watcher, 400ms debounce). Dep: `rand`.
- **Phase 2 — UI shell.** Svelte 5 runes — AppShell, TopBar, ServerPicker, StatusHero, ActivityToast. State `connection.svelte.ts` w/ 5-state pill from autosync events. Tauri cmds: `list_servers`, `get_last_selected`, `set_last_selected`.
- **Phase 3 — two-pane browser.** PathBreadcrumbs, LocalPane, RemotePane, TwoPane (column resizer, tab strip, drop handlers). Tauri cmds: `local_list_dir`, `remote_list_dir`, `upload_paths`, `download_paths`. State `browser-tabs.svelte.ts` localStorage-backed.
- **Phase 4 — sync surfaces.** ActivityFeed (virtualized), DriftReview, ConflictList + ConflictResolver, LockBadge. State expanded for `LockEntry`, `ConflictRecord`, `dirtyEdits`. Tauri cmds: `begin_edit_in_place`, `save_edit_in_place`, `close_edit_in_place`, `list_watched_edits`.

Earlier sub-phases (1d-1f) — see prior v0.1.3 entry below.

## v0.1.3-alpha (sub-phases 1d + 1e + 1f) — 2026-05-08 — Lock / Bridge / Drift hashing

Backend feature-complete vs the WPF v13.55.x sync surface. Dev-only.

### Phase 1d — `LockPresence` (`.rift-lock` cross-dev coordination)
- `sync/lock_presence.rs` — port of WPF `LockPresence.cs` (265L). 10s poll loop walks watched roots for `*.rift-lock` (depth 4 scoped, 6 fallback). Stale lock sweep at 180s. `acquire`/`release`/`find_lock_by_other`. Fires `autosync://locks` Tauri event.
- AutoSync wiring: drops a lock fire-and-forget on first dirty event for a path; releases on `Deleted` or successful flush. Pre-push foreign-lock check requeues w/ 30s delay.

### Phase 1e — `BridgeClient` (FXServer hot-reload)
- `bridge/mod.rs` — port of WPF `BridgeClient.cs`. `reqwest` (rustls), 8s timeout, `X-Rift-Token` bearer. `POST /sync-done?resource=<name>`. Auto-fired by AutoSync after each successful batch when ServerProfile has both `bridge_port` + `bridge_token`.

### Phase 1f — DriftScanner SHA1 hashing (deferred from 1b)
- Per-folder hash budget: 25 SHA1 calls per folder (WPF v13.55.18 fix replicated).
- Stat-only jitter elimination, false-conflict collapse, first-scan opportunistic equality.
- Replaced 1b's `should_ignore_basic` w/ `crate::sync::ignore::should_ignore` for full parity.

### Deps added (1e)
`reqwest = "0.13"` w/ `rustls,charset,http2,system-proxy` features (no openssl).

---

## v0.1.2-alpha — 2026-05-08 — Phase 1c — AutoSync engine

Port of `Services/Sync/AutoSync.cs` (1167L C#) to Rust. Dev-only.

- `sync/auto_sync.rs` — `AutoSyncEngine` (~750L). `notify` v8 file watcher → mpsc → tokio event task → 700ms debounce / 3000ms ceiling per-file → 150ms-tick flush task → `SftpClient::upload_file_atomic`. DashMap state. Mass-delete circuit breaker. Conflict pre-flight + `ConflictRecord` event. `BypassPreflight` flag for drift-resolved pushes. Auto-retry backoff 30s/2m/10m.
- `sync/ignore.rs` — full WPF `ShouldIgnore` parity. 7 extensions, 4 exact filenames, 25 path-segments. `web/build/` + `web/dist/` FiveM bypass. `.tmp.<digits>` + `.backup.<digits>` editor patterns.
- SftpClient additions: `remote_stat`, `rename`, `delete`, `mkdir_p`, `upload_file_atomic`, `download_file_atomic`, `OpResult`.
- Tauri commands: `start_autosync`, `stop_autosync`, `get_autosync_status`, `enqueue_for_flush_batch`, `resolve_conflict`, `retry_failed`. Events: `autosync://status`, `autosync://activity`, `autosync://conflict`.
- Deps: `dashmap`, `walkdir`, `notify-debouncer-full`, `futures`.
- Tests: 32/32.

## v0.1.1-alpha — 2026-05-08 — Phase 1a + 1b backend port

### Phase 1a — state caches (`src-tauri/src/state/`)
- `sync_snapshot` — 3-way drift baseline. `Entry { local_size, local_mtime_utc, remote_size, remote_mtime_utc, sha1 }`. Static helpers `local_matches`/`remote_matches` (2s mtime tolerance) + `compute_sha1`.
- `remote_state` — `RemoteStateCache`, last-known remote `(size, mtime_utc)` per file.
- `discovery` — `ResourceDiscoveryCache`, discovered resource folders + cachedAt.
- `paths` — `~/.rift/` resolver, profile-key sanitizer, atomic tmp+rename.

### Phase 1b — SFTP + drift detection
- `sftp::SftpClient` — connect, `list_recursive`, `list_recursive_batch`, `get_remote_sha1`, `remote_exists`, `upload_bytes`, `download_file`. Subset port.
- `sync::edit_trail` — `.rift-trail.jsonl` append-only log on remote, capped at 500 lines.
- `sync::drift_scanner` — 3-way bucket (`Synced` / `ToPush` / `ToPull` / `Conflict`).
- `profile::RiftConfig` — read-only `~/.rift/rift.json` loader. Preserves unknown fields via `serde(flatten)`.

### Tests
17/17 passing. 2766-entry real snapshot deserialize verified (ignored test).

## v0.1.0-alpha — 2026-05-08 — Phase 0 stub

Toolchain probe + foundation. Connected to a server, listed a remote dir. Tauri 2.0 + Svelte 5 + TS scaffold + `russh` 0.54 + `russh-sftp` 2.1 + `velopack` + `vpk pack`. Released to `Blazzer10200/rift-tauri` v0.1.0-alpha. Phase 0 stub command `sftp_list` removed in Session 9 cleanup once Phase 5 superseded the use case.
