# Rift — Issue Tracker

> Single source of truth for open work. **Active work dashboard** sits at the top (current sprint). **Priority tiers** are the navigation index. Detailed blocks follow.
>
> Open issues only — when something ships, **delete the block** (`git log -- docs/ISSUES.md` preserves history). Each block carries `Where` (file:line, may have drifted — re-grep before acting), `Symptom`, optional `Fix sketch`. Issue IDs are durable — never re-number, only append.
>
> Pruned 2026-05-26: shipped Wave-1/2/3 audit blocks + the AA clippy table moved to `docs/archive/audit-history.md`. Pre-2026-05-26 prune history lives in `git log -- docs/ISSUES.md`.
> Re-verified 2026-05-26 (late): #8 #18 #32 #81 #153 #247 confirmed shipped + bodies pruned. Then session-end pass: #20 M5 shipped (-91L), #2 Option A spacing shipped, #5 confirmed already shipped via Composer streaming bar + bubble stage-strip (StatusHub.svelte flagged as dead code). See closed log.

---

## Active work — current sprint

> Live dashboard. HANDOFF.md = session state only; this section = what's actually queued. Touched 2026-05-22 (post v0.4.25-alpha).

### Smoke gate (verify-then-ship, no code)
Clear before next ship cuts. Each is a UI/live-build verification, not a code change.
- **S129 banner** — drop network 90s, edit file, push → banner should go red + Reconnect
- **S131 Shiki** — send ` ```rust ` fenced block → confirm header bar + syntax highlight
- **S132 splash** — cold-launch eyes-on; muddy-blur fallback = drop `backdrop-filter`, keep flat `--bg @ 86%`
- **S133 Whisper FFI** — `winget install LLVM.LLVM` (admin) + `cargo build --release --features whisper-rs`; CPU first, then CUDA (`whisper-cuda` feature)
- **S124** — ctx-pill inflation **FIXED 2026-05-27** (unshipped): pill read cumulative `result.usage` → spiked ~1M after a multi-step task, false-tripping auto-compact. Now reads the last `assistant` envelope (point-in-time) — `recordTurnUsage` in [assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) + regression test. Auto-compact threshold itself still needs a live multi-step verify. **S124 live-verified 2026-05-28** — pill shows point-in-time (47K), not inflated.

### Code review 2026-05-28 — uncommitted git-rcon + onboarding + UI batch
Adversarial review (18-agent workflow) of the uncommitted batch. **4 FIXED + verified this session** (cargo check green · 3/3 `git_local` tests · svelte-check 0/0; still UNSHIPPED, ride the batch):
- `git_local.rs` `tool_git_diff` truncation — `&out.stdout[..MAX_DIFF_BYTES]` could **panic** mid-codepoint on a >64 KB multibyte diff (the prior `is_char_boundary` check ran on the already-sliced str = tautology). Now steps back to the last char boundary *before* slicing.
- `git_local.rs` `validate_path` — Windows drive-relative (`C:foo`) / UNC (`\\server\share`) paths report `is_absolute()==false` with no `ParentDir`, slipping past both validation branches. Now rejects any `Prefix` component on a non-absolute path; +3 `#[cfg(windows)]` regression cases.
- `mod.rs` `assistant_send` allowlist — `GIT_WRITE_MCP` was in `--allowed-tools` even at `readonly` trust (server still blocked it, but allowlist > gate). Now gated on `trust_level ∈ {standard,full}` via a `git_write` suffix, mirroring `mcp_server::trust_at_least`.
- `assistant.svelte.ts` `setAllowRemoteShell` — didn't refresh `trustLevel`, so the Settings Git-tools segment showed stale state after toggling remote-shell. Now re-pulls `assistant_get_trust_level`.

