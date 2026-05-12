# Codex Backend Fixes - 2026-05-12

| Item | File:line | Change | Verified |
|---:|---|---|---|
| 1 | `src-tauri/src/sftp/ops.rs:85` | `delete_recursive_via` now uses `symlink_metadata` for root + children, rejects empty / `.` / `/`, and never recurses through symlinks. | `cargo check` |
| 2 | `src-tauri/src/sftp/ops.rs:144`, `src-tauri/src/sftp/transfer.rs:260` | Split strict user rename from overwrite tmp-swap: `rename_via` preflights target existence; only atomic upload calls `rename_overwriting_via`. | `cargo check` |
| 3 | `src-tauri/src/lib.rs:1049`, `src-tauri/src/lib.rs:1099`, `src-tauri/src/path_guard.rs:23` | Remote rename/delete/list now use profile remote containment, reject destructive remote root, and preserve foreign-lock blocking. | `cargo check` |
| 4 | `src-tauri/src/lib.rs:467`, `src-tauri/src/lib.rs:1170`, `src-tauri/src/lib.rs:1208` | Local rename/delete now require active watched-root containment, reject target collisions, and return per-path `OpStatus { ok, error }`. | `cargo check` |
| 5 | `src-tauri/src/path_guard.rs:69` | Local guard now checks joined non-existent child parent stays exactly under canonical parent. | `cargo check` |
| 6 | `src-tauri/src/lib.rs:77`, `src-tauri/src/lib.rs:194`, `src-tauri/src/lib.rs:454` | Diagnostics/status paths clone engine before awaiting, avoiding holding the autosync mutex across `.await`. | `cargo check` |
| 7 | `src-tauri/src/lib.rs:801`, `src-tauri/src/lib.rs:1289` | `remote_list_dir` is guarded under `remote_root`; bootstrap downloads now derive local destinations from profile `local_root`. | `cargo check` |
| 8 | `src-tauri/src/lib.rs:497`, `src-tauri/src/lib.rs:514` | `enqueue_for_flush_batch` / conflict resolution reject paths outside watched roots. | `cargo check` |
| 9 | `src-tauri/src/sftp/mod.rs:38`, `src-tauri/src/sftp/list.rs:1`, `src-tauri/src/sftp/transfer.rs:1`, `src-tauri/src/sftp/ops.rs:1`, `src-tauri/src/sftp/remote_exec.rs:1` | Split SFTP implementation into session core, listing, transfer, ops, and remote exec modules w/ public API unchanged. | `cargo check` |
| 10 | `src-tauri/src/sync/auto_sync.rs:46`, `src-tauri/src/sync/auto_sync/path.rs:1` | Moved pure autosync path helpers/tests into `auto_sync/path.rs`; added `watch.rs` / `flush.rs` flagged stubs for deferred split edges. | `cargo check` |
| 11 | `src-tauri/src/sftp/mod.rs:204`, `src-tauri/src/sftp/ops.rs:62`, `src-tauri/src/lib.rs:271` | Added connect-time write probe under profile `remote_root` before surfacing a healthy SFTP connection. | `cargo check` |
| 12 | `src-tauri/src/sync/lock_presence.rs:181`, `src-tauri/src/sync/auto_sync.rs:441` | Watch attach now sweeps stale `.rift-lock` files owned by the local user, reusing lock parsing helpers. | `cargo check` |
| 13 | `src-tauri/src/sync/auto_sync.rs:1855` | Permanent autosync failures are dropped from retry map after final "gave up" activity instead of growing indefinitely. | `cargo check` |
| 14 | `src-tauri/src/transport/env.rs:30`, `src-tauri/src/sync/edit_trail.rs:88` | Increased temp id entropy to 64 bits and made trail trim use `.lines()` for CRLF-safe JSONL handling. | `cargo check` |
| 15 | `src-tauri/src/sftp/mod.rs`, `src-tauri/src/sftp/remote_exec.rs`, `src-tauri/src/sync/auto_sync.rs` | Removed dead public `ensure_remote_parent_dir`, `get_remote_folder_size`, and `resource_name_for` after final reference sweep. | `cargo check` |

## Skipped / flagged

| Item | Reason |
|---|---|
| `src-tauri/src/lib.rs:local_list_dir` profile containment | Command has no `server_key` or active profile input. Changing renderer contract while frontend work is parallel would be risky; left unchanged. |
| `src-tauri/src/sync/auto_sync/watch.rs` split | `try_watch`, notify ingestion, queueing, lock acquisition, and state updates share private engine state. Left a `CODEX-FLAG` stub instead of guessing a broad move. |
| `src-tauri/src/sync/auto_sync/flush.rs` split | `flush_batch` / `process_entry` cross dirty/failed/conflict/cache/bridge/trail state. Left a `CODEX-FLAG` stub instead of doing a high-risk mechanical move. |
| `scan-lib` diagnostics log redaction / capability tightening / CSP / profile JSON depth | Medium/low policy items need product decisions or frontend/capability changes outside this backend-only scope. |
| `scan-sync` safe file-count cache | Requires watch-level cache invalidation design; skipped to avoid stale mass-delete thresholds. |
| `scan-transport` tunnel per-connection cancellation | Requires tunnel task ownership changes across accept/stop; skipped as too broad for this pass. |

Verification command:

```text
cargo check --manifest-path src-tauri/Cargo.toml
```

Final output:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
```
