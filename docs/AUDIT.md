# Rift — Audit Log

Single source of truth for code audit findings. Open items at top, archive below. Line numbers are pre-cleanup (2026-05-11 tree) unless noted — re-verify against current HEAD before fixing.

Originally split across `audit/AUDIT-OPEN.md` + `audit/AUDIT-ARCHIVE.md`; merged 2026-05-15. Pre-merge history: `git log -- docs/audit/`.

---

# Open Findings

Consolidated 2026-05-13 from `scan-frontend-2026-05-11.md`, `scan-lib-2026-05-11.md`, `scan-sync-2026-05-11.md`, `scan-transport-2026-05-11.md`. Items resolved by codex fix-passes have been moved to the Archive section below.

## Frontend (`scan-frontend-2026-05-11`)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| HIGH | `RemotePane.svelte:44-47` | `$effect`'s async `void load()` swallows rejection + stale-closure race overwrites newer entries. | Destroyed flag in effect cleanup; propagate rejection. |
| HIGH | `LocalPane.svelte:44-47` | Same pattern as `RemotePane`. | Same fix. |
| HIGH | `AppShell.svelte:95` | `addEventListener` in `onMount` leaks on HMR/remount. | Convert to `$effect` w/ cleanup. |
| HIGH | `Diagnostics.svelte:38-49` | Synchronous `clientHeight` read inside `$effect` before DOM settles. | Drive `viewport` from `ResizeObserver` only. |
| HIGH | `connection.svelte.ts:249-321` | `wireEvents()` failure has no UI surface or retry. | Surface failure + retry trigger. |
| MED | `RemotePane.svelte:89-95` | Conflict detection matches basename → cross-dir false positives. | Use `c.remote_path === e.full_path`. |
| MED | `LocalPane.svelte:92-99` | Same basename-only match. | Use full-path equality w/ OS normalization. |
| MED | `TwoPane.svelte:41-46` | `toastTimer` not cleared in `onDestroy`. | `onDestroy` clearTimeout. |
| MED | `AppShell.svelte:196-204` | TOFU `.then()` may execute after destroy. | Capture `alive` flag, guard the handler. |
| MED | `diagnostics.svelte.ts:174-181` | `generateReport()` busy-polls 50ms × 2s. | One-shot listener filtered to `drift_scan_result`. |
| MED | `AddServer.svelte:148-151` | Triple `as unknown as` cast hides `ServerProfile` drift. | Extend `ServerProfile` type w/ optional fields. |
| MED | `AppShell.svelte:62-85` | `commands` array recreates each tick, churning `CommandPalette` props. | Split static/dynamic; `$derived.by()` memoize. |
| LOW | `updates.svelte.ts:20` | `catch {}` silently swallows `app_version` invoke failure. | `console.warn(...)`. |
| LOW | `ConflictResolver.svelte:18-23` | `$effect` reset is redundant w/ `{#key}` remount. | Drop the effect. |
| LOW | `AddServer.svelte:59-68` | Async IIFE in `$effect` writes after potential destroy. | Cancel flag in cleanup. |

(`DriftReview.svelte` items from this scan are obsolete — file removed in `79f6fae` UI consolidation.)

