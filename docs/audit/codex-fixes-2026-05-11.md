# Codex Backend Fixes - 2026-05-11

| Item | File:line | Change | Verified |
|---:|---|---|---|
| 1 | `src-tauri/src/lib.rs:626`, `src-tauri/src/lib.rs:723`, `src-tauri/src/lib.rs:739`, `src-tauri/src/lib.rs:762`, `src-tauri/src/lib.rs:973` | Applied profile path guards to local/remote list, upload, download, and bootstrap job creation. | `cargo check` |
| 2 | `src-tauri/src/state/sync_snapshot.rs:74`, `src-tauri/src/state/sync_snapshot.rs:107` | Kept snapshot mutex held through serialize + atomic write for `set()`/`forget()`. | `cargo check` |
| 3 | `src-tauri/src/sync/lock_presence.rs:52`, `src-tauri/src/sync/lock_presence.rs:125` | Replaced `DashMap::clear()`/reinsert with rebuild + `RwLock` map swap. | `cargo check` |
| 4 | `src-tauri/src/sync/auto_sync.rs:886`, `src-tauri/src/sync/auto_sync.rs:920` | Lock acquire now spawns only on first dirty entry; delete still releases. | `cargo check` |
| 5 | `src-tauri/src/sync/auto_sync.rs:1192` | Unreadable/vanished modified files now enter failed/backoff instead of endless immediate requeue. | `cargo check` |
| 6 | `src-tauri/src/sync/drift_watcher.rs:312` | ConflictRecord creation now aborts if `remote_stat` failed. | `cargo check` |
| 7 | `src-tauri/src/sync/drift_scanner.rs:312` | First scan w/o baseline now chooses `ToPush`/`ToPull` by mtime instead of `Conflict`. | `cargo check` |
| 8 | `src-tauri/src/transport/ssh_handler.rs:31`, `src-tauri/src/transport/ssh_handler.rs:53` | Fingerprints normalize to bare b64 and compare by exact equality. | `cargo check` |
| 9 | `src-tauri/src/sftp/mod.rs:206` | Worker handshakes collect outside `workers` mutex; append happens once after open. | `cargo check` |
| 10 | `src-tauri/src/sftp/mod.rs:466`, `src-tauri/src/sftp/mod.rs:739` | `shell_quote` rejects NUL/newline/carriage return before SSH exec command construction. | `cargo check` |
| 11 | `src-tauri/src/sftp/mod.rs:359` | Worker list errors no longer insert empty vecs, allowing main-session retry path. | `cargo check` |
| 12 | `src-tauri/src/sync/auto_sync.rs:580`, `src-tauri/src/lib.rs:854` | Manual deletes suppress matching auto-sync uploads for a short window and clear pending dirty/failed entries. | `cargo check` |
| 13 | `src-tauri/src/sync/auto_sync.rs:556`, `src-tauri/src/lib.rs:825`, `src-tauri/src/lib.rs:852` | Rename/delete commands reject paths locked by another user. | `cargo check` |
| 14 | `src-tauri/src/sync/drift_scanner.rs:164` | Remote relative path derivation now uses `strip_prefix` instead of byte slicing. | `cargo check` |
| 15 | `src-tauri/src/path_guard.rs:21` | Documented case-sensitive remote containment assumption. | `cargo check` |
| 16 | `src-tauri/Cargo.toml:25` | Pinned `russh` with `~0.60`. | `cargo check` |

Verification command:

```text
cargo check --manifest-path src-tauri/Cargo.toml
```

Final output:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.41s
```
