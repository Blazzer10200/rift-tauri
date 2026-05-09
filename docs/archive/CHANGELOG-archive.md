# rift-tauri — Changelog Archive

Older changelog entries flow here as new versions ship. Live entry stays in `docs/CHANGELOG.md`.

## v0.2.5-alpha — 2026-05-09 — Agent-driven sweep: M13 cancel-tokens + Vitest + russh 0.60 + L9 partial

End-to-end campaign using the new Claude Code agent roster (recon/scout/architect/operator/verifier). Five Phase-0 reconnaissance agents mapped IPC contract + test coverage + russh upstream + rustls feature flags + Velopack signing. Three Phase-1 architects designed the cancel-token API + russh migration + Vitest harness. Phase-2 operators implemented in parallel.

### Landed

- **M13** Bootstrap mid-chunk cancel — `tokio_util::sync::CancellationToken` threaded through `download_paths` → `download_files_batch` → `download_atomic_via`. `tokio::select!` races the inner `sftp.read` future, aborting mid-chunk without rewriting to a chunk loop. New `cancel_download` Tauri cmd; `connection.cancelDownload()` exposed. UI abort wiring deferred.
- **L4** russh 0.54 → 0.60 — closes future-incompat warning **AND** patches DoS vuln (GHSA-f5v4-2wr6-hqmg, scout-discovered, bigger than the audit flagged). Required: `rand` 0.8 → 0.10 (russh 0.60 dropped rand_core 0.6 path); two source-file fixes (`ssh_keygen.rs`, `transport/env.rs`) for the `OsRng → rand::rng()` API change.
- **Vitest harness from zero** — frontend test infra scaffolded: `vitest.config.ts`, `__mocks__/@tauri-apps/api/`, 11 passing seed tests across `utils.test.ts` + `connection.test.ts`. Backend already has 49 inline `#[test]` fns across 13 modules.
- **H4** Velopack signing doc TODO enriched at `update_service.rs:75` — `signtool` + GH-secrets P12-base64 pattern + AAS upgrade path embedded.
- **L9** partial — direct `rustls` w/ `default-features=false, features=["ring",...]` forces ring at our crate level, but `aws-lc-rs` still pulled transitively by `hyper-rustls`/`tokio-rustls`/`ureq`/`rustls-platform-verifier`. Full drop needs `cargo deny` or `[patch.crates-io]`.

### Deferred

- **M6** POSIX perms / **L8** hostname POSIX — latent (Windows-only app)
- **L14** ConflictResolver — interactive runtime behavior test, not static

### Verify

- `cargo check`: clean
- `npm run check`: 0 errors, 1 pre-existing warning
- `npm run test`: 11/11 pass

v0.2.4-alpha archived.

## v0.2.4-alpha — 2026-05-09 — Audit fix-pass round 3: 4 more findings + verified no-ops

Continuation autopilot of the 2026-05-09 audit sweep while user AFK. Round 1+2 closed 42 findings. Round 3 closes 4 more and verifies 3 already-correct items, leaving 6 genuinely deferred (POSIX-port / upstream / scoped-session work).

### Closed
- **M4** `atomic_write_json` hardened on Windows — `sync_all()` before rename for crash-durability + 5-attempt retry on `MoveFileExW` sharing violations + tmp cleanup.
- **M16** drag-drop payload `catch {}` upgraded to `console.warn` in `LocalPane.svelte` and `RemotePane.svelte`.
- **L11** `connection.disconnect()` doc comment — explains tauri event listeners are deliberately preserved across disconnect/reconnect.
- **L2** Dead utility types removed from `$lib/utils.ts`.

### Verified no-op
- **M10** `selectedConflict` prune already at `AppShell.svelte:46-51`.
- **L7** Tauri capabilities already minimal.
- **L10** Ctrl+P wired at `AppShell.svelte:106`.

### Deferred (carried into v0.2.5-alpha)
- **M6** POSIX file perms — latent. **M13** Bootstrap mid-chunk cancel — needs cancellation token. **L4** russh future-incompat — upstream. **L8** hostname POSIX — latent. **L9** dual-crypto stack — needs feature-flag work. **L14** ConflictResolver semantics — behavioral verification only.

