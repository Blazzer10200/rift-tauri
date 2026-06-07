# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-07 (cont. 71) — CHECKPOINT: Phase 1 + Phase 2 committed (review of Session D)

**Phases 1 & 2 committed `1205f12`** (were 3 phases of uncommitted work stacked on v0.6.5 — now checkpointed before Session E). All green: `cargo check` 0 err, `npm run check` 0/0 (4064 files). **NOT shipped — no version bump yet** (still owed; bump THREE files + Cargo.lock + CHANGELOG → release.ps1).

**Reviewed Session D (Phase 2), verdict = ships-worthy:**
- **2a multi-provider (cc-switch):** `AssistantConfig.providers: Vec<ProviderProfile>{id,name,base_url,model,key_ref}` + `active_provider_id`. Legacy single `base_url`/`provider_model` (the shipped v0.6.5 hatch) **migrates once** into the list (gated on empty, clears legacy fields, reuses `ASSISTANT_API_KEY`) — backward-compatible. Keys keychain-scoped (`assistant.provider.<id>`), never serialized; `ProviderDto.has_key` only. Cmds `assistant_{list,save,delete,set_active}_provider` (all under `CONFIG_WRITE_LOCK`). `assistant_send` routes `resolve_active_provider`; model-pin + `--effort` skips still gate on `custom_base.is_none()`.
- **2b insights (`usage/insights.rs`):** deterministic, observational-only "Rift noticed…" probes (dominant-model, cost-sink-ws, peak-window, cache-trend, tool-intensity, custom-provider-spend) w/ real corpus gates (bail <10 turns). Wired into CostPage. No auto-action.

**Not done by me:** no CDP live-verify of Phase 2 (compile + type-check only); the legacy-config→providers migration deserves a live "old hatch still routes" test before/at ship.

### RESUME HERE (next = Session E, Phase 3 — final)
- **Dev STOPPED** (quit `rift-tauri.exe` exact for verification) — restart via `scripts/run-dev.bat` + `npm run cdp:serve`. Never `rift*` glob.
- Open [session-kickoffs.md](design/session-kickoffs.md) → **Session E** (Phase 3: 3a sandbox primitive, 3b edit-applying swarm, 3c compression toggle). Plan = [idea-phase-plan.md](design/idea-phase-plan.md) §2 Phase 3.
- **Owed ship:** after E (or before, as a Phase 1+2 release) — bump + CHANGELOG + release.ps1. Consider a CDP pass on the provider migration first.

## Prior — cont.70 (C) / 69 (A) / 68 (planning)
cont.70 Phase 1 cost cockpit (CDP-verified: 58 turns backfilled survived restart, gauge/budget round-trip). cont.69 v0.6.5 shipped (escape hatch + hardening `c1cc817`). Plan of record = [idea-phase-plan.md](design/idea-phase-plan.md). **Key finding:** Rift already persists per-turn `TurnRecord[]` → Pillars 2/3 = aggregate+price+read-layer, not instrument. Decided: D1 SQLite `~/.rift/rift.db` · D5 cockpit = Harness sub-tab.

## Shipped + prior arcs — detail in `git log`
- **v0.6.5** (cont.69) escape hatch + hardening (`c1cc817`). · **v0.6.4** (cont.65) 401 fix.
- **release.ps1 gotchas:** bump THREE files + `Cargo.lock` BEFORE; clean tree or `-Force`; quit `rift-tauri.exe` (dev) before build; Setup.exe-only; vpk CLI ver == velopack crate ver; NO stderr redirect (PS5.1 NativeCommandError trap).

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
