# Rift — Issue Tracker

> Single source of truth for **open work only**. When something ships, **delete the block** — `git log -- docs/ISSUES.md` preserves history. Each block carries `Where` (file:line, may have drifted — re-grep before acting), `Symptom`, optional `Fix sketch`. Issue IDs are durable — never re-number, only append.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables live in `docs/archive/audit-history.md`. Last prune: 2026-05-29 (removed shipped-log graveyard + dead file refs).

---

## Active work — current sprint

> Live queue. HANDOFF.md = session state; this section = what's queued. Touched 2026-05-29 (post v0.4.37).

### Smoke gate (verify-then-ship, no code)
Clear before next ship cut. Each is a UI/live-build verification, not a code change.
- **S129 banner** — drop network 90s, edit file, push → banner should go red + Reconnect
- **S131 Shiki** — send ` ```rust ` fenced block → confirm header bar + syntax highlight
- **S132 splash** — cold-launch eyes-on; muddy-blur fallback = drop `backdrop-filter`, keep flat `--bg @ 86%`
- **S133 Whisper FFI** — `winget install LLVM.LLVM` (admin) + `cargo build --release --features whisper-rs`; CPU first, then CUDA (`whisper-cuda` feature)

### Code review 2026-05-28 — OPEN items (git-rcon + onboarding + UI batch)
From the adversarial review of the v0.4.37 batch. The 4 high-severity fixes shipped in v0.4.37; these remain:
- **CR1 (warning)** `AssistantPage.svelte:15-21` — `dockSlide` width keyframe is a no-op; inline `style="width…"` (~line 232) overrides it (CSS specificity) so only opacity animates → the ResizeObserver→syncBounds intent for the native WebView is defeated. Animate a wrapper or drive width solely via the transition.
- **CR2 (minor)** `FirstSync.svelte:9` — `serverKey` prop typed but never destructured in `$props()`; silently dropped (no current reader; would break any future use). `OnboardingFlow` passes it.
- **CR3 (minor)** `AppShell.svelte` gate — `probeSshKey()` fires from onMount *after* SplashOverlay sets `serversLoaded`; on a slow fresh-install IPC the normal UI can flash before onboarding snaps in. Fix: probe inside SplashOverlay alongside `loadServers`. (`=== false` guard already prevents existing-user flash.)
- **CR4 (minor)** `ActivityPanel.svelte:30` — passes the 1 s `now` ticker as `liveActivity` `fallbackTs`, forcing a re-sort/alloc every tick even with no pending legacy blocks. Use a constant fallback (mount time).
- **CR5 (minor)** `assistant.svelte.ts:10` — `Info` (lucide-svelte component) imported into a `.ts` state module (layering smell; matches the established toast API, low impact).
- **CR-UX** Trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can never return to the derived `None`→"full" state via UI; "full" (rank 2) is functionally identical to "standard" (nothing checks `trust_at_least("full")`, `remote_bash` gates independently). Decide: collapse to a true 2-level enum, or surface "full".
- **Permission round-trip — code-complete, needs live-verify.** Wired end-to-end: `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → `write_control_response` → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`. Remaining: live-verify with a throwaway repo — a git-write op in default/acceptEdits/plan mode should surface the Allow/Deny bar (Bash still auto-approves via the user's full-config allowlist).

### Code lanes (pick one)
Ordered by recommended attack sequence. Cross-reference detailed blocks below.
1. **Files diff-dot per row** — needs new `drift_scanner` per-row verdict backend cmd. (Lane-only, not in numbered tracker.)
2. **Hot-file splits** — issue **#20**: `assistant.svelte.ts` **2314L** (M0-M7 carved, M8/M9 open); `assistant/mod.rs` **2795L** (grew from git-rcon) → next backend split candidate; `auto_sync.rs` **2232L** → candidate after mod.rs.
3. **RCON MCP tools** — git tools shipped v0.4.37 (`git_local.rs`); RCON half (`rcon_resource`, `dev_cycle`) still pending. Brief archived at `docs/archive/git-rcon-tools.md`.

### Active design briefs
- `docs/design/assistant-svelte-split.md` (#20 — M0-M7 shipped; M8 streaming + M9 send open)

---

## 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** Not a single bug — tracking the user's stated goal of an app-wide consistency pass. Settings page is densest control surface and the natural starting point. App-wide pass after.
- **Goal:** Every visible control is wired, every section is necessary, terminology + styling consistent. Navigation is intuitive — current state has "hard to navigate" hotspots per user feedback.
- **Approach when actioned:** Per-page audit checklist (control → wired? necessary? consistent?). Hotspot list grows as specific pain points are flagged. [src/lib/components/settings/Settings.svelte](../src/lib/components/settings/Settings.svelte) ~1540L — audit still non-trivial.

## 14. No CI — release path still local-only (PARTIAL)

- **Status:** `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). No release workflow — old `release.yml` placeholder was deleted in the v0.4.34 cleanup since it referenced the dead vpk + signing path.
- **Symptom (release path):** ~5 min wall time blocking the build machine per release. Only ships from Blazzer's box (needs `gh auth` + Node + Rust toolchain). Cross-machine reproducibility untested.
- **Fix sketch:** New `release.yml` on tag-push: checkout → setup-node@v4 → dtolnay/rust-toolchain@stable → cargo cache (same key as check.yml) → `npm ci` → `pwsh scripts/release.ps1 -Force` w/ `${{ secrets.GITHUB_TOKEN }}`. Much smaller scope than the deleted placeholder b/c no signing secrets to inject anymore. Gated behind #15.

