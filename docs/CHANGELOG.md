# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.19-alpha-test — 2026-05-11 — Security hardening + reconcile UX + lock heartbeat

Rebases the S22–S25 local batch onto v0.2.18. S22 rename/delete shipped as v0.2.13 upstream; S24's `just_pulled_suppress_until` echo fix dropped in favor of v0.2.18's `recently_written` (same idea, theirs landed first). Net-new in v0.2.19: S23 audit hardening, S24 reconcile UX + orphan-tmp self-heal, S25 lock heartbeat.

### Landed
- **Stale-lock heartbeat** (S25) — own `.rift-lock` files re-stamp `since` every 60s while held. Closes the bug where a long edit (>180s, the foreign-sweep threshold) would get its own lock swept by another Rift. New `last_heartbeat: DashMap<String, Instant>`; `acquire()` records, `release()` clears, poll-driven `refresh_my_locks()` re-uploads body if ≥60s elapsed. Foreign-sweep math unchanged — crash recovery still ~3 min.
- **Reconcile button rework** (S24) — middle-rail Edit/Diff/Delete buttons (which lied) removed. Sync button fires `diag_force_drift_scan` and surfaces a live progress chip (bottom-right, matches ActivityToast styling) with per-folder progress + variant-coded result. Drift-discovered `ToPush` items auto-enqueue into the flush queue.
- **Orphan tmp self-heal** (S24) — atomic SFTP upload best-effort removes any pre-existing `<file>.rift-tmp` before creating its own. Recovers from crashed-mid-upload orphans (incl. foreign-owned tmps when the parent dir is group-writable).
- **Unified toast vocabulary** (S24) — TwoPane + DriftReview inline toasts use a shared `FlashToast` component matching the ActivityToast chip language.
- **Listing guards relaxed** (S24) — Codex's S23 guards on `local_list_dir` / `remote_list_dir` were over-applied; hybrid policy says listing is free. Reverted to OS-level access only.
- **Path-containment guards** (S23, `path_guard.rs`) — all destructive Tauri cmds reject `..`/empty/root/escape paths. Hybrid policy: browser may navigate anywhere, destructive ops gated to `profile.remote_root`/`local_root`.
- **Strict rename** (S23) — `rename_via` errors on collision; separate `rename_overwriting_via` for internal atomic upload tmp-swap. No more silent data loss.
- **SFTP delete symlink-safe** (S23) — uses `symlink_metadata` per node, unlinks symlinks instead of traversing.
- **TOFU fingerprint** (S23) — substring → exact-normalized match. Accepts OpenSSH, WinSCP, legacy WPF formats.
- **Worker pool** (S23) — parallel SSH handshakes happen outside the workers mutex; lock taken once for final append.
- **Lock-by-other reject** (S23) — manual rename/delete refuses paths held by another user.
- **Manual-delete suppress window** (S23) — auto_sync ignores re-upload events for 2s after manual delete; prevents resurrect.
- **Sync snapshot** (S23) — mutex held through serialize+write; concurrent `set()` race fixed.
- **Lock-presence map** (S23) — `RwLock` swap replaces `clear()`+reinsert gap.
- **Frontend** (S23) — stale-invoke race in panes (token guard), HMR-safe global keydown, wire-error banner + retry, tunnel CancellationToken cleanup, structured `OpStatus[]` delete results, conflict-badge exact-path match, busy-poll → one-shot listener.
- **Code-signing TODO permanently dropped** (`update_service.rs`).

### Verify
- `cargo check`: clean. `svelte-check`: 0 errors.
- See `docs/audit/codex-fixes-2026-05-11.md` + spot-check transcript.

v0.2.18-alpha-test archived to git log.
