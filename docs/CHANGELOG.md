# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.36 + v0.2.37-alpha-test — 2026-05-12 — Local-delete guard + manual push/pull mode

Two ships in one go. Blazzer saw `BLOCKED — 232 deletes in one batch` on the push side fire correctly, then noticed the tombstone-pull path was silently deleting hundreds of files from his local disk with no equivalent guard. That + the auto-sync cascade fatigue was the trigger to add manual mode.

### v0.2.36 — Mass local-delete circuit breaker
Mirrors the existing push-side mass-delete guard. Before drift_watcher's run_tick dispatches any `ToDelete` entries, they're now grouped by resource. Each group is checked against `scaled_delete_threshold(local_root)` — same formula as push side (`(file_count * 0.30).clamp(5, 25)`). If a resource's delete batch ≥ threshold, the entire batch is **blocked**, no files are touched, a single `BLOCKED — N local-deletes in one batch (≥ scaled threshold T of F files)` activity row fires with `kind=block`. Next drift tick re-evaluates; if remote is still missing those files, the guard re-fires but still no deletes. User must take explicit action.

### v0.2.37 — Manual push/pull mode
New `force_push_now` backend command mirrors `force_pull_now` — drains every dirty entry NOW regardless of debounce. SyncModal grew a `mode="push"` rendering with Upload icon, purple accent, `pulse-up` animation, "Pushing to {profile}" title. OpRail gained a third action button (UploadCloud icon) wired to `onPushNow`. New `auto_flush_enabled` atomic flag on AutoSyncEngine — when off, `flush_cycle` + `drift_watcher::run_tick` short-circuit immediately so watcher detection still populates dirty queue but nothing flushes until user clicks. TabRail surfaces a "Mode" toggle (auto | manual) next to the existing Auto-sync toggle. Connection store gained `setAutoFlush`/`refreshAutoFlush` and refreshes on connect.

### Verify
`cargo check`: clean (25.55s). `svelte-check`: 0 errors, 2 pre-existing warnings (section a11y suppressions).

### Recovery for what was already deleted before the guard
FiveM session restore (worked for v0.2.27), fresh clone of qbox/ox_* public repos, or Trey pushes his copy back. All paths viable.

v0.2.35 archived to git log. **NOT YET COMMITTED OR SHIPPED** — Blazzer testing in dev.
