# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.21-alpha-test — 2026-05-12 — Snappier auto-pull + Pull Now button

Closes the "buddy pushed but my Rift hasn't ticked yet" UX gap. Tested by Blazzer + Trey: push direction was instant, pull direction needed a manual Sync click to feel timely. Two changes fix it.

### Landed
- **Faster auto-pull cycle** — `DEFAULT_SCAN_INTERVAL_SECS` 30 → 10. Drift watcher now polls every 10s instead of 30s, so buddy-side pushes appear within ~10s of upload (was up to 30s). 3x more SFTP listings, ~2s each on a typical tree — negligible. Users who want the old behavior can still set their own interval via Settings.
- **Pull Now button** — appears in the SyncModal footer when a completed scan reports `To Pull > 0`. New `diag_force_pull_now` Tauri cmd calls `AutoSyncEngine::force_pull_now()`, which re-runs the drift scan AND dispatches `pull_one` for every ToPull entry (vs. plain Sync, which only auto-enqueues ToPush). Modal re-enters scanning phase, activity feed populates with `RemotePullStart/Done` events, completion shows `pull_dispatched` count.
- **Modal listing-phase hint** — "Listing remote files… (this may take a moment on the first scan)" status line + activity entry on `drift_scan_start`. Closes the silent ~30s pre-listing window where the modal looked frozen.
- **Pull-button styling** — new `.btn-accent` variant in the modal matches the existing UI language (soft accent fill, accent border, hover swell).

### Verify
- `cargo check`: clean. `svelte-check`: 0 errors, 5 a11y warnings (all pre-existing).

v0.2.20-alpha-test archived to git log.