### Note
Source bumped to 0.2.4-alpha but **no installer built** — user steered to dev-server-default workflow. v0.2.3-alpha archived.

## v0.2.3-alpha — 2026-05-09 — Audit fix-pass round 2: 14 more findings landed

Continuation of the 2026-05-09 audit sweep. Round 1 (v0.2.2-alpha) closed 28 findings; round 2 closes another 14 across the medium and low tiers, plus a defense-in-depth path-traversal extension. 18 audit findings remain (mostly LOW/style).

### High-impact
- **H2** `refreshStatus()` no longer swallows IPC errors silently — `console.error` surfaces channel-closed / serialization failures.
- **H9** `askConfirm()` Promise leak fix — pending confirm resolvers tracked in a Set; `onDestroy` resolves them all to `false` so awaiting code paths unblock if the shell unmounts mid-dialog.

### Medium
- **M2** `reject_path_traversal` guard extended to `local_list_dir`, `enqueue_for_flush_batch`, and `resolve_conflict`. Defense-in-depth — any path with `..` components from JS is rejected at the boundary.
- **M5** `state/discovery.rs` and `state/remote_state.rs` mutex-poison recovery now logs an `error!` before falling through. The original poisoning panic is no longer hidden.
- **M9** `loadServers()` fetches `list_servers` and `get_last_selected` in a single `Promise.all` round so `selected` doesn't briefly resolve to `null` between awaits.
- **M12** `ConflictResolver` "Edit in editor" switched from raw `invoke("plugin:opener|open_path")` to typed `import { openPath } from "@tauri-apps/plugin-opener"`. Failures now surface in the dialog instead of being swallowed.
- **M18** `TopBar.connCfg` wrapped in `$derived.by(...)` so detail strings re-evaluate when fingerprint / status.detail change. Was a script-eval-time snapshot before.
- **M21** `DriftReview` `selected` is pruned via `$effect` whenever the `selectableIds` derived list changes (sideFilter / grouping / filtered shifts). `applyPushPull` no longer operates on entries the user can't see.

### Low
- **L3** `Cargo.toml` velopack pin tightened from `"0.0"` to `"=0.0.1298"`. 0.0.x crate families have shipped breaking changes between patches.
- **L9** Verified via `cargo tree -d`: ring is the canonical crypto stack, but rustls-webpki transitively pulls aws-lc-rs through `rustls-platform-verifier`. **Documented for next round** — needs a feature-flag investigation to fully de-dupe; not a blocker.
- **L12** `browser-tabs.svelte.ts` `savePersisted` empty catch upgraded to `console.warn`.
- **L13** `Reupload.svelte` Enter-key always picks `"reupload"` (one-time) regardless of the "Always" checkbox state. Persistent default now requires an explicit button click — prevents accidental pin while typing.

### Verify
- `cargo check --lib` — clean, only the known russh future-incompat note.
- `cargo clippy --lib --tests` — clean.
- `npm run check` — 3933 / 0 errors / 1 advisory (intentional Settings:18).

## v0.2.2-alpha — 2026-05-09 — Debug-sweep fix pass: 28 audit findings landed

Audit-driven fix sweep — 28 of 60 findings actioned (all 5 CRITICALs).

**Critical:** C1 CSP set; C2 TOFU prompt (probe + Confirm + set_server_fingerprint); C3 bootstrap_list_files non-ASCII panic; C4 wireEvents race + try/catch rollback; C5 EditChangedEvent.serverKey filter.

**High:** H1 openBootstrap guard; H3+H12 editor_for narrowed lock; H4 Velopack pinning TODO docs; H5 default_ssh_key_path command; H6 stale-phase toasts; H7 lockForBasename → remoteForLocalPath; H11 reject_path_traversal; H13 toast handle tracking; H15 stepper bypass; H16 ssh-preview NaN.

**Medium / cosmetic:** M3/M7/M8/M11/M14/M19 + L1/L5/L15/L16/M15.

**Verify:** cargo check + clippy clean; npm run check 3933 / 0 errors / 1 advisory.

v0.2.1-alpha archived.

## v0.2.1-alpha — 2026-05-09 — Refresh build + SSH commit signing

