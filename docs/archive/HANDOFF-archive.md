# rift-tauri — Handoff Archive

Older session blocks flow here as new sessions land. Live handoff stays in `docs/HANDOFF.md`.

## Session 12 — 2026-05-09 — UI redesign port (Phases 0–3)

Claude Design "Rift App UI" deliverable bundle ported. **No backend touched.** 4 locked decisions: per-hunk conflict UI ships visually w/ file-level resolution under hood; drift severity skipped (no backend tag); ServerPicker retired (TopBar dropdown + Settings → Servers); version bump deferred to P11.

**P0** Foundation: OKLCH Rift tokens (4-step text scale, status semantics, density variants) + shadcn-svelte aliases. `@fontsource-variable/{inter,jetbrains-mono}` + `lucide-svelte`. `decorations: false`, 1600×1000 / min 1280×800. ModeWatcher dropped (dark-only).
**P1** Shell: Custom Titlebar (drag region + window controls + Cmd+K), TopBar rewrite (server dropdown + ConnPill), TabRail (200px sidebar w/ count pips + foot stats), StatusBar (22px). AppShell grid `32/44/1fr/22`. Ctrl+1..5 tab switching.
**P2** Settings: 7-section nav (Appearance/Tokens/Servers/Keys/Sync/Editor/About). Servers section absorbed ServerPicker CRUD; standalone deleted. Tokens section copies OKLCH blob.
**P3** Browser: New `1fr 36px 1fr` grid (no draggable splitter). `OpRail` center column (upload/download/sync/edit/diff/delete). PathBreadcrumbs gained side tag (LOCAL/REMOTE) + refresh + filter. LockBadge restyled.

`npm run check` clean. Dev build NOT yet runtime-tested; user to launch `npm run tauri dev` first thing next session.

## Session 10 — 2026-05-09 — Backlog commit + UI foundation + Claude Design brief

Sessions 2–9 bundled into `5b9f5f7 Phases 1-5 + 1i — migration core complete (v0.1.4-alpha)`. `build/` added to `.gitignore`.

UI foundation: **shadcn-svelte (nova/zinc) + Tailwind v4 + Bits UI + Svelte 5 native transitions**. Files: `vite.config.js` (Tailwind plugin), `src/app.css` (OKLCH zinc theme), `src/app.html` (`class="dark"`), `src/routes/+layout.svelte` (ModeWatcher), `src/lib/utils.ts` (`cn` helper), `components.json`, `src/lib/components/ui/button/` (smoke-test).

Claude Design brief at `docs/design/CLAUDE-DESIGN-BRIEF.md` — pre-digested context for claude.ai/design (product summary, tech constraints, component inventory, OKLCH tokens, 4 direction prompts: Linear / Raycast / Sublime / Win11-Mica).

Bumped 0.1.4 → 0.1.5. `npm run check` 318/0/0 ✓ · `tauri dev` boots ✓.

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

## Session 8 — 2026-05-08 — Phase 4 (sync surfaces UI)

Wired four placeholder sidebar tabs + lock badges + edit-in-place flow.
- `activity/ActivityFeed.svelte` — virtualized 26 px row, 8-row overscan, ResizeObserver-driven viewport. Filter input + kind dropdown.
- `drift/DriftReview.svelte` — `scan_drift` w/ user-typed subpaths (per-server `localStorage`). 3-bucket view + checkbox selections + Apply→`enqueue_for_flush_batch`/`download_paths`.
- `conflicts/ConflictList.svelte` + `ConflictResolver.svelte` — split-view local/remote metadata + 5 actions (Skip / Take local / Take remote / Save copy + pull / Edit in place). Edit opens via `plugin:opener|open_path`.
- `browser/LockBadge.svelte` — pill on file rows. RemotePane matches by full path; LocalPane by basename.
- State: `LockEntry` corrected to `{file_path,user,host,since}`; pill simplified. `ConflictRecord` expanded. New `dirtyEdits: Set<string>` from `edit://changed` + `edit://reuploaded` listeners.
- Tauri commands appended to lib.rs: `begin_edit_in_place`, `save_edit_in_place`, `close_edit_in_place`, `list_watched_edits`. Per-server `EditInPlaceManager` lazy-init via `editor_for(server_key)`.

Verified: `npm run check` 0 errors · `cargo check` clean · `cargo clippy` zero new warnings · `cargo test --lib` 47/47 pass.

## Session 7 — 2026-05-08 — Phase 1g + 1j + Phase 2 + Phase 3

**Phase 1g (SshTunnel):** new `tunnel/mod.rs` (~190L). russh `direct-tcpip` forwarder w/ same fingerprint-pin shape as sftp. Stop via `oneshot::Sender` drop. Bridge calls now hit local forwarded port (no external `ssh -L` needed). Wired into start_autosync/stop_autosync.

