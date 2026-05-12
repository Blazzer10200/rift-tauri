# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.45-alpha — 2026-05-12 — Unified Sync page + partial-listing root-cause fix

### Sync correctness — root-cause for v0.2.44's phantom ToDeletes

Spam-rescan diagnostic revealed the same resource ([depend], [ox], [world]) would surface a different ToDelete population each click — proof the remote SFTP listing was non-deterministic. Root cause: `list_via_exec` (the `find`-over-exec fast path) broke its channel-read loop on `ExitStatus`, but SSH does not order `ExitStatus` after final `Data`. On fast `find` runs over deep trees the server sent `ExitStatus` while trailing `Data` chunks were still buffered on the channel — the early break discarded them, producing random short listings. Fix: drain the channel until close (`wait()` returns None) which is guaranteed to follow `Eof + Close` per SSH-CONNECTION. Same one-line bug in `get_remote_sha1` patched identically — truncated SHA was silently causing spurious file-equality misses on the push pre-flight collapse path.

Defense-in-depth: new `SyncSnapshot::count_under(prefix)` helper + suspicious-shrink guard in `drift_scanner::scan_folder`. If the baseline had ≥10 files under a remote_root but the listing returned <50%, the folder aborts as `SuspiciousEmptyAborted` and emits a `DriftScanProgress` warn with `baseline_count` / `listing_count` for visibility. Belt: russh `Config { window_size: 2 MiB, maximum_packet_size: 32 KiB }` in both `sftp::open_session` and `tunnel::start`, up from the conservative defaults that previously pressured the SFTP worker fallback path.

### Unified Sync page (v0.2.45)

Replaces the corner Quick Actions panel + transient scan modal with a dedicated tab. Per-resource expandable cards with checkbox selection, bucket-tone pills, tone-mixed surfaces (push=accent, pull=info, delete=danger, conflict=warn), inset-stripe active state per canon, dominant-tone resource cards, hide-when-zero totals (only renders when 2+ buckets active), empty-state title+hint pair (Everything in sync / Not connected / Scanning), Rescan + Pull all + Push all in the hero. Tab tab→ `Ctrl+3`, palette entries shifted Conflicts→Ctrl+4, Settings→Ctrl+5. Backend: `drift_snapshot()` getter + `apply_selected(local_paths)` engine method with per-bucket dispatch. Tauri cmds `sync_get_drift_snapshot` + `sync_apply_selected`.

### Mass-delete guard — policy refinement

Explicit user selection from the Sync page now **bypasses** the mass-delete circuit breaker with a `WARN — N local-deletes — user-selected, dispatching anyway` activity entry. The breaker exists to catch SCAN-DRIVEN runaways (`force_pull_now`, tombstone propagation) — those still hard-block. Checkbox-and-click is informed consent: the operator saw each path. Auditability preserved via activity log.

### Window chrome / scroll lock

`html, body { overflow: hidden }` + `.shell { width: 100vw; overflow: hidden }` — document-level scrollbar can no longer appear regardless of internal content width, fixing the symptom where restoring from maximize would slide the titlebar's right side (Pulls/Push/min/max/close) out of the visible viewport. Titlebar flex priorities flipped: `.left` shrinks first (server picker truncates), `.drag-fill` takes leftover space (window drag handle), `.right` stays flex-shrink:0 (window controls always reachable).

### Verify

`svelte-check` 0/0/3996 · `cargo check` clean (incremental via `tauri dev` watcher) · spam-rescan now deterministic across 10+ rapid clicks.
