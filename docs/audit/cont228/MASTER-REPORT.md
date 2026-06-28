# Rift Mega-Audit cont.228 — Master Report

> Self-directed ULTRACODE full-codebase audit (user: "get it all done correctly, all angles, FE→BE, CLI-compat, perf — it feels like a mess and slow"). Four parallel sweeps: **static** (line-level bugs/dead-code/perf), **dynamic** (telemetry forensics + live CLI-compat + deps/build + test-coverage), **architecture** (structural coherence), and a **perf ground-truth** baseline from prod telemetry. Branch `mega-audit-cont228`, NOT shipped.
>
> Baseline held green throughout: `cargo check` clean · `svelte-check` 0/0 (4134) · vitest 378/378.
>
> **Data artifacts (this dir, `docs/audit/cont228/`):** `report-data.json` (static 52+19+21) · `dynamic-result.json` (56 confirmed) · `architecture-result.json` (27 endorsed). Perf baseline = `PERF-BASELINE.md` (session scratchpad; key numbers inlined in §D).

## TL;DR — the verdict on "feels like a mess"

The codebase is **not rotten — it's accreted**. All 7 architecture reviewers returned **`minor-drift`**; *zero* returned `needs-restructure`. The "mess" feeling is real but localized: a handful of god-functions/god-objects and ~10 copy-paste duplications across the 200+ continuation sessions. None is a redesign; **25 of 27 endorsed restructures are `small` effort**, 2 are `medium`. The "slow" feeling is **almost entirely CLI/API latency, not Rift** (ttft_first_line median 828ms — Rift's own overhead is small).

## Totals across all four sweeps

| Sweep | Raised | Confirmed | Applied | Report-only / Endorsed | Killed FPs |
|---|---|---|---|---|---|
| Static (discovery) | 95 | 71 | **52** | 19 (→18 unique) | 21 |
| Dynamic (runtime/external) | 61 | **56** | 0 | 56 catalogued | 5 |
| Architecture (structural) | 45 | 45 | 0 | **27 endorsed** | 18 (not-worth-it) |
| Perf baseline | — | ground truth (n=183 prod turns) | — | 1 blind-spot fix | — |

**52 fixes already applied + committed + green** (commits `e5c4b35` 43-fix batch, `fc2d852` T1 + 5 turn.rs). Everything below is the *remaining catalogued work* — none auto-applied this pass.

---

## A. STATIC SWEEP — 18 report-only (confirmed real, not yet applied)

Full detail: `report-data.json`. T2 first.

