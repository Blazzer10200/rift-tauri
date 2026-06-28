# HANDOFF — Mega-Audit cont.228 (branch `mega-audit-cont228`)

> Self-directed ULTRACODE full-codebase audit (user: "get it all done correctly, all angles, FE→BE, CLI-compat, perf"). NOT shipped — branch left for review. Resume from here.

## STATUS: 52 fixes applied + green + committed. 2 sweeps + master report in flight.

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
