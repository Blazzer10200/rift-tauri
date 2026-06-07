# Session kickoff prompts — idea-phase arc

> Copy-paste the prompt for the session you're starting. Each is self-contained: a fresh session
> inherits zero chat history, so the prompt names every doc to read and every guardrail up front.
> Plan of record: [idea-phase-plan.md](idea-phase-plan.md). Mark a session done here when you finish it.

## STATUS — at a glance (verified against the tree 2026-06-07)

> Single source of truth for arc progress. Update this block the moment a phase lands — the per-session
> prompts below are HTML-commented (already used), so this table is what you read first.

| Session | Phase | State | Evidence |
|---|---|---|---|
| A | 0 — ship insurance | ☑ | v0.6.5 released (`ca5e083`) |
| B | 1a–1b — SQLite store + pricing | ☑ | `usage/{store,pricing}.rs`, `model-prices.json`, `rusqlite` bundled (`1205f12`) |
| C | 1c–1e — aggregation + gauge + cockpit | ☑ | `usage/{aggregate,budget}.rs`, `CostPage.svelte`, `usage.svelte.ts` (`1205f12`) |
| D | 2a–2b — multi-provider + insights | ☑ | `ProviderProfile`/`active_provider_id` route turns (`mod.rs:3091`→`ANTHROPIC_BASE_URL` `:3390`); presets DeepSeek/GLM/Bedrock; `usage/insights.rs` + `usage_insights` (`1205f12`) |
| E | 3 — multi-agent + compression | ◐ | **3a done** — sandbox eval + worktree-harness prototype ([edit-swarm-safety-layer.md](edit-swarm-safety-layer.md), `scripts/proto/swarm-harness.ps1`). **3b, 3c remain.** |

> **Ship debt:** Phase 1+2 are committed but **not released** — still on v0.6.5. A version bump (THREE files +
> `Cargo.lock` + CHANGELOG → `release.ps1`) is owed before the next public build.

---

## Session A — Phase 0: ship the insurance  ☑ (v0.6.5)
**Paste this:**