**T2 (2):**
1. `correctness-stop-permission-registry-not-cancelled` (turn.rs:2596) — **T2 the big one.** `assistant_stop` doesn't cancel pending `PermissionRegistry` entries → UI stuck up to 120s after Stop. *(Cross-confirmed by dynamic CLI-COMPAT-05 / F5 — a real prod incident 2026-06-28: child wedged 9+ min after a permission card timed out.)*
2. `cli-compat-shim-dedup-version-none` (cli_install.rs:310-328) — `.cmd` shim always returns `version: None` (CreateProcess can't run batch files) → dedup guard never fires → spurious install-list entry.

**T3 (8):** destroy/init HMR listener race (stt.svelte.ts:198) · initInner guard race (assistant.svelte.ts:922) · local_llm ephemeral reqwest clients skip corp certs (local_llm.rs:92) *(also arch + dynamic)* · ctxWindowForModel dup in streaming.ts:83 *(also arch)* · double-autosize per keystroke (Composer:163) · drain/collect resample alloc in audio callback (audio.rs:215) · `grouped` full pipeline per token (MessageBubble:168) *(also arch)* · MCP config written+deleted every warm hit (turn.rs:787).

**T4 (8):** CLI_RECOMMENDED_VERSION lockstep drift (cliUpdate:53) · nothink pooled client skips corp certs (nothink.rs:39) · **duplicate tree_kill** oneshot vs warm_pool (oneshot.rs:80) *(also arch T3 — collapse)* · fresh VAD per tick (vad.rs:26, =#46 R10) · markdown wrapWords ancestor walk (Markdown:214) · query_turn_perf materializes full NDJSON (perf.rs:481) · markdown triple template parse (Markdown:485) · full serde parse per NDJSON line (turn.rs:2185).

*(Note: static #14/#15 were the same tree_kill finding double-filed — collapsed to 18 unique.)*

---

## B. DYNAMIC SWEEP — 56 confirmed (NEW this pass)

By angle: perf-runtime 14 · cli-compat-live 14 · deps-build-config 19 · test-coverage 9. Full detail: `dynamic-result.json`. **The high-severity new findings:**

**T1 (3) — highest priority:**
- `deps-build/F2` **R2 dual-publish is skippable but `update_service.rs` reads R2 ONLY** → missing CI secrets = silent-green build shipping a **stale update feed**. Fix: preflight in `release.ps1` that fails if `$Ci` and any R2 env var absent. *(safe)*
- `test-coverage/TC-001` **turn.rs effort→flag inline match (turn.rs:1154) is untested** → wrong `--effort` silently sent. Fix: extract `effort_tier_to_flag` to config.rs + `#[test]` all 5 tiers. *(safe; directly guards the PERF lockstep)*
- `test-coverage/TC-002` **`thinking_on=false → --effort low` override (turn.rs:1176) has no test** → silent regression reinstates 12s+ TTFT. *(this is the v0.67.0 "fast hello" win — currently unguarded)*

**T2 (11):** cli_api_ms is cumulative not wall-clock → Health-pane overhead p50 underestimated ~38% (perf.rs:349 `saturating_sub` clamps the 13/42 impossible turns to 0) · Haiku ran full default thinking despite effort=low (turn.rs:1104/1177 — now dead code, HAIKU_DISABLED) · opus 10–53s post-thinking silent gap before first text · **effort baked into SpawnKey → cold respawn on every effort-tier change** (warm_pool:107) · permission-timeout child-wedge (×2, real incident) · **aws-lc-rs IS compiled despite ring-override comment** (Cargo.toml:94 → use `rustls-no-provider`) · release.yml verify queries `rift-releases` not `rift` (relies on 301 redirect) · 4× untested security-relevant pure fns (update apply, validate_attachments, build_user_envelope, thinking-default lockstep).

**T3 (24) / T4 (18):** cli-compat `// est` floors (eclipsed, low-impact), perf micro-allocs, deps hygiene, more test-coverage gaps — catalogued in `dynamic-result.json`.

---

## C. ARCHITECTURE SWEEP — 27 endorsed restructures (NEW this pass)

All 7 subsystems = **`minor-drift`** (none `needs-restructure`). 45 findings → 27 survived the pragmatic "is the churn worth it" crosscheck. **25 small, 2 medium.** Full detail: `architecture-result.json`.

**The 2 medium (the real "god" complaints):**
- `arch-turn/T1` **`run_or_prewarm` is a ~740-line monolith** behind two thin command shims (assistant_send/assistant_prewarm). Single seam exists: *resolve everything → spawn*. Fix: extract `SpawnParams` + `resolve_spawn_params()`, reducing the fn to ~80 lines. No public-API change. Crosscheck: real 3-5h refactor (owned `Command` is `!Clone`, `_mcp_guard` lifetime) — accurate medium. Highest-value structural win (hottest backend file).
- `arch-fe-components/T2` **ToolChip (1444L) carries the full AskUser interactive form + card delegation** in one file → split the form out.

**Duplication cleanups (7 — several collapse with static/dynamic, high-confidence):**
- `tree_kill` ×2 (oneshot vs warm_pool) — *also static #14/#15.* Delete oneshot copy, call `warm_pool::kill_child_tree`.
- `ctxWindowFor` verbatim in store + streaming.ts — *also static.* Extract to helpers.ts.
- `dirs_home()` ×3 divergent — one `state::paths::dirs_home_or_temp()`.
- tool-name dispatch ×3 across render pipeline — consolidate to `toolCaption.ts`.
- `read_body_capped` ×2 (local_llm + news) — shared `http_util.rs`.
- `strip_unc`/`strip_verbatim` ×2 (mcp_server + git_local) — one helper in mod.rs.
- trust gating ×2 (tools/list + tools/call) — leave as-is until a 3rd trust level appears.

**Misplaced-responsibility / over-complexity / dead (18 small):** AssistantStore still 1784L god-object despite 9 extractions · `shortToolLabel` 40-branch table inside store · `SLASH_COMMANDS` static inside Composer · `stashTabUi`/`restoreTabUi` no-ops at 8+ sites (dead) · 14KB hardcoded system prompts in turn.rs · mod.rs owns non-glue logic (MCP write, DACL, seq) · PathFilter recompiled from env per tool-call · vestigial `dunce` shim · STT reaches into `assistant::` internals (hidden coupling) · `load_native_certs()` called twice · update `apply()` child-reap inline with no test seam · 4 spare-detection booleans in run_turn_loop · command surface split across 4 naming tiers.

---

## D. PERF GROUND TRUTH (prod telemetry, n=183)

`PERF-BASELINE.md`. ttft_text median **5.2s** / p90 15.8s. **Rift's own overhead is small** (ttft_first_line 828ms) — latency is CLI/API. Cold 64% but 86/107 are legit new-session first-turns (only 18 premature evicts) → warm pool mostly fine. thinking-off→`--effort low` **works** (turn.rs:1180). **Blind spot:** telemetry logs FE effort *tier*, not the actual sent effort → candidate fix: log `thinking_on` + real `send_effort` in the turn record. *(Dynamic TC-001/TC-002 now also flag this path as untested.)*

---

## Recommended next actions (priority order)

1. **T1 release-safety:** `release.ps1` R2-secret preflight (dynamic F2) — prevents silent stale-feed ships. ~30 min.
2. **T1 test-coverage:** extract+test `effort_tier_to_flag` and the `thinking_on→low` override (TC-001/TC-002) — guards the v0.67.0 fast-hello win against silent regression. ~1h.
3. **T2 correctness:** wire `assistant_stop` to cancel PermissionRegistry + force-kill wedged child on watchdog (static stop-permission + dynamic F5/CLI-COMPAT-05 — one real prod incident). ~1-2h.
4. **Cleanup batch (all `small`, cross-confirmed):** the 7 duplication collapses (tree_kill, ctxWindowFor, dirs_home, tool-dispatch, read_body_capped, strip_unc) + dead `stashTabUi`/`dunce`. One focused PR, gate-green. ~half day.
5. **T2 build hygiene:** `rustls-no-provider` to drop aws-lc-rs; `release.yml` repo name `rift-releases`→`rift`. ~15 min.
6. **Medium (own session):** `run_or_prewarm` → `SpawnParams` extraction. The single biggest readability win for the "feels like a mess" complaint. ~3-5h.

## DO NOT
Ship/merge this branch unattended · re-run any sweep (all 4 complete — recover from JSON artifacts) · touch the effort/thinking lockstep without updating all 3 sites + tests · re-file the 21+5+18 killed FPs.
