# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-08 (cont. 75) — SHIPPED v0.8.1 (app-update observability hotfix)

**Update-click "does nothing" report (user on v0.7.0).** Exhaustively cleared the update *infrastructure*: v0.8.0 nupkg byte-perfect (size+SHA1+SHA256 == feed), engine+apply both work (velopack_Rift.log shows clean 0.6.3 apply), and every update-flow file is **byte-identical v0.7.0→HEAD** + reads correct (commands registered/managed, capabilities fine). No 0.8.0 nupkg ever staged in `packages/` → failure is at check/download, never reached apply. Couldn't repro (prod exposes no CDP; **no persistent app log** — in-process Velopack logs go to stderr = `/dev/null` in GUI prod). **That invisibility WAS the bug** → recurred undiagnosed.

**Fix (SHIPPED v0.8.1, `5845487`, pushed origin + published rift-releases).** (1) `diagnostics::LogForwarder` now has a rotating file sink → `<appLogDir>/rift.log` (5MB roll), captures Velopack internals + all `log` records. (2) `update_service` explicit info/error markers around check/download/apply. (3) `updates.svelte.ts` download failure forces dialog open + sticky toast w/ **[Get it on GitHub]** manual-install fallback → never silent, never stuck.

### RESUME HERE (next session)
- **SHIPPED v0.8.1 (release `5845487`, tag v0.8.1):** Setup.exe + `Rift-0.8.1-full.nupkg` on `rift-releases`. **User will manually run Setup.exe** (broken v0.7.0 updater can't self-deliver). After they're on v0.8.1: if any future update fails, `rift.log` in Settings→Logs has the exact error — **ask them for it to root-cause the real download failure** (still unknown; only made observable).
- **Open thread:** the actual v0.7.0→v0.8.0 download failure cause is NOT yet found — v0.8.1 only instruments it. Next datapoint = the user's `rift.log` after a failed click on v0.8.1+.
- **Release gotcha (reconfirmed this session):** never redirect `release.ps1` streams (`*>`/`*>&1`/`Tee`) under PS5.1 — `tauri build`'s stderr wraps as `NativeCommandError` + `Stop` aborts at first cargo line. **Run bare; the background runner captures output itself.** (Re-tripped it via `*> file`, then succeeded bare.)
- **⚠️ Kill dev by PID only** (post-Velopack prod app + dev binary both `rift-tauri.exe`). Build ran fine alongside running prod (→`target/`, not `%LOCALAPPDATA%`).
- **Carried open:** #21 stream-pump playback harness (unblocks #20 M8/M9); in-app `[Sign in]` only compile/CDP-verified.

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
