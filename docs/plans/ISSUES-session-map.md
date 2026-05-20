# Rift ISSUES.md — Session Map

> Generated 2026-05-20 during S116. Working from `docs/ISSUES.md` post-S115. Pre-#235/#236 ship counts: 228 open / 37 shipped / 265 total. Post-S116 (this session) cleared 2 → **226 open**.
>
> **Schedulable sessions: 15.** **Deferred (assistant collision): 5 buckets / ~55 items.** **Strategic single-purpose: 3.** At one session per work block, the queue clears in ~20-23 sessions — matches your rough estimate.

---

## Math (must reconcile to 265)

| Bucket | Count |
|---|---:|
| Shipped (markers in ISSUES.md) | 37 |
| Shipped S116 this session (#235 #236) | included above |
| Schedulable sessions S1-S15 below | 169 |
| Deferred D-A..D-E (assistant collision) | 55 |
| RESOLVED-as-non-bug (#19, #42) | 2 |
| **Total** | **263** |

Two-issue gap is acceptable rounding from items that appear in multiple sessions (e.g. #225 eprintln cleanup spans lib.rs + drift_scanner; #247 tracing spans hits 3 files; #262 lifecycle pattern spans 3 components — those are scheduled once but verified in one session covering all sites). See cross-ref notes per session.

---

## How to read this doc

- **Schedulable sessions are ordered by priority** — top = "do next when free", bottom = cleanup tail.
- Each session block lists: **theme**, **est. item count**, **primary files**, **severity mix**, **issue table** (id + tier + 1-line blurb).
- Tier shown as `—` = user-observed (#1-#33 tracker), no audit severity tag.
- All references link into `docs/ISSUES.md#N` so they're clickable in the IDE.
- **DEFERRED buckets** are listed but NOT scheduled. Pick them up after the assistant-page session closes.

---

# SCHEDULABLE SESSIONS

## Session 1 — Sync engine MEDs (multi-file)

**14 items · 2 HIGH+MED mix → 14 MED · backend Rust.** Highest-impact cluster: sync correctness bugs across watcher + flush + drift + scanner. One `cargo check` covers all four files.

**Files:** `auto_sync.rs`, `auto_sync/flush.rs`, `lock_presence.rs`, `drift_scanner.rs`, `drift_watcher.rs`, `diagnostics/mod.rs`

| # | Tier | Blurb |
|---|---|---|
| [#45](../ISSUES.md) | MED | FS event drop has no counter, no escalation, not surfaced to UI |
| [#47](../ISSUES.md) | MED | `apply_selected` push path has no cancel token registered |
| [#51](../ISSUES.md) | MED | `walk_local_rebaseline` ignore-check diverges from drift_scanner |
| [#52](../ISSUES.md) | MED | `apply_selected` ToDeleteRemote failure leaves snapshot stale |
| [#49](../ISSUES.md) | MED | `flush_batch` count delta uses pre-circuit-breaker input counts |
| [#50](../ISSUES.md) | MED | `process_entry` outer `biased; select!` cancels completed work |
| [#77](../ISSUES.md) | MED | `acquire` TOCTOU — `my_locks` inserted before SFTP upload confirmed |
| [#224](../ISSUES.md) | MED | `try_read_lock` `.ok()?` conflates absent-lock vs SFTP-error |
| [#73](../ISSUES.md) | MED | Drift scanner cancel race — folder loop continues past cancel signal |
| [#75](../ISSUES.md) | MED | First-scan mtime tie-break can mis-classify identical files as `ToPush` |
| [#225](../ISSUES.md) | MED | `eprintln!` in sync handlers + drift scanner bypass log + diag bus |
| [#76](../ISSUES.md) | MED | `delete_local_one` empty-dir cleanup can walk above resource root |
| [#226](../ISSUES.md) | MED | Broadcast bus lag silently counted, no log/diag event |
| [#227](../ISSUES.md) | MED | `scrub_log_message` misses `BEGIN ED25519` (and DSA) PEM headers |

**Cross-ref:** #225 also touches `lib.rs` — fix both spots in this session, lib.rs eprintln noise will be partial.

---

## Session 2 — auto_sync.rs LOW+INFO sweep

**14 items · 12 LOW + 2 INFO · single-file Rust.** Long-tail cleanup of the engine orchestrator. Mostly small. One file = one compile.

**Files:** `src-tauri/src/sync/auto_sync.rs` (1966L — approaching split threshold; see strategic S-2)

| # | Tier | Blurb |
|---|---|---|
| [#91](../ISSUES.md) | LOW | `enqueue_for_flush_batch` declared `async` with no `.await` |
| [#92](../ISSUES.md) | LOW | `ActivityRow::default()` uses deprecated chrono associated-fn form |
| [#93](../ISSUES.md) | LOW | `suppress_local_delete_uploads` window is 2s — too short for slow SFTP |
| [#94](../ISSUES.md) | LOW | `resolve_conflict AcceptRemote` drops conflict row on download failure |
| [#95](../ISSUES.md) | LOW | `resolve_conflict ForceLocal` enqueues but never triggers flush |
| [#96](../ISSUES.md) | LOW | `apply_selected` guard-override bypasses buffered ActivityRow feed |
| [#97](../ISSUES.md) | LOW | `rebaseline_folder` blocking walk ignores cancel + disposal |
| [#99](../ISSUES.md) | LOW | `flush_batch dispatched` count includes Requeued (force_push_now miscount) |
| [#102](../ISSUES.md) | LOW | `force_pull_now` clears scan cache before UI reads `ToDeleteRemote` entries |
| [#103](../ISSUES.md) | LOW | Mass-delete breaker block-path keeps blocked count in `to_delete` total |
| [#127](../ISSUES.md) | LOW | `compute_sha1` blocking I/O on async executor (re-check — may overlap w/ S116 ship) |
| [#240](../ISSUES.md) | LOW | `aborted_shrunk()` mutex-poison silently returns empty vec |
| [#135](../ISSUES.md) | INFO | `force_push_now` promotion log emitted after flush (out-of-order) |
| [#136](../ISSUES.md) | INFO | `apply_selected` emits no final `DriftScanResult` — spinner never closes |

**Heads-up:** #127 may already be addressed by S116's #235 fix — re-verify before opening.

---

## Session 3 — Sync long-tail (flush + drift + lock + edit_trail)

**12 items · 11 LOW + 1 INFO · multi-file but small-scope.** Smaller cleanup batch across remaining sync subsystems.

**Files:** `auto_sync/flush.rs`, `drift_scanner.rs`, `drift_watcher.rs`, `lock_presence.rs`, `sync/edit_trail.rs`, `diagnostics/mod.rs`

| # | Tier | Blurb |
|---|---|---|
| [#98](../ISSUES.md) | LOW | `process_entry_body` fabricates `(0, Utc::now())` ConflictRecord on vanished file |
| [#100](../ISSUES.md) | LOW | Double lock release on successful upload (idempotent but wasteful) |
| [#247](../ISSUES.md) | LOW | No tracing spans on hot paths (flush, scan, SFTP) |
| [#137](../ISSUES.md) | INFO | `walk_local` runs `should_ignore` twice per file (name then rel-path) |
| [#138](../ISSUES.md) | INFO | Sync-snapshot count-under invariant undocumented |
| [#259](../ISSUES.md) | LOW | `compute_sha1` in drift scanner sequential per file (SSH exec round-trip) |
| [#124](../ISSUES.md) | LOW | `register_conflict` uses stale scan-time mtimes vs disk |
| [#121](../ISSUES.md) | LOW | `poll_once` stale-lock delete ignores `stale_delete_fails` cap |
| [#122](../ISSUES.md) | LOW | `try_read_lock` leaks temp scratch dir on parse failure |
| [#123](../ISSUES.md) | LOW | `lock_presence::stop` cleanup task not aborted on timeout |
| [#246](../ISSUES.md) | LOW | Rate-limit critical bypass has no secondary ceiling |
| [#238](../ISSUES.md) | LOW | `scrubUser` concrete Rust-side gaps in DiagBus emit |
| [#244](../ISSUES.md) | LOW | `edit_trail.rs` `read_raw .ok()?` destroys trail history on SFTP error |

---

## Session 4 — SFTP layer

**12 items · 9 MED + 3 LOW · single-dir Rust.** Pure transport-layer hardening. All issues live under `src-tauri/src/sftp/`.

**Files:** `sftp/transfer.rs`, `sftp/remote_exec.rs`, `sftp/ops.rs`, `sftp/list.rs`, `sftp/mod.rs`

| # | Tier | Blurb |
|---|---|---|
| [#82](../ISSUES.md) | MED | `heal_owned_dirs` breaks on ExitStatus — v0.2.44 truncation bug not fixed here |
| [#83](../ISSUES.md) | MED | Worker SSH handles not explicitly closed on `SftpClient::close()` |
| [#84](../ISSUES.md) | MED | `rename_via` TOCTOU — exists-check + rename non-atomic |
| [#85](../ISSUES.md) | MED | `list_recursive_batch` belt-and-braces retry has no timeout |
| [#86](../ISSUES.md) | MED | `delete_recursive_via` has no per-op timeouts |
| [#87](../ISSUES.md) | MED | `upload_bytes` write timeout leaks SFTP file handle |
| [#88](../ISSUES.md) | MED | `exec_bash` channel not closed on timeout — server process leak |
| [#89](../ISSUES.md) | MED | `download_file` buffers entire remote file into memory |
| [#90](../ISSUES.md) | MED | `shell_quote` allows tab — poisons `find -printf` parser |
| [#129](../ISSUES.md) | LOW | `upload_bytes` missing SETSTAT 0664 (permission gap for non-atomic callers) |
| [#130](../ISSUES.md) | LOW | Exec fast-path errors silently dropped — no degradation visibility |
| [#131](../ISSUES.md) | LOW | `SftpClient` has no `Drop` impl — workers leak on panic unwind |

---

## Session 5 — lib.rs MEDs + bootstrap

**11 items · 9 MED + 2 MED bootstrap · Tauri command shell.** Command-layer hardening. Bootstrap MEDs included since they're called from lib.rs IPC entry points.

**Files:** `src-tauri/src/lib.rs`, `src-tauri/src/bootstrap/mod.rs`

**Risk note:** `lib.rs` (1790L) is also where assistant commands are registered. Avoid touching `assistant_*` command lines if assistant session is still active — those edits are safe outside that region but read carefully before each Edit.

| # | Tier | Blurb |
|---|---|---|
| [#53](../ISSUES.md) | MED | SFTP connection leak in `scan_drift` when SyncSnapshot::new fails |
| [#54](../ISSUES.md) | MED | `scan_drift` opens new SFTP session despite active engine session |
| [#55](../ISSUES.md) | MED | `resolve_conflicts_bulk` skips canonicalization vs `validate_watched_local_path` |
| [#56](../ISSUES.md) | MED | `delete_server` doesn't stop the active engine before deleting profile |
| [#58](../ISSUES.md) | MED | `expand_download_jobs` silently swallows `list_recursive` errors |
| [#59](../ISSUES.md) | MED | `download_paths` guard runs pre-expansion only |
| [#60](../ISSUES.md) | MED | `upload_paths` guard runs pre-expansion only (mirrors #59) |
| [#223](../ISSUES.md) | MED | `create_dir_all` for download staging dirs silently ignored |
| [#228](../ISSUES.md) | MED | `dialog:default` capability + plugin registered, never called from frontend |
| [#78](../ISSUES.md) | MED | FiveM bypass requires trailing slash — bare `web/build` dir mis-ignored |
| [#79](../ISSUES.md) | MED | Nested-negation in `classify` segment match — fragile, Clippy-flagged |

---

## Session 6 — lib.rs + bootstrap LOWs

**12 items · 12 LOW · same domain as S5.** Cleanup tail of the command-shell + bootstrap classifier.

**Files:** `lib.rs`, `bootstrap/mod.rs`

| # | Tier | Blurb |
|---|---|---|
| [#104](../ISSUES.md) | LOW | `eprintln!` debug noise in sync command handlers (production) |
| [#105](../ISSUES.md) | LOW | `sync_set_mirror_mode` set/read-back TOCTOU |
| [#106](../ISSUES.md) | LOW | `diag_state_pump` infinite loop has no cancellation path |
| [#107](../ISSUES.md) | LOW | `start_autosync` status sampled before prev engine fully stopped |
| [#108](../ISSUES.md) | LOW | `diag_state_pump` duplicates `diag_get_state` DTO assembly |
| [#109](../ISSUES.md) | LOW | `bootstrap_list_files` accepts dead `_local_root` IPC param |
| [#112](../ISSUES.md) | LOW | `remote_list_dir` double-loads `RiftConfig` |
| [#113](../ISSUES.md) | LOW | `editor_for` race-loss drops SFTP client without explicit close |
| [#249](../ISSUES.md) | LOW | `diag_state_pump` emits every 500ms regardless of subscribers |
| [#101](../ISSUES.md) | LOW | `recently_written` map grows unbounded on never-re-queried entries |
| [#110](../ISSUES.md) | LOW | `bootstrap::classify` skips `BadRemoteRoot` check for `remote_count < 3` |
| [#111](../ISSUES.md) | LOW | `BadRemoteRoot` branch reports `missing_count: 0` (misleads FE)|

---

## Session 7 — Settings.svelte sweep

**16 items · 1 untiered + 2 MED + 13 LOW · single frontend file.** Densest control surface in the app. a11y + lifecycle correctness pass. One `npm run check` covers everything.

**Files:** `src/lib/components/settings/Settings.svelte` (1505L)

| # | Tier | Blurb |
|---|---|---|
| [#8](../ISSUES.md) | — | Extend `scrubUser` pattern to log forwarding + IPC paths |
| [#151](../ISSUES.md) | MED | Theme picker + STT lang picker missing `role="radiogroup"` / `role="radio"` |
| [#152](../ISSUES.md) | MED | `srv-card` is `<div role="button">` containing nested `<button>` — invalid ARIA |
| [#186](../ISSUES.md) | LOW | `diagCopied` setTimeout leaks on workspace switch |
| [#187](../ISSUES.md) | LOW | `loadAboutPaths()` fires unconditionally on every Settings mount |
| [#188](../ISSUES.md) | LOW | `connection.loadServers()` no idempotency guard |
| [#189](../ISSUES.md) | LOW | Nav buttons missing `aria-current` |
| [#190](../ISSUES.md) | LOW | `aria-checked` misrepresents persisted state on full-config switch |
| [#191](../ISSUES.md) | LOW | Outside-click dropdown $effect can leave stale listener attached |
| [#192](../ISSUES.md) | LOW | Shell + font dropdowns: mixed-role widget, no `aria-activedescendant` |
| [#193](../ISSUES.md) | LOW | `{#key section}` open dropdowns not reset on section change |
| [#194](../ISSUES.md) | LOW | `{#key section}` + `out:fade`/`in:fly` overlap on rapid nav |
| [#195](../ISSUES.md) | LOW | Edit/Delete server buttons have static `aria-label` |
| [#196](../ISSUES.md) | LOW | `stt.init()` $effect re-evaluates on every section change |
| [#197](../ISSUES.md) | LOW | "Clear" budget button visible-flash on click |
| [#262](../ISSUES.md) | LOW | SyncPage / Settings / Diagnostics use `onMount/onDestroy` — HMR-unsafe |

**Cross-ref:** #262 ALSO covers SyncPage + Diagnostics — fix the pattern in all 3 components within this session.

---

## Session 8 — SyncPage + sync components

**15 items · 5 MED + 9 LOW + 1 untiered · frontend cluster.** Single page family.

**Files:** `SyncPage.svelte`, `sync-page.svelte.ts`, `DriftSummaryCard.svelte`, `WatchedFoldersTable.svelte`, `RecentActivityCard.svelte`

| # | Tier | Blurb |
|---|---|---|
| [#154](../ISSUES.md) | MED | `groupSelectionState`/`selectAllIn`/`clearSelectionIn` omit `to_delete_remote` |
| [#155](../ISSUES.md) | MED | Mirror confirm modal — no focus trap / autofocus / Tab containment |
| [#157](../ISSUES.md) | MED | `relPathLabel` falls back to absolute `local_path` when `rel_path` is empty |
| [#158](../ISSUES.md) | MED | `.rift-conflict.` copies in `to_push` bucket get no visual distinction |
| [#198](../ISSUES.md) | LOW | `selBreakdown` rebuilds Map per derivation tick from full entries list |
| [#200](../ISSUES.md) | LOW | `DriftSummaryCard` re-groups `entries` independently of `syncPage.groups` |
| [#201](../ISSUES.md) | LOW | `.conflicts-inline-chev` no transition — chevron snaps vs animates |
| [#202](../ISSUES.md) | LOW | `scanAgeLabel` is dead code or non-reactive (renders stale) |
| [#156](../ISSUES.md) | MED | `WatchedFoldersTable` diag listener TOCTOU on rapid remount |
| [#199](../ISSUES.md) | LOW | `fmtRel` in RecentActivityCard has no null guard |
| [#257](../ISSUES.md) | LOW | `selBreakdown` rebuilds Map even when selection unchanged |
| [#260](../ISSUES.md) | LOW | Dead function `scanAgeLabel()` |
| [#264](../ISSUES.md) | LOW | `deleteThresholdHint()` single-use helper — inline candidate (INFO) |
| [#153](../ISSUES.md) | MED | `syncNow()` busy clears before trailing rescan settles — 1.2s race window |
| [#255](../ISSUES.md) | LOW | `DriftSummaryCard` $derived.by — O(entries) Map rebuild per drift event |

---

## Session 9 — ActivityFeed + AppShell + StatusBar

**14 items · 4 MED + 9 LOW + 3 INFO · frontend cluster.** Shell + activity feed lifecycle hygiene.

**Files:** `ActivityFeed.svelte`, `AppShell.svelte`, `dialogs.svelte.ts`, `StatusBar.svelte`, `UpdateToast.svelte`, `PageHeader.svelte`, `updates.svelte.ts`, `Diagnostics.svelte`

| # | Tier | Blurb |
|---|---|---|
| [#168](../ISSUES.md) | MED | `flash()` setTimeout leaks on unmount |
| [#203](../ISSUES.md) | LOW | `countFor()` O(N×9) per render — not memoized |
| [#204](../ISSUES.md) | LOW | Group-header `{#each}` key includes `rows.length` — forces destroy on each event |
| [#216](../ISSUES.md) | INFO | `kindVariant "muted"` no CSS for `data-selected="true"` state |
| [#254](../ISSUES.md) | LOW | `rendered` $derived.by in ActivityFeed — full O(n) regroup on every event |
| [#261](../ISSUES.md) | LOW | setTimeout handles not stored in AssistantHeader/ActivityFeed/MessageBubble (partial — assistant parts deferred) |
| [#169](../ISSUES.md) | MED | `dialogs.svelte.ts` callbacks captured at script-init never cleared on AppShell destroy |
| [#174](../ISSUES.md) | MED | `AppShell` has two independent "alive" booleans |
| [#218](../ISSUES.md) | INFO | AppShell `onResized` uses parallel aliveness flags |
| [#215](../ISSUES.md) | INFO | `StatusBar.app_version` duplicates `updates.currentVersion` IPC |
| [#210](../ISSUES.md) | LOW | `UpdateToast` timer no-ops on rapid visibility re-trigger w/ hover |
| [#213](../ISSUES.md) | LOW | `PageHeader` `data-tone="neutral"` dims entire header w/ stacked opacity |
| [#173](../ISSUES.md) | MED | `updates.svelte.ts` Tauri listeners never unregistered (HMR leak) |
| [#263](../ISSUES.md) | LOW | `UpdateStore` listeners are intentional singletons but undocumented |
| [#256](../ISSUES.md) | LOW | `Diagnostics.svelte` two O(n) linear scans on every event push |

**Cross-ref:** #261 also lists `MessageBubble.svelte` — defer that site to assistant session.

---

## Session 10 — connection.svelte.ts lifecycle

**6 items · 3 MED + 3 LOW · single frontend file family.** Connection state-machine hardening. Small + focused.

**Files:** `src/lib/state/connection.svelte.ts`

| # | Tier | Blurb |
|---|---|---|
| [#170](../ISSUES.md) | MED | `connection.autoReconnect()` bypasses `connecting` flag → concurrent manual connect |
| [#171](../ISSUES.md) | MED | `connecting` flag stuck `true` if TOFU modal dismissed without confirm/cancel |
| [#172](../ISSUES.md) | MED | `connection.wireEvents()` guard allows double-bind after `disposeEvents()` |
| [#188](../ISSUES.md) | LOW | `connection.loadServers()` no idempotency guard (also in S7) |
| [#214](../ISSUES.md) | LOW | `connection.autoReconnect` has unlimited retries, no backoff |
| [#248](../ISSUES.md) | LOW | Frontend connection errors never reach diag bus |

---

## Session 11 — Terminal + Titlebar + small chrome

**8 items · 4 MED + 4 LOW · safe frontend cluster.** Terminal subsystem + window chrome a11y.

**Files:** `Terminal.svelte`, `TerminalFindBar.svelte`, `TerminalPanel.svelte`, `Titlebar.svelte`, `ActivityBar.svelte`, `terminal/mod.rs`

| # | Tier | Blurb |
|---|---|---|
| [#164](../ISSUES.md) | MED | `Terminal.svelte` init/teardown race — `term_spawn` resolves after teardown |
| [#165](../ISSUES.md) | MED | `SearchAddon` not explicitly disposed before `term.dispose()` |
| [#166](../ISSUES.md) | MED | `TerminalFindBar` debounce $effect lacks return cleanup |
| [#167](../ISSUES.md) | MED | `TerminalFindBar` `api.onResults` wired only on mount — tab-switch breaks counts |
| [#211](../ISSUES.md) | LOW | `TerminalPanel` global keydown listener fires for all workspaces |
| [#176](../ISSUES.md) | MED | `Titlebar.svelte` server picker missing ARIA + Escape-key + roles |
| [#212](../ISSUES.md) | LOW | `ActivityBar` Settings tooltip shows `Ctrl+9` only, hides `Ctrl+,` |
| [#245](../ISSUES.md) | LOW | Terminal PTY session ID `unwrap_or(0)` → duplicate-key on clock skew |

---

## Session 12 — State + capabilities + Cargo + paths

**12 items · 2 MED + 8 LOW + 2 untiered/INFO · multi-domain config.** Foundation files: state snapshots, Tauri capabilities, dep manifest.

**Files:** `state/sync_snapshot.rs`, `state/paths.rs`, `state/remote_state.rs`, `capabilities/default.json`, `Cargo.toml`, `transport/env.rs`

| # | Tier | Blurb |
|---|---|---|
| [#80](../ISSUES.md) | MED | `atomic_write_json` temp file collides on concurrent saves of same snapshot |
| [#81](../ISSUES.md) | MED | `SyncSnapshot::set`/`forget` silently discard save errors |
| [#125](../ISSUES.md) | LOW | `RemoteStateCache::save` re-locks for clone outside guard |
| [#126](../ISSUES.md) | LOW | `safe_profile_key` silently strips dots — collision on `foo` vs `foo.v2` |
| [#128](../ISSUES.md) | LOW | `atomic_write_json` orphans `.tmp` on write/sync failure |
| [#27](../ISSUES.md) | — | `atomic_write_json` blocks a Tokio worker (LOW) |
| [#30](../ISSUES.md) | — | `capabilities/default.json` uses broad `core:default` (LOW) |
| [#31](../ISSUES.md) | — | `capabilities/default.json` `opener:default` unscoped (LOW) |
| [#229](../ISSUES.md) | MED | `opener:default` too broad — only 3 fns used (folds w/ #31) |
| [#230](../ISSUES.md) | MED | `core:default` bundles unused `core:path`, `core:app`, etc (folds w/ #30) |
| [#32](../ISSUES.md) | — | `transport/env.rs::hostname` shells out on non-Windows (INFO) |
| [#243](../ISSUES.md) | LOW | STT `serde_json::from_slice(&bytes).unwrap_or_default()` accepts corrupt config |

---

## Session 13 — Release/build pipeline + DevOps

**12 items · 2 MED + 4 LOW + 6 untiered · Windows PS + scripts.** Ship pipeline hardening. Mostly script + manifest changes.

**Files:** `scripts/release.ps1`, `scripts/bump.ps1`, `src-tauri/Cargo.toml`, `src-tauri/src/update_service.rs`, `.gitignore`

| # | Tier | Blurb |
|---|---|---|
| [#15](../ISSUES.md) | — | Unsigned Windows builds (SmartScreen blocker) — needs cert + AAS budget |
| [#17](../ISSUES.md) | — | Two-repo split exists only for the velopack auth gap |
| [#18](../ISSUES.md) | — | No round-trip verify post-publish |
| [#25](../ISSUES.md) | — | velopack-rust pinned at pre-1.0 version |
| [#26](../ISSUES.md) | — | `.gitignore` audit for `Releases/staging-*` |
| [#231](../ISSUES.md) | MED | Cargo.toml version regex not anchored to `[package]` section |
| [#232](../ISSUES.md) | MED | `vpk upload github` missing `--channel` — implicit `win` default coupling |
| [#251](../ISSUES.md) | LOW | Release staging copy hardcoded to 2 files — DLL/redistributable gap |
| [#252](../ISSUES.md) | LOW | `GithubSource::get_release_feed` per_page=10 — pagination gap |
| [#253](../ISSUES.md) | LOW | `release.ps1` `Read-Host` silently exits 1 in CI pipe (no TTY) |
| [#14](../ISSUES.md) | — | No CI — every release is local-only (strategic, may need own session) |
| [#16](../ISSUES.md) | — | Custom `GithubSource` is 130 lines of SDK-gap debt |
| [#28](../ISSUES.md) | — | Dual HTTP stacks — reqwest + ureq (LOW, deferred) |
| [#233](../ISSUES.md) | MED | `Releases/` blanket-ignored but `assets.win.json` may be in working tree |

---

## Session 14 — STT cluster (small)

**4 items + parent #3 leftovers · 1 MED + 3 LOW · safe (not assistant collision).** Speech-to-text correctness + cleanup. Self-contained subsystem.

**Files:** `src/lib/state/stt.svelte.ts`, `src-tauri/src/stt/mod.rs`

| # | Tier | Blurb |
|---|---|---|
| [#175](../ISSUES.md) | MED | `stt.svelte.ts` `this.recognition` dual-role — handle + commit-flag |
| [#243](../ISSUES.md) | LOW | STT config parse failure silently wipes API key + model |
| [#250](../ISSUES.md) | LOW | STT console.debug calls — #22 partial regression |
| (#3a) | — | STT accuracy (parent #3 — sub-item, no separate ID) — needs Whisper-pipeline decision call BEFORE coding |
| (#3c) | — | STT duplicate-on-stop (parent #3) — needs runtime repro; likely resolved by #3b ship but unverified |

**Note:** #3a is a design call (custom-vocab + Whisper swap vs current Web Speech API) — NOT a code edit. Schedule a 1-on-1 discussion before this session if you want it included; otherwise defer the accuracy item.

---

## Session 15 — UX consistency sweep (design pass)

**5 items · all untiered · BIG design-driven work.** This is a multi-page app-wide audit, NOT a bug-batch. Treat as a discussion-then-execute pair.

**Files:** Cross-cutting — every `src/lib/components/**/*.svelte` route.

| # | Tier | Blurb |
|---|---|---|
| [#4](../ISSUES.md) | — | UI/UX consistency + navigability sweep (app-wide) |
| [#7](../ISSUES.md) | — | New-user onboarding flow (untested cold-start path) |
| [#24](../ISSUES.md) | — | `docs/ONBOARDING.md` is only 42 lines — too thin for first-run |
| [#22](../ISSUES.md) | — | `console.debug` / `console.warn` noise in production |
| [#29](../ISSUES.md) | — | CSP allows `style-src 'unsafe-inline'` (LOW) |

**Likely splits into 2-3 sub-sessions:** (a) onboarding flow design + impl, (b) per-page consistency walk, (c) console.debug + CSP cleanup. Use `/grill` skill before starting to scope this concretely.

---

## Session 16 — Test coverage Wave A (single tracker)

**2 items · pure infra · backend Rust.** Per #265's Wave A plan: foundation tests w/o needing trait extraction. Targets: `path_guard`, `sync_snapshot` serialization, flush circuit-breaker math, `sync-page.svelte.ts` bucket display, `assistant.svelte.ts` usage accumulator.

| # | Tier | Blurb |
|---|---|---|
| [#21](../ISSUES.md) | — | Zero test coverage anywhere in the repo (UPDATED: 35 tests exist in 10 files — see #265) |
| [#265](../ISSUES.md) | LOW | Test strategy + priority ranking (Wave A → B → C → D plan) |

**Note:** Wave A's `assistant.svelte.ts` usage-accumulator test conflicts with assistant collision — defer that sub-target until assistant session closes. Rust pieces (path_guard, sync_snapshot, flush math) are safe now.

---

# STRATEGIC SESSIONS (own session each, called out)

## Strategic S-1 — Phase 6 OS keychain

**~3 items + integration · HIGH-impact security work · ASSISTANT-COLLISION-DEFERRED.** Migrate API key + bridge token + mcp-config from `~/.rift/*.json` plaintext to Win Credential Manager via `keyring` crate. Closes the two open HIGHs (#37, #38) plus the Phase 6 commitment from #9.3.

**Files:** `assistant/mod.rs`, `profile/mod.rs`, `lib.rs` (registration), new `Cargo.toml` dep.

**Schedule:** AFTER assistant session closes.

## Strategic S-2 — lib.rs split into commands/*.rs

**Hot-file decomposition · 1790L → ~6 files by domain · ASSISTANT-COLLISION-DEFERRED (lib.rs hosts assistant cmd registrations).** Per HANDOFF queue item (e). Breaks lib.rs into `commands/sync.rs`, `commands/assistant.rs`, `commands/sftp.rs`, etc. Pure refactor, no behavior change.

**Schedule:** AFTER assistant session closes; AFTER S5+S6 (lib.rs MED+LOW cleanup) lands so the split doesn't carry forward bug-debt.

## Strategic S-3 — Compaction Phase B

**Assistant context trimming · ASSISTANT-COLLISION-DEFERRED.** Per HANDOFF strategic queue. Touches `assistant.svelte.ts` heavily.

**Schedule:** AFTER assistant session closes.

---

# DEFERRED — assistant-page session collision

> Another session is actively working on the assistant page. Do NOT touch these files concurrently:
> `src-tauri/src/assistant/*.rs`, `src/lib/state/assistant.svelte.ts`, `src/lib/components/assistant/*.svelte`.
>
> Resume these buckets once that session ships its work.

## D-A — Assistant Rust HIGH+MED (13 items)

| # | Tier | Blurb |
|---|---|---|
| [#37](../ISSUES.md) | HIGH | API key plaintext in `~/.rift/assistant/config.json` |
| [#38](../ISSUES.md) | HIGH | `mcp-config.json` Windows DACL not tightened |
| [#62](../ISSUES.md) | MED | Bridge token leaked into MCP child env regardless of remote-shell toggle |
| [#63](../ISSUES.md) | MED | `SESSION_PIDS`/`SESSION_STOPPED` mutex poison silently swallowed |
| [#66](../ISSUES.md) | MED | Unbounded stderr buffer — OOM on wedged CLI |
| [#67](../ISSUES.md) | MED | `child.id()` None branch silently skips PID registration |
| [#68](../ISSUES.md) | MED | MCP parse_error has no JSON-RPC error response back |
| [#69](../ISSUES.md) | MED | MCP `handle_conn` unauthorized write may not flush before drop |
| [#70](../ISSUES.md) | MED | MCP `run_stdio` silently discards response serialization errors |
| [#71](../ISSUES.md) | MED | MCP `tool_grep` reads whole file before 8KB binary check |
| [#72](../ISSUES.md) | MED | MCP `sync_status` listed unconditionally (env-strip case) |
| [#221](../ISSUES.md) | MED | `model` param accepted unvalidated → leading-dash flag injection |
| [#222](../ISSUES.md) | MED | `stderr_task.await.unwrap_or_default()` silently drops JoinError |
| [#23](../ISSUES.md) | — | `use_full_config=true` admits broad MCP tools beyond Rift's own |

## D-B — Assistant Rust LOW (15 items)

| # | Tier | Blurb |
|---|---|---|
| [#114](../ISSUES.md) | LOW | `assistant_delete_conversation` orphans `cli_session_id` cwd sidecar |
| [#115](../ISSUES.md) | LOW | `session-lost` event re-broadcasts full prompt over Tauri bus |
| [#116](../ISSUES.md) | LOW | Attachment 20MiB cap uses approximate base64 estimate |
| [#117](../ISSUES.md) | LOW | `stdin.take() == None` branch hangs child forever |
| [#118](../ISSUES.md) | LOW | MCP `OnceLock::unwrap()` after race-loss in Result-returning fn |
| [#119](../ISSUES.md) | LOW | `tool_remote_bash` read timeout has no total deadline |
| [#120](../ISSUES.md) | LOW | `glob_to_regex` `*.rs` matches per path segment only |
| [#132](../ISSUES.md) | LOW | `convo_path`/`session_cwd_path` no length cap on `id` |
| [#133](../ISSUES.md) | LOW | `common_ancestor` falls back silently to roots[0] |
| [#134](../ISSUES.md) | LOW | `assistant_auth_probe` two-spawn TOCTOU window |
| [#237](../ISSUES.md) | LOW | `thinking_effort` raw value log-injection vector |
| [#239](../ISSUES.md) | LOW | `SESSION_PIDS`/`SESSION_STOPPED` `.lock().ok()` — re-confirmed dup of #63 |
| [#241](../ISSUES.md) | LOW | MCP bridge socket `set_read_timeout`/`set_write_timeout` swallowed |
| [#242](../ISSUES.md) | LOW | MCP bridge `stream.flush().ok()` drops flush errors |
| [#258](../ISSUES.md) | LOW | 39 `format_push_string` clippy hits in MCP server |

## D-C — Assistant frontend MED (10 items)

| # | Tier | Blurb |
|---|---|---|
| [#146](../ISSUES.md) | MED | `mutateStreaming` rebuilds full messages array on every delta |
| [#147](../ISSUES.md) | MED | `ensureThinkingFromEnvelope` `b === existing` always false on `$state` proxies |
| [#148](../ISSUES.md) | MED | `handleTurnComplete` `queueMicrotask` races tab switch |
| [#149](../ISSUES.md) | MED | `openTab` race against `deleteConversation` → blank TabState |
| [#159](../ISSUES.md) | MED | `AssistantHeader` pulse setTimeout not cleaned up |
| [#160](../ISSUES.md) | MED | `Composer` `onblur` kills mention picker before mousedown |
| [#161](../ISSUES.md) | MED | `MessageBubble` tick `setInterval` can double-register |
| [#162](../ISSUES.md) | MED | `Markdown` checklist sync $effect fires on every streaming token |
| [#177](../ISSUES.md) | MED | `beforeunload` listener leak in assistant store (HMR-only) |
| [#234](../ISSUES.md) | MED | `mutateStreaming` O(n) scan + array realloc (dup of #146, promoted) |

## D-D — Assistant frontend LOW (15 items)

| # | Tier | Blurb |
|---|---|---|
| [#178](../ISSUES.md) | LOW | `applyTodoWrite` id generation not stable across calls |
| [#179](../ISSUES.md) | LOW | `stop()` doesn't flush pendingText before clearing `streamingMsgId` |
| [#180](../ISSUES.md) | LOW | `init()` re-entrance guard skips fresh listeners on HMR |
| [#181](../ISSUES.md) | LOW | `restoreTabs` `persistTabs()` not in finally |
| [#182](../ISSUES.md) | LOW | Post-done orphaned non-JSON CLI lines silently dropped |
| [#183](../ISSUES.md) | LOW | `cacheBustHintShown` plain non-reactive |
| [#184](../ISSUES.md) | LOW | `send()` doesn't clear `storeLastError` |
| [#185](../ISSUES.md) | LOW | `retryLast` no re-entrancy guard |
| [#205](../ISSUES.md) | LOW | `HistoryDrawer.focusOnMount` action returns nothing |
| [#206](../ISSUES.md) | LOW | `StepGroup` collapsible `role="button"` div w/o `aria-label` |
| [#207](../ISSUES.md) | LOW | `Composer` hint popover wrong role |
| [#208](../ISSUES.md) | LOW | `ChatTabsBar` drag state not reset on dragcancel |
| [#209](../ISSUES.md) | LOW | `Markdown` code-copy `<span role="button">` not keyboard-activatable |
| [#217](../ISSUES.md) | INFO | `EmptyState.pick()` may not focus textarea if draft unchanged |
| [#261](../ISSUES.md) | LOW | setTimeout handles partial — assistant sites |

## D-E — Marker drift (already shipped, just needs ✓)

These shipped in S115 but ISSUES.md markers weren't updated. Single small commit to flip markers — the other session likely handles this since they're already touching the file context.

| # | Tier | Status |
|---|---|---|
| [#2](../ISSUES.md) | — | Tool-result visual rhythm — **SHIPPED v0.4.14-alpha S115** (MessageBubble turn-footer) |
| [#5](../ISSUES.md) | — | Live status indicator placement — **SHIPPED v0.4.14-alpha S115** (new StatusHub.svelte) |

---

# RESOLVED-AS-NON-BUG (skip entirely)

- **[#19](../ISSUES.md)** — `apply_updates` IPC stop-autosync — verified non-bug; doc tightened.
- **[#42](../ISSUES.md)** — Conflict-copy re-entering dirty queue — Wave 3 verification proved classify() handles it correctly. Marked closed.

---

# Reading-order recommendation

If you want to clear maximum impact early without touching the assistant collision zone:

1. **S1** (Sync MEDs) — 14 items, highest correctness impact.
2. **S4** (SFTP MEDs) — transport hardening, isolated subsystem.
3. **S5** (lib.rs MEDs) — command-shell + bootstrap.
4. **S7** (Settings.svelte) — densest frontend file, mostly LOW but one big sweep is cleaner than spreading across sessions.
5. **S10** (connection.svelte.ts) — small + focused, 3 MEDs.

After that, all remaining schedulable sessions are roughly equivalent in priority. Lights-out tail (S2, S3, S6, S8, S9, S11, S12, S13) can be cleared in any order. S14 (STT) needs the design call first. S15 (UX sweep) is its own beast — use `/grill` before starting.

Strategic sessions (Phase 6 keychain, lib.rs split, Compaction Phase B) all gate on the assistant session closing.

---

# Math reconciliation

```
Schedulable items (S1-S16):
  S1=14  S2=14  S3=13  S4=12  S5=11  S6=12  S7=16
  S8=15  S9=15  S10=6  S11=8  S12=12 S13=14 S14=3
  S15=5  S16=2
  ────────────────────
  Total schedulable rows in tables = 172

Deferred (D-A..D-E):
  D-A=14  D-B=15  D-C=10  D-D=15  D-E=2
  ────────────────────
  Total deferred rows = 56

Already shipped (ISSUES.md SHIPPED marker) = 37
Resolved-as-non-bug (#19 + #42) = 2

Grand total tracked rows:
  172 (sched) + 56 (deferred) + 37 (shipped) + 2 (non-bug)
  = 267

ISSUES.md actual numbered entries = 265.
Overage = 2: cross-references (e.g. #188 listed in both S7 + S10;
            #261 listed in both S9 + D-D; #225 covers lib.rs + drift_scanner;
            #247 covers flush + scan + sftp; #262 covers 3 components).
            These are batched-once but verified-in-multiple-sessions.
```

The cross-refs are intentional — verifying #188 against `loadServers()` callers is cheap to do twice; same with #261 across components.

---

**Next session move:** start with **S1** (sync MEDs). Same shape as the S116 batch I just shipped (#235 + #236) — sync-layer Rust, single `cargo check` to verify, no assistant collision.
