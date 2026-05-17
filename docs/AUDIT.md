# Rift — Audit Log

Single source of truth for code audit findings. Open items at top, archive below. Line numbers re-verified 2026-05-17 against HEAD (v0.4.1-alpha tree). Re-verify before fixing if HEAD has moved.

Originally split across `audit/AUDIT-OPEN.md` + `audit/AUDIT-ARCHIVE.md`; merged 2026-05-15. Verification pass 2026-05-17 moved 16 items to Archive (silently fixed by v0.3/v0.4 refactors) + downgraded 3 to PARTIAL. S81 fix-pass closed 6 (B2/B4/B5/B9 + S6 + T4). S82 fix-pass closed 6 more (B7 + B11 + S4 + T1 + T2 + T3) + downgraded B16 to INFO. Total Open: 42 → **11 actionable**.

---

# Open Findings

## Frontend (3 items)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| MED-PARTIAL | `src/lib/components/browser/RemotePane.svelte:50-53` | `$effect` async `void load()` uses `loadToken` guard (race fixed) but no destroy-flag in cleanup; rejection caught into `error` state only. | Destroyed flag in effect cleanup; propagate fatal rejection. |
| MED-PARTIAL | `src/lib/components/browser/LocalPane.svelte:50-53` | Same pattern as `RemotePane` (loadToken landed; cleanup-cancel didn't). | Same fix. |

(F3-F15 verified resolved 2026-05-17 — moved to Archive.)

## Backend — lib / config / capabilities (6 items)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| MED | `src-tauri/src/diagnostics/mod.rs:277` `LogForwarder` | Forwards every log msg to frontend incl. error bodies w/ potential key paths. | Audit `log::error!/warn!` callers; add `RUST_LOG_DIAG_SCRUB` env flag. |
| LOW | `src-tauri/src/profile/mod.rs:31-36` `bridge_token` | Plaintext in `~/.rift/rift.json`. Comment acknowledges. | Phase-6 tracking only — Stronghold/DPAPI/keyring. |
| LOW | `src-tauri/src/state/paths.rs:68` `atomic_write_json` | `std::thread::sleep` retry loop on async cmd thread blocks Tokio worker. | `spawn_blocking` or async sleep + async save. |
| LOW | `src-tauri/Cargo.toml:41-44` reqwest+ureq | Two HTTP stacks. | Defer — `velopack` 0.0.1298 `UpdateSource` is sync (ureq); reqwest is async elsewhere. Revisit when velopack ships async source. |
| LOW | `src-tauri/tauri.conf.json:24` csp | `style-src 'self' 'unsafe-inline'`. | Nonce/strict-dynamic once Tailwind supports hashed styles. |
| LOW | `src-tauri/capabilities/default.json:7` | `core:default` broad superset. | Pin specific `core:*` perms in use. |
| LOW | `src-tauri/capabilities/default.json:12` | `opener:default` unscoped. | Scope to known prefixes (update URL, docs URL). |
| INFO | `src-tauri/src/path_guard.rs:21` Linux-only containment | Containment is case-sensitive and assumes Linux remote (Samba/macOS unguarded). Documented inline. Accepted: Rift's deploy target IS Linux. | None — file in incident if a Samba/macOS user reports drift. |

(B1 + B15 fixed by 2026-05-17 verification pass; B2 + B4 + B5 + B9 closed by S81; B7 + B11 closed + B16 → INFO by S82 — all in Archive.)

## Backend — sync (2 items)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| MED | `src-tauri/src/sync/auto_sync/flush.rs:37` | `flush_batch` awaits `safe_count_files` inline every delete-cycle. | Per-watch cached count, refresh on add/remove. |
| LOW | `src-tauri/src/state/sync_snapshot.rs:141` `compute_sha1` | Reads whole file (≤64 MiB) into heap per concurrent call. | Stream via BufReader 8 KiB. |
| INFO-MOOT | `src-tauri/src/sync/edit_trail.rs:75-80` `read_raw` | Subdir w/ PID+`short_id` race could cross-delete concurrent reads. `short_id` widened to 8 bytes 2026-05-12; PID+rand collision now astronomical. Keep tracked but no action needed. | Optional `NamedTempFile` migration. |

(S2 fixed by 2026-05-17 verification pass; S6 closed by S81; S4 closed by S82 — all in Archive.)

## Backend — transport/sftp/tunnel/edit/update (0 actionable, 2 INFO accepted)

| Sev | File:line | Issue | Fix |
|---|---|---|---|
| INFO | `src-tauri/src/bridge/mod.rs:57` | Token in plaintext over loopback HTTP. | Accepted; documented. |
| INFO | `src-tauri/src/transport/env.rs:16-24` `hostname` | Spawns external `hostname` binary on non-Windows; ambient PATH risk. | Document. |

(T4 closed by S81; T1 + T2 + T3 closed by S82 — all in Archive. Section is functionally clean.)

## Originally-listed but out of scope at filing time (still open)

- `lib.rs:local_list_dir` profile containment — frontend contract change deferred.
- Safe file-count caching — needs watch-level cache invalidation design.

---

# Archive — Resolved

All items below: verified via `cargo check --manifest-path src-tauri/Cargo.toml` at time of fix (Rust) or `npm run check` (frontend). Line numbers reflect pre-cleanup tree.

## S82 Fix-Pass 2026-05-17 (6 items + 1 downgrade)

Second backend batch — Cargo.toml doc + 1 sync + 3 transport + 1 lib/config + 1 INFO downgrade. `cargo check` clean (3.71s after one corrective edit on T1).

| # | File:line | Change |
|---:|---|---|
| B7 | `profile/mod.rs:54-64 RiftConfig::load` | Added `RIFT_CONFIG_MAX_BYTES = 1 MiB` const + pre-parse `std::fs::metadata` size check. Rejects crafted oversize configs before `serde_json` can stack-overflow on nested `extra` flatten. |
| B11 | `Cargo.toml:40-44 velopack` | Doc comment added: "EXACT pin (=0.0.1298) is intentional, 0.0.x semver = breaking-change noise. Last reviewed 2026-05-17; revisit quarterly." Tracks audit B11 explicitly. |
| S4 | `sync/auto_sync/watch.rs:39-45, 65-69 try_watch` | `local_root` build no longer uses `MAIN_SEPARATOR_STR` (would produce backslash-paths on Win); `probe` for `ignore::should_ignore` now forward-slash normalized. Aligns with `auto_sync/path.rs` conventions. |
| T1 | `sftp/mod.rs:280-293 close` | `workers` lock snapshot now clones `Arc<Worker>` list under lock, drops lock, then awaits each `sftp.close()` outside. Slow close on one worker no longer blocks observers on the same lock. |
| T2 | `update_service.rs:108-122 resolve_manager` | `RIFT_UPDATE_FEED` env-var → local FileSource branch wrapped in `#[cfg(debug_assertions)]`. Release builds physically cannot be tricked into an attacker-controlled local update feed. |
| T3 | `edit/in_place.rs:301-313 Drop` | `std::fs::remove_dir_all` detached via `std::thread::spawn`. Slow FS (AV scanner / locked handles) can't block the async runtime worker executing Drop. |
| B16 | `path_guard.rs:21` | Downgraded LOW-PARTIAL → INFO. Inline doc already states the Linux-only assumption; Rift's deploy target IS Linux remotes (FiveM/RedM servers). Filed as "incident on Samba/macOS report" rather than open work. |

## S81 Fix-Pass 2026-05-17 (6 items)

Targeted batch — 3 MED + 3 LOW. `cargo check` clean (5.51s, 0 errors).

| # | File:line | Change |
|---:|---|---|
| B2 | `lib.rs:1576-1602` `editor_for` | Replaced `or_insert` (which silently dropped the loser Arc) w/ explicit `get`-then-`insert` + `log::warn!` on race-loss. Drop is now visible in logs/diag instead of silent. |
| B4 | `diagnostics/mod.rs:basename_only` + `publish:148` | Added `basename_only()` helper; applied at `publish()` chokepoint so `DiagEvent.file` is reduced to trailing basename before broadcast. Renderer no longer sees absolute paths. |
| B5 | `diagnostics/mod.rs:110-113, 128-129, 153-160, 195-211` `DiagBus` | Converted `last_rescan_signal_at` + `last_drift_scan_at` from `std::sync::Mutex<Option<DateTime<Utc>>>` to `AtomicI64` (epoch-ms, `i64::MIN` sentinel for None). Hot-path is now lock-free; panic-poison risk gone. |
| B9 | `state/paths.rs:29-41` `safe_profile_key` | Empty-sanitized key now emits `log::warn!` and returns `"_empty"` sentinel instead of empty string. Prevents downstream cache-path filename collisions. |
| S6 | `sync/lock_presence.rs:50-71, 213-244` | Added `STALE_DELETE_MAX_FAILS=3` const + per-path `DashMap<String, u8>` `stale_delete_fails`. Sweep skips paths that hit cap; success resets counter. Eliminates indefinite warn-log noise on permanently-unreachable locks. |
| T4 | `transport/ssh_keygen.rs:51-92` `generate` | `#[cfg(unix)]` block sets `0o600` perms on private key immediately after write. Doc comment updated to drop the "caller is responsible" line. Windows path unchanged. |

## Verification Pass 2026-05-17 (16 items resolved by v0.3/v0.4/v0.4.1 refactors)

Re-verified against HEAD after 40 commits landed between audit-refresh (2026-05-16) and v0.4.1-alpha ship (2026-05-17). Files moved into subdirs during reorg; lines mostly shifted; underlying issues independently fixed.

| # | Original anchor | Current state |
|---:|---|---|
| F3 | `AppShell.svelte:184` addEventListener leak | ✅ — `onMount` uses `win.onResized` w/ cleanup callback at L198-201; no raw addEventListener |
| F4 | `Diagnostics.svelte:38-49` sync `clientHeight` | ✅ — `diagnostics/Diagnostics.svelte:37` drives `viewport = el.clientHeight` inside ResizeObserver only |
| F5 | `connection.svelte.ts:249-321` `wireEvents()` no UI surface | ✅ — `wireError` state captured; AppShell `onMount` catches w/ "banner will offer retry" + `retryWire()` at L210 |
| F6 | `RemotePane.svelte:89-95` basename conflict match | ✅ — `browser/RemotePane.svelte:105` uses `c.remote_path === e.full_path` |
| F7 | `LocalPane.svelte:92-99` basename conflict match | ✅ — `browser/LocalPane.svelte:107-111` uses full-path equality w/ `\\` → `/` normalization |
| F8 | `TwoPane.svelte:41-46` `toastTimer` leak | ✅ — `browser/TwoPane.svelte:161` clears in `onDestroy` |
| F9 | `AppShell.svelte:196-204` TOFU `.then()` after destroy | ✅ — `alive` flag at L199-200 gates the handler |
| F10 | `diagnostics.svelte.ts:174-181` 50ms busy-poll | ✅ — `state/diagnostics.svelte.ts:174-188` one-shot listener filtered to `drift_scan_result` |
| F11 | `AddServer.svelte:148-151` triple `as unknown as` cast | ✅ — `dialogs/AddServer.svelte:138-149` clean `ServerProfile` literal |
| F12 | `AppShell.svelte:62-85` commands array churn | ✅ — `AppShell.svelte:116-164` split into `sharedCommands` / `v02Commands` / `v03Commands` `$derived` |
| F13 | `updates.svelte.ts:20` silent `catch {}` | ✅ — `state/updates.svelte.ts:22-24` `console.warn("app_version invoke failed", e)` |
| F14 | `ConflictResolver.svelte:18-23` redundant `$effect` | ✅ — `conflicts/ConflictResolver.svelte` no `$effect` block in current file |
| F15 | `AddServer.svelte:59-68` async IIFE | ✅ — `dialogs/AddServer.svelte:60-72` `cancelled` flag + cleanup return |
| B1 | `lib.rs:diag_state_pump:313` + `diag_get_state:78` lock across await | ✅ — both sites use `let engine = { state.0.lock().await.clone() };` clone-and-drop (L81 + L324) |
| B15 | `path_guard.rs:78-91` non-existent path NUL/overlong edge | ✅ — `path_guard.rs:141` `joined.parent() != Some(canon_parent.as_path())` assertion landed |
| S2 | `sync/auto_sync.rs::mark_failed` unbounded `failed` map | ✅ — `sync/auto_sync/flush.rs:602-605` explicit `self.failed.remove()` on permanent giveup |

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

## Resolved between 2026-05-11 and 2026-05-16 (verified by audit pass)

Six prior Open items confirmed fixed in HEAD code by the 2026-05-16 audit. Moved here from Open Findings.

| # | Original anchor | Current state |
|---:|---|---|
| 1 | `tunnel/mod.rs:117` per-conn `tokio::spawn` outlives `SshTunnel::stop()` (MED) | ✅ Fixed — `SshTunnel.conn_cancel: CancellationToken` at `tunnel/mod.rs:55`, cancelled in `stop()` at L185 and `Drop` at L198. Per-conn `copy_bidirectional` races against `conn_cancel.cancelled()` via `tokio::select!`. |
| 2 | `sftp/mod.rs:delete_recursive_via:880` relies on version-dependent `is_symlink()` method (MED) | ✅ Fixed — sftp module split v0.2.49 moved this to `sftp/ops.rs:98-154` (`delete_recursive_via`). New impl uses `sftp.symlink_metadata()` + `ft.is_symlink()` + `ft.is_dir() && !ft.is_symlink()` chain (russh-sftp 2.1 stable surface). |
| 3 | `edit/in_place.rs:begin_edit:100` `remote_path` not validated against profile `remote_root` (MED) | ✅ Fixed — `guard_remote_path()` helper at `edit/in_place.rs:84-90` loads fresh profile, runs `path_guard::validate_remote_child`. Called from `begin_edit` (L116) AND `save` (L240). |
| 4 | `edit/in_place.rs:short_id:126` 4-byte collision risk (LOW) | ✅ Fixed — `short_id()` now lives at `transport/env.rs:30-34` and produces 16-hex (8 bytes) via `rand::fill(&mut buf)`. Old 4-byte impl is gone. |
| 5 | `drift_watcher.rs:56` after-pushing skip → full `secs` re-sleep doubles pull latency (MED) | ✅ Moot — v0.2.38 removed the auto-poll loop entirely (`drift_watcher::spawn` / `run_tick` / `flush_cycle` all gone). Module is now pure helpers (`pull_one`, `delete_local_one`, `register_conflict`) — no sleep loop to double. |
| 6 | `ignore.rs:163` `_disabled_archive` falls through to `"seg:?"` (MED) | ✅ Fixed — explicit arm at `sync/ignore.rs:226`: `"_disabled_archive" => "seg:_disabled_archive"`. Stable label, not the catch-all. |

## Skipped at time of fix (now tracked in Open Findings above)

- `lib.rs:local_list_dir` profile containment (no server_key input — frontend contract change).
- `scan-lib` log redaction / capability tightening / CSP nonce — needs product decision.
- `scan-sync` safe file-count cache — needs watch-level cache invalidation design.
- `scan-transport` tunnel per-connection cancellation — completed 2026-05-16 (see "Resolved between 2026-05-11 and 2026-05-16" above).
