# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.49-alpha — 2026-05-12 — Rebaseline UX + listing-accuracy instrumentation

### Bug 5 real fix — Suspicious-shrink rebaseline path

S52's "Created+Dir debounce" closed the wrong layer. Real root cause: `drift_scanner::SuspiciousEmptyAborted` (v0.2.45 data-safety guard) trips on every bracket where the snapshot baseline is much larger than the current remote listing — by design, to prevent phantom mass-deletes after a transient SFTP failure. After Session 7's intentional cleanup (mass-rebuild + bracket-dupe park to `_disabled_extras/`), four [endure]/[ox]/[voice]/[cfx-default] baselines were frozen at 600+ files while real remote shrunk to 56–90. Guard correctly bailed each bracket — but the bail is bracket-wide, so new resources dropped in (e.g., `endure_rifttest`) and unsynced local files (7 .ogg sounds for xsound) were invisible to Rift. Twice diagnosed as a watcher bug in v0.2.47/48; actually a baseline-staleness bug.

Fix: rebaseline UX. Backend `auto_sync::rebaseline_folder` re-lists remote authoritatively, walks local with same ignore rules, re-hashes every local file from disk (no SHA trust — Session 7 included real edits), atomically replaces snapshot rows under the bracket prefix via `sync_snapshot::replace_under`. Both-sides-present → fresh Synced baseline; local-only → row dropped → next scan buckets as ToPush; remote-only → row dropped → next scan buckets as ToPull. Frontend: warn-toned banner per aborted bracket above the Sync totals strip with baseline-vs-listing counts, "Why this matters" tooltip explaining new resources will be invisible until rebaselined, inline confirm card warning about re-hash cost, success banner with delta + ToPush count. Diagnostics emit `BaselineShrinkDetected` (per aborted folder) + `BaselineRebaselined` (post-replace). New Tauri commands `sync_get_aborted_shrunk` + `sync_rebaseline_folder`.

### Listing-accuracy instrumentation

[endure] cross-check on 2026-05-12: real remote `find -type f` returned 61 files; `list_via_exec` emitted 56. Five files vanishing inside the SFTP parse loop. Cause not yet known — instrumentation lands now to name them. `sftp/list.rs::list_via_exec` counts raw_lines vs emitted, buckets skips into `skipped_short` (splitn malformed), `skipped_bad_size` (size parse fail), `skipped_by_ext` (filter mismatch), captures up to 5 sample skipped lines verbatim. Emits `RemoteScanResult` Warn diag + `eprintln!` whenever raw≠emitted. Fix lands next release once the 5 dropped lines are characterized.

### Verify

`cargo check` clean 3.81s · `svelte-check` 0 errors / 0 warnings across 3996 files. v0.2.48 fixes preserved (web/build prune, Created+Dir debounce, push-reliability stack).

### Deferred to v0.2.50

- Listing-accuracy targeted fix (item 2 finish, instrumentation-driven)
- Integration test suite (phase 1: new-resource, single-edit, single-delete, mass-edit, ignore-edges) — gates Mirror mode
- Mirror mode for Push-all (Bug 1)
- Stale-lock sweep UI button, mass-delete guard tune, Terminal Settings, Appearance controls
