# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 87) — v0.8.10 SHIPPED (stable update pill) + CI build-speed probe

**v0.8.10 shipped + CI-verified** (run 27204770581, success 33m). Fixes the *real* root cause of the 4-attempt "update button won't click" bug: the update affordance was a sticky toast at the top of an upward-growing, FLIP-animated stack → it slid out from under the cursor (~50/50). Replaced with a dedicated singleton **`UpdatePill.svelte`** (never reflows). Two render bugs caught in live CDP verify: generic `.pill` class collided with global `app.css:264 .pill{height:20px}` (renamed `.upd-pill`); `backdrop-filter` on a bottom-anchored fixed el → WebView2 garbage (solid bg). `updates.svelte.ts` drops the available-toast machinery; `pillVisible` drives it. Toast kept only for install-FAILURE. Verified live: render·click→dialog·snooze→persist.

**CI efficiency work (commit `a2af532`):**
- **Shipped:** `actions/cache` → `Swatinem/rust-cache@v2` (`workspaces: src-tauri`, key `release`). Old cache churned every version bump (Cargo.lock-keyed) + re-uploaded multi-GB target/. Modest win (cache round-trip, not compile). Validates on next release.
- **Probed + REJECTED:** opt-level=1 release profile. Measured root-crate rebuild (warm deps, the per-release cost): **opt-3=28.6s · opt-1/cu256=32.2s · opt-1/cu16=30.1s**. The build is **LINK-bound, not opt-bound** — lowering opt-level buys nothing, cu=256 hurts (more objects to link). Reverted.

### RESUME HERE (cont.87) — make releases fast
- **REAL lever = the Windows linker.** MSVC `link.exe` dominates the rift-crate rebuild. Switch to **`rust-lld`** via `.cargo/config.toml`: `[target.x86_64-pc-windows-msvc] linker="rust-lld"` (bundled w/ rustup; typically 2-5× faster link). MUST measure locally (touch `src/lib.rs`→`cargo build --release` warm) AND confirm `lld-link` works on `windows-latest` CI before shipping. This is THE follow-up.
- Higher-ceiling: self-hosted Windows runner on Proxmox w/ persistent warm `target/` (near-incremental builds, kills cache up/download). Or GitHub 8-core larger runner (paid, one-line `runs-on:`).
- **`RELEASES_TOKEN` still dirty at source** — re-set: `gh secret set RELEASES_TOKEN --repo Blazzer10200/rift-tauri`.
- **Node 20 action deprecation — June 16** — bump `actions/*@v4` (checkout/setup-node/setup-dotnet) majors before then.
- Carried: drag-reorder manual-verify; dedupe `dirs_home` (`assistant/mod.rs:1101` vs `state::paths`); Whisper-in-release; ISSUES #100 hero-pill hardcode (`SettingsPage.svelte:318`).
- Local = Win PowerShell 5.1 (no `pwsh`).

## Prior arcs — detail in `git log`
cont.86 first tag-driven CI release SHIPPED (v0.8.9); fixed 2 `release.ps1` bugs — dirty `RELEASES_TOKEN` (non-ASCII char broke Octokit+gh auth; strip to `[^\x21-\x7E]`, commit `a4fbbce`) + already-shipped-version targeting; `timeout-minutes:40` added. cont.85 committed all WIP + standalone hardening (env_check cmd + Local-tools card, per-workspace model fix). cont.84 tag-driven CI scaffolding. cont.83 steer/queue (CDP-verified). cont.82 MCP-nonce + Rail v1. cont.72 v0.7.0 + edit-swarm.

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
