# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.52-alpha — 2026-05-13 — Rename-event handling, state-machine fix, watched-root poll

User reported a folder he renamed/deleted locally did not propagate to the remote, and the "Sync error" badge reappeared after the operation. Web-researched notify-rs v8.2.0 Windows behavior to ground the diagnosis (sources: notify GitHub issues #261, #376, #403; PR #674; docs.rs v8.2.0). Three root causes confirmed and fixed.

### Rename-event handling (`auto_sync.rs::on_fs_event`)

notify v8 on Windows emits folder renames as two separate events: `Modify(ModifyKind::Name(RenameMode::From))` for the old path and `Modify(ModifyKind::Name(RenameMode::To))` for the new path (Windows backend never emits `RenameMode::Both` — issue #376). Recycle-Bin delete is technically a MOVE to `$RECYCLE.BIN` and surfaces as `Modify(Name(From))` with no matching `To` since the destination is outside watched scope. Pre-v0.2.52 these bucketed via the catch-all `Modify(_) => Modified` arm, which failed `wait_for_readable` on the now-vanished path, flipping engine to Error without propagating the delete. Explicit arms now route `From → Deleted` (sftp.delete handles dir recursion through `delete_recursive_via`) and `To → Created` (existing 500 ms `kick_drift_reconcile` debounce picks up the new subtree).

### Smarter state machine (`auto_sync.rs::flush_batch`)

Don't escalate `AutoSyncState::Error` on a single fail. New `consecutive_failed_batches: AtomicU64` field tracks fail streak. Threshold = 3. Single/double fails surface as `AutoSyncState::Watching` with a "N retry pending" detail string; only 3+ consecutive batches with no clean batch between flip to Error. Any clean batch resets the streak. `ConnectionWedged` from v0.2.50's `with_t()` timeout helper still escalates independently — that's a real transport signal, not a single-file fluke.

### Watched-root-vanished poll (notify issue #403 mitigation)

Open notify-rs bug #403 (unfixed in v8.2.0): when the user deletes the dir Rift is directly watching, the Windows backend silently unregisters the watch with NO `Remove` event. v8.1.0 fixed an infinite-hang in this path but did not add event emission. Pre-v0.2.52 this left the engine alive but oblivious. New 5 s tick polls `local_root.exists()` across every watched folder; on miss, logs at Error level, emits a visible Diagnostics event, and kicks a drift reconcile. De-duped via a `seen_missing` HashSet so the panel doesn't spam — fires once per missing root until it returns.

### Verify

`cargo check` clean · `cargo test --lib` 46 passed · no frontend changes.

### Deferred to v0.2.53

- Mirror mode (Bug 1) — `local-missing + remote-has + baseline-has` → new `DriftBucket::ToDeleteRemote` bucket, opt-in toggle, typed "MIRROR" confirm gate, dry-run preview default. Needs its own evening for the UI to ship safely.
- Full auto-reconnect with 3× exponential backoff on `ConnectionWedged` events.
- Integration test suite phase 1 (10 scenarios w/ mock SFTP).