## Backend — lib / config / capabilities (`scan-lib-2026-05-11`)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| LOW | `lib.rs:diag_state_pump:176-222`, `diag_get_state:76-108` | `AutoSyncState` lock held across `status().await`. | Clone `Arc` under lock, drop, then await. |
| MED | `lib.rs:editor_for:1034-1057` | Double-init race silently drops first `Arc<EditInPlaceManager>`. | `tokio::sync::OnceCell` per server key OR `warn!` on collision. |
| MED | `diagnostics/mod.rs:LogForwarder:277` | Forwards every log msg to frontend incl. error bodies w/ potential key paths. | Audit `log::error!/warn!` callers; add `RUST_LOG_DIAG_SCRUB` env flag. |
| MED | `diagnostics/mod.rs:DiagEvent.file:84-93` | Absolute paths can be sent verbatim to renderer. | Relativize to watch root or basename-only. |
| LOW | `diagnostics/mod.rs:DiagBus:95-176` | `last_rescan_signal_at` / `last_drift_scan_at` in `std::sync::Mutex`, hot path, panic-poison silent. | `AtomicU64` epoch-ms. |
| LOW | `profile/mod.rs:bridge_token:15-39` | Plaintext in `~/.rift/rift.json`. | Phase-6 tracking only — Stronghold/DPAPI/keyring. |
| LOW | `profile/mod.rs:RiftConfig::load:55-59` | Unbounded `extra` flatten depth → stack overflow on crafted config. | Depth-limit `serde_json::Deserializer` or size guard. |
| LOW | `state/paths.rs:atomic_write_json:41-79` | `thread::sleep` retry loop on async cmd thread blocks Tokio worker. | `spawn_blocking` or async sleep + async save. |
| LOW | `state/paths.rs:safe_profile_key:29-33` | Sanitized empty key risk. | Log/assert if empty. |
| LOW | `Cargo.toml:reqwest+ureq` | Two HTTP stacks. | Defer — `velopack` 0.0.1298 `UpdateSource` is sync (ureq); reqwest is async elsewhere. Revisit when velopack ships async source. |
| LOW | `Cargo.toml:velopack="=0.0.1298"` | 0.0.x semver = API instability; exact pin blocks security patches silently. | Comment intent + quarterly review note. |
| LOW | `tauri.conf.json:csp` | `style-src 'self' 'unsafe-inline'`. | Nonce/strict-dynamic once Tailwind supports hashed styles. |
| LOW | `capabilities/default.json:7` | `core:default` broad superset. | Pin specific `core:*` perms in use. |
| LOW | `capabilities/default.json:12` | `opener:default` unscoped. | Scope to known prefixes (update URL, docs URL). |
| LOW | `path_guard.rs:78-91` | Non-existent path joins raw filename; NUL/overlong edge case. | Post-join `canon.parent() == canon_parent`. |
| LOW | `path_guard.rs:55,20-60,20` | Remote validation: `root_norm == "/"`, case-sensitivity, per-segment backslash. | Reject `/` root; document Linux-only; future per-segment check. |

## Backend — sync (`scan-sync-2026-05-11`)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| MED | `auto_sync.rs:953` | `flush_batch` awaits `safe_count_files` inline every delete-cycle. | Per-watch cached count, refresh on add/remove. |
| MED | `auto_sync.rs:1302` | `mark_failed` inserts after "giving up" log → unbounded `failed` map growth. | Return without insert post-giveup. |
| MED | `edit_trail.rs:read_raw:75` | Subdir w/ PID+short_id race can cross-delete concurrent reads. | `tempfile::NamedTempFile` direct in temp dir. |
| MED | `drift_watcher.rs:56` | After pushing-skip, full `secs` re-sleep doubles pull latency. | Short retry sleep on skip. |
| MED | `ignore.rs:163` | `_disabled_archive` falls through to `"seg:?"` in match. | Add explicit arm. |
| LOW | `auto_sync.rs:try_watch:444` | Probe path uses `MAIN_SEPARATOR` (backslash on Win) — inconsistent w/ rest of codebase. | Build via `.to_string_lossy().replace('\\','/')`. |
| LOW | `auto_sync.rs:is_dir:818` | Symlinked dirs skipped silently. | Debug log the skip. |
| LOW | `sync_snapshot.rs:compute_sha1:105` | Reads whole file (≤64 MiB) into heap per concurrent call. | Stream via BufReader 8 KiB. |
| LOW | `lock_presence.rs:stale-delete:197` | Stale-lock delete failures silent → 10s retry loop indefinite. | Warn log + backoff counter. |

