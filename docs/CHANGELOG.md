# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.50-alpha — 2026-05-12 — Connection-reliability foundation

Diagnosed two stacked failure modes during v0.2.49 validation that turned the sync pipeline brittle: (1) editor atomic-save tmp files (`<file>.tmp.<pid>.<hex>` — observed `client.lua.tmp.9076.310b94f68378` etc.) were being captured by the watcher, acquiring remote `.rift-lock` sentinels, then failing to upload (local renamed away by the editor) and leaking orphan locks; (2) russh keepalive detects truly-dead sessions in ~60 s but not wedged-but-not-dead ones (NAT timeout, half-closed TCP, server stall), so SFTP ops would hang the worker indefinitely, surface as "Error" badge after a batch failed, and never reconnect. The v0.2.49 ship is also gated on a Mirror-mode arc — pivoted to fix the foundation first.

### Editor-tmp ignore (root cause of recurring orphan locks)

`sync/ignore.rs::classify` already caught `<base>.tmp.<digits>`; extended to also match `<base>.tmp.<pid>.<hex>` with a tight rule (pid ≤8 digits, hash ≥8 hex chars, no third dot-segment) so legit user paths like `report.tmp.draft.md` keep passing. Added `.crswap` (VSCode crash-recovery) + `.crdownload` (Chrome partial-download) to the extension blacklist. Three new unit tests on the observed Endure patterns. **This alone eliminates the most-common orphan-lock pathology.**

### SFTP op-level timeouts

Added `with_t()` helper in `sftp/transfer.rs` wrapping every op against `tokio::time::timeout`. Tiers: `T_QUICK` 10 s (cleanup, set_metadata, close), `T_NORMAL` 30 s (mkdir, rename, create-tmp), `T_BODY` 120 s (write/read file body). On timeout, returns a wedged-connection error string AND emits `DiagStage::ConnectionWedged` at Error level — distinct signal so the UI can surface a Reconnect affordance instead of a generic upload fail. Applied to `upload_atomic_via`, `download_atomic_via`, `upload_bytes`, `download_file`. Listing path covered in `sftp/list.rs` (`LIST_T` 120 s) across exec fast-path, serial fallback, and worker pool.

### Lock-release race fix

Worker terminal-path lock release (`auto_sync.rs::process_entry`) switched from `tokio::spawn` + `track_background` to inline-await with a 5 s timeout. The previous spawn could be aborted by engine `stop()` before the SFTP delete fired — that was the second source of orphan locks in the probe2.txt sanity-test (engine flipped to Error on the same batch, killed the spawned release task before it ran). Inline await blocks the worker for typically <100 ms on the release; 5 s ceiling caps the cost on a hung session.

### Manual recovery: Sweep locks button

New `sync_sweep_stale_locks` Tauri command walks every watched remote root and reclaims our-own `.rift-lock` files older than `STALE_SEC` (180 s) via `LockPresence::sweep_stale_mine`. Wired to a new "Sweep locks" button on the Sync page header next to Rescan; reports the swept count or "No stale locks found." Safe to call anytime — gates on `body.user == me` + age, so foreign locks stay untouched.

### Prod-side cleanup

Out-of-band: chowned 9× `[qbx]/qbx_*` dirs that were `root:root drwxr-xr-x` (initial FiveM install) to `blazzer:fxserver drwxrwsr-x` (2775, group-writable). These were the source of the "sync failed: create tmp …Qbox_F8F76…" red errors in the diagnostics panel — Rift's `mkdir_p_via` chmods only on dir CREATION, not on pre-existing root-owned dirs.

### Verify

`cargo check` clean 5.06 s · `cargo test --lib` 46 passed (incl. new `editor_tmp_pid_hex` + `cr_swap_and_download`) · `svelte-check` 0 errors / 0 warnings across 3996 files.

### Deferred to v0.2.51

- Full auto-reconnect with 3× exponential backoff on `ConnectionWedged` (manual sweep + reconnect via existing UI for now)
- Mirror mode for Push-all (Bug 1) — gated on integration test suite
- Integration test suite (phase 1: 10 scenarios w/ mock SFTP)
- Listing-accuracy targeted fix (v0.2.49 instrumentation never reproduced the 56≠61 mismatch — kept logging in place)
