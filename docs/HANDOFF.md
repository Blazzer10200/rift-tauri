# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-07 (cont. 65) — SHIPPED v0.6.4: collaborator 401 fix + leaner releases

**Bug:** a collaborator (Pro/Max, signed in, terminal `claude` works) got `401 Invalid authentication credentials` in Rift; in-app CLI update sat stuck "still behind after update." **Root cause (proven via his screenshots):** `install_is_better` ranked **newest version above on_path**, so when a box has >1 install Rift spawned a never-logged-into native copy off-PATH (`%LocalAppData%\AnthropicClaude`) over his on-PATH npm (logged-in) copy → 401. The same dead native copy's broken self-update = "reported success without bumping." **Fix 1 (cure):** reordered `install_is_better` → real-binary > **on_path** > version > method ([mod.rs:468](src-tauri/src/assistant/mod.rs)); Rift now runs what the shell/login uses. **Fix 2 (UX):** result-frame 401s were forwarded RAW (friendly remap only covered the stderr-exit path) → added auth-error detection at the `result`-frame forward ([mod.rs:~3270](src-tauri/src/assistant/mod.rs)) emitting an actionable ERROR_EVENT (names active CLI path → `claude login` / Settings). **Releases:** `release.ps1` `vpk pack --delta None` → no more delta `.nupkg` (asset list 7→6; the 2 GitHub "Source code" archives are auto-generated + unremovable). `cargo check` 0/0 · `npm run check` 0/0 (4062). SHIPPED via `release.ps1` (commit `3d89538`, pushed to both repos).

### RESUME HERE (cont.65)
- **v0.6.4 SHIPPED + published** to `rift-releases` (tag `v0.6.4`, 4 vpk assets: RELEASES + releases.win.json + full.nupkg + Setup.exe — delta gone). Source commit `3d89538` pushed to `origin/main`. Tree clean.
- **⏳ Owed live-verify:** collaborator updates to 0.6.4 → his 401 should be gone (Rift now spawns his on-PATH logged-in install). NOT yet confirmed on his box.
- **NOT done (deliberate):** (a) **in-app sign-in button** (user asked for "in-app auth") — deferred; subscription OAuth needs interactive `claude login`, can't verify autonomously. Install fix makes it non-urgent. (b) **Retroactive delta trim** on prior published releases (v0.6.1–0.6.3 still carry deltas) — left alone to avoid any risk to the just-fixed update feed; trim via `gh release delete-asset` if wanted. (c) dead native install still nags `isAnyStale` until uninstalled (native self-updater no-op is Claude's, not Rift's).
- **Cosmetic, deferred:** update-surface drift (toast "update available" vs card "up to date").
- **v0.6.0 carry-over live-verify still owed:** browser render-flash · mid-turn steer · permission bar · fresh-install onboarding.
- **Open:** #21 test harness (T1) · #4/#20/#17 strategic · #29 Tailwind-blocked · CR-UX trust-enum sign-off.

---

## Shipped + prior arcs — detail in `git log`
- **v0.6.3** (cont.64) live-verify no-op bump — in-app auto-apply CONFIRMED in prod · **v0.6.2** (cont.64, `f67e2d7`) update apply-path fix (child-process file-lock reap) · **v0.6.1** (cont.63) CLI multi-install + unified update UI · **v0.6.0** (cont.61, `316dc5e`) browser dock + polish · **v0.5.0** (cont.51, `62dae27`) Velopack stable.
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
