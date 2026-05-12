# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`.

## Session 49 — 2026-05-12 — Partial-listing root-cause + ship v0.2.45-alpha

### Completed
- **Root-caused** the v0.2.44 phantom-ToDelete bug via spam-rescan diagnostic. The `list_via_exec` channel-read loop was breaking on `ExitStatus`, but SSH does not order ExitStatus after final Data — fast `find` runs over deep trees discarded trailing buffered Data, producing non-deterministic short listings. Fix: drain to channel close ([sftp/list.rs:253](src-tauri/src/sftp/list.rs)). Same bug in `get_remote_sha1` ([sftp/remote_exec.rs](src-tauri/src/sftp/remote_exec.rs)) — truncated SHA was silently breaking push pre-flight collapse.
- **Defense-in-depth**: new `SyncSnapshot::count_under(prefix)` + suspicious-shrink guard in `drift_scanner::scan_folder` — aborts folder if baseline ≥10 files but listing <50%, emits warn diag event.
- **russh buffer bump** to 2 MiB window / 32 KiB packet in both `sftp::open_session` + `tunnel::start` — prevents same shape of bug on SFTP worker fallback path.
- **Unified Sync page polish**: hide-when-zero totals (only 2+ buckets), tone-mixed surfaces, inset-stripe active state, empty-state title+hint pair, removed double-bracket render bug on resource names, dropped misleading `.kbd` styling on footer hint, fixed Apply button label.
- **Window chrome lock**: `html/body/.shell { overflow: hidden }` + Titlebar flex priority flip — `.left` shrinks first (server picker truncates), `.drag-fill` takes leftover, `.right` flex-shrink:0 (window controls always reachable).
- **Guard policy**: explicit user selection from Sync page bypasses mass-delete breaker (emits WARN to activity feed, still auditable). Scan-driven paths (`force_pull_now`, tombstone propagation) still hard-block.

### Files Modified (S49)
- `src-tauri/src/sftp/list.rs` — drain channel to close (drop break on ExitStatus)
- `src-tauri/src/sftp/remote_exec.rs` — same drain fix in `get_remote_sha1`
- `src-tauri/src/state/sync_snapshot.rs` — new `count_under(prefix)` helper
- `src-tauri/src/sync/drift_scanner.rs` — suspicious-shrink guard in `scan_folder`
- `src-tauri/src/sftp/mod.rs` + `src-tauri/src/tunnel/mod.rs` — russh window/packet bump
- `src-tauri/src/sync/auto_sync.rs` — guard policy: user-selected = WARN instead of BLOCK
- `src/lib/components/sync/SyncPage.svelte` — full UI polish + label fixes + bracket fix
- `src/app.css` — `html, body { overflow: hidden; overscroll-behavior: none }`
- `src/lib/components/AppShell.svelte` — `.shell { width: 100vw; overflow: hidden }`
- `src/lib/components/shell/Titlebar.svelte` — flex priority restoration for drag region
- `package.json` + `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json` — 0.2.44-alpha-test → 0.2.45-alpha
- `docs/CHANGELOG.md` — v0.2.45 entry

### Audit notes — Treyday safety
Foreign-lock check + dirty-local check intact on both `pull_one` and `delete_local_one`. setgid 2775 chmod in `mkdir_p_via` preserved for shared-group teammate pushes. Keepalive 20s × 3 on both russh sessions for Tailscale half-dead detection. Conflict-rename guard preserved in `pull_one`.

### Next session priorities
1. Smoke v0.2.45 on both Blazzer + Treyday sides over Tailscale — spam-rescan stability check, push/pull/delete round-trip, conflict-rename when both edit same file.
2. Optionally — re-fine-tune the mass-delete guard threshold (user noted "good thing to have, but needs fine-tuning later").

---

## Session 48 — 2026-05-12 — Unified Sync page (v0.2.44-alpha-test, unshipped)

> **NOTE:** User flagged mid-session that this may not be the intended working copy. Dev server launched + connected cleanly (v0.2.44-alpha-test, 8 watchers attached) so changes landed in the live tree. Verify folder path before shipping.

