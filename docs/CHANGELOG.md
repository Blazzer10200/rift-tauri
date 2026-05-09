# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.1.6-alpha — 2026-05-09 — Backend audit sweep: 85 findings, 6204 LOC touched

Comprehensive backend audit + fix pass across all `src-tauri/src/` (23 files). 85 findings cataloged in [`docs/audit/BACKEND-AUDIT-2026-05-09.md`](audit/BACKEND-AUDIT-2026-05-09.md); critical/high/medium findings landed this version. Dev-only — no public ship. UI untouched (redesign in flight).

### Critical (panic / data-loss paths)
- **Mutex-poison hardening** in `state/sync_snapshot.rs`, `remote_state.rs`, `discovery.rs` — `lock()` helper recovers poisoned mutex via `into_inner()` so a single panic-while-holding can no longer cascade through every state-cache caller.
- **`SaveLocalCopy` data-loss guard** (`sync/auto_sync.rs:534`) — `let _ = std::fs::rename(local_path, &aside)` now bails before downloading remote if the rename fails. Previously the user's local file was silently overwritten.
- **`RiftConfig::save()` made atomic** (`profile/mod.rs`) — `~/.rift/rift.json` now writes via tmp+rename instead of plain `fs::write`. Crash mid-write no longer corrupts the only source of server profiles.
- **Fire-and-forget JoinHandle tracking** (`sync/auto_sync.rs`) — lock acquire/release, bridge ping, edit-trail append now tracked in `background_tasks` Mutex and aborted on `engine.stop()`. Tasks otherwise outlive the engine via `Arc<SftpClient>` keepalive.
- **`stat_local` Option return** — metadata-error path no longer poisons snapshot baselines with `(0, Utc::now())`. Callers skip the write instead.

### High (correctness + hygiene)
- **`get_remote_sha1` stderr capture** — `ChannelMsg::ExtendedData { ext: 1 }` now debug-logged instead of silently dropped, so drift-scan SHA1 misses are diagnosable.
- **Empty-root retry tightened** (`sftp/mod.rs:list_recursive_batch`) — only retries roots the worker pool failed to populate (`HashMap::contains_key`-based), not every empty folder. Cuts N round-trips on resource trees w/ legitimately empty folders.
- **Per-permanent-drop log warn** (`sync/auto_sync.rs:mark_failed`) — files exhausting all 3 retry backoffs now emit `log::warn!` + activity row instead of vanishing silently from the queue.
- **`spawn_blocking` for blocking I/O on tokio runtime**: `safe_count_files` (mass-delete circuit breaker), `walk_local` (drift scanner). `wait_for_readable` migrated to `tokio::fs`.
- **`editor_for` TOCTOU fix** (`lib.rs`) — `EditInPlaceState` lock now held across SFTP open so concurrent `begin_edit_in_place` for the same server can't both open + leak SFTP connections.

### Medium / cleanup
- **SHA1_MAX_BYTES consolidation** — three definitions across `sync_snapshot.rs` (5 MiB, dead), `auto_sync.rs` (64 MiB), `drift_scanner.rs` (64 MiB) collapsed to one canonical `pub const` in `state::sync_snapshot` (64 MiB; matches WPF). Comment corrected.
- **MTIME_TOLERANCE_SECS** exported + reused (was magic `2` in `drift_scanner.rs`).
- **Shared `transport` module additions**: `ssh_handler.rs` (`PinningHandler` + `pubkey_fingerprint` shared by `sftp` + `tunnel`, ~50L de-duped); `env.rs` (`current_user`, `hostname`, `short_id` — was 3 hostname() impls + 3 home_dir() impls + nanosecond temp-dir collisions).
- **`paths::dirs_home` made public** — `edit/in_place.rs` + `transport/ssh_keygen.rs` now use the canonical resolver.
- **`local_fs::list_directory` becomes canonical walker** — `lib.rs:local_list_dir` adapts to the legacy `{path, mtime: unix-secs}` shape via field map; frontend type unchanged.
- **`temp dir` collision fix** — `lock_presence`/`edit_trail` `read_raw` use `pid + short_id` instead of `nanosecond timestamp`. Resolves race when two `poll_once` invocations fire in the same nanosecond.
- **`ensure_workers` short-circuit** (`sftp/mod.rs`) — `FuturesUnordered` replaces `join_all` so first ready connection becomes available without waiting for stragglers.
- **Private key loaded once** in `OwnedConnectArgs` — workers reuse the parsed key via `Arc` (saves 3 disk-reads + parses per pool spin-up).
- **Tokio features narrowed** from `["full"]` to `["macros", "rt-multi-thread", "sync", "time", "net", "io-util"]` — drops unused `process`, `signal`, `fs`, `tracing`, etc.
- **Dead deps removed**: `anyhow`, `thiserror`, `notify-debouncer-full`, `async-trait` — all declared in `Cargo.toml`, zero usages in source.
- **`classify_action` case unification** (`auto_sync.rs`) — all matchers operate on the lowercased copy; "blocked" lowercase no longer falls through to Error.
- **`hex_upper` write! optimization** (`sync_snapshot.rs`) — `write!(s, "{:02X}", b)` instead of `s.push_str(&format!(...))` per byte (~20× cheaper for SHA1).
- **`ignore::classify` needle pre-lower** + lowercase-segments invariant test.
- **`edit_trail::trim_to_tail`** preserves trailing newline so JSONL stays well-formed.
- **`bootstrap_list_files` UTF-8 boundary safety** — slice via lowercased prefix length.
- **`check_for_updates` async + spawn_blocking** — Velopack's blocking network probe no longer parks the runtime.

### Doc passes
- Inline notes added: `LockPresence::poll_once` advisory window, `RiftConfig::bridge_token` plaintext caveat, `shell_quote` NUL/newline limitation, `BAD_REMOTE_ROOT_BRACKETED_RATIO` Qbox-tuning context, `ResourceDiscoveryCache` Phase-6 reservation, `AutoSyncEngine::stop` event-task abort rationale, autosync 850ms debounce+tick worst-case latency, `SyncSnapshot::save` last-write-wins semantics, `editor_for` lock-held-across-init rationale, `_unused_export_warning_silencer` test removed (no longer needed).

### Verified
- `cargo clippy --lib --tests -- -D warnings` ✓ zero warnings
- `cargo test --lib` ✓ 47/47 pass + 1 new (`segments_are_lowercase`) = 48/48
- `npm run check` ✓ 318 files / 0 errors / 0 warnings

### Versions bumped
`Cargo.toml`, `package.json`, `tauri.conf.json` → 0.1.6. v0.1.5 entry archived.