## Backend — transport/sftp/tunnel/edit/update (`scan-transport-2026-05-11`)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| MED | `sftp/mod.rs:delete_recursive_via:880` | Relies on `russh-sftp` `is_symlink()` method whose presence is version-dependent. | Match `FileType::Symlink` variant explicitly + compile-time assertion. |
| MED | `tunnel/mod.rs:per-conn-tasks:117` | Per-conn `tokio::spawn` outlives `SshTunnel::stop()`. | `CancellationToken` per conn; cancel in `stop()`. |
| MED | `edit/in_place.rs:begin_edit:100` | `remote_path` not validated against profile `remote_root`. | Apply containment guard in `begin_edit` + `save`. |
| LOW | `sftp/mod.rs:close:247` | `workers` lock held across each `sftp.close()` await. | Clone Arcs, drop lock, close outside. |
| LOW | `update_service.rs:RIFT_UPDATE_FEED:110` | Env-var local FileSource bypass not gated by build profile. | `#[cfg(debug_assertions)]` or signed marker file. |
| LOW | `edit/in_place.rs:Drop:287` | Synchronous `fs::remove_dir_all` in async drop context. | `spawn_blocking` or explicit `async close_all()`. |
| LOW | `transport/ssh_keygen.rs:generate:75` | Private key file inherits umask on POSIX. | `set_permissions(0o600)` on `#[cfg(unix)]`. |
| LOW | `edit/in_place.rs:short_id:126` | 4-byte short_id collision risk at scale. | Bump to 8 bytes or `Uuid::new_v4()`. |
| INFO | `bridge/mod.rs:57` | Token in plaintext over loopback HTTP. | Accepted; documented. |
| INFO | `transport/env.rs:hostname:17` | Spawns external `hostname` binary on non-Windows; ambient PATH risk. | Document. |

## Originally-listed but out of scope at filing time (still open)

- `lib.rs:local_list_dir` profile containment — frontend contract change deferred.
- Safe file-count caching — needs watch-level cache invalidation design.
- Tunnel per-conn cancellation — already filed above (`tunnel/mod.rs:117`).

---

# Archive — Resolved

All items below: verified via `cargo check --manifest-path src-tauri/Cargo.toml` at time of fix. Line numbers reflect pre-cleanup tree.

## Codex Fix-Pass 2026-05-11 (16 items)

| # | File:line | Change |
|---:|---|---|
| 1 | `lib.rs:626/723/739/762/973` | Profile path guards on local/remote list, upload, download, bootstrap jobs. |
| 2 | `state/sync_snapshot.rs:74,107` | Snapshot mutex held through serialize + atomic write for `set()`/`forget()`. |
| 3 | `sync/lock_presence.rs:52,125` | DashMap clear/reinsert → rebuild + RwLock map swap. |
| 4 | `sync/auto_sync.rs:886,920` | Lock acquire spawns only on first dirty entry. |
| 5 | `sync/auto_sync.rs:1192` | Unreadable/vanished modified files enter failed/backoff (not endless requeue). |
| 6 | `sync/drift_watcher.rs:312` | ConflictRecord creation aborts if `remote_stat` failed. |
| 7 | `sync/drift_scanner.rs:312` | First scan w/o baseline picks ToPush/ToPull by mtime instead of Conflict. |
| 8 | `transport/ssh_handler.rs:31,53` | Fingerprints normalized to bare b64, compared by exact equality. |
| 9 | `sftp/mod.rs:206` | Worker handshakes collect outside `workers` mutex; one append after open. |
| 10 | `sftp/mod.rs:466,739` | `shell_quote` rejects NUL/newline/CR before SSH exec. |
| 11 | `sftp/mod.rs:359` | Worker list errors no longer insert empty vecs (main-session retry path). |
| 12 | `sync/auto_sync.rs:580`, `lib.rs:854` | Manual deletes suppress matching auto-sync uploads + clear pending dirty/failed. |
| 13 | `sync/auto_sync.rs:556`, `lib.rs:825,852` | Rename/delete commands reject paths locked by another user. |
| 14 | `sync/drift_scanner.rs:164` | Remote relative path derivation uses `strip_prefix` (no byte slicing). |
| 15 | `path_guard.rs:21` | Documented case-sensitive remote containment assumption. |
| 16 | `Cargo.toml:25` | `russh = "~0.60"` pin. |

