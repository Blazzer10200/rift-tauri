# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.38-alpha-test — 2026-05-12 — Auto-sync ripped, manual-only

v0.2.37's mode toggle wasn't enough. With auto on by default, Blazzer kept seeing `pulled → removed locally → pulled → removed locally` ping-pong cycles on `[world]` resources at 10s tick cadence — drift_watcher classifying entries as ToDelete on one tick and ToPull on the next while the snapshot baseline + bridge devbridge regen raced. The fix is to delete the auto path entirely. Push/Pull buttons only.

### Backend
- `drift_watcher::spawn` + `run_tick` deleted — no more 10s remote-poll loop.
- `flush_cycle` deleted — no more debounced auto-flush. `flush_all_now` now folds in the failed→dirty backoff-promotion that lived in the killed loop, so transient SFTP failures still retry when the user clicks Push again.
- `auto_flush_enabled` flag deleted — no longer needed; the loops it gated are gone.
- `remote_scan_interval_secs` + `set_remote_scan_interval` + `get_remote_scan_interval` deleted — no interval to configure.
- Mass local-delete circuit breaker moved into `force_pull_now`. Same formula (`(file_count * 0.30).clamp(5, 25)`), same single `BLOCKED — N local-deletes (≥ scaled threshold T of F files)` row, same `kind=block`. Pull is the only path that can propagate tombstones now, so the guard moved with it.
- `kick_drift_reconcile` (called by notify's `need_rescan` signal + Diagnostics > Force Drift Scan) no longer auto-enqueues the discovered ToPush set. It just refreshes the cached scan result; user must click Push to act on it.
- Conflicts from a Pull Now scan are now surfaced via `register_conflict` (was only wired from the killed run_tick before).
- Removed: `track_pull_handle`, `register_scan_cancel`, `clear_scan_cancel`, `drift_watcher_task` + `flush_task` fields, `LOOP_TICK_MS` constant.

### Frontend
- TabRail "Mode" toggle gone. "Auto-sync on/off" relabeled to "Watcher on/off" (it was always toggling the connection, not an auto-sync flag).
- `connection.autoFlush` + `setAutoFlush` + `refreshAutoFlush` removed.
- Diagnostics "Pull every Xs" selector + `getRemoteScanInterval` / `setRemoteScanInterval` removed.

### Behavior
Watcher still runs locally to populate the dirty queue (so user sees pending count in StatusBar). Push Now drains the queue. Pull Now does an inline drift scan + dispatches with the guard in front. Nothing happens in the background. No more ping-pongs.

### Verify
`cargo check`: clean (1.55s). `svelte-check`: 0 errors, 2 pre-existing a11y warnings.

**NOT YET COMMITTED OR SHIPPED** — Blazzer testing in dev. v0.2.36 + v0.2.37 archived to git log.
