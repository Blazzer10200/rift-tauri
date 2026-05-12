# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.26-alpha-test — 2026-05-12 — Perms-recurrence killer + working Cancel + capped pulls

Trey kept hitting `Permission denied` on every push even after v0.2.25's server-side chgrp/chmod fix, because each new file Blazzer's Rift uploaded came down as `0644` again (umask 0022 on the server). The fix has to live in Rift, not on the server.

### Landed
- **SFTP upload now chmods files to `0664` after rename** — best-effort `set_metadata` call in [sftp/mod.rs](src-tauri/src/sftp/mod.rs) `upload_atomic_via`. Combined with the existing `setgid` bit on parent dirs (`drwxrwsr-x`), every Rift-uploaded file is now group-writable. No more recurring EACCES for teammates in the shared group.
- **Cancel button actually works for Pull Now AND drift_watcher ticks.** `force_pull_now` now registers a `CancellationToken` in the shared `current_scan_cancel` slot, checks it between dispatched pulls, and emits `DriftScanResult { cancelled: true }` on abort. `drift_watcher::run_tick` also registers its token so the 30s+ SFTP listing on slow links can be aborted from the modal. New `pub(crate)` helpers `register_scan_cancel` / `clear_scan_cancel` on `AutoSyncEngine`.
- **Concurrent-pull cap (4 permits via `tokio::sync::Semaphore`)** in `force_pull_now`. N parallel downloads was flooding the SFTP session on Trey's Tailscale link; now max 4 in flight, the rest queue.
- **Modal filters stray `drift_scan_progress` in pull mode** — a parallel `drift_watcher` tick used to leak `scanning [ox] (8/8)` rows into the Pull Now activity feed even though Pull Now doesn't scan. Modal now drops scan progress when `mode === "pull"`.

### Server-side (out-of-band this session)
- `chmod -R g+w` across all `blazzer`-owned files under `/opt/fxserver/.../resources/` to clear the backlog. Future uploads self-chmod via the SFTP fix above.

### Verify
- `cargo check`: clean (1.71s incremental). `svelte-check`: 0 errors, 5 pre-existing a11y warnings.

v0.2.20-v0.2.25 archived to git log.
