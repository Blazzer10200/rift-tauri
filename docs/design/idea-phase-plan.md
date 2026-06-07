# Rift — Idea-Phase Master Plan (the build sheet)

> The executable plan for the four-pillar "cockpit for Claude Code" arc. Written 2026-06-07.
> Companion to [rift-roadmap.md](rift-roadmap.md) (the *why*) and [../IDEAS.md](../IDEAS.md) (the *backlog*).
> This doc is the *how* — file-accurate, pattern-grounded, sequenced. Read before implementing.
>
> Accuracy basis: every file:line below was verified against the working tree on 2026-06-07 by two
> codebase-mapping passes + three reference-repo deep-dives. Re-confirm line numbers by snippet anchor
> before editing — any prior commit shifts them.

---

## 0. The one insight that shapes everything

**Rift already collects and persists the prize.** Per-turn metrics are written to disk today:

| Store | Path | Contents |
|---|---|---|
| Session telemetry log | `~/.rift/assistant/session-logs/<session-id>.json` | full `TurnRecord[]`: `costUsd`, `envelopeUsage`/`resultUsage` (input/output/cache_read/cache_create), `modelId`, `firstPaintAt` (ttfp), `doneAt`, `thinkingTotalMs`, `toolUses[]` w/ per-tool timing |
| Conversation JSON | `~/.rift/assistant/conversations/<uuid>.json` | per-turn `costUsd` on each `ChatMessage` (tokens NOT stored here) |

Consequences for the plan:
- **Pillar 2 (cost cockpit) = aggregation + pricing + budgeting**, NOT instrumentation. The data exists.
- **Pillar 3 (grows-with-you) = a read layer over the same `TurnRecord[]`**, NOT new capture.
- **Two real holes:** (a) session logs are **ring-buffered** (`session_log.rs:138-168` prunes to N newest on startup) → no durable long-term history; (b) there is **no cross-session aggregation** — `HarnessPage.svelte` renders one session (live or one loaded snapshot) at a time.
- **Cost trust hole:** Rift reads cost from the CLI's `total_cost_usd` (`assistant.svelte.ts:817-825`). For custom-provider turns (escape hatch) that number is wrong/absent. A **bundled price table** is required to compute cost independently — same approach ccusage takes with LiteLLM.

---

## 1. Cross-cutting architecture decisions (the "best patterns")

These are decided with rationale. The user delegated the calls; any is vetoable next session.

