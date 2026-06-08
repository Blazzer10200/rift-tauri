# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-07 (cont. 73) — BUILT Phase 3c compression toggle (idea-phase arc COMPLETE)

**Phase 3c — BUILT + live-verified, committed `0c34161`, NOT shipped.** Opt-in context-compression toggle: routes turns through a local compression proxy via the existing `ANTHROPIC_BASE_URL` seam (same seam the provider router uses). Off by default; the Python proxy runtime (`headroom`) is a SOFT dep Rift never bundles or spawns — user runs it, Rift owns only the env seam + a reachability check.
- **Backend** (`assistant/mod.rs`): `compression_enabled`/`compression_proxy_url` config; `resolve_compression()`; `assistant_get/set_compression` + `compression_env_check` (TCP probe + headroom/python PATH detect); `assistant_send` injects `ANTHROPIC_BASE_URL`→proxy when enabled AND no custom provider active (custom provider wins the seam). Registered `lib.rs`.
- **Frontend:** state+setter+env-check in `assistant.svelte.ts`; "Context compression (advanced)" card in `SettingsPage.svelte` (between provider list + compaction).
- **Verified:** `cargo check` 0 err · `npm run check` 0/0 (4066) · **LIVE via CDP** — toggle off by default, persists to disk (`compression_enabled`), env-check honestly reported no-proxy + python-found, then reset to off.

This **completes the idea-phase arc** (Phases 0–3 all built). 3a+3b+3c done.

### RESUME HERE (next session)
- **SHIP: HOLDING FOR A SOAK (user decision, cont.73).** 3b (`db01c70`) + 3c (`0c34161`) committed + verified + UNSHIPPED; repo on v0.7.0. Ship as a bundle (suggest **v0.8.0**) in a later dedicated session once they've soaked.
- **To ship:** `pwsh scripts/bump.ps1 0.8.0` → write `docs/CHANGELOG.md` top entry (≤600w, must match version) + commit `Cargo.lock` → quit `rift-tauri.exe` (EXACT, never `rift*` glob) → `pwsh scripts/release.ps1`. Guardrails: THREE files + Cargo.lock lockstep or preflight bails; clean tree or `-Force`; vpk ver == velopack crate ver (`=1.2.0`); no PS5.1 stderr redirect.
- Dev + cdp wrapper were stopped at session end.

## Prior — cont.72/71/70
cont.72 SHIPPED v0.7.0 (`f687873`, cost cockpit + multi-provider + insights) + BUILT Phase 3b edit-swarm (`db01c70`, `swarm/mod.rs` + Harness→Swarm sub-tab, live end-to-end verified, UNSHIPPED). cont.71 checkpointed Phase 1+2 (`1205f12`). **Key finding:** Rift persists per-turn `TurnRecord[]`; D1 SQLite `~/.rift/rift.db`. 3b SAFE cleanup = rmdir junction THEN worktree remove (never recurse the junction). Detail in [edit-swarm-safety-layer.md](design/edit-swarm-safety-layer.md) §4+§7.

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
