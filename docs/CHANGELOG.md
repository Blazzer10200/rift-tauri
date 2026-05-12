# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.25-alpha-test — 2026-05-12 — Actionable SFTP error messages

Trey hit "Permission denied: Permission denied" upload failures across all his edits — russh-sftp doubles the message AND the literal text doesn't tell the user it's a Linux ownership issue on the FiveM server (Blazzer-owned files in 0755 dirs; Trey's `treyday` user can read but not create new files / tmps in the parent).

New `format_sftp_err` helper applied to all upload_atomic_via failure paths (create-tmp, write-tmp, rename). Collapses "X: X" duplicates and, when the error contains "Permission denied", appends the actual server-side fix:

> Server admin: sudo chgrp -R \<shared-group\> \<parent dir\> && sudo chmod -R g+w \<parent dir\> && sudo find \<parent dir\> -type d -exec chmod g+s {} \;

Also catches "No such file" for the missing-parent-dir case.

## v0.2.24-alpha-test — 2026-05-12 — Pull Now: actually fast (cache-based, no scan)

v0.2.21-v0.2.23 Pull Now re-ran the full drift scan before dispatching pulls — so it felt identical to Reconcile (the 30s SFTP batch listing was the slow part, not the pulls). Trey confirmed it: "same exact task and taking just as long if not longer."

**Real fix:** `last_scan_entries` cache on AutoSyncEngine. drift_watcher's 10s tick + every kick_drift_reconcile run now write their scan entries into the cache. `force_pull_now` dispatches pulls directly from the cache — NO scan. Sub-second.

Modal gets a new `mode: "scan" | "pull"` axis. In pull mode the status line says "Pulling cached changes… (no scan needed)" instead of "Listing remote files…" so the user sees what's happening. Activity feed shows "pull-now started" + individual file pulls as they land.

Edge case: if drift_watcher hasn't ticked yet (very first second post-connect), cache is empty → Pull Now shows "nothing pending" with 0 counts. Hit Reconcile to populate.

## v0.2.23-alpha-test — 2026-05-12 — Auto-snap browser tabs to profile root

Fixes "not a directory" left-pane error after a profile's local_root (or remote_root) changes — e.g. user moved their FiveM dir and updated Settings. Browser tabs persist their navigation across sessions in localStorage, so old paths survived the root change and the pane errored out against folders that no longer exist at the new location.

New `$effect` in TwoPane: when profile.localRoot / remoteRoot changes, walks every tab; if a tab's current path doesn't start with the new root (normalized: lowercase, forward slashes), snaps that tab back to the new root. Belt-and-suspenders for the move-your-server-dir flow.

## v0.2.22-alpha-test — 2026-05-12 — Pull Now in OpRail (discoverability fix)

v0.2.21 put the Pull Now button only inside the SyncModal footer, which meant you had to scan first to see it — defeats the purpose. v0.2.22 adds the button to the OpRail (middle column) right below the Reconcile button, always visible. New `DownloadCloud` icon to distinguish from regular Download (single-arrow) and Reconcile (refresh circle). Tooltip: "Pull Now — fetch any remote changes immediately (auto-pulls every 10s otherwise)."

Modal-internal Pull Now button retained for the case where the user clicked Reconcile and now sees a pull count.

## v0.2.21-alpha-test — 2026-05-12 — Snappier auto-pull + Pull Now button

Closes the "buddy pushed but my Rift hasn't ticked yet" UX gap. Tested by Blazzer + Trey: push direction was instant, pull direction needed a manual Sync click to feel timely. Two changes fix it.

### Landed
- **Faster auto-pull cycle** — `DEFAULT_SCAN_INTERVAL_SECS` 30 → 10. Drift watcher now polls every 10s instead of 30s, so buddy-side pushes appear within ~10s of upload (was up to 30s). 3x more SFTP listings, ~2s each on a typical tree — negligible. Users who want the old behavior can still set their own interval via Settings.
- **Pull Now button** — appears in the SyncModal footer when a completed scan reports `To Pull > 0`. New `diag_force_pull_now` Tauri cmd calls `AutoSyncEngine::force_pull_now()`, which re-runs the drift scan AND dispatches `pull_one` for every ToPull entry (vs. plain Sync, which only auto-enqueues ToPush). Modal re-enters scanning phase, activity feed populates with `RemotePullStart/Done` events, completion shows `pull_dispatched` count.
- **Modal listing-phase hint** — "Listing remote files… (this may take a moment on the first scan)" status line + activity entry on `drift_scan_start`. Closes the silent ~30s pre-listing window where the modal looked frozen.
- **Pull-button styling** — new `.btn-accent` variant in the modal matches the existing UI language (soft accent fill, accent border, hover swell).

### Verify
- `cargo check`: clean. `svelte-check`: 0 errors, 5 a11y warnings (all pre-existing).

v0.2.20-alpha-test archived to git log.
