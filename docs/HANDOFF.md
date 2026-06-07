# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-06 (cont. 64) — SHIPPED v0.6.2: in-app update apply fix (file-lock)

**Bug:** clicking Update downloaded but never applied — app relaunched on same build. **Root cause (proven):** Velopack applies by renaming `current/`; its `Update.exe` waits only for the MAIN pid. But each turn's claude CLI spawns `rift-tauri.exe` (`RIFT_MCP_SERVER=1`) as MCP server, and ANY live `rift-tauri.exe` holds an exclusive lock on `current/` (verified: rename → sharing violation). `app.exit(0)` = `std::process::exit` skips `Drop` → `kill_on_drop` never reaps the children → they orphan → swap blocked. Standalone download+apply proven working (probe + manual `Update.exe apply`). **Fix:** `update_service::apply()` now reaps lockers before `app.exit(0)` — `assistant::kill_all_session_children()` (tree-kills tracked claude trees) then a `taskkill /FI "IMAGENAME eq rift-tauri.exe" /FI "PID ne <self>"` sweep. `cargo check` + `npm run check` 0/0. SHIPPED via `release.ps1` (commit `f67e2d7`, pushed).

### cont.65 — FIX: buddy 401 + stuck CLI update (install-resolution root cause)
- **Symptom:** collaborator (Pro/Max, logged in, terminal `claude` works) got `401 Invalid authentication credentials` in Rift; in-app CLI update stuck "still behind after update" at native 2.1.153→2.1.168.
- **Root cause:** `install_is_better` ([mod.rs:468](src-tauri/src/assistant/mod.rs)) ranked **newest version above on_path**. His native off-PATH install (LOCALAPPDATA, never logged into) outranked his on-PATH npm (logged-in) copy → Rift spawned the wrong one → 401. Same dead native copy's broken self-update = "reported success without bumping."
- **Fix 1 (cure):** reordered `install_is_better` → shim-check → **on_path** → version → method. Rift now spawns the install the user's shell/login uses. `cargo check` 0 errors.
- **Fix 2 (UX):** result-frame 401s were forwarded RAW (the existing friendly remap only covered the stderr-exit path). Added auth-error detection at the `result`-frame forward ([mod.rs:~3270](src-tauri/src/assistant/mod.rs)) → emits actionable ERROR_EVENT (names the active CLI path, points to `claude login` / Settings → CLI session).
- **NOT YET:** (a) not shipped — buddy needs a published build (release.ps1); (b) in-app sign-in button (user asked for "in-app auth") still a follow-up — subscription OAuth needs interactive `claude login`; (c) dead native install will still nag `isAnyStale` until buddy uninstalls it (native self-updater no-op is Claude's, not Rift's).

### cont.65 — release asset cleanup
- User flagged GitHub release clutter (7 assets/release). Reality: `releases.win.json` + full `.nupkg` + `Setup.exe` mandatory for Velopack; `RELEASES` (74B legacy) left in place (risky to remove — feed continuity); `Source code zip`+`tar.gz` are GitHub auto-generated per tag, **unremovable**. Only real lever = delta package.
- **Done:** `release.ps1` `vpk pack` now passes `--delta None` → drops `Rift-X-delta.nupkg` (7→6 assets/release). Clients download full pkg (fine at this size/userbase). Takes effect NEXT release; published v0.6.3 etc. keep their delta unless retroactively trimmed via `gh release delete-asset`.
- Offered-but-not-done: retroactive delta trim on published releases; aggressive `RELEASES` removal (small brick risk).

### RESUME HERE (cont.64)
- **v0.6.2 + v0.6.3 SHIPPED + published** to `rift-releases`. v0.6.2 = the apply fix; v0.6.3 = a no-op version bump shipped solely as the live-verify vehicle (CHANGELOG + design-doc + this-file CLAUDE.md updater note all updated to match). This box manually bootstrapped to 0.6.2 via the published `Rift-win-Setup.exe` (verified `sq.version`=0.6.2), left running on 0.6.2 on purpose. **Bootstrap caveat:** the fix lives in the APPLYING binary, so clients on ≤0.6.1 still hit the lock updating *to* 0.6.2 → one-time manual Setup.exe (called out in CHANGELOG + release notes). v0.6.2→later auto-applies.
- **✅ Update live-verified (cont.65):** clicked **Update** in running 0.6.2 → auto-applied to 0.6.3, relaunched clean, no manual step. v0.6.2 child-reap/file-lock fix CONFIRMED in prod.
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
