# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 91) — autonomous detect-and-fix sweep (new CC model trial)

Full autonomy granted. **Commits local only — NOT pushed** (no push auth assumed; runner VM 100 OFFLINE — pushed CI would queue). Baselines green before+after: Rust 95, vitest 51, svelte-check 0/0.

- **`dirs_home` deduped** (`4bbc805`) — `assistant/mod.rs` delegates to `state::paths`; `stt/` infallible variants untouched (behavior change).
- **GH Actions Node-20 bump** (`062bc3b`) — checkout@v6, setup-node@v6, setup-dotnet@v5; runner 2.335.1 compatible.
- **Settings polish** (`a825cb6`) — hero banner = single auth+CLI surface (version inline, pill+stamp+Re-probe right, update CTA conditional); CLI row deleted; ` (Claude Code)` suffix stripped; proxy copy −40%. CDP-verified.
- **ISSUES.md pruned** (`cd9cbf9`) — #21/Queue/Rail-v1 deleted (shipped); Auth-Rec → live-verify stub; Rail-v2 filed T3.
- **Model probe:** `claude -p --model claude-fable-5` answered live — ID valid on this Max sub; swap still held (see RESUME).

### RESUME HERE (cont.91)
- **Push the 6 local commits** when user OKs (runner was offline — verify it's back, jobs will queue otherwise).
- **Update test PENDING** — prod still **0.8.10**; user click on "Update available" pill verifies Velopack download→apply→relaunch.
- **`--no-bundle` validation** — confirm green + timing on next real release.
- **MODEL SWAP half-unblocked:** `claude-fable-5` live-verified 2026-06-09 (the "live API access" path). Still DO NOT wire until pricing + ctx + effort tiers publish — picker/cost-cockpit metadata would be fabricated. Rift stays on `opus` alias (auto-tracks newest Opus on GA).
- **Perf roadmap (NOT applied):** runner `CARGO_INCREMENTAL=1` (vs sccache); `opt-level=2`; pre-bake gh+vpk; vpk `--noPortable`.
- [carried] drag-reorder verify; `RELEASES_TOKEN` non-ASCII (strip works; re-set cleanly). Dropped: "ISSUES #100 hero-pill" — dangling ref, no #100 was ever filed; re-file if still real.

## Prior arcs — detail in `git log`
cont.90 v0.8.11 SHIPPED — first real tag-driven release on VM 100 `rift-runner` (3 provisioning fixes: PS5.1, gh CLI, gh-zip layout) + `--no-bundle` perf fix. cont.88/89 Settings + Harness redesigns → in v0.8.11. cont.88 self-hosted runner LIVE (`docs/design/self-hosted-runner.md`): **`RunnerKeepAlive` startup task load-bearing — DON'T delete**. cont.87 v0.8.10. cont.72 v0.7.0 + edit-swarm.

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
