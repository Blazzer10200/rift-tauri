# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.46-alpha — 2026-05-12 — Push reliability + orphan-lock fix (Endure RP report)

### Bug 2/4 — Silent file drops + new-directory failure

Cross-session Endure RP report confirmed Push uploading some files but silently dropping others when a fresh remote bracket directory was involved (`[depend]/oxmysql/` — all ~50 files missing) or when individual files raced into a new dir (`ox_doorlock/fxmanifest.lua`, `ox_fuel/{config,client/init,data/stations}.lua`, `qbx_seatbelt/data/seatbelt_sounds.dat54.rel`). Root cause: per-file mkdir was best-effort `let _ = create_dir(...)` and N parallel workers raced the same fresh parent tree.

Two-layer fix:
- **`mkdir_p_strict_via`** (`sftp/ops.rs`) — each `create_dir` failure is verified against a metadata probe. "Already exists as dir" stays idempotent; real failures (perm denied, ENOSPC, parent missing, transient SFTP) now propagate as `OpResult::err("mkdir parent X: ...")` from `upload_atomic_via`.
- **Batch pre-mkdir** (`sync/auto_sync.rs::flush_batch`) — collects unique parents from all entries and serializes the mkdir tree on the main session BEFORE the parallel upload loop dispatches. Eliminates the 50-worker race on fresh trees. Per-file strict mkdir is the inner safety net.

### Bug 3 — Orphan `.rift-lock` files (44 cleaned manually)

Three distinct leak sources collapsed to one root cause: `release()` only fired on the success branch of `process_entry_body`. Any Fail/Conflict/read-error path left the `<file>.rift-lock` orphaned. Three sources:
- **Directory-level locks** (`[depend]/oxmysql.rift-lock`) — `notify` fires Modified events on directory paths when child files change on Windows; lock acquired, upload failed (read-dir-as-file), no release. Fix: `queue_path` now gates lock-acquire on `path.is_file()`.
- **Editor tmp locks** (`<file>.tmp.<pid>.<hash>.rift-lock`) — same release-on-success-only flaw.
- **Stream-file locks** (~29 across `[world]/pillbox/stream/`, `[world]/postapo-interior/stream/`) — same.

Fix in `process_entry` wrapper: every terminal result (Ok OR Fail) releases the lock. Only Requeued preserves it (entry still pending). Release is idempotent so the inner success-path release stays a no-op.

### `wait_for_readable` — 200 ms → 3.2 s exp backoff

Mass-push 48-error pattern from v0.2.45 traced to `wait_for_readable` allowing only 200 ms total (4 × 50 ms) for files in editor atomic-save windows. Bumped to 6 attempts × 50/100/200/400/800/1600 ms exp backoff. Matches Windows Defender real-time scan + editor save windows.

### Deferred to v0.2.47

- **Mirror mode for Push-all** — new drift bucket needed for `local-missing + remote-has + baseline-exists` → propose remote-delete. Significant classification + UI work.
- **Stale-lock sweep button** — existing `sweep_stale_mine` (180 s STALE_SEC) already catches own-user stale locks on watcher attach.

### Verify

`cargo check` clean · no API surface change · all fixes behind existing code paths.
