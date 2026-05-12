# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.33-alpha-test — 2026-05-12 — Deletion propagation (tombstone semantics via baseline snapshot)

Blazzer deleted `gt_zombies_qb` locally. Remote delete propagated correctly. Trey refreshed — file still on his side. Root cause: drift scanner had no tombstone path. When local has a file, remote doesn't, AND baseline snapshot says it WAS previously synced — that's a delete to PULL, not a "remote vanished, re-push" panic. Pre-v0.2.33 logic classified this as `ToPush` with reason `"remote vanished — re-pushing local"`, which meant the file lingered indefinitely on teammates' machines and risked accidental resurrection on the next touch (autosync watcher would re-upload, undoing the original delete).

### Landed
- **`DriftBucket::ToDelete` variant** ([drift_scanner.rs](src-tauri/src/sync/drift_scanner.rs)). When `local exists + remote missing + has_baseline` → classify as `ToDelete` with reason `"remote deleted — removing local"`. Without baseline (genuinely new local file) → still `ToPush` as before.
- **`delete_local_one` handler** ([drift_watcher.rs](src-tauri/src/sync/drift_watcher.rs)). Foreign-lock → defer. Dirty local (unflushed edit) → skip + warn (never blow away user's work). Otherwise `fs::remove_file` → `snapshot.forget` + `cache.forget` + best-effort empty-parent-dir cleanup walking up until non-empty. Mirrors `pull_one`'s safety guards. Emits `"deleted local (remote removed)"` activity row w/ `ActivityKind::Delete`.
- **Wired into both auto + manual paths**: `drift_watcher::run_tick` dispatches `ToDelete` alongside `ToPull`. `auto_sync::force_pull_now` routes by bucket (ToDelete → `delete_local_one`, else `pull_one`) sharing the 4-permit semaphore.
- **SyncModal** gained a "To Delete" count cell (grid 3→4 cols); `to_delete` field surfaced on `drift_scan_result` payloads.

### Convergence (what user does)
- Both relaunch Rift to pick up v0.2.33. Trey's next scan tick (≤10s after launch) auto-deletes `gt_zombies_qb` locally. No manual action.

### Verify
- `cargo check`: clean (8.33s). `svelte-check`: 0 errors, 2 warnings (pre-existing svelte-ignore quirk).

v0.2.32 archived to git log.