## Codex Fix-Pass 2026-05-12 (15 items)

| # | File:line | Change |
|---:|---|---|
| 1 | `sftp/ops.rs:85` | `delete_recursive_via` uses `symlink_metadata` for root + children; rejects empty/`.`/`/`; never recurses symlinks. |
| 2 | `sftp/ops.rs:144`, `sftp/transfer.rs:260` | Split strict user rename from overwrite tmp-swap: `rename_via` preflights target existence; only atomic upload calls `rename_overwriting_via`. |
| 3 | `lib.rs:1049,1099`, `path_guard.rs:23` | Remote rename/delete/list now use profile remote containment + reject destructive remote root. |
| 4 | `lib.rs:467,1170,1208` | Local rename/delete require active watched-root containment + return per-path `OpStatus { ok, error }`. |
| 5 | `path_guard.rs:69` | Local guard for non-existent paths asserts joined child parent equals canonical parent. |
| 6 | `lib.rs:77,194,454` | Diagnostics/status paths clone engine before awaiting (no mutex across await). |
| 7 | `lib.rs:801,1289` | `remote_list_dir` guarded under `remote_root`; bootstrap downloads use profile `local_root`. |
| 8 | `lib.rs:497,514` | `enqueue_for_flush_batch` / conflict resolution reject paths outside watched roots. |
| 9 | `sftp/mod.rs:38`, `sftp/list.rs:1`, `sftp/transfer.rs:1`, `sftp/ops.rs:1`, `sftp/remote_exec.rs:1` | Split SFTP impl into session core / listing / transfer / ops / remote-exec modules. Public API unchanged. |
| 10 | `sync/auto_sync.rs:46`, `sync/auto_sync/path.rs:1` | Pure path helpers/tests moved to `auto_sync/path.rs`. (`watch.rs`/`flush.rs` were stubbed — completed 2026-05-13.) |
| 11 | `sftp/mod.rs:204`, `sftp/ops.rs:62`, `lib.rs:271` | Connect-time write probe under profile `remote_root` before healthy-SFTP signal. |
| 12 | `sync/lock_presence.rs:181`, `sync/auto_sync.rs:441` | Watch attach sweeps stale `.rift-lock` files owned by local user. |
| 13 | `sync/auto_sync.rs:1855` | Permanent autosync failures drop from retry map after final "gave up" activity. |
| 14 | `transport/env.rs:30`, `sync/edit_trail.rs:88` | Temp id entropy 64 bits; trail trim uses `.lines()` (CRLF-safe). |
| 15 | `sftp/mod.rs`, `sftp/remote_exec.rs`, `sync/auto_sync.rs` | Removed dead `ensure_remote_parent_dir`, `get_remote_folder_size`, `resource_name_for`. |

## Cleanup Fix-Pass 2026-05-13

See `HANDOFF.md` Session 56 (via `git log -- docs/HANDOFF.md`) for the full list. Highlights:
- `auto_sync.rs` finished split into `auto_sync/watch.rs` + `auto_sync/flush.rs` (codex item 10 deferred work).
- `Releases/` pruned to last 2 versions.
- `components.json` + 13 dead shadcn CSS aliases removed.
- `@vitest/coverage-v8` removed.
- Audit docs consolidated.

## Skipped at time of fix (now tracked in Open Findings above)

- `lib.rs:local_list_dir` profile containment (no server_key input — frontend contract change).
- `scan-lib` log redaction / capability tightening / CSP nonce — needs product decision.
- `scan-sync` safe file-count cache — needs watch-level cache invalidation design.
- `scan-transport` tunnel per-connection cancellation — needs tunnel task ownership refactor.
