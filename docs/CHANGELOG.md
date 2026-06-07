# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.7.0 — 2026-06-07 — feat: cost cockpit + multi-provider list + "Rift noticed…" insights

> **Why.** From June 15 the Claude subscription stops covering programmatic/headless turns, so every Rift turn draws metered credit. You need to *see* the burn, *project* the dry-out, and *route off* the metered pool when you choose. This release turns Rift's per-turn telemetry — already captured, but ring-buffered and single-session — into a durable, cross-session cost cockpit.

**Durable usage store (SQLite).** New `usage/` backend module on bundled `rusqlite` at `~/.rift/rift.db`. Every finalized turn upserts a `turns` row (idempotent on `session_id,turn_index`) via `usage_ingest_turn`, hooked into the existing session-log debounce — so history survives the session-log ring buffer's pruning. A one-shot `usage_backfill_from_logs()` seeds the DB from existing `session-logs/*.json` so day-one history isn't blank.

**Independent cost (price table).** Bundled `model-prices.json` (USD per Mtok, input/output/cache) computes `cost_usd_calc` at ingest, overridable from `~/.rift/model-prices.json`. Custom-provider turns whose CLI cost is wrong/absent now get a correct computed cost; unknown models are flagged **EST**.

**Cost cockpit (Harness → Cost sub-tab).** A credit-pool fuel gauge: pick your plan ($20 Pro / $100 Max-5× / $200 Max-20× / custom) + reset cadence, see % pool left, spend, burn rate, runway, and a projected dry-out date that moves as you spend. Plus daily-spend bars and by-model / by-workspace breakdowns. Reuses the Harness bento language; preserves the 4-workspace IA and the one-viewport invariant (it's a sub-tab, not a 5th workspace).

**Multi-provider list (cc-switch pattern).** The single custom-provider field becomes a saved list: `providers: Vec<ProviderProfile>` + `active_provider_id`, with presets (DeepSeek, GLM/Zhipu, Bedrock, generic OpenAI-compat) and one-click active toggle. The active profile routes `assistant_send` via `ANTHROPIC_BASE_URL`. The v0.6.5 single-field hatch migrates once into the list (backward-compatible). Keys stay OS-keychain-scoped, never serialized.

**"Rift noticed…" insights.** Deterministic, observational-only probes over the corpus — dominant model, costliest workspace, peak burn window, cache trend, tool intensity, custom-provider spend — surfaced in the cockpit (no auto-action). Bails below 10 turns of history.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 · cockpit + insights + provider list live-verified via CDP against real history.

## Older versions

v0.6.5 custom-provider escape hatch + cont.66 hardening · v0.6.4 collaborator-401 install-selection fix + leaner releases · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
