# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 85) — Committed all WIP + standalone hardening

**Tree was multi-arc + entangled in shared files → committed as 4 logical groups (NOT one sweep):** `1810c2e` chore housekeeping · `8076bc7` ci release-workflow+test-harness (cont.84) · `dbb3c31` backend tests (cont.80) · `2c02632` feat assistant-UX (cont.78 Bento + cont.82 Rail/nonce + cont.83 steer/queue + per-workspace fix) + standalone hardening. Tree clean.

**Done:**
1. **cont.83 steer/queue VERIFIED live (CDP)** — steer marker renders, Pending Rail send-now/edit/rail-Steer-flash/queue-drain all work, 0 console errors. Drag-reorder NOT CDP-driveable → manual confirm pending.
2. **Per-workspace model pollution FIXED** (`helpers.ts` `saveModel`/`saveEffort`) — per-workspace save writes ONLY `base::<root>`, never clobbers the global baseline. Verified: pinning exfil-v1→Opus wrote only the `::root` key.
3. **Standalone hardening** — audit (Explore agent) then verified each finding. **Built:** `environment_check` cmd (git/node/npm/cargo/code) + `environment.svelte.ts` store → Settings→About→**Local tools** card (pixel-verified Installed/Not-found); gate "Open in VS Code" on `code` (F3); clickable onboarding install link (F5); immediate OAuth notice (F12); `minWidth` 1280→1100 (F10).
4. **DEBUNKED agent findings** (verify-before-act): F1 (Tauri 2 auto-bootstraps WebView2), F7 (Whisper already gated via `stt.backendAvailable`), F12-orig (3-min timeout already existed).

### RESUME HERE (cont.85)
- **First CI release still UNtested** — ship via `v*` tag-push, watch Actions publish to rift-releases (`RELEASES_TOKEN` secret in place).
- **NOT pixel-verified:** drag-reorder (manual), F10 `minWidth` 1100 (needs app restart), F12 notice (needs real OAuth).
- **Optional follow-ups:** surface git/npm/cargo inline at swarm trigger (deeper F2); dedupe duplicate `dirs_home` (`assistant/mod.rs:1101` vs `state::paths`); decide whether to ship Whisper in release; ISSUES #100 hero-pill hardcode (`SettingsPage.svelte:318`) — needs `updates`-store wiring.
- Dev server LEFT RUNNING. Local = Win PowerShell 5.1 (no `pwsh`).

## Prior arcs — detail in `git log`
cont.84 tag-driven CI + T9 update-test harness (RESOLVED). cont.83 steer/queue. cont.82 MCP-config nonce + Rail v1. cont.79 v0.8.3→v0.8.8 updater. cont.72 v0.7.0 + edit-swarm.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = **hero + sticky pill-tabs + per-section bento** (redesigned cont.78, was sidebar+5-sections); Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.7.0 stands** (shipped 2026-06-07, cont.72). Harness now has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