### Completed
- Built unified **Sync page** consolidating the 3 Quick Action buttons (Scan drift / Pull pending / Push pending) into a dedicated tab with full drift-review UX.
- Backend: `drift_snapshot()` getter + `apply_selected(local_paths)` engine method (per-bucket dispatch: ToPull→pull_one, ToDelete→delete_local_one, ToPush→queue_path+flush; circuit breaker preserved). Tauri commands: `sync_get_drift_snapshot` + `sync_apply_selected` wired in lib.rs invoke_handler.
- Frontend state: `src/lib/state/sync-page.svelte.ts` — SyncPageStore (runes class: loading/busy/entries/expanded/selected; groups+totals getters; refresh/rescan/applySelected/pullAll/pushAll).
- Frontend component: `src/lib/components/sync/SyncPage.svelte` — hero strip, 5-cell totals (push/pull/delete/conflict/scan-age), per-resource expandable cards with checkbox select + bucket-tone pills + guard-threshold warning + reason text, Apply selected + Pull all/Push all footer.
- AppShell: Tab type extended, SyncPage imported + rendered (Ctrl+3), palette entries shifted (Conflicts→Ctrl+4, Settings→Ctrl+5), keyboard route [1-5] updated.
- TabRail: Quick Actions panel + ~80 lines `.qa-*` CSS removed; Sync tab added (RefreshCcw icon, accent tone).
- SyncModal left alive (still rendered by TwoPane.svelte:344).
- `cargo check` clean 0.45s · `svelte-check` 0 errors / 0 warnings / 3996 files · `tauri dev` launched OK.

### Not Done / Still Open
- NOT committed, NOT version-bumped, NOT pushed — awaiting user smoke-test + folder confirmation.
- Session 47's 5 v0.2.39+ backlog items still open (pre-flight write probe, activity-feed grouping, DriftReview mismatch, svelte-ignore, .rift-lock sweep).
- Session 47's partial-listing root-cause investigation (`to_delete: 430` phantom entries) still open — Sync page now exposes paths so user can triage.

### Key Decisions
- HANDOFF-flagged "DriftReview bucket-string mismatch" = moot (no PascalCase bucket filters exist in shipped `src/`). Phase 2 skipped.
- Mass-delete circuit breaker unchanged — Sync page shows guard warning when selection would trip it.

### Next Steps
1. Confirm this is the correct rift-tauri working copy (check `git remote -v` against expected repo).
2. Open Sync tab, connect to Endure RP, Rescan — verify 233 ToDelete entries render with paths.
3. Pick < threshold items, click Apply selected — confirm activity feed shows the dispatched actions.
4. If all good: `CHANGELOG` entry → version bump → `/check` → `/git-ship`.

### Files Modified (S48)
- `src-tauri/src/sync/auto_sync.rs` — `drift_snapshot()` + `apply_selected()` methods (~line 1170)
- `src-tauri/src/lib.rs` — 2 new commands + 2 invoke_handler entries
- `src/lib/state/sync-page.svelte.ts` — new file
- `src/lib/components/sync/SyncPage.svelte` — new file (~500 lines)
- `src/lib/components/AppShell.svelte` — Tab type, import, palette, keyboard route, render branch
- `src/lib/components/shell/TabRail.svelte` — Quick Actions removed, Sync tab added

---

## Session 47 — 2026-05-12 — Per-resource scan breakdown + open partial-listing investigation (v0.2.44)

### Completed
- Live-dev session ran v0.2.40 → v0.2.44 (5 ships in one day). Push direction was the focus across all of them; root-caused and verified working at the end. Pulls always worked.
- **Push fixed end-to-end** (v0.2.43): scan-cache ToPush entries promote into dirty queue (`promote_scan_pushes_to_dirty`), auto-scan fallback when both dirty + cache empty, cache clears after successful push, critical `DriftScanResult` event bypasses the 200/s diag rate limit (was getting dropped during 192-file bursts → modal hung forever).
- **Cancel bulletproofed**: outer + inner `tokio::select!` in `process_entry`, 60s frontend hard-watchdog, 3s force-close button, X-button works while cancelling, activity rows emit on every early-return path.
- **Connection liveness** (v0.2.42): russh keepalive 20s × 3 in both `sftp::open_session` + `tunnel::start`, `upload_bytes` shutdown() fixed write-probe ENOENT, cleanup is best-effort.
- **Open bug** (v0.2.44 diagnostic added, not fixed): after a fresh pull on Endure RP, drift scan reports `to_delete: 430` on a fully-synced server. Mass-delete circuit breaker correctly BLOCKED `[depend]` (39) + `[ox]` (108). Remaining ~283 distribution unknown. Hypothesis: `list_recursive_batch` silently dropping files mid-listing for deeply-nested paths (ox_lib has 4-5 level deep dirs) or `[bracket]`-encoded resource dirs. v0.2.44 adds `by_resource` map to drift_scan_result so next diagnostic localizes which folders are partial.

