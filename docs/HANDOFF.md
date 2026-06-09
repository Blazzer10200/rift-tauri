# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 96) — v0.8.14 SHIPPED: update-dialog crash fixed (the real "can't click update" root cause)

**End of the v0.8.3→v0.8.12 "can't click update" saga.** Never a click/layout/z-index bug. The pill handler fired + set `dialogOpen=true`, then `UpdateDialog` **threw on render** so the overlay never committed → looked like a dead button. Root cause: notes `{#each}` keyed on `kind+'|'+text`; every blank line → same `blank|` key → `each_key_duplicate` aborts render (data-dependent; dev "worked" w/o consecutive-blank notes). Fix = key on index (`{#each notes as ln, i (i)}`) — this WAS the dirty edit cont.95 flagged "left for user". CDP-proven: reproduced the throw live on the installed v0.8.12 prod build, confirmed clean render in dev w/ multi-blank notes. `npm run check` 0/0.

- **v0.8.14 also bundles cont.95 SEC-1 backend hardening** (swarm worktree-escape guard + 6 fixes). CI green 3m1s → published to `rift-releases`.
- **NEW open #29** (rewritten): runtime CSP `nonce` nullifies `'unsafe-inline'` → Svelte inline transition styles + download progress-bar fill blocked. **Cosmetic** (download/apply/clicks all work). Kept OUT of v0.8.14 — app-wide blast radius.

### RESUME HERE (cont.96)
- **User on v0.8.12 must update via manual `Setup.exe` ONCE** (rift-releases/releases/latest) — 0.8.12's dialog still crashes so the in-app Download button is unreachable until they're on 0.8.14; in-app updates work permanently after.
- Optional: CDP/live pass on SEC-1 (shipped source-only) · #29 CSP-nonce fix when ready (app-wide — verify every transition + `style:` binding first).

## Prior arcs — detail in `git log` + CHANGELOG
cont.95 SEC-1 backend security review (shipped in v0.8.14). cont.94 v0.8.13 Claude Fable 5 limited-run model (`claude-fable-5` front+back; **Jun 22 Rift-side sunset gate** — `FABLE_SUNSET_EPOCH_SECS=1_782_172_800`; self-heals to Sonnet/Opus after). cont.93 v0.8.12 SHIPPED (`feea28f` — pill `×` → 24h `{version,until}` snooze; blur stripped from dialog/toasts). cont.92 18-agent sweep → 3 fixed. cont.90 v0.8.11 (first tag-driven release on VM 100 `rift-runner`). cont.88 self-hosted runner LIVE: **`RunnerKeepAlive` startup task load-bearing — DON'T delete**. cont.72 v0.7.0 + edit-swarm. **Latest release = v0.8.14; user's prod = 0.8.12 until they run Setup.exe (PID-only kills, NEVER by image name).**
[carried] `.slideover`/`.tip` blur (fix on new scuff only) · runner perf roadmap · drag-reorder verify · `RELEASES_TOKEN` re-set.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step**; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = **hero + sticky pill-tabs + single-column titled cards** (`.st-block`=card, header band inside; cont.88). Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.8.14 stands** (shipped 2026-06-09, cont.96). Harness has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
