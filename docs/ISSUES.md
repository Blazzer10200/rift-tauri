# Rift — Issue Tracker

> Single source of truth for open work. **Active work dashboard** sits at the top (current sprint). **Priority tiers** are the navigation index. Detailed blocks follow.
>
> Open issues only — when something ships, **delete the block** (`git log -- docs/ISSUES.md` preserves history). Each block carries `Where` (file:line, may have drifted — re-grep before acting), `Symptom`, optional `Fix sketch`. Issue IDs are durable — never re-number, only append.
>
> Originally captured 2026-05-19 (1638 lines, 265 items). Pruned 2026-05-21 to open items only (~170). Re-verification pass 2026-05-25 removed ~55 already-shipped Wave-2/3 blocks across three sweeps: (1) leak/ARIA cluster #151-152, #154-162, #168, #173-176, #186-190, #195-196, #199, #205-207, #209, #214; (2) assistant/sync/perf cluster #170, #177-185, #197-198, #201, #203-204, #208, #210, #212-213, #215-216, #239, #246, #257-258; (3) dialog/wire/events cluster #169, #172, #200 (shipped this session via DriftSummaryCard.svelte rewrite), #218 (dup of #174). Misanalysis dropped: #256 (events are prepended at diagnostics.svelte.ts:104, so `.find()` from head IS optimal). Stale: #191-194 (Settings dropdowns no longer exist), #159/#261 (AssistantHeader.svelte deleted).

---

## Active work — current sprint

> Live dashboard. HANDOFF.md = session state only; this section = what's actually queued. Touched 2026-05-22 (post v0.4.25-alpha).

