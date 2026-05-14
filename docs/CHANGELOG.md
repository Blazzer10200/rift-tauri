# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.54-alpha — 2026-05-13 — Fresh-install bootstrap + titlebar dropdown hotfix

Two onboarding bugs surfaced while bringing a second dev (Trey) on board for the first time. Both block the empty-local → populated-remote first sync path.

### Fresh-install bootstrap (Bug 1)

`auto_sync::try_watch` was silently returning `Ok(false)` when a folder's local subdir didn't exist on disk, leaving the engine with `watches = 0` for a brand-new install. The drift scanner only iterates registered folders, so Rescan returned zero entries → Sync page rendered "Everything in sync" → no way for the user to pull the remote tree down without finding the hidden `Ctrl+K → Bootstrap from remote…` dialog. Now: when the profile's `local_root` exists but a per-folder subdir doesn't, `try_watch` `mkdir_all`s the subdir, logs `"auto-created local folder for first-time bootstrap"` to diagnostics, and attaches the watcher normally. If the profile `local_root` itself is missing (genuine typo / config error) we still bail with `Ok(false)` + a clearer log, never silently mkdir somewhere unexpected. After this, a fresh install with empty local just works: connect → 8 bracket dirs auto-create → drift scan finds remote-only files → Sync page shows ToPull entries → Pull all streams the tree down.

### Titlebar server dropdown clipped (Bug 2)

The titlebar's server-picker dropdown menu opens below the 44 px titlebar row. Its parent `.left` flex container had `overflow: hidden` to constrain text overflow into the drag region, which also clipped the menu vertically — z-index can't escape an overflow-clip ancestor. Moved the overflow constraint from the parent down to the child spans (`.svr-name` / `.svr-host` now use `white-space: nowrap; text-overflow: ellipsis; overflow: hidden`) so long server/host text still truncates cleanly, but the dropdown can render outside the titlebar height. Defensive `z-index: 100 → 1000` on `.svr-menu` too.

### Verify

`cargo check` clean · `cargo test --lib` 46 passed · `svelte-check` 0 errors / 0 warnings across 3996 files.

## v0.2.53-alpha — 2026-05-13 — Mirror mode + auto-reconnect

The two queued safety nets land in one release. Mirror mode gives Rescan a recovery path when watcher events get missed (rare, but happens — e.g. notify-rs Windows issue #403 silently dropping events on a watched-dir delete). Auto-reconnect closes the loop on v0.2.50's `ConnectionWedged` detection: instead of just emitting a diag event and waiting for the user to click Sweep + manually reconnect, the frontend now self-heals after 3+ wedges in a 60 s window.

### Mirror mode (Bug 1)

New `DriftBucket::ToDeleteRemote` variant. When the drift scanner runs with `mirror = true` and sees `l.is_none() && r.is_some() && snap.is_some()`, it now buckets as ToDeleteRemote ("local deleted — removing remote") instead of ToPull ("remote-only — pull"). Normal mode keeps treating this case as ToPull (the safer non-destructive direction). The flag is session-scoped on the engine (`mirror_mode: AtomicBool`) and exposed via two new Tauri commands, `sync_set_mirror_mode(enabled)` and `sync_get_mirror_mode()`. Dispatch lives in `auto_sync::apply_selected`, which routes ToDeleteRemote entries to `sftp.delete(remote_path)` — the SftpClient::delete router already handles dirs through `delete_recursive_via` and files through `remove_file`, so folder deletes propagate cleanly. The mass-delete circuit breaker is intentionally skipped for ToDeleteRemote because the user reached dispatch through the typed-confirm modal — that gate is the consent.

Frontend: a "Mirror" toggle next to Rescan/Sweep on the Sync page (red accent when enabled). Toggling triggers an immediate Rescan so the bucket counts redraw. When entries are in the ToDeleteRemote bucket, a red "Apply Mirror (N)" button appears. Clicking opens a hard-gate modal: count of files to delete, warning copy about irreversibility and multi-user baseline coordination, and a typed-confirm input requiring the literal text "MIRROR" before the Confirm button enables. Backdrop click and Escape both cancel. Backend session-scoped means the toggle resets to off on engine restart — paranoia against accidental destructive ops on a fresh launch.

### Auto-reconnect (v0.2.50 follow-through)

`connection.svelte.ts` now listens to `diag://event` for `stage === "connection_wedged"` emits (these come from `sftp/transfer.rs::with_t` when an SFTP op blows the timeout). A rolling 60 s window holds the timestamps; once 3+ wedges land inside the window, the frontend calls `stop_autosync`, sleeps 1 s for clean teardown, then calls `startAutosyncForSelected()` to re-open the session with the same server + folder spec. A `reconnecting` guard prevents overlapping reconnects. Single wedges still don't reconnect — those usually self-resolve on the next op and aren't worth the session churn. Lives entirely client-side so we don't have to refactor the engine's owned `SftpSession` (which isn't behind a RwLock).

### Verify

`cargo check` clean 5.00 s · `cargo test --lib` 46 passed · `svelte-check` 0 errors / 0 warnings across 3996 files.

### Deferred to v0.2.54

- Integration test suite phase 1 — 10 mock-SFTP scenarios (clean reconcile, local-add, local-delete Normal + Mirror, remote-add, conflict, SuspiciousEmptyAborted, dry-run Mirror, Mirror-disabled-when-shrunk). Requires either an SftpClient trait abstraction for mocking or a testcontainers-based real SFTP server in CI — its own evening.
- Dry-run Mirror preview UI (current modal goes straight to confirm; a "preview rows" pre-confirm step would let the user spot-check before typing MIRROR).
