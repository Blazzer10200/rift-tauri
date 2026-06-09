# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 95) — backend security review + fixes

Full `src-tauri/src/` defensive review (5 parallel subsystem finders → adversarial false-positive verify). **1 real bug + 6 hardening fixes in-tree, `cargo check` green (EXIT=0, 0 warn). Full block: ISSUES.md → SEC-1.**

- **T1 — swarm worktree escape** (`swarm/mod.rs`): unvalidated `Finding.file` from `swarm_run` IPC reached git ops + a `bypassPermissions` edit agent w/ `current_dir(wt)` → `../`/absolute path escaped the sandbox. Fix: `validate_rel_path()` (rejects absolute/drive-rel/`..`/newlines) before dispatch.
- **T4:** `about:`→exact `about:blank` + `read_page` uses trusted `wv.url()` (`browser/mod.rs`) · STT re-verifies SHA256 of pre-existing model file (`model_manager.rs`) · SSH `StrictHostKeyChecking=yes` (`git_local.rs`) · `trust_level()` `OnceLock`-frozen (`mcp_server.rs`) · non-finite cost filtered (`usage/mod.rs`) · budget uses `atomic_write_json` (`budget.rs`).
- **Refuted:** `apply_pending_update` IPC (dock has no IPC; DoS-only) · unsigned update (Velopack verifies). **Not runtime-verified — source-only.**
- **Untouched, not mine:** `UpdateDialog.svelte` `{#each}` keyed→indexed change was already dirty in the tree pre-session — left for user.

### RESUME HERE (cont.95)
- SEC-1 fixes are **source-only** — consider a CDP/live pass on swarm + browser dock before ship.
- Ship still queued (cont.94): bump → CHANGELOG → commit → tag → push.

## Session 2026-06-09 (cont. 94) — Claude Fable 5 wired (limited run, until Jun 22)

**`claude-fable-5` added front+back** (supersedes cont.93's "don't wire" hold — user override; specs live-confirmed from anthropic.com/claude/fable + platform docs: 1M ctx, 128k out, $10/$50 MTok, adaptive thinking always-on, no fast mode. Docs publish NO sunset — **Jun 22 cutoff is a Rift-side gate per user instruction**, easy to bump if a real date lands).

- `types.ts`/`helpers.ts` — ModelSel += fable; `FABLE_SUNSET_MS = Date.UTC(2026,5,23)` + `fableAvailable()`; stored pref self-heals → sonnet post-sunset; modelFamily → opus hue.
- `Composer.svelte` — Fable 5 top row, accent name + "UNTIL JUN 22" badge (`limited` flag; `.model-badge` tints via `color-mix(in oklab, var(--accent)…)`). Row vanishes after sunset, shortcuts renumber. effort→ultra, fastMode false.
- `mod.rs` — `FABLE_SUNSET_EPOCH_SECS = 1_782_172_800` (=2026-06-23T00:00Z, verified); `assistant_send` guard AFTER pin resolution → stale pref/pinned session falls back to `opus` (Anthropic route only).
- `model-prices.json` fable 10/50, cache 12.5/1.0 · HomePage label "Fable 5".

**Verified:** svelte-check 0/0 · cargo check green · CDP pixels (picker selected+unselected, pill "Fable 5 · Smart", per-ws key stores `claude-fable-5`, 0 console errors).

[carried] ship w/ Fable queued · 0.8.12 pill test awaiting user report · `.slideover`/`.tip` blur (fix on new scuff only) · runner perf roadmap · drag-reorder verify · `RELEASES_TOKEN` re-set.

## Prior arcs — detail in `git log`
cont.93 v0.8.12 SHIPPED (`feea28f` — pill `×` now 24h `{version,until}` snooze, not permanent; blur stripped from dialog/toasts). cont.92 18-agent sweep → 3 fixed (`b78f2c5`·`7cc2ce2`·`d0821fd`). cont.91 `dirs_home` dedupe · Actions @v6. cont.90 v0.8.11 SHIPPED (first tag-driven release on VM 100 `rift-runner`). cont.88 self-hosted runner LIVE: **`RunnerKeepAlive` startup task load-bearing — DON'T delete**. cont.88/89 Settings+Harness redesigns. cont.72 v0.7.0 + edit-swarm. Prod = 0.8.11 (PID-only kills, NEVER by image name).

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
