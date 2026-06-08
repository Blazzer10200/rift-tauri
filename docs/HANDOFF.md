# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-08 (cont. 77) — SHIPPED v0.8.2 (`bdb78ad`): live update-path validation

**Cut to prove the v0.8.1 updater fix end-to-end.** User reported never receiving the v0.8.1 update (and believed nothing reached GitHub). Investigated: v0.8.1 IS live + complete on `rift-releases` (assets byte-match local) — the previous session DID ship it; the handoff's "mid-delivery" wording was wrong. User's "few more changes" never landed on disk (tree was byte-clean, only two stale stashes). With no code on the table, cut a clean **version-only bump 0.8.1→0.8.2** as the right vehicle to test delivery: a real higher version on GitHub so a v0.8.1 client exercises check→download→apply→relaunch through the live Velopack feed. `cargo check` 0/0 · `svelte-check` 0/0 (4068). `release.ps1` green: v0.8.2 Latest, Setup.exe 15,696,424 (distinct from v0.8.1's 15,687,446 → real binary, not no-op), portable dropped. Commit pushed origin/main.

### RESUME HERE (next session)
- **v0.8.2 live (`bdb78ad`).** To test the updater: machine must be on **v0.8.1 first** (broken v0.7.0 updater can't self-deliver — manual Setup.exe once). On v0.8.1, trigger update check → should report 0.8.2, download, apply-on-exit, relaunch onto 0.8.2. **If it fails, the rotating `rift.log` (Settings→Help & diagnostics→Logs) now captures Velopack internals — grab it to finally root-cause the still-open v0.7.0→v0.8.0 download failure.**
- **Carried open:** in-app `[Sign in]` (Auth-Rec) only compile/CDP-verified — needs a real logged-out machine. Steer/Permission bars need live multi-step-tool verify.
- **Reminders:** run `release.ps1` bare (no stream redirect under PS5.1). **⚠️ Kill dev by PID only** (prod + dev binary both `rift-tauri.exe`).

## Session 2026-06-08 (cont. 76) — #21 RESOLVED: conversation-playback test harness

**Closed the last T1 ship-blocker.** New `src/lib/state/assistant.playback.test.ts` covers both halves the pure-assistant rip left bare: (1) **stream pump** — real `TabState.onStream`/`onDone`/`onError` over recorded NDJSON frames, pumped-rAF so text paints between frames (coalescing, dribble, tool lifecycle, thinking, usage/cost/model, all done+error branches); (2) **send/queue/steer orchestrator** — real `send()` w/ mocked `invoke` (turn-init, auth gate, empty drop, queue+drain, `steer()` inject/fallback/`no_active_turn`). Suite 27→51; svelte-check 0/0. Two real bugs caught: no-op rAF collapsed interleaving; blank-turn lastError blocks queue drain (both fixed). Test+docs only — pushed origin/main, not released.

## Prior arcs — detail in `git log`
cont.76 #21 conversation-playback harness (`cd87b11`). cont.75 SHIPPED v0.8.1 (`5845487`) app-update observability: rotating `rift.log` sink + loud download-failure UI (v0.7.0 failure root cause still open — only instrumented). cont.73 compression toggle (`0c34161`). cont.72 SHIPPED v0.7.0 (`f687873`) + edit-swarm (`db01c70`). release.ps1 gotchas → project CLAUDE.md.

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
