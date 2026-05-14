# Audit Archive — resolved items

Consolidated 2026-05-13 (cleanup/full-audit). Combines two codex fix-pass transcripts. Source files (`codex-fixes-2026-05-11.md`, `codex-fixes-2026-05-12.md`) retired into this archive — full session-by-session view available via `git log -- docs/audit/`.

All items below: verified via `cargo check --manifest-path src-tauri/Cargo.toml` at time of fix. Line numbers reflect pre-cleanup tree.

---

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
| 10 | `sync/auto_sync.rs:46`, `sync/auto_sync/path.rs:1` | Pure path helpers/tests moved to `auto_sync/path.rs`. (`watch.rs`/`flush.rs` were stubbed — completed 2026-05-13, see new archive entry below.) |
| 11 | `sftp/mod.rs:204`, `sftp/ops.rs:62`, `lib.rs:271` | Connect-time write probe under profile `remote_root` before healthy-SFTP signal. |
| 12 | `sync/lock_presence.rs:181`, `sync/auto_sync.rs:441` | Watch attach sweeps stale `.rift-lock` files owned by local user. |
| 13 | `sync/auto_sync.rs:1855` | Permanent autosync failures drop from retry map after final "gave up" activity. |
| 14 | `transport/env.rs:30`, `sync/edit_trail.rs:88` | Temp id entropy 64 bits; trail trim uses `.lines()` (CRLF-safe). |
| 15 | `sftp/mod.rs`, `sftp/remote_exec.rs`, `sync/auto_sync.rs` | Removed dead `ensure_remote_parent_dir`, `get_remote_folder_size`, `resource_name_for`. |

## Cleanup Fix-Pass 2026-05-13 (this branch)

See [HANDOFF.md](../HANDOFF.md) Session 56 for the full list. Highlights:
- `auto_sync.rs` finished split into `auto_sync/watch.rs` + `auto_sync/flush.rs` (codex item 10 deferred work).
- `Releases/` pruned to last 2 versions.
- `components.json` + 13 dead shadcn CSS aliases removed.
- `@vitest/coverage-v8` removed.
- Audit docs consolidated → AUDIT-ARCHIVE.md + AUDIT-OPEN.md.

---

## Skipped at time of fix (now tracked in [AUDIT-OPEN.md](AUDIT-OPEN.md))

- `lib.rs:local_list_dir` profile containment (no server_key input — frontend contract change).
- `scan-lib` log redaction / capability tightening / CSP nonce — needs product decision.
- `scan-sync` safe file-count cache — needs watch-level cache invalidation design.
- `scan-transport` tunnel per-connection cancellation — needs tunnel task ownership refactor.
