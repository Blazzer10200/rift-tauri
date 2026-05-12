# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`.

## Session 52 — 2026-05-12 — FiveM web/build false-deletes + Created+Dir debounce → ship v0.2.48-alpha

### Trigger
v0.2.47 stress test surfaced two issues. Bug 7: 45 phantom ToDelete-local rows on FiveM `web/build/` + `web/dist/` trees, SSH-verified files existed on prod intact — destructive on apply. Bug 5 still unresolved: `endure_rifttest` (7 files) never appeared in any push diff even after Modified-event touches + Rescans.

### Diagnosis
Bug 7 = filter asymmetry. `sftp/list.rs::list_via_exec` builds `find ... -name build -o -name dist ... -prune` from `ignored_directory_names()`; server-side prune can't see path context, kills FiveM web bundles. Local walker uses path-aware `classify()` which bypasses `/web/build/` + `/web/dist/`. Remote_map has zero, local_map has files, drift = ToDelete.

Bug 5 = v0.2.47's immediate `kick_drift_reconcile` on Create(Dir) fires BEFORE Windows finishes writing the files inside the new tree. Scan walks an empty dir, surfaces nothing. Modified events afterward never re-trigger reconcile because the dir's already-known to the watcher.

### Completed (file:line)
- **Bug 7 fix** [sync/ignore.rs::ignored_directory_names()](src-tauri/src/sync/ignore.rs) — exclude `build` and `dist` from server-prune list. Drift scanner's post-listing `should_ignore(rel)` filter handles generic `app/build/foo` paths client-side w/ path context. ~32 fonts of extra find traffic per FiveM web build; trivial.
- **Bug 5 fix** [sync/auto_sync.rs::on_fs_event](src-tauri/src/sync/auto_sync.rs) — wrap kick_drift_reconcile in 500 ms tokio::sleep + new `pending_dir_reconcile: AtomicBool` coalesce flag. compare_exchange owns dispatch slot; lost-race events skip. 50 rapid Create(Dir) events collapse to one delayed scan.
- **Tests** — new assertions on `ignored_dir_names_excludes_brackets` confirm `build` + `dist` excluded. `cargo test --lib sync::ignore` 11 pass / 0 fail. `cargo check` clean 2.89s.

### Files Modified (S52)
`src-tauri/src/sync/ignore.rs` — `ignored_directory_names` excludes build/dist + test asserts. `src-tauri/src/sync/auto_sync.rs` — `pending_dir_reconcile` AtomicBool field + 500 ms delayed dispatch in on_fs_event. `package.json` + `Cargo.toml` + `tauri.conf.json` 0.2.47 → 0.2.48. `docs/CHANGELOG.md` v0.2.48 entry.

### Validation gates user will check
1. Rescan post-update → 45 web/build false-deletes gone.
2. `endure_rifttest` (7 files local, 0 remote) finally surfaces as 7 ToPush under [endure] after a fresh Rescan or after recreating the dir.

### Still deferred to v0.2.49
Mirror mode (Bug 1) — new drift bucket for `local-missing + remote-has + baseline-exists` → propose remote-delete + Mirror toggle in Sync hero. Stale-lock sweep UI button. Mass-delete guard fine-tune.

### Audit notes — Treyday safety
Bug 7 fix is read-side only; never causes deletes. Bug 5 fix is read-side scan trigger; never destructive. setgid 2775 mkdir, foreign-lock check, dirty-local guard, keepalive 20s×3, conflict-rename guard all preserved.

---

## Session 51 — 2026-05-12 — New-resource discovery + park-dir ignore → ship v0.2.47-alpha

### Trigger
Post-v0.2.46 Endure RP stress test surfaced two new bugs:
- **Bug 5**: created `[endure]/endure_rifttest/` (7 files / 13 nested dirs) — 0 push entries 10+ min later. Bracket-level `.rift-trail.jsonl` proved Rift WAS watching the bracket; sibling `endure_devbridge` worked.
- **Bug 6**: after SSH-cleaning prod into `_disabled_extras/`, Rescan still surfaced 183 pull / 43 delete on `[depend]` — out-of-band moves not reconciling.

