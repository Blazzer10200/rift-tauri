# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 90) — v0.8.11 SHIPPED + release pipeline hardened; model-swap blocked

**v0.8.11 shipped green** via the self-hosted runner — the **first real tag-driven release on VM 100 `rift-runner`**. Bundles the cont.88 Settings redesign + cont.89 Harness one-viewport overhaul (both now RELEASED, not "unreleased"). Published to `rift-releases` w/ full Velopack feed. Warm runner did build+pack+publish in ~3.5 min.

**Runner needed 3 provisioning fixes** — the cont.88 runner flip was never exercised by a real release (v0.8.10 shipped on the old GH-hosted runner just before the flip):
- `pwsh`→`powershell` — WS2022 has Windows PS 5.1 only (`b49ca40`)
- gh CLI absent → added a download-latest-zip step (`5ba1967`)
- gh-zip layout (`bin/` at archive root, not nested) → recursive `gh.exe` find (`15ddce7`)

**`--no-bundle` perf fix committed** this session: release.ps1 ran a full `tauri build` then **discarded** the NSIS installer (Velopack makes its own Setup.exe) — ~11s/release of wasted makensis + NSIS downloads. Now skipped. **Untested in CI — validates on the NEXT release.** Build is **codegen-bound ~2m14s** (the real cost; rust-lld already rejected).

### RESUME HERE (cont.90)
- **Update test PENDING** — prod still **0.8.10**; awaiting a user click on the "Update available" pill to verify the full Velopack download→apply→relaunch. Pill is one-click by design (won't auto-install). Prod exe: `%LOCALAPPDATA%\Rift\current\rift-tauri.exe`.
- **`--no-bundle` validation** — confirm green + measure timing on the next real release.
- **MODEL SWAP BLOCKED — do NOT fabricate an ID.** The "new model" hype (Claude **Fable**/Mythos, Project Glasswing, dropped Jun 9) is a **defensive-cyber** model, ~2× Opus, **invitation-only, NO documented self-serve API model ID** — not in the official models table, not a coding-assistant model. Rift stays on **Opus 4.8** (`claude-opus-4-8`, newest GA). Wire a new model ONLY given a verified `claude-...` ID from official docs or live API access; then also update cost-cockpit pricing.
- **Perf roadmap (documented, NOT applied — needs benchmark):** `CARGO_INCREMENTAL=1` on the persistent runner (conflicts w/ sccache — biggest potential win on the 2m14s build); `[profile.release] opt-level=2`; pre-bake gh+vpk into the `toolchain-ready` snapshot (~19s); vpk `--noPortable` (skip create→upload→delete, ~5s).
- [carried] Settings polish (merge green banner + "Claude Code CLI" row; compression-proxy copy; strip `(Claude Code)` from CLI chip); drag-reorder verify; dedupe `dirs_home` (`assistant/mod.rs:1101`); ISSUES #100 hero-pill; `RELEASES_TOKEN` non-ASCII (strip works; re-set cleanly); Node-20 action deprecation Jun 16 (`actions/*@v4`).

## Prior arcs — detail in `git log`
cont.88/89 Settings + Harness redesigns → SHIPPED in v0.8.11. cont.88 self-hosted runner LIVE (`docs/design/self-hosted-runner.md`): VM 100 @ blazzer-labs, **`RunnerKeepAlive` startup task load-bearing — DON'T delete**. cont.87 v0.8.10 (stable UpdatePill). cont.72 v0.7.0 + edit-swarm.

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