**OPEN (reviewed, not fixed — lower severity):**
- **CR1 (warning)** `AssistantPage.svelte:15-21` — `dockSlide` width keyframe is a no-op; inline `style="width…"` (~line 232) overrides it (CSS specificity) so only opacity animates → the ResizeObserver→syncBounds intent for the native WebView is defeated. Animate a wrapper or drive width solely via the transition.
- **CR2 (minor)** `FirstSync.svelte:9` — `serverKey` prop typed but never destructured in `$props()`; silently dropped (no current reader; would break any future use). `OnboardingFlow` passes it.
- **CR3 (minor)** `AppShell.svelte` gate — `probeSshKey()` fires from onMount *after* SplashOverlay sets `serversLoaded`; on a slow fresh-install IPC the normal UI can flash before onboarding snaps in. Fix: probe inside SplashOverlay alongside `loadServers`. (`=== false` guard already prevents existing-user flash.)
- **CR4 (minor)** `ActivityPanel.svelte:30` — passes the 1 s `now` ticker as `liveActivity` `fallbackTs`, forcing a re-sort/alloc every tick even with no pending legacy blocks. Use a constant fallback (mount time).
- **CR5 (minor)** `assistant.svelte.ts:10` — `Info` (lucide-svelte component) imported into a `.ts` state module (layering smell; matches the established toast API, low impact).
- **CR-UX** Trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can never return to the derived `None`→"full" state via UI; "full" (rank 2) is functionally identical to "standard" (nothing checks `trust_at_least("full")`, `remote_bash` gates independently). Decide: collapse to a true 2-level enum, or surface "full".
- **Permission round-trip (lane 0) — still NOT live-verified.** Bash auto-approves even in a prompting mode (it's in the user's full-config allowlist; and programmatic `setPermissionMode` didn't drive the per-turn arg — config stayed `null`). The only deterministic trigger left (git-write) has real side-effects on a live repo. Needs the in-app mode selector + a throwaway repo.

### Code lanes (pick one)
Ordered by recommended attack sequence. All cross-reference detailed blocks below.

0. **Permission modes — Piece 2 (NEXT SESSION)** — Piece 1 (picker UI + plumbing) DONE & uncommitted, see HANDOFF. Modes `default`/`acceptEdits`/`plan` are wired but DEAD: headless `claude -p` surfaces per-action permission requests over the stream-json **control channel** (no `--permission-prompt-tool` in CLI v2.1.152), and Rift has no catcher/UI. Build the approval round-trip reusing `ask_user` ([ask_user.rs](../src-tauri/src/assistant/ask_user.rs)). **Spike first (~30min):** capture the exact `control_request`/`can_use_tool` wire format. Then: `--input-format stream-json` always-on in `assistant_send` + a control-message parser writing `control_response` to stdin + UI prompt component.
1. **Files diff-dot per row** — needs new `drift_scanner` per-row verdict backend cmd. (Not in numbered tracker; lane-only.)
2. **Hot-file splits** — issue **#20**: `lib.rs` DONE (285L post-split into `commands/*.rs`); `assistant.svelte.ts` **3355L** (grew) → per-concern classes next; `assistant/mod.rs` **2308L** → continued extraction; `auto_sync.rs` **2207L** (crossed threshold) → next candidate after `assistant.svelte.ts`.
3. **Design brief `git-rcon-tools.md` v2.2** — git + RCON MCP tools. See `docs/design/git-rcon-tools.md`.

### Active design briefs
- `docs/design/git-rcon-tools.md` (lane 3)
- `docs/design/assistant-svelte-split.md` (#20 M0-M9 extraction plan)

---

## 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** Not a single bug — tracking the user's stated goal of an app-wide consistency pass. Settings page is densest control surface and the natural starting point. App-wide pass after.
- **Goal:** Every visible control is wired, every section is necessary, terminology + styling consistent. Navigation is intuitive — current state has "hard to navigate" hotspots per user feedback.
- **Approach when actioned:** Per-page audit checklist (control → wired? necessary? consistent?). Hotspot list grows as specific pain points are flagged (currently #11 is the open concrete instance — #6 shipped). [src/lib/components/settings/Settings.svelte](../src/lib/components/settings/Settings.svelte) **1540L 2026-05-26** — audit still non-trivial.

## 7. New-user onboarding flow (untested cold-start path)

- **Where:** No dedicated first-run UI exists. End-user install path is documented in [docs/DEVELOPING.md §1](DEVELOPING.md#1-end-user-install-onboarding) but no in-app flow guides a fresh user.
- **Symptom:** Unknown what happens on a fresh install — no profile, no SSH keys, no server configured, no Claude auth. Empty states across Sync / Assistant / Activity pages will likely confuse a new user.
- **Fix sketch:** Deliberate first-run flow — welcome → SSH key generate-or-import → profile setup → server add → Claude auth handoff → first sync. Empty states across every page should guide, not confuse. Should be self-contained: no manual file edits, no env vars.

## 14. No CI — release path still local-only

- **Status:** `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). No release workflow — old `release.yml` placeholder was deleted in the v0.4.34 cleanup since it referenced the dead vpk + signing path.
- **Symptom (release path):** ~5 min wall time blocking the build machine per release. Only ships from Blazzer's box (needs `gh auth` + Node + Rust toolchain). Cross-machine reproducibility: theoretically anyone w/ those tools + repo access can run `release.ps1`, but never tested elsewhere.
- **Fix sketch:** New `release.yml` on tag-push: checkout → setup-node@v4 → dtolnay/rust-toolchain@stable → cargo cache (same key as check.yml) → `npm ci` → `pwsh scripts/release.ps1 -Force` w/ `${{ secrets.GITHUB_TOKEN }}`. Much smaller scope than the deleted placeholder b/c no signing secrets to inject anymore.

## 15. Unsigned Windows builds (SmartScreen blocker)

- **Where:** No code-signing step in `scripts/release.ps1`. Setup.exe ships raw.
- **Symptom:** Every fresh install triggers Windows SmartScreen "Unknown publisher" dialog. Real adoption blocker for non-technical users.
- **Fix sketch (options ranked by cost/value):**
  1. **Azure Code Signing** (~$10/mo) — EV-equivalent reputation, no hardware token, CI-friendly.
  2. **SignPath.io free OSS tier** — only viable if the repo goes public.
  3. **DigiCert/Sectigo EV cert** (~$300-400/yr) — instant SmartScreen reputation, hardware token, less CI-friendly.
- **Pipeline integration:** Sign the NSIS Setup.exe post-build, pre-`gh release create`. Authenticode is recoverable on key/cert loss (unlike the ed25519 path we ripped out in v0.4.34) — buy a new cert, existing timestamped binaries still verify.

## 17. Two-repo split — historic, low-priority collapse

- **Where:** [scripts/release.ps1](../scripts/release.ps1) hardcodes `Blazzer10200/rift-releases`; [src-tauri/src/commands/update.rs](../src-tauri/src/commands/update.rs) hits `api.github.com/repos/Blazzer10200/rift-releases/releases/latest`.
- **Symptom:** Every release requires manual sync between private source repo and public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** Collapse to a single repo if the source repo goes public. Two-line change (release.ps1 + commands/update.rs constant). The original velopack auth gap that forced the split is long gone.

## 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** Per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Current state (re-measured 2026-05-25):
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — **3355L (worst; grew from 2320L)**
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — 2308L (crossed threshold)
  - [src-tauri/src/sync/auto_sync.rs](../src-tauri/src/sync/auto_sync.rs) — 2207L (crossed threshold)
  - ~~[src-tauri/src/lib.rs](../src-tauri/src/lib.rs)~~ — **DONE 2026-05-22 M9. 1790L → 285L via `commands/*.rs` per-domain split.**
- **Symptom:** Targeted edits become brittle, LSP slows down, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** `assistant.svelte.ts` next — design brief in [docs/design/assistant-svelte-split.md](design/assistant-svelte-split.md) (2026-05-26) — 9-module extraction plan, ranked by blast radius, w/ TabState invariants enforced. Then `assistant/mod.rs` continued extraction; then `auto_sync.rs` along the `flush.rs` / `watch.rs` precedent (own brief once `assistant.svelte.ts` lands).
- **Status (2026-05-26 Round 4):** M0+M1+M2+M3+M4+**M5+M5b** SHIPPED. `assistant.svelte.ts` 3356L → **2648L** (-21.1%). New modules: `src/lib/state/assistant/{types,helpers,telemetry,workspace,attachments,persistence}.ts`. M5+M5b scope: refresh/buildRecord/flushNow/scheduleSave/rename/persistTabs/loadConversation/deleteConversation. M6-M9 still open — full tab lifecycle (M6: addPane/closePane/setFocusedPane/dropTabIntoPane/openTab/closeTab/newTab/reorder/cycle/closeOthers/closeAll/closeRight) requires manual UI verification on drag/drop semantics — deferred until user can exercise.

## 21. Zero test coverage anywhere in the repo

- **Where:** As of 2026-05-26: 98 `#[test]` fns (96 pass / 2 ignored) across the Rust crate; 38 vitest tests across 3 files (`assistant.test.ts` 21, `sync-page.test.ts` 11, `connection.test.ts` 6). See #265 for the strategy + uncovered-HIGH-risk list. Wave A additions 2026-05-26: 7 new tests on `validate_local_listable` + 3 on `SyncSnapshot::count_under` / `replace_under` / mtime tolerance boundary.
- **Symptom:** For a release-grade app that moves real files over SFTP, atomically renames into a running FXServer, and resolves drift between three states — large portions still uncovered. One regression in `flush_batch`, the drift reconciler, or the ignore-rule parser can corrupt user data silently.
- **Severity:** HIGH for long-term sustainability, MEDIUM for current alpha velocity (was HIGH; Wave A coverage dropped it).
- **Fix sketch:** Continue Wave A — flush counter math is **DEFERRED** (extraction is >50L of churn through `FuturesUnordered` + `dirty.remove` interleaving; see deferred note below). Move to Wave B (`SyncSnapshot::for_path` + tempfile injection) so drift_scanner + flush integration tests become possible.

### 21.1 Flush counter extraction — DEFERRED

`auto_sync/flush.rs::flush_batch` returns `(dispatched, ok, fail)` and the counter logic is the line-by-line `dispatched += 1` / `ok += 1` / `fail += 1` interleaved with `FuturesUnordered` polling, `self.dirty.remove(...)`, `process_entry` dispatch + wedge handling (`auto_sync/flush.rs:139-191`). Pulling the counter math into a pure fn would require either: (a) factoring the entire dispatch loop, breaking the engine ownership / cancel-token plumbing, or (b) a `CounterTally` struct + manual call sites — adds ~30 lines of plumbing for 4 lines of tested logic. **Decision (2026-05-26):** keep inline; cover via an integration test once an `SftpOps` trait (#265 Wave C) lets us mock the SFTP layer and exercise `flush_batch` end-to-end.

---

## Backend hardening — migrated from AUDIT.md 2026-05-19

Open audit items folded in when AUDIT.md was archived to `docs/archive/AUDIT-fix-log.md`. All low-severity backend hardening; full fix-pass history (S81-S86 + Codex passes) lives in the archive.

## 29. CSP allows `style-src 'unsafe-inline'` (LOW)

- **Where:** [src-tauri/tauri.conf.json:24](../src-tauri/tauri.conf.json#L24).
- **Symptom:** Inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** Switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end.

Also accepted as INFO (no action expected): `path_guard.rs:21` Linux-only remote containment (matches Rift's deploy target); `bridge/mod.rs:57` token over loopback HTTP (documented); `edit/edit_trail.rs:75-80` subdir PID-race (collision astronomical after `short_id` widened to 8 bytes); **#32** hostname shell-out SHIPPED — `transport/env.rs:17` switched to env-var + `/proc/sys/kernel/hostname` + absolute-path fallback.

---

## Priority tiers

**S120 — Wave-2 backend MED + LOW sweep, ~40 issues SHIPPED v0.4.17-alpha (commit 0e91393).** SHIPPED set: #54 #55 #56 #68 #70 #72 #73 #75 #77 #79 #80 #83 #84 #86 #91 #93 #94 #95 #97 #98 #101 #105 #110 #111 #114 #116 #117 #118 #121 #122 #123 #124 #126 #128 #130 #132 #133 #136 #137 #138. Body blocks for those numbers pruned (see `git log -- docs/ISSUES.md`).

### SHIPPED 2026-05-26 (closed log — pruned bodies; git log preserves detail)

#18 round-trip verify (`release.ps1:224-244` SHA256 verify) · #23 MCP wildcarding hint · #37 API-key IPC seal · #38 mcp-config DACL · #171 TOFU-stuck-connecting unmount path · #217 EmptyState focus (verified, no code change) · #26 .gitignore audit (verified non-bug) · #8 `scrubUser` lifted to `src/lib/util/redact.ts` + Rust-side gap closed via #238 (inventory dry Round 3) · #247 entry/exit `log::debug!` shipped on `flush_batch` / `drift_scanner.scan_with_cancel` / `sftp.download_file` (long-term tracing bridge deferred) · #81 `SyncSnapshot::set`/`forget` now `log::error!` on save failure (full `Result` propagation deferred) · #153 `force_busy: AtomicBool` serializes `force_push_now`/`force_pull_now` at `auto_sync.rs:317` (FE rescan visual flicker remains) · #32 hostname shell-out switched to env-var + `/proc` fallback at `transport/env.rs:17` · #24 ONBOARDING.md expanded 42L → 221L.

### SHIPPED 2026-05-26 Round 3 (autonomous batch session)

- **#21 Wave B enabler** — `SyncSnapshot::for_path(PathBuf)` constructor + 5 tempfile-injected tests. Cargo tests 96 → 101 pass. Unblocks drift_scanner integration tests w/o `~/.rift/` pollution.
- **#47 #58 #59 #60 #62** — all SHIPPED in v0.4.16-alpha S119 per git-log recon; verified by current-code grep. Recon at `state/issue-recon-2026-05-26.md`. Tier-2 listing was stale; now corrected (see priority tier 2 below).
- **#20 hot-file split M0-M5b** — `assistant.svelte.ts` 3356L → **2648L** (-708L, -21.1%). Six new modules under `src/lib/state/assistant/`: `types.ts` (242L), `helpers.ts` (96L), `telemetry.ts` (192L), `workspace.ts` (87L), `attachments.ts` (38L), `persistence.ts` (261L). External imports unchanged via re-exports. M5+M5b scope: full conversation persistence path (refresh/build/flush/schedule/rename/persistTabs/load/delete). M6-M9 still open — tabs lifecycle (M6), compaction (M7), streaming (M8), send (M9).
- **#2 narration-grouping spacing (Option A)** — pure-CSS `data-group-cont` attribute on `MessageBubble.svelte:.tl-node` w/ tighter top-margin (0.25rem) for consecutive same-narration nodes vs new-beat (0.5rem). Threads through `grouped` derivation at line 480.
- **#5 status hub** — confirmed already shipped via two earlier impls: `Composer.svelte:855-876` `.composer.streaming::before` animated streaming bar + `MessageBubble.svelte:692-720` `.stage-strip` in-bubble label. `StatusHub.svelte` unimported anywhere — dead code flagged (not deleted, per CLAUDE.md). Brief at `docs/design/issue-5-status-hub.md` marked OBSOLETE.
- **#2 deeper grouping** — Option A SHIPPED (pure CSS narration-grouping spacing on `MessageBubble.svelte:.tl-node[data-group-cont]`). Brief at `docs/design/issue-2-grouping.md`.
- **#5 status hub** — already shipped via `Composer.svelte:.composer.streaming::before` animated streaming bar + `MessageBubble.svelte:.stage-strip` in-bubble label. `StatusHub.svelte` is dead code (no imports — flagged, not deleted). Brief at `docs/design/issue-5-status-hub.md` marked OBSOLETE.
- **#27 / #29 / #81 / #8 (further expansion)** — investigated, marked no-op per plan literal (#27 all call sites sync; #29 needs dev restart + Tailwind 4 still emits inline styles; #81 all 6 callers hot-path async = pure churn; #8 grep inventory dry). Status notes appended in `docs/NEXT-SESSION.md` per-batch.

**Tier 1 — ship blockers / data safety**
- #21 Zero test coverage — see #265 for plan; reality 96 Rust + 38 vitest. Wave A landed; B-D structurally blocked.
- #15 Unsigned Windows builds (adoption blocker; $-gated on Azure Code Signing)

**Tier 2 — recurring friction**
- #14 No CI **PARTIAL** — `.github/workflows/check.yml` shipped; `release.yml` skeleton awaits #15.
- ~~**Sync MEDs:** #47 #58 #59 #60 #62~~ — all SHIPPED v0.4.16-alpha S119 per Round 3 recon (`state/issue-recon-2026-05-26.md`). No open sync MEDs.

**Tier 3 — UX**
- *(empty — #2 + #5 closed 2026-05-26 late; see closed log)*

**Tier 4 — strategic / longer-term**
- #4 App-wide UX consistency sweep · #7 new-user onboarding · #20 hot-file split M6-M9 (brief at `docs/design/assistant-svelte-split.md`) · #17 two-repo split debt

**Tier 5 — backend LOW (opportunistic)**
- #27 `atomic_write_json` Tokio worker (contract documented at fn; spawn_blocking wrap at call sites = real refactor)
- #29 CSP `style-src 'unsafe-inline'` (Tailwind-blocked)
- #89 `download_file` whole-file buffer (deferred-complexity — streaming path for >16 MB)
- **Wave-1 LOWs** #91-#134 — clippy/doc/perf nits (see `docs/archive/audit-history.md`)

---

## Audit 2026-05-20 — open items (post-prune)

> Shipped Wave 1/2/3 items moved to `docs/archive/audit-history.md` on 2026-05-26. The remainder stays here as live work. Reports persisted at `state/audit-2026-05-20/{A..AA}-*.md`; synthesis at `SYNTHESIS-wave[1-3].md`.

## 89. `download_file` buffers entire remote file into memory

- **Where:** [sftp/transfer.rs:231](../src-tauri/src/sftp/transfer.rs#L231), [:337](../src-tauri/src/sftp/transfer.rs#L337)
- **Symptom:** `sftp.read(remote_path)` loads full bytes into `Vec<u8>` before writing local. Hundreds-of-MB asset files (FiveM map packs, .ytd) OOM on low-RAM servers / mobile WiFi.
- **Fix:** For files >16 MB, `sftp.open` + stream chunks to local tmp via `AsyncRead`. Deferred-complexity.

## 265. Test strategy + priority ranking

- **Where:** plan content below (audit shards purged 2026-05-21; Wave A landed 2026-05-26).
- **Status:** Reality 2026-05-26: 98 `#[test]` fns in 11 files + 38 vitest tests. Wave A landed: `path_guard::validate_local_listable` (7 tests), `SyncSnapshot::count_under` / `replace_under` / mtime tolerance (3 tests). Flush counter math deferred (see #21.1). Wave B-D structural blockers unchanged: no `SftpOps` trait (concrete `SftpClient`); `AutoSyncEngine` needs `AppHandle`; `SyncSnapshot::new` writes to `~/.rift/`; `BRIDGE: OnceLock` set-once. Uncovered HIGH-risk modules: `drift_scanner.rs`, `sftp/transfer.rs`, `auto_sync/flush.rs`, `assistant/remote_bridge.rs`, `assistant/mod.rs`, `assistant.svelte.ts`, `drift_watcher.rs`.
- **Next:** Wave B — `SyncSnapshot::for_path` constructor + tempfile-injected snapshot tests so drift_scanner can be unit-tested without `~/.rift/` pollution.