## 15. Unsigned Windows builds (SmartScreen blocker)

- **Where:** No code-signing step in `scripts/release.ps1`. Setup.exe ships raw.
- **Symptom:** Every fresh install triggers Windows SmartScreen "Unknown publisher" dialog. Real adoption blocker for non-technical users.
- **Fix sketch (options ranked by cost/value):**
  1. **Azure Code Signing** (~$10/mo) — EV-equivalent reputation, no hardware token, CI-friendly.
  2. **SignPath.io free OSS tier** — only viable if the repo goes public.
  3. **DigiCert/Sectigo EV cert** (~$300-400/yr) — instant SmartScreen reputation, hardware token, less CI-friendly.
- **Pipeline integration:** Sign the NSIS Setup.exe post-build, pre-`gh release create`. Authenticode is recoverable on key/cert loss (unlike the ed25519 path ripped out in v0.4.34) — buy a new cert, existing timestamped binaries still verify.

## 17. Two-repo split — historic, low-priority collapse

- **Where:** [scripts/release.ps1](../scripts/release.ps1) hardcodes `Blazzer10200/rift-releases`; [src-tauri/src/commands/update.rs](../src-tauri/src/commands/update.rs) hits `api.github.com/repos/Blazzer10200/rift-releases/releases/latest`.
- **Symptom:** Every release requires manual sync between private source repo and public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** Collapse to a single repo if the source repo goes public. Two-line change (release.ps1 + commands/update.rs constant). The original velopack auth gap that forced the split is long gone.

## 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** Per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Open targets (re-measured 2026-05-28):
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — **2795L (worst; grew from git-rcon v0.4.37)**
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — 2314L (M0-M7 carved from 3356L; M8/M9 open)
  - [src-tauri/src/sync/auto_sync.rs](../src-tauri/src/sync/auto_sync.rs) — 2232L (crossed threshold)
- **Symptom:** Targeted edits become brittle, LSP slows down, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** `assistant.svelte.ts` next — design brief in [docs/design/assistant-svelte-split.md](design/assistant-svelte-split.md) — 9-module extraction plan, ranked by blast radius, w/ TabState invariants enforced. Then `assistant/mod.rs` continued extraction; then `auto_sync.rs` along the `flush.rs` / `watch.rs` precedent (own brief once `assistant.svelte.ts` lands).
- **Status (2026-05-28):** M0-M7 SHIPPED, `assistant.svelte.ts` 3356L → **2314L** (-31.1%); modules under `src/lib/state/assistant/`. M8 (streaming pump) + M9 (send orchestrator) still open — the two highest-blast-radius extractions; deferred until a full conversation-playback test harness exists.

## 21. Test coverage gaps

- **Where:** As of 2026-05-28: 111 `#[test]` fns + 39 vitest tests across 3 files (`assistant.test.ts`, `sync-page.test.ts`, `connection.test.ts`). See #265 for the strategy + uncovered-HIGH-risk list.
- **Symptom:** For a release-grade app that moves real files over SFTP, atomically renames into a running FXServer, and resolves drift between three states — large portions still uncovered. One regression in `flush_batch`, the drift reconciler, or the ignore-rule parser can corrupt user data silently.
- **Severity:** HIGH for long-term sustainability, MEDIUM for current alpha velocity (Wave A coverage dropped it from HIGH).
- **Fix sketch:** Move to Wave B (`SyncSnapshot::for_path` + tempfile injection — enabler landed) so drift_scanner + flush integration tests become possible.

