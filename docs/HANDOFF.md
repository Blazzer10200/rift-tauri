# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 94) — Claude Fable 5 wired (limited run, until Jun 22)

**`claude-fable-5` added front+back** (supersedes cont.93's "don't wire" hold — user override; specs live-confirmed from anthropic.com/claude/fable + platform docs: 1M ctx, 128k out, $10/$50 MTok, adaptive thinking always-on, no fast mode. Docs publish NO sunset — **Jun 22 cutoff is a Rift-side gate per user instruction**, easy to bump if a real date lands).

- `types.ts`/`helpers.ts` — ModelSel += fable; `FABLE_SUNSET_MS = Date.UTC(2026,5,23)` + `fableAvailable()`; stored pref self-heals → sonnet post-sunset; modelFamily → opus hue.
- `Composer.svelte` — Fable 5 top row, accent name + "UNTIL JUN 22" badge (`limited` flag; `.model-badge` tints via `color-mix(in oklab, var(--accent)…)`). Row vanishes after sunset, shortcuts renumber. effort→ultra, fastMode false.
- `mod.rs` — `FABLE_SUNSET_EPOCH_SECS = 1_782_172_800` (=2026-06-23T00:00Z, verified); `assistant_send` guard AFTER pin resolution → stale pref/pinned session falls back to `opus` (Anthropic route only).
- `model-prices.json` fable 10/50, cache 12.5/1.0 · HomePage label "Fable 5".

**Verified:** svelte-check 0/0 · cargo check green · CDP pixels (picker selected+unselected, pill "Fable 5 · Smart", per-ws key stores `claude-fable-5`, 0 console errors).

### RESUME HERE (cont.94)
- **Ship release w/ Fable** (user queued post-compaction): bump → CHANGELOG → commit → tag `vX.Y.Z` → push --tags.
- [carried] prod 0.8.11→0.8.12 pill update test awaiting user report · `.slideover`/`.tip` backdrop-filter (fix only on new scuff reports) · perf roadmap (CARGO_INCREMENTAL/opt-level-2/pre-bake/--noPortable) · drag-reorder verify · `RELEASES_TOKEN` non-ASCII re-set.

## cont.93 (same day) — update-flow root cause FIXED + v0.8.12 SHIPPED

Pill `×` had version-permanently silenced updates (`dismissed-version` in prod localStorage); scuff = WebView2 backdrop-filter mis-composite. Fixed (`feea28f`): 24h `{version,until}` snooze JSON (legacy string self-discards) · snooze-proof gear dot · blur stripped from dialog/toasts. **v0.8.12 tagged + CI green** (4 assets; validated vpk-idempotency + `--no-bundle`). Prod = 0.8.11 (PID-only kills, NEVER by image name).

## cont.92/91 (same day, earlier) — sweeps

cont.92: 18-agent Workflow sweep → 3 fixed (`b78f2c5` vpk idempotent · `7cc2ce2` stt lock · `d0821fd` enhance race), pushed, CI green. cont.91: `dirs_home` dedupe · Actions @v6 · Settings hero polish · ISSUES prune.

## Prior arcs — detail in `git log`
cont.90 v0.8.11 SHIPPED — first tag-driven release on VM 100 `rift-runner`. cont.88 self-hosted runner LIVE (`docs/design/self-hosted-runner.md`): **`RunnerKeepAlive` startup task load-bearing — DON'T delete**. cont.88/89 Settings+Harness redesigns. cont.72 v0.7.0 + edit-swarm.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step**; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = **hero + sticky pill-tabs + single-column titled cards** (`.st-block`=card, header band inside; cont.88). Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.8.11 stands** (shipped 2026-06-09, cont.90). Harness has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
