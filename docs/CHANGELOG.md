# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.41-alpha-test — 2026-05-12 — Quick Actions accuracy + real cancel + background mode

Stop buttons used to lie. `force_push_now` checked cancel once before handing control to `flush_all_now` — after that, in-flight pushes ran to completion (minutes) regardless of clicks. Pull awaited every in-flight handle before emitting the result event, so the modal hung at "Pulling…" until the last download settled. Labels lied too: "Reconcile" only scanned, "Pull/Push all" only drained the drift bucket + dirty queue. `diag_*` command prefix leaked the dev-era origin.

### Backend
- Cancel token plumbed through `flush_all_now` → `flush_batch` (`auto_sync.rs:560,1435`). Dispatch loop checks the token before each entry; on bail, un-dispatched stay in the dirty queue for the next Push to pick up.
- **Lazy-pop from dirty:** entries pop at dispatch time, not up-front. Cancel mid-batch no longer loses queued work. Re-dirty during in-flight upload still creates a new entry (path is already out of the map).
- `force_pull_now` orphans in-flight handles into the background-task tracker on cancel (`auto_sync.rs:1212`). Modal closes within ~1s; the 1–4 active russh streams finish naturally.
- Commands renamed (`lib.rs:127-175`): `diag_force_drift_scan` → `sync_reconcile`, `diag_force_pull_now` → `sync_pull_pending`, `diag_force_push_now` → `sync_push_pending`, `diag_cancel_drift_scan` → `sync_cancel`.

### Frontend
- TabRail labels rewritten to match backend: **Scan drift**, **Pull pending**, **Push pending**. Tooltips spell out scope.
- Tones rebalanced: Reconcile `neutral` (read-only), Pull `info`, Push `accent` (was `warn` — wrong, push isn't a caution op).
- Dropped dead local `pulling/pushing/scanning` flags — they flipped false within ms of click. Buttons gate on `syncModal.busy` (real op lifecycle).
- **Run in background** footer button on SyncModal — dismisses modal, op continues, progress lands in `connection.activityFeed` (Activity tab).
- `SyncModalStore.busy` added; listeners gated on `busy || open`.

### Verify
`svelte-check` 0/0/3994 · `cargo check` clean · `vitest` 6/6.
