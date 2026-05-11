# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.18-alpha-test — 2026-05-11 — Pull→push loop fix + bulk conflict resolve

Treyday's first-night session surfaced a real bug behind the conflict storm: every successful pull triggered Delete+Modify fs-events on the destination path (Windows atomic-replace semantics), which Rift queued as outbound uploads. Result: pull → upload → pull → upload, files looping forever with mtimes drifting a few seconds each cycle. The `.rift-tmp` ignore rule only covered the temp sidecar, not the real path's events. Also adding bulk conflict resolution so when drift baselines start dirty, you don't click 2000+ times.

### Landed

- `AutoSyncEngine.recently_written: DashMap<PathBuf, Instant>` w/ `mark_recently_written` + `is_recently_written` (5s window, lazy eviction).
- All 3 atomic-pull sites (`drift_watcher::pull_one_remote`, `auto_sync` conflict accept-remote, `auto_sync` conflict save-copy) now bracket the `download_file_atomic` call with `mark_recently_written` before + after.
- `queue_path` checks `is_recently_written` before the `ignore::classify` step; matching events surface in diagnostics as `[recent-pull]` so the suppression is observable.
- New `resolve_conflicts_bulk` Tauri cmd — takes a path list + ConflictResolution, iterates the engine resolver, emits one ActivityRow per attempt w/ the real reason on failure.
- ConflictList sidebar gets "Use Remote for all" + "Use Local for all" buttons w/ confirm dialog + partial-failure banner.

### Verify

- `cargo check --release`: clean (3.67s incremental).
- `svelte-check`: 0 errors, 5 pre-existing a11y warnings unrelated.

v0.2.17-alpha-test archived to git log.
