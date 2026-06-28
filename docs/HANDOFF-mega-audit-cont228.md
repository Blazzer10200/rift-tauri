# HANDOFF — Mega-Audit cont.228 (branch `mega-audit-cont228`)

> Self-directed ULTRACODE full-codebase audit (user: "get it all done correctly, all angles, FE→BE, CLI-compat, perf"). NOT shipped — branch left for review. Resume from here.

## Session — 2026-06-28 (backlog applied + live-verified; organization task QUEUED)

### Completed (commits cd23562 → on branch mega-audit-cont228, all gate-green: cargo test 121/121 · svelte-check 0/0 (4134) · vitest 78/78)
- **All 3 T1s** (`cd23562`): R2-secret CI preflight in release.ps1 (silent-stale-feed guard) · release.yml verify repo `rift-releases`→`rift` · extracted `effort_tier_to_flag`+`send_effort_flag` to config.rs from the untested turn.rs inline match (+2 lockstep tests, mirror helpers.ts).
- **T2 correctness** (`4bad457`): `assistant_stop` cancels pending PermissionRegistry (now session-tagged + `cancel_all_for_session`) AND `TurnOutcome::Stalled` tree-kills the wedged child by PID. Fixes the real prod incident (session 87a27f20, child wedged 9+ min). +1 test.
- **T2 build** (`35e446d`): dropped aws-lc-rs (reqwest `rustls`→`rustls-no-provider`) — `cargo tree -i aws-lc-rs` now empty. CAUGHT a latent runtime panic the audit fix alone would've shipped: added `rustls::crypto::ring::default_provider().install_default()` in run(). Runtime-verified real HTTPS 200 OK.
- **5 of 7 dup cleanups** (`59f5023`+`2363d57`): tree_kill, ctxWindowFor, dirs_home(→dirs_home_or_temp), read_body_capped(cap param), strip_unc. All cross-confirmed by 2+ sweeps.
- **LIVE CDP VERIFICATION** (dev app launched, driven via c.sh): app boots (crypto swap safe) · turns stream + reply exactly · **Stop halts mid-stream + recovers clean** · AI Health/Workspace/Settings/model-picker all render · ctx ring shows "75k/1.0M (8%)" (ctxWindowFor dedup live) · **0 console errors across full nav sweep**.

### Key Decisions
- "Does thinking help?" → YES for hard reasoning, NO for everyday turns (adds multi-sec pre-pass, ttft p90 15.8s). v0.67.0 off-by-default toggle is correct. BLIND SPOT: telemetry logs FE effort TIER not actual sent effort/thinking_on — can't measure on-vs-off in prod (candidate fix still open).
- Stopped applying at ~85% done: all high-risk work retired + verified. Remainder is optional low-risk tail.

### In Progress / Next Steps (QUEUED — do on FRESH context)
1. **PROJECT ORGANIZATION TASK (user-requested, NOT yet started):** clean up + organize the whole `rift-tauri` folder; user expects markdown files deleted/merged/organized. ⚠️ Do as READ-ONLY AUDIT FIRST → present a concrete delete/merge/keep plan → get approval → execute. User's CLAUDE.md: "NEVER delete w/o explicit instruction; redundancy=FLAG not delete." Deferred from this session b/c context hit 30% (hard fence) mid-request — too risky to do read+delete-heavy work on degraded ctx.
2. Remaining backlog (all OPTIONAL, none blocking): ~14 static report-only (T3/T4 perf/race) · 42 dynamic T3/T4 · ~21 arch (tool-name dispatch ×3 dedup, dead `stashTabUi`/`dunce`, 2 medium refactors `run_or_prewarm`+ToolChip split).
3. Unexercised verify paths: permission Allow/Deny (tested Bypass only — Stop half proven, permission-cancel half not), git tools, Local LLM page.
4. Ship decision: branch is 10 commits ahead of main, NOT merged. User's call.