### D1 — Introduce SQLite as the durable metrics + memory store. **(biggest decision)**
- **Why:** the ring-buffer prunes session logs, so an all-time cost dashboard and any "grows-with-you" learning need a store that doesn't lose history. cc-switch (same Tauri 2 + Rust stack) uses SQLite at `~/.cc-switch/cc-switch.db` for exactly this. One store serves Pillars 2 **and** 3.
- **Shape:** `~/.rift/rift.db` via `rusqlite` (bundled, no system dep) — single Rust module `src-tauri/src/usage/store.rs`. An **ingestion pass** on the existing debounced session-log write (`assistant.svelte.ts:1155-1171` → new `usage_ingest_turn` command) appends each finalized `TurnRecord` into a `turns` table *before* the ring buffer can prune it. Session logs stay as the live/replay format; SQLite is the append-only historical truth.
- **Alternative considered:** append-only JSONL rollup (`~/.rift/usage/rollup.jsonl`). Simpler, no crate, but hand-rolled query/aggregation. Rejected — SQLite's `GROUP BY` does the daily/monthly/per-model folds for free and Pillar 3 will want joins.
- **Migration:** on first run, backfill from existing `session-logs/*.json` (they're not pruned yet for current users) so history isn't blank on day one.

### D2 — Bundle a static model price table, ship-updatable.
- **Why:** custom-provider cost is untrustworthy; even Anthropic cost should be recomputable. ccusage uses LiteLLM's price file.
- **Shape:** `src-tauri/assets/model-prices.json` → `{ model_id: { input, output, cache_read, cache_write } }` (USD per Mtok). Loaded at startup, overrideable by `~/.rift/model-prices.json` (user can add a custom-provider's pricing, e.g. DeepSeek rates). Cost = `Σ tokens × price`. When a turn has a CLI `total_cost_usd` AND a known model, prefer the table for consistency; fall back to CLI cost when the model is unknown.

### D3 — Aggregation lives in Rust, not the frontend.
- **Why:** file/DB access, perf over N sessions, and it keeps the heavy fold out of Svelte reactivity. Mirror `telemetry.ts::summarizeSession` (`telemetry.ts:59-265`) but at the corpus level. New module `src-tauri/src/usage/` exposes DTOs via tauri commands; the frontend just renders.

### D4 — Rift's own session logs are the source of truth (not `~/.claude`).
- ccusage reads Claude's own JSONL. Rift's logs are *richer* (per-tool timing, ttfp) and scoped to Rift-driven turns only — which is what the cockpit cares about. Skip reading `~/.claude` for v1; revisit only if users want to account for non-Rift CC usage.

### D5 — Respect the "Harness fits ONE viewport, no scroll" invariant.
- The cost cockpit is a **new view/route**, not crammed into the existing KPI rail. The current rail (`HarnessPage.svelte:518-525`) stays the single-session live gauge; the cockpit is cross-session and lives behind a new nav affordance or a Harness sub-tab. (Confirm placement when we get there — see Open Questions.)

---

## 2. The build, phase by phase

Each chunk: **Goal · Files · Pattern · Done-when.** Chunks are sized for one focused sitting.

### PHASE 0 — Ship the insurance (owed, do first next session)
Already-built work that must land before June 15.

- **0a · Ship escape hatch + hardening.**
  - Files: commit the 4 uncommitted (`assistant/mod.rs`, `lib.rs`, `SettingsPage.svelte`, `assistant.svelte.ts`) together with cont.66's `c1cc817`.
  - Pattern: smoke-test live (CDP) → bump THREE version files + `Cargo.lock` → `/git-ship` (see release gotchas in HANDOFF).
  - Done-when: tagged release on `Blazzer10200/rift-releases`, custom-provider routing confirmed in a prod build.

### PHASE 1 — Cost cockpit (the core build of this arc)
Turn the existing per-turn data into a credit-pool fuel gauge. Order matters: store → pricing → aggregation → gauge → UI.

- **1a · SQLite usage store + ingestion.**
  - Files (new): `src-tauri/src/usage/mod.rs`, `src-tauri/src/usage/store.rs`; register module in `lib.rs`; add `rusqlite` (bundled feature) to `Cargo.toml`.
  - Pattern: `turns(session_id, ts, model_id, provider, input, output, cache_read, cache_write, cost_usd_cli, cost_usd_calc, ttfp_ms, duration_ms, workspace, tool_count)`. New command `usage_ingest_turn(TurnRecord)` called from the frontend's existing `recordSessionLog` debounce (`assistant.svelte.ts:1155-1171`). Idempotent upsert keyed on `(session_id, turn_index)`. Backfill command `usage_backfill_from_logs()` reads `session-logs/*.json` once.
  - Done-when: every completed turn lands a row; restarting the app preserves rows the ring buffer would have pruned; backfill populates history.

- **1b · Price table + cost recompute.**
  - Files (new): `src-tauri/assets/model-prices.json`, `src-tauri/src/usage/pricing.rs`. Optional override read from `~/.rift/model-prices.json`.
  - Pattern: D2. Fill `cost_usd_calc` at ingest time. Mark rows whose model isn't in the table so the UI can surface "estimated/unknown pricing" (important for custom providers).
  - Done-when: a DeepSeek turn (custom provider, CLI cost wrong) shows a correct computed cost from a user-supplied price entry.

- **1c · Cross-session aggregation DTOs + commands.**
  - Files (new): `src-tauri/src/usage/aggregate.rs`; commands in `commands/usage.rs` (or fold into `usage/mod.rs`).
  - Pattern: mirror ccusage's views — `usage_daily(range)`, `usage_monthly()`, `usage_by_model()`, `usage_by_workspace()`, `usage_blocks(window=5h)` (Claude billing windows), `usage_session(id)`. Each returns a flat DTO array; SQLite `GROUP BY date/model/workspace`.
  - Done-when: commands return correct rollups verified against a hand-summed sample session.

- **1d · Credit-pool fuel gauge.**
  - Files: `usage/aggregate.rs` (+`store.rs` for the plan config); a small `budget` config block in `AssistantConfig` (`mod.rs:636-701`) or a new `~/.rift/usage-budget.json`.
  - Pattern: user picks plan tier ($20 Pro / $100 Max-5x / $200 Max-20x) + reset cadence. `usage_budget_status()` → `{ spent, limit, pct_remaining, window_start, projected_exhaustion_date }` computed from current-window spend + burn rate. This is the **post-June-15 killer feature** — the 5h-block view (1c) feeds it.
  - Done-when: gauge shows % of pool remaining + a projected dry-out date that moves as turns burn credit.

- **1e · Cost cockpit UI.**
  - Files (new): `src/lib/components/workspaces/CostPage.svelte` (or a Harness sub-view); new store `src/lib/state/usage.svelte.ts`; wire nav.
  - Pattern: fuel gauge hero + daily/monthly bars + per-model/per-provider breakdown + burn-rate projection. Reuse the bento/KPI visual language from `HarnessPage.svelte`. Honor D5 (new view, no scroll-cram).
  - Done-when: live app (CDP `shot`) shows the cockpit reading real persisted history across an app restart.

### PHASE 2 — Escape hatch v2 + grows-with-you foundation
- **2a · Multi-provider list (cc-switch pattern).**
  - Files: `AssistantConfig` (`mod.rs:636-701`) → `providers: Vec<ProviderProfile> { id, name, base_url, model, key_ref }` + `active_provider_id`; new commands alongside the existing four (`mod.rs:1771-1797`); Settings card (`SettingsPage.svelte:639-668`) → list + presets + one-click active toggle.
  - Pattern: bundle presets (DeepSeek, GLM/Zhipu, Bedrock, generic OpenAI-compat gateway). Keep atomic temp+rename writes (already the config pattern). `assistant_send` routing (`mod.rs:2863-2882`) reads the *active* profile instead of the single `base_url`.
  - Done-when: switch providers from a saved list without retyping; the active one routes turns.
  - Optional: route prompt-enhance wand + compaction (`assistant_summarize_session`, `mod.rs:2386`) through the active custom endpoint too (currently Anthropic-only).

- **2b · Insight layer v1 (read-only "Rift noticed…").**
  - Files (new): `src-tauri/src/usage/insights.rs`; a panel in CostPage or Harness.
  - Pattern: deterministic queries over the SQLite corpus — top tools, model mix per workspace, recurring cost sinks, time-of-day burn, cache-efficiency trend. **Observational only** for v1 (no auto-action). This is the seed of Pillar 3; Hermes / compound-engineering inform *what* to surface, not the code.
  - Done-when: panel surfaces ≥3 true, non-trivial patterns from the user's own history.

### PHASE 3 — Multi-agent + compression (later)
- **3a · Sandbox primitive.** Evaluate `anthropic-experimental/sandbox-runtime` (TS, OS-level FS/net restriction, no container) vs `NVIDIA/OpenShell` (Rust). Safety layer for any write-mode fan-out.
- **3b · Edit-applying swarm** (the parked IDEAS.md seed, now de-risked): worktree isolation + one-file-one-agent + `oh-my-pi`-style hash-anchored edits + verify-gate (`cargo check`/`npm run check`) + adversarial diff review. Feed it the audit-swarm's confirmed-findings list.
- **3c · Compression toggle (headroom).** Optional local compression proxy in front of the CLI via the existing `ANTHROPIC_BASE_URL` seam. Caveat: Python-primary runtime dep — ship as an opt-in, not a hard dependency. CacheAligner is interesting given Rift already tracks cache efficiency.

---

## 3. Reference repos (steal these patterns)

| Repo | Stars (2026-06-07) | What to take |
|---|---|---|
| `ryoppippi/ccusage` | 15.6k (Rust) | **Pillar 2 blueprint** — data model, LiteLLM-style price table, daily/monthly/session/5h-block/per-project views |
| `chopratejas/headroom` | 16k (Py/Rust) | Cost-*reduction* proxy; `wrap claude` + `ANTHROPIC_BASE_URL` integration; deterministic local compression |
| `farion1231/cc-switch` | 93k (Tauri+Rust) | Multi-provider escape-hatch UX, SQLite store, presets, atomic writes — direct stack match |
| `stablyai/orca` | 4.3k (TS) | Pillar 4 product reference — fleet of parallel agents, BYO subscription |
| `anthropic-experimental/sandbox-runtime` | 4.3k (TS) | Swarm safety primitive (FS/net restriction, no container) |
| `supermemoryai/supermemory` | 25.9k (TS) | Pillar 3 memory substrate candidate (TS matches frontend) |
| `EveryInc/compound-engineering-plugin` | 20k (TS) | "system improves as you use it" — Pillar 3 design inspiration |
| `colbymchenry/codegraph` | 43k (TS) | Local code knowledge graph — token-reduction + Pillar 3 substrate |
| `can1357/oh-my-pi` | 11k (TS) | Hash-anchored edits — collision-safe parallel-edit pattern for 3b |

---

## 4. Open questions for the user (decide at kickoff, not blocking)
1. **SQLite (D1)** — OK to introduce `rusqlite` as the durable store? (Recommended. Alternative: append-only JSONL.)
2. **Cost cockpit placement (D5)** — new top-level workspace (would make it IA: 5 workspaces) vs a sub-tab inside Harness? Leaning Harness sub-tab to preserve the 4-workspace IA.
3. **Phase 1 scope for first implementation session** — all of 1a–1e, or stop at 1c (data + aggregation) and review before building the gauge + UI?

---

## 5. Session boundaries (how to run the implementation)
- **This session:** planning only (this doc + IDEAS + roadmap updates). ✅
- **Next session = Phase 0 + start Phase 1a–1b** (ship, then store + pricing). Fresh context — read this doc + HANDOFF first.
- **Session +2:** Phase 1c–1e (aggregation → gauge → UI), live-verified via CDP.
- **Session +3+:** Phase 2, then Phase 3.
- Keep each session ≤ ~200K context; `/handoff` extends the project handoff at each close.
