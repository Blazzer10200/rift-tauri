# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-07 (cont. 68) — PLANNING: idea-phase master plan + session kickoffs

**Output = three planning docs, research-grounded (95% accuracy target):**
- **[idea-phase-plan.md](design/idea-phase-plan.md)** — file-accurate build sheet for the 4-pillar arc. Phases 0→3, each chunk Goal·Files·Pattern·Done-when. **Plan of record — read before implementing.**
- **[session-kickoffs.md](design/session-kickoffs.md)** — paste-ready launch prompt per upcoming session (A=ship, B=SQLite+pricing, C=aggregation+gauge+UI, D=hatch v2+insights, E=multi-agent).
- **[IDEAS.md](IDEAS.md)** + roadmap updated w/ deep trending re-sweep + competitive reframe.

**Load-bearing findings (codebase-verified):** Rift ALREADY persists per-turn metrics to `~/.rift/assistant/session-logs/<id>.json` (`TurnRecord[]`: cost/tokens/cache/ttfp/tool-timing) → **Pillar 2 = aggregate+price+budget, NOT instrument; Pillar 3 = read-layer over same data.** Two holes: session-logs ring-buffered (`session_log.rs:138-168`, history lossy) → need durable store; cost from CLI `total_cost_usd` is WRONG for custom providers → need bundled price table. Reference blueprints: `ccusage` (Pillar 2), `cc-switch` (hatch v2, same stack), `headroom` (compression), `sandbox-runtime` (swarm safety).

**Decided (vetoable at kickoff):** D1 introduce SQLite `~/.rift/rift.db` (`rusqlite`) as durable metrics+memory store · D5 cost cockpit = Harness sub-tab, NOT 5th workspace · Phase 1 order: store→price→aggregate→gauge→UI.

**Escape hatch (cont.67) still UNCOMMITTED + owed ship:** 4 files (`assistant/mod.rs`, `lib.rs`, `SettingsPage.svelte`, `assistant.svelte.ts`), routing at `mod.rs:2863-3172`, green (`cargo check` 0 · `npm check` 0/0). Ship w/ cont.66 `c1cc817`.

### RESUME HERE (next session = Session A, Phase 0)
- **Start clean:** open [session-kickoffs.md](design/session-kickoffs.md) → paste the **Session A** prompt (ship escape hatch + hardening). Then Session B (SQLite+pricing).
- **Dev/cdp from cont.67 likely dead** (machine state) — restart via `scripts/run-dev.bat` + `npm run cdp:serve` if needed. Kill targets `rift-tauri` EXACTLY, never `rift*`.
- **Don't re-plan** — transcribe from idea-phase-plan.md; re-anchor line numbers by snippet (commits drift them).

## Shipped + prior arcs — detail in `git log`
- **cont.66** hardening pass — 74 fixes, 36 files, `c1cc817`; committed, NOT shipped (owed smoke-test). · **v0.6.4** (cont.65, `3d89538`) collaborator 401 fix + leaner releases.
- **release.ps1 gotchas:** bump THREE files + `Cargo.lock` BEFORE; clean tree or `-Force`; quit `rift-tauri.exe` (dev) before build (Win file-lock); Setup.exe-only; vpk CLI ver == velopack crate ver.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = sidebar + 5 sections; Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. **v0.6.1 stands** (shipped 2026-06-06, cont.63).
