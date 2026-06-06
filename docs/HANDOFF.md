# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-06 (cont. 64) — SHIPPED v0.6.2: in-app update apply fix (file-lock)

**Bug:** clicking Update downloaded but never applied — app relaunched on same build. **Root cause (proven):** Velopack applies by renaming `current/`; its `Update.exe` waits only for the MAIN pid. But each turn's claude CLI spawns `rift-tauri.exe` (`RIFT_MCP_SERVER=1`) as MCP server, and ANY live `rift-tauri.exe` holds an exclusive lock on `current/` (verified: rename → sharing violation). `app.exit(0)` = `std::process::exit` skips `Drop` → `kill_on_drop` never reaps the children → they orphan → swap blocked. Standalone download+apply proven working (probe + manual `Update.exe apply`). **Fix:** `update_service::apply()` now reaps lockers before `app.exit(0)` — `assistant::kill_all_session_children()` (tree-kills tracked claude trees) then a `taskkill /FI "IMAGENAME eq rift-tauri.exe" /FI "PID ne <self>"` sweep. `cargo check` + `npm run check` 0/0. SHIPPED via `release.ps1` (commit `f67e2d7`, pushed).

### RESUME HERE (cont.64)
- **v0.6.2 + v0.6.3 SHIPPED + published** to `rift-releases`. v0.6.2 = the apply fix; v0.6.3 = a no-op version bump shipped solely as the live-verify vehicle (CHANGELOG + design-doc + this-file CLAUDE.md updater note all updated to match). This box manually bootstrapped to 0.6.2 via the published `Rift-win-Setup.exe` (verified `sq.version`=0.6.2), left running on 0.6.2 on purpose. **Bootstrap caveat:** the fix lives in the APPLYING binary, so clients on ≤0.6.1 still hit the lock updating *to* 0.6.2 → one-time manual Setup.exe (called out in CHANGELOG + release notes). v0.6.2→later auto-applies.
- **⏳ Owed live-verify (user-driven):** click **Update** in the running 0.6.2 app → should land on 0.6.3 with no manual step. For the strongest proof, start a turn first so an MCP `rift-tauri.exe` child is alive (the exact lock case). If it relaunches as 0.6.3, the fix is fully confirmed; if it comes back 0.6.2, pull the diagnostic. As of cont.64 close the box was still 0.6.2 (button not yet clicked).
- **Cosmetic, deferred:** update-surface drift (toast "update available" vs a card showing "up to date") — user flagged via screenshot, not yet chased.
- **v0.6.0 carry-over live-verify still owed:** browser render-flash · mid-turn steer · permission bar · fresh-install onboarding.
- **Open:** #21 test harness (T1) · #4/#20/#17 strategic · #29 Tailwind-blocked · CR-UX trust-enum sign-off.

---

## Shipped + prior arcs — detail in `git log`
- **v0.6.1** (cont.63) CLI multi-install + unified update UI · **v0.6.0** (cont.61, `316dc5e`) browser dock + polish (includes the cont.57 model-picker capability matrix — that work shipped here, NOT pending) · **v0.5.0** (cont.51, `62dae27`) Velopack stable.
- **release.ps1 gotchas:** bump THREE files + `Cargo.lock` (run `cargo check` so the lock updates) BEFORE; commit for a clean tree or pass `-Force`; quit `rift-tauri.exe` (dev) before build — Win file-lock; drop portable AFTER `vpk upload`; never wrap `release.ps1`/`tauri build` in `*>&1` from the PS tool (PS5.1 → terminating `NativeCommandError`). Setup.exe-only. vpk CLI version == velopack crate version.
- **Carry-over:** `check.yml` per-push email spam; prod app now ALSO `rift-tauri.exe` → revisit "never blanket-kill rift" rule.

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
