# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.47-alpha — 2026-05-12 — New-resource discovery + park-dir ignore (Endure stress test)

### Bug 5 — New resource subdir under a watched bracket not picked up

Stress test 2026-05-12: creating `[endure]/endure_rifttest/` (sibling of `endure_devbridge`) with 7 files across 13 nested dirs produced 0 push entries even after 10 minutes. Bracket-level `[endure]/` was watched recursively, but on Windows `ReadDirectoryChangesW` can race the new-subtree registration — Create events for files nested inside a freshly-created dir before it fully registers get dropped. The Dir-Create event itself reaches us but `queue_path` discarded it (dir events return early, children supposed to fire their own).

Fix in `on_fs_event` ([sync/auto_sync.rs:1041](src-tauri/src/sync/auto_sync.rs#L1041)): on Created+Dir events, fire `kick_drift_reconcile` as the safety net. The scan walks local recursively + lists remote, so the entire new tree surfaces as ToPush regardless of any missed file events. `kick_drift_reconcile` is debounced via its cancel-replace-token semantics, so rapid mkdir bursts collapse to a single scan.

### Bug 6 — Park dirs under `resources/` keep surfacing as ToPull

User parks bracket dupes / freemode cruft into `_disabled_extras/bracket_dupes_<date>/` directly under the watched remote_root during cleanup sessions. The live SFTP recursive listing kept walking into them, drift scanner kept emitting `<file>: local-missing, remote-has` → ToPull. Rescan was working as designed (not a snapshot-staleness bug); ignore rules just didn't cover the prefix.

Fix in `sync/ignore.rs`: new `IGNORE_SEGMENT_PREFIXES = ["_disabled_"]` matched only against non-terminal segments (so legitimate files named `_disabled_for_review.lua` don't trip). Existing exact `_disabled_archive` segment match preserved for stable labeling. New unit test `disabled_prefix_segments` covers four cases incl. the false-trip guard.

### Verify

`cargo check` clean 1.44s · `cargo test --lib sync::ignore` → 11 passed / 0 failed. No API surface change. All v0.2.46 push-reliability fixes (F1 strict mkdir, F1.b strict upload, F2 release-on-every-terminal-path, F3 no-dir-locks, F4 batch pre-mkdir, F7 wait_for_readable exp backoff) preserved.

### Still deferred to v0.2.48

- **Mirror mode for Push-all** (Bug 1) — new drift bucket for `local-missing + remote-has + baseline-exists` → propose remote-delete + Mirror UI toggle. Significant classification + frontend work.
- **Stale-lock sweep UI button** — existing `sweep_stale_mine` covers own-user stale locks on watcher attach.
- **Mass-delete guard fine-tune** — still open from v0.2.45.