### Smoke gate (verify-then-ship, no code)
Clear before next ship cuts. Each is a UI/live-build verification, not a code change.
- **S129 banner** — drop network 90s, edit file, push → banner should go red + Reconnect
- **S131 Shiki** — send ` ```rust ` fenced block → confirm header bar + syntax highlight
- **S132 splash** — cold-launch eyes-on; muddy-blur fallback = drop `backdrop-filter`, keep flat `--bg @ 86%`
- **S133 Whisper FFI** — `winget install LLVM.LLVM` (admin) + `cargo build --release --features whisper-rs`; CPU first, then CUDA (`whisper-cuda` feature)
- **S124** — auto-compact + ctx stats still in gate

### Code lanes (pick one)
Ordered by recommended attack sequence. All cross-reference detailed blocks below.

1. ~~**Wave-2 frontend HIGH cluster**~~ — **#146/#147/#148/#149 + #234 all SHIPPED** (verified 2026-05-25 via inline citations in `assistant.svelte.ts`).
2. ~~**Wave-3 backend security HIGHs**~~ — **#221 / #227 / #237 / #238 SHIPPED 2026-05-22 (Lane 2, uncommitted v0.4.30-alpha train).** #228 re-scoped (dialog plugin in active use — see updated block).
3. **Files diff-dot per row** — needs new `drift_scanner` per-row verdict backend cmd. (Not in numbered tracker; lane-only.)
4. **Hot-file splits** — issue **#20**: `lib.rs` DONE (285L post-split into `commands/*.rs`); `assistant.svelte.ts` **3355L** (grew) → per-concern classes next; `assistant/mod.rs` **2308L** → continued extraction; `auto_sync.rs` **2207L** (crossed threshold) → next candidate after `assistant.svelte.ts`.
5. **Design brief `git-rcon-tools.md` v2.2** — git + RCON MCP tools. See `docs/design/git-rcon-tools.md`.

### Active design briefs
- `docs/design/assistant-compaction.md`
- `docs/design/git-rcon-tools.md` (lane 5)
- `docs/design/ui-audit-2026-05-21.md`

---

## 2. Tool-result blocks lack visual rhythm — "done" ambiguous mid-turn

- **Where:** [src/lib/components/assistant/MessageBubble.svelte](../src/lib/components/assistant/MessageBubble.svelte) (1069L 2026-05-25), inline diff renderer + StepGroup
- **Symptom:** When an assistant turn contains short narration → Edit block → more narration → another Edit block → final summary, the visual cadence reads as if the message ended after the first big block. User assumes Claude is done and looks away; new block appears "out of nowhere." Particularly bad w/ multi-Edit batches (verified in user screenshots S104 era).
- **Fix sketch:** Stronger end-of-turn marker (footer w/ cost + model + duration is partially there via `costLabel` / `modelLabel` derivations, but isn't visually distinct from a mid-turn block). Consider dimming/collapsing intermediate tool blocks once a turn finishes, tighter visual grouping of "narration + its tool call(s)" as one unit, or a "still working…" pulse on the role row until `streaming=false`.

## 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** Not a single bug — tracking the user's stated goal of an app-wide consistency pass. Settings page is densest control surface and the natural starting point. App-wide pass after.
- **Goal:** Every visible control is wired, every section is necessary, terminology + styling consistent. Navigation is intuitive — current state has "hard to navigate" hotspots per user feedback.
- **Approach when actioned:** Per-page audit checklist (control → wired? necessary? consistent?). Hotspot list grows as specific pain points are flagged (currently #11 is the open concrete instance — #6 shipped). [src/lib/components/settings/Settings.svelte](../src/lib/components/settings/Settings.svelte) was 1505L; **1436L 2026-05-25** after the terminal-section strip — audit still non-trivial.

## 5. Live status indicator placement (QoL)

- **Where:** [src/lib/components/assistant/MessageBubble.svelte:189-194](../src/lib/components/assistant/MessageBubble.svelte#L189-L194) `stageLabel` derivation; renders inline at top of assistant bubble.
- **Symptom:** Status word ("Cogitating…", "Reading X", "Running cargo check") shows at the top of the in-flight response bubble, easy to miss while scrolling or focused on the input.
- **User proposal:** Surface the live status more prominently — possibly a small "hub" above the prompt input alongside the send + mic buttons. Could consolidate current activity, elapsed time, maybe per-turn token delta.
- **Fix sketch:** Defer to whoever does the frontend pass. Likely files: `src/lib/components/assistant/` (Composer + a new StatusStrip component), `src/lib/state/assistant.svelte.ts` (status state already exists since `Cogitating` renders).

## 7. New-user onboarding flow (untested cold-start path)

- **Where:** No dedicated first-run UI exists. [docs/ONBOARDING.md](ONBOARDING.md) is only 42 lines (see #24).
- **Symptom:** Unknown what happens on a fresh install — no profile, no SSH keys, no server configured, no Claude auth. Empty states across Sync / Assistant / Activity pages will likely confuse a new user.
- **Fix sketch:** Deliberate first-run flow — welcome → SSH key generate-or-import → profile setup → server add → Claude auth handoff → first sync. Empty states across every page should guide, not confuse. Should be self-contained: no manual file edits, no env vars.

## 8. Extend `scrubUser` pattern to log forwarding + IPC paths

- **Where:** [Settings.svelte:91-97](../src/lib/components/settings/Settings.svelte#L91-L97) `scrubUser()` redacts `C:\Users\<name>\` → `<user>` for the copy-diagnostic button. Pattern is good but not applied anywhere else.
- **Symptom:** Log forwarding from Rust to frontend (DiagBus + LogForwarder) passes raw paths unredacted (verified — see #9 and `diagnostics/mod.rs:326`). Anywhere a path crosses the IPC boundary or surfaces to a log line is a potential username leak.
- **Fix sketch:** Lift `scrubUser` into a shared util (frontend + Rust-side equivalent). Apply at every log-emission point in Rust and at every path-surfacing IPC return value. See also #9 for the specific Rust gap.

## 14. No CI — every release is local-only

- **Where:** No `.github/workflows/` directory exists in the repo.
- **Symptom:** 5-15 min wall time blocking your machine per release. Mid-run build failure leaves stale `Releases/staging-*` (cleanup is on success path only at line 138). No cross-machine reproducibility — only works on Blazzer's box w/ `gh auth` + `vpk` + Node toolchain installed.
- **Fix sketch:** `.github/workflows/release.yml` triggered on `v*` tag push. Job: checkout → setup-node → cargo cache → `dotnet tool install -g vpk` → run `release.ps1` w/ `GITHUB_TOKEN`. Removes local toolchain dependency. Add a cleanup hook for `Releases/staging-*` on failure.

## 15. Unsigned Windows builds (SmartScreen blocker)

- **Where:** Acknowledged in [scripts/release.ps1:4-5](../scripts/release.ps1#L4-L5): "Unsigned for now (audit H4 — signing deferred until cert + AAS budget is in place)."
- **Symptom:** Every fresh install triggers Windows SmartScreen "Unknown publisher" dialog. Real adoption blocker for non-technical users.
- **Fix sketch (options ranked by cost/value):**
  1. **Azure Code Signing** (~$10/mo) — EV-equivalent reputation, no hardware token, CI-friendly.
  2. **SignPath.io free OSS tier** — only viable if the repo goes public.
  3. **DigiCert/Sectigo EV cert** (~$300-400/yr) — instant SmartScreen reputation, hardware token, less CI-friendly.
- **Pipeline integration:** `vpk pack --signParams` flag exists; CI secret holds the cert handle.

## 16. Custom `GithubSource` is 130 lines of SDK-gap debt

- **Where:** [src-tauri/src/update_service.rs:138-269](../src-tauri/src/update_service.rs#L138-L269).
- **Symptom:** Written because velopack-rust 0.0.1298 ships no `GithubSource` type — `AutoSource` for an HTTP URL only works against flat static `releases.{channel}.json`, not real GitHub release pages. Reimplements REST asset enumeration, prerelease filtering, asset-URL caching, download streaming. Every bug in this code is yours; every bug fixed in upstream Velopack you don't get.
- **Fix sketch:** Check current velopack-rust crate version. If newer ships a `GithubSource`, swap impl + delete 130 lines. If still missing, file an upstream issue/PR — the impl is reusable.

## 17. Two-repo split exists only for the velopack auth gap

- **Where:** Coupling btw [scripts/release.ps1:62](../scripts/release.ps1#L62) (`Blazzer10200/rift-releases` hardcoded) + [src-tauri/src/update_service.rs:29-30](../src-tauri/src/update_service.rs#L29-L30) (`GITHUB_OWNER` / `GITHUB_REPO` consts).
- **Symptom:** Every release requires manual sync btw private source repo and public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** Revisit when #16 lands. If upstream `GithubSource` supports a token, the public-mirror repo becomes optional. Until then, document the split prominently in `CONTRIBUTING.md` so it's not folklore.

## 18. No round-trip verify post-publish

- **Where:** [scripts/release.ps1:134-135](../scripts/release.ps1#L134-L135) — only runs `gh release view` (metadata only).
- **Symptom:** Script reports success even if Setup.exe is corrupt, wrong asset name, or missing nupkg. You only find out when users' auto-update fails.
- **Fix sketch:** Cheap baseline — after `vpk upload`, `gh release download $tag --pattern "*Setup.exe" -D Releases/verify-$version/` then SHA256 against the local pre-upload artifact. Better: launch the downloaded Setup.exe in a sandbox/VM via CI smoke test.

## 19. `apply_updates` IPC may not stop autosync before binary swap (RESOLVED — non-bug)

> **RESOLVED v0.4.12-alpha** — verified [lib.rs:1554-1573](../src-tauri/src/lib.rs#L1554-L1573) `apply_updates` already stops autosync (engine.stop) + tunnel (t.stop) BEFORE `spawn_blocking(|| UpdateService::new().apply())`. The audit re-read mistook the boundary — frontend correctly does NOT stop anything; the Tauri command layer owns it. Tightened doc comment at `update_service.rs:82-88` to make explicit that direct callers of `UpdateService::apply` must do their own stop, but the `apply_updates` command handles it.

- **Where:** Comment at [src-tauri/src/update_service.rs:85-86](../src-tauri/src/update_service.rs#L85-L86) explicitly says "Caller MUST stop autosync + tunnel BEFORE invoking — in-flight uploads die when the process exits." Frontend [src/lib/state/updates.svelte.ts:40-50](../src/lib/state/updates.svelte.ts#L40-L50) just calls `invoke("apply_updates")` with no JS-side stop.
- **Symptom (if confirmed):** In-flight SFTP uploads + tunnel sessions die mid-operation when process exits for binary swap. Possible orphaned `.rift-tmp` files on remote, possible partial uploads to the running FXServer.
- **Verification needed:** Grep `lib.rs` for the `apply_updates` Tauri command body — does it call into autosync/tunnel stop before delegating to `update_service.apply()`? If yes, the comment is stale (still worth fixing for clarity). If no, this is a real bug.

## 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** Per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Current state (re-measured 2026-05-25):
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — **3355L (worst; grew from 2320L)**
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — 2308L (crossed threshold)
  - [src-tauri/src/sync/auto_sync.rs](../src-tauri/src/sync/auto_sync.rs) — 2207L (crossed threshold)
  - ~~[src-tauri/src/lib.rs](../src-tauri/src/lib.rs)~~ — **DONE 2026-05-22 M9. 1790L → 285L via `commands/*.rs` per-domain split.**
- **Symptom:** Targeted edits become brittle, LSP slows down, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** `assistant.svelte.ts` next — extract per-concern classes (tabs, streaming, usage, tasks). Then `assistant/mod.rs` continued extraction; then `auto_sync.rs` along the `flush.rs` / `watch.rs` precedent.

## 21. Zero test coverage anywhere in the repo

- **Where:** I found no `test/` dir at repo root, no substantive `#[cfg(test)]` blocks in `src-tauri/src/`, no vitest / playwright config, no Rust integration tests dir.
- **Symptom:** For a release-grade app that moves real files over SFTP, atomically renames into a running FXServer, and resolves drift between three states — zero automated tests is genuinely scary. One regression in `flush_batch`, the drift reconciler, or the ignore-rule parser can corrupt user data silently.
- **Severity:** HIGH for long-term sustainability, LOW for current alpha velocity.
- **Fix sketch:** Start small — Rust unit tests for `sync/ignore.rs` (pure logic, no I/O, easy wins), `state/sync_snapshot.rs` (serialization), `sftp/transfer.rs` atomic rename semantics (needs mock SFTP). Frontend: vitest for `stt.svelte.ts` consume/onResult/onEnd state machine, `assistant.svelte.ts` usage accumulation.

## 22. `console.debug` / `console.warn` noise in production

- **Where:**
  - [src/lib/state/assistant.svelte.ts:764](../src/lib/state/assistant.svelte.ts#L764) — S105 cache-hit-ratio probe (line shifted from 564 post-S106).
  - [src/lib/state/assistant.svelte.ts:939](../src/lib/state/assistant.svelte.ts#L939) — new `console.debug` added by S106 telemetry layer.
  - [src/lib/state/assistant.svelte.ts:821](../src/lib/state/assistant.svelte.ts#L821) — `console.debug` for non-JSON idle stream lines.
  - [src/lib/state/stt.svelte.ts:104, 202, 266](../src/lib/state/stt.svelte.ts#L104) — warns on routine paths (config load fail, stop fail, recognition error).
- **Symptom:** Users w/ devtools open see scary-looking spam during normal operation.
- **Fix sketch:** Strip the S105 probe log (it's served its purpose if cache investigation is complete). Gate routine warns behind a dev flag or downgrade to `console.debug`. Reserve `console.warn` for actionable failure modes only.

## 23. `use_full_config=true` admits broad MCP tools beyond Rift's own

- **Where:** `src-tauri/src/assistant/mod.rs:1193` (per security fork — I did not re-verify line number, claim is the fork's).
- **Symptom:** Passes `mcp__*` to `--allowed-tools`, which lets the model use any tool from user-side MCP servers the Claude CLI merges in. Surface area users should know they're opting into when flipping the related toggle.
- **Fix sketch:** Document explicitly in Settings → Assistant near the toggle ("This admits MCP tools from your global Claude config — see ~/.claude/.mcp.json for what's enabled"). Optionally allowlist instead of wildcarding.

## 24. `docs/ONBOARDING.md` is only 42 lines — too thin for the first-run experience

- **Where:** [docs/ONBOARDING.md](ONBOARDING.md).
- **Symptom:** Way too thin for the cold-start path #7 is worried about. No screenshots, minimal walkthrough.
- **Fix sketch:** Expand before any public release. Cover: install from Setup.exe, SmartScreen warning (until #15 lands), SSH key setup (generate or import), first server profile, first sync, troubleshooting (no SFTP connect, no fingerprint match, no Claude CLI found).

## 25. velopack-rust pinned at pre-1.0 version

- **Where:** [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) — `velopack = "0.0.1298"` per source comment in `update_service.rs`.
- **Symptom:** Pre-1.0 software (no SemVer guarantees) carrying your entire distribution path. Every minor bump can break the custom `GithubSource` from #16. Implicit risk every `cargo update`.
- **Fix sketch:** Pin the exact version in Cargo.toml (not a range). Audit upgrade cadence — check the velopack-rust changelog before each bump. Coordinate w/ #16 (when upstream `GithubSource` lands, that's the upgrade trigger worth taking).

## 26. `.gitignore` audit for `Releases/staging-*`

> **VERIFIED non-bug v0.4.18-alpha (S121).** `.gitignore:8` has `Releases/` covering the whole folder. No code change needed.

- **Where:** [scripts/release.ps1:90-92, 138](../scripts/release.ps1#L90) — creates `Releases/staging-$version/`, deletes on success.
- **Symptom:** Mid-run build failure leaves the staging dir on disk. Next `git add .` could accidentally commit build artifacts if `.gitignore` doesn't cover it.
- **Fix sketch:** Verify `.gitignore` has `Releases/staging-*` (or `Releases/` entirely if no other content lives there). One-line check.

---

## Backend hardening — migrated from AUDIT.md 2026-05-19

Open audit items folded in when AUDIT.md was archived to `docs/archive/AUDIT-fix-log.md`. All low-severity backend hardening; full fix-pass history (S81-S86 + Codex passes) lives in the archive.

## 28. Dual HTTP stacks — reqwest + ureq (LOW, deferred)

- **Where:** [src-tauri/Cargo.toml:41-44](../src-tauri/Cargo.toml#L41-L44).
- **Symptom:** Two HTTP transports compiled in. reqwest is async (used app-wide); ureq is sync (forced by `velopack` 0.0.1298's sync `UpdateSource`).
- **Fix sketch:** Revisit when velopack ships an async source. No action until then — pin acknowledged.

## 29. CSP allows `style-src 'unsafe-inline'` (LOW)

- **Where:** [src-tauri/tauri.conf.json:24](../src-tauri/tauri.conf.json#L24).
- **Symptom:** Inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** Switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end.

## 30. `capabilities/default.json` uses broad `core:default` (LOW)

- **Where:** [src-tauri/capabilities/default.json:7](../src-tauri/capabilities/default.json#L7).
- **Symptom:** `core:default` pulls in a superset of permissions; specific `core:*` perms actually in use are a subset.
- **Fix sketch:** Audit actual `invoke()` surface, pin specific `core:*` perms, drop the wildcard.

## 31. `capabilities/default.json` `opener:default` unscoped (LOW)

- **Where:** [src-tauri/capabilities/default.json:12](../src-tauri/capabilities/default.json#L12).
- **Symptom:** Opener plugin can launch any URL/path the renderer hands it.
- **Fix sketch:** Scope to known prefixes — update URL, docs URL, `https://github.com/Blazzer10200/*`.

## 33. `lib.rs::local_list_dir` missing profile containment (open from 2026-05-11)

- **Where:** `src-tauri/src/lib.rs` `local_list_dir` Tauri cmd.
- **Symptom:** Lacks `path_guard::validate_local_child` against active profile's `local_root`. Skipped at fix-pass time b/c the command has no `server_key` input — fixing requires a frontend contract change.
- **Fix sketch:** Add a `server_key` param, validate against that profile's roots. Frontend `LocalPane` calls pass the active server. Coordinated frontend + backend change.

Also accepted as INFO (no action expected): `path_guard.rs:21` Linux-only remote containment (matches Rift's deploy target); `bridge/mod.rs:57` token over loopback HTTP (documented); `edit/edit_trail.rs:75-80` subdir PID-race (collision astronomical after `short_id` widened to 8 bytes).

---

## Priority tiers

**S120 — Wave-2 backend MED + LOW sweep, ~40 issues SHIPPED v0.4.17-alpha (commit 0e91393).** SHIPPED set: #54 #55 #56 #68 #70 #72 #73 #75 #77 #79 #80 #83 #84 #86 #91 #93 #94 #95 #97 #98 #101 #105 #110 #111 #114 #116 #117 #118 #121 #122 #123 #124 #126 #128 #130 #132 #133 #136 #137 #138. Body blocks for those numbers pruned from Wave-1 detail 2026-05-21 (see `git log -- docs/ISSUES.md` for the deleted blocks). Wave-1 detail below = open items only.


**Tier 0 — verify before anything else**
- ~~#19 `apply_updates` autosync-stop~~ — verified 2026-05-19, non-bug (Tauri cmd already handles it; doc tightened)

**Tier 1 — ship blockers / data safety**
- #21 Zero test coverage — see #265 for plan; reality 35 tests in 10 files; uncovered HIGH-risk modules listed
- #9 Bridge token plaintext + IPC leak — 9.1+9.2 SHIPPED 2026-05-19; 9.3 (OS keyring) deferred to Phase 6
- #15 Unsigned Windows builds (adoption blocker)
- ~~#10 Silent TOFU on first sync~~ — SHIPPED 2026-05-19 (`require_pinned_fingerprint` guard)
- ~~#36~~ `save_server` overwrites server list — SHIPPED v0.4.14-alpha S113 (`map_err` propagation)
- **#37** API key plaintext in `assistant/config.json` (mirrors #9.3 deferred — Phase 6)
- **#38** `mcp-config.json` Windows DACL gap (continues #9.2 — Phase 6)
- ~~**#221**~~ model-flag injection — SHIPPED 2026-05-22 Lane 2 (uncommitted v0.4.30-alpha train)
- ~~**#227**~~ Ed25519/DSA scrub gap — SHIPPED 2026-05-22 Lane 2
- ~~**#237**~~ thinking_effort log-injection — SHIPPED 2026-05-22 Lane 2
- ~~**#238**~~ scrubUser Rust-side gap — SHIPPED 2026-05-22 Lane 2 (completes #8)
- ~~#42~~ — VERIFIED NOT A BUG by Wave 3 T (see verdict above). Closed.
- ~~#41~~ Bridge lock leak — SHIPPED v0.4.14-alpha S113 (`BridgeLockGuard` RAII w/ Drop)
- ~~#74~~ `walk_local` panic → mass ToPull — SHIPPED v0.4.14-alpha S113 (JoinError → SuspiciousEmptyAborted)
- ~~#219~~ No panic hook — SHIPPED v0.4.14-alpha S113 (`std::panic::set_hook` after LogForwarder)
- ~~#220~~ session_id traversal — SHIPPED v0.4.14-alpha S113 (`is_valid_session_id` UUID guard)

**Tier 2 — recurring friction**
- ~~#12 Manual 3-file version bump~~ — shipped 2026-05-19 (`scripts/bump.ps1`)
- ~~#3b STT send doesn't stop recognizer~~ — shipped 2026-05-19 (`stt.svelte.ts` `consume()`)
- ~~#1 Context counter semantics + double-write~~ — shipped 2026-05-19 (per-turn, result-only render)
- #14 No CI (deferred — pairs w/ #15 signing)
- ~~#34~~ Cancel-token ownership — SHIPPED v0.4.14-alpha S113 (u64 nonce identity compare; agent + 4-site fixup)
- ~~#35~~ Perm-heal spawn untracked — SHIPPED v0.4.14-alpha S113 (`track_background(h)`)
- ~~#39~~ Stop-flag race — SHIPPED v0.4.14-alpha S113 (post-PID re-check + mark_session_stopped)
- ~~#40~~ Single shared mcp-config.json — SHIPPED v0.4.14-alpha S113 (per-session `mcp-config-<id>.json` + `McpConfigGuard` Drop)
- ~~#43~~ is_pushing race — SHIPPED v0.4.14-alpha S114 (Err→true safer direction)
- ~~#44~~ stop_watch order — SHIPPED v0.4.14-alpha S114 (unwatch before remove)
- ~~#45~~ FS-drop counter — PARTIAL S114 (AtomicU64 + 100-drop Error escalation; AutoSyncStatus exposure deferred)
- ~~#46~~ pending_dir_reconcile order — SHIPPED v0.4.14-alpha S114 (kick before flag-clear)
- ~~#48~~ force_pull_now poison silent — SHIPPED v0.4.14-alpha S114 (diag Error + closing DriftScanResult)
- ~~#57~~ download_paths ghost CT — SHIPPED v0.4.14-alpha S114 (open_sftp before CT register)
- ~~#61~~ probe_server_fingerprint write probe — SHIPPED v0.4.14-alpha S114 (write_probe_root: None)
- ~~#63~~ SESSION_PIDS poison silent — PARTIAL S114 (into_inner recovery; orphan-kill on drop deferred)
- ~~#64~~ CLAUDE_EXE stale — SHIPPED v0.4.14-alpha S114 (Mutex<Option<Option<PathBuf>>> w/ is_file revalidate)
- ~~#65~~ save_config torn-write — SHIPPED v0.4.14-alpha S114 (tmp+rename pattern)
- **Remaining sync MEDs:** #47 (CT plumbing), #58 (silent list_recursive), #59 #60 (post-expansion guard refactor), #62 (token split — needs FE coord)
- ~~#139~~ `drainTick` rAF on dropped tab — SHIPPED v0.4.14-alpha S113 (`dropTab` calls `flushPendingText`)
- ~~#140~~ confirmMirrorApply bucket — SHIPPED v0.4.14-alpha S113 (re-filter at dispatch verified already correct; invariant doc'd)
- ~~#141-#142~~ ActivityFeed reactivity — SHIPPED v0.4.14-alpha S113 (`$state` + `untrack` — Agent C, clean)
- ~~#143~~ store-level per-tab fields — SHIPPED v0.4.14-alpha S114 (moved to TabState w/ delegating getters)
- ~~#144~~ closeTabsToRight/closeOtherTabs leak — SHIPPED v0.4.14-alpha S114 (dropTab + pruneTabUi per removed id)
- ~~#145~~ scheduleSave cross-tab save — SHIPPED v0.4.14-alpha S114 (per-tab saveTimer + tab-snapshot capture + flushNow iterates all tabs)
- ~~#150~~ Settings $effect draft clobber — SHIPPED v0.4.14-alpha S114 (untrack store reads; effect tracks only section)

**Tier 3 — UX + cleanup**
- #11 Settings dead UI cluster — PARTIAL shipped v0.4.12-alpha (mechanical bits done; placeholder card + SSH Keys empty + font-picker class rename remain)
- #2 Tool-block rendering rhythm
- ~~#22 Console noise~~ — shipped v0.4.12-alpha (assistant.svelte.ts probes removed, stt warns downgraded)
- ~~#6 Scrollbar collision~~ — shipped v0.4.12-alpha (Phase 3c — `+` button + `scrollbar-gutter: stable`)
- #5 Status indicator placement
- ~~#13 Release notes auto-flow~~ — shipped v0.4.12-alpha (`release.ps1` `--releaseNotes`)

**Tier 4 — strategic / longer-term**
- #4 App-wide UX consistency sweep
- #7 New-user onboarding flow + #24 ONBOARDING.md expansion
- #20 Hot-file split threshold (start w/ `lib.rs` per CLAUDE.md queue item (e))
- #16 Custom `GithubSource` → upstream Velopack
- #17 Two-repo split debt
- #8 Extend `scrubUser` to log forwarding
- #18 Round-trip publish verify
- #25 velopack-rust pin policy
- #23 MCP tool wildcarding documentation
- #26 `.gitignore` audit for `Releases/staging-*`
- #3a STT accuracy polish (separate epic — likely needs Whisper pipeline)
- #3c STT duplicate-on-stop (likely resolved by #3b fix, verify after)

**Tier 5 — backend hardening (LOW, opportunistic)**
- #27 `atomic_write_json` blocks Tokio worker
- #29 CSP `style-src 'unsafe-inline'`
- ~~#30~~ `core:default` superset — SHIPPED (see #230)
- ~~#31~~ `opener:default` unscoped — SHIPPED (see #229)
- #33 `local_list_dir` profile containment (needs FE contract change)
- #32 `transport/env.rs::hostname` shell-out (INFO)
- #28 Dual HTTP stacks (blocked on velopack async)
- **Wave-1 LOWs** #91-#134 — see entries above; mostly clippy-adjacent, doc gaps, perf nits
- **Wave-1 INFOs** #135-#138 — comment/ordering tweaks only

---

## Audit 2026-05-20 — Wave 1 (backend deep audit)

> 11 parallel `operator` agents over `src-tauri/src/`. Reports persisted at `state/audit-2026-05-20/{A..K}-*.md`; synthesis at `SYNTHESIS-wave1.md`. Format below is compressed (Where / Symptom / Fix); see reports for root-cause detail. Dupes collapsed where multiple agents found same site.

### HIGH (7 — 1 PARTIAL, 6 open)

## 81. `SyncSnapshot::set`/`forget` silently discard save errors
- **Where:** [state/sync_snapshot.rs:74](../src-tauri/src/state/sync_snapshot.rs#L74), [:80](../src-tauri/src/state/sync_snapshot.rs#L80)
- **Symptom:** Both methods `let _ = self.save_locked(&g)` — disk-write fail silently leaves in-memory state diverged from on-disk. Next restart loads stale data → phantom drift / false ToDelete/ToPull. `replace_under` at L125 correctly propagates.
- **Fix:** Change signatures to `-> std::io::Result<()>`; propagate via `?`; callers surface via DiagBus.
> PARTIAL v0.4.16-alpha S119 (uncommitted) — `set` + `forget` now match the save_locked Result; failures emit `log::error!` with remote_path context. Signatures kept `-> ()` to avoid touching every caller (hot path: flush). Full `Result<(), io::Error>` propagation + DiagBus surface deferred — log is enough to diagnose; the silent-divergence case is closed.

## 89. `download_file` buffers entire remote file into memory
- **Where:** [sftp/transfer.rs:231](../src-tauri/src/sftp/transfer.rs#L231), [:337](../src-tauri/src/sftp/transfer.rs#L337)
- **Symptom:** `sftp.read(remote_path)` loads full bytes into `Vec<u8>` before writing local. Hundreds-of-MB asset files (FiveM map packs, .ytd) OOM on low-RAM servers / mobile WiFi.
- **Fix:** For files >16 MB, `sftp.open` + stream chunks to local tmp via `AsyncRead`. Deferred-complexity.

## 96. ~~`apply_selected` bypasses buffered feed~~ — VERIFIED SHIPPED
- [auto_sync.rs:1547-1550](../src-tauri/src/sync/auto_sync.rs#L1547-L1550) routes the WARN row through `engine.log_activity(...)` w/ `#96` cited inline.

## 99. ~~`flush_batch dispatched` count includes Requeued~~ — VERIFIED SHIPPED
- `flush_batch` now returns `(dispatched, ok, fail)` ([auto_sync/flush.rs:141-143](../src-tauri/src/sync/auto_sync/flush.rs#L141-L143)); `force_push_now` cache-clear gates on `ok > 0` ([auto_sync.rs:901](../src-tauri/src/sync/auto_sync.rs#L901)).

## 107. ~~`start_autosync` status sampled before prev engine fully stopped~~ — VERIFIED SHIPPED
- [commands/sync.rs:473-476](../src-tauri/src/commands/sync.rs#L473-L476) — `engine.status()` now sampled AFTER `prev.stop().await` + slot replacement, w/ `#107` cited inline.

## 109. ~~`bootstrap_list_files` accepts dead `_local_root` IPC param~~ — SHIPPED 2026-05-25
- Backend signature already pruned (cmd now at [commands/sftp.rs:590](../src-tauri/src/commands/sftp.rs#L590) post-split, only `server_key`). FE caller in [Bootstrap.svelte:94](../src/lib/components/dialogs/Bootstrap.svelte#L94) updated to drop the `localRoot` IPC field.

## 115. ~~`session-lost` re-broadcasts full prompt~~ — VERIFIED SHIPPED
- [assistant/mod.rs:2241-2248](../src-tauri/src/assistant/mod.rs#L2241-L2248) — emits only `{ session_id }`; `#115` cited inline.

### LOW (1)

## 134. `assistant_auth_probe` two-spawn TOCTOU window for CLI replacement
- **Where:** [assistant/mod.rs:592-658](../src-tauri/src/assistant/mod.rs#L592-L658)
- **Fix:** Single `claude auth status --version` call if CLI supports it; OR parallel via `tokio::join!`.

### INFO (1)

## 135. ~~`force_push_now` promotion log out-of-order~~ — VERIFIED SHIPPED
- Post-refactor ordering: promotion `log::debug!` lands at [auto_sync.rs:842](../src-tauri/src/sync/auto_sync.rs#L842), `flush_all_now` runs at L890. Correct order.

---

## Audit 2026-05-20 — Wave 2 (frontend deep audit)

> 8 parallel `operator` agents over `src/lib/`. Reports persisted at `state/audit-2026-05-20/{L..S}-*.md`; synthesis at `SYNTHESIS-wave2.md`. Two agents (O bail-recovered; S wrote to wrong path then was relocated). Same compressed format as Wave 1.

### HIGH (4)

## 146. ~~`mutateStreaming` rebuilds full messages array~~ — VERIFIED SHIPPED
- [assistant.svelte.ts:743-753](../src/lib/state/assistant.svelte.ts#L743-L753) — caches `streamingMsgIdx`; direct index write when index matches, full `.map` only as fallback. `#146/#234` cited inline. Also closes #234.

## 147. ~~`ensureThinkingFromEnvelope` reference-equality on `$state` proxies~~ — VERIFIED SHIPPED
- [assistant.svelte.ts:846-852](../src/lib/state/assistant.svelte.ts#L846-L852) — match by `startedAt` stable key; `#147` cited inline.

## 148. ~~`handleTurnComplete` microtask races tab switch~~ — VERIFIED SHIPPED
- [assistant.svelte.ts:2907-2917](../src/lib/state/assistant.svelte.ts#L2907-L2917) — captures `capturedConvoId`; re-queues onto original tab if convo changed before microtask fires. `#148` cited inline.

## 149. ~~`openTab` race against `deleteConversation`~~ — VERIFIED SHIPPED
- [assistant.svelte.ts:2322-2327](../src/lib/state/assistant.svelte.ts#L2322-L2327) — after `refreshConversations`, explicitly closes any tab pointing at the deleted id. `#149` cited inline.

## 153. `syncNow()` busy clears before trailing rescan settles — 1.2s race window
- **Where:** [sync-page.svelte.ts:431,462-464](../src/lib/state/sync-page.svelte.ts#L431)
- **Symptom:** Double-click Sync within 1.2s starts second `syncNow()` while prior `rescan()` is still pending → overlapping reconciles.
- **Fix:** Chain `rescan()` into the same promise chain (keep `busy=true` until it returns), OR debounce Sync button for 1.5s post-success.

## 171. `connecting` flag stuck `true` if TOFU modal dismissed without confirm/cancel
- **Where:** [connection.svelte.ts:207-233](../src/lib/state/connection.svelte.ts#L207-L233)
- **Symptom:** Fingerprint-probe branch `return`s early without `finally` clearing `connecting`. AppShell `fingerprintHandled` effect normally pipes back the result, but if `askConfirm` never resolves (component unmount mid-dialog), `connecting` stays true → StatusBar button locked.
- **Fix:** Verify `cancelFingerprint` reaches every dismiss path; OR wrap in try/finally that clears `connecting` if neither callback fired.

### LOW (28)

### INFO (5)

## 217. `EmptyState.pick()` may not focus textarea if draft unchanged
- **Where:** [EmptyState.svelte:76-78](../src/lib/components/assistant/EmptyState.svelte#L76-L78)
- **Fix:** CDP-verify; if needed, add explicit `tick().then(() => ta?.focus())`.

---

## Audit 2026-05-20 — Wave 3 (cross-cutting)

> 8 parallel `operator` agents. Reports persisted at `state/audit-2026-05-20/{T..AA}-*.md`; synthesis at `SYNTHESIS-wave3.md`. Six of eight bailed skeleton-only and were SendMessage-recovered. Y was re-spawned tighter after the agent-dispatch-guard hook blocked the original 38-line prompt.

### Wave 1 #42 cross-verification — NOT A REAL BUG

Agent T verified via [auto_sync/watch.rs:245](../src-tauri/src/sync/auto_sync/watch.rs#L245) + [sync/ignore.rs:91-97,157](../src-tauri/src/sync/ignore.rs#L91-L157). `classify()` extracts basename via `rsplit('/').next()` before `.rift-conflict.` substring check. Absolute vs relative path makes no difference for the conflict-copy marker rule. **#42 is closed/INFO.** Frontend safety chip from #158 is still useful but not security-critical.

### HIGH (2)

## 222. ~~`stderr_task.await.unwrap_or_default()` drops JoinError~~ — VERIFIED SHIPPED
- [assistant/mod.rs:1339-1342, 2204+](../src-tauri/src/assistant/mod.rs#L1339-L1342) — both sites now `log::error!` + surface `(stderr drain task panicked: {e})`.

## 223. ~~`create_dir_all` for download staging silently ignored~~ — VERIFIED SHIPPED
- All 3 sites (now [commands/sftp.rs:191, 211, 219](../src-tauri/src/commands/sftp.rs#L191) post-split) wrap in `if let Err(e)` + `log::warn!("download mkdir ...")`.

## 224. ~~`try_read_lock` `.ok()?` conflates absent-lock vs SFTP-error~~ — VERIFIED SHIPPED
- [lock_presence.rs:48-54](../src-tauri/src/sync/lock_presence.rs#L48-L54) — `LockReadOutcome` enum (`Present`/`Absent`/`Error`) distinguishes the three. Doc comment cites `#224`.

## 225. ~~`eprintln!` in sync handlers + drift scanner~~ — SHIPPED 2026-05-25
- 14 `eprintln!` in `sync/auto_sync.rs` (force_push_now / force_pull_now / reconcile) → `log::debug!` (most) + `log::info!` (reconcile summary) + `log::warn!` (no-watched-folders). Drift-scanner duplicate `eprintln!` deleted (kept the `emit_with_fields` follower). Verify: `Grep eprintln! src-tauri/src/sync/` returns zero.
- **Remaining eprintlns audited 2026-05-25**: `sftp/list.rs:389`, `sftp/ops.rs:88` already cleaned. `profile/mod.rs:248` + `state/sync_snapshot.rs:370,377` are inside `#[test] #[ignore]` blocks — test diagnostic output, not production code paths. Leave as-is.

## 226. ~~Broadcast bus lag silently counted~~ — VERIFIED SHIPPED
- [diagnostics/mod.rs:481](../src-tauri/src/diagnostics/mod.rs#L481) — `log::warn!("diag bus lagged: {n} events dropped")` lands after `record_bus_lag(n)`.

## 228. `dialog:default` re-scoped — plugin is in use, not dead
- **Where:** 5 prod frontend sites use `@tauri-apps/plugin-dialog` (ProfileSetup.svelte, ServerAdd.svelte, SSHKeySetup.svelte, assistant.svelte.ts).
- **Status:** Original "remove plugin" fix is NOT viable (investigated 2026-05-24). Re-scoped to: audit dialog call-sites, replace w/ native Svelte dialogs or remove the call-sites first, then drop plugin.

## 229. ~~`opener:default` too broad~~ — VERIFIED SHIPPED 2026-05-25
- [capabilities/default.json](../src-tauri/capabilities/default.json) already lists the 3 explicit `opener:allow-*` perms; no `opener:default`. Closes #31 + #229.

## 230. ~~`core:default` bundles unused~~ — VERIFIED SHIPPED 2026-05-25
- [capabilities/default.json](../src-tauri/capabilities/default.json) pinned to `core:event:default` + `core:path:default` + `core:webview:default` + `core:window:default` + 4 explicit `core:window:allow-*`. `core:app`/`core:menu`/`core:resources` excluded. `core:path:default` retained — `Settings.svelte` uses `appConfigDir`/`appLogDir`. Closes #30 + #230.

## 231. Cargo.toml version regex not anchored to `[package]` section
- **Where:** [scripts/release.ps1:34](../scripts/release.ps1#L34), [scripts/bump.ps1:57-63](../scripts/bump.ps1#L57-L63)
- **Symptom:** First `version = "..."` in Cargo.toml is currently the package — fragile to layout change. Workspace merge or dep-block-before-`[package]` would silently write the wrong field.
- **Fix:** Anchor pattern: `(?ms)\[package\].*?^\s*version\s*=\s*"([^"]+)"`.

## 232. `vpk upload github` missing `--channel` — implicit `win` default coupling
- **Where:** [scripts/release.ps1:166-178](../scripts/release.ps1#L166-L178) + [update_service.rs:173](../src-tauri/src/update_service.rs#L173) `UpdateManager::new(src, None, None)`
- **Symptom:** Both sides default to `win` but neither documents it. Multi-channel rollout would silently diverge w/o compile- or runtime-error.
- **Fix:** Explicit `--channel win` to vpk + `Some("win")` to `UpdateManager::new`.

## 233. `Releases/` blanket-ignored but `assets.win.json` + `releases.win.json` may be in working tree
- **Where:** [.gitignore:1](../.gitignore#L1) + `Releases/*.json` artifacts
- **Symptom:** Local pack regenerates feed files; if accidentally shipped from local rather than GitHub-uploaded, clients hit stale packages.
- **Fix:** Verify `git ls-files Releases/` empty; add comment in release.ps1 that local files are pack-state only, canonical feed is GitHub asset.

## 234. ~~Re-cite of #146~~ — VERIFIED SHIPPED (see #146).

## 240. ~~`aborted_shrunk()` mutex-poison silently returns empty vec~~ — VERIFIED SHIPPED
- [auto_sync.rs:1328-1334](../src-tauri/src/sync/auto_sync.rs#L1328-L1334) — explicit `Err(p)` arm logs + recovers via `p.into_inner().clone()`.

## 241. ~~MCP bridge socket timeouts swallowed~~ — VERIFIED SHIPPED
- [mcp_server.rs:35-40](../src-tauri/src/assistant/mcp_server.rs#L35-L40) — `set_read_timeout`/`set_write_timeout` failures now `log::warn!` with label.

## 242. ~~MCP bridge `stream.flush().ok()` drops errors~~ — VERIFIED SHIPPED
- 4 bridge flush sites ([mcp_server.rs:407, 590, 676, 774](../src-tauri/src/assistant/mcp_server.rs#L407)) all now `.map_err(|e| format!("bridge flush: {e}"))?`.

## 243. ~~STT `from_slice().unwrap_or_default()` accepts corrupt config~~ — VERIFIED SHIPPED
- [stt/mod.rs:138-141](../src-tauri/src/stt/mod.rs#L138-L141) — parse failure now logs `stt-config parse failed ({e}), using defaults` before defaulting.

## 244. ~~`edit_trail.rs` destroys trail on SFTP error~~ — VERIFIED SHIPPED
- [edit_trail.rs:56-67](../src-tauri/src/sync/edit_trail.rs#L56-L67) — `ReadOutcome` enum (`Present`/`Absent`/`Error`); error arm logs + early-returns to preserve remote history.

## 247. No tracing spans on hot paths (flush, scan, SFTP)
- **Where:** [auto_sync/flush.rs:35](../src-tauri/src/sync/auto_sync/flush.rs#L35), [drift_scanner.rs:122](../src-tauri/src/sync/drift_scanner.rs#L122), [sftp/transfer.rs:20](../src-tauri/src/sftp/transfer.rs#L20)
- **Symptom:** No structured timing or hierarchical causality. Only `latency_ms` in `log_activity_rich`.
- **Fix:** Short-term: entry/exit `log::debug!` w/ timing. Long-term: `tracing` + `tracing-log` bridge.

## 248. Frontend connection errors never reach diag bus
- **Where:** [connection.svelte.ts:230,475](../src/lib/state/connection.svelte.ts#L230)
- **Symptom:** `connect failed` + `auto-reconnect failed` only `console.error`'d → Diagnostics panel never shows them.
- **Fix:** Add `diag_log_frontend_error` Tauri cmd publishing `DiagStage::System` / `DiagLevel::Error`.

## 249. `diag_state_pump` emits every 500ms regardless of subscribers
- **Where:** [lib.rs:343,386](../src-tauri/src/lib.rs#L343)
- **Symptom:** Persistent background serialization+emit even when Diagnostics tab closed.
- **Fix:** `diag_active: AtomicBool` toggled by subscribe/unsubscribe; OR pull model via `diag_get_state`.

## 250. ~~STT console.debug calls~~ — VERIFIED SHIPPED 2026-05-25
- Re-grep of `src/lib/state/stt.svelte.ts` returns zero `console.*` matches. Both #22 and #250 closed.

## 251. Release staging copy hardcoded to 2 files — DLL/redistributable gap
- **Where:** [scripts/release.ps1:126-130](../scripts/release.ps1#L126-L130)
- **Symptom:** Only `rift-tauri.exe` + `icon.ico` copied. Future Tauri/WebView2 redistributables would be silently absent from `vpk pack` payload.
- **Fix:** Comment intent explicitly, OR add `*.dll` glob.

## 252. `GithubSource::get_release_feed` per_page=10 — pagination gap
- **Where:** [update_service.rs:229](../src-tauri/src/update_service.rs#L229)
- **Symptom:** `Releases/` has 14 tags already; if newest eligible doesn't carry `releases.win.json`, walker errors out instead of paginating.
- **Fix:** `per_page=50` + pagination loop following `Link: <url>; rel="next"`.

## 253. `release.ps1` `Read-Host` silently exits 1 in CI pipe (no TTY)
- **Where:** [scripts/release.ps1:86-92](../scripts/release.ps1#L86-L92)
- **Symptom:** Non-TTY stdin returns empty string → exit 1 w/ no log explaining why.
- **Fix:** Replace w/ `Write-Host` + `-Force` parameter; explicit error message.

## 254. `rendered` $derived.by in ActivityFeed — full O(n) regroup on every event
- **Where:** [ActivityFeed.svelte:218](../src/lib/components/activity/ActivityFeed.svelte#L218)
- **Symptom:** During sync burst, full re-derivation chain triggers every event.
- **Fix:** Stable base + burst accumulator; or debounce $derived via setTimeout batching.

## 255. `DriftSummaryCard` $derived.by — O(entries) Map rebuild per drift event
- **Where:** [DriftSummaryCard.svelte:18](../src/lib/components/sync/DriftSummaryCard.svelte#L18)
- **Fix:** Move group-by into store as derived field; component reads pre-aggregated.

## 259. `compute_sha1` in drift scanner sequential per file (SSH exec round-trip)
- **Where:** [drift_scanner.rs:384,422,445](../src-tauri/src/sync/drift_scanner.rs#L384)
- **Symptom:** `get_remote_sha1` calls SSH exec serially within scan loop. Up to 10×RTT wall time per scan.
- **Fix:** Collect into batch; `FuturesUnordered` bounded by `hash_budget`.

## 262. SyncPage / Settings / Diagnostics use `onMount/onDestroy` instead of `$effect` (HMR-unsafe)
- **Where:** [SyncPage.svelte:66-67](../src/lib/components/sync/SyncPage.svelte#L66-L67), [Settings.svelte:43](../src/lib/components/settings/Settings.svelte#L43), [Diagnostics.svelte:39](../src/lib/components/diagnostics/Diagnostics.svelte#L39)
- **Symptom:** Lifecycle-correct but inconsistent w/ adopted `$effect`-return pattern. Three sites left.
- **Fix:** Migrate to `$effect(() => { ...add...; return () => ...remove... })`.

## 263. `UpdateStore` listeners are intentional singletons but undocumented
- **Where:** [updates.svelte.ts:171](../src/lib/state/updates.svelte.ts#L171)
- **Fix:** Add comment: `// intentional: app-lifetime singleton`.

## 264. `deleteThresholdHint()` single-use helper — inline candidate (INFO)
- **Where:** [SyncPage.svelte:195](../src/lib/components/sync/SyncPage.svelte#L195)
- **Fix:** Leave for readability or inline. No action required.

### Z — Test gap plan (single tracker entry)

## 265. Test strategy + priority ranking
- **Where:** plan content below (audit shards purged 2026-05-21).
- **Symptom:** ISSUES #21 said "zero tests"; reality: 35 `#[test]` fns in 10 files (ignore.rs 14, auto_sync/path.rs 6, bootstrap 4, others). Uncovered HIGH-risk: `drift_scanner.rs`, `state/sync_snapshot.rs`, `sftp/transfer.rs`, `auto_sync/flush.rs`, `assistant/remote_bridge.rs`, `assistant/mod.rs`, `assistant.svelte.ts`, `sync-page.svelte.ts`, `path_guard.rs`, `drift_watcher.rs`.
- **Structural blockers:** No `SftpOps` trait (concrete `SftpClient`); `AutoSyncEngine` needs `AppHandle`; `SyncSnapshot::new` writes to `~/.rift/`; `BRIDGE: OnceLock` set-once. Recommended sequencing: Wave A (no infra) → B (`SyncSnapshot::for_path` + `tempfile`) → C (`SftpOps` trait + mock SFTP) → D (`DiagSink` trait + Tauri `mock_builder`).
- **Fix sketch:** Follow Wave A first — `path_guard` containment, `sync_snapshot` serialization, flush circuit-breaker math, `sync-page.svelte.ts` bucket display, `assistant.svelte.ts` usage accumulator. Updates #21.

### AA top-10 clippy perf lints (table for triage)

| # | Rule | Count | Representative | Fix |
|---|------|-------|----------------|-----|
| 1 | `format_push_string` | 39 | [mcp_server.rs:185](../src-tauri/src/assistant/mcp_server.rs#L185) | `write!(out, ...)` (see #258) |
| 2 | `redundant_closure_for_method_calls` | ~12 | [mcp_server.rs:205](../src-tauri/src/assistant/mcp_server.rs#L205) | Method-ref instead of closure |
| 3 | `map_unwrap_or` | 3 | [mcp_server.rs:174](../src-tauri/src/assistant/mcp_server.rs#L174) | `map_or(default, f)` |
| 4 | `needless_pass_by_value` | 3 | [assistant/mod.rs:447](../src-tauri/src/assistant/mod.rs#L447) | `&str` over `String` |
| 5 | `uninlined_format_args` | ~8 | [mcp_server.rs:188](../src-tauri/src/assistant/mcp_server.rs#L188) | Inline vars in format str |
| 6 | `unnecessary_sort_by` | 1 | [assistant/mod.rs:442](../src-tauri/src/assistant/mod.rs#L442) | `sort_by_key(\|b\| Reverse(...))` |
| 7 | `map_unwrap_or` (Option) | 1 | [assistant/mod.rs:428](../src-tauri/src/assistant/mod.rs#L428) | `map_or` |
| 8 | `redundant_else` | 1 | [drift_scanner.rs:245](../src-tauri/src/sync/drift_scanner.rs#L245) | Flatten else |
| 9 | `manual_let_else` | ~8 | [mcp_server.rs:164](../src-tauri/src/assistant/mcp_server.rs#L164) | `let Ok(x) = ... else { ... }` |
| 10 | `match_same_arms` | 1 | [mcp_server.rs:642](../src-tauri/src/assistant/mcp_server.rs#L642) | Merge arms |

---