### Next session priorities
1. User triggers Sync on v0.2.44, exports `/diagnostics`. The `by_resource` block in drift_scan_result pinpoints which resources have the phantom ToDelete entries.
2. Root-cause partial listing: instrument `list_recursive_batch` to log per-directory results + final count vs local walk count. Possible fixes — russh `window_size`/`maximum_packet_size` increase, retry partial listings w/ smaller depth chunks, or detect "remote count vs snapshot count" mismatch and abort scan per-resource as SuspiciousEmptyAborted.
3. Consider lowering mass-delete circuit breaker ceiling from 25 → 10 OR adding a global "total ToDelete > 30% of all local files" abort.

### Files Modified (S47)
- `src-tauri/src/sync/auto_sync.rs` — by_resource breakdown in reconcile result emit
- Bumped to v0.2.44-alpha-test (package.json + Cargo.toml + tauri.conf.json)

### Today's known-good (v0.2.44)
Push parity ✓, Pull cache parity ✓, Cancel 4-layer ✓, russh keepalive ✓, write-probe shutdown ✓, modal rate-limit bypass ✓, activity orphan-row guards ✓, in-app StatusBar sync pill while busy ✓.

## Earlier sessions — compressed

**S46 (v0.2.43, 3 root causes)** — `promote_scan_pushes_to_dirty()` + auto-scan in `force_push_now` (push couldn't see scan results); both force_push/pull_now clear `last_scan_entries` after non-cancelled ops (was re-pushing same files indefinitely); critical lifecycle stages (DriftScanStart/Result, RescanSignal, SftpConnect/Disconnect, RemoteScanResult, BridgeAck, System) bypass `spawn_frontend_pump`'s 200/s rate cap (was silently dropping DriftScanResult on 192-file bursts → modal hang forever). Plus orphan-row guards on every `process_entry` early-return + `eprintln!` breadcrumbs in `sync_*_pending` cmds + `force_pull/push_now` bodies.
**S45 (v0.2.42)** — `upload_bytes` shutdown() fix (probe ENOENT root cause), `probe_write_access` cleanup best-effort, russh keepalive 20s × 3 in both Config sites, `process_entry` takes `Option<CancellationToken>` + `tokio::select!` against upload future, push activity start-rows ("uploading…"), StatusBar sync pill while `syncModal.busy && !syncModal.open`.
**S44 (v0.2.41)** — TabRail Quick Actions accuracy (Reconcile→Scan drift, Pull all→Pull pending, Push all→Push pending; Push warn→accent), backend cmds `diag_*` → `sync_*`, `flush_batch`/`flush_all_now` cancel-aware w/ lazy-pop, `force_pull_now` orphans handles on cancel, SyncModal `busy` flag + Run-in-background button.
**S43 (v0.2.40)** — Codex audit 15-item backend hardening pass (symlink-safe deletes, rename collision split, OpStatus per-path, profile-path containment, autosync deadlock fix, write-probe at connect, `.rift-lock` sweep), `sftp/mod.rs` split into 5 files (1348L→1254L public API unchanged), `auto_sync/path.rs` extracted. Frontend cleanup: 5 audit fixes, 9 svelte-check warnings → 0, 5 dead CSS rules dropped, TwoPane horizontal-scroll polish.
**S42 + earlier** — Settings 7→4 sections, Terminal multi-tab w/ AppShell-persistence, OpRail kill, Titlebar/TopBar merge, palette, tone-coded TabRail, `.btn` skeleton. Full diffs in `git log -- docs/HANDOFF.md`.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri (Rift). Path: `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.45-alpha** — partial-listing root-caused + fixed (exec-channel ExitStatus race), unified Sync page shipped, mass-delete guard refined to bypass on explicit user selection. Shipped via Velopack to `Blazzer10200/rift-releases`.

**Rhythm:** apply canon (`docs/UI-POLISH-MAP.md`) to remaining unpolished pages. Tone via `data-tone` + `--tone` var, surface 8-14% rest / 22% hover, hover icon scale 1.1-1.18 w/ overshoot + reduced-motion guard, active inset-stripe `inset 2px 0 var(--tone)`, focus-within blur, hide-when-zero, title+hint empty states, truncation tooltips, `hour12: true` everywhere.

**Polish done:** AppShell, Titlebar, TabRail, StatusHero, StatusBar (+ sync pill S45), CommandPalette, TwoPane, LocalPane, RemotePane, PathBreadcrumbs, LockBadge, Confirm, Reupload, FlashToast, ActivityToast, ActivityFeed, ConflictList+Resolver, Terminal multi-tab, Settings 4 sections, SyncModal (Run-in-bg + Close-anyway + 60s watchdog), `.btn` skeleton.

**Pending:** Tier 3 login (Bootstrap, AddServer, Keygen), Tier 4 modals (UpdateDialog), Diagnostics polish. Carryover: Terminal Settings sub-view (APIs ready on `terminal.svelte.ts` store). Appearance density/font/accent-tint controls. `auto_sync/{watch,flush}.rs` stubs (cross-cut state, mechanical move risky).

**Top-of-queue debug:** the partial-listing root cause (see Session 47 above). Install v0.2.44, click Sync, export diagnostics, read the new `by_resource` field in drift_scan_result.

**Don't reintroduce:** OpRail, TopBar (merged), rail kbd hints, StatusBar `⌘K` pip, titlebar gear, StatusHero big H1, dupe "watching" words, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design tokens/Sync/Editor sections, `.btn.lg`/`.pill.warn`/`.pill.xs`/`.vdivider`/`.count-pip.warn` dead CSS, `bg-backlog.sh`, the `diag_*` Tauri command names (renamed to `sync_*` in S44), local `pulling/pushing/scanning` flags in TabRail (replaced w/ `syncModal.busy`).

**Flagged v0.2.40+:** `local_list_dir` profile containment (needs frontend contract change), log redaction / CSP / capability tightening (product policy), safe file-count cache (watch-level invalidation design), tunnel per-connection cancellation (cross-cuts task ownership).

---

## CRITICAL DON'T-TOUCH

- russh `ring` backend + reqwest `rustls` only (NASM blocks aws-lc-rs).
- `~/.rift/*.json` compat — don't change rename rules; keep `serde(flatten) extra`.
- `VelopackApp::build().run()` MUST be first call in `lib.rs::run()`.
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver).
- DriftWatcher conflict-rename guard — never overwrite dirty local.
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it.
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo.
- `path_guard.rs` API frozen — `edit/in_place.rs` + lib cmds depend.
- `rename_via` is strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- Source `.secrets/env.sh` first on ship/auth tasks — non-interactive bash won't auto-load.
- `last_scan_entries` is `std::sync::Mutex` (NOT tokio) — called from sync notify handler; tokio `blocking_lock` panics.
- `force_pull_now` does inline drift scan + dispatches w/ guard in front. Don't "optimize" to cache-only — stale tombstones fire mass deletes.
- `force_push_now` (v0.2.43+) promotes scan ToPush → dirty, auto-scans on cold cache, clears cache after non-cancelled push. Don't simplify back to "drain dirty only."
- **NEVER `FileAttributes::default()` for SETSTAT** — sends zeros → file truncation + epoch mtime. Use `FileAttributes::empty()` + set only fields you want. (v0.2.27 post-mortem.)
- `SftpClient::delete` routes by remote stat — dirs go through `delete_recursive_via`. Don't shortcut to `remove_file` for "files only." (v0.2.29.)
- `mkdir_p_via` chmods each segment to 2775 — setgid + group-writable required for shared-group teammate pushes. Don't drop SETSTAT. (v0.2.31.)
- Upload pre-flight SHA-collapse before raising CONFLICT — sizes match + baseline SHA → hash local cheap + remote via SSH exec; both = baseline → refresh baseline mtime + drop push. (v0.2.32 — 53 phantom conflicts.)
- `DriftBucket::ToDelete` = `local + no remote + has_baseline`. Routes to `delete_local_one` (guards on foreign-lock + dirty-local + empty-parent walk-up). (v0.2.33.)
- All time displays MUST pass `[], { hour12: true }` — locale-default emits 24h on non-US. (v0.2.34.)
- Mass local-delete circuit breaker: `(file_count * 0.30).clamp(5, 25)`, `BLOCKED — N local-deletes`, `kind=block`. Pull is the ONLY tombstone-propagation path post-auto-sync rip. (v0.2.36 / v0.2.38.)
- `spawn_frontend_pump` rate-limits at 200/s — critical stages (DriftScanStart/Result, RescanSignal, SftpConnect/Disconnect, RemoteScanResult, BridgeAck, System) ALWAYS bypass. Don't fold them back into the throttled path. (v0.2.43 post-mortem.)
- russh `Config { keepalive_interval: Some(20s), keepalive_max: 3, .. }` in both `sftp::open_session` + `tunnel::start`. Don't revert to `default()` — half-dead TCP sockets hang Windows ~2hr otherwise. (v0.2.42.)
- DO NOT restore `drift_watcher::spawn` / `run_tick` / `flush_cycle` / `auto_flush_enabled` / `remote_scan_interval_secs` / `drift_watcher_task` / `flush_task` / `LOOP_TICK_MS` / `track_pull_handle` / `register_scan_cancel` / `clear_scan_cancel` — deleted v0.2.38. Auto path ping-ponged. Push/Pull buttons only. Watcher still runs to populate dirty queue.
