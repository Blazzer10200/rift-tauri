# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 97) — #20 BOTH splits: TS COMPLETE (M8+M9) + mod.rs 5/8 + UI-drift fixed (autonomous, unshipped)

Eight code commits on `main`, **source-verified only** (svelte-check 0/0 · vitest 51/51 · cargo test 95/95 + zero warnings per Rust commit; no dev launch — user gaming):

- **TS split DONE:** `b4ea421` M8 stream pump → `assistant/streaming.ts`; `ea514e8` M9 send orchestrator → `assistant/send.ts`. **`assistant.svelte.ts` 2709 → 1700L; M0-M9 ALL DONE.** (M8/M9 use type-only parent imports — documented in headers + brief.)
- **`6e7cb21` UI-drift fixed:** Settings chip now renders from new `updates.summary` (one derived `{kind,label}`), never hard-coded.
- **mod.rs split 5/8** per new `docs/design/assistant-mod-split.md`: `9338179` R1 `cli_install` · `3602312` R5 `env_checks` · `a8a2c0b` R7 `workspace` · `e091890` R4 `auth_update` · `99864c4` R3 `convo_store`. **mod.rs 4331 → 2917L.** Lessons IN THE BRIEF: tauri commands need GLOB re-exports (`__cmd__*` items); `pub(super)` covers the whole assistant subtree.

### RESUME HERE (cont.97)
- **User on v0.8.12 still needs ONE manual `Setup.exe`** (rift-releases/releases/latest) — in-app Download unreachable until 0.8.14+.
- **PUSHED + RELEASED as v0.8.15** (runtime smoke pass SKIPPED on user call) — next dev session still do the CDP pass: real turn (stream/tools/thinking), steer, stop, /retry, queue drain, Settings update-chip states, auth pill, History list/load/delete (convo_store moved!).
- Remaining #20: mod.rs R2 (config) → R6 (oneshot) → R8 (turn, last). Then `Composer.svelte` brief.
- Parked: SEC-1 live pass · #29 CSP-nonce (app-wide).

## Prior arcs — detail in `git log` + CHANGELOG
cont.96 v0.8.14 SHIPPED — update-dialog crash root-caused (`each_key_duplicate` on blank-line notes keys; keyed by index now) + SEC-1 hardening bundled; saga over.
cont.94 v0.8.13 Claude Fable 5 limited-run model (**Jun 22 Rift-side sunset gate** — `FABLE_SUNSET_EPOCH_SECS=1_782_172_800`; self-heals to Sonnet/Opus after). cont.93 v0.8.12 (pill `×` → 24h snooze). cont.90 first tag-driven release on VM 100 `rift-runner`; cont.88 runner LIVE: **`RunnerKeepAlive` startup task load-bearing — DON'T delete**. **Latest release = v0.8.15 (shipped cont.97, CI 3m25s); user prod = 0.8.12 until they run Setup.exe ONCE (PID-only kills, NEVER by image name).**
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
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.8.15 stands** (shipped 2026-06-09, cont.97). Harness has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