### 21.1 Flush counter extraction — DEFERRED

`auto_sync/flush.rs::flush_batch` returns `(dispatched, ok, fail)` and the counter logic is the line-by-line `dispatched += 1` / `ok += 1` / `fail += 1` interleaved with `FuturesUnordered` polling, `self.dirty.remove(...)`, `process_entry` dispatch + wedge handling (`auto_sync/flush.rs:139-191`). Pulling the counter math into a pure fn would require either: (a) factoring the entire dispatch loop, breaking the engine ownership / cancel-token plumbing, or (b) a `CounterTally` struct + manual call sites — adds ~30 lines of plumbing for 4 lines of tested logic. **Decision (2026-05-26):** keep inline; cover via an integration test once an `SftpOps` trait (#265 Wave C) lets us mock the SFTP layer and exercise `flush_batch` end-to-end.

---

## Backend hardening (LOW)

> Open audit items folded in when AUDIT.md was archived 2026-05-19. Full fix-pass history lives in `docs/archive/audit-history.md`.

## 29. CSP allows `style-src 'unsafe-inline'` (LOW)

- **Where:** [src-tauri/tauri.conf.json:24](../src-tauri/tauri.conf.json#L24).
- **Symptom:** Inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** Switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end.

## 89. `download_file` buffers entire remote file into memory

- **Where:** [sftp/transfer.rs:231](../src-tauri/src/sftp/transfer.rs#L231), [:337](../src-tauri/src/sftp/transfer.rs#L337)
- **Symptom:** `sftp.read(remote_path)` loads full bytes into `Vec<u8>` before writing local. Hundreds-of-MB asset files (FiveM map packs, .ytd) OOM on low-RAM servers / mobile WiFi.
- **Fix:** For files >16 MB, `sftp.open` + stream chunks to local tmp via `AsyncRead`. Deferred-complexity.

**Accepted as INFO (no action expected):** `path_guard.rs:21` Linux-only remote containment (matches Rift's deploy target); `bridge/mod.rs:57` token over loopback HTTP (documented); `edit/edit_trail.rs:75-80` subdir PID-race (collision astronomical after `short_id` widened to 8 bytes).

---

## Priority tiers

**Tier 1 — ship blockers / data safety**
- #21 Test coverage gaps — see #265 for plan; Wave A landed, B-D structurally blocked.
- #15 Unsigned Windows builds (adoption blocker; $-gated on Azure Code Signing).

**Tier 2 — recurring friction**
- #14 No CI **PARTIAL** — `check.yml` shipped; `release.yml` skeleton awaits #15.

**Tier 3 — strategic / longer-term**
- #4 App-wide UX consistency sweep · #20 hot-file split M8-M9 (brief at `docs/design/assistant-svelte-split.md`) · #17 two-repo split debt.

**Tier 4 — backend LOW (opportunistic)**
- #29 CSP `style-src 'unsafe-inline'` (Tailwind-blocked)
- #89 `download_file` whole-file buffer (deferred-complexity — streaming path for >16 MB)
- **Wave-1 LOWs** #91-#134 — clippy/doc/perf nits (see `docs/archive/audit-history.md`)

---

## 265. Test strategy + priority ranking

- **Status (2026-05-28):** 111 `#[test]` fns + 39 vitest tests (3 git_local regression tests added v0.4.37). Wave A landed: `path_guard::validate_local_listable` (7 tests), `SyncSnapshot::count_under` / `replace_under` / mtime tolerance (3 tests). Flush counter math deferred (see #21.1). Wave B-D structural blockers: no `SftpOps` trait (concrete `SftpClient`); `AutoSyncEngine` needs `AppHandle`; `SyncSnapshot::new` writes to `~/.rift/`; `BRIDGE: OnceLock` set-once. Uncovered HIGH-risk modules: `drift_scanner.rs`, `sftp/transfer.rs`, `auto_sync/flush.rs`, `assistant/remote_bridge.rs`, `assistant/mod.rs`, `assistant.svelte.ts`, `drift_watcher.rs`.
- **Next:** Wave B — `SyncSnapshot::for_path` constructor + tempfile-injected snapshot tests so drift_scanner can be unit-tested without `~/.rift/` pollution.
