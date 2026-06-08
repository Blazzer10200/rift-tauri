# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-08 (cont. 74) — Auth-recovery feature + test-coverage stabilization (autonomous)

**Auth recovery (BUILT + CDP-verified, UNSHIPPED).** Fixes the dead-end 401 a collaborator hit. Root cause: `claude auth status` can report `loggedIn:true` for a stale OAuth token the API rejects — send-gate passes, turn 401s, only fix offered was "open a terminal." Backend `assistant_open_login(console)` (`mod.rs`, reg. `lib.rs`) spawns `<active claude> auth login` in its OWN console (`CREATE_NEW_CONSOLE`), strips `ANTHROPIC_API_KEY`; creds land in the CLI's store → real fix. Store `startLogin()` polls probe→green then clears; `recheckAuth()` clears on Re-check. `AssistantPane.svelte` → actionable banner: **[Sign in]** (login 401) / **[Open Settings]** (key 401) / **[Re-check]**. Commits `9c468a4`+`2d72af8`; CDP-verified all states + nav live (not the real login spawn).

**Test coverage (6 commits `756c95b`→`d929408`, UNSHIPPED).** Rust suite 15→61, vitest 22→27 — covered the ZERO-test MCP/git/validator/usage/browser/paths/stt/redact surfaces (`tempfile` now dev-dep). **#21 half DONE; stream-pump still open** (frame-classify inline in async loops `mod.rs` ~2374/2685/3750 → needs playback harness, also unblocks #20 M8/M9).

### RESUME HERE (next session)
- **SHIP: HOLDING FOR A SOAK.** 3b (`db01c70`) + 3c (`0c34161`) + **auth-recovery (`9c468a4`,`2d72af8`)** + test coverage committed + verified + UNSHIPPED; repo on v0.7.0. Ship as a bundle (**v0.8.0**) — auth recovery is the headline user-facing change.
- **To ship:** `pwsh scripts/bump.ps1 0.8.0` → write `docs/CHANGELOG.md` top entry (≤600w, must match version) + commit `Cargo.lock` → quit `rift-tauri.exe` (EXACT, never `rift*` glob) → `pwsh scripts/release.ps1`. Guardrails: THREE files + Cargo.lock lockstep or preflight bails; clean tree or `-Force`; vpk ver == velopack crate ver (`=1.2.0`); no PS5.1 stderr redirect.
- Dev + cdp wrapper were stopped at session end.

## Prior arcs — detail in `git log`
cont.73 BUILT Phase 3c compression toggle (`0c34161`, UNSHIPPED) via the `ANTHROPIC_BASE_URL` seam — completes the idea-phase arc (3a+3b+3c). cont.72 SHIPPED v0.7.0 (`f687873`, cost cockpit + multi-provider + insights) + BUILT Phase 3b edit-swarm (`db01c70`, `swarm/mod.rs` + Harness→Swarm sub-tab, UNSHIPPED). cont.71 Phase 1+2 (`1205f12`); 3b cleanup safety in [edit-swarm-safety-layer.md](design/edit-swarm-safety-layer.md) §4+§7. v0.6.5 (cont.69) `c1cc817`; v0.6.4 (cont.65) 401 fix. release.ps1 gotchas → project CLAUDE.md.

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
