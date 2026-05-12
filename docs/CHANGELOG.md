# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.20-alpha-test — 2026-05-12 — Sync modal + scan cancel + cleanup

Center-stage sync experience replaces the corner chip. Backend now supports mid-scan cancel; frontend trusts progress events instead of a hardcoded timer. v0.2.19's "Scan timed out (30s)" false alarm (backend kept running after the toast fired) is fixed.

### Landed
- **SyncModal** — full-overlay modal triggered by the Sync button. Shows live progress bar, per-folder status, push/pull/conflict counts, scrolling activity feed (drift + queue events). Backdrop blur disables other interaction during the scan. Esc cancels (during scan) or dismisses (after).
- **Scan cancel** — `DriftScanner::scan_with_cancel` checks a `CancellationToken` between folders; new `diag_cancel_drift_scan` Tauri cmd fires it. Partial results (entries collected before bail) are returned w/ `cancelled: true` — no auto-enqueue of the partial push set when the user just asked to stop.
- **Watchdog, not timeout** — modal trusts progress events. 30s of silence shows a "scan may be slow" banner but does NOT auto-fail. Scales naturally with reconcile size.
- **Path-guards re-applied** — `upload_paths` / `download_paths` now gate every job's local + remote target through `path_guard::validate_local_child` / `validate_remote_child` against the active server profile. Closes the v0.2.19 gap from the rebase.
- **Open in default editor** — LocalPane context menu gains "Open in default editor" for files. Uses `@tauri-apps/plugin-opener` directly (already in capabilities). Closes the v0.2.19 UX gap where the chip dropped the Edit button without a replacement.
- **Dead code removal** — `just_pulled_suppress_until` mechanism in `auto_sync.rs` (field, init, suppress_just_pulled, callsite, is_just_pulled_suppressed) — orphaned during the v0.2.19 rebase; v0.2.18's `recently_written` covers the same window. `ScanProgressChip.svelte` + `scan-progress.svelte.ts` removed (superseded by SyncModal).

### Verify
- `cargo check`: clean. `svelte-check`: 0 errors, 6 a11y warnings (all pre-existing patterns).

v0.2.19-alpha-test archived to git log.