### Don't Touch / Load-Bearing
- DON'T re-run any audit sweep (all 4 complete; recover from docs/audit/cont228/*.json).
- DON'T kill rift-tauri.exe by name (dev binary + user's REAL installed app share it — PID-only). Dev app may still be RUNNING from this session (CDP :9222, cdp:serve :9223).
- effort/thinking lockstep = 3 sites (config.rs effort_tier_to_flag ↔ helpers.ts effortToFlag ↔ modelMatrix.ts) — change together + tests.
- DON'T merge/ship the branch unattended.

### Failed / Don't Retry
- `cmd //c scripts\run-dev.bat` from Bash → detaches+dies silently (the `pause` at EOF). Launch dev via PowerShell `Start-Process cmd.exe -ArgumentList '/k', "<bat>"` instead.

---

## STATUS: ✅ AUDIT COMPLETE. 52 fixes applied+green+committed. All 4 sweeps done + master report shipped to docs/audit/cont228/ (commit 821bce8). Branch NOT merged — review the report, then decide what to apply.

### RESUME-ITEMS — all done (cont.228 final pass):
- (1) Dynamic sweep — was ALREADY complete (`w73q9ry42` finished after the handoff was written). Recovered, NOT re-run. 56 confirmed.
- (2) Architecture sweep — prior attempt `w5bfm2b6w` genuinely died (all 7 rate-limited, 0 findings). RE-RAN solo as `w7692fdzs` → clean, 45 findings → 27 endorsed, all subsystems `minor-drift`.
- (3) Master report — written: `docs/audit/cont228/MASTER-REPORT.md` + ISSUES.md #69 block + JSON artifacts (report-data/dynamic-result/architecture-result). Committed 821bce8.

### APPLYING THE BACKLOG (cont.228 continued — all gate-green, committed on branch):
- ✅ `cd23562` T1: R2 release preflight (release.ps1) + release.yml rift-releases→rift + effort_tier_to_flag/send_effort_flag extracted+tested (TC-001/002).
- ✅ `4bad457` T2: assistant_stop cancels PermissionRegistry (now session-tagged, +cancel_all_for_session) + Stalled tree-kills wedged child by PID. The real prod wedge (session 87a27f20).
- ✅ `35e446d` T2 build: dropped aws-lc-rs (reqwest→rustls-no-provider) + ring install_default in run(). Runtime-verified TLS 200 OK. Baseline: cargo test 121/121.

- ✅ `59f5023` cleanup: tree_kill (oneshot→warm_pool::kill_child_tree, 3 sites) + ctxWindowFor (→helpers::ctxWindowForModelId, store+pump delegate).
- ✅ `2363d57` cleanup: dirs_home (paths::dirs_home_or_temp) + read_body_capped (assistant::read_body_capped(resp,cap), local_llm 256K/news 8M) + strip_unc (assistant::strip_unc, mcp_server+git_local delegate). Gate: cargo test 121/121 · svelte-check 0/0 (4134) · vitest 78/78.

### 5 of 7 dups done. REMAINING:
- **tool-name dispatch ×3** (ToolChip 169-174 + streamModel.ts 87-90 + canonical toolCaption.ts) → consolidate to `summaryForTool`/`captionForTool` in toolCaption.ts. Riskier (hot render files, verb-led vs noun-led divergence) — needs CDP verify of tool chips after. NOT done.
- **trust-gating ×2** (tools/list + tools/call) → auditor said LEAVE until a 3rd trust level appears. Skip.
- **dead code:** stashTabUi/restoreTabUi (no-ops 8+ sites) + vestigial dunce shim (mcp_server.rs) — not yet removed.

### THEN (medium, own session): run_or_prewarm ~740L → SpawnParams + resolve_spawn_params extraction (biggest readability win). Full list: docs/audit/cont228/MASTER-REPORT.md.


### Baseline (held green throughout): cargo check clean · svelte-check 0/0 (4134) · vitest 378/378

## DONE
- **Discovery:** static sweep (30 finders, all .rs+.svelte+.ts) → 95 findings. Rate-limit corrupted the
  workflow's auto-aggregation; RECOVERED all 95 by parsing agent transcripts → `scratchpad/audit/findings/*.json`.
