# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 27 — 2026-05-12 — v0.2.26: perms-recurrence killer + working Cancel

Continuation of S26 after Trey's chmod-fix wasn't sticking. Root cause: umask 0022 on the FiveM server kept making every NEW file 0644, so v0.2.25's one-time chgrp/chmod cycle had to be re-run after every push. Fix moved into Rift itself.

### Ship trail
- **v0.2.26** —
  - **SFTP `set_metadata(perms=0o664)` after each `upload_atomic_via` rename** ([sftp/mod.rs](src-tauri/src/sftp/mod.rs#L1013)). Combined w/ existing setgid on parent dirs, every Rift-uploaded file is group-writable on the fly. Permanent fix for the recurring teammate EACCES.
  - **Cancel button works for `force_pull_now` AND `drift_watcher::run_tick`.** Both now register `CancellationToken`s in the shared `current_scan_cancel` slot via new `pub(crate)` helpers `register_scan_cancel`/`clear_scan_cancel` on `AutoSyncEngine`. force_pull_now checks token between dispatched pulls + emits `DriftScanResult { cancelled: true }` on abort.
  - **`tokio::sync::Semaphore` (4 permits)** caps concurrent pulls in `force_pull_now` — was flooding SFTP session on Trey's Tailscale link.
  - **SyncModal drops `drift_scan_progress` when `mode === "pull"`** — parallel drift_watcher ticks were leaking `scanning [ox] (8/8)` rows into Pull Now's activity feed.

### Server-side (out-of-band)
- Ran `chmod -R g+w` across all `blazzer`-owned files under `/opt/fxserver/.../resources/` for backlog cleanup. New uploads self-chmod via v0.2.26.
- `setfacl` not available on the server (no `acl` package). Considered but skipped — Rift-side chmod is cleaner anyway.

### Verify
- `cargo check`: clean (1.71s). `svelte-check`: 0 errors, 5 pre-existing a11y warnings.
- Tag pushed: `60db799` → `Blazzer10200/rift-tauri` + Velopack release at `rift-releases/releases/tag/v0.2.26-alpha-test`.

### Flagged for v0.2.27+
- **Token-slot race** — `register_scan_cancel` replaces the slot, so an overlapping `run_tick` mid-`force_pull_now` could shadow the user op. Move to `Vec<CancellationToken>` w/ `cancel_all` if it bites.
- **Per-folder streaming during initial SFTP `list_recursive_batch`** — still emits no progress during the 30s+ listing. Instrument per-root completion.
- **Pre-flight write probe** on autosync start (catch EACCES at connect time).
- **5 a11y warnings** in Settings/LocalPane/RemotePane — pre-existing, UX sweep.

### Investigation captured for future reference
The "Pull Now feels stuck for 5 min on Trey's end" mystery had multiple overlapping causes — all addressed by v0.2.26's four edits combined: (1) every push failing → file landed 0644 → next push fails → loop, (2) Cancel did nothing → user couldn't escape, (3) N parallel SFTP pulls saturated his uplink, (4) modal showed misleading "scanning" progress from background ticks. Belt-and-suspenders fix.

---

## Session 26 — 2026-05-12 — Sync modal + rapid-iteration pull-side polish (v0.2.20 → v0.2.25)

Single-session ship streak. v0.2.20 introduced SyncModal + scan cancel; v0.2.21-v0.2.25 are post-field-test polish driven by Blazzer + Trey using Rift live for a real FiveM/Endure RP sync workflow.

### Ship trail (newest first)
- **v0.2.25** — Actionable SFTP errors. Russh-sftp double "Permission denied: Permission denied" collapsed via new `format_sftp_err` helper in [sftp/mod.rs](src-tauri/src/sftp/mod.rs). EACCES on tmp-create/write/rename now appends the server-side fix command (chgrp + chmod g+w + setgid). Trey's permission errors triggered this — files were Blazzer-owned in 0755 dirs.
- **v0.2.24** — **Pull Now actually fast.** v0.2.21's force_pull_now re-ran the full 30s drift scan before dispatching, so it felt identical to Reconcile (the SFTP batch listing IS the slow part). Fixed by caching `last_scan_entries: std::sync::Mutex<Vec<DriftEntry>>` on AutoSyncEngine. drift_watcher tick (every 10s) + kick_drift_reconcile both write the cache. force_pull_now dispatches from the cache — sub-second. SyncModal gained a `mode: "scan" | "pull"` axis; pull mode shows "Pulling cached changes… (no scan needed)" instead of the misleading "Listing remote files…".
- **v0.2.23** — Auto-snap browser tabs to new profile root. After Trey moved his FiveM dir + edited Settings, the left pane stuck on `C:\fivem server\[endure]` (old path) because `browser-tabs.svelte.ts` persists tab navigation in localStorage. New `$effect` in TwoPane normalizes paths (lowercase, fwd slashes) and snaps any tab whose path doesn't start with the active profile's root back to that root.
- **v0.2.22** — Pull Now button promoted to OpRail (middle column) for discoverability. v0.2.21 buried it in the modal footer where you had to scan first to find it. New `DownloadCloud` icon (distinct from Download arrow + Reconcile circle).
- **v0.2.21** — Snappier auto-pull cadence (30s → 10s, [drift_watcher.rs:41](src-tauri/src/sync/drift_watcher.rs#L41)) + initial Pull Now button (modal-internal only) + listing-phase hint.
- **v0.2.20** — SyncModal (center-stage overlay, replaces ScanProgressChip), scan cancel (CancellationToken between folders, `current_scan_cancel: std::sync::Mutex` because kick_drift_reconcile is sync), path-guards re-applied on upload/download, Open-in-editor ctx menu, dead `just_pulled_suppress_until` purged.

### Verify (post-v0.2.25)
- `cargo check`: clean. `svelte-check`: 0 errors, 5 a11y warnings (all pre-existing in Settings/LocalPane/RemotePane).
- Working tree clean. All 6 commits pushed (`84caea6` → `5747fce`). Releases v0.2.20-v0.2.25 on `rift-releases`.
- Field-validated: push direction works (Blazzer added `endure_shooting`, Rift auto-detected + synced to fxserver). Pull direction works post-v0.2.21 (10s tick) + manual Pull Now. Remaining blocker is Linux-side perms (Blazzer-owned files in 0755 → Trey can't write tmps); user is fixing via chgrp + chmod g+w + setgid on the server.

### Flagged for v0.2.26+
- **Per-folder streaming during initial SFTP listing.** Backend's `SftpClient::list_recursive_batch` is the slow part of a scan (~30s deep trees), emits no progress events. v0.2.20 mitigated with "Listing remote files…" hint. Real fix: instrument the batch to emit per-root completion. ~30-40min, touches [sftp/mod.rs](src-tauri/src/sftp/mod.rs).
- **Pre-flight write probe** on autosync start — catch EACCES at connect time instead of first push. ~15min.
- **Review Pull button in modal** when result.pull > 0 → opens DriftReview. Lower urgency now that Pull Now exists.
- **5 a11y warnings** — Settings/LocalPane/RemotePane. Pre-existing patterns; UX-only sweep.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`.

**Current state (post S26):** **v0.2.25-alpha-test SHIPPED** to `rift-releases`. 6 ships this session (v0.2.20-v0.2.25). Blazzer + Trey running it live on Endure RP FiveM server. Outstanding non-Rift task: Blazzer fixing server-side Linux perms (chgrp fxserver + chmod g+w + setgid on `/opt/fxserver/.../resources/`). Once that's done, push/pull bidirectional works without permission errors.

**Next session likely entry points:**
1. Confirm chmod fix landed + bidirectional push works on Trey's end.
2. Pick next item: per-folder listing streaming (cleanest pickup), pre-flight write probe, brainstorm #4 (per-resource sync mode), or brainstorm #2 (buddy presence).

## CRITICAL DON'T-TOUCH
- russh `ring` backend + reqwest `rustls` features only (NASM blocks aws-lc-rs)
- `~/.rift/*.json` compat — don't change rename rules; don't drop `serde(flatten) extra`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver)
- DriftWatcher conflict-rename guard — never overwrite dirty local
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo
- `path_guard.rs` API frozen (`validate_remote_child`, `validate_local_child`) — `edit/in_place.rs` + lib cmds depend
- `rename_via` is strict (user-facing); `rename_overwriting_via` is ONLY for atomic upload tmp-swap
- **Source `.secrets/env.sh` first on ship/auth tasks** — Claude Code bash is non-interactive, won't auto-load
- **`current_scan_cancel` + `last_scan_entries` are std::sync::Mutex** (NOT tokio) — `kick_drift_reconcile` is sync and called from notify event handler; tokio Mutex `blocking_lock` panics there. Don't "fix" it.
- **`force_pull_now` dispatches from cache, NOT a fresh scan** — re-scanning makes it identical to Reconcile (30s SFTP batch listing is the cost). drift_watcher's 10s tick keeps cache fresh.
