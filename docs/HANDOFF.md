# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-07 (cont. 69) — SHIPPED v0.6.5 (Phase 0 / Session A): escape hatch + hardening

**Done — Phase 0 ✅.** Custom-provider escape hatch + cont.66 hardening shipped as **v0.6.5**, tagged release live on `Blazzer10200/rift-releases` (Setup.exe + full.nupkg + releases.win.json). Commits on origin/main: escape hatch `a230565`, hardening `c1cc817`, version bump `ca5e083`, planning docs `4831e4c`.

**Smoke-test (CDP, dev) all green:** *Custom provider (advanced)* card renders; Save persists to `~/.rift/assistant/config.json` (`base_url`+`provider_model`); routed turn confirmed CLI POSTs `/v1/messages` to the custom endpoint w/ bearer auth (not api.anthropic.com). Clear works; **config restored to `null` (dev+prod share that file — left clean).**

**Routing (mod.rs `assistant_send`):** `base_url` set → `ANTHROPIC_BASE_URL`+`ANTHROPIC_AUTH_TOKEN`, `provider_model` overrides tier, skips Anthropic-only model-pin + `--effort`. Cmds: `assistant_{get,set}_{base_url,provider_model}`.

**Ship traps hit:** (1) `@'...'@` is a PS here-string — in **Git Bash** it injects a literal `@` into the commit subject; use `-F file`/multiple `-m`. (2) `& .\release.ps1 *>&1 | Tee` under PS5.1 turns cargo-tauri stderr "Info" into terminating `NativeCommandError` (script `$EAP='Stop'`) → **run release.ps1 with NO redirect.**

### RESUME HERE (next session = Session B, Phase 1a–1b)
- **Dev/cdp stopped for the build** — restart via `scripts/run-dev.bat` + `npm run cdp:serve`. Kill targets `rift-tauri` EXACTLY, never `rift*`.
- **Optional prod-confirm:** in-app updater pulls v0.6.5 on next launch/6h; routing already behavior-verified in dev (same spawn code).
- Open [session-kickoffs.md](design/session-kickoffs.md) → **Session B** (SQLite usage store + pricing). Plan of record = [idea-phase-plan.md](design/idea-phase-plan.md) §1 (SQLite at `~/.rift/rift.db`, `rusqlite`) + §2 Phase 1. Re-anchor line numbers by snippet. Don't re-plan.

## Prior — cont.68 (planning)
Plan of record: [idea-phase-plan.md](design/idea-phase-plan.md) (+ [session-kickoffs.md](design/session-kickoffs.md), roadmap, [IDEAS.md](IDEAS.md)). **Key finding:** Rift already persists per-turn `TurnRecord[]` to `~/.rift/assistant/session-logs/<id>.json` → Pillar 2 = aggregate+price (NOT instrument); Pillar 3 = read-layer. Holes: logs ring-buffered (`session_log.rs:138-168`, lossy) → durable store; CLI `total_cost_usd` wrong for custom providers → price table. **Decided:** D1 SQLite `~/.rift/rift.db` · D5 cockpit = Harness sub-tab · order store→price→aggregate→gauge→UI.

## Shipped + prior arcs — detail in `git log`
- **v0.6.5** (cont.69) escape hatch + cont.66 hardening (74 fixes, 36 files, `c1cc817`). · **v0.6.4** (cont.65, `3d89538`) collaborator 401 fix.
- **release.ps1 gotchas:** bump THREE files + `Cargo.lock` BEFORE; clean tree or `-Force`; quit `rift-tauri.exe` (dev) before build; Setup.exe-only; vpk CLI ver == velopack crate ver; run with NO stderr redirect (PS5.1 NativeCommandError trap).

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = sidebar + 5 sections; Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.6.5 stands** (shipped 2026-06-07, cont.69).