- **Reverify:** solo workflow, 3-skeptic adversarial panel → **71 confirmed / 21 killed FPs**, 52 autofix-safe.
  Adjudicated set: `scratchpad/audit/adjudicated-final.json`. Report data: `scratchpad/audit/report-data.json`.
- **52 fixes APPLIED + COMMITTED (all gate-green):**
  - `e5c4b35` — 43 fixes / 28 files (FE+BE). 3 self-inflicted regressions caught by gating + fixed
    (restored load-bearing `{#if true}` `{@const}` wrappers in ToolChip + StreamAskUser; LivePills prop trim).
  - `fc2d852` — T1 + 5 turn.rs fixes (agent had timed out; done by hand + verified):
    - **T1 session-lost-prompt-missing** — `onSessionLost` (assistant.svelte.ts) now recovers the retry prompt
      from the last user message's text blocks (backend emits only `{session_id}`) → retry re-sends the real
      turn, not `undefined`. Guarded on non-empty.
    - stop-on-already-dead (assistant_stop taskkill/kill non-zero exit → Ok); reused-control-response leak
      (gate on handshake_done alone); option-wrapper (with_session_pids/stopped → R not Option<R>);
      validate-attachments (MIME check before base64 byte scan).

## IN FLIGHT (resume these)
1. **Dynamic sweep** `w73q9ry42` (`scripts`→ `scratchpad/audit/phase1b-dynamic.js`): telemetry/log forensics,
   live `claude --help` diff, changelog verify of `// est` floors, deps/build/config, test-coverage. Running solo.
2. **Architecture sweep** (`scratchpad/audit/phase1c-architecture.js`): 7 subsystem structural reviewers.
   1st attempt `w5bfm2b6w` ALL FAILED on rate-limit — RE-RUN SOLO after dynamic finishes.
3. **MASTER REPORT** — consolidate applied(52) + report-only(19) + dynamic + architecture into:
   `scratchpad/audit/MASTER-REPORT.md` + a new cont.228 block in `docs/ISSUES.md`.

## 19 REPORT-ONLY (confirmed real, need review/bigger lift — full detail in report-data.json):
Notable: **correctness-stop-permission-registry-not-cancelled** (T2 — assistant_stop leaves pending
PermissionRegistry entries, UI stuck ≤120s); **cli-compat-shim-dedup-version-none** (T2); duplicate
tree-kill helpers (oneshot vs warm_pool — consolidate); perf-warmhit-mcp-write (T3 needs-care — MCP config
written+deleted every warm hit); markdown/stream hot-derived perf (needs-care); local_llm ephemeral clients
skip corp certs (T3).

## 21 KILLED FPs (DON'T re-file): the 5 cli-compat `// est` floors (eclipsed by MIN_SUPPORTED=2.1.161, zero
runtime impact), onSessionLost-messages-strip, resolve-manager-under-lock, compaction-fallback-ordering,
several perf-reactive (thinking-delta, index-keyed-each, askquestions-remap, termlines-reclassify, rawlinelog).

## PERF GROUND TRUTH (`scratchpad/audit/PERF-BASELINE.md`, from prod turns.ndjson n=183):
ttft_text median 5.2s / p90 15.8s. Rift's OWN overhead small (ttft_first_line 828ms) — latency is CLI/API.
Cold 64% BUT 86/107 are legit new-session first-turns (only 18 premature evicts) → warm pool mostly fine.
thinking-off→`--effort low` WORKS (turn.rs:1180). Telemetry logs FE effort TIER not the actual sent effort
→ blind spot (candidate fix: log thinking_on + real send_effort in the turn record).

## KEY LESSON: the ceiling is the **server request-rate limit** (NOT user usage). Run ONE heavy workflow at a
time (~14 internal concurrency self-cap is fine). Stacking 3 = throttle storm that killed architecture +
starved 138 static verifiers. Recover throttled workflow data by parsing agent transcripts, don't blindly re-run.

## DO NOT: ship/merge this branch unattended · re-run reverify (done) · touch lockstep invariants.
