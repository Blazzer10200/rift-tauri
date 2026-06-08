# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-08 (cont. 74) — SHIPPED v0.8.0 (auth-recovery + edit-swarm + compression + test coverage)

**Auth recovery (SHIPPED v0.8.0, `9c468a4`+`2d72af8`).** Root cause of the collaborator's dead-end 401: `claude auth status` reports `loggedIn:true` for a stale OAuth token the API rejects → send-gate passes, turn 401s. Fix: backend `assistant_open_login(console)` (`mod.rs`/`lib.rs`) spawns `claude auth login` in its OWN console (`CREATE_NEW_CONSOLE`); creds land in the CLI's store → real fix. `AssistantPane.svelte` actionable banner [Sign in]/[Open Settings]/[Re-check]; `startLogin()` polls probe→green + clears. CDP-verified (not the real login spawn).

**Test coverage (`756c95b`→`d929408`, shipped v0.8.0).** Rust 15→61, vitest 22→27 — covered ZERO-test MCP/git/validator/usage/browser/paths/stt surfaces. **#21 half done; stream-pump open** (`mod.rs` ~2374/2685/3750, needs playback harness → also unblocks #20 M8/M9).

### RESUME HERE (next session)
- **SHIPPED v0.8.0 (2026-06-08, release commit `5a4a2d5`):** auth-recovery + 3b edit-swarm + 3c compression + test coverage, published to `rift-releases` (Setup.exe + `Rift-0.8.0-full.nupkg`). Clients auto-update on next launch / 6h Velopack check. Repo now on v0.8.0.
- **⚠️ Killed the user's PROD app TWICE this arc** (name collision: post-Velopack the installed app AND dev binary are BOTH `rift-tauri.exe`). Rule fixed in project CLAUDE.md → **kill dev by PID only, never `taskkill /IM rift-tauri.exe`.** v0.8.0 build ran fine alongside the running app (build → `target/`, not `%LOCALAPPDATA%`), so no quit needed.
- **Release gotcha (new):** never wrap `release.ps1` with `*>&1`/`Tee` under PS5.1 — `tauri build`'s informational stderr wraps as `NativeCommandError` and `$ErrorActionPreference='Stop'` aborts it. Run bare; tool captures stderr. (No `pwsh` on PATH — use the PowerShell 5.1 tool.)
- **Open:** #21 stream-pump playback harness (also unblocks #20 M8/M9); the in-app `[Sign in]` spawn path is only compile/CDP-verified, not a real logged-out round-trip.

## Prior arcs — detail in `git log`
cont.73 Phase 3c compression toggle (`0c34161`) via the `ANTHROPIC_BASE_URL` seam — completes the idea-phase arc (3a+3b+3c), all shipped in v0.8.0. cont.72 SHIPPED v0.7.0 (`f687873`, cost cockpit + multi-provider + insights) + Phase 3b edit-swarm (`db01c70`, `swarm/mod.rs` + Harness→Swarm sub-tab). cont.71 Phase 1+2 (`1205f12`); 3b cleanup safety in [edit-swarm-safety-layer.md](design/edit-swarm-safety-layer.md) §4+§7. v0.6.5 (cont.69) `c1cc817`; v0.6.4 (cont.65) 401 fix. release.ps1 gotchas → project CLAUDE.md.

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
