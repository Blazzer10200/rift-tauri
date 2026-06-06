# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-06 (cont. 61) — SHIPPED v0.6.0 (browser arc + polish)

Full pre-ship review + release. **v0.6.0 published** to `rift-releases` (commit `316dc5e`, pushed to origin). Browser arc + harness/model-picker polish + #31–36 all shipped.
- **Review:** `svelte-check` 0/0 · `cargo check` 0/0 · `clippy` → 1 intentional ([mod.rs:2432](../src-tauri/src/assistant/mod.rs) arity) · browser dock + per-turn MCP pipeline CDP-verified live (omnibox, add-to-chat, back/fwd, reload). `{@html}` in [EditDiff.svelte](../src/lib/components/assistant/EditDiff.svelte) safe — shiki-escaped tokens.
- **Fixes this session (behavior-preserving, verified):** removed dead [secrets.rs](../src-tauri/src/secrets.rs) keys (`bridge_token_key`/`rcon_password_key`); 6 clippy idioms (`sort_by_key`/`?`/match).
- **Release:** **Setup.exe-only** — portable dropped *post-upload* (`release.ps1` patched + committed). Assets: Setup.exe + full/delta nupkg + releases.win.json + RELEASES. Velopack delta 0.5.0→0.6.0 built. The two GitHub "Source code" archives = LICENSE+README only (rift-releases repo), auto-gen + unremovable + harmless. Also pruned the portable from the live v0.5.0 release.
- **release.ps1 gotchas learned:** (1) drop portable AFTER `vpk upload` — vpk's pack manifest needs the file present at upload time; (2) if a pack half-runs, clean the 0.6.0 artifacts from `Releases/` before retry (vpk refuses to re-pack over an existing ≥ version); (3) never wrap `release.ps1`/`tauri build` in `*>&1`/`2>&1` from the PS tool — PS5.1 turns native stderr into a terminating `NativeCommandError`.

### RESUME HERE (cont.61)
- **v0.6.0 SHIPPED + pushed. Tree clean.**
- **PENDING USER live-verify:** v0.6.0 auto-update on a real machine (v0.5.0→v0.6.0 = 2nd Velopack proof point) · browser render-flash visual (native webview CDP-invisible) · steer mid-turn on a tool turn · permission Allow/Deny bar · beta onboarding on a fresh install.
- **Open:** #21 test harness (T1) · #30 Update-UI redesign · #4/#20/#17 strategic · #29 Tailwind-blocked · CR-UX trust-enum sign-off.
- Dev pipeline was killed for the release build — relaunch via `scripts/run-dev.bat` (sets the CDP port) if resuming UI work.

---

## Prior unshipped (detail in git log, all ride next ship)
- **cont.57:** Model-picker capability matrix (`Composer.svelte` `ModelOpt`) — per-model `effortStops`+`$effect` auto-clamp, amber Ultracode caption, fast-mode behind `FAST_MODE_WIRED=false` (→#31). **cont.55–56:** Harness motion polish + beta-notice onboarding step (see CRITICAL gate). **cont.52–54:** `Markdown.svelte` streaming reveal · Harness dead-wait `.tl-dead` · ISSUES #32–35 + KPI no-scroll redesign.

---

## Shipped + prior arcs — detail in `git log`
- **v0.5.0** (2026-06-04, cont.51, `62dae27`): Velopack stable to `rift-releases`. **Pending live-confirm:** 2nd auto-update proof point on a real machine.
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
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. **v0.5.0 stands** (shipped 2026-06-04, cont.51).