### Diagnosis
- Bug 5: bracket DOES get `RecursiveMode::Recursive` watch via `try_watch` at try_watch:427. But on Windows `ReadDirectoryChangesW` can race new-subtree registration — file Create events nested inside a freshly-created dir BEFORE the dir fully registers get silently dropped. The dir-Create event itself reaches us but `queue_path` returns early for dir paths.
- Bug 6: NOT a snapshot bug. Drift scanner re-walks remote live every Rescan via `list_recursive_batch`. Parking into `_disabled_extras/` under the watched remote_root means the listing still walks them; drift scanner emits ToPull because local doesn't have those files. Working as designed; ignore rules just didn't cover the prefix.

### Completed (file:line)
- **F5 — Created+Dir kicks reconcile** [sync/auto_sync.rs:1041-1054](src-tauri/src/sync/auto_sync.rs#L1041) — on `Create(Dir)` in `on_fs_event`, fire `kick_drift_reconcile` as Windows-race safety net. Cancel-replace token semantics debounce rapid bursts to one scan.
- **F6 — Prefix-match `_disabled_*`** [sync/ignore.rs](src-tauri/src/sync/ignore.rs) — new `IGNORE_SEGMENT_PREFIXES = ["_disabled_"]` applied only to non-terminal segments (so a file literally named `_disabled_for_review.lua` doesn't false-trip). Existing `_disabled_archive` exact match preserved.
- **Tests** — new `disabled_prefix_segments` covers park-dir paths + false-trip guard. `cargo test --lib sync::ignore` → 11 passed / 0 failed.
- `cargo check` clean 1.44s.

### Files Modified (S51)
- `src-tauri/src/sync/auto_sync.rs` — `on_fs_event` Created+Dir reconcile dispatch
- `src-tauri/src/sync/ignore.rs` — IGNORE_SEGMENT_PREFIXES + non-terminal-segment gate + new test
- `package.json` + `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json` — 0.2.46 → 0.2.47-alpha
- `docs/CHANGELOG.md` — v0.2.47 entry (v0.2.46 falls to git log)

### Still deferred to v0.2.48
- **Mirror mode** (Bug 1) — new drift bucket for `local-missing + remote-has + baseline-exists` → propose remote-delete + UI toggle.
- **Stale-lock sweep UI button** — existing `sweep_stale_mine` (180 s) handles own-user stale on watcher attach.
- **Mass-delete guard fine-tune** — still open from v0.2.45 backlog.

### Next session priorities
1. Smoke v0.2.47 — create new resource dir under a bracket, verify it appears in push queue within a few seconds (vs 10+ min before).
2. Smoke park-dir ignore — `mv` some files into a fresh `_disabled_temp/` under watched root, run Rescan, confirm they disappear from the pull queue.
3. Treyday Tailscale round-trip on v0.2.47.

### Audit notes — Treyday safety
All v0.2.46 push-reliability + lock-release fixes preserved. Reconcile dispatch on Created+Dir is read-only (drift scan), no destructive ops. Prefix ignore rule is restrictive (non-terminal segments only); won't ignore any file under a non-park dir.

---

## Session 50 — 2026-05-12 — Push reliability + orphan-lock fix → ship v0.2.46-alpha

### Trigger
Cross-session report from Endure RP FiveM server work: 4 production bugs after a 16-resource bulk-replace + `[depend]/` bracket reorg via Push-all:
1. Push doesn't propose remote-deletes for local-orphans (UX/intent).
2. Push **silently drops files** — `[depend]/oxmysql/` (~50 files) missing entirely; `ox_doorlock/fxmanifest.lua`, `ox_fuel/{config,client/init,data/stations}.lua`, `qbx_seatbelt/data/seatbelt_sounds.dat54.rel` all dropped without surfacing the failure path-by-path.
3. 44 orphan `.rift-lock` files across the prod tree (Pattern A tmp-locks, B dir-locks, C stream-locks).
4. Directory-creation has a separate broken path — new `[depend]/oxmysql/` left an orphan `.rift-lock` and zero files.

User did manual SSH cleanup; needed code fix to prevent recurrence.

### Completed (file:line)
- **F1 — Strict mkdir** [sftp/ops.rs:185-220](src-tauri/src/sftp/ops.rs#L185) — `mkdir_p_strict_via` probes metadata after each failed `create_dir`. Real failures propagate; "already exists as dir" stays idempotent.
- **F1.b — Upload uses strict** [sftp/transfer.rs:242-252](src-tauri/src/sftp/transfer.rs#L242) — `upload_atomic_via` surfaces parent-mkdir failure as `OpResult::err("mkdir parent X: ...")` instead of letting downstream `sftp.create(&tmp)` mask it.
- **F2 — Lock release on every terminal result** [sync/auto_sync.rs:1876-1925](src-tauri/src/sync/auto_sync.rs#L1876) — `process_entry` wrapper releases on Ok AND Fail; Requeued preserves it. Inner success-path release stays a no-op (idempotent). Kills the 44-orphan-lock leak vector.
- **F3 — No locks for directory paths** [sync/auto_sync.rs:1685-1710](src-tauri/src/sync/auto_sync.rs#L1685) — `queue_path` gates `locks.acquire(...)` on `path.is_file()`. Windows `notify` firing Modified on parent dirs no longer leaks `<dir>.rift-lock`.
- **F4 — Batch pre-mkdir** [sync/auto_sync.rs:1755-1800](src-tauri/src/sync/auto_sync.rs#L1755) — `flush_batch` collects unique parents and serializes mkdir on main session BEFORE the parallel upload loop. Eliminates the 50-worker race on fresh `[depend]/oxmysql/` tree.
- **F7 — `wait_for_readable` 3.2 s exp backoff** [sync/auto_sync/path.rs:69-87](src-tauri/src/sync/auto_sync/path.rs#L69) — 200 ms → 6 × 50/100/200/400/800/1600 ms. Fixes the 48-skipped-file pattern from v0.2.45 mass-push.
- **`pub(crate) mod ops`** [sftp/mod.rs:41](src-tauri/src/sftp/mod.rs#L41) — exposes `remote_parent` to flush_batch.
- `cargo check` clean 4.22s.

### Files Modified (S50)
- `src-tauri/src/sftp/ops.rs` — `mkdir_p_strict_via` + `mkdir_p_inner` + `pub(crate)` on `remote_parent`
- `src-tauri/src/sftp/transfer.rs` — `upload_atomic_via` uses strict mkdir, surfaces error
- `src-tauri/src/sftp/mod.rs` — `pub(crate) mod ops;`
- `src-tauri/src/sync/auto_sync.rs` — F2 wrapper release, F3 is_file gate, F4 batch pre-mkdir
- `src-tauri/src/sync/auto_sync/path.rs` — wait_for_readable exp backoff
- `package.json` + `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json` — 0.2.45 → 0.2.46-alpha
- `docs/CHANGELOG.md` — v0.2.46 entry (v0.2.45 falls to git log)

### Deferred to v0.2.47
- **Mirror mode for Push-all** (Bug 1) — needs new drift bucket for `local-missing + remote-has + baseline-exists` → propose-remote-delete, plus UI toggle. Significant classification + frontend work.
- **Stale-lock sweep UI button** — existing `sweep_stale_mine` (180 s threshold) already catches own-user stale locks on watcher attach. Manual UI is nice-to-have.

### Next session priorities
1. Smoke v0.2.46 on Blazzer + Treyday over Tailscale. Specifically: create new bracket dir w/ ≥50 fresh files, push, verify every file lands. Concurrent edit-during-push to validate lock-release fix.
2. Mirror mode design — UX decision: dedicated "Mirror" mode toggle vs always-show-remote-orphans-as-ToDelete?
3. v0.2.45's mass-delete guard fine-tune (still open from S49).

### Audit notes — Treyday safety
All v0.2.45 safety preserved: foreign-lock check, dirty-local check, setgid 2775 mkdir mode for shared-group teammate pushes, 20s × 3 keepalive on both russh sessions, conflict-rename guard in `pull_one`. New strict mkdir applies setgid 2775 same as before. New `is_file()` gate doesn't affect file uploads.

---

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