<!-- > Read `docs/HANDOFF.md`, then `docs/design/idea-phase-plan.md` (§2 Phase 0). We're shipping the
> already-built work before June 15. Tasks:
> 1. Commit the 4 uncommitted escape-hatch files (`src-tauri/src/assistant/mod.rs`, `src-tauri/src/lib.rs`,
>    `src/lib/components/settings/SettingsPage.svelte`, `src/lib/state/assistant.svelte.ts`) together with
>    the cont.66 hardening commit `c1cc817`.
> 2. Live smoke-test the custom-provider escape hatch via CDP (start dev with `scripts/run-dev.bat`,
>    `npm run cdp:serve`, drive with `scripts/cdp/c.sh`) — render + save + on-disk persist + a routed turn.
> 3. Ship: bump THREE version files (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`)
>    **+ `Cargo.lock`**, update `docs/CHANGELOG.md`, then `/git-ship`.
>
> Guardrails: quit `rift-tauri.exe` (dev binary, EXACT name — never a `rift*` glob, the prod app is
> `rift.exe`) before any build. Don't run `cargo check` while dev is alive. Version lockstep is the #1
> ship failure — all three files + Cargo.lock or `release.ps1` bails.
>
> Done-when: a tagged release on `Blazzer10200/rift-releases`, escape hatch confirmed in a prod build. -->

---

## Session B — Phase 1a–1b: SQLite store + price table  ☑
**Paste this:**

<!-- > Read `docs/HANDOFF.md`, then `docs/design/idea-phase-plan.md` (§0–1 + §2 Phase 1a/1b). Build the
> durable usage store and the pricing layer — the foundation for the cost cockpit. Context: Rift already
> persists per-turn metrics to `~/.rift/assistant/session-logs/<id>.json` as `TurnRecord[]`, but that dir
> is ring-buffered/pruned (`src-tauri/src/assistant/session_log.rs:138-168`), so history is lossy.
>
> 1a — New module `src-tauri/src/usage/{mod,store}.rs`; add `rusqlite` (bundled) to `Cargo.toml`; register
>   in `lib.rs`. Table `turns(session_id, turn_index, ts, model_id, provider, input, output, cache_read,
>   cache_write, cost_usd_cli, cost_usd_calc, ttfp_ms, duration_ms, workspace, tool_count)`, idempotent
>   upsert on `(session_id, turn_index)`. New command `usage_ingest_turn` called from the existing
>   `recordSessionLog` debounce (`src/lib/state/assistant.svelte.ts:1155-1171`). `usage_backfill_from_logs()`
>   one-shot reads existing `session-logs/*.json`.
> 1b — `src-tauri/assets/model-prices.json` ({model_id → input/output/cache_read/cache_write USD per Mtok}),
>   override from `~/.rift/model-prices.json`; `src-tauri/src/usage/pricing.rs` fills `cost_usd_calc` at
>   ingest. Flag rows whose model isn't in the table (custom-provider "estimated" case).
>
> Guardrails: backend-only session — quit dev before `cargo check`. Verify each chunk: paste verbatim
> `cargo check` exit (zero `error[E…]`). Confirm a DeepSeek-style custom turn computes correct cost from a
> user price entry even though the CLI cost is wrong.
>
> Done-when: every completed turn lands a SQLite row that survives an app restart; backfill populates history. -->

---

## Session C — Phase 1c–1e: aggregation → fuel gauge → cockpit UI  ☑
**Paste this:**

<!-- > Read `docs/HANDOFF.md`, then `docs/design/idea-phase-plan.md` (§2 Phase 1c/1d/1e). Build the cross-session
> rollups, the credit-pool fuel gauge, and the UI. Foundation (store + pricing) is done from Session B.
>
> 1c — `src-tauri/src/usage/aggregate.rs` + commands: `usage_daily`, `usage_monthly`, `usage_by_model`,
>   `usage_by_workspace`, `usage_blocks(window=5h)` (Claude billing windows), `usage_session`. SQLite GROUP BY.
> 1d — Fuel gauge: user picks plan ($20 Pro / $100 Max-5x / $200 Max-20x) + reset cadence (store in a
>   `budget` block). `usage_budget_status()` → {spent, limit, pct_remaining, window_start,
>   projected_exhaustion_date} from current-window spend + burn rate. **This is the post-June-15 killer feature.**
> 1e — UI: new `src/lib/components/workspaces/CostPage.svelte` as a **Harness sub-tab** (do NOT add a 5th
>   workspace — preserve the 4-workspace IA invariant in HANDOFF). New store `src/lib/state/usage.svelte.ts`.
>   Reuse the bento/KPI visual language from `HarnessPage.svelte`. Honor "Harness fits ONE viewport, no scroll."
>
> Guardrails: frontend edits → `npm run check` (0/0). UI work → verify with CDP `shot` after HMR, not DOM-only
> (a cost dashboard is pixels, not structure). Don't cram cross-session data into the existing live KPI rail.
>
> Done-when: live app (CDP shot) shows the cockpit reading real persisted history across an app restart, with
> a fuel gauge whose dry-out projection moves as turns burn credit. -->

---

## Session D — Phase 2: escape hatch v2 + grows-with-you v1  ☑ (`1205f12`)
**Paste this:**

<!-- > Read `docs/HANDOFF.md`, then `docs/design/idea-phase-plan.md` (§2 Phase 2). Two builds:
> 2a — Multi-provider escape hatch (the `cc-switch` pattern). Extend `AssistantConfig`
>   (`src-tauri/src/assistant/mod.rs:636-701`) from a single `base_url` to `providers: Vec<ProviderProfile>
>   { id, name, base_url, model, key_ref }` + `active_provider_id`. New commands beside the existing four
>   (`mod.rs:1771-1797`). `assistant_send` routing (`mod.rs:2863-2882`) reads the *active* profile. Settings
>   card (`SettingsPage.svelte:639-668`) → saved list + presets (DeepSeek, GLM/Zhipu, Bedrock, generic
>   OpenAI-compat) + one-click active toggle. Keep atomic temp+rename writes.
> 2b — Insight layer v1: `src-tauri/src/usage/insights.rs` — deterministic queries over the SQLite corpus
>   (top tools, model mix per workspace, cost sinks, time-of-day burn, cache-efficiency trend). Surface
>   **observational only** ("Rift noticed…") in a Cost/Harness panel. Seed of Pillar 3; no auto-action yet.
>
> Open: memory substrate for later Pillar 3 depth — `supermemory` (TS) vs MemPalace (Py) vs roll-your-own on
> the SQLite we already have. Decide only if v1 insights prove the value.
>
> Done-when: switch providers from a saved list without retyping; insight panel surfaces ≥3 true non-trivial
> patterns from real history. -->

---

## Session E+ — Phase 3: multi-agent + compression (later)  ◐ (3a done; 3b/3c remain)

**3a — DONE 2026-06-07.** Evaluated both candidates: **neither is a native-Windows sandbox** —
`anthropic-experimental/sandbox-runtime` has no Windows support (Seatbelt/bubblewrap only); `NVIDIA/OpenShell`
is a Linux-only K3s/Docker control-plane (Landlock/seccomp), alpha, WSL2-only. **Decision:** the write-mode
safety layer = a cross-platform **worktree + verify-gate + adversarial-diff harness** (also the spine of 3b);
OS-level FS/net sandboxing is deferred to an **optional WSL2/Linux/macOS tier** (`sandbox-runtime` /
`claude --sandbox`). Prototype proven on Windows: `scripts/proto/swarm-harness.ps1` (worktree isolate →
node_modules junction → gate discriminates pass/fail → main tree untouched). Full writeup +
open questions: [edit-swarm-safety-layer.md](edit-swarm-safety-layer.md). **Next: 3b** (build the swarm on the
harness; feed it the audit-swarm's confirmed-findings list) or **3c** (headroom compression toggle).

**Paste this (for 3b/3c):**

> Read `docs/HANDOFF.md`, `docs/design/session-kickoffs.md` (STATUS block), `docs/design/idea-phase-plan.md`
> (§2 Phase 3), `docs/design/edit-swarm-safety-layer.md` (the 3a decision), and the 3b seed in `docs/IDEAS.md`.
> This session FINISHES the idea-phase arc: ship the owed release, build Phase 3b, then optional 3c.
>
> Verified starting state: Phases 0/1/2 are committed (`1205f12`) but UNSHIPPED — repo is on v0.6.5. Phase 3a
> is done — decision: a cross-platform **worktree + verify-gate + adversarial-diff harness** (prototype at
> `scripts/proto/swarm-harness.ps1`); OS-level FS/net sandboxing is deferred to an optional WSL2/Linux/macOS
> tier (`sandbox-runtime` / `claude --sandbox`). DON'T redo 3a.
>
> STEP 1 — Ship the owed release (clears the debt; do first, it's low-risk).
> - Live smoke-test via CDP (start dev with `scripts/run-dev.bat`, `npm run cdp:serve`, drive `scripts/cdp/c.sh`):
>   Harness→Cost cockpit renders real history across a restart; provider switch routes a turn; insights show ≥3.
> - Update `docs/CHANGELOG.md` (≤600 words) with the Phase 1+2 summary. Bump ALL THREE version files
>   (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`) + commit the `Cargo.lock` update; `/git-ship`.
> - Guardrails: quit `rift-tauri.exe` (dev binary, EXACT name — NEVER a `rift*` glob; prod app is `rift.exe`)
>   before build. Don't `cargo check` while dev is alive. Version lockstep is the #1 ship failure: all three +
>   Cargo.lock or `release.ps1` bails. Clean tree or `-Force`; Setup.exe-only; vpk CLI ver == velopack crate ver;
>   no stderr redirect in PS5.1 (NativeCommandError trap).
> - Done-when: tagged release on `Blazzer10200/rift-releases`; cockpit + provider switch confirmed in the prod build.
>
> STEP 2 — Phase 3b: productionize the edit-applying swarm on the 3a harness.
> - Decide the open questions from `edit-swarm-safety-layer.md §7` (delegated): (a) cargo gate during dev
>   (shared `CARGO_TARGET_DIR`+quit-dev, or skip Rust gate when dev alive); (b) merge-on-accept (`git cherry-pick`
>   worktree commit vs apply-patch); (c) module home (new `src-tauri/src/swarm/` vs fold into `assistant/`);
>   (d) adversarial reviewer (reuse audit reviewer vs dedicated diff-vs-finding prompt).
> - Build a Rust orchestrator over a confirmed-findings list `{file,line,evidence,suggested_fix}`: group by file
>   (one-file-one-agent, serialized within file); per agent → `git worktree add --detach` → junction
>   `node_modules`/shared cargo target → claude-CLI child applies hash-anchored (`oh-my-pi`-style) edits →
>   verify gate (`cargo check`/`npm run check`) → fail = auto-revert+flag → pass = adversarial diff-vs-finding
>   review → accept = merge back, reject = discard → SAFE cleanup (rmdir junction THEN worktree remove; never
>   recursive-delete through the junction — design §4.2). Findings source: chain from the audit swarm if it
>   exists, else a hand-supplied array for v1.
> - Minimal UI: a swarm-run panel (per-agent progress, gate verdict, accept/reject) reusing Harness bento
>   language; honor the 4-workspace IA + "Harness fits ONE viewport" invariants (HANDOFF CRITICAL DON'T-TOUCH).
> - Verify: backend `cargo check` 0 err (quit dev first); frontend `npm run check` 0/0; run the swarm against a
>   real 2-3 finding list end-to-end; confirm a build-breaking fix auto-reverts and a good fix merges; main tree intact.
> - Done-when: a findings list drives parallel worktree-isolated edits that each pass gate + diff review before
>   merging, main tree never corrupted, demonstrated live.
>
> STEP 3 — Phase 3c (OPTIONAL — skip if Steps 1-2 fill the session): compression toggle.
> - `headroom`-style local proxy in front of the CLI via the existing `ANTHROPIC_BASE_URL` seam (the same seam
>   the provider router uses, `mod.rs:3390`). Opt-in setting only — the Python runtime is a soft dep, must NOT
>   become mandatory. Off by default.
> - Done-when: a toggle routes turns through the local compressor when on, bypasses it when off; off by default.
>
> Close: update CHANGELOG + `/handoff`; in this STATUS block tick D as shipped and E → ☑ once 3b lands. If you
> ship again after 3b, repeat the Step-1 release guardrails. Scope honestly — a clean stop after Step 1 or Step 2
> is fine; 3c can be its own session.

---

## Maintenance — keep this alive
- Tick the ☐ for each session as it lands.
- If the plan shifts mid-arc, update `idea-phase-plan.md` first, then the affected prompt here.
- Line numbers drift after any commit — re-anchor by snippet, not by the numbers quoted above.