Maintenance bump w/ no code changes. Triggered by signing-pipeline configuration + first end-to-end verified build of the post-S13 codebase.

### Repo / signing
- SSH commit signing live globally — `gpg.format=ssh`, `user.signingkey=~/.ssh/id_ed25519.pub`, `commit.gpgsign=true`, `tag.gpgsign=true`. Verified by GitHub on push (`reason: valid`, green Verified badge). `~/.config/git/allowed_signers` configured for local `git log --show-signature` verification.
- Sessions 11+12+13 commit (`6e9d5f1`) landed on `origin/main` — pre-signing config, stays unverified by design (no force-push).

### Build pipeline
- Bumped `Cargo.toml`, `package.json`, `tauri.conf.json` → 0.2.1-alpha.
- Fresh NSIS installer at `src-tauri/target/release/bundle/nsis/Rift_0.2.1-alpha_x64-setup.exe`.
- Silent install + desktop shortcut refresh + icon cache bust + explorer restart.

v0.2.0-alpha entry archived.

## v0.2.0-alpha — 2026-05-09 — Linear-precise UI port: phases 0–11 complete

Full Claude Design "Rift App UI" deliverable ported into the codebase. **All 12 phases done in one session arc** (0–3 from S12, 4–11 this session). Backend untouched. Dev-only — public ship still gated on user say-so.

### Phase 4 — Activity feed
Segmented filter w/ live count pips per kind, semantic kind badges (RefreshCw/Download/Trash2/AlertTriangle/etc via lucide w/ ok/warn/danger/info backgrounds), time gutter, pause/resume w/ "N new since pause" banner, clear button. `connection.clearActivity()` added.

### Phase 5 — Drift tab
Sub-toolbar w/ side filter (All / Local wins / Remote wins, count pips) + grouping (Dir / Flat) + Auto-resolve safe button. Sticky bulk-action bar appears only on selection (Apply combined push/pull). Dir-grouped expandable rows w/ chevron, side pill, inline size stat, expandable peek (paths, sizes, mtimes, snapshot, conflict→Conflicts-tab note). Conflicts shown un-selectable w/ red bg.

### Phase 6 — Dialogs
Shared `.dialog-overlay/.dialog-shell/.dialog-head/.dialog-body/.dialog-foot/.stepper/.step` primitives in `app.css`. All 5 dialogs refactored to tokens + lucide icons: **Confirm** (variant icon, Don't ask), **Reupload** (now auto-pops from `dirtyEdits` queue — skip/re-upload/always w/ per-server localStorage pref), **Keygen** (key blob + fingerprint preview + copy), **Bootstrap** (variant icons per detection state, top-level folder chips, progress bar), **AddServer** (3-step stepper Connection→Workspace→Bridge w/ live ssh preview + summary card).

### Phase 7 — Command palette
Fuzzy scoring (prefix > substring > group/subtitle), group column, kbd shortcut chips. Auto-scroll selected into view. Command registry in AppShell now tagged w/ `group` field.

### Phase 8 — Conflicts
ConflictList w/ count pip + active-row red border. ConflictResolver: click-to-pick side meta cards (info-bordered local / warn-bordered remote), diff peek (paths, size delta, last-known sync), action toolbar (Skip / Save copy + pull / Apply primary). Per-hunk merge deferred to backend ticket per S12 decision.

### Phase 9 — Polish
StatusHero: data-variant border tones, 24px LED w/ pulse-soft, lucide icons in card labels. ActivityToast: lucide icons, semantic borders, click-to-dismiss. Density persistence via new `state/ui-prefs.svelte.ts` (localStorage `rift.ui.density.v1`, init in `+layout.svelte` mount). Reduced-motion already gated.

### Phase 10 — Verify
- `npm run check` — 3933 files, 0 errors, 1 advisory (intentional Settings:18)
- `cargo clippy --lib --tests` — clean
- `cargo test --lib` — 47 passed, 2 ignored

### Tauri 2 fix
`core:window:allow-{start-dragging,minimize,toggle-maximize,close}` added to `default.json` capability — `data-tauri-drag-region` was silently no-op'ing.

### Versions
`Cargo.toml`, `package.json`, `tauri.conf.json` → 0.2.0-alpha. v0.1.6-alpha archived.

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
