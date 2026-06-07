# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-07 (cont. 72) — SHIPPED v0.7.0 + BUILT Phase 3b edit-swarm

**v0.7.0 SHIPPED** (`f687873`, tag on `Blazzer10200/rift-releases`, Setup.exe + full.nupkg). Cleared the Phase 1+2 ship debt: cost cockpit + multi-provider + insights. CDP-smoke-tested live first (real history, 3 insight patterns, provider list). Bumped via `bump.ps1`; `release.ps1` clean.

**Phase 3b edit-applying swarm — BUILT + live-verified, committed `db01c70`, NOT shipped.** New `src-tauri/src/swarm/mod.rs` orchestrator + `swarm_run`/`swarm_env_check` (registered `lib.rs`) + Harness→**Swarm** sub-tab (`SwarmPage.svelte` + `swarm.svelte.ts`). Flow + §7 decisions (dedicated cargo target / cherry-pick merge / own module / diff-vs-finding review) fully in [edit-swarm-safety-layer.md](design/edit-swarm-safety-layer.md) §4+§7. SAFE cleanup = rmdir junction THEN worktree remove (never recurse the junction).
- **Verified:** `cargo check` 0/0 · `npm run check` 0/0 (4066) · deterministic `#[ignore]` mechanics test (`cargo test ... swarm -- --ignored`: gate discrimination + main-tree isolation + no leak) · **LIVE end-to-end** (throwaway repo: real edit agent → review ACCEPT → cherry-pick → `merged:true`, main intact, zero leak).

### RESUME HERE (next session)
- **Dev was running this session; quit it (`rift-tauri.exe` EXACT) before any `cargo`/build. Never `rift*` glob.** Restart: `scripts/run-dev.bat` + `npm run cdp:serve`.
- **3c (compression toggle) is the LAST idea-phase item** — `headroom`-style local proxy via the `ANTHROPIC_BASE_URL` seam (`mod.rs:~3390`); opt-in only, Python soft-dep, off by default. See [session-kickoffs.md](design/session-kickoffs.md) Session E STEP 3.
- **Optional:** ship Phase 3b (committed, unshipped) as its own release after a soak, or bundle with 3c. Repeat the release guardrails (THREE files + Cargo.lock + CHANGELOG → `release.ps1`).

## Prior — cont.71/70/69
cont.71 checkpointed Phase 1+2 (`1205f12`). cont.70 cost cockpit. cont.69 v0.6.5 escape hatch. **Key finding:** Rift persists per-turn `TurnRecord[]` → Pillars 2/3 = aggregate+price+read-layer. D1 SQLite `~/.rift/rift.db`.

## Shipped + prior arcs — detail in `git log`
- **v0.6.5** (cont.69) escape hatch + hardening (`c1cc817`). · **v0.6.4** (cont.65) 401 fix.
- **release.ps1 gotchas** (full list in project CLAUDE.md): THREE files + `Cargo.lock` BEFORE; clean tree or `-Force`; quit `rift-tauri.exe`; Setup-only; vpk ver == velopack crate ver; no PS5.1 stderr redirect.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = sidebar + 5 sections; Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.7.0 stands** (shipped 2026-06-07, cont.72). Harness now has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
