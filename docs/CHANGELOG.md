# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.51-alpha — 2026-05-13 — "Disconnected" UX clarity hotfix

User reported "I'm getting disconnected error" after pushes that actually succeeded — file landed on prod cleanly, but the topbar badge flipped to red "Error" anyway, which he mentally translated to "disconnected." Diagnosis: when ANY entry in a flush batch returns `EntryResult::Fail` (a single editor-tmp race on `wait_for_readable`, an adjacent backup file the editor created, etc.), `auto_sync::flush_batch` flips engine state to `AutoSyncState::Error` (`auto_sync.rs:2064`). The connection is fine — the LAST batch had a partial failure. The badge label "Error" + tooltip "auth failed" fallback misleadingly implies a transport drop.

Cosmetic-only fix this release:
- `Titlebar.svelte` error badge label: `"Error"` → `"Sync error"`; tooltip enriched to `"{detail} — connection still active"` so the user can see the connection is up + read which file failed.
- `connection.svelte.ts::computePill` returns `"Sync error"` instead of `"Disconnected"` for the `state === "error"` case. `SyncPill` type extended.

No behavior change — engine still tracks `state === "error"`, batches still tally `fail`, all diagnostics events still emit. Just clearer labeling.

### Deferred to v0.2.52 (the real fix)

Smarter state-machine: don't flip engine to Error on 1-file-fail. Only escalate to Error on (a) genuine `ConnectionWedged` events from v0.2.50's timeout helper, OR (b) 3+ consecutive failed batches with no success between. For a single failed entry among many successful, surface a "1 retry pending" indicator and stay in `Idle`/`Watching`. This is coupled with the auto-reconnect arc already queued for v0.2.52.

### Verify

`svelte-check` 0 errors / 0 warnings across 3996 files. No Rust changes — `cargo check` not re-run, prior v0.2.50 clean.