**Phase 1j (tail services):** five new modules — `local_fs.rs`, `bootstrap/mod.rs`, `transport/ssh_keygen.rs`, `update_service.rs`, `edit/in_place.rs`. In-process ed25519 keygen via `ssh-key`. Notify watcher w/ 400ms debounce + mtime/size delta. `Cargo.toml` adds `rand = "0.8"`.

**Phase 2 (UI shell):** Svelte 5 runes shell mounted. Top bar + sidebar + server picker + status hero + activity toast. State store `connection.svelte.ts` w/ `$state` + `$derived` 5-state pill. Tauri cmds: `list_servers`, `get_last_selected`, `set_last_selected`. `set_last_selected` round-trips `RiftConfig` w/ `to_string_pretty` so `serde(flatten) extra` preserves unknown WPF fields.

**Phase 3 (browser):** Two-pane file browser. Tauri cmds: `local_list_dir`, `remote_list_dir`, `upload_paths`, `download_paths` (4-way batch). Components: `PathBreadcrumbs`, `LocalPane`, `RemotePane`, `TwoPane` (mouse-drag column resizer, tab strip, drop handlers w/ MIME pairs `application/x-rift-local`/`application/x-rift-remote`). State `browser-tabs.svelte.ts` persists to `localStorage`.

Verified: 47/47 tests · clippy clean · `npm run check` 0 errors.

## Session 6 — 2026-05-08 — Phase 1h SftpClient gap-fill

Closed major SFTP feature gaps. Highlights:
- **Fingerprint pinning** — `ConnectArgs.trusted_fingerprint` substring-matches both `SHA256:<b64>` and full WPF `ssh-ed25519 256 SHA256:<b64>` form. TOFU first-connect captures via `fingerprint()` for caller persistence.
- **Worker pool** — up to 4 independent SSH+SFTP sessions. `download_files_batch`, `upload_files_batch`, worker-aware `list_recursive_batch` w/ belt-and-braces empty-root retry, `discover_manifest_folders` (pruned BFS).
- **Tail methods** — `list_directory` (sorted dirs-first), `ensure_remote_parent_dir`, `get_remote_folder_size` (server-side `find -prune` exec).

Verified: `cargo check` ✓ · `cargo clippy --lib --tests` ✓ zero warnings · `cargo test --lib` 35/35 ✓.

Known gaps: DriftScanner has no logger callback for suspicious-empty-remote bail. Per-file transfer-pct callbacks deferred to Phase 4 activity feed. Bridge calls fail w/ connection-refused unless external SSH tunnel up — Phase 1g.

## Session 5 — 2026-05-08 — Audit + cleanup pass

Re-mapped phases to honest status (over-claim correction — backend SYNC engine complete, NOT full WPF Services tree). 5 clippy warnings cleared. Resource leak fix: `start_autosync` was leaking `LockPresence` poll task on `start_with` Err / `try_watch` Err — now tears down via `lp.stop()` / `engine.stop()` on every early-Err path. `flush_batch` removed unnecessary `let this = self;` rebind inside `FuturesUnordered` push.

Verified clean: `cargo check` ✓ · `cargo clippy --lib --tests` ✓ (zero warnings) · `cargo test --lib` 35/35 ✓ (2 ignored — gated on real-user `~/.rift/snapshot-endure-rp.json`).

## Session 4 — 2026-05-08 — Phase 1d + 1e + 1f

LockPresence (270L, port of WPF `Edit/LockPresence.cs`): 10s poll, 180s stale sweep, scoped depth-4 walk; emits `autosync://locks`. Wired into AutoSync — acquire on first dirty, release on flush/Deleted, pre-push foreign-lock check w/ 30s requeue.

BridgeClient (102L, port of `Transport/BridgeClient.cs`): `reqwest` rustls, `X-Rift-Token`, `sync_done(resource)`. Auto-fires after each ok>0 batch when profile has bridge_port + bridge_token. **No SSH tunnel** — caller's responsibility (Phase 1g).

DriftScanner Phase 1f hashing: per-folder 25 SHA1 budget, stat-only jitter elimination, false-conflict collapse, first-scan opportunistic equality. Replaced `should_ignore_basic` w/ full `ignore::should_ignore`.

State at close: v0.1.3 in-progress, `cargo check` ✓ · `cargo test --lib` 35/35 ✓.

## Session 3 — 2026-05-08 — Phase 1c — AutoSync engine

`AutoSyncEngine` ~750L: notify v8 watcher → mpsc → tokio event task → 700ms/3000ms debounce-ceiling → 150ms-tick flush → bounded 4-way atomic upload via `FuturesUnordered`. DashMap state. Mass-delete circuit breaker (scaled threshold). 3-way conflict pre-flight w/ BypassPreflight. Auto-retry 30s/2m/10m. Conflict-resolve: Skip / SaveLocalCopy / ForceLocal / AcceptRemote.

