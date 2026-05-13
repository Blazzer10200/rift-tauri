# Audit Open Items — outstanding findings

Consolidated 2026-05-13 (cleanup/full-audit) from `scan-frontend-2026-05-11.md`, `scan-lib-2026-05-11.md`, `scan-sync-2026-05-11.md`, `scan-transport-2026-05-11.md`. Line numbers are pre-cleanup (2026-05-11 tree). Re-verify before fixing.

Items resolved by 2026-05-11 or 2026-05-12 codex fix passes have been moved into [AUDIT-ARCHIVE.md](AUDIT-ARCHIVE.md).

---

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
