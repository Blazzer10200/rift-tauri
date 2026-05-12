# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.43-alpha-test — 2026-05-12 — Push parity + stale-cache fix + rate-limit bypass for critical events

Three real root causes for the "modal hangs at 0 PUSHED" complaint, all confirmed in live dev-server tracing.

**Root cause 1 — Push couldn't see scan results.** The `Push pending` button drained the watcher dirty queue only. Files that existed locally but never went through a watcher event (pre-existing drift) lived only in the scan cache's `ToPush` bucket and were invisible to push. New helper `promote_scan_pushes_to_dirty()` converts ToPush drift entries into dirty queue entries before flush, restoring symmetry with `force_pull_now`'s scan-cache dispatch. Adds an inline scan fallback when both dirty queue AND cache are empty so cold-session Push pending no longer dispatches zero against a server full of pending files.

**Root cause 2 — Stale scan cache re-pushing same files forever.** After a successful push the cache still held the same ToPush entries, so re-clicking Push promoted+re-pushed them indefinitely. The phantom-conflict SHA-collapse hid the no-op uploads but the count lied. `force_push_now` now clears `last_scan_entries` after a non-cancelled push; same logic added to `force_pull_now` for symmetry. Next click triggers an auto-scan, gets fresh state.

**Root cause 3 — DriftScanResult event silently rate-limited.** The diagnostics bus capped frontend emits at 200/sec to absorb FsEvent bursts. A 192-file push generated 800+ events in seconds, overflowing the cap; the terminal `drift_scan_result` event got dropped and the modal hung at "Pushing pending local edits…" forever waiting for it. Critical lifecycle stages (`DriftScanStart`, `DriftScanResult`, `RescanSignal`, `SftpConnect/Disconnect`, `RemoteScanResult`, `BridgeAck`, `System`) now bypass the rate limit unconditionally.

### Other fixes
- Activity-feed orphan rows: every early-return path in `process_entry` (cancel-at-top, file-vanished, file-unreadable, phantom-conflict collapse, outer-cancel-select) now emits an activity row so the feed never shows an "uploading…" without follow-up.
- `eprintln!` breadcrumbs added to `sync_push_pending` / `sync_pull_pending` / `sync_reconcile` Tauri commands plus `force_push_now` / `force_pull_now` task bodies for fast diagnosis in dev console.

### Verify
Live-dev tested: edited `[depend]/oxmysql/fxmanifest.lua`, clicked Push pending, observed `dispatched=2 elapsed_ms=253` and modal correctly transitioned to "Push complete · 2 pushed". Cache cleared post-push as expected.