`sync/ignore.rs` — full ShouldIgnore parity (7 ext, 4 exact, 25 segs incl. `[disabled]/`, `target/`, `.svelte-kit/`; `web/build|dist` FiveM bypass; `.tmp.<digits>` + `.backup.<digits>` patterns).

SftpClient additions: `remote_stat`, `rename`, `delete`, `mkdir_p`, `upload_file_atomic` (.rift-tmp + rename), `download_file_atomic`. `list_recursive_batch` now concurrent.

Tauri State + 6 commands (`start_autosync` / `stop_autosync` / `get_autosync_status` / `enqueue_for_flush_batch` / `resolve_conflict` / `retry_failed`). Events: `autosync://status` / `activity` / `conflict`.

Deps: `dashmap`, `walkdir`, `notify-debouncer-full`, `futures`.

State at close: v0.1.2 in-progress. `cargo check` ✓ · `cargo test --lib` 32/32 ✓.

## Session 2 — 2026-05-08 — Phase 1a + 1b backend port

### Phase 1b additions on top of Session 2 Phase 1a
- `src-tauri/src/sftp/mod.rs` — `SftpClient` struct (russh + russh-sftp wrapped). Methods: `connect`, `list_recursive(root, max_depth, ext_filter)`, `list_recursive_batch` (currently serial), `remote_exists`, `get_remote_sha1` (sha1sum exec), `upload_bytes`, `download_file`. Subset port of WPF's 1386L `SftpClient.cs`.
- `src-tauri/src/profile/mod.rs` — `RiftConfig` + `ServerProfile`. Read-only `~/.rift/rift.json` loader; `serde(flatten)` preserves unknown fields for Phase 2 write-back.
- `src-tauri/src/sync/edit_trail.rs` — `EditTrail` over SftpClient. 500-line cap.
- `src-tauri/src/sync/drift_scanner.rs` — `DriftScanner`. 3-way bucket against snapshot baseline. Stat-only first pass.
- `lib.rs` Tauri commands added: `load_servers`, `scan_drift({server_key, folders})`.

### Phase 1a state caches
Ported 4 persistent state caches → Rust modules under `src-tauri/src/state/`. Tree: `paths.rs`, `sync_snapshot.rs`, `remote_state.rs`, `discovery.rs`, `mod.rs`. **File-format compat:** PascalCase fields for `snapshot-{key}.json` + `state-{key}.json`, camelCase for `discovery-{key}.json`. Atomic write = `std::fs::rename` after `.json.tmp` write.

### State at session close
Cargo.toml + tauri.conf.json + package.json all at v0.1.1. `cargo check` ✓ · `cargo test --lib state::` 14/14 ✓ · 2766-entry real snapshot deserialize verified.

## Session 1 — 2026-05-08 — Phase 0 stub shipped (v0.1.0-alpha)

Toolchain installed clean (Rust 1.95.0, MSVC Build Tools, npm 11.12.1, Tauri CLI 2.11.1, vpk 0.0.1298). Scaffolded `C:/AI Workflow/rift-tauri/` via Tauri 2.0 + Svelte 5 + TS template (`com.blazzer.rift`). Cargo.toml: russh 0.54 (ring backend, no NASM), russh-sftp 2.1, notify 8, velopack, tokio full, anyhow, thiserror, chrono. `lib.rs` had one Tauri command `sftp_list` (since removed Session 9 cleanup).

**Gotchas recorded.** pnpm v11 `[ERR_PNPM_IGNORED_BUILDS]` blocks Tauri's pre-build install gate → switched to npm. `aws-lc-sys` needs NASM on Win → russh `default-features=false, features=["ring","rsa","flate2","async-trait"]`. `russh::Session::best_supported_rsa_hash()` returns `Result<Option<Option<HashAlg>>>` (triple-wrapped) → explicit `match Ok(Some(Some(h))) => Some(h)`. GitHub `PATCH /releases/<id>` 422s on empty repo → push initial commit before promoting from draft.

Released to `Blazzer10200/rift-tauri` v0.1.0-alpha (prerelease, marked latest, draft promoted). `main` branch pushed.

### CRITICAL DON'T-TOUCH (carries forward into Phase 1+)
- russh `ring` backend — never re-enable `aws-lc-rs` default (NASM blocker)
- npm runner (NOT pnpm) for Tauri build hooks
- `tauri.conf.json` `beforeBuildCommand: "npm run build"` + `beforeDevCommand: "npm run dev"`
- File-format compat w/ WPF Rift's `~/.rift/*.json` — Rust serde must read existing files 1:1
- Velopack `VelopackApp::build().run()` MUST be the first call in `run()`
