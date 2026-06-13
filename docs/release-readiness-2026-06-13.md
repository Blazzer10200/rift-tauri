# Rift — Release-Readiness Audit (2026-06-13)

> Pre-wider-distribution audit. Goal: surface everything that should be verified or fixed before Rift goes out to more than a handful of buddies.
> **Version audited:** `0.9.2` (lockstep confirmed across `package.json` / `Cargo.toml` / `tauri.conf.json`). Backend findings were gathered at `0.9.1` via 5 parallel read-only auditors and re-checked against current `main`.
> **Method:** 5 subsystem auditors (update chain · auth/CLI-spawn · MCP security · release config · crash robustness). Load-bearing claims re-verified by hand — see [Verification log](#verification-log).
>
> **Re-verified firsthand 2026-06-13** (4 parallel verifiers + manual, against current `main`): every 🔍 item below was re-grepped. Net changes — **RR-3 REVERSED** (npmjs URL is a live runtime fetch, NOT dead — do not remove), **B5 + B9 cite files that no longer exist** (deleted in the minimal-core strip), **B3 has partial mitigation**, **RR-2 mechanism refined**. CI: v0.9.2 release run `27477310839` = **success**, prod auto-update path live. Inline corrections tagged **[RV]**.

## Legend

- **Verify status** — ✅ confirmed firsthand this audit · 🔍 agent-reported, not independently re-checked · 🧪 needs runtime/prod-build confirmation
- **Tier** — `T1` ship-blocker for wider release · `T2` should-fix soon · `T3` conscious decision · `T4` robustness backlog / low

---

## Verdict

**Structurally sound. Two real ship-blockers, a handful of should-fix items, several conscious decisions.**

The hard stuff is right: workspace-scoped file access is genuinely locked, the self-update reap-before-exit dance is correct, version lockstep + release pipeline are well-guarded, there are no `panic!`/`unreachable!` anywhere, and every `.unwrap()` in a hot-path file is test-only. The gaps are mostly at the **new-user edge** (first-run with no CLI) and in **field observability** (you're blind to crashes once it's on someone else's machine).

---

## What's solid (verified strong)

| Area | Finding | Ref | Status |
|---|---|---|---|
| MCP file access | `resolve_under_roots` canonicalizes then `starts_with(root)`, fail-closed; uncanonicalizable roots dropped at load; missing paths error before any I/O; TOCTOU-safe capped read | `mcp_server.rs:92`,`149` | ✅ |
| MCP git surface | git cwd pinned to workspace root, no override param; `GIT_DIR`/`GIT_WORK_TREE`/etc env-stripped per call; `GIT_SSH_COMMAND` BatchMode; force-push hard-rejected | `git_local.rs:60`,`148`,`398` | 🔍 |
| Trust gating | git-write tools gated `trust_at_least("standard")` at BOTH `tools/list` and dispatch; trust frozen via `OnceLock` (no in-process escalation) | `mcp_server.rs:674`,`803` | 🔍 |
| Self-update apply | `wait_exit_then_apply_updates` → `kill_all_session_children` → taskkill sweep → `app.exit(0)`, correct order; reap prevents stray `rift-tauri.exe` MCP child locking `current/` | `update_service.rs:219`–`272` | ✅ |
| Update error surfacing | `apply_pending_update` propagates `Err` via `?` → frontend rejects → `downloadError` + sticky toast | `commands/update.rs:125`; `updates.svelte.ts:232` | 🔍 |
| Update cadence | launch + 6h (`AUTO_MS`); auto-tick skips while downloading/installing/dialog-open | `updates.svelte.ts:106`,`324` | 🔍 |
| Velopack pin | `velopack = "=1.2.0"`; `release.ps1` asserts `vpk` CLI == crate version; CI pins vpk `1.2.0` | `Cargo.toml:79`; `release.ps1:96` | 🔍 |
| Version lockstep | all three files + Cargo.lock at `0.9.2`; `bump.ps1` cross-checks, `release.ps1` preflight hard-throws on mismatch + tag guard | (see above) | ✅ |
| Release CI | tag-driven (`release.yml` on `v*`); PR/push runs `cargo check/test/audit/machete` + `svelte-check`/`vitest`/`npm audit` | `.github/workflows/*` | 🔍 |
| Auth detection | `is_auth_rejection` covers 401/auth_error/invalid-key on both stdout-frame + stderr-exit; actionable banner (Sign in / Open Settings / Re-check); `startLogin()` polls `refreshAuth` 2.5s × 3min | `turn.rs:992`,`1278`,`1298`; `AssistantPane.svelte:374` | 🔍 |
| Missing-CLI handling | resolves to clear error string `"claude CLI not on PATH…"`, not a hang/panic; spawn failures map to string errors | `turn.rs:585`,`847` | 🔍 |
| Panic safety | no `panic!`/`unreachable!` in codebase; all hot-file `.unwrap()`/`.expect()` are `#[cfg(test)]`; panic hook installed pre-builder, scrubs home-dir/keys, logs to file + DiagBus | `lib.rs:38`; per-file counts | 🔍 |
| Capabilities | drag-region granted; no `shell:allow-execute`, no `fs:` allow-all; clipboard read-only | `capabilities/default.json` | 🔍 |

---

## Findings

| ID | Title | Tier | Verify |
|----|-------|------|--------|
| RR-1 | New-user auth dead-end (skip-onboarding + no CLI) | T1 | ✅ |
| RR-2 | No field crash observability; startup-panic silent to user | T1 | ✅ **[RV]** |
| ~~RR-3~~ | ~~CSP `connect-src` allows `registry.npmjs.org` (dead leftover)~~ **REVERSED — load-bearing, do NOT remove** | ~~T2~~ | ❌ **[RV]** |
| RR-4 | `opener:allow-open-path` scoped to `"**"` | T2 | ✅ **[RV]** |
| RR-5 | #29 CSP nonce/`unsafe-inline` — prod-build verify pending | T2 | 🧪 |
| RR-6 | Silent steer loss on init-handshake backlog | T2 | ✅ **[RV]** |
| RR-7 | Empty error bodies on oneshot (enhance/title) failures | T2 | ✅ **[RV]** |
| RR-8 | Permission Allow/Deny bar never exercised live | T2 | ✅ code / 🧪 runtime **[RV]** |
| RR-9 | Zombie download task after stall can flip `downloaded=true` | T2 | ✅ **[RV]** |
| RR-10 | `ALLOW_PRERELEASE = true` — field users auto-pull alpha tags | T3 | ✅ |
| RR-11 | Unsigned exe → Windows SmartScreen friction | T3 | — |
| RR-12 | Two-repo source↔releases manual sync | T3 | 🔍 |
| RR-13 | Robustness backlog (silent-swallows + 1 expect race) | T4 | 🔍 |
| RR-14 | Update-chain minor: check-timeout thread leak + stale module comment | T4 | 🔍 |

---

### T1 — Ship-blockers

#### RR-1. New-user auth dead-end ✅
- **Where:** `AssistantWelcome.svelte:163`–`176`; `Composer.svelte:329`,`817`; `OnboardingFlow.svelte:129`.
- **Symptom:** A user who hits "Skip setup" with no CLI installed lands on the `needsAuth` welcome card, which is **static** — a `https://claude.com/download` link + "run `claude login`" text + a Settings hint. There is no in-app Sign-in button and no `startLogin()` call on this path. The *interactive* Sign-in button lives only in the post-turn error-recovery banner — but `fire()` and the send button are both disabled unless auth pill is green/yellow, so a red-pill user **can never fire a turn**, so that banner never renders. The remediation loop is closed off.
- **Extra:** `AssistantWelcome.svelte:174` says "…and **hit refresh**" but there is no refresh control in that block — a broken affordance pointing at a button that doesn't exist.
- **Fix sketch:** Add a live "Sign in" button (calls `assistant.startLogin()`) + a "Re-check" button directly on the `needsAuth` welcome card, mirroring the recovery banner. This is the single most-traveled new-user path and it currently terminates in a static hint.

#### RR-2. No field crash observability ✅ **[RV]**
- **Where:** `diagnostics/mod.rs:119`,`185`; `lib.rs:38`,`221-222`.
- **Symptom:** Crash persistence is a single 5 MB rotating `rift.log` + one `.log.old` backup (rotation at `:185`) — a second crash in a session overwrites the backup before the user can grab it. No Sentry / minidump / structured crash file (confirmed: no such pattern anywhere in the file). A **startup** panic (Tauri `.build().expect()` at `lib.rs:221-222`) fires the hook, but the frontend pump (`spawn_frontend_pump`) is wired inside `.setup()` which runs *during* `.build()` — so a panic before setup completes never reaches the UI.
- **[RV] mechanism correction:** the panic hook (`lib.rs:38`, installed before `.build()`) emits via `log::error!` + `diagnostics::emit_with_fields`, **not** a direct DiagBus `tx.send` — the original "`tx.send` returns Err and is dropped" framing was imprecise. The structural risk (pre-setup panic invisible to user, captured only in `rift.log`/stderr) is real and stands; only the exact failure mechanism was misdescribed.
- **Why it matters for wider release:** once it's on a stranger's machine you get nothing back unless they manually find and send the log. You'll be debugging blind.
- **Fix sketch (pick one):** (a) write a dedicated `crash-<ts>.txt` on panic (don't reuse the rotating log slot); (b) bump `.log.old` retention to N files; (c) minimal opt-in remote crash sink. Even (a) alone closes the worst gap.

### T2 — Should-fix soon

#### ~~RR-3. CSP `connect-src` allows `registry.npmjs.org`~~ ❌ REVERSED **[RV]**
- **Where:** `tauri.conf.json:27`.
- **ORIGINAL CLAIM WAS WRONG.** Re-grep found a **live runtime fetch**: `src/lib/state/cliUpdate.svelte.ts:30` — `const LATEST_URL = ` https://registry.npmjs.org/${PKG}/latest ` ` — this is the in-app Claude-CLI update check. The CSP `connect-src` entry is **load-bearing**; removing it would break CLI-update detection with a CSP violation. The first auditor only grepped `package-lock.json` and missed the `src/` caller. **Action: do NOT remove. Close as not-a-bug.**

#### RR-4. `opener:allow-open-path` scoped to `"**"` 🔍
- **Where:** `capabilities/default.json:24`–`29`.
- **Symptom:** Broadest capability present — `allow-open-path` with `"path": "**"` can open any path, incl. arbitrary executables, via the OS opener. `reveal-item-in-dir` at `**` is benign; `open-path` is the one to scope.
- **Fix sketch:** Audit callers of the open-path command; narrow the scope to the directories Rift actually needs to open (workspace + downloads), or drop `open-path` if only reveal is used.

#### RR-5. #29 CSP prod-build verify 🧪
- **Where:** `tauri.conf.json:28` (`dangerousDisableAssetCspModification: ["style-src"]`).
- **Status:** Fix is in-tree but dev builds don't exercise Tauri's asset-CSP rewrite. On the next **prod** build, confirm: Svelte transitions animate, the update progress-bar fills, and zero CSP violations in console.

#### RR-6. Silent steer loss 🔍
- **Where:** `turn.rs:964` (`let _ = stdin.write_all(&env).await` draining `steer_pending`).
- **Symptom:** A steer queued during the init-handshake backlog drops its write error silently — the steer can be lost with no signal to user or frontend.
- **Fix sketch:** Surface the write error (emit an ERROR_EVENT or re-queue) instead of `let _ =`.

#### RR-7. Empty oneshot error bodies 🔍
- **Where:** `oneshot.rs:381`,`521` (`stderr_task.await.unwrap_or_default()`).
- **Symptom:** If the stderr-drain task panics, enhance/title failures surface with an empty body ("enhance cancelled" / "title generation failed" with no reason). `turn.rs:1184` already surfaces its version; `oneshot.rs` doesn't.
- **Fix sketch:** Mirror turn.rs — capture and surface the join error.

#### RR-8. Permission Allow/Deny bar unexercised 🔍
- **Symptom:** The full `can_use_tool` → control-response → `PermissionBar` Allow/Deny round-trip has never been watched fire (it only fires at `trust_level=standard`, and the dev box stays at derived trust). Safety-critical path, unverified.
- **Fix sketch:** Pin `trust_level=standard` on a throwaway repo, trigger a git-write, watch one Allow + one Deny round-trip.

#### RR-9. Zombie download task after stall 🔍
- **Where:** `commands/update.rs:95`–`102`; `update_service.rs:211`.
- **Symptom:** When the stall watchdog fires, the underlying `spawn_blocking` download isn't cancelled — it can finish and flip `g.downloaded = true` after the error was returned. A user who retries could then reach `apply` against a package written by a zombie task. Bounded by the `Inner` mutex but a real race window.
- **Fix sketch:** Carry a cancellation token into the blocking download, or invalidate `g.downloaded` when a stall error is returned.

### T3 — Conscious decisions (your call, not bugs)

#### RR-10. `ALLOW_PRERELEASE = true` ✅
- **Where:** `update_service.rs:35`,`304`.
- **Implication:** Field installs auto-pick alpha/beta tags as "newest." A bad pre-release tag reaches everyone before a final is cut. Fine if intentional for a buddy/alpha cohort; revisit before a "stable" audience.

#### RR-11. Unsigned executable
- Signing was previously declined (SmartScreen friction not worth a fee for self-distributed alpha). "More people" = more strangers hitting the unsigned-app SmartScreen warning. Re-decide consciously; no code change needed either way.

#### RR-12. Two-repo source↔releases sync 🔍
- **Where:** `release.ps1:202` (`Blazzer10200/rift-releases`); `update_service.rs` GithubSource.
- Every release needs manual sync between private source and public releases repo; contributors can't test the update path against real source. Collapse only if the source repo goes public (tracked as issue #17).

### T4 — Robustness backlog (low, but log them)

All 🔍 (agent-reported, not individually re-verified):

- **B1** `turn.rs:931,960,967,1065` — `let _ = stdin.flush()` swallows broken-pipe; diagnosis delayed until next stdout read. ✅ **[RV]** (all four sites exact)
- **B3** `turn.rs:868,881` — `let _ = child.start_kill()` swallowed; on Windows a registered PID could outlive and risk a recycled-PID taskkill. **[RV] partial-mitigation note:** the `:881` path clears the PID via `clear_session_pid` at `:884`; the `:868` stop-during-spawn path does **not** — it re-arms `mark_session_stopped` and leaves the PID registered, so a silent `start_kill` failure there is the real exposure. Scope the fix to the `:868` arm.
- **B4** `turn.rs:366` — `let _ = app.emit(PERMISSION_EVENT…)`; if window closed mid-turn the permission request vanishes but the CLI still gets a response. ✅ **[RV]**
- ~~**B5** `auth_update.rs:56`~~ — **❌ [RV] FILE DOES NOT EXIST.** No `commands/auth_update.rs`; no `installs_res`/`unwrap_or_default` anywhere in `commands/`. Path was deleted/renamed in the minimal-core strip (or hallucinated by the original auditor). **Drop this finding** unless the install-enumeration logic is relocated and re-found.
- **B7** `bridge.rs:200,249,280` — `session_id.unwrap_or_default()` → `""` → IPC response misrouted/dropped. ✅ **[RV]**
- ~~**B9** `convo_store.rs:154,192`~~ — **❌ [RV] FILE DOES NOT EXIST.** No `state/convo_store.rs`. The only `let _ = remove_file()` sites are `state/paths.rs:69,90` — and those are **benign temp-file cleanup** in `atomic_write_json` (removing the `.tmp` on a failed write/rename), NOT session-metadata removal. The described symptom (stale session metadata loadable next session) does not apply. **Drop or rewrite.**
- **B10** `stt/mod.rs:240` — `let _ = h.join()` (in `Drop for ActiveSession`); panicked capture thread → mic handle leaked to process exit, no error. ✅ **[RV]**
- **B11** `diagnostics/mod.rs:185` — `let _ = rename(log → log.old)`; a failed rotation reopens the 5 MB log in append mode → unbounded growth. ✅ **[RV]**
- **A1** `turn.rs:887` — `.expect("stdin checked is_some above")`; safe in normal flow, but a child-exit race between the `is_none()` guard (`:880`) and `take()` (`:887`) could panic. Prefer `ok_or` + `?`. ✅ **[RV]** (race is real but theoretical — no other thread touches `child.stdin` at that point)
- **A4** `diagnostics/mod.rs:119` — `unwrap_or(Value::String(...))`; malformed scrubbed-fields blob silently degrades to a raw string (fidelity loss, not a crash). ✅ **[RV]**

#### RR-14. Update-chain minor 🔍
- `commands/update.rs:36`–`39` — 30s check timeout leaves the blocking reqwest thread running uncancelled; leaks a thread per timed-out check.
- `update_service.rs:1` — module comment says "v0.4.47+" but crate is pinned `=1.2.0`; stale/misleading for the next person verifying the vpk-version-sync rule.
- *(Downgraded — NOT a risk:* the hardcoded `"rift-tauri.exe"` fallback at `update_service.rs:244` is correct, since Velopack installs the prod binary as exactly that name; it only fires if `current_exe()` fails, where the fallback is right anyway.)*

---

## Recommended order

1. **RR-1** (new-user auth button) — small frontend change, highest leverage; without it, fresh users who skip onboarding hit a wall. All 4 sub-claims ✅ re-confirmed.
2. **RR-2** (crash file) — small backend change; without it you can't debug field reports.
3. ~~**RR-3**~~ — **DROPPED, was wrong** (npmjs URL is load-bearing, see RV correction).
4. **RR-5** (#29 prod verify) — must run a real prod build anyway before shipping.
5. **RR-8** (permission bar live-verify) — safety path, do once. Code path ✅ verified sound; only the runtime round-trip is unwatched.
6. Then T2 remainder (RR-4/6/7/9 — all ✅ re-confirmed), decisions (RR-10/11), T4 backlog as time permits.

---

## Verification log

Confirmed firsthand this audit (not just agent-reported):
- ✅ `resolve_under_roots` canonicalize + `starts_with` gate, fail-closed + capped read — `mcp_server.rs:70`–`182` read directly.
- ✅ Update reap-before-exit ordering — `update_service.rs:205`–`273` read directly.
- ✅ Version lockstep at `0.9.2` across all three files — grepped directly.
- ❌ **CORRECTED [RV]:** `registry.npmjs.org` is NOT dead — `src/lib/state/cliUpdate.svelte.ts:30` fetches it at runtime (CLI-update check). The original grep stopped at `package-lock.json` and missed the live caller. CSP entry is load-bearing.
- ✅ `ALLOW_PRERELEASE = true` — `update_service.rs:35`,`304` read directly.
- ✅ New-user auth dead-end — `AssistantWelcome.svelte:163`–`176` + `Composer.svelte:329`,`817` read directly.

Everything marked 🔍 above is agent-reported with a `file:line` anchor; re-grep before acting on it (line numbers may drift on a moving `main`).

---

## Open questions for the user

1. Fix the two T1 blockers (RR-1 + RR-2) now, in this session?
2. Is `ALLOW_PRERELEASE = true` (RR-10) intended for the wider cohort, or should the audience get final tags only?
3. Revisit code-signing (RR-11) for a non-buddy audience, or accept SmartScreen for now?
