# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-08 (cont. 76) — #21 RESOLVED: conversation-playback test harness

**Closed the last T1 ship-blocker.** New `src/lib/state/assistant.playback.test.ts` (**24 tests**) covers both halves the pure-assistant rip left bare. (1) **Stream pump** — drives real `TabState.onStream`/`onDone`/`onError` w/ recorded NDJSON frames (the verbatim wire shapes the backend forwards); rAF backed by a pumped queue so text paints between frames → faithful block ordering. Covers text coalescing, dribble, tool lifecycle, thinking, usage/cost/model, all onDone+onError branches. (2) **Send/queue/steer orchestrator** — drives real `send()` w/ mocked `invoke`: turn-init, auth gate, empty drop, queue+drain-on-completion, `steer()` (inject/queue-fallback/`no_active_turn`). Suite **27→51 tests**; svelte-check 0/0. Two real bugs caught mid-build: no-op rAF collapsed text/tool interleaving; blank-turn lastError blocks queue drain (both fixed; latter documents a real invariant). **Test+docs only — no binary change → NOT released** (a Velopack bump ships an identical binary + muddies v0.8.1 delivery). Pushed origin/main only. #20 M8+M9 both now have their net.

### RESUME HERE (next session)
- **#21 done** (`assistant.playback.test.ts`); block kept in ISSUES until `/git-ship` deletes it. The Rust per-turn reader is a verbatim line-forwarder (no parse logic) — deliberately not harnessed; all turn logic is in the store, now covered.
- **v0.8.1 still mid-delivery (release `5845487`):** Setup.exe + `Rift-0.8.1-full.nupkg` on `rift-releases`. **User manually runs Setup.exe** (broken v0.7.0 updater can't self-deliver). Once on v0.8.1: if any update fails, `rift.log` (Settings→Logs) has the error — **ask for it to root-cause the real v0.7.0→v0.8.0 download failure** (still unknown; v0.8.1 only made it observable).
- **Carried open:** in-app `[Sign in]` (Auth-Rec) only compile/CDP-verified — needs a real logged-out machine. Steer/Permission bars need live multi-step-tool verify.
- **Reminders (full detail in project CLAUDE.md):** run `release.ps1` bare (no stream redirect under PS5.1). **⚠️ Kill dev by PID only** (prod + dev binary both `rift-tauri.exe`).

## Prior arcs — detail in `git log`
cont.75 SHIPPED v0.8.1 (`5845487`) app-update observability: rotating `rift.log` sink + loud download-failure UI (v0.7.0 failure root cause still open — only instrumented). cont.73 compression toggle (`0c34161`). cont.72 SHIPPED v0.7.0 (`f687873`) + edit-swarm (`db01c70`). release.ps1 gotchas → project CLAUDE.md.

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
